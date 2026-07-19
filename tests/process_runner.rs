use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use bhcp::adapter::{
    AdapterRequest, CancellationToken, MAX_ADAPTER_INPUT_BYTES, VerifierProcessRunner,
};
use bhcp::cbor::decode_deterministic;
use bhcp::manifest::{VerifierAdapterDeclaration, WorkingScope};
use bhcp::verification::{VerifierConclusion, VerifierExecution};

static NEXT_PROJECT: AtomicUsize = AtomicUsize::new(1);

struct TestProject {
    root: PathBuf,
}

impl TestProject {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "bhcp-process-runner-{}-{}",
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

fn request(payload: &[u8]) -> AdapterRequest<'_> {
    AdapterRequest {
        verifier: "example/verifier.fixture@0",
        obligations: &["clause-2".to_owned(), "clause-3".to_owned()],
        payload,
        effect_ceiling: &["bhcp-effect/process@0".to_owned()],
    }
}

#[test]
fn exact_registration_targets_and_argv_are_retained_without_shell_or_path_lookup() {
    let project = TestProject::new();
    let runner = VerifierProcessRunner::new(&project.root).unwrap();
    let mut declaration = declaration("accepted");
    declaration.argv.push("; touch injected".to_owned());
    let run = runner
        .run(&declaration, request(b"candidate"), &CancellationToken::new())
        .unwrap();

    let VerifierExecution::Completed(VerifierConclusion::Accepted(evidence)) = &run.execution
    else {
        panic!("fixture must be accepted")
    };
    assert_eq!(evidence.evidence_class, "static");
    assert_eq!(evidence.predicate, declaration.symbol);
    assert_eq!(evidence.payload, b"fixture evidence");
    assert_eq!(run.record.declaration, declaration);
    assert_eq!(run.record.obligations, ["clause-2", "clause-3"]);
    assert!(run.record.registration_artifact.validate().is_ok());
    assert!(run.record.executable_artifact.as_ref().unwrap().validate().is_ok());
    assert!(run.record.request_artifact.validate().is_ok());
    assert!(run.record.response_artifact.as_ref().unwrap().validate().is_ok());
    assert_eq!(run.record.exit_code, Some(0));
    assert!(!project.root.join("injected").exists());

    let encoded_request = decode_deterministic(&run.record.request_bytes).unwrap();
    assert_eq!(
        encoded_request.get("verifier"),
        Some(&bhcp::value::Value::Text(declaration.symbol.clone()))
    );
    assert_eq!(
        encoded_request.get("obligations"),
        Some(&bhcp::value::Value::Array(vec![
            bhcp::value::Value::Text("clause-2".to_owned()),
            bhcp::value::Value::Text("clause-3".to_owned()),
        ]))
    );
}

#[test]
fn accepted_rejected_unresolved_and_faulted_results_remain_distinct() {
    let project = TestProject::new();
    let runner = VerifierProcessRunner::new(&project.root).unwrap();
    for (mode, expected) in [
        ("accepted", "accepted"),
        ("rejected", "rejected"),
        ("unresolved", "unresolved"),
        ("faulted", "faulted"),
    ] {
        let run = runner
            .run(
                &declaration(mode),
                request(b"candidate"),
                &CancellationToken::new(),
            )
            .unwrap();
        let actual = match run.execution {
            VerifierExecution::Completed(VerifierConclusion::Accepted(_)) => "accepted",
            VerifierExecution::Completed(VerifierConclusion::Rejected(_)) => "rejected",
            VerifierExecution::Completed(VerifierConclusion::Unresolved { .. }) => "unresolved",
            VerifierExecution::Faulted(_) => "faulted",
        };
        assert_eq!(actual, expected);
    }
}

