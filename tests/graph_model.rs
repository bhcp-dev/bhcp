use std::fs;
use std::path::PathBuf;

use bhcp::cbor::encode_deterministic;
use bhcp::graph::{GraphDocument, GraphKind};
use bhcp::hash::HashAlgorithm;
use bhcp::inspection::render_artifact;
use bhcp::schema::parse_diagnostic;
use bhcp::value::Value;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn fixture(name: &str) -> Value {
    parse_diagnostic(&fs::read_to_string(root().join("schemas/v0/examples").join(name)).unwrap())
        .unwrap()
}

fn reference() -> Value {
    Value::map([
        ("media_type", Value::Text("application/cbor".to_owned())),
        ("size", Value::Integer(0)),
        (
            "digests",
            Value::Array(vec![Value::map([
                ("algorithm", Value::Text("bhcp.hash/sha3-512@0".to_owned())),
                ("digest", Value::Bytes(vec![0; 64])),
            ])]),
        ),
    ])
}

fn obligation_graph(nodes: Vec<Value>, edges: Vec<Value>) -> Value {
    Value::map([
        ("version", Value::Text("bhcp/v0".to_owned())),
        ("features", Value::Array(Vec::new())),
        ("kind", Value::Text("obligation-graph".to_owned())),
        ("semantic_ir", reference()),
        ("nodes", Value::Array(nodes)),
        ("edges", Value::Array(edges)),
    ])
}

fn obligation(id: &str, status: &str) -> Value {
    Value::map([
        ("id", Value::Text(id.to_owned())),
        ("kind", Value::Text("requirement".to_owned())),
        ("clause", Value::Text(format!("clause-{id}"))),
        ("status", Value::Text(status.to_owned())),
    ])
}

fn edge(id: &str, from: &str, to: &str) -> Value {
    Value::map([
        ("id", Value::Text(id.to_owned())),
        ("from", Value::Text(from.to_owned())),
        ("to", Value::Text(to.to_owned())),
        ("kind", Value::Text("depends-on".to_owned())),
    ])
}

fn execution_node(id: &str, dependencies: &[&str]) -> Value {
    Value::map([
        ("id", Value::Text(id.to_owned())),
        ("kind", Value::Text("goal".to_owned())),
        ("executor", Value::Text("example/executor@0".to_owned())),
        ("inputs", Value::Map(Vec::new())),
        ("outputs", Value::Map(Vec::new())),
        (
            "effects",
            Value::map([("effects", Value::Array(Vec::new()))]),
        ),
        ("capability_decisions", Value::Array(Vec::new())),
        ("budgets", Value::Array(Vec::new())),
        ("expected_evidence", Value::Array(Vec::new())),
        (
            "dependencies",
            Value::Array(
                dependencies
                    .iter()
                    .map(|dependency| Value::Text((*dependency).to_owned()))
                    .collect(),
            ),
        ),
    ])
}

fn execution_graph(nodes: Vec<Value>, entrypoints: Vec<&str>) -> Value {
    Value::map([
        ("version", Value::Text("bhcp/v0".to_owned())),
        ("features", Value::Array(Vec::new())),
        ("kind", Value::Text("execution-graph".to_owned())),
        ("semantic_ir", reference()),
        ("nodes", Value::Array(nodes)),
        ("edges", Value::Array(Vec::new())),
        (
            "entrypoints",
            Value::Array(
                entrypoints
                    .into_iter()
                    .map(|entrypoint| Value::Text(entrypoint.to_owned()))
                    .collect(),
            ),
        ),
    ])
}

fn capability_graph(nodes: Vec<Value>) -> Value {
    Value::map([
        ("version", Value::Text("bhcp/v0".to_owned())),
        ("features", Value::Array(Vec::new())),
        ("kind", Value::Text("capability-graph".to_owned())),
        ("semantic_ir", reference()),
        ("nodes", Value::Array(nodes)),
        ("edges", Value::Array(Vec::new())),
    ])
}

fn state_graph(nodes: Vec<Value>, transitions: Vec<Value>) -> Value {
    Value::map([
        ("version", Value::Text("bhcp/v0".to_owned())),
        ("features", Value::Array(Vec::new())),
        ("kind", Value::Text("state-graph".to_owned())),
        ("semantic_ir", reference()),
        ("nodes", Value::Array(nodes)),
        ("edges", Value::Array(Vec::new())),
        ("transitions", Value::Array(transitions)),
    ])
}

