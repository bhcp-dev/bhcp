use bhcp::hash::HashAlgorithm;
use bhcp::kernel::Reason;
use bhcp::model::{BhcpType, ClauseKind, ContentReference, FieldType};
use bhcp::pipeline::{Compilation, compile_source};
use bhcp::schema::validate_root;
use bhcp::value::Value;
use bhcp::verification::{
    VerificationDecision, VerificationRequest, VerificationState, Verifier, VerifierConclusion,
    VerifierContext, VerifierEvidence, VerifierExecution, VerifierRegistry,
};

const SOURCE: &str = r#"
§goal experiment/Verify@0 {
    §input repository: Text;
    §output publicPassed: Bool;
    §output oraclePassed: Bool;

    §requires "pinned": repository == "subject@0";
    §ensures "public": publicPassed;
    §ensures "oracle": oraclePassed;

    §verify "public verifier": with experiment/verifier.public@0 for "public";
    §verify "oracle verifier": with experiment/verifier.oracle@0 for "oracle";
}
"#;

#[derive(Clone)]
struct FakeVerifier {
    symbol: &'static str,
    execution: VerifierExecution,
}

struct PanicVerifier;

impl Verifier for PanicVerifier {
    fn symbol(&self) -> &str {
        "experiment/verifier.oracle@0"
    }

    fn artifact(&self) -> ContentReference {
        reference("panic-verifier")
    }

    fn verify(&self, _context: &VerifierContext<'_>) -> VerifierExecution {
        panic!("adapter defect")
    }
}

impl Verifier for FakeVerifier {
    fn symbol(&self) -> &str {
        self.symbol
    }

    fn artifact(&self) -> ContentReference {
        ContentReference::from_bytes(
            "application/vnd.bhcp.verifier",
            self.symbol.as_bytes(),
            HashAlgorithm::default(),
        )
    }

    fn verify(&self, _context: &VerifierContext<'_>) -> VerifierExecution {
        self.execution.clone()
    }
}

fn evidence(predicate: &str, payload: &str) -> VerifierEvidence {
    VerifierEvidence::new(
        "empirical",
        predicate,
        "text/plain",
        payload.as_bytes().to_vec(),
        vec!["experiment/trust.local@0".to_owned()],
    )
}

fn accepted(symbol: &'static str) -> FakeVerifier {
    FakeVerifier {
        symbol,
        execution: VerifierExecution::Completed(VerifierConclusion::Accepted(evidence(
            symbol, "accepted",
        ))),
    }
}

fn compiled() -> Compilation {
    compile_source(SOURCE, "verification.bhcp").unwrap()
}

fn reference(name: &str) -> ContentReference {
    ContentReference::from_bytes(
        "application/cbor",
        name.as_bytes(),
        HashAlgorithm::default(),
    )
}

fn input() -> Value {
    Value::map([("repository", Value::Text("subject@0".to_owned()))])
}

fn output(public: bool, oracle: bool) -> Value {
    Value::map([
        ("publicPassed", Value::Bool(public)),
        ("oraclePassed", Value::Bool(oracle)),
    ])
}

fn verify(
    compilation: &Compilation,
    registry: &VerifierRegistry,
    candidate: &Value,
) -> bhcp::diagnostic::Result<bhcp::verification::VerificationReport> {
    registry.verify(VerificationRequest {
        compilation,
        goal: "experiment/Verify@0",
        execution_instance: None,
        input: &input(),
        output: candidate,
        subject: reference("subject"),
        subject_bytes: b"subject",
        execution_graph: reference("execution-graph"),
        produced_at: "2026-07-18T10:00:00Z",
    })
}

