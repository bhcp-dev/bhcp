use std::collections::BTreeMap;

use bhcp::cbor::encode_deterministic;
use bhcp::graph::GraphDocument;
use bhcp::hash::{HashAlgorithm, artifact_hash_with, semantic_hash_with};
use bhcp::model::{BhcpType, Clause, ClauseKind, ExecutionPattern, Expression, ExpressionForm};
use bhcp::obligation::{build_obligation_graph, validate_obligation_graph};
use bhcp::pipeline::{
    Compilation, compile_source, compile_source_with_policy, parse_policy_source,
};
use bhcp::policy::{EffectivePolicyDocument, compose_policies};
use bhcp::value::Value;

const PROGRAM: &str = r#"
§goal example/Child@0 {
    §output value: Bool;
    §requires "child-ready": true;
    §ensures "child-done": value;
    §limit "child-attempts": example/limit.attempts@0: 1 <= 2;
}

§goal example/Parent@0 {
    §output child: { value: Bool };
    §requires "parent-ready": true;
    §ensures "parent-done": true;
    §verify "all-contracts": with example/verifier.all@0;
    §verify "selected-contracts": with example/verifier.selected@0
        for "parent-done", "parent-ready";
    §all {
        child = example/Child@0();
    };
}
"#;

const POLICY: &str = r#"
§policy example/policy.organization@0 {
    layer organization;
    rule a-requirement: requirement add {
        requirement: example/requirement.review@0
    } nonwaivable;
    rule b-evidence: evidence add {
        obligation: example/obligation.audit@0,
        classes: [formal, static],
        minimum: 2
    } nonwaivable;
    rule c-limit: limit tighten {
        dimension: example/limit.attempts@0,
        unit: example/unit.count@0,
        maximum: ["integer", 3]
    } nonwaivable;
}

§policy example/policy.team@0 {
    layer team;
    rule a-requirement: requirement add {
        requirement: example/requirement.review@0
    } nonwaivable;
    rule b-evidence: evidence add {
        obligation: example/obligation.audit@0,
        classes: [formal, static],
        minimum: 2
    } nonwaivable;
    rule c-limit: limit tighten {
        dimension: example/limit.attempts@0,
        unit: example/unit.count@0,
        maximum: ["integer", 3]
    } nonwaivable;
}
"#;

fn policy() -> EffectivePolicyDocument {
    let parsed = parse_policy_source(POLICY, "policy.bhcp").unwrap();
    compose_policies(&parsed.documents, Default::default()).unwrap()
}

fn compilation() -> Compilation {
    with_retained_invariants(
        compile_source_with_policy(PROGRAM, "obligations.bhcp", &policy()).unwrap(),
    )
}

fn rematerialize(mut compilation: Compilation) -> Compilation {
    let algorithm = HashAlgorithm::from_id(&compilation.semantic_hash.algorithm).unwrap();
    compilation.semantic_hash = semantic_hash_with(&compilation.ir, algorithm).unwrap();
    compilation.ir.semantic_id = Some(compilation.semantic_hash.clone());
    compilation.ir_hash = artifact_hash_with(&compilation.ir.to_value(false), algorithm).unwrap();
    compilation.ir.artifact_id = Some(compilation.ir_hash.clone());
    compilation.ir_bytes = encode_deterministic(&compilation.ir.to_value(true)).unwrap();
    compilation.ir.validate().unwrap();
    compilation
}

fn with_retained_invariants(mut compilation: Compilation) -> Compilation {
    for (symbol, suffix) in [("example/Child@0", "child"), ("example/Parent@0", "parent")] {
        let goal = compilation
            .ir
            .goals
            .iter_mut()
            .find(|goal| goal.symbol == symbol)
            .unwrap();
        goal.clauses.push(Clause {
            id: format!("retained-invariant-{suffix}"),
            label: Some(format!("{suffix}-stable")),
            kind: ClauseKind::Contract {
                kind: "invariant",
                dimension: None,
                condition: Expression {
                    id: format!("retained-invariant-expression-{suffix}"),
                    value_type: BhcpType::Primitive("Bool"),
                    form: ExpressionForm::Literal(Value::Bool(true)),
                },
            },
        });
    }
    rematerialize(compilation)
}

fn with_retained_case(mut compilation: Compilation, source_id: &str) -> Compilation {
    let parent = compilation
        .ir
        .goals
        .iter_mut()
        .find(|goal| goal.symbol == "example/Parent@0")
        .unwrap();
    parent.clauses.push(Clause {
        id: source_id.to_owned(),
        label: Some("scenario-label".to_owned()),
        kind: ClauseKind::Case {
            inputs: BTreeMap::new(),
            expected: ExecutionPattern::Completed("satisfied"),
        },
    });
    rematerialize(compilation)
}

fn text<'a>(value: &'a Value, field: &str) -> &'a str {
    match value.get(field) {
        Some(Value::Text(value)) => value,
        _ => panic!("missing text field {field}"),
    }
}

