use bhcp::capability::{build_capability_graph, validate_capability_graph};
use bhcp::cbor::encode_deterministic;
use bhcp::graph::GraphDocument;
use bhcp::hash::{HashAlgorithm, artifact_hash_with, semantic_hash_with};
use bhcp::pipeline::{
    Compilation, compile_source, compile_source_with_policy,
    compile_source_with_waiver_decision_time, parse_policy_source,
};
use bhcp::policy::{EffectivePolicyDocument, compose_policies};
use bhcp::value::Value;

const PROGRAM: &str = r#"
§goal example/Read@0 {
    §input repository: example/Repository@0;
    §output value: Text;
    §allows bhcp-effect/fs.read@0(repository);
}
§goal example/Parent@0 {
    §input repository: example/Repository@0;
    §allows bhcp-effect/fs.read@0(repository);
    §forbids bhcp-effect/network@0;
}
"#;

const POLICY: &str = r#"
§policy example/policy.organization@0 {
    layer organization;
    rule a-read: capability narrow {
        effect: bhcp-effect/fs.read@0,
        scope: {
            goals: [example/Parent@0, example/Read@0],
            resources: [example/Repository@0]
        }
    } nonwaivable;
    rule aa-process: capability narrow {
        effect: bhcp-effect/process@0,
        scope: { goals: [example/Parent@0, example/Read@0] }
    } nonwaivable;
    rule b-network: prohibition deny {
        effect: bhcp-effect/network@0,
        scope: { goals: [example/Parent@0] }
    } nonwaivable;
    rule c-mode: type-mode strengthen infer-strict nonwaivable;
}
"#;

fn effective(source: &str) -> EffectivePolicyDocument {
    let parsed = parse_policy_source(source, "capability-policy.bhcp").unwrap();
    compose_policies(&parsed.documents, Default::default()).unwrap()
}

fn governed() -> Compilation {
    compile_source_with_policy(PROGRAM, "capabilities.bhcp", &effective(POLICY)).unwrap()
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

fn array<'a>(value: &'a Value, field: &str) -> &'a [Value] {
    match value.get(field) {
        Some(Value::Array(values)) => values,
        _ => panic!("missing array field {field}"),
    }
}

fn text<'a>(value: &'a Value, field: &str) -> &'a str {
    match value.get(field) {
        Some(Value::Text(value)) => value,
        _ => panic!("missing text field {field}"),
    }
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

fn policy_node<'a>(graph: &'a GraphDocument, category: &str) -> &'a bhcp::graph::GraphNode {
    graph
        .nodes()
        .iter()
        .find(|node| {
            node.value()
                .get("policy")
                .and_then(|policy| policy.get("category"))
                == Some(&Value::Text(category.to_owned()))
        })
        .unwrap()
}

#[test]
fn requests_resources_ceilings_policy_and_decisions_are_complete() {
    let compilation = governed();
    let graph = build_capability_graph(&compilation).unwrap();

    assert!(graph.semantic_id().is_some());
    assert!(graph.artifact_id().is_some());
    for kind in ["request", "resource", "grant", "denial", "decision"] {
        assert!(
            graph.nodes().iter().any(|node| node.kind == kind),
            "missing {kind} node"
        );
    }

    let requests = graph
        .nodes()
        .iter()
        .filter(|node| node.kind == "request")
        .count();
    let decisions = graph
        .nodes()
        .iter()
        .filter(|node| node.kind == "decision")
        .collect::<Vec<_>>();
    assert_eq!(requests, 2);
    assert_eq!(decisions.len(), requests);
    for decision in decisions {
        let capability = decision.value().get("capability").unwrap();
        assert_eq!(text(capability, "decision"), "allow");
        assert_eq!(array(capability, "sources").len(), 2);
    }
    assert!(graph.edges().iter().any(|edge| edge.kind == "requests"));
    assert!(graph.edges().iter().any(|edge| edge.kind == "authorizes"));
    assert!(graph.edges().iter().any(|edge| edge.kind == "constrains"));

    let policy_grant = policy_node(&graph, "capability");
    assert_eq!(
        array(policy_grant.value().get("policy").unwrap(), "sources").len(),
        1
    );
    assert_eq!(
        policy_grant.value().get("capability").unwrap().get("scope"),
        policy_grant
            .value()
            .get("policy")
            .unwrap()
            .get("value")
            .unwrap()
            .get("scope")
    );
    let policy_denial = policy_node(&graph, "prohibition");
    assert_eq!(policy_denial.kind, "denial");
    assert_eq!(
        policy_denial
            .value()
            .get("capability")
            .unwrap()
            .get("scope"),
        policy_denial
            .value()
            .get("policy")
            .unwrap()
            .get("value")
            .unwrap()
            .get("scope")
    );
    validate_capability_graph(&compilation, &graph).unwrap();
}

