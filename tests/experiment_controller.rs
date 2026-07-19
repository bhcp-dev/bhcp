use std::fs;
use std::path::{Path, PathBuf};

use bhcp::experiment::{
    ArmOutcome, ExperimentArm, ExperimentController, ExperimentPins, ExperimentPlan, JudgeCommand,
    RejectionReason, SessionStatus,
};

fn pins() -> ExperimentPins {
    ExperimentPins {
        model: "fake-model@1".to_owned(),
        reasoning: "medium".to_owned(),
        sandbox: "workspace-write/no-network".to_owned(),
        toolchain: "rust-1.97.1".to_owned(),
    }
}

fn fake_agent() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_bhcp-experiment-fake-agent"))
}

fn fresh_scratch(name: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/experiment-controller-tests")
        .join(name);
    if root.exists() {
        fs::remove_dir_all(&root).unwrap();
    }
    root
}

fn fake_judge() -> JudgeCommand {
    JudgeCommand::new("fake", fake_agent(), ["judge-success"])
}

#[test]
fn pins_and_run_order_are_mandatory_and_closed() {
    let mut plan = ExperimentPlan::new(
        "pilot-test",
        PathBuf::from("fixture"),
        PathBuf::from("scratch"),
        pins(),
    );
    let error = plan.validate().unwrap_err();
    assert_eq!(error.code, "BHCP7501");
    assert!(error.message.contains("at least one arm"));

    plan.arms.push(ExperimentArm::new(
        "prose",
        "prompt.txt",
        PathBuf::from("fake-agent"),
    ));
    plan.arms.push(ExperimentArm::new(
        "prose",
        "prompt.txt",
        PathBuf::from("fake-agent"),
    ));
    let error = plan.validate().unwrap_err();
    assert_eq!(error.code, "BHCP7501");
    assert!(error.message.contains("duplicate arm"));

    plan.arms.pop();
    plan.pins.model.clear();
    let error = plan.validate().unwrap_err();
    assert!(error.message.contains("model"));
}

#[test]
fn rejection_reason_values_are_closed_and_distinct() {
    let interrupted = ArmOutcome::rejected(
        "prose",
        RejectionReason::Interrupted,
        "agent exited before a complete result",
    );
    let contaminated = ArmOutcome::rejected(
        "prose",
        RejectionReason::Contaminated,
        "immutable input changed",
    );
    let adaptive = ArmOutcome::rejected(
        "prose",
        RejectionReason::AdaptiveOracle,
        "oracle appeared before agent stop",
    );
    let incomplete = ArmOutcome::rejected(
        "prose",
        RejectionReason::Incomplete,
        "token metrics missing",
    );

    assert_eq!(interrupted.status, SessionStatus::Rejected);
    assert_eq!(interrupted.rejection, Some(RejectionReason::Interrupted));
    assert_eq!(contaminated.rejection, Some(RejectionReason::Contaminated));
    assert_eq!(adaptive.rejection, Some(RejectionReason::AdaptiveOracle));
    assert_eq!(incomplete.rejection, Some(RejectionReason::Incomplete));
}

#[test]
fn controller_rejects_a_workspace_that_could_expose_the_oracle() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture = root.join("experiments/minimal-coding-agent");
    let scratch = fresh_scratch("preflight");
    let mut plan = ExperimentPlan::new("preflight", fixture, &scratch, pins());
    plan.arms
        .push(ExperimentArm::new("prose", "TASK.md", fake_agent()));
    plan.judges.push(JudgeCommand::new(
        "public",
        PathBuf::from(env!("CARGO")),
        ["test", "--offline", "--manifest-path", "subject/Cargo.toml"],
    ));
    plan.oracle_source = Some(root.join("experiments/minimal-coding-agent/oracle"));

    let controller = ExperimentController::new();
    let report = controller.run(&plan).unwrap();
    assert_eq!(report.arms.len(), 1);
    assert_ne!(report.arms[0].status, SessionStatus::Accepted);
    assert_eq!(report.arms[0].rejection, Some(RejectionReason::Incomplete));
    assert!(report.to_markdown().contains("fake-model@1"));
    fs::remove_dir_all(scratch).unwrap();
}

