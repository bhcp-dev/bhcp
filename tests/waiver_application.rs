use bhcp::hash::HashAlgorithm;
use bhcp::inspection::render_artifact;
use bhcp::pipeline::parse_policy_source;
use bhcp::policy::{ExactNumber, WaiverDocument, apply_waiver, compose_policies};
use bhcp::value::Value;

fn text(value: &str) -> Value {
    Value::Text(value.to_owned())
}

fn array(values: impl IntoIterator<Item = Value>) -> Value {
    Value::Array(values.into_iter().collect())
}

fn reference() -> Value {
    Value::map([
        ("media_type", text("application/vnd.bhcp.audit+cbor")),
        ("size", Value::Integer(1)),
        (
            "digests",
            array([Value::map([
                ("algorithm", text("bhcp.hash/sha3-512@0")),
                ("digest", Value::Bytes(vec![0; 64])),
            ])]),
        ),
    ])
}

fn limit_value(maximum: i64) -> Value {
    Value::map([
        ("dimension", text("example/dimension.attempts@0")),
        ("unit", text("example/unit.count@0")),
        (
            "maximum",
            array([text("integer"), Value::Integer(i128::from(maximum))]),
        ),
        (
            "scope",
            Value::map([("goals", array([text("example/goal.deploy@0")]))]),
        ),
    ])
}

fn waiver_value(issuer: &str, decision_rule: &str) -> Value {
    Value::map([
        ("version", text("bhcp/v0")),
        ("features", array([])),
        (
            "authorization",
            array([Value::map([("proof", text("signed"))])]),
        ),
        ("kind", text("waiver")),
        ("symbol", text("example/waiver.attempts@0")),
        (
            "targets",
            array([Value::map([
                (
                    "rule",
                    array([text("example/policy.org@0"), text(decision_rule)]),
                ),
                (
                    "scope",
                    Value::map([("goals", array([text("example/goal.deploy@0")]))]),
                ),
                (
                    "weakening",
                    Value::map([
                        ("category", text("limit")),
                        ("operation", text("loosen")),
                        ("from", limit_value(3)),
                        ("to", limit_value(4)),
                    ]),
                ),
            ])]),
        ),
        ("justification", text("one migration retry")),
        ("issuer", text(issuer)),
        ("authority_chain", array([])),
        (
            "issued_at",
            Value::Tag(0, Box::new(text("2026-07-19T12:00:00Z"))),
        ),
        (
            "not_before",
            Value::Tag(0, Box::new(text("2026-07-19T13:00:00Z"))),
        ),
        (
            "expires_at",
            Value::Tag(0, Box::new(text("2026-07-19T15:00:00Z"))),
        ),
        ("audit_reference", reference()),
    ])
}

fn replace_target(document: Value, rule: &str, weakening: Value) -> Value {
    let Value::Map(mut entries) = document else {
        unreachable!()
    };
    let Value::Array(targets) = &mut entries
        .iter_mut()
        .find(|(key, _)| key == "targets")
        .unwrap()
        .1
    else {
        unreachable!()
    };
    let Value::Map(target) = &mut targets[0] else {
        unreachable!()
    };
    target.iter_mut().find(|(key, _)| key == "rule").unwrap().1 =
        array([text("example/policy.org@0"), text(rule)]);
    target
        .iter_mut()
        .find(|(key, _)| key == "weakening")
        .unwrap()
        .1 = weakening;
    Value::owned_map(entries)
}

fn remove_target_scope(document: Value) -> Value {
    let Value::Map(mut entries) = document else {
        unreachable!()
    };
    let Value::Array(targets) = &mut entries
        .iter_mut()
        .find(|(key, _)| key == "targets")
        .unwrap()
        .1
    else {
        unreachable!()
    };
    let Value::Map(target) = &mut targets[0] else {
        unreachable!()
    };
    target.retain(|(key, _)| key != "scope");
    Value::owned_map(entries)
}

fn replace_root_field(document: Value, key: &str, replacement: Value) -> Value {
    let Value::Map(mut entries) = document else {
        unreachable!()
    };
    entries.iter_mut().find(|(name, _)| name == key).unwrap().1 = replacement;
    Value::owned_map(entries)
}

fn policy(waivable: bool) -> bhcp::policy::EffectivePolicyDocument {
    let governance = if waivable {
        "waivable by [\"security-team\"]"
    } else {
        "nonwaivable"
    };
    let source = format!(
        r#"§policy example/policy.org@0 {{
  layer organization;
  rule attempts: limit tighten {{
    dimension: example/dimension.attempts@0,
    unit: example/unit.count@0,
    maximum: ["integer", 3],
    scope: {{ goals: [example/goal.deploy@0] }}
  }} {governance};
}}"#
    );
    let parsed = parse_policy_source(&source, "waiver-policy.bhcp").unwrap();
    compose_policies(&parsed.documents, HashAlgorithm::default()).unwrap()
}

