use bhcp::graph::GraphDocument;
use bhcp::hash::{HashAlgorithm, artifact_hash_with, semantic_hash_with};
use bhcp::kernel::{
    ChildObservation, ExecutionResult, KernelRuntime, OperationalFault, Reason, Reduction, Verdict,
};
use bhcp::model::{BhcpType, ClauseKind, ContentReference};
use bhcp::obligation::build_obligation_graph;
use bhcp::pipeline::{
    Compilation, compile_source, compile_source_with_policy, parse_policy_source,
};
use bhcp::policy::compose_policies;
use bhcp::proof::{ObligationProofRequest, ProofState, verify_obligation_proof};
use bhcp::value::Value;
use bhcp::verification::{
    EvidenceBundle, PayloadArtifact, VerificationRequest, VerificationState, Verifier,
    VerifierConclusion, VerifierContext, VerifierEvidence, VerifierExecution, VerifierRegistry,
};

const SOURCE: &str = r#"
§goal example/Child@0 {
    §output value: Bool;
    §requires "ready": CONDITION;
    §verify "audit": with example/verifier.audit@0 for "ready";
}

§goal example/Parent@0 {
    §output child: { value: Bool };
    §all {
        child = example/Child@0();
    };
}
"#;

#[derive(Clone)]
struct AuditVerifier {
    execution: VerifierExecution,
}

#[derive(Clone)]
struct PolicyVerifier;

impl Verifier for PolicyVerifier {
    fn symbol(&self) -> &str {
        "example/verifier.policy@0"
    }

    fn artifact(&self) -> ContentReference {
        reference("application/vnd.bhcp.verifier", b"policy-verifier")
    }

    fn verify(&self, _context: &VerifierContext<'_>) -> VerifierExecution {
        VerifierExecution::Completed(VerifierConclusion::Accepted(VerifierEvidence::new(
            "static",
            "example/obligation.audit@0",
            "text/plain",
            b"policy-accepted".to_vec(),
            vec![],
        )))
    }
}

impl Verifier for AuditVerifier {
    fn symbol(&self) -> &str {
        "example/verifier.audit@0"
    }

    fn artifact(&self) -> ContentReference {
        reference("application/vnd.bhcp.verifier", b"audit-verifier")
    }

    fn verify(&self, _context: &VerifierContext<'_>) -> VerifierExecution {
        self.execution.clone()
    }
}

fn reference(media_type: &str, bytes: &[u8]) -> ContentReference {
    ContentReference::from_bytes(media_type, bytes, HashAlgorithm::default())
}

fn compilation(condition: bool) -> Compilation {
    compile_source(
        &SOURCE.replace("CONDITION", if condition { "true" } else { "false" }),
        "proof-checker.bhcp",
    )
    .unwrap()
}

fn none_compilation(condition: bool) -> Compilation {
    let source = SOURCE
        .replace("CONDITION", if condition { "true" } else { "false" })
        .replace(
            "    §output child: { value: Bool };\n    §all {",
            "    §none {",
        );
    compile_source(&source, "proof-checker-none.bhcp").unwrap()
}

fn any_compilation() -> Compilation {
    let source = r#"
§goal example/ChildA@0 {
    §output value: Bool;
    §requires "ready-a": true;
    §verify "audit-a": with example/verifier.audit@0 for "ready-a";
}

§goal example/ChildB@0 {
    §output value: Bool;
    §requires "ready-b": true;
    §verify "audit-b": with example/verifier.audit@0 for "ready-b";
}

§goal example/Parent@0 {
    §output output: { value: Bool };
    §output tag: Text;
    §any {
        first = example/ChildA@0();
        second = example/ChildB@0();
    };
}
"#;
    compile_source(source, "proof-checker-any.bhcp").unwrap()
}

fn policy_compilation() -> Compilation {
    let source = r#"
§goal example/Child@0 {
    §output value: Bool;
}

§goal example/Parent@0 {
    §output child: { value: Bool };
    §all {
        child = example/Child@0();
    };
}
"#;
    let policy = r#"
§policy example/policy@0 {
    layer repository;
    rule audit: evidence add {
        obligation: example/obligation.audit@0,
        classes: [static],
        minimum: 2,
        scope: { goals: [example/Parent@0] }
    } nonwaivable;
}
"#;
    let parsed = parse_policy_source(policy, "proof-policy.bhcp").unwrap();
    let policy = compose_policies(&parsed.documents, Default::default()).unwrap();
    compile_source_with_policy(source, "proof-policy-program.bhcp", &policy).unwrap()
}

