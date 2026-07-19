use std::ffi::{OsStr, OsString};
use std::fs::{self, OpenOptions};
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use bhcp::experiment_codex::summarize_events;
use serde_json::Value;

const FINAL_SCHEMA: &str = r#"{"type":"object","properties":{"claimed_success":{"type":"boolean"}},"required":["claimed_success"],"additionalProperties":false}"#;
const MAX_STDERR_BYTES: usize = 1024 * 1024;
#[cfg(target_os = "macos")]
const SANDBOX_PROFILE: &str = r#"
(version 1)
(allow default)
(deny file-read* file-write* (subpath (param "DENY_ROOT")))
(deny file-read* file-write* (subpath "/tmp"))
(deny file-read* file-write* (subpath "/private/tmp"))
(deny file-read* file-write* (subpath "/private/var/folders"))
(allow file-read-metadata
  (literal "/private")
  (literal "/private/tmp")
  (subpath "/private/tmp")
  (literal "/private/var")
  (literal "/private/var/folders")
  (subpath "/private/var/folders"))
(allow file-read* file-write* (subpath (param "WORKSPACE")))
(allow file-read* file-write* (subpath (param "TARGET")))
(allow file-read* file-write* (subpath (param "CARGO_HOME")))
(allow file-read* file-write* (subpath (param "RUSTUP_HOME")))
(allow file-read* (literal (param "CODEX")))
(deny file-read* file-write* (literal (param "AUTH")))
(allow file-read* file-write*
  (require-all (literal (param "AUTH")) (process-path (param "CODEX"))))
"#;

