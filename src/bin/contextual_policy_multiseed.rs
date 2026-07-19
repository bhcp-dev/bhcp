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
    if arguments.len() < 10 || arguments.len() > 11 {
        return Err("expected MODE DRIVER CODEX CODEX_HOME HOME CARGO_HOME RUSTUP_HOME TOOL_BIN CARGO SCRATCH [OUTPUT]".to_owned());
    }
    let mode = arguments[0]
        .to_str()
        .ok_or_else(|| "mode is not UTF-8".to_owned())?;
    let (experiment_id, should_run) = match mode {
        "freeze-001" => ("contextual-policy-multiseed-001", false),
        "run-001" => ("contextual-policy-multiseed-001", true),
        "freeze-002" => ("contextual-policy-multiseed-002", false),
        "run-002" => ("contextual-policy-multiseed-002", true),
        _ => return Err("mode must be freeze-001, run-001, freeze-002, or run-002".to_owned()),
    };
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
    let plan = plan(
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
    let frozen = plan.freeze().map_err(|error| error.message)?;
    println!("plan_digest={}", frozen.plan_digest);
    println!("fixture_digest={}", frozen.fixture_digest);
    println!("run_order={}", frozen.run_order.join(","));
    match (should_run, arguments.len()) {
        (false, 10) => Ok(()),
        (true, 11) => {
            let output = absolute_path(&arguments[10], "output")?;
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
                let candidate = scratch
                    .join(experiment_id)
                    .join("workspaces")
                    .join(arm)
                    .join("subject/src/lib.rs");
                write_patch(
                    &fixture.join("subject/src/lib.rs"),
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
fn plan(
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

fn existing_directory(value: &std::ffi::OsStr, name: &str) -> Result<PathBuf, String> {
    let path = absolute_path(value, name)?;
    if !path.is_dir() {
        return Err(format!("{name} is not an existing directory"));
    }
    Ok(path)
}

fn absolute_path(value: &std::ffi::OsStr, name: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(value);
    if !path.is_absolute() {
        return Err(format!("{name} must be absolute"));
    }
    Ok(path)
}