fn restricted_verifier_compilation() -> Compilation {
    let mut compilation = compilation(true);
    let goal = compilation
        .ir
        .goals
        .iter_mut()
        .find(|goal| goal.symbol == "example/Child@0")
        .unwrap();
    let binding = goal
        .clauses
        .iter_mut()
        .find_map(|clause| match &mut clause.kind {
            ClauseKind::Verify { binding, .. } => Some(binding),
            _ => None,
        })
        .unwrap();
    binding.output = BhcpType::Evidence(vec!["static".to_owned()]);
    binding.trust = vec!["static".to_owned()];
    let algorithm = HashAlgorithm::default();
    compilation.semantic_hash = semantic_hash_with(&compilation.ir, algorithm).unwrap();
    compilation.ir.semantic_id = Some(compilation.semantic_hash.clone());
    compilation.ir_hash = artifact_hash_with(&compilation.ir.to_value(false), algorithm).unwrap();
    compilation.ir.artifact_id = Some(compilation.ir_hash.clone());
    compilation.ir_bytes =
        bhcp::cbor::encode_deterministic(&compilation.ir.to_value(true)).unwrap();
    compilation.ir.validate().unwrap();
    compilation
}

fn candidate() -> ContentReference {
    reference("application/vnd.bhcp.candidate", b"candidate-v1")
}

fn accepted_verifier() -> AuditVerifier {
    AuditVerifier {
        execution: VerifierExecution::Completed(VerifierConclusion::Accepted(
            VerifierEvidence::new(
                "static",
                "example/verifier.audit@0",
                "text/plain",
                b"accepted".to_vec(),
                vec!["example/trust.local@0".to_owned()],
            ),
        )),
    }
}

fn faulted_verifier() -> AuditVerifier {
    AuditVerifier {
        execution: VerifierExecution::Faulted(Reason {
            code: "bhcp.fault/verifier-contract@0".to_owned(),
            message: "verifier contract failed".to_owned(),
            details: None,
        }),
    }
}

fn verification(
    compilation: &Compilation,
    registry: &VerifierRegistry,
) -> bhcp::verification::VerificationReport {
    let input = Value::owned_map(vec![]);
    let output = Value::map([("value", Value::Bool(true))]);
    registry
        .verify(VerificationRequest {
            compilation,
            goal: "example/Child@0",
            input: &input,
            output: &output,
            subject: candidate(),
            subject_bytes: b"candidate-v1",
            execution_graph: reference("application/cbor", b"execution-graph"),
            produced_at: "2026-07-21T09:30:00Z",
        })
        .unwrap()
}

fn verification_for(
    compilation: &Compilation,
    registry: &VerifierRegistry,
    goal: &str,
) -> bhcp::verification::VerificationReport {
    let input = Value::owned_map(vec![]);
    let output = Value::map([("value", Value::Bool(true))]);
    registry
        .verify(VerificationRequest {
            compilation,
            goal,
            input: &input,
            output: &output,
            subject: candidate(),
            subject_bytes: b"candidate-v1",
            execution_graph: reference("application/cbor", b"execution-graph"),
            produced_at: "2026-07-21T09:30:00Z",
        })
        .unwrap()
}

fn merge_evidence(
    mut first: bhcp::verification::VerificationReport,
    mut second: bhcp::verification::VerificationReport,
) -> (EvidenceBundle, Vec<PayloadArtifact>) {
    for claim in &mut second.bundle.claims {
        claim.id = format!("second-{}", claim.id);
    }
    for item in &mut second.bundle.items {
        item.id = format!("second-{}", item.id);
        for claim in &mut item.claims {
            *claim = format!("second-{claim}");
        }
    }
    for gap in &mut second.bundle.gaps {
        gap.id = format!("second-{}", gap.id);
    }
    for edge in &mut second.bundle.edges {
        edge.id = format!("second-{}", edge.id);
        edge.from = format!("second-{}", edge.from);
        edge.to = format!("second-{}", edge.to);
    }
    first.bundle.claims.extend(second.bundle.claims);
    first.bundle.items.extend(second.bundle.items);
    first.bundle.gaps.extend(second.bundle.gaps);
    first.bundle.edges.extend(second.bundle.edges);
    first
        .bundle
        .obligation_status
        .extend(second.bundle.obligation_status);
    first.payloads.extend(second.payloads);
    rematerialize(&mut first.bundle);
    (first.bundle, first.payloads)
}

