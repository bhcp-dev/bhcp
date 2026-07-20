use std::ffi::OsStr;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use bhcp::cbor::encode_deterministic;
use bhcp::experiment::{
    ExperimentArm, ExperimentController, ExperimentLimits, ExperimentPins, ExperimentPlan,
    JudgeCommand,
};
use bhcp::value::Value;

const EXPERIMENT_ID: &str = "in-session-evidence-forward-001";
const RUST_TOOLCHAIN: &str = "1.97.1";

fn main() {
    if let Err(error) = run() {
        eprintln!("in-session evidence forward run failed: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    let mode = arguments
        .first()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "missing UTF-8 mode".to_owned())?;
    let (prepare, should_run) = match mode {
        "prepare-001" => (true, false),
        "freeze-001" => (false, false),
        "run-001" => (false, true),
        _ => return Err("mode must be prepare-001, freeze-001, or run-001".to_owned()),
    };
    if arguments.len() != if should_run { 14 } else { 13 } {
        return Err("expected MODE DRIVER CODEX CODEX_HOME CARGO_HOME RUSTUP_HOME BHCP RUSTUP TOOLCHAIN_BIN DENY_ROOT ADAPTER PREPARED_FIXTURE SCRATCH [OUTPUT]".to_owned());
    }
    let driver = canonical_file(&arguments[1], "driver")?;
    let codex = canonical_file(&arguments[2], "Codex")?;
    let codex_home = canonical_directory(&arguments[3], "Codex home")?;
    let cargo_home = canonical_directory(&arguments[4], "Cargo home")?;
    let rustup_home = canonical_directory(&arguments[5], "Rustup home")?;
    let bhcp = canonical_file(&arguments[6], "BHCP")?;
    let adapter_sandbox = canonical_file(
        bhcp.parent()
            .ok_or_else(|| "BHCP has no parent directory".to_owned())?
            .join("bhcp-adapter-sandbox")
            .as_os_str(),
        "BHCP adapter sandbox helper",
    )?;
    let rustup = canonical_file(&arguments[7], "Rustup")?;
    let toolchain_bin = canonical_directory(&arguments[8], "toolchain bin")?;
    let deny_root = canonical_directory(&arguments[9], "denied read root")?;
    let adapter = canonical_file(&arguments[10], "in-session adapter")?;
    let prepared = absolute_path(&arguments[11], "prepared fixture")?;
    let scratch = absolute_path(&arguments[12], "scratch")?;
    let base =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("experiments/in-session-evidence-agent");
    let original_oracle = fs::canonicalize(base.join("oracle/tests/invariants.rs"))
        .map_err(|error| format!("cannot resolve original oracle: {error}"))?;
    if !original_oracle.starts_with(&deny_root) {
        return Err("denied read root does not contain the original oracle".to_owned());
    }
    if prepare {
        prepare_fixture(&base, &prepared, &adapter)?;
    } else if !prepared.is_dir() {
        return Err("prepared fixture does not exist".to_owned());
    }
    let prepared = fs::canonicalize(prepared)
        .map_err(|error| format!("cannot resolve prepared fixture: {error}"))?;

    let toolchain_executables = [
        "cargo",
        "rustc",
        "rustdoc",
        "rustfmt",
        "cargo-clippy",
        "clippy-driver",
    ]
    .into_iter()
    .map(|name| canonical_file(toolchain_bin.join(name).as_os_str(), name))
    .collect::<Result<Vec<_>, _>>()?;
    if !toolchain_bin.starts_with(&rustup_home) {
        return Err("frozen toolchain is outside the supplied Rustup home".to_owned());
    }
    for (name, executable) in [
        "cargo",
        "rustc",
        "rustdoc",
        "rustfmt",
        "cargo-clippy",
        "clippy-driver",
    ]
    .into_iter()
    .zip(&toolchain_executables)
    {
        verify_rustup_selection(&rustup, name, executable)?;
    }

    let mut plan = ExperimentPlan::new(
        EXPERIMENT_ID,
        &prepared,
        &scratch,
        ExperimentPins {
            model: "gpt-5.4-mini".to_owned(),
            reasoning: "medium".to_owned(),
            sandbox: "workspace-write/no-network/read-confined".to_owned(),
            toolchain: "codex-cli-0.142.4+rust-1.97.1".to_owned(),
        },
    );
    plan.limits = ExperimentLimits {
        timeout_millis: 15 * 60 * 1_000,
        max_agent_output_bytes: 16 * 1024,
        max_judge_output_bytes: 2 * 1024 * 1024,
    };
    plan.allowed_changes = vec![PathBuf::from("subject/src/lib.rs")];
    plan.oracle_source = Some(prepared.join("oracle"));
    plan.trusted_executables = vec![
        codex.clone(),
        bhcp.clone(),
        adapter_sandbox,
        rustup.clone(),
        adapter.clone(),
    ];
    plan.trusted_executables
        .extend_from_slice(&toolchain_executables);
    let mut arm = ExperimentArm::new("forward-01", "FORWARD_PROMPT.md", &driver);
    arm.arguments = [
        &codex,
        &codex_home,
        &cargo_home,
        &rustup_home,
        &bhcp,
        &toolchain_bin,
        Path::new(RUST_TOOLCHAIN),
        &deny_root,
        &original_oracle,
    ]
    .into_iter()
    .map(|path| path.to_string_lossy().into_owned())
    .collect();
    arm.contract_files = vec![
        PathBuf::from("TASK.md"),
        PathBuf::from("contract.bhcp"),
        PathBuf::from("contract.semantic-id"),
        PathBuf::from("bhcp-project.toml"),
        PathBuf::from("candidate.cbor"),
    ];
    plan.arms = vec![arm];
    plan.judges = vec![
        cargo_judge(
            "format",
            &rustup,
            ["fmt", "--check", "--manifest-path", "subject/Cargo.toml"],
        ),
        cargo_judge(
            "clippy",
            &rustup,
            [
                "clippy",
                "--offline",
                "--manifest-path",
                "subject/Cargo.toml",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ],
        ),
        cargo_judge(
            "public",
            &rustup,
            ["test", "--offline", "--manifest-path", "subject/Cargo.toml"],
        ),
        cargo_judge(
            "oracle",
            &rustup,
            ["test", "--offline", "--manifest-path", "oracle/Cargo.toml"],
        )
        .with_oracle(),
        JudgeCommand::new("change-policy", &adapter, ["judge-change-policy"]),
    ];

    let frozen = plan.freeze().map_err(|error| error.message)?;
    println!("experiment_id={EXPERIMENT_ID}");
    println!("sandbox={}", plan.pins.sandbox);
    println!("plan_digest={}", frozen.plan_digest);
    println!("fixture_digest={}", frozen.fixture_digest);
    println!("run_order={}", frozen.run_order.join(","));
    if !should_run {
        return Ok(());
    }
    let output = absolute_path(&arguments[13], "output")?;
    if output.exists() {
        return Err("output directory already exists".to_owned());
    }
    let report = ExperimentController::new()
        .run(&plan)
        .map_err(|error| error.message)?;
    fs::create_dir(&output).map_err(|error| format!("cannot create output: {error}"))?;
    report
        .write_markdown(output.join("CONTROLLER.md"))
        .map_err(|error| error.message)?;
    write_patch(
        &prepared.join("subject/src/lib.rs"),
        &plan
            .scratch_root
            .join(EXPERIMENT_ID)
            .join("workspaces/forward-01/subject/src/lib.rs"),
        &output.join("forward-01.patch"),
    )
}

fn prepare_fixture(base: &Path, prepared: &Path, adapter: &Path) -> Result<(), String> {
    if prepared.exists() {
        return Err("prepared fixture already exists".to_owned());
    }
    copy_tree(base, prepared)?;
    let tools = prepared.join("subject/tools");
    fs::create_dir_all(&tools).map_err(|error| format!("cannot create fixture tools: {error}"))?;
    let adapter_destination = tools.join("in-session-evidence-adapter");
    fs::copy(adapter, &adapter_destination)
        .map_err(|error| format!("cannot stage adapter: {error}"))?;
    #[cfg(unix)]
    fs::set_permissions(&adapter_destination, fs::Permissions::from_mode(0o500))
        .map_err(|error| format!("cannot protect staged adapter: {error}"))?;
    let skill_source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("experiments/contextual-policy-agent/subject/.agents/skills/interpret-bhcp-contract");
    copy_tree(
        &skill_source,
        &prepared.join("subject/.agents/skills/interpret-bhcp-contract"),
    )?;
    create_file(
        &prepared.join("candidate.cbor"),
        &encode_deterministic(&Value::map([
            (
                "input",
                Value::map([(
                    "repository",
                    Value::Text("in-session-evidence@0".to_owned()),
                )]),
            ),
            (
                "output",
                Value::map([
                    ("publicPassed", Value::Bool(true)),
                    ("oraclePassed", Value::Bool(true)),
                    ("policyPassed", Value::Bool(true)),
                    (
                        "changedFiles",
                        Value::Array(vec![Value::Text("integer".to_owned()), Value::Integer(1)]),
                    ),
                ]),
            ),
        ]))
        .map_err(|error| error.to_string())?,
    )
}

fn copy_tree(source: &Path, destination: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(source)
        .map_err(|error| format!("cannot inspect fixture input: {error}"))?;
    if metadata.file_type().is_symlink() {
        return Err("fixture preparation rejects symbolic links".to_owned());
    }
    if metadata.is_file() {
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("cannot create fixture parent: {error}"))?;
        }
        fs::copy(source, destination)
            .map_err(|error| format!("cannot copy fixture file: {error}"))?;
        return Ok(());
    }
    if !metadata.is_dir() {
        return Err("fixture preparation rejects unsupported file types".to_owned());
    }
    fs::create_dir_all(destination)
        .map_err(|error| format!("cannot create fixture directory: {error}"))?;
    let mut entries = fs::read_dir(source)
        .map_err(|error| format!("cannot read fixture directory: {error}"))?
        .collect::<std::io::Result<Vec<_>>>()
        .map_err(|error| format!("cannot enumerate fixture directory: {error}"))?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        copy_tree(&entry.path(), &destination.join(entry.file_name()))?;
    }
    Ok(())
}