#[test]
fn controller_observes_every_fail_closed_session_transition() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for (mode, expected) in [
        ("interrupted", RejectionReason::Interrupted),
        ("incomplete", RejectionReason::Incomplete),
        ("adaptive", RejectionReason::AdaptiveOracle),
        ("contaminated", RejectionReason::Contaminated),
        ("empty-contamination", RejectionReason::Contaminated),
        ("pin-mismatch", RejectionReason::Contaminated),
        ("overflow", RejectionReason::Incomplete),
        ("timeout", RejectionReason::Interrupted),
    ] {
        let scratch = fresh_scratch(mode);
        let mut plan = ExperimentPlan::new(
            mode,
            root.join("experiments/minimal-coding-agent"),
            &scratch,
            pins(),
        );
        let mut arm = ExperimentArm::new("prose", "TASK.md", fake_agent());
        arm.arguments.push(mode.to_owned());
        plan.arms.push(arm);
        plan.judges.push(fake_judge());
        if mode == "overflow" {
            plan.limits.max_agent_output_bytes = 128;
        }
        if mode == "timeout" {
            plan.limits.timeout_millis = 20;
        }

        let report = ExperimentController::new().run(&plan).unwrap();
        assert_eq!(report.arms[0].status, SessionStatus::Rejected);
        assert_eq!(report.arms[0].rejection, Some(expected));
        fs::remove_dir_all(scratch).unwrap();
    }
}

#[test]
fn judge_cannot_mutate_the_frozen_candidate() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let scratch = fresh_scratch("judge-mutation");
    let mut plan = ExperimentPlan::new(
        "judge-mutation",
        root.join("experiments/minimal-coding-agent"),
        &scratch,
        pins(),
    );
    let mut arm = ExperimentArm::new("prose", "TASK.md", fake_agent());
    arm.arguments.push("complete".to_owned());
    plan.arms.push(arm);
    plan.judges.push(JudgeCommand::new(
        "mutating-judge",
        fake_agent(),
        ["judge-mutate"],
    ));

    let report = ExperimentController::new().run(&plan).unwrap();
    assert_eq!(
        report.arms[0].rejection,
        Some(RejectionReason::Contaminated)
    );
    assert_eq!(report.arms[0].judges.len(), 1);
    fs::remove_dir_all(scratch).unwrap();
}

#[cfg(unix)]
#[test]
fn controller_reaps_background_processes_that_keep_capture_pipes_open() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let scratch = fresh_scratch("background-process");
    let mut plan = ExperimentPlan::new(
        "background-process",
        root.join("experiments/minimal-coding-agent"),
        &scratch,
        pins(),
    );
    let mut arm = ExperimentArm::new("prose", "TASK.md", fake_agent());
    arm.arguments.push("background".to_owned());
    plan.arms.push(arm);
    plan.judges.push(fake_judge());

    let report = ExperimentController::new().run(&plan).unwrap();
    assert_eq!(report.arms[0].status, SessionStatus::Rejected);
    assert_eq!(report.arms[0].rejection, Some(RejectionReason::Incomplete));
    assert!(report.arms[0].elapsed_millis < 1_000);
    fs::remove_dir_all(scratch).unwrap();
}

