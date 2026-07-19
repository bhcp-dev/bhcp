use std::fs;
#[cfg(unix)]
use std::net::{TcpListener, TcpStream};
#[cfg(unix)]
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

use bhcp::adapter::{
    AdapterRequest, CancellationToken, MAX_ADAPTER_INPUT_BYTES, VerifierProcessRunner,
};
use bhcp::cbor::decode_deterministic;
use bhcp::hash::HashAlgorithm;
use bhcp::manifest::{VerifierAdapterDeclaration, WorkingScope};
use bhcp::model::ContentReference;
use bhcp::verification::{VerifierConclusion, VerifierExecution};
#[cfg(unix)]
use nix::fcntl::{FcntlArg, FdFlag, fcntl};

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
    static OBLIGATIONS: OnceLock<Vec<String>> = OnceLock::new();
    static EFFECTS: OnceLock<Vec<String>> = OnceLock::new();
    AdapterRequest {
        verifier: "example/verifier.fixture@0",
        obligations: OBLIGATIONS.get_or_init(|| vec!["clause-2".to_owned(), "clause-3".to_owned()]),
        payload,
        subject: subject_reference(),
        subject_bytes: b"subject",
        effect_ceiling: EFFECTS.get_or_init(|| vec!["bhcp-effect/process@0".to_owned()]),
    }
}

fn subject_reference() -> &'static ContentReference {
    static SUBJECT: OnceLock<ContentReference> = OnceLock::new();
    SUBJECT.get_or_init(|| {
        ContentReference::from_bytes("application/test", b"subject", HashAlgorithm::default())
    })
}

fn runner(project_root: &Path) -> VerifierProcessRunner {
    assert!(Path::new(env!("CARGO_BIN_EXE_bhcp-adapter-sandbox")).is_file());
    VerifierProcessRunner::new(project_root).unwrap()
}

#[test]
fn exact_registration_targets_and_argv_are_retained_without_shell_or_path_lookup() {
    let project = TestProject::new();
    let runner = runner(&project.root);
    let mut declaration = declaration("accepted");
    declaration.argv.push("; touch injected".to_owned());
    let run = runner
        .run(
            &declaration,
            request(b"candidate"),
            &CancellationToken::new(),
        )
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
    assert!(
        run.record
            .executable_artifact
            .as_ref()
            .unwrap()
            .validate()
            .is_ok()
    );
    assert!(run.record.request_artifact.validate().is_ok());
    assert!(
        run.record
            .response_artifact
            .as_ref()
            .unwrap()
            .validate()
            .is_ok()
    );
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
    assert_eq!(
        encoded_request.get("subject"),
        Some(&subject_reference().to_value())
    );
    assert_eq!(
        encoded_request.get("subject_content"),
        Some(&bhcp::value::Value::Bytes(b"subject".to_vec()))
    );
}

#[test]
fn request_target_must_match_the_exact_registered_symbol() {
    let project = TestProject::new();
    let runner = runner(&project.root);
    let obligations = vec!["clause-2".to_owned()];
    let effects = vec!["bhcp-effect/process@0".to_owned()];
    let mismatched = AdapterRequest {
        verifier: "example/another-verifier@0",
        obligations: &obligations,
        payload: b"candidate",
        subject: subject_reference(),
        subject_bytes: b"subject",
        effect_ceiling: &effects,
    };

    let error = runner
        .run(
            &declaration("accepted"),
            mismatched,
            &CancellationToken::new(),
        )
        .unwrap_err();

    assert_eq!(
        error.to_string(),
        "<artifact>:1:1: BHCP7001: adapter request does not match the exact registration symbol"
    );
}

#[test]
fn accepted_rejected_unresolved_and_faulted_results_remain_distinct() {
    let project = TestProject::new();
    let runner = runner(&project.root);
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
    let runner = runner(&project.root);

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
        fs::copy(env!("CARGO_BIN_EXE_bhcp-verifier-fixture"), &outside).unwrap();
        std::os::unix::fs::symlink(&outside, project.root.join("tools/escape")).unwrap();
        let mut escaped = declaration("accepted");
        escaped.executable = PathBuf::from("tools/escape");
        assert_fault(
            runner
                .run(&escaped, request(b"candidate"), &CancellationToken::new())
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
    let runner = runner(&project.root);
    let mut timed = declaration("sleep");
    timed.timeout_ms = 30;
    assert_unresolved(
        runner
            .run(&timed, request(b"candidate"), &CancellationToken::new())
            .unwrap()
            .execution,
        "bhcp.reason/adapter-timeout@0",
    );

    let mut descendant = declaration("descendant");
    descendant.timeout_ms = 30;
    assert_unresolved(
        runner
            .run(
                &descendant,
                request(b"candidate"),
                &CancellationToken::new(),
            )
            .unwrap()
            .execution,
        "bhcp.reason/adapter-timeout@0",
    );

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
    let runner = runner(&project.root);
    let declaration = declaration("accepted");

    let oversized = vec![0; MAX_ADAPTER_INPUT_BYTES + 1];
    let diagnostic = runner
        .run(&declaration, request(&oversized), &CancellationToken::new())
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
                subject: subject_reference(),
                subject_bytes: b"subject",
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
                obligations: &["clause-2".to_owned()],
                payload: b"candidate",
                subject: subject_reference(),
                subject_bytes: b"another subject",
                effect_ceiling: &["bhcp-effect/process@0".to_owned()],
            },
            &CancellationToken::new(),
        )
        .unwrap_err();
    assert!(diagnostic.message.contains("subject bytes"));

    let diagnostic = runner
        .run(
            &declaration,
            AdapterRequest {
                verifier: "example/verifier.fixture@0",
                obligations: &["clause-3".to_owned(), "clause-2".to_owned()],
                payload: b"candidate",
                subject: subject_reference(),
                subject_bytes: b"subject",
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
                subject: subject_reference(),
                subject_bytes: b"subject",
                effect_ceiling: &[],
            },
            &CancellationToken::new(),
        )
        .unwrap_err();
    assert!(diagnostic.message.contains("effect ceiling"));
}

