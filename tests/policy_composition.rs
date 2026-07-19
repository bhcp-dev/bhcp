use bhcp::hash::HashAlgorithm;
use bhcp::policy::{
    ExactNumber, PolicyCategory, PolicyDocument, PolicyLayer, SourcePolicyDocument, TypeMode,
    compose_policies,
};
use bhcp::value::Value;

fn text(value: &str) -> Value {
    Value::Text(value.to_owned())
}

fn array(values: impl IntoIterator<Item = Value>) -> Value {
    Value::Array(values.into_iter().collect())
}

fn rule(
    id: &str,
    category: &str,
    operation: &str,
    value: Value,
    issuers: &[&str],
) -> Value {
    let mut entries = vec![
        ("id".to_owned(), text(id)),
        ("category".to_owned(), text(category)),
        ("operation".to_owned(), text(operation)),
        ("value".to_owned(), value),
        ("waivable".to_owned(), Value::Bool(!issuers.is_empty())),
    ];
    if !issuers.is_empty() {
        entries.push((
            "authorized_issuers".to_owned(),
            array(issuers.iter().map(|issuer| text(issuer))),
        ));
    }
    Value::owned_map(entries)
}

fn requirement(id: &str, symbol: &str, issuers: &[&str]) -> Value {
    rule(
        id,
        "requirement",
        "add",
        Value::map([("requirement", text(symbol))]),
        issuers,
    )
}

fn evidence(id: &str, symbol: &str) -> Value {
    rule(
        id,
        "evidence",
        "add",
        Value::map([
            ("obligation", text(symbol)),
            ("classes", array([text("static")])),
            ("minimum", Value::Integer(1)),
        ]),
        &[],
    )
}

fn scoped_effect_rule(
    id: &str,
    category: &str,
    operation: &str,
    effect: &str,
    goals: Option<&[&str]>,
) -> Value {
    let mut value = vec![("effect".to_owned(), text(effect))];
    if let Some(goals) = goals {
        value.push((
            "scope".to_owned(),
            Value::map([("goals", array(goals.iter().map(|goal| text(goal))))]),
        ));
    }
    rule(
        id,
        category,
        operation,
        Value::owned_map(value),
        &[],
    )
}

fn limit(id: &str, maximum: i64, unit: &str, goals: Option<&[&str]>) -> Value {
    let mut value = vec![
        ("dimension".to_owned(), text("example/limit.memory@0")),
        ("unit".to_owned(), text(unit)),
        (
            "maximum".to_owned(),
            array([text("integer"), Value::Integer(maximum)]),
        ),
    ];
    if let Some(goals) = goals {
        value.push((
            "scope".to_owned(),
            Value::map([("goals", array(goals.iter().map(|goal| text(goal))))]),
        ));
    }
    rule(id, "limit", "tighten", Value::owned_map(value), &[])
}

fn type_mode(id: &str, mode: &str) -> Value {
    rule(id, "type-mode", "strengthen", text(mode), &[])
}

fn source(
    symbol: &str,
    layer: &str,
    extends: Option<&str>,
    rules: Vec<Value>,
) -> SourcePolicyDocument {
    let mut entries = vec![
        ("version".to_owned(), text("bhcp/v0")),
        ("features".to_owned(), array([])),
        ("kind".to_owned(), text("policy")),
        ("form".to_owned(), text("source")),
        ("symbol".to_owned(), text(symbol)),
        ("layer".to_owned(), text(layer)),
        ("rules".to_owned(), array(rules)),
    ];
    if let Some(extends) = extends {
        entries.push(("extends".to_owned(), text(extends)));
    }
    let PolicyDocument::Source(document) =
        PolicyDocument::from_value(&Value::owned_map(entries)).unwrap()
    else {
        unreachable!()
    };
    document
}

