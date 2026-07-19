use bhcp::hash::HashAlgorithm;
use bhcp::inspection::render_artifact;
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

fn rule(id: &str, category: &str, operation: &str, value: Value, issuers: &[&str]) -> Value {
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
    rule(id, category, operation, Value::owned_map(value), &[])
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

fn exact_limit_source(symbol: &str, maximum: Value) -> SourcePolicyDocument {
    source(
        symbol,
        "team",
        None,
        vec![rule(
            "a-limit",
            "limit",
            "tighten",
            Value::map([
                ("dimension", text("example/limit.memory@0")),
                ("unit", text("example/unit.byte@0")),
                ("maximum", maximum),
            ]),
            &[],
        )],
    )
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
            scoped_effect_rule(
                "d-network-capability",
                "capability",
                "narrow",
                "bhcp-effect/network@0",
                None,
            ),
            type_mode("e-mode", "strict"),
        ],
    );

    let document =
        compose_policies(&[repository.clone(), org.clone()], HashAlgorithm::default()).unwrap();
    PolicyDocument::Effective(document.clone())
        .validate()
        .unwrap();
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
    assert_eq!(document.effective.capabilities.len(), 2);
    let fs_read = document
        .effective
        .capabilities
        .iter()
        .find(|rule| rule.value.effect == "bhcp-effect/fs.read@0")
        .unwrap();
    assert_eq!(
        fs_read
            .value
            .scope
            .as_ref()
            .unwrap()
            .goals
            .as_ref()
            .unwrap(),
        &["example/goal.b@0"]
    );
    assert!(
        document
            .effective
            .capabilities
            .iter()
            .any(|rule| rule.value.effect == "bhcp-effect/network@0")
    );
    assert!(
        document
            .effective
            .prohibitions
            .iter()
            .any(|rule| rule.value.effect == "bhcp-effect/network@0")
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
    let outline = render_artifact(
        &PolicyDocument::Effective(document.clone()).to_value(true),
        None,
    );
    assert!(outline.contains("policy effective"));
    assert!(outline.contains("source-layer organization 1"));
    assert!(outline.contains("source-layer repository 1"));
    assert!(outline.contains("prohibitions 1"));
    assert!(outline.contains("capabilities 2"));
    assert!(outline.contains("type-mode strict"));
    assert!(outline.contains("rule-provenance"));
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

    for (later_rule, code, expected) in [
        (
            scoped_effect_rule(
                "a-capability",
                "capability",
                "narrow",
                "bhcp-effect/fs.read@0",
                Some(&["example/goal.a@0", "example/goal.b@0"]),
            ),
            "BHCP8101",
            "repository policy example/policy.repo@0 rule a-capability broadens capability bhcp-effect/fs.read@0 from goals=[example/goal.a@0] to goals=[example/goal.a@0,example/goal.b@0]; earlier organization authority example/policy.org@0:a-capability; waiver required",
        ),
        (
            limit("a-limit", 6, "example/unit.byte@0", None),
            "BHCP8102",
            "repository policy example/policy.repo@0 rule a-limit loosens limit example/limit.memory@0 from integer(5) to integer(6); earlier organization authority example/policy.org@0:b-limit; waiver required",
        ),
        (
            type_mode("a-mode", "gradual"),
            "BHCP8103",
            "repository policy example/policy.repo@0 rule a-mode weakens type mode from strict to gradual; earlier organization authority example/policy.org@0:c-mode; waiver required",
        ),
    ] {
        let later = source(
            "example/policy.repo@0",
            "repository",
            None,
            vec![later_rule],
        );
        let error = compose_policies(&[org.clone(), later], HashAlgorithm::default()).unwrap_err();
        assert_eq!(error.code, code);
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
    let error =
        compose_policies(&[base.clone(), wrong_unit], HashAlgorithm::default()).unwrap_err();
    assert_eq!(error.code, "BHCP8107");
    assert_eq!(
        error.message,
        "repository policy example/policy.repo@0 rule a-limit uses incompatible unit example/unit.word@0 for overlapping limit example/limit.memory@0; earlier unit example/unit.byte@0; earlier organization authority example/policy.base@0:a-limit; waiver required"
    );

    let missing = source(
        "example/policy.child@0",
        "organization",
        Some("example/policy.missing@0"),
        vec![],
    );
    let missing_error = compose_policies(&[missing], HashAlgorithm::default()).unwrap_err();
    assert_eq!(missing_error.code, "BHCP8110");
    assert_eq!(
        missing_error.message,
        "policy example/policy.child@0 extends missing policy example/policy.missing@0"
    );

    let duplicate = compose_policies(&[base.clone(), base], HashAlgorithm::default()).unwrap_err();
    assert_eq!(duplicate.code, "BHCP8110");
    assert_eq!(
        duplicate.message,
        "duplicate policy source example/policy.base@0"
    );
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
    let cycle = compose_policies(&[left, right], HashAlgorithm::default()).unwrap_err();
    assert_eq!(cycle.code, "BHCP8110");
    assert_eq!(
        cycle.message,
        "policy inheritance cycle includes example/policy.left@0"
    );

    let org = source("example/policy.org@0", "organization", None, vec![]);
    let repo = source(
        "example/policy.repo@0",
        "repository",
        Some("example/policy.org@0"),
        vec![],
    );
    let cross_layer = compose_policies(&[org, repo], HashAlgorithm::default()).unwrap_err();
    assert_eq!(cross_layer.code, "BHCP8110");
    assert_eq!(
        cross_layer.message,
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

#[test]
fn identity_scope_exact_numbers_and_same_layer_units_are_canonical() {
    let identity = compose_policies(&[], HashAlgorithm::default()).unwrap();
    assert!(identity.source_layers.is_empty());
    assert!(identity.rule_provenance.is_empty());
    assert_eq!(identity.effective.type_mode.value, TypeMode::Dynamic);
    assert!(!identity.effective.type_mode.waivable);

    let absent_scope = source(
        "example/policy.a@0",
        "team",
        None,
        vec![requirement(
            "a-requirement",
            "example/requirement.lint@0",
            &[],
        )],
    );
    let empty_scope = source(
        "example/policy.b@0",
        "team",
        None,
        vec![rule(
            "a-requirement",
            "requirement",
            "add",
            Value::map([
                ("requirement", text("example/requirement.lint@0")),
                ("scope", Value::owned_map(vec![])),
            ]),
            &[],
        )],
    );
    let normalized =
        compose_policies(&[empty_scope, absent_scope], HashAlgorithm::default()).unwrap();
    assert_eq!(normalized.effective.requirements.len(), 1);
    assert!(normalized.effective.requirements[0].value.scope.is_none());

    let half = source(
        "example/policy.half@0",
        "organization",
        None,
        vec![rule(
            "a-limit",
            "limit",
            "tighten",
            Value::map([
                ("dimension", text("example/limit.memory@0")),
                ("unit", text("example/unit.byte@0")),
                (
                    "maximum",
                    array([text("rational"), Value::Integer(1), Value::Integer(2)]),
                ),
            ]),
            &[],
        )],
    );
    let four_tenths = source(
        "example/policy.decimal@0",
        "repository",
        None,
        vec![rule(
            "a-limit",
            "limit",
            "tighten",
            Value::map([
                ("dimension", text("example/limit.memory@0")),
                ("unit", text("example/unit.byte@0")),
                (
                    "maximum",
                    array([text("decimal"), Value::Integer(4), Value::Integer(-1)]),
                ),
            ]),
            &[],
        )],
    );
    let exact = compose_policies(&[four_tenths, half], HashAlgorithm::default()).unwrap();
    assert_eq!(
        exact.effective.limits[0].value.maximum,
        ExactNumber::Decimal {
            coefficient: 4,
            exponent: -1
        }
    );

    let rational_half = array([text("rational"), Value::Integer(1), Value::Integer(2)]);
    let decimal_half = array([text("decimal"), Value::Integer(5), Value::Integer(-1)]);
    let representation_a = compose_policies(
        &[
            exact_limit_source("example/policy.a@0", rational_half.clone()),
            exact_limit_source("example/policy.b@0", decimal_half.clone()),
        ],
        HashAlgorithm::default(),
    )
    .unwrap();
    let representation_b = compose_policies(
        &[
            exact_limit_source("example/policy.a@0", decimal_half),
            exact_limit_source("example/policy.b@0", rational_half),
        ],
        HashAlgorithm::default(),
    )
    .unwrap();
    assert_eq!(representation_a.effective, representation_b.effective);
    assert_eq!(
        representation_a.header.semantic_id,
        representation_b.header.semantic_id
    );

    let mixed_units = source(
        "example/policy.units@0",
        "team",
        None,
        vec![
            limit("a-byte", 5, "example/unit.byte@0", None),
            limit("b-word", 4, "example/unit.word@0", None),
        ],
    );
    let error = compose_policies(&[mixed_units], HashAlgorithm::default()).unwrap_err();
    assert_eq!(error.code, "BHCP8107");
    assert_eq!(
        error.message,
        "team policy example/policy.units@0 rule b-word uses incompatible unit example/unit.word@0 for overlapping limit example/limit.memory@0; earlier unit example/unit.byte@0; earlier team authority example/policy.units@0:a-byte; waiver required"
    );
}
