use bhcp::graph::GraphKind;
use bhcp::pipeline::compile_source;
use bhcp::state::{build_state_graph, validate_state_graph};
use bhcp::value::Value;

const OWNERSHIP: &str = r#"
§goal conformance/Read@0 {
    §input file: borrowed read affine 'scope conformance/File@0;
}
§goal conformance/ReadBoundary@0 {
    §input enabled: Bool;
    §input file: owned write affine 'scope conformance/File@0;
    §gate when enabled {
        child = conformance/Read@0(file = borrow file);
    };
}
"#;

const RETENTION: &str = r#"
§goal example/StateRead@0 {
    §input resource: Text;
    §output state: Text;
    §allows bhcp-effect/state.read@0;
}
§goal example/Candidate@0 {
    §input prior: { state: Text };
    §input resource: Text;
    §output state: Text;
}
§goal example/CompareAndSwap@0 {
    §input expected_version: { state: Text };
    §input new_value: { state: Text };
    §input resource: Text;
    §output committed: Text;
    §allows bhcp-effect/state.compare-and-swap@0;
}
§goal example/Retain@0 {
    §input resource: Text;
    §output committed: Text;
    §allows bhcp-effect/state.compare-and-swap@0, bhcp-effect/state.read@0;
    §compose using bhcp/prelude.retain-reducer@0 {
        state-read = example/StateRead@0(resource = resource);
        candidate = example/Candidate@0(prior = state-read, resource = resource);
        compare-and-swap = example/CompareAndSwap@0(
            expected_version = state-read,
            new_value = candidate,
            resource = resource
        );
    };
}
"#;

fn values<'a>(graph: &'a Value, field: &str) -> &'a [Value] {
    let Some(Value::Array(values)) = graph.get(field) else {
        panic!("{field} must be an array")
    };
    values
}

#[test]
fn ownership_resources_borrows_and_invariants_are_explicit_and_deterministic() {
    let compilation = compile_source(OWNERSHIP, "own-01-read-overlap.bhcp").unwrap();
    let first = build_state_graph(&compilation).unwrap();
    let second = build_state_graph(&compilation).unwrap();
    assert_eq!(first.kind(), GraphKind::State);
    assert_eq!(first.to_value(), second.to_value());
    assert_eq!(first.to_cbor().unwrap(), second.to_cbor().unwrap());
    assert!(first.semantic_id().is_some());
    assert!(first.artifact_id().is_some());

    let value = first.to_value();
    let nodes = values(&value, "nodes");
    for kind in ["resource", "ownership", "borrow", "invariant"] {
        assert!(
            nodes
                .iter()
                .any(|node| node.get("kind") == Some(&Value::Text(kind.to_owned()))),
            "missing {kind} node"
        );
    }
    let edges = values(&value, "edges");
    for kind in ["owns", "borrows", "guards"] {
        assert!(
            edges
                .iter()
                .any(|edge| edge.get("kind") == Some(&Value::Text(kind.to_owned()))),
            "missing {kind} edge"
        );
    }
    assert!(values(&value, "transitions").is_empty());
}