#[test]
fn unsafe_and_foreign_authority_stays_granted_but_requires_an_explicit_gap() {
    let compilation = compile_source(
        r#"
§goal example/Risky@0 {
    §allows bhcp-effect/foreign@0, bhcp-effect/unsafe@0;
}
"#,
        "risky.bhcp",
    )
    .unwrap();
    let graph = build_capability_graph(&compilation).unwrap();
    let decisions = graph
        .nodes()
        .iter()
        .filter(|node| node.kind == "decision")
        .collect::<Vec<_>>();
    assert_eq!(decisions.len(), 2);
    assert!(decisions.iter().all(|node| {
        text(node.value().get("capability").unwrap(), "decision") == "allow"
            && node.value().get("gap").and_then(|gap| gap.get("required"))
                == Some(&Value::Bool(true))
    }));
}

#[test]
fn child_requests_propagate_through_parent_intersections_without_planner_grants() {
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
    let graph = build_capability_graph(&compile_source(source, "nested.bhcp").unwrap()).unwrap();
    assert!(
        graph
            .edges()
            .iter()
            .any(|edge| edge.kind == "propagates-to")
    );
    assert_eq!(
        graph
            .nodes()
            .iter()
            .filter(|node| node.kind == "decision")
            .count(),
        2
    );
    assert!(
        graph
            .nodes()
            .iter()
            .filter(|node| node.kind == "decision")
            .all(|node| text(node.value().get("capability").unwrap(), "decision") == "allow")
    );
}

#[test]
fn unsupported_namespaced_effect_remains_visible_as_a_required_gap() {
    let compilation = compile_source(
        "§goal example/Extended@0 { §allows example-effect/custom@0; }",
        "unsupported-effect.bhcp",
    )
    .unwrap();
    let graph = build_capability_graph(&compilation).unwrap();
    let decision = graph
        .nodes()
        .iter()
        .find(|node| node.kind == "decision")
        .unwrap();
    assert_eq!(
        decision.value().get("gap").and_then(|gap| gap.get("kind")),
        Some(&Value::Text("unsupported".to_owned()))
    );
}

#[test]
fn denied_unresolved_and_out_of_scope_requests_emit_no_planning_graph() {
    let denied = compile_source(
        "§goal example/G@0 { §allows bhcp-effect/network@0; §forbids bhcp-effect/network@0; }",
        "denied.bhcp",
    )
    .unwrap_err();
    assert_eq!(denied.code, "BHCP4502");

    let unresolved = compile_source_with_policy(
        "§goal example/G@0 { §allows bhcp-effect/network@0; }",
        "unresolved.bhcp",
        &effective(
            r#"
§policy example/policy@0 {
    layer organization;
    rule a-read: capability narrow { effect: bhcp-effect/fs.read@0 } nonwaivable;
    rule b-mode: type-mode strengthen infer-strict nonwaivable;
}
"#,
        ),
    )
    .unwrap_err();
    assert_eq!(unresolved.code, "BHCP8203");
}