#[test]
fn layers_compose_restrictively_with_canonical_provenance_and_identity() {
    let org = source(
        "example/policy.org@0",
        "organization",
        None,
        vec![
            requirement(
                "a-requirement",
                "example/requirement.lint@0",
                &["security", "team-owner"],
            ),
            evidence("b-evidence", "example/obligation.review@0"),
            scoped_effect_rule(
                "c-prohibition",
                "prohibition",
                "deny",
                "bhcp-effect/network@0",
                None,
            ),
            scoped_effect_rule(
                "d-capability",
                "capability",
                "narrow",
                "bhcp-effect/fs.read@0",
                Some(&["example/goal.a@0", "example/goal.b@0"]),
            ),
            limit("e-limit", 10, "example/unit.byte@0", None),
            type_mode("f-mode", "gradual"),
        ],
    );
    let repository = source(
        "example/policy.repo@0",
        "repository",
        None,
        vec![
            requirement(
                "a-requirement",
                "example/requirement.lint@0",
                &["release", "team-owner"],
            ),
            scoped_effect_rule(
                "b-capability",
                "capability",
                "narrow",
                "bhcp-effect/fs.read@0",
                Some(&["example/goal.b@0"]),
            ),
            limit("c-limit", 5, "example/unit.byte@0", None),
            type_mode("d-mode", "strict"),
        ],
    );

    let document = compose_policies(
        &[repository.clone(), org.clone()],
        HashAlgorithm::default(),
    )
    .unwrap();
    document.validate().unwrap();
    assert_eq!(
        document
            .source_layers
            .iter()
            .map(|layer| layer.layer)
            .collect::<Vec<_>>(),
        vec![PolicyLayer::Organization, PolicyLayer::Repository]
    );
    assert_eq!(document.effective.requirements.len(), 1);
    assert!(document.effective.requirements[0].waivable);
    assert_eq!(
        document.effective.requirements[0].authorized_issuers,
        vec!["team-owner"]
    );
    assert_eq!(document.effective.evidence.len(), 1);
    assert_eq!(document.effective.prohibitions.len(), 1);
    assert_eq!(document.effective.capabilities.len(), 1);
    assert_eq!(
        document.effective.capabilities[0]
            .value
            .scope
            .as_ref()
            .unwrap()
            .goals
            .as_ref()
            .unwrap(),
        &["example/goal.b@0"]
    );
    assert_eq!(
        document.effective.limits[0].value.maximum,
        ExactNumber::Integer(5)
    );
    assert_eq!(document.effective.type_mode.value, TypeMode::Strict);
    assert!(document.header.semantic_id.is_some());
    assert!(document.header.artifact_id.is_some());

    let requirement_provenance = document
        .rule_provenance
        .iter()
        .find(|entry| entry.category == PolicyCategory::Requirement)
        .unwrap();
    assert_eq!(requirement_provenance.sources.len(), 2);
    let bytes = PolicyDocument::Effective(document.clone())
        .to_cbor(true)
        .unwrap();
    assert_eq!(
        PolicyDocument::from_cbor(&bytes).unwrap(),
        PolicyDocument::Effective(document)
    );
}

#[test]
fn later_layers_cannot_hide_capability_limit_or_type_weakening() {
    let org = source(
        "example/policy.org@0",
        "organization",
        None,
        vec![
            scoped_effect_rule(
                "a-capability",
                "capability",
                "narrow",
                "bhcp-effect/fs.read@0",
                Some(&["example/goal.a@0"]),
            ),
            limit("b-limit", 5, "example/unit.byte@0", None),
            type_mode("c-mode", "strict"),
        ],
    );

    for (later_rule, expected) in [
        (
            scoped_effect_rule(
                "a-capability",
                "capability",
                "narrow",
                "bhcp-effect/fs.read@0",
                Some(&["example/goal.a@0", "example/goal.b@0"]),
            ),
            "repository policy example/policy.repo@0 rule a-capability broadens capability bhcp-effect/fs.read@0",
        ),
        (
            limit("a-limit", 6, "example/unit.byte@0", None),
            "repository policy example/policy.repo@0 rule a-limit loosens limit example/limit.memory@0",
        ),
        (
            type_mode("a-mode", "gradual"),
            "repository policy example/policy.repo@0 rule a-mode weakens type mode",
        ),
    ] {
        let later = source(
            "example/policy.repo@0",
            "repository",
            None,
            vec![later_rule],
        );
        let error = compose_policies(&[org.clone(), later], HashAlgorithm::default()).unwrap_err();
        assert_eq!(error.code, "BHCP8002");
        assert_eq!(error.message, expected);
    }
}

