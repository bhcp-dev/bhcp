use bhcp::model::BhcpType;
use bhcp::pipeline::{compile_source, compile_source_with_policy, parse_policy_source};
use bhcp::policy::{EffectivePolicyDocument, compose_policies};
use bhcp::value::Value;

fn effective(source: &str) -> EffectivePolicyDocument {
    let parsed = parse_policy_source(source, "policy.bhcp").unwrap();
    compose_policies(&parsed.documents, Default::default()).unwrap()
}

fn effect_ids(compiled: &bhcp::pipeline::Compilation, symbol: &str) -> Vec<String> {
    let value = compiled.ir.to_value(false);
    let Value::Array(goals) = value.get("goals").unwrap() else {
        panic!("semantic IR goals must be an array")
    };
    let goal = goals
        .iter()
        .find(|goal| goal.get("symbol") == Some(&Value::Text(symbol.to_owned())))
        .unwrap();
    let Value::Array(effects) = goal.get("effects").unwrap().get("effects").unwrap() else {
        panic!("goal effects must be an array")
    };
    effects
        .iter()
        .map(|effect| match effect.get("id") {
            Some(Value::Text(id)) => id.clone(),
            _ => panic!("effect requires an ID"),
        })
        .collect()
}

#[test]
fn eff_01_child_effects_propagate_without_becoming_kernel_metadata() {
    let source = r#"
§goal example/Read@0 {
    §output value: Text;
    §allows bhcp-effect/fs.read@0;
}
§goal example/Parent@0 {
    §output child: { value: Text };
    §all {
        child = example/Read@0();
    };
}
"#;
    let compiled = compile_source(source, "effects.bhcp").unwrap();
    assert_eq!(
        effect_ids(&compiled, "example/Read@0"),
        ["bhcp-effect/fs.read@0"]
    );
    assert_eq!(
        effect_ids(&compiled, "example/Parent@0"),
        ["bhcp-effect/fs.read@0"]
    );
    let parent = compiled
        .ir
        .goals
        .iter()
        .find(|goal| goal.symbol == "example/Parent@0")
        .unwrap();
    let network = parent.body.as_ref().unwrap().to_value();
    assert!(network.get("effects").is_none());
    assert!(network.get("budgets").is_none());
}

#[test]
fn composite_allows_are_ceilings_not_invented_possible_effects() {
    let source = r#"
§goal example/Read@0 {
    §output value: Text;
    §allows bhcp-effect/fs.read@0;
}
§goal example/Parent@0 {
    §output child: { value: Text };
    §allows bhcp-effect/fs.read@0, bhcp-effect/fs.write@0;
    §all { child = example/Read@0(); };
}
"#;
    let compiled = compile_source(source, "effects.bhcp").unwrap();
    assert_eq!(
        effect_ids(&compiled, "example/Parent@0"),
        ["bhcp-effect/fs.read@0"]
    );
}

#[test]
fn semantic_ir_validation_rejects_hidden_materialized_effects() {
    let source = r#"
§goal example/Read@0 {
    §output value: Text;
    §allows bhcp-effect/fs.read@0;
}
§goal example/Parent@0 {
    §output child: { value: Text };
    §all { child = example/Read@0(); };
}
"#;
    let compiled = compile_source(source, "effects.bhcp").unwrap();
    let mut tampered = compiled.ir;
    tampered
        .goals
        .iter_mut()
        .find(|goal| goal.symbol == "example/Parent@0")
        .unwrap()
        .effects
        .effects
        .clear();
    let diagnostic = tampered.validate().unwrap_err();
    assert_eq!(diagnostic.code, "BHCP4001");
    assert!(diagnostic.message.contains("materialized effect row"));
}

#[test]
fn eff_03_parent_prohibitions_and_explicit_ceilings_deny_child_excess() {
    let prohibited = r#"
§goal example/Network@0 { §output value: Text; §allows bhcp-effect/network@0; }
§goal example/Parent@0 {
    §output child: { value: Text };
    §forbids bhcp-effect/network@0;
    §all { child = example/Network@0(); };
}
"#;
    let diagnostic = compile_source(prohibited, "effects.bhcp").unwrap_err();
    assert_eq!(diagnostic.code, "BHCP4502");
    assert!(diagnostic.message.contains("example/Parent@0"));
    assert!(diagnostic.message.contains("bhcp-effect/network@0"));

    let excess = r#"
§goal example/Write@0 { §output value: Text; §allows bhcp-effect/fs.write@0; }
§goal example/Parent@0 {
    §output child: { value: Text };
    §allows bhcp-effect/fs.read@0;
    §all { child = example/Write@0(); };
}
"#;
    let diagnostic = compile_source(excess, "effects.bhcp").unwrap_err();
    assert_eq!(diagnostic.code, "BHCP4502");
    assert!(diagnostic.message.contains("ceiling"));
    assert!(diagnostic.message.contains("bhcp-effect/fs.write@0"));
}

