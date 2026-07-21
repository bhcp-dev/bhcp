use bhcp::hash::HashAlgorithm;
use bhcp::inspection::render_artifact;
use bhcp::model::ContentReference;
use bhcp::parser::parse_canonical;
use bhcp::pipeline::{
    compile_source, compile_source_with_policy, compile_source_with_waiver_decision_time,
};
use bhcp::policy::{apply_waiver, compose_policies};
use bhcp::schema::validate_root;

const POLICY: &str = r#"
§policy example/policy.org@0 {
  layer organization;
  rule a-requirement: requirement add {
    requirement: example/requirement.audit@0,
    scope: { goals: [example/Deploy@0] }
  } nonwaivable;
  rule b-evidence: evidence add {
    obligation: example/obligation.review@0,
    classes: [static],
    minimum: 1,
    scope: { goals: [example/Deploy@0] }
  } nonwaivable;
  rule c-prohibition: prohibition deny {
    effect: bhcp-effect/network@0,
    scope: { goals: [example/Deploy@0] }
  } nonwaivable;
  rule d-capability: capability narrow {
    effect: bhcp-effect/fs-read@0,
    scope: { goals: [example/Deploy@0] }
  } nonwaivable;
  rule e-limit: limit tighten {
    dimension: example/limit.attempts@0,
    unit: example/unit.count@0,
    maximum: ["integer", 3],
    scope: { goals: [example/Deploy@0] }
  } waivable by ["security"];
  rule f-type-mode: type-mode strengthen infer-strict nonwaivable;
}
"#;

const WAIVER: &str = r#"
§waiver example/waiver.attempts@0 {
  issuer "security";
  targets [{
    rule: [example/policy.org@0, "e-limit"],
    scope: { goals: [example/Deploy@0] },
    weakening: {
      category: "limit",
      operation: "loosen",
      from: {
        dimension: example/limit.attempts@0,
        unit: example/unit.count@0,
        maximum: ["integer", 3],
        scope: { goals: [example/Deploy@0] }
      },
      to: {
        dimension: example/limit.attempts@0,
        unit: example/unit.count@0,
        maximum: ["integer", 4],
        scope: { goals: [example/Deploy@0] }
      }
    }
  }];
  justification "one audited retry";
  issued_at time "2026-07-20T00:00:00Z";
  not_before time "2026-07-20T00:00:00Z";
  expires_at time "2026-07-22T00:00:00Z";
  authorization [{
    scheme: example/signature@0,
    issuer: "security",
    subject: {
      media_type: "application/vnd.bhcp.waiver+cbor",
      size: 1,
      digests: [{ algorithm: example/hash@0, digest: h'01' }]
    },
    signature: h'01'
  }];
  audit_reference {
    media_type: "application/vnd.bhcp.audit+cbor",
    size: 1,
    digests: [{ algorithm: example/hash@0, digest: h'02' }]
  };
}
"#;

const GOAL: &str = r#"
§goal example/Deploy@0 {
  §input attempts: Integer;
  §output accepted: Bool;
  §requires "nonnegative": attempts >= 0;
  §ensures "accepted": accepted == true;
  §allows bhcp-effect/fs-read@0;
  §forbids bhcp-effect/network@0;
  §limit example/limit.attempts@0: attempts <= 4;
}
"#;

fn governed_source() -> String {
    format!("{POLICY}\n{WAIVER}\n{GOAL}")
}

fn waiver_source(rule: &str, scope: bool, weakening: &str) -> String {
    let scope = if scope {
        "scope: { goals: [example/Deploy@0] },"
    } else {
        ""
    };
    format!(
        r#"
§waiver example/waiver.{rule}@0 {{
  issuer "security";
  targets [{{
    rule: [example/policy.org@0, "{rule}"],
    {scope}
    weakening: {weakening}
  }}];
  justification "reviewed source waiver";
  issued_at time "2026-07-20T00:00:00Z";
  not_before time "2026-07-20T00:00:00Z";
  expires_at time "2026-07-22T00:00:00Z";
  authorization [{{
    scheme: example/signature@0,
    issuer: "security",
    subject: {{
      media_type: "application/vnd.bhcp.waiver+cbor",
      size: 1,
      digests: [{{ algorithm: example/hash@0, digest: h'01' }}]
    }},
    signature: h'01'
  }}];
  audit_reference {{
    media_type: "application/vnd.bhcp.audit+cbor",
    size: 1,
    digests: [{{ algorithm: example/hash@0, digest: h'02' }}]
  }};
}}
"#
    )
}

