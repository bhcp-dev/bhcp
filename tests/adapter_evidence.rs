use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use bhcp::adapter::{CancellationToken, VerifierProcessRunner};
use bhcp::cbor::decode_deterministic;
use bhcp::hash::HashAlgorithm;
use bhcp::inspection::render_artifact;
use bhcp::manifest::{VerifierAdapterDeclaration, WorkingScope};
use bhcp::model::{ClauseKind, ContentReference};
use bhcp::pipeline::{Compilation, compile_source};
use bhcp::schema::validate_root;
use bhcp::value::Value;
use bhcp::verification::{
    VerificationDecision, VerificationReport, VerificationRequest, VerificationState,
    VerifierRegistry,
};

const SOURCE: &str = r#"
§goal example/Verify@0 {
    §input repository: Text;
    §output publicPassed: Bool;

    §requires "pinned": repository == "subject@0";
    §ensures "public": publicPassed;

    §verify "process verifier": with example/verifier.fixture@0 for "public";
}
"#;

static NEXT_PROJECT: AtomicUsize = AtomicUsize::new(1);

struct TestProject {
    root: PathBuf,
}

impl TestProject {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "bhcp-adapter-evidence-{}-{}",
            std::process::id(),
            NEXT_PROJECT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("tools")).unwrap();
        let source = Path::new(env!("CARGO_BIN_EXE_bhcp-verifier-fixture"));
        let target = root.join("tools/verifier-fixture");
        fs::copy(source, &target).unwrap();
        fs::set_permissions(&target, fs::metadata(source).unwrap().permissions()).unwrap();
        Self { root }
    }
}

impl Drop for TestProject {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}

fn compilation() -> Compilation {
    compile_source(SOURCE, "adapter-evidence.bhcp").unwrap()
}

fn declaration(mode: &str) -> VerifierAdapterDeclaration {
    VerifierAdapterDeclaration {
        symbol: "example/verifier.fixture@0".to_owned(),
        executable: PathBuf::from("tools/verifier-fixture"),
        argv: vec![mode.to_owned()],
        working_scope: WorkingScope::Project,
        input_media_type: "application/vnd.bhcp.verification-request+cbor".to_owned(),
        output_media_type: "application/vnd.bhcp.verifier-result+cbor".to_owned(),
        timeout_ms: 2_000,
        allowed_effects: vec!["bhcp-effect/process@0".to_owned()],
        evidence_kind: "static".to_owned(),
    }
}

fn input() -> Value {
    Value::map([("repository", Value::Text("subject@0".to_owned()))])
}

fn output() -> Value {
    Value::map([("publicPassed", Value::Bool(true))])
}

fn reference(name: &str) -> ContentReference {
    ContentReference::from_bytes(
        "application/cbor",
        name.as_bytes(),
        HashAlgorithm::default(),
    )
}

fn registry(project: &TestProject, mode: &str) -> VerifierRegistry {
    let mut registry = VerifierRegistry::new();
    registry
        .register_adapter(
            VerifierProcessRunner::new(&project.root).unwrap(),
            declaration(mode),
            vec!["bhcp-effect/process@0".to_owned()],
            CancellationToken::new(),
        )
        .unwrap();
    registry
}

fn verify(
    compilation: &Compilation,
    registry: &VerifierRegistry,
    produced_at: &str,
) -> VerificationReport {
    let input = input();
    let output = output();
    registry
        .verify(VerificationRequest {
            compilation,
            goal: "example/Verify@0",
            input: &input,
            output: &output,
            subject: reference("subject"),
            execution_graph: reference("execution-graph"),
            produced_at,
        })
        .unwrap()
}

fn process_targets(compilation: &Compilation) -> Vec<String> {
    compilation.ir.goals[0]
        .clauses
        .iter()
        .find_map(|clause| match &clause.kind {
            ClauseKind::Verify { obligations, .. } => Some(obligations.clone()),
            _ => None,
        })
        .unwrap()
}