fn cargo_judge<const N: usize>(name: &str, rustup: &Path, arguments: [&str; N]) -> JudgeCommand {
    let arguments = ["run", RUST_TOOLCHAIN, "cargo"]
        .into_iter()
        .chain(arguments)
        .collect::<Vec<_>>();
    JudgeCommand::new(name, rustup, arguments)
}

fn verify_rustup_selection(rustup: &Path, name: &str, expected: &Path) -> Result<(), String> {
    let tool = if name == "clippy-driver" {
        "clippy-driver"
    } else if name == "cargo-clippy" {
        "cargo-clippy"
    } else {
        name
    };
    let output = Command::new(rustup)
        .args(["which", tool, "--toolchain", RUST_TOOLCHAIN])
        .env_clear()
        .output()
        .map_err(|error| format!("cannot ask Rustup for {name}: {error}"))?;
    if !output.status.success() {
        return Err(format!("Rustup could not select frozen {name}"));
    }
    let selected = String::from_utf8(output.stdout)
        .map_err(|_| format!("Rustup returned a non-UTF-8 {name} path"))?;
    let selected = fs::canonicalize(selected.trim())
        .map_err(|error| format!("cannot resolve Rustup-selected {name}: {error}"))?;
    if selected != expected {
        return Err(format!("Rustup did not select the frozen {name}"));
    }
    Ok(())
}

