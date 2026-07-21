use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

use bhcp::consistency::{build_analysis_graphs, validate_analysis_graphs};
use bhcp::graph::GraphDocument;
use bhcp::hash::{HashAlgorithm, format_hash};
use bhcp::pipeline::{
    Compilation, compile_source, compile_source_bytes_with_profile_registry_and_waivers,
    compile_source_with_policy, parse_profile_source,
};
use bhcp::policy::{WaiverDocument, apply_waiver};
use bhcp::schema::parse_diagnostic;
use bhcp::value::Value;

const RETENTION: &str = r#"
§type example/Resource@0 = { name: Text };
§goal example/StateRead@0 {
    §input resource: example/Resource@0;
    §output state: Text;
    §allows bhcp-effect/state.read@0(resource);
}
§goal example/Candidate@0 {
    §input prior: { state: Text };
    §input resource: example/Resource@0;
    §output state: Text;
}
§goal example/CompareAndSwap@0 {
    §input expected_version: { state: Text };
    §input new_value: { state: Text };
    §input resource: example/Resource@0;
    §output committed: Text;
    §allows bhcp-effect/state.compare-and-swap@0(resource);
}
§goal example/Retain@0 {
    §input resource: example/Resource@0;
    §output committed: Text;
    §allows bhcp-effect/state.compare-and-swap@0(resource), bhcp-effect/state.read@0(resource);
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

fn repository() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn reference(name: &str) -> String {
    fs::read_to_string(
        repository()
            .join("conformance/v0/reference-program")
            .join(name),
    )
    .unwrap()
}

fn registry() -> BTreeMap<String, String> {
    reference("registry.txt")
        .lines()
        .map(|line| {
            let (key, value) = line.split_once('|').unwrap();
            (key.to_owned(), value.to_owned())
        })
        .collect()
}

fn governed_reference() -> (Compilation, Compilation) {
    let registry_values = registry();
    let profile_source = format!(
        "{}\n{}\n{}",
        reference("policy.bhcp"),
        reference("syntax.bhcp"),
        reference("profile.bhcp")
    );
    let profiles = parse_profile_source(&profile_source, "reference-profiles.bhcp").unwrap();
    let resolved = profiles
        .registry
        .resolve("bhcp.reference/review-profile@0", HashAlgorithm::default())
        .unwrap();
    let waiver =
        WaiverDocument::from_value(&parse_diagnostic(&reference("waiver.diag")).unwrap()).unwrap();
    let decision_time = &registry_values["waiver-decision-at"];
    let policy = apply_waiver(
        &resolved.effective_policy,
        &waiver,
        decision_time,
        HashAlgorithm::default(),
    )
    .unwrap();
    let extension = reference("extension.bhcp");
    let canonical_source = format!("{extension}\n{}", reference("program.bhcp"));
    let canonical = compile_source_with_policy(&canonical_source, "program.bhcp", &policy).unwrap();

    let profiled = reference("program.words.bhcp");
    let (preamble, body) = profiled.split_once('\n').unwrap();
    let profiled_source = format!("{preamble}\n{extension}\n{body}");
    let profiled = compile_source_bytes_with_profile_registry_and_waivers(
        profiled_source.as_bytes(),
        "program.words.bhcp",
        &profiles.registry,
        std::slice::from_ref(&waiver),
        decision_time,
    )
    .unwrap();
    (canonical, profiled)
}

#[test]
fn governed_reference_emits_one_exact_mutually_consistent_graph_set() {
    let (canonical, profiled) = governed_reference();
    let canonical_graphs = build_analysis_graphs(&canonical).unwrap();
    let profiled_graphs = build_analysis_graphs(&profiled).unwrap();
    let report = validate_analysis_graphs(&canonical, &canonical_graphs).unwrap();
    validate_analysis_graphs(&profiled, &profiled_graphs).unwrap();

    assert_eq!(
        report.features(),
        &BTreeSet::from([
            "bhcp/feature.capability-graph-builder@0".to_owned(),
            "bhcp/feature.obligation-graph-builder@0".to_owned(),
            "bhcp/feature.state-graph-builder@0".to_owned(),
        ])
    );
    assert_eq!(canonical.semantic_hash, profiled.semantic_hash);
    assert_ne!(canonical.ast_hash, profiled.ast_hash);
    assert_eq!(canonical_graphs, profiled_graphs);
    let expected_goals = reference("expected-obligations.txt")
        .lines()
        .filter_map(|line| line.strip_prefix("source|"))
        .filter_map(|line| line.split_once(':').map(|(goal, _)| goal.to_owned()))
        .collect::<BTreeSet<_>>();
    assert!(
        expected_goals.is_subset(
            &report
                .checker_obligations()
                .keys()
                .cloned()
                .collect::<BTreeSet<_>>()
        ),
        "reference obligation inventory is disconnected from checker goals"
    );
    for graph in canonical_graphs.iter() {
        assert!(graph.semantic_id().is_some());
        assert!(graph.artifact_id().is_some());
        assert_eq!(graph.to_cbor().unwrap(), graph.to_cbor().unwrap());
    }
    let identities = canonical_graphs
        .iter()
        .map(|graph| {
            let bytes = graph.to_cbor().unwrap();
            format!(
                "{}|semantic|{}|artifact|{}|cbor|{}",
                graph.kind().as_str(),
                format_hash(graph.semantic_id().unwrap()),
                format_hash(graph.artifact_id().unwrap()),
                format_hash(&HashAlgorithm::default().hash(&bytes)),
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    assert_eq!(reference("graph-identities.txt"), identities);
}

#[test]
fn every_parameterized_cas_decision_joins_one_state_authority_and_transition() {
    let source = RETENTION.replace(
        "bhcp-effect/state.compare-and-swap@0(resource)",
        "bhcp-effect/state.compare-and-swap@0(resource, \"primary\"), bhcp-effect/state.compare-and-swap@0(resource, \"audit\")",
    );
    let compilation = compile_source(&source, "cross-graph-multi-cas.bhcp").unwrap();
    let graphs = build_analysis_graphs(&compilation).unwrap();
    validate_analysis_graphs(&compilation, &graphs).unwrap();

    let cas_decisions = graphs
        .capability
        .nodes()
        .iter()
        .filter(|node| {
            node.kind == "decision"
                && node.value().get("goal") == Some(&Value::Text("example/Retain@0".to_owned()))
                && node
                    .value()
                    .get("capability")
                    .and_then(|value| value.get("effect"))
                    .and_then(|value| value.get("id"))
                    == Some(&Value::Text(
                        "bhcp-effect/state.compare-and-swap@0".to_owned(),
                    ))
        })
        .count();
    let authorities = graphs
        .state
        .nodes()
        .iter()
        .filter(|node| node.kind == "authority")
        .count();
    let state_value = graphs.state.to_value();
    let transition_authorities = values(&state_value, "transitions")[0]
        .get("authority")
        .and_then(|value| match value {
            Value::Array(values) => Some(values.len()),
            _ => None,
        })
        .unwrap();
    assert_eq!(cas_decisions, 2);
    assert_eq!(authorities, 2);
    assert_eq!(transition_authorities, 2);
}

#[test]
fn rematerialized_feature_decision_and_obligation_substitutions_fail_closed() {
    let (compilation, _) = governed_reference();
    let graphs = build_analysis_graphs(&compilation).unwrap();

    let mut invalid_compilation = compilation.clone();
    invalid_compilation.ir_bytes.push(0);
    assert_eq!(
        build_analysis_graphs(&invalid_compilation)
            .unwrap_err()
            .code,
        "BHCP7501"
    );

    let other = compile_source(
        "§goal example/Other@0 { §output result: Text; }",
        "other.bhcp",
    )
    .unwrap();
    let mut substituted = graphs.clone();
    substituted.obligation = build_analysis_graphs(&other).unwrap().obligation;
    assert_eq!(
        validate_analysis_graphs(&compilation, &substituted)
            .unwrap_err()
            .code,
        "BHCP7502"
    );

    let mut unsupported = graphs.clone();
    let mut value = unsupported.obligation.to_value();
    values_mut(&mut value, "features").push(Value::Text(
        "bhcp/feature.execution-graph-builder@0".to_owned(),
    ));
    unsupported.obligation = rematerialize(value);
    assert_eq!(
        validate_analysis_graphs(&compilation, &unsupported)
            .unwrap_err()
            .code,
        "BHCP7503"
    );

    let mut missing = graphs.clone();
    let mut value = missing.obligation.to_value();
    let used = values(&value, "edges")
        .iter()
        .flat_map(|edge| [text_field(edge, "from"), text_field(edge, "to")])
        .collect::<BTreeSet<_>>();
    let removable = values(&value, "nodes")
        .iter()
        .find(|node| !used.contains(text_field(node, "id")))
        .map(|node| text_field(node, "id").to_owned())
        .unwrap();
    values_mut(&mut value, "nodes").retain(|node| text_field(node, "id") != removable);
    missing.obligation = rematerialize(value);
    assert_eq!(
        validate_analysis_graphs(&compilation, &missing)
            .unwrap_err()
            .code,
        "BHCP7506"
    );

    let mut mismatched = graphs.clone();
    let mut value = mismatched.capability.to_value();
    let decision = values_mut(&mut value, "nodes")
        .iter_mut()
        .find(|node| node.get("kind") == Some(&Value::Text("decision".to_owned())))
        .unwrap();
    let effect = field_mut(field_mut(decision, "capability"), "effect");
    replace(effect, "id", Value::Text("bhcp-effect/forged@0".to_owned()));
    mismatched.capability = rematerialize(value);
    assert_eq!(
        validate_analysis_graphs(&compilation, &mismatched)
            .unwrap_err()
            .code,
        "BHCP7505"
    );
}

#[test]
fn state_joins_reject_dangling_cross_typed_and_same_typed_substitutions() {
    let compilation = compile_source(RETENTION, "cross-graph-retention.bhcp").unwrap();
    let graphs = build_analysis_graphs(&compilation).unwrap();
    validate_analysis_graphs(&compilation, &graphs).unwrap();
    let request = graphs
        .capability
        .nodes()
        .iter()
        .find(|node| node.kind == "request")
        .unwrap()
        .id
        .clone();
    let read_decision = graphs
        .capability
        .nodes()
        .iter()
        .find(|node| {
            node.kind == "decision"
                && node
                    .value()
                    .get("capability")
                    .and_then(|value| value.get("effect"))
                    .and_then(|value| value.get("id"))
                    == Some(&Value::Text("bhcp-effect/state.read@0".to_owned()))
        })
        .unwrap()
        .id
        .clone();
    let capability_resource = graphs
        .capability
        .nodes()
        .iter()
        .find(|node| node.kind == "resource")
        .unwrap()
        .id
        .clone();

    for (replacement, code) in [
        ("missing-decision".to_owned(), "BHCP7504"),
        (request, "BHCP7504"),
        (read_decision, "BHCP7507"),
    ] {
        let mut forged = graphs.clone();
        let mut value = forged.state.to_value();
        let authority = values_mut(&mut value, "nodes")
            .iter_mut()
            .find(|node| node.get("kind") == Some(&Value::Text("authority".to_owned())))
            .unwrap();
        replace(
            field_mut(authority, "payload"),
            "decision",
            Value::Text(replacement),
        );
        forged.state = rematerialize(value);
        assert_eq!(
            validate_analysis_graphs(&compilation, &forged)
                .unwrap_err()
                .code,
            code
        );
    }

    let mut cross_typed = graphs.clone();
    let mut value = cross_typed.state.to_value();
    let authority = values_mut(&mut value, "nodes")
        .iter_mut()
        .find(|node| node.get("kind") == Some(&Value::Text("authority".to_owned())))
        .unwrap();
    replace(
        field_mut(authority, "payload"),
        "resource",
        Value::Text(capability_resource),
    );
    cross_typed.state = rematerialize(value);
    assert_eq!(
        validate_analysis_graphs(&compilation, &cross_typed)
            .unwrap_err()
            .code,
        "BHCP7504"
    );

    let mut wrong_version = graphs.clone();
    let mut value = wrong_version.state.to_value();
    replace(
        &mut values_mut(&mut value, "transitions")[0],
        "to_version",
        Value::Integer(2),
    );
    wrong_version.state = rematerialize(value);
    assert_eq!(
        validate_analysis_graphs(&compilation, &wrong_version)
            .unwrap_err()
            .code,
        "BHCP7507"
    );
}

fn rematerialize(mut value: Value) -> GraphDocument {
    remove(&mut value, "semantic_id");
    remove(&mut value, "artifact_id");
    let mut graph = GraphDocument::from_value(&value).unwrap();
    graph
        .materialize_identities(HashAlgorithm::default())
        .unwrap();
    graph
}

fn values<'a>(value: &'a Value, field: &str) -> Vec<&'a Value> {
    let Some(Value::Array(values)) = value.get(field) else {
        panic!("{field} must be an array")
    };
    values.iter().collect()
}

fn values_mut<'a>(value: &'a mut Value, field: &str) -> &'a mut Vec<Value> {
    let Value::Array(values) = field_mut(value, field) else {
        panic!("{field} must be an array")
    };
    values
}

fn text_field<'a>(value: &'a Value, field: &str) -> &'a str {
    let Some(Value::Text(value)) = value.get(field) else {
        panic!("{field} must be text")
    };
    value
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