fn accepted_claims(bundle: &EvidenceBundle, polarity: &str) -> Vec<String> {
    bundle
        .claims
        .iter()
        .filter(|claim| claim.status == "accepted" && claim.polarity == polarity)
        .map(|claim| claim.id.clone())
        .collect()
}

fn rematerialize(bundle: &mut EvidenceBundle) {
    bundle.artifact_id = None;
    bundle.artifact_id =
        Some(artifact_hash_with(&bundle.to_value(false), HashAlgorithm::default()).unwrap());
    bundle.validate().unwrap();
}

fn replace_expression_payload(
    bundle: &mut EvidenceBundle,
    payloads: &mut [PayloadArtifact],
    replacement: Value,
) {
    let item = bundle
        .items
        .iter_mut()
        .find(|item| item.verifier == "bhcp.verifier/expression@0")
        .unwrap();
    let payload = payloads
        .iter_mut()
        .find(|payload| payload.reference == item.payload)
        .unwrap();
    let bytes = bhcp::cbor::encode_deterministic(&replacement).unwrap();
    let reference = reference("application/cbor", &bytes);
    item.payload = reference.clone();
    payload.reference = reference;
    payload.bytes = bytes;
    rematerialize(bundle);
}

fn proof(
    compilation: &Compilation,
    graph: &GraphDocument,
    bundle: &EvidenceBundle,
    payloads: &[PayloadArtifact],
    verifier_registry: &VerifierRegistry,
    child_result: ExecutionResult,
) -> (
    Reduction,
    bhcp::diagnostic::Result<bhcp::proof::ProofReport>,
) {
    let observations = [ChildObservation {
        child: "child-1".to_owned(),
        result: child_result,
    }];
    let parent = Value::owned_map(vec![]);
    let claimed = KernelRuntime::new(&compilation.ir)
        .reduce("network-1", parent.clone(), &observations)
        .unwrap();
    let checked = verify_obligation_proof(ObligationProofRequest {
        compilation,
        obligation_graph: graph,
        network: "network-1",
        parent: &parent,
        observations: &observations,
        claimed: &claimed,
        evidence: bundle,
        payloads,
        verifier_registry,
        candidate: &candidate(),
        candidate_bytes: b"candidate-v1",
        produced_at: "2026-07-21T09:30:00Z",
    });
    (claimed, checked)
}

#[test]
fn complete_graph_proofs_preserve_satisfied_refuted_unresolved_and_faulted() {
    let satisfied_compilation = compilation(true);
    let satisfied_graph = build_obligation_graph(&satisfied_compilation).unwrap();
    let mut registry = VerifierRegistry::new();
    registry.register(accepted_verifier()).unwrap();
    let satisfied_evidence = verification(&satisfied_compilation, &registry);
    let (_, checked) = proof(
        &satisfied_compilation,
        &satisfied_graph,
        &satisfied_evidence.bundle,
        &satisfied_evidence.payloads,
        &registry,
        ExecutionResult::Completed(Verdict::Satisfied {
            output: Value::map([("value", Value::Bool(true))]),
            evidence: accepted_claims(&satisfied_evidence.bundle, "supports"),
        }),
    );
    assert_eq!(checked.unwrap().state, ProofState::Satisfied);

    let refuted_compilation = compilation(false);
    let refuted_graph = build_obligation_graph(&refuted_compilation).unwrap();
    let refuted_evidence = verification(&refuted_compilation, &registry);
    let (_, checked) = proof(
        &refuted_compilation,
        &refuted_graph,
        &refuted_evidence.bundle,
        &refuted_evidence.payloads,
        &registry,
        ExecutionResult::Completed(Verdict::Refuted {
            counter_evidence: accepted_claims(&refuted_evidence.bundle, "refutes"),
        }),
    );
    assert_eq!(checked.unwrap().state, ProofState::Refuted);

    let unresolved_compilation = compilation(true);
    let unresolved_graph = build_obligation_graph(&unresolved_compilation).unwrap();
    let unresolved_registry = VerifierRegistry::new();
    let unresolved_evidence = verification(&unresolved_compilation, &unresolved_registry);
    let reason = unresolved_evidence.bundle.gaps[0].reason.clone();
    let (_, checked) = proof(
        &unresolved_compilation,
        &unresolved_graph,
        &unresolved_evidence.bundle,
        &unresolved_evidence.payloads,
        &unresolved_registry,
        ExecutionResult::Completed(Verdict::Unresolved {
            reason,
            partial_evidence: accepted_claims(&unresolved_evidence.bundle, "supports"),
        }),
    );
    assert_eq!(checked.unwrap().state, ProofState::Unresolved);

    let faulted_compilation = compilation(true);
    let faulted_graph = build_obligation_graph(&faulted_compilation).unwrap();
    let mut faulted_registry = VerifierRegistry::new();
    faulted_registry.register(faulted_verifier()).unwrap();
    let faulted_evidence = verification(&faulted_compilation, &faulted_registry);
    let VerificationState::Faulted(faults) = &faulted_evidence.state else {
        panic!("expected faulted verification")
    };
    let (_, checked) = proof(
        &faulted_compilation,
        &faulted_graph,
        &faulted_evidence.bundle,
        &faulted_evidence.payloads,
        &faulted_registry,
        ExecutionResult::Faulted(OperationalFault {
            error: faults[0].clone(),
            trace: vec![],
        }),
    );
    assert_eq!(checked.unwrap().state, ProofState::Faulted);
}