#[test]
fn missing_escape_malformed_nonzero_and_output_limits_are_distinguishable() {
    let project = TestProject::new();
    let runner = VerifierProcessRunner::new(&project.root).unwrap();

    let mut missing = declaration("accepted");
    missing.executable = PathBuf::from("tools/missing");
    assert_fault(
        runner
            .run(&missing, request(b"candidate"), &CancellationToken::new())
            .unwrap()
            .execution,
        "bhcp.fault/adapter-executable-missing@0",
    );

    #[cfg(unix)]
    {
        let outside = project.root.parent().unwrap().join(format!(
            "outside-verifier-{}",
            NEXT_PROJECT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::copy(
            env!("CARGO_BIN_EXE_bhcp-verifier-fixture"),
            &outside,
        )
        .unwrap();
        std::os::unix::fs::symlink(&outside, project.root.join("tools/escape")).unwrap();
        let mut escaped = declaration("accepted");
        escaped.executable = PathBuf::from("tools/escape");
        assert_fault(
            runner
                .run(
                    &escaped,
                    request(b"candidate"),
                    &CancellationToken::new(),
                )
                .unwrap()
                .execution,
            "bhcp.fault/adapter-path-escape@0",
        );
        fs::remove_file(outside).unwrap();
    }

    for (mode, code) in [
        ("malformed", "bhcp.fault/adapter-malformed-output@0"),
        ("nonzero", "bhcp.fault/adapter-nonzero-exit@0"),
        ("flood", "bhcp.fault/adapter-output-limit@0"),
        ("stderr-flood", "bhcp.fault/adapter-stderr-limit@0"),
    ] {
        assert_fault(
            runner
                .run(
                    &declaration(mode),
                    request(b"candidate"),
                    &CancellationToken::new(),
                )
                .unwrap()
                .execution,
            code,
        );
    }
}

#[test]
fn timeout_and_cancellation_are_distinct_unresolved_outcomes() {
    let project = TestProject::new();
    let runner = VerifierProcessRunner::new(&project.root).unwrap();
    let mut timed = declaration("sleep");
    timed.timeout_ms = 30;
    let started = Instant::now();
    assert_unresolved(
        runner
            .run(&timed, request(b"candidate"), &CancellationToken::new())
            .unwrap()
            .execution,
        "bhcp.reason/adapter-timeout@0",
    );
    assert!(started.elapsed() < Duration::from_secs(2));

    let cancellation = CancellationToken::new();
    let trigger = cancellation.clone();
    let worker = thread::spawn(move || {
        thread::sleep(Duration::from_millis(25));
        trigger.cancel();
    });
    assert_unresolved(
        runner
            .run(&declaration("sleep"), request(b"candidate"), &cancellation)
            .unwrap()
            .execution,
        "bhcp.reason/adapter-cancelled@0",
    );
    worker.join().unwrap();
}

#[test]
fn request_and_effect_boundaries_fail_before_process_execution() {
    let project = TestProject::new();
    let runner = VerifierProcessRunner::new(&project.root).unwrap();
    let declaration = declaration("accepted");

    let oversized = vec![0; MAX_ADAPTER_INPUT_BYTES + 1];
    let diagnostic = runner
        .run(
            &declaration,
            request(&oversized),
            &CancellationToken::new(),
        )
        .unwrap_err();
    assert_eq!(diagnostic.code, "BHCP7001");
    assert!(diagnostic.message.contains("input limit"));

    let diagnostic = runner
        .run(
            &declaration,
            AdapterRequest {
                verifier: "example/verifier.other@0",
                obligations: &["clause-2".to_owned()],
                payload: b"candidate",
                effect_ceiling: &["bhcp-effect/process@0".to_owned()],
            },
            &CancellationToken::new(),
        )
        .unwrap_err();
    assert!(diagnostic.message.contains("registration symbol"));

    let diagnostic = runner
        .run(
            &declaration,
            AdapterRequest {
                verifier: "example/verifier.fixture@0",
                obligations: &["clause-3".to_owned(), "clause-2".to_owned()],
                payload: b"candidate",
                effect_ceiling: &["bhcp-effect/process@0".to_owned()],
            },
            &CancellationToken::new(),
        )
        .unwrap_err();
    assert!(diagnostic.message.contains("normalized"));

    let diagnostic = runner
        .run(
            &declaration,
            AdapterRequest {
                verifier: "example/verifier.fixture@0",
                obligations: &["clause-2".to_owned()],
                payload: b"candidate",
                effect_ceiling: &[],
            },
            &CancellationToken::new(),
        )
        .unwrap_err();
    assert!(diagnostic.message.contains("effect ceiling"));
}

fn assert_fault(execution: VerifierExecution, code: &str) {
    let VerifierExecution::Faulted(reason) = execution else {
        panic!("expected faulted execution")
    };
    assert_eq!(reason.code, code);
}

fn assert_unresolved(execution: VerifierExecution, code: &str) {
    let VerifierExecution::Completed(VerifierConclusion::Unresolved { reason, .. }) = execution
    else {
        panic!("expected unresolved execution")
    };
    assert_eq!(reason.code, code);
}
