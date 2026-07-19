use std::fs;
use std::path::PathBuf;

use bhcp::cbor::encode_deterministic;
use bhcp::hash::{HashAlgorithm, artifact_hash_with, hash_value};
use bhcp::policy::{PolicyDocument, PolicyLayer, PolicyRule};
use bhcp::schema::parse_diagnostic;
use bhcp::value::Value;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn text(value: &str) -> Value {
    Value::Text(value.to_owned())
}

fn array(values: impl IntoIterator<Item = Value>) -> Value {
    Value::Array(values.into_iter().collect())
}

fn common_rule(id: &str, category: &str, operation: &str, value: Value) -> Value {
    Value::map([
        ("id", text(id)),
        ("category", text(category)),
        ("operation", text(operation)),
        ("value", value),
        ("waivable", Value::Bool(false)),
    ])
}

fn all_rules() -> Vec<Value> {
    vec![
        common_rule(
            "a-requirement",
            "requirement",
            "add",
            Value::map([("requirement", text("example/requirement.lint@0"))]),
        ),
        common_rule(
            "b-evidence",
            "evidence",
            "add",
            Value::map([
                ("obligation", text("example/obligation.review@0")),
                ("classes", array([text("static")])),
                ("minimum", Value::Integer(1)),
            ]),
        ),
        common_rule(
            "c-prohibition",
            "prohibition",
            "deny",
            Value::map([("effect", text("bhcp-effect/network@0"))]),
        ),
        Value::map([
            ("id", text("d-capability")),
            ("category", text("capability")),
            ("operation", text("narrow")),
            (
                "value",
                Value::map([
                    ("effect", text("bhcp-effect/fs.read@0")),
                    (
                        "scope",
                        Value::map([("goals", array([text("example/goal.check@0")]))]),
                    ),
                ]),
            ),
            ("waivable", Value::Bool(true)),
            ("authorized_issuers", array([text("security-team")])),
        ]),
        common_rule(
            "e-limit",
            "limit",
            "tighten",
            Value::map([
                ("dimension", text("example/limit.attempts@0")),
                ("unit", text("example/unit.count@0")),
                ("maximum", array([text("integer"), Value::Integer(3)])),
            ]),
        ),
        common_rule("f-type-mode", "type-mode", "strengthen", text("strict")),
    ]
}

fn source_policy(layer: &str) -> Value {
    Value::map([
        ("version", text("bhcp/v0")),
        ("features", array([])),
        ("kind", text("policy")),
        ("form", text("source")),
        ("symbol", text(&format!("example/policy.{layer}@0"))),
        ("layer", text(layer)),
        ("rules", array(all_rules())),
    ])
}

fn effective_value() -> Value {
    Value::map([
        (
            "requirements",
            array([Value::map([
                ("waivable", Value::Bool(false)),
                (
                    "value",
                    Value::map([("requirement", text("example/requirement.lint@0"))]),
                ),
            ])]),
        ),
        ("evidence", array([])),
        ("prohibitions", array([])),
        ("capabilities", array([])),
        ("limits", array([])),
        (
            "type_mode",
            Value::map([("waivable", Value::Bool(false)), ("value", text("dynamic"))]),
        ),
    ])
}

fn effective_policy() -> Value {
    let effective = effective_value();
    let semantic_id = hash_value(&effective, HashAlgorithm::default()).unwrap();
    let without_artifact = Value::map([
        ("version", text("bhcp/v0")),
        ("features", array([])),
        (
            "provenance",
            Value::map([
                ("producer", text("example/policy.builder@0")),
                (
                    "created_at",
                    Value::Tag(0, Box::new(text("2026-07-19T00:00:00Z"))),
                ),
            ]),
        ),
        ("kind", text("policy")),
        ("form", text("effective")),
        ("semantic_id", semantic_id.to_value()),
        ("effective", effective),
        ("source_layers", array([])),
        ("rule_provenance", array([])),
    ]);
    let artifact_id = artifact_hash_with(&without_artifact, HashAlgorithm::default()).unwrap();
    let Value::Map(mut entries) = without_artifact else {
        unreachable!()
    };
    entries.push(("artifact_id".to_owned(), artifact_id.to_value()));
    Value::owned_map(entries)
}