fn evidence_graph(gaps: Vec<Value>) -> Value {
    Value::map([
        ("version", Value::Text("bhcp/v0".to_owned())),
        ("features", Value::Array(Vec::new())),
        ("kind", Value::Text("evidence-bundle".to_owned())),
        ("semantic_ir", reference()),
        ("execution_graph", reference()),
        ("claims", Value::Array(Vec::new())),
        ("items", Value::Array(Vec::new())),
        ("gaps", Value::Array(gaps)),
        ("edges", Value::Array(Vec::new())),
        (
            "obligation_status",
            Value::map([("obligation", Value::Text("unresolved".to_owned()))]),
        ),
    ])
}

fn provenance(annotations: Option<Value>) -> Value {
    let mut value = Value::map([
        ("producer", Value::Text("example/compiler@0".to_owned())),
        (
            "created_at",
            Value::Tag(0, Box::new(Value::Text("2026-01-01T00:00:00Z".to_owned()))),
        ),
    ]);
    if let Some(annotations) = annotations {
        replace_field(&mut value, "annotations", annotations);
    }
    value
}

fn ordered_colliding_value() -> Value {
    Value::map([
        (
            "items",
            Value::Array(vec![
                Value::Text("aa".to_owned()),
                Value::Text("b".to_owned()),
            ]),
        ),
        (
            "sources",
            Value::Array(vec![
                Value::Text("aa".to_owned()),
                Value::Text("b".to_owned()),
            ]),
        ),
        (
            "effects",
            Value::Array(vec![
                Value::Text("aa".to_owned()),
                Value::Text("b".to_owned()),
            ]),
        ),
        (
            "claims",
            Value::Array(vec![
                Value::Text("aa".to_owned()),
                Value::Text("b".to_owned()),
            ]),
        ),
    ])
}

fn bool_type() -> Value {
    Value::Array(vec![
        Value::Text("primitive".to_owned()),
        Value::Text("Bool".to_owned()),
    ])
}

fn literal_expression(id: &str, literal: bool) -> Value {
    Value::map([
        ("id", Value::Text(id.to_owned())),
        ("type", bool_type()),
        (
            "form",
            Value::Array(vec![
                Value::Text("literal".to_owned()),
                Value::Bool(literal),
            ]),
        ),
    ])
}

fn replace_field(value: &mut Value, field: &str, replacement: Value) {
    let Value::Map(entries) = value else {
        panic!("expected map")
    };
    if let Some((_, value)) = entries.iter_mut().find(|(key, _)| key == field) {
        *value = replacement;
    } else {
        entries.push((field.to_owned(), replacement));
    }
}

fn field_mut<'a>(value: &'a mut Value, field: &str) -> &'a mut Value {
    let Value::Map(entries) = value else {
        panic!("expected map")
    };
    &mut entries.iter_mut().find(|(key, _)| key == field).unwrap().1
}

fn assert_cbor_item_sorted(items: &[Value]) {
    let encoded = items
        .iter()
        .map(|item| encode_deterministic(item).unwrap())
        .collect::<Vec<_>>();
    assert!(encoded.windows(2).all(|pair| pair[0] < pair[1]));
}

#[test]
fn every_graph_root_decodes_validates_and_reencodes_deterministically() {
    for (name, kind) in [
        ("obligation-graph.diag", GraphKind::Obligation),
        ("capability-graph.diag", GraphKind::Capability),
        ("state-graph.diag", GraphKind::State),
        ("execution-graph.diag", GraphKind::Execution),
        ("evidence-bundle.diag", GraphKind::Evidence),
    ] {
        let value = fixture(name);
        let bytes = encode_deterministic(&value).unwrap();
        let document = GraphDocument::from_cbor(&bytes).unwrap();
        assert_eq!(document.kind(), kind);
        assert_eq!(document.to_cbor().unwrap(), bytes);
    }
}

