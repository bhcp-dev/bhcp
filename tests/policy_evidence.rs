use std::sync::{Arc, Mutex};

use bhcp::hash::HashAlgorithm;
use bhcp::inspection::render_artifact;
use bhcp::kernel::Reason;
use bhcp::model::ContentReference;
use bhcp::pipeline::{Compilation, compile_source_with_policy, parse_policy_source};
use bhcp::policy::{EffectivePolicyDocument, compose_policies};
use bhcp::schema::validate_root;
use bhcp::value::Value;
use bhcp::verification::{
    VerificationDecision, VerificationRequest, VerificationState, Verifier, VerifierConclusion,
    VerifierContext, VerifierEvidence, VerifierExecution, VerifierRegistry,
};

const GOAL: &str = r#"
§goal example/Review@0 {
  §input repository: Text;
  §output accepted: Bool;
  §ensures "accepted": accepted;
}
"#;

const POLICY: &str = r#"
§policy example/policy.organization@0 {
  layer organization;
  rule a-review: evidence add {
    obligation: example/obligation.review@0,
    classes: [static],
    minimum: 1,
    scope: { goals: [example/Review@0] }
  } nonwaivable;
}
§policy example/policy.team@0 {
  layer team;
  rule a-audit: evidence add {
    obligation: example/obligation.audit@0,
    classes: [static],
    minimum: 1,
    scope: { goals: [example/Review@0] }
  } nonwaivable;
}
"#;

#[derive(Clone)]
struct PolicyVerifier {
    symbol: &'static str,
    execution: VerifierExecution,
    targets: Arc<Mutex<Vec<Vec<String>>>>,
}

impl Verifier for PolicyVerifier {
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

    fn verify(&self, context: &VerifierContext<'_>) -> VerifierExecution {
        self.targets
            .lock()
            .unwrap()
            .push(context.obligations.to_vec());
        self.execution.clone()
    }
}

fn evidence(predicate: &'static str) -> VerifierExecution {
    evidence_with_class(predicate, "static")
}

fn evidence_with_class(predicate: &'static str, class: &'static str) -> VerifierExecution {
    VerifierExecution::Completed(VerifierConclusion::Accepted(VerifierEvidence::new(
        class,
        predicate,
        "text/plain",
        b"accepted".to_vec(),
        vec![],
    )))
}

fn policy() -> EffectivePolicyDocument {
    effective(POLICY)
}

fn effective(source: &str) -> EffectivePolicyDocument {
    let parsed = parse_policy_source(source, "policy.bhcp").unwrap();
    compose_policies(&parsed.documents, Default::default()).unwrap()
}

fn compilation() -> Compilation {
    compile_source_with_policy(GOAL, "review.bhcp", &policy()).unwrap()
}

fn reference(name: &str) -> ContentReference {
    ContentReference::from_bytes(
        "application/cbor",
        name.as_bytes(),
        HashAlgorithm::default(),
    )
}

fn verify(
    compilation: &Compilation,
    registry: &VerifierRegistry,
) -> bhcp::diagnostic::Result<bhcp::verification::VerificationReport> {
    let input = Value::map([("repository", Value::Text("subject@0".to_owned()))]);
    let output = Value::map([("accepted", Value::Bool(true))]);
    registry.verify(VerificationRequest {
        compilation,
        goal: "example/Review@0",
        input: &input,
        output: &output,
        subject: reference("subject"),
        subject_bytes: b"subject",
        execution_graph: reference("execution-graph"),
        produced_at: "2026-07-19T08:00:00Z",
    })
}

fn register(
    registry: &mut VerifierRegistry,
    symbol: &'static str,
    predicate: &'static str,
    execution: VerifierExecution,
) -> Arc<Mutex<Vec<Vec<String>>>> {
    let targets = Arc::new(Mutex::new(vec![]));
    registry
        .register(PolicyVerifier {
            symbol,
            execution,
            targets: targets.clone(),
        })
        .unwrap();
    registry.bind_policy_evidence(predicate, symbol).unwrap();
    targets
}

