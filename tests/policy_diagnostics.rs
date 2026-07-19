use bhcp::hash::HashAlgorithm;
use bhcp::pipeline::parse_policy_source;
use bhcp::policy::{
    PolicyLayer, PolicyWeakeningAttempt, SourceRuleIdentity, compose_policies,
    reject_policy_weakening,
};

fn diagnostic(source: &str) -> bhcp::diagnostic::Diagnostic {
    let parsed = parse_policy_source(source, "conflict.bhcp").unwrap();
    compose_policies(&parsed.documents, HashAlgorithm::default()).unwrap_err()
}

#[test]
fn widening_loosening_and_type_weakening_have_distinct_auditable_codes() {
    let capability = diagnostic(
        r#"§policy example/org@0 {
  layer organization;
  rule a: capability narrow { effect: bhcp-effect/fs.read@0, scope: { goals: [example/goal.a@0] } } nonwaivable;
}
§policy example/repo@0 {
  layer repository;
  rule a: capability narrow { effect: bhcp-effect/fs.read@0, scope: { goals: [example/goal.a@0, example/goal.b@0] } } nonwaivable;
}"#,
    );
    assert_eq!(capability.code, "BHCP8101");
    assert_eq!(
        capability.message,
        "repository policy example/repo@0 rule a broadens capability bhcp-effect/fs.read@0 from goals=[example/goal.a@0] to goals=[example/goal.a@0,example/goal.b@0]; earlier organization authority example/org@0:a; waiver required"
    );

    let limit = diagnostic(
        r#"§policy example/org@0 {
  layer organization;
  rule a: limit tighten { dimension: example/limit.memory@0, unit: example/unit.byte@0, maximum: ["integer", 5] } nonwaivable;
}
§policy example/repo@0 {
  layer repository;
  rule a: limit tighten { dimension: example/limit.memory@0, unit: example/unit.byte@0, maximum: ["integer", 6] } nonwaivable;
}"#,
    );
    assert_eq!(limit.code, "BHCP8102");
    assert_eq!(
        limit.message,
        "repository policy example/repo@0 rule a loosens limit example/limit.memory@0 from integer(5) to integer(6); earlier organization authority example/org@0:a; waiver required"
    );

    let mode = diagnostic(
        r#"§policy example/org@0 {
  layer organization;
  rule a: type-mode strengthen strict nonwaivable;
}
§policy example/repo@0 {
  layer repository;
  rule a: type-mode strengthen gradual nonwaivable;
}"#,
    );
    assert_eq!(mode.code, "BHCP8103");
    assert_eq!(
        mode.message,
        "repository policy example/repo@0 rule a weakens type mode from strict to gradual; earlier organization authority example/org@0:a; waiver required"
    );
}

#[test]
fn diagnostics_name_the_governing_contributor_after_earlier_rules_compose() {
    let capability = diagnostic(
        r#"§policy example/a-org@0 {
  layer organization;
  rule a: capability narrow { effect: bhcp-effect/fs.read@0, scope: { goals: [example/goal.a@0, example/goal.b@0] } } nonwaivable;
}
§policy example/z-team@0 {
  layer team;
  rule a: capability narrow { effect: bhcp-effect/fs.read@0, scope: { goals: [example/goal.b@0] } } nonwaivable;
}
§policy example/repo@0 {
  layer repository;
  rule a: capability narrow { effect: bhcp-effect/fs.read@0, scope: { goals: [example/goal.a@0, example/goal.b@0] } } nonwaivable;
}"#,
    );
    assert_eq!(capability.code, "BHCP8101");
    assert!(
        capability
            .message
            .contains("earlier team authority example/z-team@0:a")
    );

    let limit = diagnostic(
        r#"§policy example/a-org@0 {
  layer organization;
  rule a: limit tighten { dimension: example/limit.memory@0, unit: example/unit.byte@0, maximum: ["integer", 10] } nonwaivable;
}
§policy example/z-team@0 {
  layer team;
  rule a: limit tighten { dimension: example/limit.memory@0, unit: example/unit.byte@0, maximum: ["integer", 5] } nonwaivable;
}
§policy example/repo@0 {
  layer repository;
  rule a: limit tighten { dimension: example/limit.memory@0, unit: example/unit.byte@0, maximum: ["integer", 6] } nonwaivable;
}"#,
    );
    assert_eq!(limit.code, "BHCP8102");
    assert!(
        limit
            .message
            .contains("earlier team authority example/z-team@0:a")
    );

    let mode = diagnostic(
        r#"§policy example/a-org@0 {
  layer organization;
  rule a: type-mode strengthen gradual nonwaivable;
}
§policy example/z-team@0 {
  layer team;
  rule a: type-mode strengthen strict nonwaivable;
}
§policy example/repo@0 {
  layer repository;
  rule a: type-mode strengthen infer-strict nonwaivable;
}"#,
    );
    assert_eq!(mode.code, "BHCP8103");
    assert!(
        mode.message
            .contains("earlier team authority example/z-team@0:a")
    );
}