#[test]
fn ordering_and_presentation_do_not_change_semantic_identity() {
    let nodes = vec![obligation("b", "open"), obligation("a", "open")];
    let edges = vec![edge("z", "a", "b")];
    let mut first =
        GraphDocument::from_value(&obligation_graph(nodes.clone(), edges.clone())).unwrap();

    let mut reordered = obligation_graph(nodes.into_iter().rev().collect(), edges);
    let Value::Map(entries) = &mut reordered else {
        unreachable!()
    };
    entries.push((
        "provenance".to_owned(),
        Value::map([
            ("producer", Value::Text("example/compiler@0".to_owned())),
            (
                "created_at",
                Value::Tag(0, Box::new(Value::Text("2026-01-01T00:00:00Z".to_owned()))),
            ),
        ]),
    ));
    let mut second = GraphDocument::from_value(&reordered).unwrap();

    first
        .materialize_identities(HashAlgorithm::default())
        .unwrap();
    second
        .materialize_identities(HashAlgorithm::default())
        .unwrap();
    assert_eq!(first.semantic_id(), second.semantic_id());
    assert_ne!(first.artifact_id(), second.artifact_id());
    assert_eq!(first.nodes()[0].id, "a");
    assert_eq!(second.nodes()[0].id, "a");

    let mut changed = GraphDocument::from_value(&obligation_graph(
        vec![obligation("a", "refuted"), obligation("b", "open")],
        vec![edge("z", "a", "b")],
    ))
    .unwrap();
    changed
        .materialize_identities(HashAlgorithm::default())
        .unwrap();
    assert_ne!(first.semantic_id(), changed.semantic_id());
}

#[test]
fn every_semantic_set_uses_normalized_deterministic_cbor_item_order() {
    let mut long = obligation("aa", "open");
    replace_field(
        &mut long,
        "evidence",
        Value::Array(vec![
            Value::Text("aa".to_owned()),
            Value::Text("b".to_owned()),
        ]),
    );
    let mut short = obligation("b", "open");
    replace_field(
        &mut short,
        "evidence",
        Value::Array(vec![
            Value::Text("aa".to_owned()),
            Value::Text("b".to_owned()),
        ]),
    );
    let mut value = obligation_graph(vec![long, short], vec![edge("edge", "b", "aa")]);
    replace_field(
        &mut value,
        "features",
        Value::Array(vec![
            Value::Text("example/long@0".to_owned()),
            Value::Text("x/s@0".to_owned()),
        ]),
    );

    let document = GraphDocument::from_value(&value).unwrap();
    let normalized = document.to_value();
    let Value::Array(features) = normalized.get("features").unwrap() else {
        unreachable!()
    };
    assert_cbor_item_sorted(features);
    assert_eq!(features[0], Value::Text("x/s@0".to_owned()));
    let Value::Array(nodes) = normalized.get("nodes").unwrap() else {
        unreachable!()
    };
    assert_cbor_item_sorted(nodes);
    assert_eq!(document.nodes()[0].id, "b");
    for node in document.nodes() {
        let Value::Array(evidence) = node.value().get("evidence").unwrap() else {
            unreachable!()
        };
        assert_cbor_item_sorted(evidence);
        assert_eq!(
            evidence,
            &vec![Value::Text("b".to_owned()), Value::Text("aa".to_owned())]
        );
    }
}