fn main() {
    if let Err(error) = run() {
        eprintln!("Codex experiment driver failed: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    if arguments.len() != 9 {
        return Err("expected CODEX CODEX_HOME CARGO_HOME RUSTUP_HOME BHCP TOOLCHAIN_BIN RUST_TOOLCHAIN DENY_ROOT DENIED_READ_PROBE".to_owned());
    }
    let codex = existing_file(&arguments[0], "Codex executable")?;
    let codex_home = existing_directory(&arguments[1], "Codex home")?;
    let cargo_home = existing_directory(&arguments[2], "Cargo home")?;
    let rustup_home = existing_directory(&arguments[3], "Rustup home")?;
    let bhcp = existing_file(&arguments[4], "BHCP executable")?;
    let toolchain_bin = existing_directory(&arguments[5], "toolchain bin")?;
    let rust_toolchain = arguments[6]
        .to_str()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "Rust toolchain must be non-empty UTF-8".to_owned())?;
    let deny_root = existing_directory(&arguments[7], "denied read root")?;
    let denied_probe = existing_file(&arguments[8], "denied read probe")?;
    if !denied_probe.starts_with(&deny_root) {
        return Err("denied read probe is outside the denied root".to_owned());
    }

    let cwd = fs::canonicalize(
        std::env::current_dir().map_err(|error| format!("cannot resolve workspace: {error}"))?,
    )
    .map_err(|error| format!("cannot resolve workspace: {error}"))?;
    let subject = cwd.join("subject");
    if !subject.is_dir() {
        return Err("controller workspace has no subject directory".to_owned());
    }
    let prompt_relative = std::env::var_os("BHCP_EXPERIMENT_PROMPT")
        .ok_or_else(|| "controller did not provide a prompt path".to_owned())?;
    let prompt_path = fs::canonicalize(cwd.join(prompt_relative))
        .map_err(|error| format!("cannot resolve frozen prompt: {error}"))?;
    if !prompt_path.starts_with(&cwd) {
        return Err("frozen prompt escaped the controller workspace".to_owned());
    }
    let prompt = fs::read_to_string(&prompt_path)
        .map_err(|error| format!("cannot read frozen prompt: {error}"))?;
    let target = fs::canonicalize(PathBuf::from(
        std::env::var_os("CARGO_TARGET_DIR")
            .ok_or_else(|| "controller did not provide a target directory".to_owned())?,
    ))
    .map_err(|error| format!("cannot resolve controller target: {error}"))?;
    let isolated_home = target.join("home");
    let isolated_codex_home = target.join("codex-home");
    let isolated_tool_bin = target.join("tool-bin");
    for directory in [&isolated_home, &isolated_codex_home, &isolated_tool_bin] {
        fs::create_dir(directory)
            .map_err(|error| format!("cannot create isolated driver directory: {error}"))?;
    }
    let auth = isolated_codex_home.join("auth.json");
    copy_private_file(&codex_home.join("auth.json"), &auth)?;
    let isolated_bhcp = isolated_tool_bin.join("bhcp");
    copy_executable(&bhcp, &isolated_bhcp)?;
    let schema = target.join("codex-final-schema.json");
    let final_output = target.join("codex-final.json");
    create_file(&schema, FINAL_SCHEMA.as_bytes())?;

    let model = required_pin("BHCP_EXPERIMENT_MODEL")?;
    let reasoning = required_pin("BHCP_EXPERIMENT_REASONING")?;
    let sandbox = required_pin("BHCP_EXPERIMENT_SANDBOX")?;
    let toolchain = required_pin("BHCP_EXPERIMENT_TOOLCHAIN")?;
    if sandbox != "workspace-write/no-network/read-confined" {
        return Err("unsupported sandbox pin".to_owned());
    }
    let path = format!(
        "{}:{}:/usr/bin:/bin",
        isolated_tool_bin.display(),
        toolchain_bin.display()
    );

    verify_codex_version(&codex)?;
    let sandbox_parameters = SandboxParameters {
        deny_root: &deny_root,
        workspace: &cwd,
        target: &target,
        cargo_home: &cargo_home,
        rustup_home: &rustup_home,
        codex: &codex,
        auth: &auth,
    };
    verify_read_boundary(&sandbox_parameters, &prompt_path, &denied_probe)?;

    let mut command = confined_command(&sandbox_parameters)?;
    command
        .arg(&codex)
        .args([
            "exec",
            "--ephemeral",
            "--ignore-user-config",
            "--model",
            &model,
            "--sandbox",
            "workspace-write",
            "-c",
            &format!("model_reasoning_effort={reasoning:?}"),
            "-c",
            "approval_policy=\"never\"",
            "-c",
            "sandbox_workspace_write.network_access=false",
            "--skip-git-repo-check",
            "--json",
            "--output-schema",
            path_text(&schema)?,
            "-o",
            path_text(&final_output)?,
            "-C",
            path_text(&subject)?,
            "-",
        ])
        .env_clear()
        .env("HOME", &isolated_home)
        .env("CODEX_HOME", &isolated_codex_home)
        .env("CARGO_HOME", cargo_home)
        .env("CARGO_TARGET_DIR", &target)
        .env("RUSTUP_HOME", rustup_home)
        .env("RUSTUP_TOOLCHAIN", rust_toolchain)
        .env("PATH", path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(probe) = std::env::var_os("BHCP_EXPERIMENT_DENIED_READ_PROBE") {
        command.env("BHCP_EXPERIMENT_DENIED_READ_PROBE", probe);
    }
    let mut child = command
        .spawn()
        .map_err(|error| format!("cannot launch pinned Codex executable: {error}"))?;
    child
        .stdin
        .take()
        .ok_or_else(|| "cannot open Codex stdin".to_owned())?
        .write_all(prompt.as_bytes())
        .map_err(|error| format!("cannot provide frozen prompt: {error}"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "cannot capture Codex stderr".to_owned())?;
    let stderr_reader = thread::spawn(move || read_bounded(stderr, MAX_STDERR_BYTES));
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "cannot capture Codex events".to_owned())?;
    let summary = summarize_events(BufReader::new(stdout))?;
    let status = child
        .wait()
        .map_err(|error| format!("cannot wait for Codex: {error}"))?;
    let stderr_result = stderr_reader
        .join()
        .map_err(|_| "Codex stderr reader panicked".to_owned())??;
    if stderr_result {
        return Err("Codex stderr exceeded its bound".to_owned());
    }
    if !status.success() {
        return Err(format!("Codex exited with {:?}", status.code()));
    }
    let final_value: Value = serde_json::from_slice(
        &fs::read(&final_output)
            .map_err(|error| format!("cannot read Codex final claim: {error}"))?,
    )
    .map_err(|error| format!("Codex final claim is invalid JSON: {error}"))?;
    let claimed_success = final_value
        .get("claimed_success")
        .and_then(Value::as_bool)
        .ok_or_else(|| "Codex final claim has no boolean claimed_success".to_owned())?;
    let exact_claim = serde_json::json!({ "claimed_success": claimed_success });
    if final_value != exact_claim {
        return Err("Codex final claim contains unknown fields".to_owned());
    }

    println!("bhcp-agent-result@0");
    println!("status=completed");
    println!("model={model}");
    println!("reasoning={reasoning}");
    println!("sandbox={sandbox}");
    println!("toolchain={toolchain}");
    println!("claimed_success={claimed_success}");
    println!("input_tokens={}", summary.input_tokens);
    println!("cached_input_tokens={}", summary.cached_input_tokens);
    println!("output_tokens={}", summary.output_tokens);
    println!("reasoning_tokens={}", summary.reasoning_tokens);
    println!("completed_commands={}", summary.completed_commands);
    Ok(())
}

fn required_pin(name: &str) -> Result<String, String> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.is_empty() && !value.contains(['\r', '\n']))
        .ok_or_else(|| format!("controller did not provide a valid {name}"))
}

fn existing_file(value: &OsStr, name: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(value);
    if !path.is_absolute() || !path.is_file() {
        return Err(format!("{name} must be an absolute existing file"));
    }
    fs::canonicalize(path).map_err(|error| format!("cannot resolve {name}: {error}"))
}