#[test]
fn overlapping_limit_units_and_invalid_inheritance_fail_closed() {
    let base = source(
        "example/policy.base@0",
        "organization",
        None,
        vec![limit("a-limit", 5, "example/unit.byte@0", None)],
    );
    let wrong_unit = source(
        "example/policy.repo@0",
        "repository",
        None,
        vec![limit("a-limit", 4, "example/unit.word@0", None)],
    );
    let error = compose_policies(&[base.clone(), wrong_unit], HashAlgorithm::default()).unwrap_err();
    assert_eq!(error.code, "BHCP8002");
    assert_eq!(
        error.message,
        "overlapping limit example/limit.memory@0 uses incompatible units example/unit.byte@0 and example/unit.word@0"
    );

    let missing = source(
        "example/policy.child@0",
        "organization",
        Some("example/policy.missing@0"),
        vec![],
    );
    assert_eq!(
        compose_policies(&[missing], HashAlgorithm::default())
            .unwrap_err()
            .message,
        "policy example/policy.child@0 extends missing policy example/policy.missing@0"
    );

    let duplicate = compose_policies(&[base.clone(), base], HashAlgorithm::default()).unwrap_err();
    assert_eq!(duplicate.message, "duplicate policy source example/policy.base@0");
}

#[test]
fn cycles_and_cross_layer_inheritance_are_rejected() {
    let left = source(
        "example/policy.left@0",
        "organization",
        Some("example/policy.right@0"),
        vec![],
    );
    let right = source(
        "example/policy.right@0",
        "organization",
        Some("example/policy.left@0"),
        vec![],
    );
    assert_eq!(
        compose_policies(&[left, right], HashAlgorithm::default())
            .unwrap_err()
            .message,
        "policy inheritance cycle includes example/policy.left@0"
    );

    let org = source("example/policy.org@0", "organization", None, vec![]);
    let repo = source(
        "example/policy.repo@0",
        "repository",
        Some("example/policy.org@0"),
        vec![],
    );
    assert_eq!(
        compose_policies(&[org, repo], HashAlgorithm::default())
            .unwrap_err()
            .message,
        "policy example/policy.repo@0 extends a policy in another layer"
    );
}

#[test]
fn equivalent_decompositions_have_identical_effective_meaning_but_distinct_artifacts() {
    let combined = source(
        "example/policy.combined@0",
        "team",
        None,
        vec![
            requirement("a-requirement", "example/requirement.lint@0", &[]),
            type_mode("b-mode", "strict"),
        ],
    );
    let split_a = source(
        "example/policy.a@0",
        "team",
        None,
        vec![requirement(
            "a-requirement",
            "example/requirement.lint@0",
            &[],
        )],
    );
    let split_b = source(
        "example/policy.b@0",
        "team",
        None,
        vec![type_mode("a-mode", "strict")],
    );

    let one = compose_policies(&[combined], HashAlgorithm::default()).unwrap();
    let split = compose_policies(&[split_b, split_a], HashAlgorithm::default()).unwrap();
    assert_eq!(one.effective, split.effective);
    assert_eq!(one.header.semantic_id, split.header.semantic_id);
    assert_ne!(one.header.artifact_id, split.header.artifact_id);
}