#[test]
fn verifier_targets_resolve_to_structural_obligations_and_normalize_order() {
    let source = SOURCE.replace("for \"public\";", "for \"oracle\", \"public\";");
    let renamed = SOURCE
        .replace("\"public\": publicPassed", "\"visible\": publicPassed")
        .replace("for \"public\";", "for \"oracle\", \"visible\";");
    let first = compile_source(&source, "first.bhcp").unwrap();
    let second = compile_source(&renamed, "second.bhcp").unwrap();

    assert_eq!(first.semantic_hash, second.semantic_hash);
    assert_ne!(first.ast_hash, second.ast_hash);
    let ClauseKind::Verify {
        binding,
        obligations,
    } = &first.ir.goals[0].clauses[6].kind
    else {
        panic!("expected verifier clause")
    };
    assert_eq!(obligations, &["clause-5", "clause-6"]);
    assert_eq!(
        binding.input,
        BhcpType::Record(vec![
            FieldType {
                name: "input".to_owned(),
                value_type: first.ir.goals[0].input.clone(),
            },
            FieldType {
                name: "output".to_owned(),
                value_type: first.ir.goals[0].output.clone(),
            },
        ])
    );
    assert_eq!(binding.output, BhcpType::Evidence(vec![]));
    assert!(binding.trust.is_empty());
}

#[test]
fn verifier_target_diagnostics_are_stable() {
    let unknown = SOURCE.replace("for \"public\"", "for \"missing\"");
    let diagnostic = compile_source(&unknown, "unknown.bhcp").unwrap_err();
    assert_eq!(diagnostic.code, "BHCP2001");
    assert_eq!(
        diagnostic.message,
        "unresolved obligation label \"missing\""
    );

    let duplicate = SOURCE.replace(
        "§ensures \"oracle\": oraclePassed;",
        "§ensures \"public\": oraclePassed;",
    );
    let diagnostic = compile_source(&duplicate, "duplicate.bhcp").unwrap_err();
    assert_eq!(diagnostic.code, "BHCP2002");

    let fact = SOURCE
        .replace(
            "§input repository: Text;",
            "§input \"repository fact\": repository: Text;",
        )
        .replace("for \"public\"", "for \"repository fact\"");
    let diagnostic = compile_source(&fact, "fact.bhcp").unwrap_err();
    assert_eq!(diagnostic.code, "BHCP2003");
}

#[test]
fn registered_verifiers_emit_deterministic_accepted_evidence() {
    let compilation = compiled();
    let mut registry = VerifierRegistry::new();
    registry
        .register(accepted("experiment/verifier.public@0"))
        .unwrap();
    registry
        .register(accepted("experiment/verifier.oracle@0"))
        .unwrap();

    let first = verify(&compilation, &registry, &output(true, true)).unwrap();
    let second = verify(&compilation, &registry, &output(true, true)).unwrap();

    assert_eq!(
        first.state,
        VerificationState::Completed(VerificationDecision::Accepted)
    );
    assert_eq!(first.bundle_bytes, second.bundle_bytes);
    assert_eq!(first.bundle_hash, second.bundle_hash);
    assert_eq!(first.payloads, second.payloads);
    first.bundle.validate().unwrap();
    validate_root(&first.bundle.to_value(true), "evidence-bundle").unwrap();
    assert_eq!(first.bundle.claims.len(), 5);
    assert_eq!(first.bundle.items.len(), 5);
    assert!(first.bundle.gaps.is_empty());
    assert!(
        first
            .bundle
            .obligation_status
            .values()
            .all(|status| status == "discharged")
    );

    let mut tampered = first.bundle.clone();
    tampered.artifact_id = None;
    let obligation = tampered.obligation_status.keys().next().unwrap().clone();
    tampered
        .obligation_status
        .insert(obligation, "refuted".to_owned());
    assert_eq!(tampered.validate().unwrap_err().code, "BHCP7001");
}

#[test]
fn false_conditions_and_accepted_counter_evidence_reject_the_candidate() {
    let compilation = compiled();
    let mut accepted_registry = VerifierRegistry::new();
    accepted_registry
        .register(accepted("experiment/verifier.public@0"))
        .unwrap();
    accepted_registry
        .register(accepted("experiment/verifier.oracle@0"))
        .unwrap();
    let false_output = verify(&compilation, &accepted_registry, &output(true, false)).unwrap();
    assert_eq!(
        false_output.state,
        VerificationState::Completed(VerificationDecision::Rejected)
    );

    let mut rejected_registry = VerifierRegistry::new();
    rejected_registry
        .register(accepted("experiment/verifier.public@0"))
        .unwrap();
    rejected_registry
        .register(FakeVerifier {
            symbol: "experiment/verifier.oracle@0",
            execution: VerifierExecution::Completed(VerifierConclusion::Rejected(evidence(
                "experiment/verifier.oracle@0",
                "failed invariant",
            ))),
        })
        .unwrap();
    let rejected = verify(&compilation, &rejected_registry, &output(true, true)).unwrap();
    assert_eq!(
        rejected.state,
        VerificationState::Completed(VerificationDecision::Rejected)
    );
}

