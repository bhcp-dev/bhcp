use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn fresh_root(name: &str) -> PathBuf {
    let root = std::env::temp_dir()
        .join(format!("bhcp-contextual-runner-{}", std::process::id()))
        .join(name);
    if root.exists() {
        fs::remove_dir_all(&root).unwrap();
    }
    fs::create_dir_all(&root).unwrap();
    root
}

fn run(arguments: &[&Path]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_contextual_policy_multiseed"));
    for argument in arguments {
        command.arg(argument);
    }
    command.output().unwrap()
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn historical_run_ids_retain_their_direct_cargo_judges() {
    let root = fresh_root("historical");
    for directory in ["codex-home", "home", "cargo-home", "rustup-home", "tools"] {
        fs::create_dir(root.join(directory)).unwrap();
    }
    let mode = Path::new("freeze-002");
    let driver = Path::new(env!("CARGO_BIN_EXE_bhcp-experiment-fake-agent"));
    let codex = Path::new(env!("CARGO_BIN_EXE_bhcp-experiment-fake-codex"));
    let cargo = Path::new(env!("CARGO_BIN_EXE_bhcp-experiment-fake-agent"));
    let output = run(&[
        mode,
        driver,
        codex,
        &root.join("codex-home"),
        &root.join("home"),
        &root.join("cargo-home"),
        &root.join("rustup-home"),
        &root.join("tools"),
        cargo,
        &root.join("scratch"),
    ]);
    assert!(output.status.success(), "{}", text(&output));
    let output = text(&output);
    assert!(output.contains("experiment_id=contextual-policy-multiseed-002"));
    assert!(output.contains(&format!("judge=format:{}:fmt", cargo.display())));
    assert!(!output.contains(":run:1.97.1:cargo:fmt"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn corrected_read_confined_protocol_uses_a_new_run_id() {
    let root = fresh_root("corrected");
    for directory in ["codex-home", "cargo-home", "rustup-home", "toolchain/bin"] {
        fs::create_dir_all(root.join(directory)).unwrap();
    }
    for executable in [
        "cargo",
        "rustc",
        "rustdoc",
        "rustfmt",
        "cargo-clippy",
        "clippy-driver",
    ] {
        fs::write(root.join("toolchain/bin").join(executable), executable).unwrap();
    }
    let mode = Path::new("freeze-003");
    let fake = Path::new(env!("CARGO_BIN_EXE_bhcp-experiment-fake-agent"));
    let codex = Path::new(env!("CARGO_BIN_EXE_bhcp-experiment-fake-codex"));
    let deny_root = PathBuf::from(std::env::var_os("HOME").unwrap());
    let output = run(&[
        mode,
        fake,
        codex,
        &root.join("codex-home"),
        &root.join("cargo-home"),
        &root.join("rustup-home"),
        fake,
        fake,
        &root.join("toolchain/bin"),
        &deny_root,
        &root.join("scratch"),
    ]);
    assert!(output.status.success(), "{}", text(&output));
    let output = text(&output);
    assert!(output.contains("experiment_id=contextual-policy-multiseed-003"));
    assert!(output.contains("sandbox=workspace-write/no-network/read-confined"));
    assert!(output.contains("run_order=seed-01,seed-02,seed-03,seed-04,seed-05"));
    fs::remove_dir_all(root).unwrap();
}