fn capability_policy() -> bhcp::policy::EffectivePolicyDocument {
    let source = r#"§policy example/policy.org@0 {
  layer organization;
  rule filesystem: capability narrow {
    effect: bhcp-effect/fs.read@0,
    scope: { goals: [example/goal.deploy@0] }
  } waivable by ["security-team"];
}"#;
    let parsed = parse_policy_source(source, "waiver-capability.bhcp").unwrap();
    compose_policies(&parsed.documents, HashAlgorithm::default()).unwrap()
}

fn type_mode_policy() -> bhcp::policy::EffectivePolicyDocument {
    let source = r#"§policy example/policy.org@0 {
  layer organization;
  rule typing: type-mode strengthen strict waivable by ["security-team"];
}"#;
    let parsed = parse_policy_source(source, "waiver-type-mode.bhcp").unwrap();
    compose_policies(&parsed.documents, HashAlgorithm::default()).unwrap()
}

fn additive_policy() -> bhcp::policy::EffectivePolicyDocument {
    let source = r#"§policy example/policy.org@0 {
  layer organization;
  rule a-requirement: requirement add {
    requirement: example/requirement.review@0,
    scope: { goals: [example/goal.deploy@0] }
  } waivable by ["security-team"];
  rule b-evidence: evidence add {
    obligation: example/obligation.review@0,
    classes: [static],
    minimum: 1,
    scope: { goals: [example/goal.deploy@0] }
  } waivable by ["security-team"];
  rule c-prohibition: prohibition deny {
    effect: bhcp-effect/network@0,
    scope: { goals: [example/goal.deploy@0] }
  } waivable by ["security-team"];
}"#;
    let parsed = parse_policy_source(source, "waiver-additive.bhcp").unwrap();
    compose_policies(&parsed.documents, HashAlgorithm::default()).unwrap()
}

#[test]
fn valid_exact_waiver_is_deterministic_audited_and_changes_only_effective_meaning() {
    let baseline = policy(true);
    let waiver = WaiverDocument::from_value(&waiver_value("security-team", "attempts")).unwrap();
    let first = apply_waiver(
        &baseline,
        &waiver,
        "2026-07-19T13:00:00Z",
        HashAlgorithm::default(),
    )
    .unwrap();
    let second = apply_waiver(
        &baseline,
        &waiver,
        "2026-07-19T13:00:00Z",
        HashAlgorithm::default(),
    )
    .unwrap();

    assert_eq!(first, second);
    assert_eq!(
        first.effective.limits[0].value.maximum,
        ExactNumber::Integer(4)
    );
    assert_ne!(first.header.semantic_id, baseline.header.semantic_id);
    assert_ne!(first.header.artifact_id, baseline.header.artifact_id);
    let applied = &first.waivers.as_ref().unwrap()[0];
    assert_eq!(applied.targets[0].policy, "example/policy.org@0");
    assert_eq!(applied.decision_time, "2026-07-19T13:00:00Z");
    let inspection = render_artifact(
        &bhcp::policy::PolicyDocument::Effective(first).to_value(true),
        Some("waived-policy.cbor"),
    );
    assert!(inspection.contains("applied-waivers 1"));
    assert!(inspection.contains("example/policy.org@0#attempts"));
    assert!(inspection.contains("2026-07-19T13:00:00Z"));
}

#[test]
fn invalid_or_inactive_waivers_fail_atomically_with_stable_diagnostics() {
    let baseline = policy(true);
    for (name, waiver, decision_time, code) in [
        (
            "unknown rule",
            waiver_value("security-team", "missing"),
            "2026-07-19T13:00:00Z",
            "BHCP8302",
        ),
        (
            "unauthorized issuer",
            waiver_value("release-manager", "attempts"),
            "2026-07-19T13:00:00Z",
            "BHCP8305",
        ),
        (
            "expiry equality",
            waiver_value("security-team", "attempts"),
            "2026-07-19T15:00:00Z",
            "BHCP8304",
        ),
    ] {
        let waiver = WaiverDocument::from_value(&waiver).unwrap();
        let error =
            apply_waiver(&baseline, &waiver, decision_time, HashAlgorithm::default()).unwrap_err();
        assert_eq!(error.code, code, "{name}");
        assert_eq!(baseline, policy(true), "{name} mutated the input");
    }

    let waiver = WaiverDocument::from_value(&waiver_value("security-team", "attempts")).unwrap();
    let error = apply_waiver(
        &policy(false),
        &waiver,
        "2026-07-19T13:00:00Z",
        HashAlgorithm::default(),
    )
    .unwrap_err();
    assert_eq!(error.code, "BHCP8306");
}

