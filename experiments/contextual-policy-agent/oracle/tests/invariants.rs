use contextual_policy::{Decision, Effect, Policy, Rule};

fn rule(
    id: &str,
    tenant: &str,
    subject: &str,
    action: &str,
    resource: &str,
    priority: i32,
    effect: Effect,
) -> Rule {
    Rule::new(id, tenant, subject, action, resource, priority, effect)
}

fn selected(effect: Effect, id: &str) -> Decision {
    Decision {
        effect,
        rule_id: Some(id.to_owned()),
    }
}

#[test]
fn no_eligible_rule_defaults_to_deny() {
    let policy = Policy::new([rule(
        "irrelevant",
        "acme",
        "bob",
        "write",
        "ledger",
        100,
        Effect::Allow,
    )]);
    assert_eq!(
        policy.decide("acme", "alice", "read", "report"),
        Decision {
            effect: Effect::Deny,
            rule_id: None,
        }
    );
}

#[test]
fn rules_are_tenant_local() {
    let policy = Policy::new([
        rule(
            "wildcard-tenant",
            "*",
            "alice",
            "read",
            "report",
            200,
            Effect::Allow,
        ),
        rule(
            "foreign",
            "other",
            "alice",
            "read",
            "report",
            100,
            Effect::Allow,
        ),
    ]);
    assert_eq!(
        policy.decide("acme", "alice", "read", "report"),
        Decision {
            effect: Effect::Deny,
            rule_id: None,
        }
    );
}

#[test]
fn resource_specificity_dominates_other_exact_fields() {
    let policy = Policy::new([
        rule(
            "resource",
            "acme",
            "*",
            "*",
            "report",
            -10,
            Effect::Allow,
        ),
        rule(
            "identity-action",
            "acme",
            "alice",
            "read",
            "*",
            100,
            Effect::Deny,
        ),
    ]);
    assert_eq!(
        policy.decide("acme", "alice", "read", "report"),
        selected(Effect::Allow, "resource")
    );
}

#[test]
fn subject_specificity_breaks_equal_resource_scope() {
    let policy = Policy::new([
        rule(
            "subject",
            "acme",
            "alice",
            "*",
            "report",
            -10,
            Effect::Allow,
        ),
        rule(
            "action",
            "acme",
            "*",
            "read",
            "report",
            100,
            Effect::Deny,
        ),
    ]);
    assert_eq!(
        policy.decide("acme", "alice", "read", "report"),
        selected(Effect::Allow, "subject")
    );
}

#[test]
fn action_specificity_breaks_remaining_shape_ties() {
    let policy = Policy::new([
        rule("broad", "acme", "alice", "*", "report", 100, Effect::Deny),
        rule("action", "acme", "alice", "read", "report", -10, Effect::Allow),
    ]);
    assert_eq!(
        policy.decide("acme", "alice", "read", "report"),
        selected(Effect::Allow, "action")
    );
}

#[test]
fn priority_breaks_equal_specificity_ties() {
    let policy = Policy::new([
        rule("low", "acme", "alice", "read", "*", -4, Effect::Deny),
        rule("high", "acme", "alice", "read", "*", 7, Effect::Allow),
    ]);
    assert_eq!(
        policy.decide("acme", "alice", "read", "report"),
        selected(Effect::Allow, "high")
    );
}

#[test]
fn deny_breaks_an_equal_policy_tie() {
    let policy = Policy::new([
        rule("deny", "acme", "alice", "read", "*", 7, Effect::Deny),
        rule("allow", "acme", "alice", "read", "*", 7, Effect::Allow),
    ]);
    assert_eq!(
        policy.decide("acme", "alice", "read", "report"),
        selected(Effect::Deny, "deny")
    );
}

#[test]
fn smaller_rule_id_breaks_the_final_tie() {
    let policy = Policy::new([
        rule("alpha", "acme", "*", "read", "report", 3, Effect::Allow),
        rule("zeta", "acme", "*", "read", "report", 3, Effect::Allow),
    ]);
    assert_eq!(
        policy.decide("acme", "alice", "read", "report"),
        selected(Effect::Allow, "alpha")
    );
}

#[test]
fn insertion_order_is_not_semantic() {
    let alpha = rule("alpha", "acme", "alice", "read", "*", 2, Effect::Allow);
    let zeta = rule("zeta", "acme", "alice", "read", "*", 2, Effect::Allow);
    let forward = Policy::new([alpha.clone(), zeta.clone()]);
    let reverse = Policy::new([zeta, alpha]);
    let expected = selected(Effect::Allow, "alpha");
    assert_eq!(forward.decide("acme", "alice", "read", "report"), expected);
    assert_eq!(reverse.decide("acme", "alice", "read", "report"), expected);
}

#[test]
fn disabled_rules_remain_ineligible() {
    let policy = Policy::new([
        rule("enabled", "acme", "*", "read", "*", 1, Effect::Allow),
        rule("disabled", "acme", "alice", "read", "report", 100, Effect::Deny).disabled(),
    ]);
    assert_eq!(
        policy.decide("acme", "alice", "read", "report"),
        selected(Effect::Allow, "enabled")
    );
}
