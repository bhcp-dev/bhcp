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

fn rustup_installation() -> (PathBuf, PathBuf, PathBuf, PathBuf) {
    let home = PathBuf::from(std::env::var_os("HOME").unwrap());
    let cargo_home = std::env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".cargo"));
    let rustup_home = std::env::var_os("RUSTUP_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".rustup"));
    let rustup = cargo_home.join("bin/rustup");
    let selected = Command::new(&rustup)
        .args(["which", "cargo", "--toolchain", "1.97.1"])
        .output()
        .unwrap();
    assert!(selected.status.success(), "{}", text(&selected));
    let toolchain_bin = PathBuf::from(String::from_utf8(selected.stdout).unwrap().trim())
        .parent()
        .unwrap()
        .to_owned();
    (cargo_home, rustup_home, rustup, toolchain_bin)
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
    assert_eq!(
        output
            .lines()
            .filter(|line| line.starts_with("judge="))
            .collect::<Vec<_>>(),
        [
            format!(
                "judge=format:{}:fmt:--check:--manifest-path:subject/Cargo.toml",
                cargo.display()
            ),
            format!(
                "judge=clippy:{}:clippy:--offline:--manifest-path:subject/Cargo.toml:--all-targets:--:-D:warnings",
                cargo.display()
            ),
            format!(
                "judge=public:{}:test:--offline:--manifest-path:subject/Cargo.toml",
                cargo.display()
            ),
            format!(
                "judge=oracle:{}:test:--offline:--manifest-path:oracle/Cargo.toml",
                cargo.display()
            ),
        ]
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn corrected_read_confined_protocol_uses_a_new_run_id() {
    let root = fresh_root("corrected");
    fs::create_dir(root.join("codex-home")).unwrap();
    let (cargo_home, rustup_home, rustup, toolchain_bin) = rustup_installation();
    let mode = Path::new("freeze-004");
    let fake = Path::new(env!("CARGO_BIN_EXE_bhcp-experiment-fake-agent"));
    let codex = Path::new(env!("CARGO_BIN_EXE_bhcp-experiment-fake-codex"));
    let deny_root = PathBuf::from(std::env::var_os("HOME").unwrap());
    let output = run(&[
        mode,
        fake,
        codex,
        &root.join("codex-home"),
        &cargo_home,
        &rustup_home,
        fake,
        &rustup,
        &toolchain_bin,
        &deny_root,
        &root.join("scratch"),
    ]);
    assert!(output.status.success(), "{}", text(&output));
    let output = text(&output);
    assert!(output.contains("experiment_id=contextual-policy-multiseed-004"));
    assert!(output.contains("sandbox=workspace-write/no-network/read-confined"));
    assert!(output.contains("run_order=seed-01,seed-02,seed-03,seed-04,seed-05"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn corrected_protocol_rejects_a_toolchain_not_selected_by_rustup() {
    let root = fresh_root("mismatched-toolchain");
    for directory in ["codex-home", "toolchain/bin"] {
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
    let (cargo_home, rustup_home, rustup, _) = rustup_installation();
    let fake = Path::new(env!("CARGO_BIN_EXE_bhcp-experiment-fake-agent"));
    let codex = Path::new(env!("CARGO_BIN_EXE_bhcp-experiment-fake-codex"));
    let deny_root = PathBuf::from(std::env::var_os("HOME").unwrap());
    let output = run(&[
        Path::new("freeze-003"),
        fake,
        codex,
        &root.join("codex-home"),
        &cargo_home,
        &rustup_home,
        fake,
        &rustup,
        &root.join("toolchain/bin"),
        &deny_root,
        &root.join("scratch"),
    ]);
    assert!(!output.status.success());
    assert!(text(&output).contains("frozen toolchain"));
    fs::remove_dir_all(root).unwrap();
}