#[cfg(unix)]
#[test]
fn controller_reaps_background_processes_even_when_capture_pipes_are_closed() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let scratch = fresh_scratch("background-process-closed");
    let mut plan = ExperimentPlan::new(
        "background-process-closed",
        root.join("experiments/minimal-coding-agent"),
        &scratch,
        pins(),
    );
    let mut arm = ExperimentArm::new("prose", "TASK.md", fake_agent());
    arm.arguments.push("background-closed".to_owned());
    plan.arms.push(arm);
    plan.judges.push(fake_judge());

    let report = ExperimentController::new().run(&plan).unwrap();
    assert_eq!(report.arms[0].status, SessionStatus::Rejected);
    assert_eq!(report.arms[0].rejection, Some(RejectionReason::Incomplete));
    assert!(report.arms[0].elapsed_millis < 1_000);
    fs::remove_dir_all(scratch).unwrap();
}

#[test]
fn output_limit_stops_a_flooding_process_before_the_timeout() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let scratch = fresh_scratch("overflow-immediate");
    let mut plan = ExperimentPlan::new(
        "overflow-immediate",
        root.join("experiments/minimal-coding-agent"),
        &scratch,
        pins(),
    );
    plan.limits.max_agent_output_bytes = 128;
    plan.limits.timeout_millis = 5_000;
    let mut arm = ExperimentArm::new("prose", "TASK.md", fake_agent());
    arm.arguments.push("overflow-slow".to_owned());
    plan.arms.push(arm);
    plan.judges.push(fake_judge());

    let report = ExperimentController::new().run(&plan).unwrap();
    assert_eq!(report.arms[0].rejection, Some(RejectionReason::Incomplete));
    assert!(report.arms[0].elapsed_millis < 1_000);
    fs::remove_dir_all(scratch).unwrap();
}

#[test]
fn target_named_candidate_content_is_never_hidden_from_contamination_checks() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let scratch = fresh_scratch("target-content");
    let mut plan = ExperimentPlan::new(
        "target-content",
        root.join("experiments/minimal-coding-agent"),
        &scratch,
        pins(),
    );
    let mut arm = ExperimentArm::new("prose", "TASK.md", fake_agent());
    arm.arguments.push("hidden-target".to_owned());
    plan.arms.push(arm);
    plan.judges.push(fake_judge());

    let report = ExperimentController::new().run(&plan).unwrap();
    assert_eq!(
        report.arms[0].rejection,
        Some(RejectionReason::Contaminated)
    );
    fs::remove_dir_all(scratch).unwrap();
}

#[cfg(unix)]
#[test]
fn preplanted_plan_symlink_cannot_escape_the_scratch_root() {
    use std::os::unix::fs::symlink;

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let scratch = fresh_scratch("scratch-symlink");
    let outside = fresh_scratch("scratch-symlink-outside");
    fs::create_dir_all(&scratch).unwrap();
    fs::create_dir_all(&outside).unwrap();
    symlink(&outside, scratch.join("scratch-symlink-plan")).unwrap();
    let mut plan = ExperimentPlan::new(
        "scratch-symlink-plan",
        root.join("experiments/minimal-coding-agent"),
        &scratch,
        pins(),
    );
    plan.arms
        .push(ExperimentArm::new("prose", "TASK.md", fake_agent()));
    plan.judges.push(fake_judge());

    let error = ExperimentController::new().run(&plan).unwrap_err();
    assert!(error.message.contains("already exists"));
    assert!(fs::read_dir(&outside).unwrap().next().is_none());
    fs::remove_dir_all(scratch).unwrap();
    fs::remove_dir_all(outside).unwrap();
}

#[test]
fn judge_environment_is_closed_and_minimal() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let scratch = fresh_scratch("judge-environment");
    let mut plan = ExperimentPlan::new(
        "judge-environment",
        root.join("experiments/minimal-coding-agent"),
        &scratch,
        pins(),
    );
    let mut arm = ExperimentArm::new("prose", "TASK.md", fake_agent());
    arm.arguments.push("complete".to_owned());
    plan.arms.push(arm);
    plan.judges.push(JudgeCommand::new(
        "clean-environment",
        fake_agent(),
        ["judge-env-clean"],
    ));

    let report = ExperimentController::new().run(&plan).unwrap();
    assert_eq!(report.arms[0].status, SessionStatus::Accepted);
    fs::remove_dir_all(scratch).unwrap();
}