#[test]
fn process_evidence_uses_only_resolved_targets_and_retains_adapter_provenance() {
    let project = TestProject::new();
    let compilation = compilation();
    let targets = process_targets(&compilation);
    let report = verify(
        &compilation,
        &registry(&project, "accepted"),
        "2026-07-19T06:00:00Z",
    );

    assert_eq!(
        report.state,
        VerificationState::Completed(VerificationDecision::Accepted)
    );
    assert_eq!(report.adapter_records.len(), 1);
    let record = &report.adapter_records[0];
    assert_eq!(record.obligations, targets);

    let item = report
        .bundle
        .items
        .iter()
        .find(|item| item.verifier == "example/verifier.fixture@0")
        .unwrap();
    assert_eq!(item.produced_at, "2026-07-19T06:00:00Z");
    assert_eq!(
        item.provenance_source.as_ref(),
        Some(&record.registration_artifact)
    );
    assert_eq!(
        Some(&item.verifier_artifact),
        record.executable_artifact.as_ref()
    );
    let attached: Vec<_> = item
        .claims
        .iter()
        .map(|id| {
            report
                .bundle
                .claims
                .iter()
                .find(|claim| &claim.id == id)
                .unwrap()
                .obligation
                .clone()
        })
        .collect();
    assert_eq!(attached, targets);

    let request = decode_deterministic(&record.request_bytes).unwrap();
    let Value::Bytes(payload) = request.get("payload").unwrap() else {
        panic!("adapter payload must be bytes")
    };
    let candidate = decode_deterministic(payload).unwrap();
    assert_eq!(
        candidate,
        Value::map([("input", input()), ("output", output())])
    );
    report.bundle.validate().unwrap();
    validate_root(&report.bundle.to_value(true), "evidence-bundle").unwrap();
}

#[test]
fn fixed_boundary_inputs_are_byte_deterministic_and_timestamp_changes_are_artifactual() {
    let project = TestProject::new();
    let compilation = compilation();
    let first = verify(
        &compilation,
        &registry(&project, "accepted"),
        "2026-07-19T06:00:00Z",
    );
    let second = verify(
        &compilation,
        &registry(&project, "accepted"),
        "2026-07-19T06:00:00Z",
    );
    let later = verify(
        &compilation,
        &registry(&project, "accepted"),
        "2026-07-19T06:00:01Z",
    );

    assert_eq!(first.bundle_bytes, second.bundle_bytes);
    assert_eq!(first.bundle_hash, second.bundle_hash);
    assert_eq!(first.payloads, second.payloads);
    assert_ne!(first.bundle_bytes, later.bundle_bytes);
    assert_ne!(first.bundle_hash, later.bundle_hash);

    let mut without_times = first.bundle.clone();
    let mut later_without_times = later.bundle.clone();
    without_times.artifact_id = None;
    later_without_times.artifact_id = None;
    for item in &mut without_times.items {
        item.produced_at.clear();
    }
    for item in &mut later_without_times.items {
        item.produced_at.clear();
    }
    assert_eq!(without_times, later_without_times);
}

#[test]
fn missing_rejected_unresolved_faulted_and_tampered_results_stay_distinct_in_cbor_and_inspection() {
    let project = TestProject::new();
    let compilation = compilation();

    let missing = verify(
        &compilation,
        &VerifierRegistry::new(),
        "2026-07-19T06:00:00Z",
    );
    let rejected = verify(
        &compilation,
        &registry(&project, "rejected"),
        "2026-07-19T06:00:00Z",
    );
    let unresolved = verify(
        &compilation,
        &registry(&project, "unresolved"),
        "2026-07-19T06:00:00Z",
    );
    let faulted = verify(
        &compilation,
        &registry(&project, "faulted"),
        "2026-07-19T06:00:00Z",
    );
    let tampered = verify(
        &compilation,
        &registry(&project, "malformed"),
        "2026-07-19T06:00:00Z",
    );

    assert_eq!(
        missing.state,
        VerificationState::Completed(VerificationDecision::Unresolved)
    );
    assert_eq!(
        rejected.state,
        VerificationState::Completed(VerificationDecision::Rejected)
    );
    assert_eq!(
        unresolved.state,
        VerificationState::Completed(VerificationDecision::Unresolved)
    );
    assert!(matches!(faulted.state, VerificationState::Faulted(_)));
    assert!(matches!(tampered.state, VerificationState::Faulted(_)));

    for report in [&missing, &rejected, &unresolved, &faulted, &tampered] {
        report.bundle.validate().unwrap();
        validate_root(&report.bundle.to_value(true), "evidence-bundle").unwrap();
    }

    let missing_text = render_artifact(&missing.bundle.to_value(true), None);
    let rejected_text = render_artifact(&rejected.bundle.to_value(true), None);
    let unresolved_text = render_artifact(&unresolved.bundle.to_value(true), None);
    let faulted_text = render_artifact(&faulted.bundle.to_value(true), None);
    let tampered_text = render_artifact(&tampered.bundle.to_value(true), None);
    assert!(missing_text.contains("verifier-unregistered"));
    assert!(rejected_text.contains("rejected"));
    assert!(unresolved_text.contains("example/unresolved.fixture@0"));
    assert!(faulted_text.contains("example/faulted.fixture@0"));
    assert!(tampered_text.contains("bhcp.fault/adapter-malformed-output@0"));
    assert_ne!(missing.bundle_bytes, unresolved.bundle_bytes);
    assert_ne!(unresolved.bundle_bytes, faulted.bundle_bytes);
    assert_ne!(faulted.bundle_bytes, tampered.bundle_bytes);
}