#[test]
fn exact_obligation_closure_rejects_target_and_dependency_substitution() {
    let compilation = compilation(true);
    let graph = build_obligation_graph(&compilation).unwrap();
    let mut registry = VerifierRegistry::new();
    registry.register(accepted_verifier()).unwrap();
    let report = verification(&compilation, &registry);
    let valid_result = ExecutionResult::Completed(Verdict::Satisfied {
        output: Value::map([("value", Value::Bool(true))]),
        evidence: accepted_claims(&report.bundle, "supports"),
    });

    let discharge = graph
        .nodes()
        .iter()
        .find(|node| node.kind == "discharge")
        .unwrap()
        .id
        .clone();
    let original = report
        .bundle
        .obligation_status
        .keys()
        .next()
        .unwrap()
        .clone();
    let mut substituted = report.bundle.clone();
    let status = substituted.obligation_status.remove(&original).unwrap();
    substituted
        .obligation_status
        .insert(discharge.clone(), status);
    for claim in &mut substituted.claims {
        claim.obligation = discharge.clone();
    }
    rematerialize(&mut substituted);
    assert_eq!(
        proof(
            &compilation,
            &graph,
            &substituted,
            &report.payloads,
            &registry,
            valid_result.clone(),
        )
        .1
        .unwrap_err()
        .code,
        "BHCP7301"
    );

    let mut graph_value = graph.to_value();
    let Value::Map(root) = &mut graph_value else {
        unreachable!()
    };
    root.retain(|(field, _)| field != "semantic_id" && field != "artifact_id");
    let edges_value = &mut root
        .iter_mut()
        .find(|(field, _)| field == "edges")
        .unwrap()
        .1;
    let Value::Array(edges) = edges_value else {
        unreachable!()
    };
    edges.clear();
    let mut missing_dependency = GraphDocument::from_value(&graph_value).unwrap();
    missing_dependency
        .materialize_identities(HashAlgorithm::default())
        .unwrap();
    assert_eq!(
        proof(
            &compilation,
            &missing_dependency,
            &report.bundle,
            &report.payloads,
            &registry,
            valid_result,
        )
        .1
        .unwrap_err()
        .code,
        "BHCP7301"
    );
}

