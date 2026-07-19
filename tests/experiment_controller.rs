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
    JudgeCommand::new("fake", fake_agent().to_string_lossy(), ["complete"])
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
        "cargo",
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

        let report = ExperimentController::new().run(&plan).unwrap();
        assert_eq!(report.arms[0].status, SessionStatus::Rejected);
        assert_eq!(report.arms[0].rejection, Some(expected));
        fs::remove_dir_all(scratch).unwrap();
    }
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
        env!("CARGO"),
        ["test", "--offline", "--manifest-path", "subject/Cargo.toml"],
    ));
    plan.judges.push(JudgeCommand::new(
        "oracle",
        env!("CARGO"),
        ["test", "--offline", "--manifest-path", "oracle/Cargo.toml"],
    ));
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
        assert!(outcome.judges[0].accepted);
        assert!(!outcome.judges[1].accepted);
        assert!(outcome.metrics.is_some());
        assert!(scratch.join(name).join("prose/oracle").is_dir());
        assert!(!report.to_markdown().contains("{"));
        fs::remove_dir_all(scratch).unwrap();
    }
}
