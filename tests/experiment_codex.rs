use bhcp::experiment_codex::summarize_events;
use std::fs;
use std::process::Command;

#[test]
fn closed_codex_event_summary_counts_only_completed_commands_and_final_usage() {
    let events = concat!(
        "{\"type\":\"thread.started\",\"thread_id\":\"t\"}\n",
        "{\"type\":\"item.completed\",\"item\":{\"type\":\"command_execution\",\"status\":\"completed\"}}\n",
        "{\"type\":\"item.completed\",\"item\":{\"type\":\"reasoning\",\"text\":\"private\"}}\n",
        "{\"type\":\"item.completed\",\"item\":{\"type\":\"command_execution\",\"status\":\"failed\"}}\n",
        "{\"type\":\"turn.completed\",\"usage\":{\"input_tokens\":120,\"cached_input_tokens\":80,\"output_tokens\":30,\"reasoning_output_tokens\":12}}\n",
    );

    let summary = summarize_events(events.as_bytes()).unwrap();
    assert_eq!(summary.input_tokens, 120);
    assert_eq!(summary.cached_input_tokens, 80);
    assert_eq!(summary.output_tokens, 30);
    assert_eq!(summary.reasoning_tokens, 12);
    assert_eq!(summary.completed_commands, 1);
}

#[test]
fn codex_event_summary_fails_closed_on_unknown_or_incomplete_usage() {
    assert!(summarize_events(b"not-json\n".as_slice()).is_err());
    assert!(
        summarize_events(
            b"{\"type\":\"turn.completed\",\"usage\":{\"input_tokens\":1}}\n".as_slice()
        )
        .is_err()
    );
    assert!(summarize_events(b"{\"type\":\"turn.started\"}\n".as_slice()).is_err());
}

#[test]
fn driver_forwards_the_controller_owned_target_to_codex() {
    let root = std::env::temp_dir().join(format!("bhcp-codex-driver-{}", std::process::id()));
    if root.exists() {
        fs::remove_dir_all(&root).unwrap();
    }
    for directory in [
        "workspace/subject",
        "target",
        "codex-home",
        "home",
        "cargo",
        "rustup",
        "tools",
    ] {
        fs::create_dir_all(root.join(directory)).unwrap();
    }
    fs::write(root.join("workspace/prompt.md"), "frozen prompt\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_bhcp_codex_experiment_driver"))
        .args([
            env!("CARGO_BIN_EXE_bhcp-experiment-fake-codex"),
            root.join("codex-home").to_str().unwrap(),
            root.join("home").to_str().unwrap(),
            root.join("cargo").to_str().unwrap(),
            root.join("rustup").to_str().unwrap(),
            root.join("tools").to_str().unwrap(),
            "1.97.1",
        ])
        .current_dir(root.join("workspace"))
        .env_clear()
        .env("BHCP_EXPERIMENT_MODEL", "test-model")
        .env("BHCP_EXPERIMENT_REASONING", "medium")
        .env("BHCP_EXPERIMENT_SANDBOX", "workspace-write/no-network")
        .env("BHCP_EXPERIMENT_TOOLCHAIN", "test-toolchain")
        .env("BHCP_EXPERIMENT_PROMPT", "prompt.md")
        .env("CARGO_TARGET_DIR", root.join("target"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("completed_commands=1")
    );
    fs::remove_dir_all(root).unwrap();
}