#[test]
fn arbitrary_value_arrays_preserve_order_under_semantic_set_key_names() {
    let ordered = ordered_colliding_value();
    let capability = capability_graph(vec![Value::map([
        ("id", Value::Text("request".to_owned())),
        ("kind", Value::Text("request".to_owned())),
        (
            "capability",
            Value::map([
                (
                    "effect",
                    Value::map([("id", Value::Text("bhcp-effect/fs.read@0".to_owned()))]),
                ),
                ("scope", ordered.clone()),
                ("decision", Value::Text("allow".to_owned())),
                (
                    "sources",
                    Value::Array(vec![Value::Text("policy".to_owned())]),
                ),
            ]),
        ),
        ("payload", ordered.clone()),
    ])]);
    let capability = GraphDocument::from_value(&capability).unwrap().to_value();
    let Value::Array(capability_nodes) = capability.get("nodes").unwrap() else {
        unreachable!()
    };
    assert_eq!(capability_nodes[0].get("payload"), Some(&ordered));
    assert_eq!(
        capability_nodes[0].get("capability").unwrap().get("scope"),
        Some(&ordered)
    );

    let mut obligation = obligation_graph(vec![], vec![]);
    replace_field(
        &mut obligation,
        "provenance",
        provenance(Some(Value::map([("ordered", ordered.clone())]))),
    );
    let obligation = GraphDocument::from_value(&obligation).unwrap().to_value();
    assert_eq!(
        obligation
            .get("provenance")
            .unwrap()
            .get("annotations")
            .unwrap()
            .get("ordered"),
        Some(&ordered)
    );

    let evidence = evidence_graph(vec![Value::map([
        ("id", Value::Text("gap".to_owned())),
        ("kind", Value::Text("missing".to_owned())),
        (
            "obligations",
            Value::Array(vec![Value::Text("obligation".to_owned())]),
        ),
        (
            "reason",
            Value::map([
                ("code", Value::Text("example/reason@0".to_owned())),
                ("message", Value::Text("missing".to_owned())),
                ("details", ordered.clone()),
            ]),
        ),
        ("required", Value::Bool(true)),
    ])]);
    let evidence = GraphDocument::from_value(&evidence).unwrap().to_value();
    let Value::Array(gaps) = evidence.get("gaps").unwrap() else {
        unreachable!()
    };
    assert_eq!(
        gaps[0].get("reason").unwrap().get("details"),
        Some(&ordered)
    );

    let completed = Value::map([
        ("state", Value::Text("completed".to_owned())),
        (
            "verdict",
            Value::map([
                ("state", Value::Text("satisfied".to_owned())),
                ("output", ordered.clone()),
                ("evidence", Value::Array(vec![Value::Text("e".to_owned())])),
            ]),
        ),
    ]);
    let faulted = Value::map([
        ("state", Value::Text("faulted".to_owned())),
        (
            "fault",
            Value::map([
                (
                    "error",
                    Value::map([
                        ("code", Value::Text("example/fault@0".to_owned())),
                        ("message", Value::Text("fault".to_owned())),
                    ]),
                ),
                (
                    "trace",
                    Value::Array(vec![Value::map([
                        ("sequence", Value::Integer(0)),
                        ("node", Value::Text("cell".to_owned())),
                        (
                            "at",
                            Value::Tag(0, Box::new(Value::Text("2026-01-01T00:00:00Z".to_owned()))),
                        ),
                        ("kind", Value::Text("example/trace@0".to_owned())),
                        ("payload", ordered.clone()),
                    ])]),
                ),
            ]),
        ),
    ]);
    let state = state_graph(
        vec![Value::map([
            ("id", Value::Text("cell".to_owned())),
            ("kind", Value::Text("cell".to_owned())),
        ])],
        vec![
            Value::map([
                ("id", Value::Text("completed".to_owned())),
                ("cell", Value::Text("cell".to_owned())),
                ("from_version", Value::Integer(0)),
                ("to_version", Value::Integer(1)),
                ("result", completed),
                ("atomic", Value::Bool(true)),
            ]),
            Value::map([
                ("id", Value::Text("faulted".to_owned())),
                ("cell", Value::Text("cell".to_owned())),
                ("from_version", Value::Integer(1)),
                ("to_version", Value::Integer(2)),
                ("result", faulted),
                ("atomic", Value::Bool(true)),
            ]),
        ],
    );
    let state = GraphDocument::from_value(&state).unwrap().to_value();
    let Value::Array(transitions) = state.get("transitions").unwrap() else {
        unreachable!()
    };
    let completed = transitions
        .iter()
        .find(|transition| transition.get("id") == Some(&Value::Text("completed".to_owned())))
        .unwrap();
    assert_eq!(
        completed
            .get("result")
            .unwrap()
            .get("verdict")
            .unwrap()
            .get("output"),
        Some(&ordered)
    );
    let faulted = transitions
        .iter()
        .find(|transition| transition.get("id") == Some(&Value::Text("faulted".to_owned())))
        .unwrap();
    let Value::Array(trace) = faulted
        .get("result")
        .unwrap()
        .get("fault")
        .unwrap()
        .get("trace")
        .unwrap()
    else {
        unreachable!()
    };
    assert_eq!(trace[0].get("payload"), Some(&ordered));
}