#[test]
fn deny_wins_independent_of_clause_order() {
    for clauses in [
        "§allows bhcp-effect/network@0; §forbids bhcp-effect/network@0;",
        "§forbids bhcp-effect/network@0; §allows bhcp-effect/network@0;",
    ] {
        let source = format!("§goal example/G@0 {{ {clauses} }}");
        let diagnostic = compile_source(&source, "effects.bhcp").unwrap_err();
        assert_eq!(diagnostic.code, "BHCP4502");
    }
}

#[test]
fn effect_sets_are_canonical_and_resource_scopes_retain_typed_bindings() {
    let first = r#"
§goal example/G@0 {
    §input repository: owned example/Repository@0;
    §allows bhcp-effect/fs.write@0(repository), bhcp-effect/fs.read@0(repository);
}
"#;
    let second = first.replace(
        "bhcp-effect/fs.write@0(repository), bhcp-effect/fs.read@0(repository)",
        "bhcp-effect/fs.read@0(repository), bhcp-effect/fs.write@0(repository)",
    );
    let first = compile_source(first, "effects.bhcp").unwrap();
    let second = compile_source(&second, "effects.bhcp").unwrap();
    assert_eq!(first.semantic_hash, second.semantic_hash);
    assert_eq!(
        effect_ids(&first, "example/G@0"),
        ["bhcp-effect/fs.read@0", "bhcp-effect/fs.write@0"]
    );
    let Value::Map(entries) = first.ir.to_value(false) else {
        unreachable!()
    };
    let Value::Array(goals) = entries
        .iter()
        .find(|(key, _)| key == "goals")
        .unwrap()
        .1
        .clone()
    else {
        unreachable!()
    };
    let Value::Array(effects) = goals[0].get("effects").unwrap().get("effects").unwrap() else {
        unreachable!()
    };
    assert!(effects.iter().all(|effect| matches!(effect.get("resource"), Some(Value::Text(id)) if id.starts_with("binding-"))));
}

#[test]
fn effect_resource_coordinates_require_nominal_resource_bindings() {
    let source = r#"
§goal example/Read@0 {
    §input path: Text;
    §allows bhcp-effect/fs.read@0(path);
}
"#;
    let diagnostic = compile_source(source, "effects.bhcp").unwrap_err();
    assert_eq!(diagnostic.code, "BHCP4501");
    assert!(diagnostic.message.contains("path"));
    assert!(diagnostic.message.contains("resource"));
}

#[test]
fn retained_ir_rejects_non_resource_effect_coordinates() {
    let source = r#"
§goal example/Read@0 {
    §input repository: example/Repository@0;
    §allows bhcp-effect/fs.read@0(repository);
}
"#;
    let compiled = compile_source(source, "effects.bhcp").unwrap();
    let mut tampered = compiled.ir;
    let binding = tampered.goals[0]
        .clauses
        .iter_mut()
        .find_map(|clause| match &mut clause.kind {
            bhcp::model::ClauseKind::Fact { binding, .. } => Some(binding),
            _ => None,
        })
        .unwrap();
    binding.value_type = BhcpType::Primitive("Text");
    let diagnostic = tampered.validate().unwrap_err();
    assert_eq!(diagnostic.code, "BHCP4001");
    assert!(diagnostic.message.contains("effect row resource"));
}

#[test]
fn policy_resource_scopes_match_the_referenced_resource_type() {
    let source = r#"
§goal example/G@0 {
    §input repository: owned example/Repository@0;
    §allows bhcp-effect/fs.read@0(repository);
}
"#;
    let policy = effective(
        r#"
§policy example/P@0 {
    layer organization;
    rule a-read: capability narrow {
        effect: bhcp-effect/fs.read@0,
        scope: { goals: [example/G@0], resources: [example/Repository@0] }
    } nonwaivable;
    rule b-mode: type-mode strengthen infer-strict nonwaivable;
}
"#,
    );
    compile_source_with_policy(source, "effects.bhcp", &policy).unwrap();
}