#[test]
fn candidate_payload_producer_and_derivation_identity_are_not_substitutable() {
    let compilation = compilation(true);
    let graph = build_obligation_graph(&compilation).unwrap();
    let mut registry = VerifierRegistry::new();
    registry.register(accepted_verifier()).unwrap();
    let report = verification(&compilation, &registry);
    let child_result = ExecutionResult::Completed(Verdict::Satisfied {
        output: Value::map([("value", Value::Bool(true))]),
        evidence: accepted_claims(&report.bundle, "supports"),
    });
    let (claimed, valid) = proof(
        &compilation,
        &graph,
        &report.bundle,
        &report.payloads,
        &registry,
        child_result.clone(),
    );
    valid.unwrap();

    let mut wrong_subject = report.bundle.clone();
    for claim in &mut wrong_subject.claims {
        claim.subject = reference("application/vnd.bhcp.candidate", b"candidate-v2");
    }
    rematerialize(&mut wrong_subject);
    assert_eq!(
        proof(
            &compilation,
            &graph,
            &wrong_subject,
            &report.payloads,
            &registry,
            child_result.clone(),
        )
        .1
        .unwrap_err()
        .code,
        "BHCP7301"
    );

    let mut unsupported = report.bundle.clone();
    let external = unsupported
        .items
        .iter_mut()
        .find(|item| item.verifier == "example/verifier.audit@0")
        .unwrap();
    external.verifier = "bhcp/prelude.all@0".to_owned();
    external.producer = "bhcp/prelude.all@0".to_owned();
    rematerialize(&mut unsupported);
    assert_eq!(
        proof(
            &compilation,
            &graph,
            &unsupported,
            &report.payloads,
            &registry,
            child_result.clone(),
        )
        .1
        .unwrap_err()
        .code,
        "BHCP7301"
    );

    let derivation = match claimed {
        Reduction::Concluded { derivation, .. } => derivation,
        _ => unreachable!(),
    };
    let mut collision = report.bundle.clone();
    let old_item = collision.items[0].id.clone();
    collision.items[0].id = derivation.id.clone();
    for edge in &mut collision.edges {
        if edge.from == old_item {
            edge.from = derivation.id.clone();
        }
    }
    rematerialize(&mut collision);
    assert_eq!(
        proof(
            &compilation,
            &graph,
            &collision,
            &report.payloads,
            &registry,
            child_result,
        )
        .1
        .unwrap_err()
        .code,
        "BHCP7301"
    );
}

#[test]
fn reducer_re_evaluation_rejects_hidden_output_or_premise_choice() {
    let compilation = compilation(true);
    let graph = build_obligation_graph(&compilation).unwrap();
    let mut registry = VerifierRegistry::new();
    registry.register(accepted_verifier()).unwrap();
    let report = verification(&compilation, &registry);
    let child_result = ExecutionResult::Completed(Verdict::Satisfied {
        output: Value::map([("value", Value::Bool(true))]),
        evidence: accepted_claims(&report.bundle, "supports"),
    });
    let (mut claimed, _) = proof(
        &compilation,
        &graph,
        &report.bundle,
        &report.payloads,
        &registry,
        child_result.clone(),
    );
    let Reduction::Concluded { derivation, .. } = &mut claimed else {
        unreachable!()
    };
    derivation.premises.reverse();
    assert_eq!(
        verify_obligation_proof(ObligationProofRequest {
            compilation: &compilation,
            obligation_graph: &graph,
            network: "network-1",
            parent: &Value::owned_map(vec![]),
            observations: &[ChildObservation {
                child: "child-1".to_owned(),
                result: child_result,
            }],
            claimed: &claimed,
            evidence: &report.bundle,
            payloads: &report.payloads,
            verifier_registry: &registry,
            candidate: &candidate(),
            candidate_bytes: b"candidate-v1",
            produced_at: "2026-07-21T09:30:00Z",
        })
        .unwrap_err()
        .code,
        "BHCP7302"
    );
}

#[test]
fn expression_payload_goal_and_production_edges_are_exact() {
    let compilation = compilation(true);
    let graph = build_obligation_graph(&compilation).unwrap();
    let mut registry = VerifierRegistry::new();
    registry.register(accepted_verifier()).unwrap();
    let report = verification(&compilation, &registry);
    let child_result = ExecutionResult::Completed(Verdict::Satisfied {
        output: Value::map([("value", Value::Bool(true))]),
        evidence: accepted_claims(&report.bundle, "supports"),
    });

    let expression_item = report
        .bundle
        .items
        .iter()
        .find(|item| item.verifier == "bhcp.verifier/expression@0")
        .unwrap();
    let expression_payload = report
        .payloads
        .iter()
        .find(|payload| payload.reference == expression_item.payload)
        .unwrap();
    let mut wrong_goal = bhcp::cbor::decode_deterministic(&expression_payload.bytes).unwrap();
    let Value::Map(fields) = &mut wrong_goal else {
        unreachable!()
    };
    fields
        .iter_mut()
        .find(|(field, _)| field == "goal")
        .unwrap()
        .1 = Value::Text("example/WrongGoal@0".to_owned());
    let mut wrong_goal_bundle = report.bundle.clone();
    let mut wrong_goal_payloads = report.payloads.clone();
    replace_expression_payload(&mut wrong_goal_bundle, &mut wrong_goal_payloads, wrong_goal);
    assert_eq!(
        proof(
            &compilation,
            &graph,
            &wrong_goal_bundle,
            &wrong_goal_payloads,
            &registry,
            child_result.clone(),
        )
        .1
        .unwrap_err()
        .code,
        "BHCP7301"
    );

    let mut crossed_edges = report.bundle.clone();
    assert_eq!(crossed_edges.items.len(), 2);
    let first_claims = crossed_edges.items[0].claims.clone();
    crossed_edges.items[0].claims = crossed_edges.items[1].claims.clone();
    crossed_edges.items[1].claims = first_claims;
    rematerialize(&mut crossed_edges);
    assert_eq!(
        proof(
            &compilation,
            &graph,
            &crossed_edges,
            &report.payloads,
            &registry,
            child_result,
        )
        .1
        .unwrap_err()
        .code,
        "BHCP7301"
    );
}

