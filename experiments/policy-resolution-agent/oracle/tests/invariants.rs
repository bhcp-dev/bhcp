use tenant_policy::{Decision, Effect, Policy, Rule};

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
fn no_eligible_rule_is_denied_without_a_selected_rule() {
    let policy = Policy::new([rule(
        "other-tenant",
        "other",
        "bob",
        "read",
        "report",
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
fn a_rule_from_another_tenant_is_never_eligible() {
    let policy = Policy::new([
        rule("acme-deny", "acme", "*", "read", "*", 1, Effect::Deny),
        rule(
            "other-allow",
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
        selected(Effect::Deny, "acme-deny")
    );
}

#[test]
fn specificity_dominates_numeric_priority() {
    let policy = Policy::new([
        rule("exact", "acme", "alice", "read", "report", 1, Effect::Allow),
        rule("broad", "acme", "*", "read", "*", 100, Effect::Deny),
    ]);

    assert_eq!(
        policy.decide("acme", "alice", "read", "report"),
        selected(Effect::Allow, "exact")
    );
}

#[test]
fn deny_breaks_an_equal_specificity_and_priority_tie() {
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
fn lexicographically_smaller_id_breaks_a_remaining_tie() {
    let policy = Policy::new([
        rule("alpha", "acme", "*", "read", "report", 4, Effect::Allow),
        rule("zeta", "acme", "alice", "*", "report", 4, Effect::Allow),
    ]);

    assert_eq!(
        policy.decide("acme", "alice", "read", "report"),
        selected(Effect::Allow, "alpha")
    );
}

#[test]
fn insertion_order_does_not_change_the_decision() {
    let alpha = rule("alpha", "acme", "alice", "*", "report", 4, Effect::Allow);
    let zeta = rule("zeta", "acme", "*", "read", "report", 4, Effect::Allow);
    let forward = Policy::new([alpha.clone(), zeta.clone()]);
    let reverse = Policy::new([zeta, alpha]);

    let expected = selected(Effect::Allow, "alpha");
    assert_eq!(forward.decide("acme", "alice", "read", "report"), expected);
    assert_eq!(reverse.decide("acme", "alice", "read", "report"), expected);
}

#[test]
fn higher_priority_breaks_an_equal_specificity_tie() {
    let policy = Policy::new([
        rule("low", "acme", "alice", "*", "report", 2, Effect::Deny),
        rule("high", "acme", "*", "read", "report", 3, Effect::Allow),
    ]);

    assert_eq!(
        policy.decide("acme", "alice", "read", "report"),
        selected(Effect::Allow, "high")
    );
}