fn array<'a>(value: &'a Value, field: &str) -> &'a [Value] {
    match value.get(field) {
        Some(Value::Array(values)) => values,
        _ => panic!("missing array field {field}"),
    }
}

fn without_identities(mut value: Value) -> Value {
    let Value::Map(entries) = &mut value else {
        unreachable!()
    };
    entries.retain(|(field, _)| field != "semantic_id" && field != "artifact_id");
    value
}

fn field_mut<'a>(value: &'a mut Value, field: &str) -> &'a mut Value {
    let Value::Map(entries) = value else {
        panic!("expected map")
    };
    &mut entries
        .iter_mut()
        .find(|(name, _)| name == field)
        .unwrap()
        .1
}

#[test]
fn complete_source_policy_verifier_and_parent_child_obligations_are_structural() {
    let compilation = compilation();
    let graph = build_obligation_graph(&compilation).unwrap();

    assert!(graph.semantic_id().is_some());
    assert!(graph.artifact_id().is_some());
    assert!(
        graph
            .nodes()
            .iter()
            .all(|node| text(node.value(), "status") == "open")
    );

    let kinds = graph
        .nodes()
        .iter()
        .map(|node| node.kind.as_str())
        .collect::<Vec<_>>();
    for kind in [
        "requirement",
        "guarantee",
        "invariant",
        "limit",
        "verification",
        "discharge",
    ] {
        assert!(kinds.contains(&kind), "missing {kind} node");
    }

    let policy_nodes = graph
        .nodes()
        .iter()
        .filter(|node| node.value().get("policy").is_some())
        .collect::<Vec<_>>();
    assert_eq!(policy_nodes.len(), 3);
    for node in policy_nodes {
        let policy = node.value().get("policy").unwrap();
        assert_eq!(array(policy, "sources").len(), 2);
    }

    let verifier_edges = graph
        .edges()
        .iter()
        .filter(|edge| edge.kind == "verifies")
        .collect::<Vec<_>>();
    assert_eq!(verifier_edges.len(), 5);
    assert!(graph.edges().iter().any(|edge| edge.kind == "depends-on"));
    validate_obligation_graph(&compilation, &graph).unwrap();
}

#[test]
fn retained_cases_are_analysis_nodes_not_fabricated_obligations() {
    let first = build_obligation_graph(&with_retained_case(compilation(), "case-audit-1")).unwrap();
    let second =
        build_obligation_graph(&with_retained_case(compilation(), "case-audit-99")).unwrap();
    let first_case = first
        .nodes()
        .iter()
        .find(|node| node.kind == "case")
        .unwrap();
    let second_case = second
        .nodes()
        .iter()
        .find(|node| node.kind == "case")
        .unwrap();
    assert_eq!(first_case.id, second_case.id);
    assert_eq!(text(first_case.value(), "status"), "open");
    assert!(
        first
            .edges()
            .iter()
            .all(|edge| edge.to != first_case.id || edge.kind != "verifies")
    );

    let mut invalid = compilation();
    invalid
        .ir
        .goals
        .iter_mut()
        .find(|goal| goal.symbol == "example/Parent@0")
        .unwrap()
        .clauses
        .push(Clause {
            id: "invalid-case".to_owned(),
            label: None,
            kind: ClauseKind::Case {
                inputs: BTreeMap::from([("not-an-input".to_owned(), Value::Bool(true))]),
                expected: ExecutionPattern::Completed("forged"),
            },
        });
    assert_eq!(invalid.ir.validate().unwrap_err().code, "BHCP4001");
}