fn canonical_effective(policy: &str, waiver: &str) -> bhcp::policy::EffectivePolicyDocument {
    let governance_source = format!("{policy}\n{waiver}");
    let governance = parse_canonical(
        &governance_source,
        "governance.bhcp",
        ContentReference::from_bytes(
            "text/vnd.bhcp.source",
            governance_source.as_bytes(),
            HashAlgorithm::default(),
        ),
    )
    .unwrap();
    let baseline = compose_policies(
        &governance
            .policies
            .iter()
            .map(|policy| policy.document.clone())
            .collect::<Vec<_>>(),
        HashAlgorithm::default(),
    )
    .unwrap();
    apply_waiver(
        &baseline,
        governance.waivers[0].document.as_ref().unwrap(),
        "2026-07-21T00:00:00Z",
        HashAlgorithm::default(),
    )
    .unwrap()
}

#[test]
fn source_policy_and_waiver_match_the_canonical_document_path() {
    let source = governed_source();
    let compiled =
        compile_source_with_waiver_decision_time(&source, "governed.bhcp", "2026-07-21T00:00:00Z")
            .unwrap();
    validate_root(&compiled.ir.to_value(true), "semantic-ir").unwrap();

    let governance_source = format!("{POLICY}\n{WAIVER}");
    let governance = parse_canonical(
        &governance_source,
        "governance.bhcp",
        ContentReference::from_bytes(
            "text/vnd.bhcp.source",
            governance_source.as_bytes(),
            HashAlgorithm::default(),
        ),
    )
    .unwrap();
    let baseline = compose_policies(
        &governance
            .policies
            .iter()
            .map(|policy| policy.document.clone())
            .collect::<Vec<_>>(),
        HashAlgorithm::default(),
    )
    .unwrap();
    let waiver = governance.waivers[0]
        .document
        .as_ref()
        .expect("materialized source waiver");
    let effective = apply_waiver(
        &baseline,
        waiver,
        "2026-07-21T00:00:00Z",
        HashAlgorithm::default(),
    )
    .unwrap();
    let canonical = compile_source_with_policy(GOAL, "goal.bhcp", &effective).unwrap();

    assert_eq!(compiled.ir.semantic_value(), canonical.ir.semantic_value());
    assert_eq!(compiled.semantic_hash, canonical.semantic_hash);
    assert_eq!(compiled.ir_bytes, canonical.ir_bytes);
    let decision = compiled.ir.goals[0].policy_decision.as_ref().unwrap();
    assert_eq!(decision.requirements, [0]);
    assert_eq!(decision.evidence, [0]);
    assert_eq!(decision.prohibitions, [0]);
    assert_eq!(decision.capabilities, [0]);
    assert_eq!(decision.limits, [0]);
    let inspection = render_artifact(&compiled.ir.to_value(true), None);
    assert!(inspection.contains("effective-policy semantic_id"));
    assert!(inspection.contains("policy requirements [0]"));
    assert!(inspection.contains("policy evidence [0]"));
}

#[test]
fn waiver_time_is_required_injected_and_fail_closed() {
    let source = governed_source();
    let missing = compile_source(&source, "missing-time.bhcp").unwrap_err();
    assert_eq!(missing.code, "BHCP8301");
    assert!(missing.message.contains("decision time"));

    let expired =
        compile_source_with_waiver_decision_time(&source, "expired.bhcp", "2026-07-22T00:00:00Z")
            .unwrap_err();
    assert_eq!(expired.code, "BHCP8304");

    let premature =
        compile_source_with_waiver_decision_time(&source, "premature.bhcp", "2026-07-19T23:59:59Z")
            .unwrap_err();
    assert_eq!(premature.code, "BHCP8304");

    let external = canonical_effective(POLICY, WAIVER);
    let conflict =
        compile_source_with_policy(&source, "conflicting-policy.bhcp", &external).unwrap_err();
    assert_eq!(conflict.code, "BHCP8110");
}