#[test]
fn composed_policy_evidence_resolves_to_structural_targets_with_source_provenance() {
    let compilation = compilation();
    let mut registry = VerifierRegistry::new();
    let review_targets = register(
        &mut registry,
        "example/verifier.review@0",
        "example/obligation.review@0",
        evidence("example/obligation.review@0"),
    );
    let audit_targets = register(
        &mut registry,
        "example/verifier.audit@0",
        "example/obligation.audit@0",
        evidence("example/obligation.audit@0"),
    );

    let report = verify(&compilation, &registry).unwrap();
    assert_eq!(
        report.state,
        VerificationState::Completed(VerificationDecision::Accepted)
    );
    assert_eq!(report.bundle.policy_obligations.len(), 2);
    let audit = &report.bundle.policy_obligations[0];
    let review = &report.bundle.policy_obligations[1];
    assert_eq!(audit.symbol, "example/obligation.audit@0");
    assert_eq!(review.symbol, "example/obligation.review@0");
    assert_eq!(audit.sources[0].layer, "team");
    assert_eq!(audit.sources[0].policy, "example/policy.team@0");
    assert_eq!(audit.sources[0].rule, "a-audit");
    assert_eq!(review.sources[0].layer, "organization");
    assert_eq!(review.sources[0].policy, "example/policy.organization@0");
    assert_eq!(review.sources[0].rule, "a-review");
    assert_eq!(audit.minimum, 1);
    assert_eq!(review.minimum, 1);
    assert_eq!(audit.classes, ["static"]);
    assert_eq!(review.classes, ["static"]);
    assert_eq!(*audit_targets.lock().unwrap(), vec![vec![audit.id.clone()]]);
    assert_eq!(
        *review_targets.lock().unwrap(),
        vec![vec![review.id.clone()]]
    );
    assert_eq!(report.bundle.obligation_status[&audit.id], "discharged");
    assert_eq!(report.bundle.obligation_status[&review.id], "discharged");
    report.bundle.validate().unwrap();
    validate_root(&report.bundle.to_value(true), "evidence-bundle").unwrap();
    let inspected = render_artifact(&report.bundle.to_value(true), None);
    assert!(inspected.contains(&format!(
        "policy-obligation {} example/obligation.audit@0 minimum 1 classes [static]",
        audit.id
    )));
    assert!(inspected.contains("policy-source team example/policy.team@0:a-audit"));
}

#[test]
fn unavailable_policy_producer_is_an_unresolved_required_gap() {
    let compilation = compilation();
    let mut registry = VerifierRegistry::new();
    registry
        .bind_policy_evidence(
            "example/obligation.review@0",
            "example/verifier.unavailable@0",
        )
        .unwrap();
    register(
        &mut registry,
        "example/verifier.audit@0",
        "example/obligation.audit@0",
        evidence("example/obligation.audit@0"),
    );

    let report = verify(&compilation, &registry).unwrap();
    assert_eq!(
        report.state,
        VerificationState::Completed(VerificationDecision::Unresolved)
    );
    let review = report
        .bundle
        .policy_obligations
        .iter()
        .find(|obligation| obligation.symbol == "example/obligation.review@0")
        .unwrap();
    assert_eq!(report.bundle.obligation_status[&review.id], "unresolved");
    assert!(report.bundle.gaps.iter().any(|gap| {
        gap.obligations == [review.id.clone()]
            && gap.reason.code == "bhcp.reason/policy-verifier-unregistered@0"
    }));
}

#[test]
fn policy_producer_rejection_and_fault_keep_existing_semantics() {
    let compilation = compilation();
    let mut rejected = VerifierRegistry::new();
    register(
        &mut rejected,
        "example/verifier.review@0",
        "example/obligation.review@0",
        VerifierExecution::Completed(VerifierConclusion::Rejected(VerifierEvidence::new(
            "static",
            "example/obligation.review@0",
            "text/plain",
            b"rejected".to_vec(),
            vec![],
        ))),
    );
    register(
        &mut rejected,
        "example/verifier.audit@0",
        "example/obligation.audit@0",
        evidence("example/obligation.audit@0"),
    );
    assert_eq!(
        verify(&compilation, &rejected).unwrap().state,
        VerificationState::Completed(VerificationDecision::Rejected)
    );

    let mut faulted = VerifierRegistry::new();
    register(
        &mut faulted,
        "example/verifier.review@0",
        "example/obligation.review@0",
        VerifierExecution::Faulted(Reason {
            code: "example/fault.policy-verifier@0".to_owned(),
            message: "policy verifier failed".to_owned(),
            details: None,
        }),
    );
    register(
        &mut faulted,
        "example/verifier.audit@0",
        "example/obligation.audit@0",
        evidence("example/obligation.audit@0"),
    );
    assert!(matches!(
        verify(&compilation, &faulted).unwrap().state,
        VerificationState::Faulted(_)
    ));
}

#[test]
fn producer_registration_and_binding_order_do_not_change_bundle_bytes() {
    let compilation = compilation();
    let mut first = VerifierRegistry::new();
    register(
        &mut first,
        "example/verifier.review@0",
        "example/obligation.review@0",
        evidence("example/obligation.review@0"),
    );
    register(
        &mut first,
        "example/verifier.audit@0",
        "example/obligation.audit@0",
        evidence("example/obligation.audit@0"),
    );
    let mut second = VerifierRegistry::new();
    register(
        &mut second,
        "example/verifier.audit@0",
        "example/obligation.audit@0",
        evidence("example/obligation.audit@0"),
    );
    register(
        &mut second,
        "example/verifier.review@0",
        "example/obligation.review@0",
        evidence("example/obligation.review@0"),
    );

    let first = verify(&compilation, &first).unwrap();
    let second = verify(&compilation, &second).unwrap();
    assert_eq!(first.bundle_bytes, second.bundle_bytes);
    assert_eq!(first.bundle_hash, second.bundle_hash);
    assert_eq!(first.payloads, second.payloads);
}