#[test]
fn unresolved_and_faulted_results_bind_the_exact_sealed_reason() {
    let unresolved_compilation = compilation(true);
    let unresolved_graph = build_obligation_graph(&unresolved_compilation).unwrap();
    let unresolved_registry = VerifierRegistry::new();
    let unresolved_evidence = verification(&unresolved_compilation, &unresolved_registry);
    let wrong_reason = Reason {
        code: "bhcp.unresolved/substituted@0".to_owned(),
        message: "substituted unresolved reason".to_owned(),
        details: None,
    };
    assert_eq!(
        proof(
            &unresolved_compilation,
            &unresolved_graph,
            &unresolved_evidence.bundle,
            &unresolved_evidence.payloads,
            &unresolved_registry,
            ExecutionResult::Completed(Verdict::Unresolved {
                reason: wrong_reason,
                partial_evidence: accepted_claims(&unresolved_evidence.bundle, "supports"),
            }),
        )
        .1
        .unwrap_err()
        .code,
        "BHCP7302"
    );

    let faulted_compilation = compilation(true);
    let faulted_graph = build_obligation_graph(&faulted_compilation).unwrap();
    let mut faulted_registry = VerifierRegistry::new();
    faulted_registry.register(faulted_verifier()).unwrap();
    let faulted_evidence = verification(&faulted_compilation, &faulted_registry);
    assert_eq!(
        proof(
            &faulted_compilation,
            &faulted_graph,
            &faulted_evidence.bundle,
            &faulted_evidence.payloads,
            &faulted_registry,
            ExecutionResult::Faulted(OperationalFault {
                error: Reason {
                    code: "bhcp.fault/substituted@0".to_owned(),
                    message: "substituted operational fault".to_owned(),
                    details: None,
                },
                trace: vec![],
            }),
        )
        .1
        .unwrap_err()
        .code,
        "BHCP7302"
    );
}

#[test]
fn generic_checker_accepts_counter_evidence_as_a_none_reducer_premise() {
    let compilation = none_compilation(false);
    let graph = build_obligation_graph(&compilation).unwrap();
    let mut registry = VerifierRegistry::new();
    registry.register(accepted_verifier()).unwrap();
    let report = verification(&compilation, &registry);
    let (_, checked) = proof(
        &compilation,
        &graph,
        &report.bundle,
        &report.payloads,
        &registry,
        ExecutionResult::Completed(Verdict::Refuted {
            counter_evidence: accepted_claims(&report.bundle, "refutes"),
        }),
    );
    assert_eq!(checked.unwrap().state, ProofState::Satisfied);
}