#[test]
fn capability_broadening_applies_the_same_exact_scope_authority_and_audit_boundary() {
    let scope = Value::map([("goals", array([text("example/goal.deploy@0")]))]);
    let capability = |goals: &[&str]| {
        Value::map([
            ("effect", text("bhcp-effect/fs.read@0")),
            (
                "scope",
                Value::map([("goals", array(goals.iter().map(|goal| text(goal))))]),
            ),
        ])
    };
    let weakening = Value::map([
        ("category", text("capability")),
        ("operation", text("broaden")),
        ("from", capability(&["example/goal.deploy@0"])),
        (
            "to",
            capability(&["example/goal.deploy@0", "example/goal.preview@0"]),
        ),
    ]);
    let mut value = replace_target(
        waiver_value("security-team", "filesystem"),
        "filesystem",
        weakening,
    );
    let Value::Map(entries) = &mut value else {
        unreachable!()
    };
    let Value::Array(targets) = &mut entries
        .iter_mut()
        .find(|(key, _)| key == "targets")
        .unwrap()
        .1
    else {
        unreachable!()
    };
    let Value::Map(target) = &mut targets[0] else {
        unreachable!()
    };
    target.iter_mut().find(|(key, _)| key == "scope").unwrap().1 = scope;

    let waiver = WaiverDocument::from_value(&value).unwrap();
    let effective = apply_waiver(
        &capability_policy(),
        &waiver,
        "2026-07-19T13:30:00Z",
        HashAlgorithm::default(),
    )
    .unwrap();
    assert_eq!(
        effective.effective.capabilities[0]
            .value
            .scope
            .as_ref()
            .unwrap()
            .goals
            .as_ref()
            .unwrap(),
        &["example/goal.deploy@0", "example/goal.preview@0"]
    );
}

#[test]
fn type_mode_weakening_is_exact_and_preserves_the_audit_boundary() {
    let weakening = Value::map([
        ("category", text("type-mode")),
        ("operation", text("weaken")),
        ("from", text("strict")),
        ("to", text("infer-strict")),
    ]);
    let value = remove_target_scope(replace_target(
        waiver_value("security-team", "typing"),
        "typing",
        weakening,
    ));
    let waiver = WaiverDocument::from_value(&value).unwrap();
    let effective = apply_waiver(
        &type_mode_policy(),
        &waiver,
        "2026-07-19T13:30:00Z",
        HashAlgorithm::default(),
    )
    .unwrap();
    assert_eq!(
        effective.effective.type_mode.value,
        bhcp::policy::TypeMode::InferStrict
    );
}

#[test]
fn additive_removals_and_allow_over_deny_remove_only_the_exact_target() {
    let scope = || Value::map([("goals", array([text("example/goal.deploy@0")]))]);
    let cases = [
        (
            "a-requirement",
            Value::map([
                ("category", text("requirement")),
                ("operation", text("remove")),
                (
                    "value",
                    Value::map([
                        ("requirement", text("example/requirement.review@0")),
                        ("scope", scope()),
                    ]),
                ),
            ]),
            (0, 1, 1),
        ),
        (
            "b-evidence",
            Value::map([
                ("category", text("evidence")),
                ("operation", text("remove")),
                (
                    "value",
                    Value::map([
                        ("obligation", text("example/obligation.review@0")),
                        ("classes", array([text("static")])),
                        ("minimum", Value::Integer(1)),
                        ("scope", scope()),
                    ]),
                ),
            ]),
            (1, 0, 1),
        ),
        (
            "c-prohibition",
            Value::map([
                ("category", text("prohibition")),
                ("operation", text("allow")),
                (
                    "value",
                    Value::map([
                        ("effect", text("bhcp-effect/network@0")),
                        ("scope", scope()),
                    ]),
                ),
            ]),
            (1, 1, 0),
        ),
    ];
    for (rule, weakening, expected) in cases {
        let value = replace_target(waiver_value("security-team", rule), rule, weakening);
        let waiver = WaiverDocument::from_value(&value).unwrap();
        let effective = apply_waiver(
            &additive_policy(),
            &waiver,
            "2026-07-19T13:30:00Z",
            HashAlgorithm::default(),
        )
        .unwrap();
        assert_eq!(
            (
                effective.effective.requirements.len(),
                effective.effective.evidence.len(),
                effective.effective.prohibitions.len(),
            ),
            expected,
            "{rule}"
        );
    }
}

#[test]
fn finite_delegation_chain_must_connect_an_authorized_root_to_the_exact_issuer() {
    let first_delegation = Value::map([
        ("delegator", text("security-team")),
        ("delegate", text("security-lead")),
        ("authorization", reference()),
    ]);
    let second_delegation = Value::map([
        ("delegator", text("security-lead")),
        ("delegate", text("release-manager")),
        ("authorization", reference()),
    ]);
    let value = replace_root_field(
        waiver_value("release-manager", "attempts"),
        "authority_chain",
        array([first_delegation, second_delegation]),
    );
    let waiver = WaiverDocument::from_value(&value).unwrap();
    apply_waiver(
        &policy(true),
        &waiver,
        "2026-07-19T13:30:00Z",
        HashAlgorithm::default(),
    )
    .unwrap();

    let unauthorized = Value::map([
        ("delegator", text("untrusted-root")),
        ("delegate", text("release-manager")),
        ("authorization", reference()),
    ]);
    let value = replace_root_field(
        waiver_value("release-manager", "attempts"),
        "authority_chain",
        array([unauthorized]),
    );
    let waiver = WaiverDocument::from_value(&value).unwrap();
    let error = apply_waiver(
        &policy(true),
        &waiver,
        "2026-07-19T13:30:00Z",
        HashAlgorithm::default(),
    )
    .unwrap_err();
    assert_eq!(error.code, "BHCP8305");
}