#[test]
fn applied_capability_waiver_retains_exact_artifact_targets_and_decision_time() {
    let source = format!(
        r#"
§policy example/policy.organization@0 {{
    layer organization;
    rule a-read: capability narrow {{
        effect: bhcp-effect/fs.read@0,
        scope: {{
            goals: [example/Read@0],
            resources: [example/Repository@0]
        }}
    }} waivable by ["security"];
    rule b-mode: type-mode strengthen infer-strict nonwaivable;
}}
§waiver example/waiver.read@0 {{
    issuer "security";
    targets [{{
        rule: [example/policy.organization@0, "a-read"],
        scope: {{
            goals: [example/Read@0],
            resources: [example/Repository@0]
        }},
        weakening: {{
            category: "capability",
            operation: "broaden",
            from: {{
                effect: bhcp-effect/fs.read@0,
                scope: {{
                    goals: [example/Read@0],
                    resources: [example/Repository@0]
                }}
            }},
            to: {{
                effect: bhcp-effect/fs.read@0,
                scope: {{
                    goals: [example/Parent@0, example/Read@0],
                    resources: [example/Repository@0]
                }}
            }}
        }}
    }}];
    justification "reviewed parent authority";
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
{PROGRAM}
"#
    );
    let compilation = compile_source_with_waiver_decision_time(
        &source,
        "waived-capability.bhcp",
        "2026-07-21T00:00:00Z",
    )
    .unwrap();
    let graph = build_capability_graph(&compilation).unwrap();
    let waiver = graph
        .nodes()
        .iter()
        .find(|node| node.kind == "waiver")
        .unwrap();
    let detail = waiver.value().get("waiver").unwrap();
    assert_eq!(array(detail, "targets").len(), 1);
    assert_eq!(
        detail.get("decision_time"),
        Some(&Value::Tag(
            0,
            Box::new(Value::Text("2026-07-21T00:00:00Z".to_owned()))
        ))
    );
    assert!(
        graph
            .edges()
            .iter()
            .any(|edge| edge.kind == "waiver-context")
    );
}

#[test]
fn insertion_order_and_equivalent_policy_decomposition_preserve_identity_boundaries() {
    let reordered = PROGRAM
        .replace(
            "§allows bhcp-effect/fs.read@0(repository);\n    §forbids bhcp-effect/network@0;",
            "§forbids bhcp-effect/network@0;\n    §allows bhcp-effect/fs.read@0(repository);",
        )
        .replace(
            "§goal example/Parent@0 {\n    §input repository:",
            "§goal example/Parent@0 {\n    §input noise: example/Noise@0;\n    §input repository:",
        );
    let first = build_capability_graph(&governed()).unwrap();
    let second = build_capability_graph(
        &compile_source_with_policy(&reordered, "reordered.bhcp", &effective(POLICY)).unwrap(),
    )
    .unwrap();
    assert_eq!(
        first
            .nodes()
            .iter()
            .map(|node| &node.id)
            .collect::<Vec<_>>(),
        second
            .nodes()
            .iter()
            .map(|node| &node.id)
            .collect::<Vec<_>>()
    );

    let duplicate = format!(
        "{POLICY}\n{}",
        r#"
§policy example/policy.team@0 {
    layer team;
    rule a-read-again: capability narrow {
        effect: bhcp-effect/fs.read@0,
        scope: {
            goals: [example/Parent@0, example/Read@0],
            resources: [example/Repository@0]
        }
    } nonwaivable;
}
"#
    );
    let decomposed = build_capability_graph(
        &compile_source_with_policy(PROGRAM, "decomposed.bhcp", &effective(&duplicate)).unwrap(),
    )
    .unwrap();
    assert_eq!(first.semantic_id(), decomposed.semantic_id());
    assert_ne!(first.artifact_id(), decomposed.artifact_id());
}

#[test]
fn fabricated_or_stale_decisions_fail_exact_validation() {
    let compilation = governed();
    let graph = build_capability_graph(&compilation).unwrap();
    let mut value = graph.to_value();
    let Value::Map(entries) = &mut value else {
        unreachable!()
    };
    entries.retain(|(field, _)| field != "semantic_id" && field != "artifact_id");
    let Value::Array(nodes) = &mut entries
        .iter_mut()
        .find(|(field, _)| field == "nodes")
        .unwrap()
        .1
    else {
        unreachable!()
    };
    let decision = nodes
        .iter_mut()
        .find(|node| node.get("kind") == Some(&Value::Text("decision".to_owned())))
        .unwrap();
    let Value::Map(capability) = field_mut(decision, "capability") else {
        unreachable!()
    };
    capability
        .iter_mut()
        .find(|(field, _)| field == "decision")
        .unwrap()
        .1 = Value::Text("deny".to_owned());
    let forged = GraphDocument::from_value(&value).unwrap();
    assert_eq!(
        validate_capability_graph(&compilation, &forged)
            .unwrap_err()
            .code,
        "BHCP7203"
    );
}

#[test]
fn retained_policy_decisions_and_compilation_envelopes_are_revalidated() {
    let mut missing_grant = governed();
    missing_grant.ir.goals[0]
        .policy_decision
        .as_mut()
        .unwrap()
        .capabilities
        .clear();
    let missing_grant = rematerialize(missing_grant);
    assert_eq!(
        build_capability_graph(&missing_grant).unwrap_err().code,
        "BHCP7201"
    );

    let mut stale_bytes = governed();
    stale_bytes.ir_bytes.push(0);
    assert_eq!(
        build_capability_graph(&stale_bytes).unwrap_err().code,
        "BHCP7201"
    );

    let mut stale_artifact_hash = governed();
    stale_artifact_hash.ir_hash.digest[0] ^= 0xff;
    assert_eq!(
        build_capability_graph(&stale_artifact_hash)
            .unwrap_err()
            .code,
        "BHCP7201"
    );

    let mut stale_typed_artifact_id = governed();
    stale_typed_artifact_id
        .ir
        .artifact_id
        .as_mut()
        .unwrap()
        .digest[0] ^= 0xff;
    stale_typed_artifact_id.ir_bytes =
        encode_deterministic(&stale_typed_artifact_id.ir.to_value(true)).unwrap();
    assert_eq!(
        build_capability_graph(&stale_typed_artifact_id)
            .unwrap_err()
            .code,
        "BHCP7201"
    );

    let mut missing_policy = governed();
    missing_policy.effective_policy = None;
    assert_eq!(
        build_capability_graph(&missing_policy).unwrap_err().code,
        "BHCP7201"
    );
}