#[test]
fn labels_target_order_and_policy_decomposition_are_not_graph_semantics() {
    let reordered = PROGRAM.replace(
        "for \"parent-done\", \"parent-ready\";",
        "for \"parent-ready\", \"parent-done\";",
    );
    let renamed = reordered
        .replace("\"parent-done\": true", "\"finished\": true")
        .replace("\"parent-ready\": true", "\"prepared\": true")
        .replace(
            "for \"parent-ready\", \"parent-done\";",
            "for \"prepared\", \"finished\";",
        );
    let first = with_retained_invariants(
        compile_source_with_policy(&reordered, "first.bhcp", &policy()).unwrap(),
    );
    let second = with_retained_invariants(
        compile_source_with_policy(&renamed, "second.bhcp", &policy()).unwrap(),
    );
    let first = build_obligation_graph(&first).unwrap();
    let second = build_obligation_graph(&second).unwrap();
    assert_eq!(first.semantic_id(), second.semantic_id());

    let unrelated = r#"
§goal example/Unrelated@0 {
    §output value: Bool;
    §requires "noise-ready": true;
    §ensures "noise-done": value;
}
"#;
    let reordered_clauses = PROGRAM.replace(
        "    §requires \"child-ready\": true;\n    §ensures \"child-done\": value;",
        "    §ensures \"child-done\": value;\n    §requires \"child-ready\": true;",
    );
    let before = format!("{unrelated}{PROGRAM}");
    let after = format!("{reordered_clauses}{unrelated}");
    let before = build_obligation_graph(&with_retained_invariants(
        compile_source_with_policy(&before, "before-reorder.bhcp", &policy()).unwrap(),
    ))
    .unwrap();
    let after = build_obligation_graph(&with_retained_invariants(
        compile_source_with_policy(&after, "after-reorder.bhcp", &policy()).unwrap(),
    ))
    .unwrap();
    assert_eq!(
        before
            .nodes()
            .iter()
            .map(|node| &node.id)
            .collect::<Vec<_>>(),
        after
            .nodes()
            .iter()
            .map(|node| &node.id)
            .collect::<Vec<_>>()
    );
    let single_source_policy = POLICY.replace(
        "§policy example/policy.team@0 {\n    layer team;\n    rule a-requirement: requirement add {\n        requirement: example/requirement.review@0\n    } nonwaivable;\n    rule b-evidence: evidence add {\n        obligation: example/obligation.audit@0,\n        classes: [formal, static],\n        minimum: 2\n    } nonwaivable;\n    rule c-limit: limit tighten {\n        dimension: example/limit.attempts@0,\n        unit: example/unit.count@0,\n        maximum: [\"integer\", 3]\n    } nonwaivable;\n}\n",
        "",
    );
    let parsed = parse_policy_source(&single_source_policy, "single-policy.bhcp").unwrap();
    let single = compose_policies(&parsed.documents, Default::default()).unwrap();
    let single = with_retained_invariants(
        compile_source_with_policy(PROGRAM, "single.bhcp", &single).unwrap(),
    );
    let single = build_obligation_graph(&single).unwrap();
    let duplicate = build_obligation_graph(&compilation()).unwrap();
    assert_eq!(single.semantic_id(), duplicate.semantic_id());
    assert_ne!(single.artifact_id(), duplicate.artifact_id());
}

#[test]
fn malformed_or_fabricated_graphs_fail_before_planning() {
    let compilation = compilation();
    let graph = build_obligation_graph(&compilation).unwrap();

    let mut fabricated = without_identities(graph.to_value());
    let Value::Array(nodes) = field_mut(&mut fabricated, "nodes") else {
        unreachable!()
    };
    let Value::Map(entries) = &mut nodes[0] else {
        unreachable!()
    };
    let status = entries
        .iter_mut()
        .find(|(field, _)| field == "status")
        .unwrap();
    status.1 = Value::Text("discharged".to_owned());
    let fabricated = GraphDocument::from_value(&fabricated).unwrap();
    assert_eq!(
        validate_obligation_graph(&compilation, &fabricated)
            .unwrap_err()
            .code,
        "BHCP7103"
    );

    let mut missing_dependency = without_identities(graph.to_value());
    let Value::Array(edges) = field_mut(&mut missing_dependency, "edges") else {
        unreachable!()
    };
    edges.pop();
    let missing_dependency = GraphDocument::from_value(&missing_dependency).unwrap();
    assert_eq!(
        validate_obligation_graph(&compilation, &missing_dependency)
            .unwrap_err()
            .code,
        "BHCP7103"
    );

    let unknown = PROGRAM.replace("for \"parent-done\"", "for \"missing\"");
    assert_eq!(
        compile_source(&unknown, "unknown-label.bhcp")
            .unwrap_err()
            .code,
        "BHCP2001"
    );

    let mut duplicate = without_identities(graph.to_value());
    let Value::Array(nodes) = field_mut(&mut duplicate, "nodes") else {
        unreachable!()
    };
    nodes.push(nodes[0].clone());
    assert_eq!(
        GraphDocument::from_value(&duplicate).unwrap_err().code,
        "BHCP7002"
    );

    let mut dangling = without_identities(graph.to_value());
    let Value::Array(edges) = field_mut(&mut dangling, "edges") else {
        unreachable!()
    };
    let Value::Map(entries) = &mut edges[0] else {
        unreachable!()
    };
    entries
        .iter_mut()
        .find(|(field, _)| field == "to")
        .unwrap()
        .1 = Value::Text("missing-obligation".to_owned());
    assert_eq!(
        GraphDocument::from_value(&dangling).unwrap_err().code,
        "BHCP7003"
    );
}

#[test]
fn retained_policy_and_ir_envelopes_must_match_before_graph_construction() {
    let mut missing_policy = compilation();
    missing_policy.effective_policy = None;
    assert_eq!(
        build_obligation_graph(&missing_policy).unwrap_err().code,
        "BHCP7101"
    );

    let mut mismatched_ir = compilation();
    mismatched_ir.ir_bytes.push(0);
    assert_eq!(
        build_obligation_graph(&mismatched_ir).unwrap_err().code,
        "BHCP7101"
    );
}