#[test]
fn decisive_any_proof_keeps_an_unused_dependency_unresolved() {
    let compilation = any_compilation();
    let graph = build_obligation_graph(&compilation).unwrap();
    let mut registry = VerifierRegistry::new();
    registry.register(accepted_verifier()).unwrap();
    let first = verification_for(&compilation, &registry, "example/ChildA@0");
    let second = verification_for(&compilation, &VerifierRegistry::new(), "example/ChildB@0");
    let (bundle, payloads) = merge_evidence(first, second);
    let first_obligation = graph
        .nodes()
        .iter()
        .find(|node| {
            node.kind == "requirement"
                && node.value().get("goals")
                    == Some(&Value::Array(vec![Value::Text(
                        "example/ChildA@0".to_owned(),
                    )]))
        })
        .unwrap()
        .id
        .clone();
    let evidence = bundle
        .claims
        .iter()
        .filter(|claim| {
            claim.obligation == first_obligation
                && claim.status == "accepted"
                && claim.polarity == "supports"
        })
        .map(|claim| claim.id.clone())
        .collect();
    let observations = [ChildObservation {
        child: "child-1".to_owned(),
        result: ExecutionResult::Completed(Verdict::Satisfied {
            output: Value::map([("value", Value::Bool(true))]),
            evidence,
        }),
    }];
    let parent = Value::owned_map(vec![]);
    let claimed = KernelRuntime::new(&compilation.ir)
        .reduce("network-1", parent.clone(), &observations)
        .unwrap();
    let checked = verify_obligation_proof(ObligationProofRequest {
        compilation: &compilation,
        obligation_graph: &graph,
        network: "network-1",
        parent: &parent,
        observations: &observations,
        claimed: &claimed,
        evidence: &bundle,
        payloads: &payloads,
        verifier_registry: &registry,
        candidate: &candidate(),
        candidate_bytes: b"candidate-v1",
        produced_at: "2026-07-21T09:30:00Z",
    })
    .unwrap();
    assert_eq!(checked.state, ProofState::Satisfied);
    assert!(
        checked
            .obligation_status
            .values()
            .any(|status| status == "unresolved")
    );
}

#[test]
fn policy_minimum_counts_distinct_producers_not_duplicated_items() {
    let compilation = policy_compilation();
    let graph = build_obligation_graph(&compilation).unwrap();
    let mut registry = VerifierRegistry::new();
    registry.register(PolicyVerifier).unwrap();
    registry.register(accepted_verifier()).unwrap();
    registry
        .bind_policy_evidence("example/obligation.audit@0", "example/verifier.policy@0")
        .unwrap();
    let input = Value::owned_map(vec![]);
    let output = Value::map([("child", Value::map([("value", Value::Bool(true))]))]);
    let report = registry
        .verify(VerificationRequest {
            compilation: &compilation,
            goal: "example/Parent@0",
            input: &input,
            output: &output,
            subject: candidate(),
            subject_bytes: b"candidate-v1",
            execution_graph: reference("application/cbor", b"execution-graph"),
            produced_at: "2026-07-21T09:30:00Z",
        })
        .unwrap();
    let obligation = report.bundle.policy_obligations[0].id.clone();
    let mut duplicated = report.bundle.clone();
    let original_claim = duplicated
        .claims
        .iter()
        .find(|claim| claim.obligation == obligation && claim.status == "accepted")
        .unwrap()
        .clone();
    let original_item = duplicated
        .items
        .iter()
        .find(|item| item.claims.contains(&original_claim.id))
        .unwrap()
        .clone();
    let mut duplicate_claim = original_claim.clone();
    duplicate_claim.id = "duplicate-policy-claim".to_owned();
    let mut duplicate_item = original_item.clone();
    duplicate_item.id = "duplicate-policy-item".to_owned();
    duplicate_item.claims = vec![duplicate_claim.id.clone()];
    duplicated.claims.push(duplicate_claim.clone());
    duplicated.items.push(duplicate_item.clone());
    duplicated.edges.push(bhcp::verification::EvidenceEdge {
        id: "duplicate-policy-edge".to_owned(),
        from: duplicate_item.id,
        to: duplicate_claim.id,
        kind: "produces".to_owned(),
    });
    duplicated
        .gaps
        .retain(|gap| !gap.obligations.contains(&obligation));
    duplicated
        .obligation_status
        .insert(obligation, "discharged".to_owned());
    rematerialize(&mut duplicated);

    let observations = [ChildObservation {
        child: "child-1".to_owned(),
        result: ExecutionResult::Completed(Verdict::Satisfied {
            output: Value::map([("value", Value::Bool(true))]),
            evidence: vec!["derivation-child-proof".to_owned()],
        }),
    }];
    let claimed = KernelRuntime::new(&compilation.ir)
        .reduce("network-1", input.clone(), &observations)
        .unwrap();
    assert_eq!(
        verify_obligation_proof(ObligationProofRequest {
            compilation: &compilation,
            obligation_graph: &graph,
            network: "network-1",
            parent: &input,
            observations: &observations,
            claimed: &claimed,
            evidence: &duplicated,
            payloads: &report.payloads,
            verifier_registry: &registry,
            candidate: &candidate(),
            candidate_bytes: b"candidate-v1",
            produced_at: "2026-07-21T09:30:00Z",
        })
        .unwrap_err()
        .code,
        "BHCP7301"
    );

    let mut forged_producer = duplicated.clone();
    let forged_item = forged_producer
        .items
        .iter_mut()
        .find(|item| item.id == "duplicate-policy-item")
        .unwrap();
    forged_item.verifier = "example/verifier.forged@0".to_owned();
    forged_item.producer = "example/verifier.forged@0".to_owned();
    rematerialize(&mut forged_producer);
    assert_eq!(
        verify_obligation_proof(ObligationProofRequest {
            compilation: &compilation,
            obligation_graph: &graph,
            network: "network-1",
            parent: &input,
            observations: &observations,
            claimed: &claimed,
            evidence: &forged_producer,
            payloads: &report.payloads,
            verifier_registry: &registry,
            candidate: &candidate(),
            candidate_bytes: b"candidate-v1",
            produced_at: "2026-07-21T09:30:00Z",
        })
        .unwrap_err()
        .code,
        "BHCP7301"
    );

    let mut unbound_producer = duplicated.clone();
    let unbound_item = unbound_producer
        .items
        .iter_mut()
        .find(|item| item.id == "duplicate-policy-item")
        .unwrap();
    unbound_item.verifier = "example/verifier.audit@0".to_owned();
    unbound_item.producer = "example/verifier.audit@0".to_owned();
    unbound_item.verifier_artifact = accepted_verifier().artifact();
    rematerialize(&mut unbound_producer);
    assert_eq!(
        verify_obligation_proof(ObligationProofRequest {
            compilation: &compilation,
            obligation_graph: &graph,
            network: "network-1",
            parent: &input,
            observations: &observations,
            claimed: &claimed,
            evidence: &unbound_producer,
            payloads: &report.payloads,
            verifier_registry: &registry,
            candidate: &candidate(),
            candidate_bytes: b"candidate-v1",
            produced_at: "2026-07-21T09:30:00Z",
        })
        .unwrap_err()
        .code,
        "BHCP7301"
    );

    let mut forged_provenance = report.bundle.clone();
    forged_provenance.policy_obligations[0].sources[0].rule = "forged-rule".to_owned();
    rematerialize(&mut forged_provenance);
    assert_eq!(
        verify_obligation_proof(ObligationProofRequest {
            compilation: &compilation,
            obligation_graph: &graph,
            network: "network-1",
            parent: &input,
            observations: &observations,
            claimed: &claimed,
            evidence: &forged_provenance,
            payloads: &report.payloads,
            verifier_registry: &registry,
            candidate: &candidate(),
            candidate_bytes: b"candidate-v1",
            produced_at: "2026-07-21T09:30:00Z",
        })
        .unwrap_err()
        .code,
        "BHCP7301"
    );
}