fn artifact_reference() -> Value {
    Value::map([
        ("media_type", text("application/vnd.bhcp.waiver+cbor")),
        ("size", Value::Integer(0)),
        (
            "digests",
            array([Value::map([
                ("algorithm", text("bhcp.hash/sha3-512@0")),
                ("digest", Value::Bytes(vec![0; 64])),
            ])]),
        ),
    ])
}

fn effective_policy_with_waivers(waivers: Value) -> Value {
    let Value::Map(mut entries) = effective_policy() else {
        unreachable!()
    };
    entries.retain(|(key, _)| key != "artifact_id");
    entries.push(("waivers".to_owned(), waivers));
    let without_artifact = Value::owned_map(entries);
    let artifact_id = artifact_hash_with(&without_artifact, HashAlgorithm::default()).unwrap();
    let Value::Map(mut entries) = without_artifact else {
        unreachable!()
    };
    entries.push(("artifact_id".to_owned(), artifact_id.to_value()));
    Value::owned_map(entries)
}

fn applied_waiver(targets: Value, decision_time: Value) -> Value {
    Value::map([
        ("waiver", artifact_reference()),
        ("targets", targets),
        ("decision_time", decision_time),
    ])
}

#[test]
fn every_layer_and_typed_rule_round_trips_deterministically() {
    for (layer, expected_layer) in [
        ("organization", PolicyLayer::Organization),
        ("team", PolicyLayer::Team),
        ("repository", PolicyLayer::Repository),
        ("user", PolicyLayer::User),
    ] {
        let value = source_policy(layer);
        let document = PolicyDocument::from_value(&value).unwrap();
        let PolicyDocument::Source(source) = &document else {
            panic!("source policy parsed as effective")
        };
        assert_eq!(source.layer, expected_layer);
        assert!(matches!(source.rules[0], PolicyRule::Requirement { .. }));
        assert!(matches!(source.rules[1], PolicyRule::Evidence { .. }));
        assert!(matches!(source.rules[2], PolicyRule::Prohibition { .. }));
        assert!(matches!(source.rules[3], PolicyRule::Capability { .. }));
        assert!(matches!(source.rules[4], PolicyRule::Limit { .. }));
        assert!(matches!(source.rules[5], PolicyRule::TypeMode { .. }));
        assert_eq!(document.to_value(true), value);

        let bytes = document.to_cbor(true).unwrap();
        assert_eq!(bytes, encode_deterministic(&value).unwrap());
        assert_eq!(PolicyDocument::from_cbor(&bytes).unwrap(), document);
    }
}

#[test]
fn effective_policy_validates_semantic_and_artifact_identity() {
    let value = effective_policy();
    let document = PolicyDocument::from_value(&value).unwrap();
    assert_eq!(document.to_value(true), value);
    let bytes = document.to_cbor(true).unwrap();
    assert_eq!(PolicyDocument::from_cbor(&bytes).unwrap(), document);

    let Value::Map(mut entries) = value else {
        unreachable!()
    };
    let semantic = entries
        .iter_mut()
        .find(|(key, _)| key == "semantic_id")
        .unwrap();
    *semantic = (
        semantic.0.clone(),
        Value::map([
            ("algorithm", text("bhcp.hash/sha3-512@0")),
            ("digest", Value::Bytes(vec![0; 64])),
        ]),
    );
    let error = PolicyDocument::from_value(&Value::owned_map(entries)).unwrap_err();
    assert_eq!(error.code, "BHCP8001");
    assert_eq!(
        error.message,
        "effective policy semantic_id does not match effective meaning"
    );

    let value = effective_policy();
    let Value::Map(mut entries) = value else {
        unreachable!()
    };
    let artifact = entries
        .iter_mut()
        .find(|(key, _)| key == "artifact_id")
        .unwrap();
    *artifact = (
        artifact.0.clone(),
        Value::map([
            ("algorithm", text("bhcp.hash/sha3-512@0")),
            ("digest", Value::Bytes(vec![0; 64])),
        ]),
    );
    let error = PolicyDocument::from_value(&Value::owned_map(entries)).unwrap_err();
    assert_eq!(error.code, "BHCP8001");
    assert_eq!(
        error.message,
        "effective policy artifact_id does not match document"
    );
}

