use bhcp::hash::HashAlgorithm;
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
        ("maximum", array([text("integer"), Value::Integer(maximum)])),
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