#[test]
fn duplicate_layer_rules_collapse_to_one_obligation_with_both_sources() {
    let policy = effective(
        r#"
§policy example/policy.organization@0 {
  layer organization;
  rule a-review: evidence add {
    obligation: example/obligation.review@0,
    classes: [static],
    minimum: 1,
    scope: { goals: [example/Review@0] }
  } nonwaivable;
}
§policy example/policy.team@0 {
  layer team;
  rule b-review: evidence add {
    obligation: example/obligation.review@0,
    classes: [static],
    minimum: 1,
    scope: { goals: [example/Review@0] }
  } nonwaivable;
}
"#,
    );
    let compilation = compile_source_with_policy(GOAL, "review.bhcp", &policy).unwrap();
    let mut registry = VerifierRegistry::new();
    register(
        &mut registry,
        "example/verifier.review@0",
        "example/obligation.review@0",
        evidence("example/obligation.review@0"),
    );

    let report = verify(&compilation, &registry).unwrap();
    assert_eq!(report.bundle.policy_obligations.len(), 1);
    let sources = &report.bundle.policy_obligations[0].sources;
    assert_eq!(sources.len(), 2);
    assert_eq!(sources[0].layer, "organization");
    assert_eq!(sources[1].layer, "team");
}

#[test]
fn policy_minimum_counts_distinct_bound_producers() {
    let policy = effective(
        r#"
§policy example/policy.organization@0 {
  layer organization;
  rule a-review: evidence add {
    obligation: example/obligation.review@0,
    classes: [static],
    minimum: 2,
    scope: { goals: [example/Review@0] }
  } nonwaivable;
}
"#,
    );
    let compilation = compile_source_with_policy(GOAL, "review.bhcp", &policy).unwrap();
    let mut one = VerifierRegistry::new();
    register(
        &mut one,
        "example/verifier.review-one@0",
        "example/obligation.review@0",
        evidence("example/obligation.review@0"),
    );
    let one = verify(&compilation, &one).unwrap();
    assert_eq!(
        one.state,
        VerificationState::Completed(VerificationDecision::Unresolved)
    );
    assert!(
        one.bundle
            .gaps
            .iter()
            .any(|gap| gap.reason.code == "bhcp.reason/policy-evidence-minimum@0")
    );

    let mut two = VerifierRegistry::new();
    register(
        &mut two,
        "example/verifier.review-two@0",
        "example/obligation.review@0",
        evidence("example/obligation.review@0"),
    );
    register(
        &mut two,
        "example/verifier.review-one@0",
        "example/obligation.review@0",
        evidence("example/obligation.review@0"),
    );
    assert_eq!(
        verify(&compilation, &two).unwrap().state,
        VerificationState::Completed(VerificationDecision::Accepted)
    );
}

#[test]
fn later_distinct_rule_for_same_symbol_remains_an_independent_demand() {
    let policy = effective(
        r#"
§policy example/policy.organization@0 {
  layer organization;
  rule a-review: evidence add {
    obligation: example/obligation.review@0,
    classes: [static],
    minimum: 1,
    scope: { goals: [example/Review@0] }
  } nonwaivable;
}
§policy example/policy.team@0 {
  layer team;
  rule b-review: evidence add {
    obligation: example/obligation.review@0,
    classes: [empirical],
    minimum: 1,
    scope: { goals: [example/Review@0] }
  } nonwaivable;
}
"#,
    );
    let compilation = compile_source_with_policy(GOAL, "review.bhcp", &policy).unwrap();
    let mut registry = VerifierRegistry::new();
    register(
        &mut registry,
        "example/verifier.review-static@0",
        "example/obligation.review@0",
        evidence_with_class("example/obligation.review@0", "static"),
    );
    register(
        &mut registry,
        "example/verifier.review-empirical@0",
        "example/obligation.review@0",
        evidence_with_class("example/obligation.review@0", "empirical"),
    );

    let report = verify(&compilation, &registry).unwrap();
    assert_eq!(
        report.state,
        VerificationState::Completed(VerificationDecision::Accepted)
    );
    assert_eq!(report.bundle.policy_obligations.len(), 2);
    assert_ne!(
        report.bundle.policy_obligations[0].id,
        report.bundle.policy_obligations[1].id
    );
    assert_eq!(
        report.bundle.policy_obligations[0].symbol,
        report.bundle.policy_obligations[1].symbol
    );
}

#[test]
fn verification_revalidates_the_retained_effective_policy() {
    let mut compilation = compilation();
    compilation
        .effective_policy
        .as_mut()
        .unwrap()
        .effective
        .evidence[0]
        .value
        .minimum = 2;
    let diagnostic = verify(&compilation, &VerifierRegistry::new()).unwrap_err();
    assert_eq!(diagnostic.code, "BHCP7001");
    assert!(diagnostic.message.contains("invalid effective policy"));
}

#[test]
fn retained_policy_document_cannot_be_removed_to_drop_evidence_demands() {
    let mut compilation = compilation();
    compilation.effective_policy = None;
    let diagnostic = verify(&compilation, &VerifierRegistry::new()).unwrap_err();
    assert_eq!(diagnostic.code, "BHCP7001");
    assert!(diagnostic.message.contains("retained effective policy"));
}