#[test]
fn removals_and_allow_over_deny_use_explicit_rejection_codes() {
    let earlier = SourceRuleIdentity {
        policy: "example/org@0".to_owned(),
        rule: "baseline".to_owned(),
    };
    for (attempt, code, message) in [
        (
            PolicyWeakeningAttempt::RemoveRequirement {
                layer: PolicyLayer::Repository,
                policy: "example/repo@0".to_owned(),
                rule: "remove-lint".to_owned(),
                requirement: "example/requirement.lint@0".to_owned(),
                earlier: earlier.clone(),
                earlier_layer: PolicyLayer::Organization,
            },
            "BHCP8104",
            "repository policy example/repo@0 rule remove-lint removes requirement example/requirement.lint@0; earlier organization authority example/org@0:baseline; waiver required",
        ),
        (
            PolicyWeakeningAttempt::RemoveEvidence {
                layer: PolicyLayer::Repository,
                policy: "example/repo@0".to_owned(),
                rule: "remove-review".to_owned(),
                obligation: "example/obligation.review@0".to_owned(),
                earlier: earlier.clone(),
                earlier_layer: PolicyLayer::Organization,
            },
            "BHCP8105",
            "repository policy example/repo@0 rule remove-review removes evidence example/obligation.review@0; earlier organization authority example/org@0:baseline; waiver required",
        ),
        (
            PolicyWeakeningAttempt::AllowDeniedEffect {
                layer: PolicyLayer::Repository,
                policy: "example/repo@0".to_owned(),
                rule: "allow-network".to_owned(),
                effect: "bhcp-effect/network@0".to_owned(),
                earlier,
                earlier_layer: PolicyLayer::Organization,
            },
            "BHCP8106",
            "repository policy example/repo@0 rule allow-network allows denied effect bhcp-effect/network@0; earlier organization authority example/org@0:baseline; waiver required",
        ),
    ] {
        let error = reject_policy_weakening(attempt).unwrap_err();
        assert_eq!(error.code, code);
        assert_eq!(error.message, message);
    }
}

#[test]
fn invalid_layer_is_atomic_and_never_returns_a_partial_effective_policy() {
    let source = r#"§policy example/org@0 {
  layer organization;
  rule a: type-mode strengthen strict nonwaivable;
}
§policy example/repo@0 {
  layer repository;
  rule a: requirement add { requirement: example/requirement.lint@0 } nonwaivable;
  rule b: type-mode strengthen gradual nonwaivable;
}"#;
    let parsed = parse_policy_source(source, "atomic.bhcp").unwrap();
    let result = compose_policies(&parsed.documents, HashAlgorithm::default());
    let error = result.unwrap_err();
    assert_eq!(error.code, "BHCP8103");
    assert!(error.message.contains("waiver required"));
}