#[test]
fn delegated_source_authority_matches_canonical_application() {
    let delegated = WAIVER
        .replacen("issuer \"security\";", "issuer \"release-manager\";", 1)
        .replacen(
            "justification \"one audited retry\";",
            r#"justification "one audited retry";
  authority_chain [{
    delegator: "security",
    delegate: "release-manager",
    authorization: {
      media_type: "application/vnd.bhcp.delegation+cbor",
      size: 1,
      digests: [{ algorithm: example/hash@0, digest: h'03' }]
    }
  }];"#,
            1,
        )
        .replacen("issuer: \"security\"", "issuer: \"release-manager\"", 1);
    let source = format!("{POLICY}\n{delegated}\n{GOAL}");
    let from_source =
        compile_source_with_waiver_decision_time(&source, "delegated.bhcp", "2026-07-21T00:00:00Z")
            .unwrap();
    let canonical =
        compile_source_with_policy(GOAL, "goal.bhcp", &canonical_effective(POLICY, &delegated))
            .unwrap();
    assert_eq!(from_source.semantic_hash, canonical.semantic_hash);
    assert_eq!(from_source.ir_bytes, canonical.ir_bytes);
}

#[test]
fn policy_and_waiver_presentation_is_artifact_only() {
    let baseline = governed_source();
    let presented = baseline
        .replace("rule e-limit:", "rule e-limit \"retry ceiling\":")
        .replace("one audited retry", "same authority, different explanation")
        .replace(
            "§goal example/Deploy@0",
            "// presentation\n§goal example/Deploy@0",
        );
    let baseline = compile_source_with_waiver_decision_time(
        &baseline,
        "baseline.bhcp",
        "2026-07-21T00:00:00Z",
    )
    .unwrap();
    let presented = compile_source_with_waiver_decision_time(
        &presented,
        "presented.bhcp",
        "2026-07-21T00:00:00Z",
    )
    .unwrap();
    assert_eq!(baseline.semantic_hash, presented.semantic_hash);
    assert_ne!(baseline.ast.artifact_id, presented.ast.artifact_id);
    assert_ne!(baseline.ir_bytes, presented.ir_bytes);
}

#[test]
fn unwaived_inline_policy_enforces_goal_expressions_before_ir() {
    let source = format!("{POLICY}\n{GOAL}");
    let diagnostic = compile_source(&source, "unwaived.bhcp").unwrap_err();
    assert_eq!(diagnostic.code, "BHCP8204");
    assert!(diagnostic.message.contains("example/limit.attempts@0"));
}

#[test]
fn inline_policy_composes_with_derived_extension_lowering() {
    let source = format!(
        "{POLICY}\n{}",
        include_str!("../conformance/v0/reference-program/extension.bhcp")
    );
    let compiled = compile_source(&source, "policy-extension.bhcp").unwrap();
    let goal = compiled
        .ir
        .goals
        .iter()
        .find(|goal| goal.symbol == "bhcp.reference/review@0")
        .expect("derived extension goal");
    assert!(goal.policy_decision.is_some());
    assert!(goal.body.is_some());
}

