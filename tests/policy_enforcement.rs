use bhcp::pipeline::{compile_source_with_policy, parse_policy_source};
use bhcp::policy::{EffectivePolicyDocument, compose_policies};
use bhcp::schema::validate_root;

const GOAL: &str = r#"
§goal example/Greet@0 {
  §input attempts: Integer;
  §allows bhcp-effect/fs-read@0;
  §forbids bhcp-effect/network@0;
  §limit example/limit.attempts@0: attempts <= 3;
}
"#;

fn effective(source: &str) -> EffectivePolicyDocument {
    let parsed = parse_policy_source(source, "policy.bhcp").unwrap();
    compose_policies(&parsed.documents, Default::default()).unwrap()
}

fn governing_policy(maximum: i64, mode: &str) -> EffectivePolicyDocument {
    effective(&format!(
        r#"
§policy example/policy.org@0 {{
  layer organization;
  rule a-capability: capability narrow {{
    effect: bhcp-effect/fs-read@0,
    scope: {{ goals: [example/Greet@0] }}
  }} nonwaivable;
  rule b-limit: limit tighten {{
    dimension: example/limit.attempts@0,
    unit: example/unit.count@0,
    maximum: ["integer", {maximum}],
    scope: {{ goals: [example/Greet@0] }}
  }} nonwaivable;
  rule c-mode: type-mode strengthen {mode} nonwaivable;
  rule d-prohibition: prohibition deny {{
    effect: bhcp-effect/network@0,
    scope: {{ goals: [example/Greet@0] }}
  }} nonwaivable;
  rule e-requirement: requirement add {{
    requirement: example/requirement.audit@0,
    scope: {{ goals: [example/Greet@0] }}
  }} nonwaivable;
}}
"#
    ))
}

#[test]
fn exact_policy_boundary_is_retained_in_valid_semantic_ir() {
    let policy = governing_policy(3, "infer-strict");
    let compiled = compile_source_with_policy(GOAL, "goal.bhcp", &policy).unwrap();
    let reference = compiled.ir.effective_policy.as_ref().unwrap();
    assert_eq!(reference.semantic_id, policy.header.semantic_id.clone().unwrap());
    assert_eq!(reference.artifact_id, policy.header.artifact_id.clone().unwrap());

    let decision = compiled.ir.goals[0].policy_decision.as_ref().unwrap();
    assert_eq!(decision.type_mode, "infer-strict");
    assert_eq!(decision.requirements, [0]);
    assert_eq!(decision.prohibitions, [0]);
    assert_eq!(decision.capabilities, [0]);
    assert_eq!(decision.limits, [0]);
    validate_root(&compiled.ir.to_value(true), "semantic-ir").unwrap();
}

#[test]
fn type_authority_prohibition_and_limit_denials_are_stable_and_emit_no_ir() {
    let strict = compile_source_with_policy(GOAL, "goal.bhcp", &governing_policy(3, "strict"))
        .unwrap_err();
    assert_eq!(strict.code, "BHCP8201");
    assert!(strict.message.contains("infer-strict"));
    assert!(strict.message.contains("strict"));

    let unavailable = GOAL.replace("fs-read", "fs-write");
    let unavailable =
        compile_source_with_policy(&unavailable, "goal.bhcp", &governing_policy(3, "infer-strict"))
            .unwrap_err();
    assert_eq!(unavailable.code, "BHCP8203");
    assert!(unavailable.message.contains("fs-write"));

    let network = GOAL.replace("fs-read@0", "network@0");
    let network = compile_source_with_policy(
        &network,
        "goal.bhcp",
        &effective(
            r#"
§policy example/policy.org@0 {
  layer organization;
  rule a-capability: capability narrow { effect: bhcp-effect/network@0 } nonwaivable;
  rule b-prohibition: prohibition deny { effect: bhcp-effect/network@0 } nonwaivable;
  rule c-mode: type-mode strengthen infer-strict nonwaivable;
}
"#,
        ),
    )
    .unwrap_err();
    assert_eq!(network.code, "BHCP8202");
    assert!(network.message.contains("network"));

    let loose = GOAL.replace("attempts <= 3", "attempts <= 4");
    let loose =
        compile_source_with_policy(&loose, "goal.bhcp", &governing_policy(3, "infer-strict"))
            .unwrap_err();
    assert_eq!(loose.code, "BHCP8204");
    assert!(loose.message.contains("example/limit.attempts@0"));
}

#[test]
fn governing_policy_is_semantic_even_without_a_local_limit_clause() {
    let source = GOAL
        .lines()
        .filter(|line| !line.contains("§limit"))
        .collect::<Vec<_>>()
        .join("\n");
    let three = compile_source_with_policy(
        &source,
        "goal.bhcp",
        &governing_policy(3, "infer-strict"),
    )
    .unwrap();
    let two = compile_source_with_policy(
        &source,
        "goal.bhcp",
        &governing_policy(2, "infer-strict"),
    )
    .unwrap();
    assert_ne!(three.ir.semantic_id, two.ir.semantic_id);
    assert_ne!(three.ir_bytes, two.ir_bytes);
}