#[test]
fn quantified_verifier_output_must_be_evidence_type() {
    let binding = Value::map([
        ("id", Value::Text("candidate".to_owned())),
        ("type", bool_type()),
    ]);
    let verifier = Value::map([
        ("verifier", Value::Text("example/verifier@0".to_owned())),
        ("input", bool_type()),
        ("output", bool_type()),
    ]);
    let expression = Value::map([
        ("id", Value::Text("quantified".to_owned())),
        ("type", bool_type()),
        (
            "form",
            Value::Array(vec![
                Value::Text("quantify".to_owned()),
                Value::Text("forall".to_owned()),
                binding,
                literal_expression("domain", true),
                literal_expression("predicate", true),
                verifier,
            ]),
        ),
    ]);
    let captured = Value::Array(vec![
        Value::Text("captured".to_owned()),
        Value::Bool(true),
        Value::Array(vec![Value::Text("evidence".to_owned())]),
        provenance(None),
        Value::Tag(0, Box::new(Value::Text("2026-01-01T00:00:00Z".to_owned()))),
        expression,
    ]);
    let state = state_graph(
        vec![Value::map([
            ("id", Value::Text("cell".to_owned())),
            ("kind", Value::Text("cell".to_owned())),
            (
                "cell",
                Value::map([
                    ("key", Value::Text("key".to_owned())),
                    ("type", bool_type()),
                    ("state", captured),
                    ("atomic_version", Value::Integer(0)),
                ]),
            ),
        ])],
        vec![],
    );
    assert_eq!(
        GraphDocument::from_value(&state).unwrap_err().code,
        "BHCP7001"
    );

    let mut valid = state;
    let Value::Array(nodes) = field_mut(&mut valid, "nodes") else {
        unreachable!()
    };
    let Value::Array(captured) = field_mut(field_mut(&mut nodes[0], "cell"), "state") else {
        unreachable!()
    };
    let Value::Array(form) = field_mut(&mut captured[5], "form") else {
        unreachable!()
    };
    replace_field(
        &mut form[5],
        "output",
        Value::Array(vec![
            Value::Text("evidence".to_owned()),
            Value::Array(vec![Value::Text("static".to_owned())]),
        ]),
    );
    assert!(GraphDocument::from_value(&valid).is_ok());
}

#[test]
fn malformed_graphs_fail_closed_with_stable_categories() {
    let duplicate = obligation_graph(
        vec![obligation("a", "open"), obligation("a", "open")],
        vec![],
    );
    assert_eq!(
        GraphDocument::from_value(&duplicate).unwrap_err().code,
        "BHCP7002"
    );

    let dangling = obligation_graph(
        vec![obligation("a", "open")],
        vec![edge("edge", "a", "missing")],
    );
    assert_eq!(
        GraphDocument::from_value(&dangling).unwrap_err().code,
        "BHCP7003"
    );

    let cyclic = obligation_graph(
        vec![obligation("a", "open"), obligation("b", "open")],
        vec![edge("ab", "a", "b"), edge("ba", "b", "a")],
    );
    assert_eq!(
        GraphDocument::from_value(&cyclic).unwrap_err().code,
        "BHCP7004"
    );

    let mut unknown = obligation_graph(vec![], vec![]);
    let Value::Map(entries) = &mut unknown else {
        unreachable!()
    };
    entries.push(("surprise".to_owned(), Value::Bool(true)));
    assert_eq!(
        GraphDocument::from_value(&unknown).unwrap_err().code,
        "BHCP7001"
    );
}

#[test]
fn every_graph_root_rejects_malformed_nested_typed_members() {
    let mut obligation = obligation_graph(vec![], vec![]);
    let provenance = Value::map([
        ("producer", Value::Text("example/compiler@0".to_owned())),
        (
            "created_at",
            Value::Tag(0, Box::new(Value::Text("2026-01-01T00:00:00Z".to_owned()))),
        ),
        ("annotations", Value::map([("invalid", Value::Integer(1))])),
    ]);
    let Value::Map(entries) = &mut obligation else {
        unreachable!()
    };
    entries.push(("provenance".to_owned(), provenance));

    let capability = capability_graph(vec![Value::map([
        ("id", Value::Text("request".to_owned())),
        ("kind", Value::Text("request".to_owned())),
        (
            "capability",
            Value::map([
                ("effect", Value::Bool(true)),
                ("scope", Value::Text("workspace".to_owned())),
                ("decision", Value::Text("allow".to_owned())),
                (
                    "sources",
                    Value::Array(vec![Value::Text("policy".to_owned())]),
                ),
            ]),
        ),
    ])]);

    let state = state_graph(
        vec![Value::map([
            ("id", Value::Text("cell".to_owned())),
            ("kind", Value::Text("cell".to_owned())),
            (
                "cell",
                Value::map([
                    ("key", Value::Text("key".to_owned())),
                    ("type", Value::Bool(true)),
                    ("state", Value::Array(vec![Value::Text("empty".to_owned())])),
                    ("atomic_version", Value::Integer(0)),
                ]),
            ),
        ])],
        vec![],
    );
    let state_handle = state_graph(
        vec![Value::map([
            ("id", Value::Text("resource".to_owned())),
            ("kind", Value::Text("resource".to_owned())),
            ("handle", bool_type()),
        ])],
        vec![],
    );

    let mut invalid_output = execution_node("node", &[]);
    replace_field(
        &mut invalid_output,
        "outputs",
        Value::map([("value", Value::Bool(true))]),
    );
    let mut invalid_effects = execution_node("node", &[]);
    replace_field(&mut invalid_effects, "effects", Value::Bool(true));
    let mut invalid_budgets = execution_node("node", &[]);
    replace_field(
        &mut invalid_budgets,
        "budgets",
        Value::Array(vec![Value::Bool(true)]),
    );

    let evidence = evidence_graph(vec![Value::map([
        ("id", Value::Text("gap".to_owned())),
        ("kind", Value::Text("missing".to_owned())),
        (
            "obligations",
            Value::Array(vec![Value::Text("obligation".to_owned())]),
        ),
        (
            "reason",
            Value::map([
                ("code", Value::Text("example/reason@0".to_owned())),
                ("message", Value::Text("missing".to_owned())),
                (
                    "details",
                    Value::Tag(99, Box::new(Value::Text("invalid".to_owned()))),
                ),
            ]),
        ),
        ("required", Value::Bool(true)),
    ])]);

    for malformed in [
        obligation,
        capability,
        state,
        state_handle,
        execution_graph(vec![invalid_output], vec!["node"]),
        execution_graph(vec![invalid_effects], vec!["node"]),
        execution_graph(vec![invalid_budgets], vec!["node"]),
        evidence,
    ] {
        assert_eq!(
            GraphDocument::from_value(&malformed).unwrap_err().code,
            "BHCP7001"
        );
    }
}