#[test]
fn retained_verifier_type_trust_and_freshness_are_rechecked() {
    let compilation = restricted_verifier_compilation();
    let graph = build_obligation_graph(&compilation).unwrap();
    let mut registry = VerifierRegistry::new();
    registry.register(accepted_verifier()).unwrap();
    let report = verification(&compilation, &registry);
    let child_result = ExecutionResult::Completed(Verdict::Satisfied {
        output: Value::map([("value", Value::Bool(true))]),
        evidence: accepted_claims(&report.bundle, "supports"),
    });

    let mut wrong_class = report.bundle.clone();
    wrong_class
        .items
        .iter_mut()
        .find(|item| item.verifier == "example/verifier.audit@0")
        .unwrap()
        .evidence_class = "formal".to_owned();
    rematerialize(&mut wrong_class);
    assert_eq!(
        proof(
            &compilation,
            &graph,
            &wrong_class,
            &report.payloads,
            &registry,
            child_result.clone(),
        )
        .1
        .unwrap_err()
        .code,
        "BHCP7301"
    );

    let mut stale = report.bundle.clone();
    for item in &mut stale.items {
        item.produced_at = "2026-07-20T09:30:00Z".to_owned();
    }
    rematerialize(&mut stale);
    assert_eq!(
        proof(
            &compilation,
            &graph,
            &stale,
            &report.payloads,
            &registry,
            child_result,
        )
        .1
        .unwrap_err()
        .code,
        "BHCP7301"
    );
}