#[test]
fn retention_graph_binds_the_exact_read_candidate_version_cas_and_freshness_chain() {
    let compilation = compile_source(RETENTION, "retention-state-graph.bhcp").unwrap();
    let graph = build_state_graph(&compilation).unwrap();
    let value = graph.to_value();
    let nodes = values(&value, "nodes");
    for kind in ["cell", "transition", "invariant", "authority", "freshness"] {
        assert!(
            nodes
                .iter()
                .any(|node| node.get("kind") == Some(&Value::Text(kind.to_owned()))),
            "missing {kind} node"
        );
    }
    let edges = values(&value, "edges");
    for kind in [
        "reads",
        "prior-state",
        "candidate",
        "candidate-evidence",
        "expected-version",
        "requires-authority",
        "freshness-guard",
    ] {
        assert!(
            edges
                .iter()
                .any(|edge| edge.get("kind") == Some(&Value::Text(kind.to_owned()))),
            "missing {kind} edge"
        );
    }

    let transitions = values(&value, "transitions");
    assert_eq!(transitions.len(), 1);
    let transition = &transitions[0];
    assert_eq!(transition.get("from_version"), Some(&Value::Integer(0)));
    assert_eq!(transition.get("to_version"), Some(&Value::Integer(1)));
    assert_eq!(transition.get("atomic"), Some(&Value::Bool(true)));
    for field in [
        "read",
        "candidate",
        "compare_and_swap",
        "authority",
        "invariants",
        "freshness",
        "conflict",
    ] {
        assert!(
            transition.get(field).is_some(),
            "missing transition {field}"
        );
    }
    assert!(transition.get("result").is_none());
    let authority = nodes
        .iter()
        .find(|node| node.get("kind") == Some(&Value::Text("authority".into())))
        .unwrap();
    assert_ne!(
        authority.get("payload").unwrap().get("decision"),
        Some(&Value::Text("required".into()))
    );
    let freshness = nodes
        .iter()
        .find(|node| node.get("kind") == Some(&Value::Text("freshness".into())))
        .unwrap()
        .get("payload")
        .unwrap();
    for field in [
        "subject",
        "content",
        "provenance",
        "capture_time",
        "rule",
        "stale",
        "fault",
    ] {
        assert!(freshness.get(field).is_some(), "missing freshness {field}");
    }
}

#[test]
fn received_state_graph_must_exactly_reconstruct_identity_endpoints_and_kinds() {
    let compilation = compile_source(RETENTION, "retention-state-graph.bhcp").unwrap();
    let graph = build_state_graph(&compilation).unwrap();
    validate_state_graph(&compilation, &graph).unwrap();

    for mutation in [
        "endpoint",
        "kind",
        "version",
        "freshness",
        "authority-substitution",
        "invariant-substitution",
        "invariant-deletion",
    ] {
        let mut value = graph.to_value();
        let authority = node_id(&value, "authority");
        let freshness = node_id(&value, "freshness");
        let edges = values_mut(&mut value, "edges");
        match mutation {
            "endpoint" => replace(
                edges.first_mut().unwrap(),
                "to",
                Value::Text("missing".into()),
            ),
            "kind" => replace(
                edges.first_mut().unwrap(),
                "kind",
                Value::Text("forged".into()),
            ),
            "version" => {
                let transitions = values_mut(&mut value, "transitions");
                replace(&mut transitions[0], "to_version", Value::Integer(2));
            }
            "freshness" => {
                let node = values_mut(&mut value, "nodes")
                    .iter_mut()
                    .find(|node| node.get("kind") == Some(&Value::Text("freshness".into())))
                    .unwrap();
                let payload = field_mut(node, "payload");
                replace(payload, "rule", Value::Text("forged/rule@0".into()));
            }
            "authority-substitution" => {
                let transition = &mut values_mut(&mut value, "transitions")[0];
                replace(
                    transition,
                    "authority",
                    Value::Array(vec![Value::Text(freshness)]),
                );
            }
            "invariant-substitution" => {
                let transition = &mut values_mut(&mut value, "transitions")[0];
                replace(
                    transition,
                    "invariants",
                    Value::Array(vec![Value::Text(authority)]),
                );
            }
            "invariant-deletion" => {
                let transition = &mut values_mut(&mut value, "transitions")[0];
                let Value::Array(invariants) = field_mut(transition, "invariants") else {
                    unreachable!()
                };
                invariants.pop();
            }
            _ => unreachable!(),
        }
        let received = bhcp::graph::GraphDocument::from_value(&value);
        if mutation == "endpoint" {
            assert_eq!(received.unwrap_err().code, "BHCP7003");
        } else {
            assert_eq!(received.unwrap_err().code, "BHCP7005");
            remove(&mut value, "semantic_id");
            remove(&mut value, "artifact_id");
            let mut rematerialized = bhcp::graph::GraphDocument::from_value(&value).unwrap();
            rematerialized
                .materialize_identities(bhcp::hash::HashAlgorithm::default())
                .unwrap();
            let diagnostic = validate_state_graph(&compilation, &rematerialized).unwrap_err();
            assert_eq!(diagnostic.code, "BHCP7403", "{mutation}: {diagnostic}");
        }
    }
}