fn existing_directory(value: &OsStr, name: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(value);
    if !path.is_absolute() || !path.is_dir() {
        return Err(format!("{name} must be an absolute existing directory"));
    }
    fs::canonicalize(path).map_err(|error| format!("cannot resolve {name}: {error}"))
}

fn copy_private_file(source: &Path, destination: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(source)
        .map_err(|error| format!("cannot inspect Codex credentials: {error}"))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err("Codex credentials must be a regular file".to_owned());
    }
    create_file(
        destination,
        &fs::read(source).map_err(|_| "cannot read Codex credentials".to_owned())?,
    )?;
    #[cfg(unix)]
    fs::set_permissions(destination, fs::Permissions::from_mode(0o600))
        .map_err(|error| format!("cannot protect isolated Codex credentials: {error}"))?;
    Ok(())
}

fn copy_executable(source: &Path, destination: &Path) -> Result<(), String> {
    fs::copy(source, destination)
        .map_err(|error| format!("cannot copy pinned BHCP executable: {error}"))?;
    #[cfg(unix)]
    fs::set_permissions(destination, fs::Permissions::from_mode(0o500))
        .map_err(|error| format!("cannot protect isolated BHCP executable: {error}"))?;
    Ok(())
}

fn verify_codex_version(codex: &Path) -> Result<(), String> {
    let output = Command::new(codex)
        .arg("--version")
        .env_clear()
        .output()
        .map_err(|error| format!("cannot inspect pinned Codex executable: {error}"))?;
    if !output.status.success() || output.stdout != b"codex-cli 0.142.4\n" {
        return Err("Codex executable does not report codex-cli 0.142.4".to_owned());
    }
    Ok(())
}

struct SandboxParameters<'a> {
    deny_root: &'a Path,
    workspace: &'a Path,
    target: &'a Path,
    cargo_home: &'a Path,
    rustup_home: &'a Path,
    codex: &'a Path,
    auth: &'a Path,
}

#[cfg(target_os = "macos")]
fn confined_command(parameters: &SandboxParameters<'_>) -> Result<Command, String> {
    let mut command = Command::new("/usr/bin/sandbox-exec");
    for (name, value) in [
        ("DENY_ROOT", parameters.deny_root),
        ("WORKSPACE", parameters.workspace),
        ("TARGET", parameters.target),
        ("CARGO_HOME", parameters.cargo_home),
        ("RUSTUP_HOME", parameters.rustup_home),
        ("CODEX", parameters.codex),
        ("AUTH", parameters.auth),
    ] {
        let mut definition = OsString::from(name);
        definition.push("=");
        definition.push(value);
        command.arg("-D").arg(definition);
    }
    command.arg("-p").arg(SANDBOX_PROFILE);
    Ok(command)
}

#[cfg(not(target_os = "macos"))]
fn confined_command(_parameters: &SandboxParameters<'_>) -> Result<Command, String> {
    Err("read-confined Codex experiments require macOS sandbox-exec".to_owned())
}

fn verify_read_boundary(
    parameters: &SandboxParameters<'_>,
    readable: &Path,
    denied: &Path,
) -> Result<(), String> {
    let mut positive = confined_command(parameters)?;
    let positive = positive
        .args(["/usr/bin/head", "-c", "1"])
        .arg(readable)
        .env_clear()
        .output()
        .map_err(|error| format!("cannot probe read confinement: {error}"))?;
    if !positive.status.success() || positive.stdout.len() != 1 {
        return Err("read confinement did not preserve staged workspace reads".to_owned());
    }

    let mut negative = confined_command(parameters)?;
    let negative = negative
        .args(["/usr/bin/head", "-c", "1"])
        .arg(denied)
        .env_clear()
        .output()
        .map_err(|error| format!("cannot probe denied oracle read: {error}"))?;
    if negative.status.success() || !negative.stdout.is_empty() {
        return Err("read confinement left the original oracle readable".to_owned());
    }
    Ok(())
}

fn path_text(path: &Path) -> Result<&str, String> {
    path.to_str()
        .ok_or_else(|| "experiment path is not UTF-8".to_owned())
}

fn create_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("cannot create driver artifact: {error}"))?;
    file.write_all(bytes)
        .map_err(|error| format!("cannot write driver artifact: {error}"))
}

fn read_bounded(mut input: impl Read, limit: usize) -> Result<bool, String> {
    let mut total = 0_usize;
    let mut overflowed = false;
    let mut buffer = [0_u8; 8 * 1024];
    loop {
        let read = input
            .read(&mut buffer)
            .map_err(|error| format!("cannot read Codex stderr: {error}"))?;
        if read == 0 {
            return Ok(overflowed);
        }
        total = total.saturating_add(read);
        if total > limit {
            overflowed = true;
        }
    }
}