#[test]
fn effective_policy_round_trips_typed_applied_waiver_audit_entries() {
    let applied = applied_waiver(
        array([
            array([text("example/policy.org@0"), text("a-rule")]),
            array([text("example/policy.org@0"), text("b-rule")]),
        ]),
        Value::Tag(0, Box::new(text("2026-07-19T13:00:00Z"))),
    );
    let value = effective_policy_with_waivers(array([applied]));
    let document = PolicyDocument::from_value(&value).unwrap();
    let PolicyDocument::Effective(effective) = &document else {
        panic!("effective policy parsed as source")
    };
    let waiver = &effective.waivers.as_ref().unwrap()[0];
    assert_eq!(waiver.targets.len(), 2);
    assert_eq!(waiver.decision_time, "2026-07-19T13:00:00Z");
    assert_eq!(document.to_value(true), value);
    assert_eq!(
        PolicyDocument::from_cbor(&document.to_cbor(true).unwrap()).unwrap(),
        document
    );

    let plain = PolicyDocument::from_value(&effective_policy()).unwrap();
    let later =
        PolicyDocument::from_value(&effective_policy_with_waivers(array([applied_waiver(
            array([
                array([text("example/policy.org@0"), text("a-rule")]),
                array([text("example/policy.org@0"), text("b-rule")]),
            ]),
            Value::Tag(0, Box::new(text("2026-07-19T14:00:00Z"))),
        )])))
        .unwrap();
    let (PolicyDocument::Effective(plain), PolicyDocument::Effective(later)) = (&plain, &later)
    else {
        unreachable!()
    };
    assert_eq!(plain.header.semantic_id, effective.header.semantic_id);
    assert_eq!(effective.header.semantic_id, later.header.semantic_id);
    assert_ne!(plain.header.artifact_id, effective.header.artifact_id);
    assert_ne!(effective.header.artifact_id, later.header.artifact_id);
}

#[test]
fn malformed_applied_waiver_audit_entries_fail_closed() {
    let target = array([text("example/policy.org@0"), text("a-rule")]);
    for (applied, expected) in [
        (
            applied_waiver(
                array([]),
                Value::Tag(0, Box::new(text("2026-07-19T13:00:00Z"))),
            ),
            "applied waiver targets must be a non-empty sorted set",
        ),
        (
            applied_waiver(
                array([target.clone(), target]),
                Value::Tag(0, Box::new(text("2026-07-19T13:00:00Z"))),
            ),
            "applied waiver targets must be a non-empty sorted set",
        ),
        (
            applied_waiver(
                array([array([text("example/policy.org@0"), text("a-rule")])]),
                text("2026-07-19T13:00:00Z"),
            ),
            "applied waiver decision_time must be a tag-0 RFC 3339 timestamp",
        ),
        (
            applied_waiver(
                array([array([text("example/policy.org@0"), text("a-rule")])]),
                Value::Tag(0, Box::new(text("2026-02-30T13:00:00Z"))),
            ),
            "applied waiver decision_time is not a valid UTC date-time",
        ),
    ] {
        let error = PolicyDocument::from_value(&effective_policy_with_waivers(array([applied])))
            .unwrap_err();
        assert_eq!(error.code, "BHCP8001");
        assert_eq!(error.message, expected);
    }
}