#[test]
fn execution_dependency_cycles_are_forbidden_even_without_edges() {
    let cyclic = execution_graph(
        vec![execution_node("a", &["b"]), execution_node("b", &["a"])],
        vec!["a"],
    );
    assert_eq!(
        GraphDocument::from_value(&cyclic).unwrap_err().code,
        "BHCP7004"
    );

    let acyclic = execution_graph(
        vec![execution_node("a", &[]), execution_node("b", &["a"])],
        vec!["b"],
    );
    assert!(GraphDocument::from_value(&acyclic).is_ok());
}

#[test]
fn materialized_identities_are_recomputed_on_decode() {
    let mut document = GraphDocument::from_value(&obligation_graph(
        vec![obligation("a", "open"), obligation("b", "open")],
        vec![edge("ab", "a", "b")],
    ))
    .unwrap();
    document
        .materialize_identities(HashAlgorithm::default())
        .unwrap();
    let bytes = document.to_cbor().unwrap();
    assert_eq!(GraphDocument::from_cbor(&bytes).unwrap(), document);

    let mut tampered = document.to_value();
    let Value::Array(nodes) = tampered.get("nodes").unwrap().clone() else {
        unreachable!()
    };
    let mut changed_nodes = nodes;
    let Value::Map(entries) = &mut changed_nodes[0] else {
        unreachable!()
    };
    let (_, Value::Text(status)) = entries.iter_mut().find(|(key, _)| key == "status").unwrap()
    else {
        unreachable!()
    };
    *status = "refuted".to_owned();
    let Value::Map(root_entries) = &mut tampered else {
        unreachable!()
    };
    *root_entries
        .iter_mut()
        .find(|(key, _)| key == "nodes")
        .unwrap() = ("nodes".to_owned(), Value::Array(changed_nodes));
    let error = GraphDocument::from_value(&tampered).unwrap_err();
    assert_eq!(error.code, "BHCP7005");
}

#[test]
fn inspection_exposes_graph_structure_references_provenance_and_errors() {
    let mut value = obligation_graph(
        vec![obligation("a", "open"), obligation("b", "open")],
        vec![edge("ab", "a", "b")],
    );
    let Value::Map(entries) = &mut value else {
        unreachable!()
    };
    entries.push((
        "provenance".to_owned(),
        Value::map([
            ("producer", Value::Text("example/compiler@0".to_owned())),
            (
                "created_at",
                Value::Tag(0, Box::new(Value::Text("2026-01-01T00:00:00Z".to_owned()))),
            ),
        ]),
    ));
    let rendered = render_artifact(&value, None);
    assert!(rendered.contains("node a requirement"));
    assert!(rendered.contains("edge ab depends-on a -> b"));
    assert!(rendered.contains("reference semantic_ir application/cbor"));
    assert!(rendered.contains("provenance example/compiler@0 2026-01-01T00:00:00Z"));

    let invalid = obligation_graph(
        vec![obligation("a", "open")],
        vec![edge("edge", "a", "missing")],
    );
    assert!(render_artifact(&invalid, None).contains("validation-error BHCP7003"));
}