#[test]
fn os_sandbox_denies_network_and_undeclared_filesystem_access() {
    let project = TestProject::new();
    let runner = runner(&project.root);

    assert_accepted(
        runner
            .run(
                &declaration("network-denied"),
                request(b"candidate"),
                &CancellationToken::new(),
            )
            .unwrap()
            .execution,
    );

    #[cfg(unix)]
    {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (inherited, _) = listener.accept().unwrap();
        fcntl(&inherited, FcntlArg::F_SETFD(FdFlag::empty())).unwrap();
        let mut descriptor = declaration("fd-denied");
        descriptor.argv.push(inherited.as_raw_fd().to_string());
        assert_accepted(
            runner
                .run(
                    &descriptor,
                    request(b"candidate"),
                    &CancellationToken::new(),
                )
                .unwrap()
                .execution,
        );
        drop(client);
        drop(inherited);
    }
    assert_accepted(
        runner
            .run(
                &declaration("exec-denied"),
                request(b"candidate"),
                &CancellationToken::new(),
            )
            .unwrap()
            .execution,
    );

    let outside = project.root.parent().unwrap().join(format!(
        "bhcp-adapter-secret-{}",
        NEXT_PROJECT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::write(&outside, b"secret").unwrap();
    let mut denied_read = declaration("read-denied");
    denied_read.argv.push(outside.display().to_string());
    assert_accepted(
        runner
            .run(
                &denied_read,
                request(b"candidate"),
                &CancellationToken::new(),
            )
            .unwrap()
            .execution,
    );
    fs::remove_file(outside).unwrap();

    let inside = project.root.join("candidate.txt");
    fs::write(&inside, b"candidate").unwrap();
    let mut allowed_read = declaration("read-allowed");
    allowed_read.argv.push(inside.display().to_string());
    allowed_read.allowed_effects = vec![
        "bhcp-effect/fs.read@0".to_owned(),
        "bhcp-effect/process@0".to_owned(),
    ];
    let effects = allowed_read.allowed_effects.clone();
    assert_accepted(
        runner
            .run(
                &allowed_read,
                AdapterRequest {
                    verifier: &allowed_read.symbol,
                    obligations: &["clause-2".to_owned()],
                    payload: b"candidate",
                    subject: subject_reference(),
                    subject_bytes: b"subject",
                    effect_ceiling: &effects,
                },
                &CancellationToken::new(),
            )
            .unwrap()
            .execution,
    );

    let denied_write_path = project.root.join("denied-write.txt");
    let mut denied_write = declaration("write-denied");
    denied_write
        .argv
        .push(denied_write_path.display().to_string());
    assert_accepted(
        runner
            .run(
                &denied_write,
                request(b"candidate"),
                &CancellationToken::new(),
            )
            .unwrap()
            .execution,
    );
    assert!(!denied_write_path.exists());

    let allowed_write_path = project.root.join("allowed-write.txt");
    let mut allowed_write = declaration("write-allowed");
    allowed_write
        .argv
        .push(allowed_write_path.display().to_string());
    allowed_write.allowed_effects = vec![
        "bhcp-effect/fs.write@0".to_owned(),
        "bhcp-effect/process@0".to_owned(),
    ];
    let effects = allowed_write.allowed_effects.clone();
    assert_accepted(
        runner
            .run(
                &allowed_write,
                AdapterRequest {
                    verifier: &allowed_write.symbol,
                    obligations: &["clause-2".to_owned()],
                    payload: b"candidate",
                    subject: subject_reference(),
                    subject_bytes: b"subject",
                    effect_ceiling: &effects,
                },
                &CancellationToken::new(),
            )
            .unwrap()
            .execution,
    );
    assert_eq!(fs::read(allowed_write_path).unwrap(), b"adapter write");
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

fn assert_accepted(execution: VerifierExecution) {
    assert!(matches!(
        execution,
        VerifierExecution::Completed(VerifierConclusion::Accepted(_))
    ));
}