#[test]
fn child_resource_effects_project_to_the_parent_binding() {
    let source = r#"
§goal example/Read@0 {
    §input repository: example/Repository@0;
    §output value: Text;
    §allows bhcp-effect/fs.read@0(repository);
}
§goal example/Parent@0 {
    §input enabled: Bool;
    §input repository: example/Repository@0;
    §allows bhcp-effect/fs.read@0(repository);
    §gate when enabled {
        child = example/Read@0(repository = borrow repository);
    };
}
"#;
    let compiled = compile_source(source, "effects.bhcp").unwrap();
    let parent = compiled
        .ir
        .goals
        .iter()
        .find(|goal| goal.symbol == "example/Parent@0")
        .unwrap();
    let parent_resource = parent.clauses.iter().find_map(|clause| match &clause.kind {
        bhcp::model::ClauseKind::Fact { binding, .. } if binding.name == "repository" => {
            Some(binding.id.as_str())
        }
        _ => None,
    });
    assert_eq!(
        parent.effects.effects[0].resource.as_deref(),
        parent_resource
    );
    assert_eq!(parent.effects.effects.len(), 1);
}

#[test]
fn policy_operation_scopes_match_literal_effect_coordinates() {
    let source = r#"
§goal example/G@0 {
    §allows bhcp-effect/process@0("example/operation.read@0");
}
"#;
    let policy = effective(
        r#"
§policy example/P@0 {
    layer organization;
    rule a-process: capability narrow {
        effect: bhcp-effect/process@0,
        scope: { goals: [example/G@0], operations: [example/operation.read@0] }
    } nonwaivable;
    rule b-mode: type-mode strengthen infer-strict nonwaivable;
}
"#,
    );
    compile_source_with_policy(source, "effects.bhcp", &policy).unwrap();

    let outside = source.replace("operation.read", "operation.write");
    let diagnostic = compile_source_with_policy(&outside, "effects.bhcp", &policy).unwrap_err();
    assert_eq!(diagnostic.code, "BHCP8203");
}

#[test]
fn propagated_effects_require_policy_authority_at_every_goal_boundary() {
    let source = r#"
§goal example/Read@0 { §output value: Text; §allows bhcp-effect/fs.read@0; }
§goal example/Parent@0 {
    §output child: { value: Text };
    §all { child = example/Read@0(); };
}
"#;
    let policy = effective(
        r#"
§policy example/P@0 {
    layer organization;
    rule a-read: capability narrow {
        effect: bhcp-effect/fs.read@0,
        scope: { goals: [example/Read@0] }
    } nonwaivable;
    rule b-mode: type-mode strengthen infer-strict nonwaivable;
}
"#,
    );
    let diagnostic = compile_source_with_policy(source, "effects.bhcp", &policy).unwrap_err();
    assert_eq!(diagnostic.code, "BHCP8203");
    assert!(diagnostic.message.contains("example/Parent@0"));
}

#[test]
fn eff_02_unsafe_effects_remain_visible_as_unresolved_evidence() {
    let source = r#"
§goal example/Unsafe@0 {
    §requires "ready": true;
    §allows bhcp-effect/unsafe@0;
    §verify "check": with example/verifier@0 for "ready";
}
"#;
    let compiled = compile_source(source, "effects.bhcp").unwrap();
    assert_eq!(
        effect_ids(&compiled, "example/Unsafe@0"),
        ["bhcp-effect/unsafe@0"]
    );
    let goal = &compiled.ir.goals[0];
    let BhcpType::Evidence(classes) = &goal.evidence else {
        panic!("goal evidence must be an evidence type")
    };
    assert!(classes.contains(&"static".to_owned()));
    assert!(classes.contains(&"unresolved".to_owned()));
}

#[test]
fn dimensioned_limits_must_be_direct_non_negative_exact_budgets() {
    for condition in ["attempts < 3", "attempts <= -1"] {
        let source = format!(
            r#"
§goal example/G@0 {{
    §input attempts: Integer;
    §limit example/limit.attempts@0: {condition};
}}
"#
        );
        let diagnostic = compile_source(&source, "effects.bhcp").unwrap_err();
        assert_eq!(diagnostic.code, "BHCP4503");
        assert!(diagnostic.message.contains("example/limit.attempts@0"));
    }
}

#[test]
fn pareto_preferences_at_one_priority_require_compatible_objective_types() {
    let invalid = r#"
§goal example/G@0 {
    §input attempts: Integer;
    §prefer 1: attempts;
    §prefer 1: true;
}
"#;
    let diagnostic = compile_source(invalid, "effects.bhcp").unwrap_err();
    assert_eq!(diagnostic.code, "BHCP4504");
    assert!(diagnostic.message.contains("priority 1"));

    let valid = invalid.replace("§prefer 1: true", "§prefer 2: true");
    compile_source(&valid, "effects.bhcp").unwrap();
}
