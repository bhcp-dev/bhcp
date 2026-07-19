use std::fs::{self, OpenOptions};
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;

use bhcp::experiment_codex::summarize_events;
use serde_json::Value;

const FINAL_SCHEMA: &str = r#"{"type":"object","properties":{"claimed_success":{"type":"boolean"}},"required":["claimed_success"],"additionalProperties":false}"#;
const MAX_STDERR_BYTES: usize = 1024 * 1024;

fn main() {
    if let Err(error) = run() {
        eprintln!("Codex experiment driver failed: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    if arguments.len() != 7 {
        return Err(
            "expected CODEX CODEX_HOME HOME CARGO_HOME RUSTUP_HOME TOOL_BIN RUST_TOOLCHAIN"
                .to_owned(),
        );
    }
    let codex = existing_file(&arguments[0], "Codex executable")?;
    let codex_home = existing_directory(&arguments[1], "Codex home")?;
    let home = existing_directory(&arguments[2], "home")?;
    let cargo_home = existing_directory(&arguments[3], "Cargo home")?;
    let rustup_home = existing_directory(&arguments[4], "Rustup home")?;
    let tool_bin = existing_directory(&arguments[5], "tool bin")?;
    let rust_toolchain = arguments[6]
        .to_str()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "Rust toolchain must be non-empty UTF-8".to_owned())?;

    let cwd =
        std::env::current_dir().map_err(|error| format!("cannot resolve workspace: {error}"))?;
    let subject = cwd.join("subject");
    if !subject.is_dir() {
        return Err("controller workspace has no subject directory".to_owned());
    }
    let prompt_relative = std::env::var_os("BHCP_EXPERIMENT_PROMPT")
        .ok_or_else(|| "controller did not provide a prompt path".to_owned())?;
    let prompt_path = cwd.join(prompt_relative);
    let prompt = fs::read_to_string(&prompt_path)
        .map_err(|error| format!("cannot read frozen prompt: {error}"))?;
    let target = PathBuf::from(
        std::env::var_os("CARGO_TARGET_DIR")
            .ok_or_else(|| "controller did not provide a target directory".to_owned())?,
    );
    let schema = target.join("codex-final-schema.json");
    let final_output = target.join("codex-final.json");
    create_file(&schema, FINAL_SCHEMA.as_bytes())?;

    let model = required_pin("BHCP_EXPERIMENT_MODEL")?;
    let reasoning = required_pin("BHCP_EXPERIMENT_REASONING")?;
    let sandbox = required_pin("BHCP_EXPERIMENT_SANDBOX")?;
    let toolchain = required_pin("BHCP_EXPERIMENT_TOOLCHAIN")?;
    if sandbox != "workspace-write/no-network" {
        return Err("unsupported sandbox pin".to_owned());
    }
    let path = format!(
        "{}:{}/bin:/usr/bin:/bin",
        tool_bin.display(),
        cargo_home.display()
    );

    let mut child = Command::new(codex)
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
        .env("HOME", home)
        .env("CODEX_HOME", codex_home)
        .env("CARGO_HOME", cargo_home)
        .env("RUSTUP_HOME", rustup_home)
        .env("RUSTUP_TOOLCHAIN", rust_toolchain)
        .env("PATH", path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
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

fn existing_file(value: &std::ffi::OsStr, name: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(value);
    if !path.is_absolute() || !path.is_file() {
        return Err(format!("{name} must be an absolute existing file"));
    }
    Ok(path)
}

fn existing_directory(value: &std::ffi::OsStr, name: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(value);
    if !path.is_absolute() || !path.is_dir() {
        return Err(format!("{name} must be an absolute existing directory"));
    }
    Ok(path)
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
