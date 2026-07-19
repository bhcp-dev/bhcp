use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn fresh_root() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "bhcp-in-session-forward-runner-{}",
        std::process::id()
    ));
    if root.exists() {
        fs::remove_dir_all(&root).unwrap();
    }
    fs::create_dir_all(root.join("codex-home")).unwrap();
    root
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
    assert!(selected.status.success());
    let toolchain_bin = PathBuf::from(String::from_utf8(selected.stdout).unwrap().trim())
        .parent()
        .unwrap()
        .to_owned();
    (cargo_home, rustup_home, rustup, toolchain_bin)
}

fn run(mode: &str, root: &Path) -> Output {
    let (cargo_home, rustup_home, rustup, toolchain_bin) = rustup_installation();
    Command::new(env!("CARGO_BIN_EXE_in_session_evidence_forward"))
        .args([
            mode,
            env!("CARGO_BIN_EXE_bhcp-experiment-fake-agent"),
            env!("CARGO_BIN_EXE_bhcp-experiment-fake-codex"),
            root.join("codex-home").to_str().unwrap(),
            cargo_home.to_str().unwrap(),
            rustup_home.to_str().unwrap(),
            env!("CARGO_BIN_EXE_bhcp"),
            rustup.to_str().unwrap(),
            toolchain_bin.to_str().unwrap(),
            env!("CARGO_MANIFEST_DIR"),
            env!("CARGO_BIN_EXE_bhcp-in-session-evidence-adapter"),
            root.join("prepared").to_str().unwrap(),
            root.join("scratch").to_str().unwrap(),
        ])
        .output()
        .unwrap()
}

#[test]
fn preparation_freezes_the_adapter_candidate_skill_and_complete_plan() {
    let root = fresh_root();
    let prepared = run("prepare-001", &root);
    assert!(
        prepared.status.success(),
        "{}",
        String::from_utf8_lossy(&prepared.stderr)
    );
    let prepared_output = String::from_utf8(prepared.stdout).unwrap();
    assert!(prepared_output.contains("experiment_id=in-session-evidence-forward-001"));
    assert!(prepared_output.contains("run_order=forward-01"));
    assert!(prepared_output.contains("sandbox=workspace-write/no-network/read-confined"));
    for relative in [
        "candidate.cbor",
        "subject/tools/in-session-evidence-adapter",
        "subject/.agents/skills/interpret-bhcp-contract/SKILL.md",
    ] {
        assert!(root.join("prepared").join(relative).is_file(), "{relative}");
    }

    let frozen = run("freeze-001", &root);
    assert!(frozen.status.success());
    assert_eq!(String::from_utf8(frozen.stdout).unwrap(), prepared_output);
    fs::remove_dir_all(root).unwrap();
}
