use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use bhcp::experiment::{
    ExperimentArm, ExperimentController, ExperimentLimits, ExperimentPins, ExperimentPlan,
    JudgeCommand,
};

const ARMS: [&str; 5] = ["seed-01", "seed-02", "seed-03", "seed-04", "seed-05"];

fn main() {
    if let Err(error) = run() {
        eprintln!("multi-seed experiment failed: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    let mode = arguments
        .first()
        .ok_or_else(|| "missing experiment mode".to_owned())?
        .to_str()
        .ok_or_else(|| "mode is not UTF-8".to_owned())?;
    match mode {
        "freeze-001" => historical(&arguments, "contextual-policy-multiseed-001", false),
        "run-001" => historical(&arguments, "contextual-policy-multiseed-001", true),
        "freeze-002" => historical(&arguments, "contextual-policy-multiseed-002", false),
        "run-002" => historical(&arguments, "contextual-policy-multiseed-002", true),
        "freeze-003" => corrected(&arguments, false),
        "run-003" => corrected(&arguments, true),
        _ => Err("mode must select registered run 001, 002, or 003".to_owned()),
    }
}

fn historical(
    arguments: &[std::ffi::OsString],
    experiment_id: &str,
    should_run: bool,
) -> Result<(), String> {
    if arguments.len() != if should_run { 11 } else { 10 } {
        return Err("historical modes expect MODE DRIVER CODEX CODEX_HOME HOME CARGO_HOME RUSTUP_HOME TOOL_BIN CARGO SCRATCH [OUTPUT]".to_owned());
    }
    let driver = existing_file(&arguments[1], "driver")?;
    let codex = existing_file(&arguments[2], "Codex")?;
    let codex_home = existing_directory(&arguments[3], "Codex home")?;
    let home = existing_directory(&arguments[4], "home")?;
    let cargo_home = existing_directory(&arguments[5], "Cargo home")?;
    let rustup_home = existing_directory(&arguments[6], "Rustup home")?;
    let tool_bin = existing_directory(&arguments[7], "tool bin")?;
    let cargo = existing_file(&arguments[8], "Cargo")?;
    let scratch = absolute_path(&arguments[9], "scratch")?;
    let fixture =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("experiments/contextual-policy-agent");
    let plan = historical_plan(
        experiment_id,
        &fixture,
        &scratch,
        &driver,
        &codex,
        &codex_home,
        &home,
        &cargo_home,
        &rustup_home,
        &tool_bin,
        &cargo,
    );
    finish(plan, should_run, arguments.get(10))
}

fn corrected(arguments: &[std::ffi::OsString], should_run: bool) -> Result<(), String> {
    if arguments.len() != if should_run { 12 } else { 11 } {
        return Err("run 003 expects MODE DRIVER CODEX CODEX_HOME CARGO_HOME RUSTUP_HOME BHCP RUSTUP TOOLCHAIN_BIN DENY_ROOT SCRATCH [OUTPUT]".to_owned());
    }
    let driver = canonical_file(&arguments[1], "driver")?;
    let codex = canonical_file(&arguments[2], "Codex")?;
    let codex_home = canonical_directory(&arguments[3], "Codex home")?;
    let cargo_home = canonical_directory(&arguments[4], "Cargo home")?;
    let rustup_home = canonical_directory(&arguments[5], "Rustup home")?;
    let bhcp = canonical_file(&arguments[6], "BHCP")?;
    let rustup = canonical_file(&arguments[7], "Rustup")?;
    let toolchain_bin = canonical_directory(&arguments[8], "toolchain bin")?;
    let deny_root = canonical_directory(&arguments[9], "denied read root")?;
    let scratch = absolute_path(&arguments[10], "scratch")?;
    let fixture =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("experiments/contextual-policy-agent");
    let oracle_probe = fs::canonicalize(fixture.join("oracle/src/lib.rs"))
        .map_err(|error| format!("cannot resolve withheld oracle probe: {error}"))?;
    if !oracle_probe.starts_with(&deny_root) {
        return Err("denied read root does not contain the original oracle".to_owned());
    }
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
    let plan = corrected_plan(
        &fixture,
        &scratch,
        &driver,
        &codex,
        &codex_home,
        &cargo_home,
        &rustup_home,
        &bhcp,
        &rustup,
        &toolchain_bin,
        &toolchain_executables,
        &deny_root,
        &oracle_probe,
    );
    finish(plan, should_run, arguments.get(11))
}

fn finish(
    plan: ExperimentPlan,
    should_run: bool,
    output_argument: Option<&std::ffi::OsString>,
) -> Result<(), String> {
    let frozen = plan.freeze().map_err(|error| error.message)?;
    println!("experiment_id={}", plan.id);
    println!("sandbox={}", plan.pins.sandbox);
    println!("plan_digest={}", frozen.plan_digest);
    println!("fixture_digest={}", frozen.fixture_digest);
    println!("run_order={}", frozen.run_order.join(","));
    for judge in &plan.judges {
        println!(
            "judge={}:{}:{}",
            judge.name,
            judge.executable.display(),
            judge.arguments.join(":")
        );
    }
    match (should_run, output_argument) {
        (false, None) => Ok(()),
        (true, Some(output_argument)) => {
            let output = absolute_path(output_argument, "output")?;
            if output.exists() {
                return Err("output directory already exists".to_owned());
            }
            let report = ExperimentController::new()
                .run(&plan)
                .map_err(|error| error.message)?;
            fs::create_dir(&output)
                .map_err(|error| format!("cannot create output directory: {error}"))?;
            report
                .write_markdown(output.join("CONTROLLER.md"))
                .map_err(|error| error.message)?;
            for arm in &ARMS {
                let candidate = plan
                    .scratch_root
                    .join(&plan.id)
                    .join("workspaces")
                    .join(arm)
                    .join("subject/src/lib.rs");
                write_patch(
                    &plan.fixture_root.join("subject/src/lib.rs"),
                    &candidate,
                    &output.join(format!("{arm}.patch")),
                )?;
            }
            Ok(())
        }
        _ => Err("freeze modes omit OUTPUT and run modes require OUTPUT".to_owned()),
    }
}

#[allow(clippy::too_many_arguments)]
fn historical_plan(
    experiment_id: &str,
    fixture: &Path,
    scratch: &Path,
    driver: &Path,
    codex: &Path,
    codex_home: &Path,
    home: &Path,
    cargo_home: &Path,
    rustup_home: &Path,
    tool_bin: &Path,
    cargo: &Path,
) -> ExperimentPlan {
    let mut plan = ExperimentPlan::new(
        experiment_id,
        fixture,
        scratch,
        ExperimentPins {
            model: "gpt-5.4-mini".to_owned(),
            reasoning: "medium".to_owned(),
            sandbox: "workspace-write/no-network".to_owned(),
            toolchain: "codex-cli-0.142.4+rust-1.97.1".to_owned(),
        },
    );
    plan.limits = ExperimentLimits {
        timeout_millis: 15 * 60 * 1_000,
        max_agent_output_bytes: 16 * 1_024,
        max_judge_output_bytes: 2 * 1_024 * 1_024,
    };
    let driver_arguments: Vec<String> =
        [codex, codex_home, home, cargo_home, rustup_home, tool_bin]
            .into_iter()
            .map(|path| path.to_string_lossy().into_owned())
            .chain(std::iter::once("1.97.1".to_owned()))
            .collect();
    plan.arms = ARMS
        .into_iter()
        .map(|id| {
            let mut arm = ExperimentArm::new(id, "MULTISEED_PROMPT.md", driver);
            arm.arguments.clone_from(&driver_arguments);
            arm.contract_files = vec![
                PathBuf::from("TASK.md"),
                PathBuf::from("contract.bhcp"),
                PathBuf::from("contract.semantic-id"),
            ];
            arm
        })
        .collect();
    plan.oracle_source = Some(fixture.join("oracle"));
    plan.judges = vec![
        judge(
            "format",
            cargo,
            ["fmt", "--check", "--manifest-path", "subject/Cargo.toml"],
        ),
        judge(
            "clippy",
            cargo,
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
        judge(
            "public",
            cargo,
            ["test", "--offline", "--manifest-path", "subject/Cargo.toml"],
        ),
        judge(
            "oracle",
            cargo,
            ["test", "--offline", "--manifest-path", "oracle/Cargo.toml"],
        )
        .with_oracle(),
    ];
    plan
}

fn judge<const N: usize>(name: &str, cargo: &Path, arguments: [&str; N]) -> JudgeCommand {
    JudgeCommand::new(name, cargo, arguments)
}

#[allow(clippy::too_many_arguments)]
fn corrected_plan(
    fixture: &Path,
    scratch: &Path,
    driver: &Path,
    codex: &Path,
    codex_home: &Path,
    cargo_home: &Path,
    rustup_home: &Path,
    bhcp: &Path,
    rustup: &Path,
    toolchain_bin: &Path,
    toolchain_executables: &[PathBuf],
    deny_root: &Path,
    oracle_probe: &Path,
) -> ExperimentPlan {
    let mut plan = ExperimentPlan::new(
        "contextual-policy-multiseed-003",
        fixture,
        scratch,
        ExperimentPins {
            model: "gpt-5.4-mini".to_owned(),
            reasoning: "medium".to_owned(),
            sandbox: "workspace-write/no-network/read-confined".to_owned(),
            toolchain: "codex-cli-0.142.4+rust-1.97.1".to_owned(),
        },
    );
    plan.limits = ExperimentLimits {
        timeout_millis: 15 * 60 * 1_000,
        max_agent_output_bytes: 16 * 1_024,
        max_judge_output_bytes: 2 * 1_024 * 1_024,
    };
    let driver_arguments = [
        codex,
        codex_home,
        cargo_home,
        rustup_home,
        bhcp,
        toolchain_bin,
        Path::new("1.97.1"),
        deny_root,
        oracle_probe,
    ]
    .into_iter()
    .map(|path| path.to_string_lossy().into_owned())
    .collect::<Vec<_>>();
    plan.trusted_executables = vec![codex.to_owned(), bhcp.to_owned(), rustup.to_owned()];
    plan.trusted_executables
        .extend_from_slice(toolchain_executables);
    plan.arms = ARMS
        .into_iter()
        .map(|id| {
            let mut arm = ExperimentArm::new(id, "MULTISEED_PROMPT.md", driver);
            arm.arguments.clone_from(&driver_arguments);
            arm.contract_files = vec![
                PathBuf::from("TASK.md"),
                PathBuf::from("contract.bhcp"),
                PathBuf::from("contract.semantic-id"),
            ];
            arm
        })
        .collect();
    plan.oracle_source = Some(fixture.join("oracle"));
    plan.judges = vec![
        cargo_judge(
            "format",
            rustup,
            ["fmt", "--check", "--manifest-path", "subject/Cargo.toml"],
        ),
        cargo_judge(
            "clippy",
            rustup,
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
            rustup,
            ["test", "--offline", "--manifest-path", "subject/Cargo.toml"],
        ),
        cargo_judge(
            "oracle",
            rustup,
            ["test", "--offline", "--manifest-path", "oracle/Cargo.toml"],
        )
        .with_oracle(),
    ];
    plan
}

fn cargo_judge<const N: usize>(name: &str, rustup: &Path, arguments: [&str; N]) -> JudgeCommand {
    let arguments = ["run", "1.97.1", "cargo"]
        .into_iter()
        .chain(arguments)
        .collect::<Vec<_>>();
    JudgeCommand::new(name, rustup, arguments)
}

fn write_patch(original: &Path, candidate: &Path, destination: &Path) -> Result<(), String> {
    let output = Command::new("/usr/bin/git")
        .args(["diff", "--no-index", "--no-ext-diff", "--no-prefix", "--"])
        .arg(original)
        .arg(candidate)
        .output()
        .map_err(|error| format!("cannot create candidate patch: {error}"))?;
    if output.status.code() != Some(1) {
        return Err(format!(
            "candidate diff returned unexpected status {:?}",
            output.status.code()
        ));
    }
    let text =
        String::from_utf8(output.stdout).map_err(|_| "candidate diff was not UTF-8".to_owned())?;
    let mut lines = text.lines();
    let _ = lines
        .next()
        .ok_or_else(|| "candidate diff is empty".to_owned())?;
    let _ = lines
        .next()
        .ok_or_else(|| "candidate diff has no index".to_owned())?;
    let _ = lines
        .next()
        .ok_or_else(|| "candidate diff has no old path".to_owned())?;
    let _ = lines
        .next()
        .ok_or_else(|| "candidate diff has no new path".to_owned())?;
    let mut normalized =
        String::from("diff --git a/src/lib.rs b/src/lib.rs\n--- a/src/lib.rs\n+++ b/src/lib.rs\n");
    for line in lines {
        normalized.push_str(line);
        normalized.push('\n');
    }
    create_file(destination, normalized.as_bytes())
}

fn create_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("cannot create experiment artifact: {error}"))?;
    file.write_all(bytes)
        .map_err(|error| format!("cannot write experiment artifact: {error}"))
}

fn existing_file(value: &std::ffi::OsStr, name: &str) -> Result<PathBuf, String> {
    let path = absolute_path(value, name)?;
    if !path.is_file() {
        return Err(format!("{name} is not an existing file"));
    }
    Ok(path)
}

fn canonical_file(value: &std::ffi::OsStr, name: &str) -> Result<PathBuf, String> {
    let path = existing_file(value, name)?;
    fs::canonicalize(path).map_err(|error| format!("cannot resolve {name}: {error}"))
}

fn existing_directory(value: &std::ffi::OsStr, name: &str) -> Result<PathBuf, String> {
    let path = absolute_path(value, name)?;
    if !path.is_dir() {
        return Err(format!("{name} is not an existing directory"));
    }
    Ok(path)
}

fn canonical_directory(value: &std::ffi::OsStr, name: &str) -> Result<PathBuf, String> {
    let path = existing_directory(value, name)?;
    fs::canonicalize(path).map_err(|error| format!("cannot resolve {name}: {error}"))
}

fn verify_rustup_selection(rustup: &Path, tool: &str, expected: &Path) -> Result<(), String> {
    let output = Command::new(rustup)
        .args(["which", tool, "--toolchain", "1.97.1"])
        .env_clear()
        .output()
        .map_err(|error| format!("cannot resolve {tool} through frozen Rustup: {error}"))?;
    if !output.status.success() {
        return Err(format!("Rustup cannot select frozen {tool}"));
    }
    let selected = String::from_utf8(output.stdout)
        .map_err(|_| format!("Rustup returned a non-UTF-8 {tool} path"))?;
    let selected = fs::canonicalize(selected.trim())
        .map_err(|error| format!("cannot resolve Rustup-selected {tool}: {error}"))?;
    if selected != expected {
        return Err(format!(
            "Rustup does not select the frozen toolchain executable for {tool}"
        ));
    }
    Ok(())
}

fn absolute_path(value: &std::ffi::OsStr, name: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(value);
    if !path.is_absolute() {
        return Err(format!("{name} must be absolute"));
    }
    Ok(path)
}