#[test]
fn absent_unresolved_and_faulted_verifiers_remain_distinct() {
    let compilation = compiled();
    let mut absent = VerifierRegistry::new();
    absent
        .register(accepted("experiment/verifier.public@0"))
        .unwrap();
    let report = verify(&compilation, &absent, &output(true, true)).unwrap();
    assert_eq!(
        report.state,
        VerificationState::Completed(VerificationDecision::Unresolved)
    );
    assert_eq!(report.bundle.gaps.len(), 1);

    let mut unresolved = VerifierRegistry::new();
    unresolved
        .register(accepted("experiment/verifier.public@0"))
        .unwrap();
    unresolved
        .register(FakeVerifier {
            symbol: "experiment/verifier.oracle@0",
            execution: VerifierExecution::Completed(VerifierConclusion::Unresolved {
                reason: Reason {
                    code: "experiment/reason.no-result@0".to_owned(),
                    message: "no result".to_owned(),
                    details: None,
                },
                evidence: Some(evidence("experiment/verifier.oracle@0", "partial")),
            }),
        })
        .unwrap();
    let report = verify(&compilation, &unresolved, &output(true, true)).unwrap();
    assert_eq!(
        report.state,
        VerificationState::Completed(VerificationDecision::Unresolved)
    );

    let mut faulted = VerifierRegistry::new();
    faulted
        .register(accepted("experiment/verifier.public@0"))
        .unwrap();
    faulted
        .register(FakeVerifier {
            symbol: "experiment/verifier.oracle@0",
            execution: VerifierExecution::Faulted(Reason {
                code: "experiment/fault.crashed@0".to_owned(),
                message: "crashed".to_owned(),
                details: None,
            }),
        })
        .unwrap();
    let report = verify(&compilation, &faulted, &output(true, true)).unwrap();
    assert!(matches!(report.state, VerificationState::Faulted(_)));

    let mut panicked = VerifierRegistry::new();
    panicked
        .register(accepted("experiment/verifier.public@0"))
        .unwrap();
    panicked.register(PanicVerifier).unwrap();
    let report = verify(&compilation, &panicked, &output(true, true)).unwrap();
    let VerificationState::Faulted(faults) = report.state else {
        panic!("panicking verifier must fault")
    };
    assert_eq!(faults[0].code, "bhcp.fault/verifier-panic@0");
}

#[test]
fn registry_and_external_value_boundaries_reject_invalid_inputs() {
    let compilation = compiled();
    let mut registry = VerifierRegistry::new();
    registry
        .register(accepted("experiment/verifier.public@0"))
        .unwrap();
    let diagnostic = registry
        .register(accepted("experiment/verifier.public@0"))
        .unwrap_err();
    assert_eq!(diagnostic.code, "BHCP7001");

    let invalid = Value::map([
        ("publicPassed", Value::Text("yes".to_owned())),
        ("oraclePassed", Value::Bool(true)),
    ]);
    let diagnostic = verify(&compilation, &registry, &invalid).unwrap_err();
    assert_eq!(diagnostic.code, "BHCP7001");

    let input = input();
    let output = output(true, true);
    let diagnostic = registry
        .verify(VerificationRequest {
            compilation: &compilation,
            goal: "experiment/Verify@0",
            execution_instance: None,
            input: &input,
            output: &output,
            subject: reference("subject"),
            subject_bytes: b"subject",
            execution_graph: reference("execution-graph"),
            produced_at: "2026-02-30T10:00:00Z",
        })
        .unwrap_err();
    assert_eq!(diagnostic.code, "BHCP7001");
}