#[test]
fn only_oracle_judges_receive_the_withheld_oracle() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture = root.join("experiments/minimal-coding-agent");
    let scratch = fresh_scratch("oracle-visibility");
    let mut plan = ExperimentPlan::new("oracle-visibility", &fixture, &scratch, pins());
    let mut arm = ExperimentArm::new("prose", "TASK.md", fake_agent());
    arm.arguments.push("complete".to_owned());
    plan.arms.push(arm);
    plan.oracle_source = Some(fixture.join("oracle"));
    plan.judges.push(JudgeCommand::new(
        "public",
        fake_agent(),
        ["judge-expect-no-oracle"],
    ));
    plan.judges
        .push(JudgeCommand::new("oracle", fake_agent(), ["judge-expect-oracle"]).with_oracle());

    let report = ExperimentController::new().run(&plan).unwrap();
    assert_eq!(report.arms[0].status, SessionStatus::Accepted);
    fs::remove_dir_all(scratch).unwrap();
}

#[test]
fn tree_digest_is_structurally_unambiguous() {
    let root = fresh_scratch("digest-framing");
    let fixture_one = root.join("one");
    let fixture_two = root.join("two");
    fs::create_dir_all(fixture_one.join("subject")).unwrap();
    fs::create_dir_all(fixture_two.join("subject")).unwrap();
    fs::write(fixture_one.join("subject/a"), b"x\xffsubject/b\0y" as &[u8]).unwrap();
    fs::write(fixture_two.join("subject/a"), b"x").unwrap();
    fs::write(fixture_two.join("subject/b"), b"y").unwrap();

    let mut first =
        ExperimentPlan::new("digest-one", &fixture_one, root.join("scratch-one"), pins());
    first
        .arms
        .push(ExperimentArm::new("prose", "TASK.md", fake_agent()));
    let mut second =
        ExperimentPlan::new("digest-two", &fixture_two, root.join("scratch-two"), pins());
    second
        .arms
        .push(ExperimentArm::new("prose", "TASK.md", fake_agent()));

    assert_ne!(
        first.freeze().unwrap().fixture_digest,
        second.freeze().unwrap().fixture_digest
    );
    fs::remove_dir_all(root).unwrap();
}

fn fixture_plan(name: &str, fixture: &Path) -> (ExperimentPlan, PathBuf) {
    let scratch = fresh_scratch(name);
    let mut plan = ExperimentPlan::new(name, fixture, &scratch, pins());
    let mut arm = ExperimentArm::new("prose", "TASK.md", fake_agent());
    arm.arguments.push("complete".to_owned());
    plan.arms.push(arm);
    plan.oracle_source = Some(fixture.join("oracle"));
    plan.judges.push(JudgeCommand::new(
        "public",
        PathBuf::from(env!("CARGO")),
        ["test", "--offline", "--manifest-path", "subject/Cargo.toml"],
    ));
    plan.judges.push(
        JudgeCommand::new(
            "oracle",
            PathBuf::from(env!("CARGO")),
            ["test", "--offline", "--manifest-path", "oracle/Cargo.toml"],
        )
        .with_oracle(),
    );
    (plan, scratch)
}