#[test]
fn all_six_source_waiver_categories_match_canonical_application() {
    let policy = POLICY.replace("nonwaivable;", "waivable by [\"security\"];");
    let strict_policy = policy.replace(
        "type-mode strengthen infer-strict waivable",
        "type-mode strengthen strict waivable",
    );
    let safe_goal = GOAL.replace("attempts <= 4", "attempts <= 3");
    let scope = "scope: { goals: [example/Deploy@0] }";
    let cases = vec![
        (
            "a-requirement",
            policy.clone(),
            format!(
                "{{ category: \"requirement\", operation: \"remove\", value: {{ requirement: example/requirement.audit@0, {scope} }} }}"
            ),
            true,
        ),
        (
            "b-evidence",
            policy.clone(),
            format!(
                "{{ category: \"evidence\", operation: \"remove\", value: {{ obligation: example/obligation.review@0, classes: [static], minimum: 1, {scope} }} }}"
            ),
            true,
        ),
        (
            "c-prohibition",
            policy.clone(),
            format!(
                "{{ category: \"prohibition\", operation: \"allow\", value: {{ effect: bhcp-effect/network@0, {scope} }} }}"
            ),
            true,
        ),
        (
            "d-capability",
            policy.clone(),
            format!(
                "{{ category: \"capability\", operation: \"broaden\", from: {{ effect: bhcp-effect/fs-read@0, {scope} }}, to: {{ effect: bhcp-effect/fs-read@0 }} }}"
            ),
            true,
        ),
        (
            "e-limit",
            policy,
            format!(
                "{{ category: \"limit\", operation: \"loosen\", from: {{ dimension: example/limit.attempts@0, unit: example/unit.count@0, maximum: [\"integer\", 3], {scope} }}, to: {{ dimension: example/limit.attempts@0, unit: example/unit.count@0, maximum: [\"integer\", 4], {scope} }} }}"
            ),
            true,
        ),
        (
            "f-type-mode",
            strict_policy,
            "{ category: \"type-mode\", operation: \"weaken\", from: strict, to: infer-strict }"
                .to_owned(),
            false,
        ),
    ];

    for (rule, policy, weakening, scoped) in cases {
        let waiver = waiver_source(rule, scoped, &weakening);
        let source = format!("{policy}\n{waiver}\n{safe_goal}");
        let from_source = compile_source_with_waiver_decision_time(
            &source,
            &format!("{rule}.bhcp"),
            "2026-07-21T00:00:00Z",
        )
        .unwrap();
        let canonical = compile_source_with_policy(
            &safe_goal,
            "goal.bhcp",
            &canonical_effective(&policy, &waiver),
        )
        .unwrap();
        assert_eq!(from_source.semantic_hash, canonical.semantic_hash, "{rule}");
        assert_eq!(from_source.ir_bytes, canonical.ir_bytes, "{rule}");
    }
}

#[test]
fn governance_order_normalizes_and_unresolved_or_partial_waivers_fail_closed() {
    let ordered = governed_source();
    let reversed = format!("{GOAL}\n{WAIVER}\n{POLICY}");
    let ordered =
        compile_source_with_waiver_decision_time(&ordered, "ordered.bhcp", "2026-07-21T00:00:00Z")
            .unwrap();
    let reversed = compile_source_with_waiver_decision_time(
        &reversed,
        "reversed.bhcp",
        "2026-07-21T00:00:00Z",
    )
    .unwrap();
    assert_eq!(ordered.semantic_hash, reversed.semantic_hash);
    assert_eq!(ordered.ir_bytes, reversed.ir_bytes);
    assert_ne!(ordered.ast.artifact_id, reversed.ast.artifact_id);

    let unresolved = format!(
        "{POLICY}\n{}\n{GOAL}",
        include_str!("../conformance/v0/reference-program/waiver.bhcp")
    );
    let diagnostic = compile_source_with_waiver_decision_time(
        &unresolved,
        "unresolved-waiver.bhcp",
        "2026-07-21T00:00:00Z",
    )
    .unwrap_err();
    assert_eq!(diagnostic.code, "BHCP8301");
    assert!(diagnostic.message.contains("unresolved symbolic"));

    let partial = WAIVER.replacen(
        "scope: { goals: [example/Deploy@0] },",
        "scope: { goals: [] },",
        1,
    );
    let diagnostic = compile_source_with_waiver_decision_time(
        &format!("{POLICY}\n{partial}\n{GOAL}"),
        "partial-waiver.bhcp",
        "2026-07-21T00:00:00Z",
    )
    .unwrap_err();
    assert_eq!(diagnostic.code, "BHCP8303");
}
