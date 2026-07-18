use tenant_policy::{Decision, Effect, Policy, Rule};

fn rule(
    id: &str,
    subject: &str,
    action: &str,
    resource: &str,
    priority: i32,
    effect: Effect,
) -> Rule {
    Rule::new(id, "acme", subject, action, resource, priority, effect)
}

#[test]
fn a_matching_rule_selects_its_effect_and_id() {
    let policy = Policy::new([rule(
        "allow-read",
        "alice",
        "read",
        "report",
        10,
        Effect::Allow,
    )]);

    assert_eq!(
        policy.decide("acme", "alice", "read", "report"),
        Decision {
            effect: Effect::Allow,
            rule_id: Some("allow-read".to_owned()),
        }
    );
}

#[test]
fn no_match_defaults_to_deny() {
    let policy = Policy::new([rule(
        "allow-read",
        "alice",
        "read",
        "report",
        10,
        Effect::Allow,
    )]);

    assert_eq!(
        policy.decide("acme", "bob", "write", "ledger"),
        Decision {
            effect: Effect::Deny,
            rule_id: None,
        }
    );
}

#[test]
fn priority_selects_between_equally_shaped_rules() {
    let policy = Policy::new([
        rule("low", "*", "read", "*", 1, Effect::Deny),
        rule("high", "*", "read", "*", 2, Effect::Allow),
    ]);

    assert_eq!(
        policy.decide("acme", "alice", "read", "report").rule_id,
        Some("high".to_owned())
    );
}

#[test]
fn disabled_rules_are_ignored() {
    let policy = Policy::new([
        rule("enabled", "alice", "read", "report", 1, Effect::Allow),
        rule("disabled", "alice", "read", "report", 100, Effect::Deny).disabled(),
    ]);

    assert_eq!(
        policy.decide("acme", "alice", "read", "report"),
        Decision {
            effect: Effect::Allow,
            rule_id: Some("enabled".to_owned()),
        }
    );
}