#[test]
fn state_graph_contains_analysis_requirements_but_no_runtime_authority() {
    let compilation = compile_source(RETENTION, "retention-state-graph.bhcp").unwrap();
    let value = build_state_graph(&compilation).unwrap().to_value();
    let encoded = format!("{value:?}");
    for forbidden in ["executor", "planner", "retry", "mutation-authority"] {
        assert!(
            !encoded.contains(forbidden),
            "state graph leaked {forbidden}"
        );
    }
}

#[test]
fn equivalent_authority_order_does_not_change_state_graph_identity_or_bytes() {
    let reordered = RETENTION.replace(
        "bhcp-effect/state.compare-and-swap@0, bhcp-effect/state.read@0",
        "bhcp-effect/state.read@0, bhcp-effect/state.compare-and-swap@0",
    );
    let first =
        build_state_graph(&compile_source(RETENTION, "retention-order-a.bhcp").unwrap()).unwrap();
    let second =
        build_state_graph(&compile_source(&reordered, "retention-order-b.bhcp").unwrap()).unwrap();
    assert_eq!(first.semantic_id(), second.semantic_id());
    assert_eq!(first.artifact_id(), second.artifact_id());
    assert_eq!(first.to_cbor().unwrap(), second.to_cbor().unwrap());
}

#[test]
fn invalid_ownership_and_unguarded_mutation_never_reach_a_state_graph() {
    for (fixture, code) in [
        ("own-02-write-conflict.bhcp", "BHCP4402"),
        ("own-03-use-after-move.bhcp", "BHCP4403"),
        ("own-04-expired-retention.bhcp", "BHCP4404"),
    ] {
        let source = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("conformance/v0/fixtures")
                .join(fixture),
        )
        .unwrap();
        assert_eq!(compile_source(&source, fixture).unwrap_err().code, code);
    }

    let unapproved_share = r#"
§goal example/RetainShared@0 {
    §input file: shared read 'retain example/File@0;
    §state retained: shared read 'retain example/File@0 = file;
}
"#;
    assert_eq!(
        compile_source(unapproved_share, "unapproved-share.bhcp")
            .unwrap_err()
            .code,
        "BHCP4404"
    );

    let unguarded = RETENTION
        .lines()
        .filter(|line| !line.contains("§allows"))
        .collect::<Vec<_>>()
        .join("\n");
    let compilation = compile_source(&unguarded, "unguarded-retention.bhcp").unwrap();
    let diagnostic = build_state_graph(&compilation).unwrap_err();
    assert_eq!(diagnostic.code, "BHCP7402");
    assert_eq!(
        diagnostic.message,
        "mutable retention transition requires the exact compare-and-swap capability decision"
    );
}

fn values_mut<'a>(graph: &'a mut Value, field: &str) -> &'a mut Vec<Value> {
    let Value::Array(values) = field_mut(graph, field) else {
        panic!("{field} must be an array")
    };
    values
}

fn node_id(graph: &Value, kind: &str) -> String {
    values(graph, "nodes")
        .iter()
        .find(|node| node.get("kind") == Some(&Value::Text(kind.to_owned())))
        .and_then(|node| node.get("id"))
        .and_then(|id| match id {
            Value::Text(id) => Some(id.clone()),
            _ => None,
        })
        .unwrap()
}

fn field_mut<'a>(value: &'a mut Value, field: &str) -> &'a mut Value {
    let Value::Map(entries) = value else {
        panic!("value must be a map")
    };
    &mut entries
        .iter_mut()
        .find(|(name, _)| name == field)
        .unwrap()
        .1
}

fn replace(value: &mut Value, field: &str, replacement: Value) {
    let Value::Map(entries) = value else {
        panic!("value must be a map")
    };
    *entries.iter_mut().find(|(name, _)| name == field).unwrap() = (field.to_owned(), replacement);
}

fn remove(value: &mut Value, field: &str) {
    let Value::Map(entries) = value else {
        panic!("value must be a map")
    };
    entries.retain(|(name, _)| name != field);
}