#[test]
fn invalid_category_operation_value_and_unknown_fields_fail_stably() {
    let baseline = source_policy("repository");
    for (index, operation, expected) in [
        (
            0,
            "deny",
            "policy category \"requirement\" requires operation \"add\"",
        ),
        (
            1,
            "deny",
            "policy category \"evidence\" requires operation \"add\"",
        ),
        (
            2,
            "add",
            "policy category \"prohibition\" requires operation \"deny\"",
        ),
        (
            3,
            "deny",
            "policy category \"capability\" requires operation \"narrow\"",
        ),
        (
            4,
            "add",
            "policy category \"limit\" requires operation \"tighten\"",
        ),
        (
            5,
            "add",
            "policy category \"type-mode\" requires operation \"strengthen\"",
        ),
    ] {
        let error = PolicyDocument::from_value(&replace_rule_field(
            &baseline,
            index,
            "operation",
            text(operation),
        ))
        .unwrap_err();
        assert_eq!(error.code, "BHCP8001");
        assert_eq!(error.message, expected);
    }

    for (mutated, expected) in [
        (
            replace_rule_field(&baseline, 0, "value", text("untyped")),
            "requirement policy value must be a map",
        ),
        (
            replace_rule_field(&baseline, 1, "value", text("untyped")),
            "evidence policy value must be a map",
        ),
        (
            replace_rule_field(&baseline, 2, "value", text("untyped")),
            "capability policy value must be a map",
        ),
        (
            replace_rule_field(&baseline, 3, "value", text("untyped")),
            "capability policy value must be a map",
        ),
        (
            replace_rule_field(&baseline, 4, "value", text("untyped")),
            "limit policy value must be a map",
        ),
        (
            replace_rule_field(&baseline, 5, "value", text("untyped")),
            "type-mode policy value must be dynamic, gradual, infer-strict, or strict",
        ),
        (
            add_rule_field(&baseline, 0, "mystery", Value::Bool(true)),
            "unknown requirement policy rule field \"mystery\"",
        ),
        (
            add_root_field(&baseline, "mystery", Value::Bool(true)),
            "unknown source policy document field \"mystery\"",
        ),
    ] {
        let error = PolicyDocument::from_value(&mutated).unwrap_err();
        assert_eq!(error.code, "BHCP8001");
        assert_eq!(error.message, expected);
    }
}

#[test]
fn normalization_and_scalar_boundaries_fail_closed() {
    let baseline = source_policy("repository");
    for (mutated, expected) in [
        (
            replace_rule_field(
                &baseline,
                1,
                "value",
                Value::map([
                    ("obligation", text("example/obligation.review@0")),
                    ("classes", array([text("static")])),
                    ("minimum", Value::Integer(0)),
                ]),
            ),
            "evidence minimum must be a positive integer",
        ),
        (
            replace_rule_field(
                &baseline,
                4,
                "value",
                Value::map([
                    ("dimension", text("example/limit.attempts@0")),
                    ("unit", text("example/unit.count@0")),
                    ("maximum", array([text("integer"), Value::Integer(-1)])),
                ]),
            ),
            "limit maximum must be a non-negative exact number",
        ),
        (
            replace_rule_field(&baseline, 0, "id", text("z-last")),
            "source policy rules must be sorted by unique rule ID",
        ),
        (
            add_rule_field(&baseline, 0, "authorized_issuers", array([text("issuer")])),
            "non-waivable policy rule must not authorize issuers",
        ),
    ] {
        let error = PolicyDocument::from_value(&mutated).unwrap_err();
        assert_eq!(error.code, "BHCP8001");
        assert_eq!(error.message, expected);
    }
}

#[test]
fn checked_in_policy_fixture_uses_the_strong_model() {
    let source =
        fs::read_to_string(root().join("schemas/v0/examples/layered-policy.diag")).unwrap();
    let value = parse_diagnostic(&source).unwrap();
    let document = PolicyDocument::from_value(&value).unwrap();
    assert_eq!(document.to_value(true), value);
}

fn replace_rule_field(document: &Value, index: usize, key: &str, value: Value) -> Value {
    mutate_rule(document, index, |entries| {
        let entry = entries.iter_mut().find(|(name, _)| name == key).unwrap();
        entry.1 = value;
    })
}

fn add_rule_field(document: &Value, index: usize, key: &str, value: Value) -> Value {
    mutate_rule(document, index, |entries| {
        entries.push((key.to_owned(), value));
    })
}

fn mutate_rule(
    document: &Value,
    index: usize,
    mutate: impl FnOnce(&mut Vec<(String, Value)>),
) -> Value {
    let Value::Map(mut root) = document.clone() else {
        unreachable!()
    };
    let Value::Array(rules) = &mut root.iter_mut().find(|(key, _)| key == "rules").unwrap().1
    else {
        unreachable!()
    };
    let Value::Map(entries) = &mut rules[index] else {
        unreachable!()
    };
    mutate(entries);
    Value::owned_map(root)
}

fn add_root_field(document: &Value, key: &str, value: Value) -> Value {
    let Value::Map(mut entries) = document.clone() else {
        unreachable!()
    };
    entries.push((key.to_owned(), value));
    Value::owned_map(entries)
}
