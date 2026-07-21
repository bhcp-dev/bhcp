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