#[test]
fn fake_agent_is_judged_symmetrically_on_both_checked_in_rust_fixtures() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for (name, relative) in [
        ("minimal-fixture", "experiments/minimal-coding-agent"),
        ("policy-fixture", "experiments/policy-resolution-agent"),
    ] {
        let fixture = root.join(relative);
        let (plan, scratch) = fixture_plan(name, &fixture);
        let report = ExperimentController::new().run(&plan).unwrap();
        let outcome = &report.arms[0];

        assert_eq!(outcome.status, SessionStatus::Rejected);
        assert_eq!(outcome.rejection, Some(RejectionReason::VerificationFailed));
        assert_eq!(outcome.judges.len(), 2);
        assert!(outcome.judges[0].accepted, "{:?}", outcome.judges);
        assert!(!outcome.judges[1].accepted);
        assert!(outcome.metrics.is_some());
        assert!(!outcome.input_digests.is_empty());
        assert!(
            outcome
                .agent_executable_digest
                .starts_with("bhcp.hash/sha3-512@0:")
        );
        assert!(report.plan_digest.starts_with("bhcp.hash/sha3-512@0:"));
        assert!(report.fixture_digest.starts_with("bhcp.hash/sha3-512@0:"));
        assert_eq!(outcome.agent_command[1], "complete");
        assert!(outcome.agent_elapsed_millis <= outcome.elapsed_millis);
        assert_eq!(outcome.judges[0].command[1], "test");
        assert!(
            outcome.judges[0]
                .stdout_digest
                .starts_with("bhcp.hash/sha3-512@0:")
        );
        let judge_views = scratch.join(name).join("judge-views/prose");
        assert!(!judge_views.join("public/oracle").exists());
        assert!(judge_views.join("oracle/oracle").is_dir());
        let markdown = report.to_markdown();
        assert!(markdown.contains("Agent command:"));
        assert!(markdown.contains("- Input `TASK.md`:"));
        assert!(!markdown.contains("{"));
        fs::remove_dir_all(scratch).unwrap();
    }
}

#[test]
fn plan_fingerprint_is_stable_and_commits_to_pins() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture = root.join("experiments/minimal-coding-agent");
    let first_scratch = fresh_scratch("stable-plan");
    let mut first = ExperimentPlan::new("stable-plan", fixture, &first_scratch, pins());
    let mut arm = ExperimentArm::new("prose", "TASK.md", fake_agent());
    arm.arguments.push("complete".to_owned());
    first.arms.push(arm);
    let mut raw = ExperimentArm::new("raw", "TASK.md", fake_agent());
    raw.arguments.push("complete".to_owned());
    raw.contract_files.push(PathBuf::from("contract.bhcp"));
    first.arms.push(raw);
    first.judges.push(fake_judge());
    let pre_registered = first.freeze().unwrap();
    let mut reordered = first.clone();
    reordered.arms.reverse();
    assert_ne!(
        pre_registered.plan_digest,
        reordered.freeze().unwrap().plan_digest
    );
    let mut second = first.clone();
    let second_scratch = fresh_scratch("stable-plan-copy");
    second.scratch_root = second_scratch.clone();

    let first_report = ExperimentController::new().run(&first).unwrap();
    let second_report = ExperimentController::new().run(&second).unwrap();
    assert_eq!(first_report.plan_digest, second_report.plan_digest);
    assert_eq!(pre_registered.plan_digest, first_report.plan_digest);
    assert_eq!(pre_registered.fixture_digest, first_report.fixture_digest);
    assert_eq!(first_report.run_order, ["prose", "raw"]);

    let mut changed = second;
    let changed_scratch = fresh_scratch("stable-plan-changed");
    changed.scratch_root = changed_scratch.clone();
    changed.pins.model = "fake-model@2".to_owned();
    let changed_report = ExperimentController::new().run(&changed).unwrap();
    assert_ne!(first_report.plan_digest, changed_report.plan_digest);

    let summary = changed_scratch.join("summary.md");
    changed_report.write_markdown(&summary).unwrap();
    let markdown = fs::read_to_string(&summary).unwrap();
    assert!(markdown.contains(&changed_report.plan_digest));
    assert!(!markdown.contains("\"model\":"));
    assert!(changed_report.write_markdown(&summary).is_err());

    fs::remove_dir_all(first_scratch).unwrap();
    fs::remove_dir_all(second_scratch).unwrap();
    fs::remove_dir_all(changed_scratch).unwrap();
}