fn canonical_file(value: &OsStr, name: &str) -> Result<PathBuf, String> {
    let path = absolute_path(value, name)?;
    if !path.is_file() {
        return Err(format!("{name} is not an existing file"));
    }
    fs::canonicalize(path).map_err(|error| format!("cannot resolve {name}: {error}"))
}

fn canonical_directory(value: &OsStr, name: &str) -> Result<PathBuf, String> {
    let path = absolute_path(value, name)?;
    if !path.is_dir() {
        return Err(format!("{name} is not an existing directory"));
    }
    fs::canonicalize(path).map_err(|error| format!("cannot resolve {name}: {error}"))
}

fn absolute_path(value: &OsStr, name: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(value);
    if !path.is_absolute() {
        return Err(format!("{name} must be absolute"));
    }
    Ok(path)
}

fn create_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("cannot create prepared fixture file: {error}"))?;
    file.write_all(bytes)
        .map_err(|error| format!("cannot write prepared fixture file: {error}"))
}

fn write_patch(original: &Path, candidate: &Path, destination: &Path) -> Result<(), String> {
    let output = Command::new("/usr/bin/git")
        .args(["diff", "--no-index", "--no-ext-diff", "--no-prefix", "--"])
        .arg(original)
        .arg(candidate)
        .output()
        .map_err(|error| format!("cannot create candidate patch: {error}"))?;
    match output.status.code() {
        Some(0) => create_file(destination, b""),
        Some(1) => {
            let text = String::from_utf8(output.stdout)
                .map_err(|_| "candidate diff was not UTF-8".to_owned())?;
            let mut lines = text.lines();
            for _ in 0..4 {
                lines
                    .next()
                    .ok_or_else(|| "candidate diff header is incomplete".to_owned())?;
            }
            let mut normalized = String::from(
                "diff --git a/src/lib.rs b/src/lib.rs\n--- a/src/lib.rs\n+++ b/src/lib.rs\n",
            );
            for line in lines {
                normalized.push_str(line);
                normalized.push('\n');
            }
            create_file(destination, normalized.as_bytes())
        }
        code => Err(format!(
            "candidate diff returned unexpected status {code:?}"
        )),
    }
}
