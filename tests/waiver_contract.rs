use std::collections::BTreeSet;
use std::fs;

use bhcp::schema::{parse_diagnostic, validate_root};
use bhcp::value::Value;

#[derive(Clone, Debug, Eq, PartialEq)]
struct Scope {
    goals: Option<BTreeSet<&'static str>>,
    resources: Option<BTreeSet<&'static str>>,
    operations: Option<BTreeSet<&'static str>>,
}

impl Scope {
    fn new(
        goals: Option<&[&'static str]>,
        resources: Option<&[&'static str]>,
        operations: Option<&[&'static str]>,
    ) -> Self {
        Self {
            goals: goals.map(|values| values.iter().copied().collect()),
            resources: resources.map(|values| values.iter().copied().collect()),
            operations: operations.map(|values| values.iter().copied().collect()),
        }
    }

    fn no_broader_than(&self, authority: &Self) -> bool {
        fn axis(
            requested: &Option<BTreeSet<&'static str>>,
            authority: &Option<BTreeSet<&'static str>>,
        ) -> bool {
            match (requested, authority) {
                (_, None) => true,
                (None, Some(_)) => false,
                (Some(requested), Some(authority)) => requested.is_subset(authority),
            }
        }
        axis(&self.goals, &authority.goals)
            && axis(&self.resources, &authority.resources)
            && axis(&self.operations, &authority.operations)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Weakening {
    RemoveRequirement(&'static str),
    AllowProhibitedEffect(&'static str),
    BroadenCapability {
        effect: &'static str,
        from: Scope,
        to: Scope,
    },
    LoosenLimit {
        dimension: &'static str,
        unit: &'static str,
        from: u64,
        to: u64,
    },
    WeakenTypeMode {
        from: &'static str,
        to: &'static str,
    },
}

#[derive(Clone)]
struct Rule {
    policy: &'static str,
    id: &'static str,
    scope: Scope,
    weakening: Weakening,
    waivable: bool,
    issuers: BTreeSet<&'static str>,
}

#[derive(Clone)]
struct Waiver {
    policy: &'static str,
    rule: &'static str,
    scope: Scope,
    weakening: Weakening,
    issuer: &'static str,
    authority_chain: Vec<(&'static str, &'static str)>,
    issued_at: i64,
    not_before: i64,
    expires_at: i64,
    has_authorization: bool,
    has_audit_reference: bool,
}

fn authorize(rule: &Rule, waiver: &Waiver, decision_time: i64) -> Result<(), &'static str> {
    if (waiver.policy, waiver.rule) != (rule.policy, rule.id) {
        return Err("WAIVER_RULE_MISMATCH");
    }
    if waiver.weakening != rule.weakening {
        return Err("WAIVER_CHANGE_MISMATCH");
    }
    if !waiver.scope.no_broader_than(&rule.scope) {
        return Err("WAIVER_SCOPE_OVERBROAD");
    }
    if !rule.waivable {
        return Err("WAIVER_NONWAIVABLE");
    }
    if waiver.issued_at > waiver.not_before || waiver.not_before >= waiver.expires_at {
        return Err("WAIVER_INTERVAL_INVALID");
    }
    if decision_time < waiver.not_before || decision_time >= waiver.expires_at {
        return Err("WAIVER_INACTIVE");
    }
    if !waiver.has_authorization {
        return Err("WAIVER_AUTHORIZATION_MISSING");
    }
    if !waiver.has_audit_reference {
        return Err("WAIVER_AUDIT_MISSING");
    }

    if waiver.authority_chain.is_empty() {
        return rule
            .issuers
            .contains(waiver.issuer)
            .then_some(())
            .ok_or("WAIVER_ISSUER_UNAUTHORIZED");
    }
    if !rule.issuers.contains(waiver.authority_chain[0].0) {
        return Err("WAIVER_CHAIN_UNAUTHORIZED_ROOT");
    }
    let mut seen = BTreeSet::from([waiver.authority_chain[0].0]);
    let mut expected_delegator = waiver.authority_chain[0].0;
    for (delegator, delegate) in &waiver.authority_chain {
        if *delegator != expected_delegator {
            return Err("WAIVER_CHAIN_DISCONNECTED");
        }
        if !seen.insert(*delegate) {
            return Err("WAIVER_CHAIN_CYCLE");
        }
        expected_delegator = delegate;
    }
    (waiver.authority_chain.last().unwrap().1 == waiver.issuer)
        .then_some(())
        .ok_or("WAIVER_CHAIN_WRONG_ISSUER")
}

fn baseline() -> (Rule, Waiver) {
    let restricted = Scope::new(
        Some(&["example/goal.deploy@0"]),
        Some(&["example/resource.prod@0"]),
        Some(&["write"]),
    );
    let rule = Rule {
        policy: "example/policy.org@0",
        id: "memory-limit",
        scope: restricted.clone(),
        weakening: Weakening::LoosenLimit {
            dimension: "example/dimension.memory@0",
            unit: "example/unit.byte@0",
            from: 5,
            to: 6,
        },
        waivable: true,
        issuers: ["security-team"].into_iter().collect(),
    };
    let waiver = Waiver {
        policy: rule.policy,
        rule: rule.id,
        scope: restricted,
        weakening: rule.weakening.clone(),
        issuer: "security-team",
        authority_chain: vec![],
        issued_at: 5,
        not_before: 10,
        expires_at: 20,
        has_authorization: true,
        has_audit_reference: true,
    };
    (rule, waiver)
}

#[test]
fn exact_scope_time_authority_and_nonwaivable_vectors_fail_closed() {
    let (rule, waiver) = baseline();
    assert_eq!(authorize(&rule, &waiver, 10), Ok(()));
    assert_eq!(authorize(&rule, &waiver, 19), Ok(()));

    let mut delegated = waiver.clone();
    delegated.issuer = "release-manager";
    delegated.authority_chain = vec![
        ("security-team", "security-lead"),
        ("security-lead", "release-manager"),
    ];
    assert_eq!(authorize(&rule, &delegated, 10), Ok(()));

    let mut vectors: Vec<(&str, Rule, Waiver, i64, &str)> = Vec::new();
    let mut changed = waiver.clone();
    changed.rule = "other-rule";
    vectors.push((
        "exact rule",
        rule.clone(),
        changed,
        10,
        "WAIVER_RULE_MISMATCH",
    ));

    let mut changed = waiver.clone();
    changed.weakening = Weakening::LoosenLimit {
        dimension: "example/dimension.memory@0",
        unit: "example/unit.byte@0",
        from: 5,
        to: 7,
    };
    vectors.push((
        "precise change",
        rule.clone(),
        changed,
        10,
        "WAIVER_CHANGE_MISMATCH",
    ));

    let mut changed = waiver.clone();
    changed.scope = Scope::new(None, None, None);
    vectors.push((
        "scope superset",
        rule.clone(),
        changed,
        10,
        "WAIVER_SCOPE_OVERBROAD",
    ));

    let mut nonwaivable = rule.clone();
    nonwaivable.waivable = false;
    vectors.push((
        "nonwaivable",
        nonwaivable,
        waiver.clone(),
        10,
        "WAIVER_NONWAIVABLE",
    ));

    let time_mutations: [(&str, fn(&mut Waiver), &str); 2] = [
        (
            "premature",
            |waiver: &mut Waiver| waiver.not_before = 4,
            "WAIVER_INTERVAL_INVALID",
        ),
        (
            "empty interval",
            |waiver: &mut Waiver| waiver.expires_at = 10,
            "WAIVER_INTERVAL_INVALID",
        ),
    ];
    for (name, mutate, expected) in time_mutations {
        let mut changed = waiver.clone();
        mutate(&mut changed);
        vectors.push((name, rule.clone(), changed, 10, expected));
    }
    vectors.push((
        "before active",
        rule.clone(),
        waiver.clone(),
        9,
        "WAIVER_INACTIVE",
    ));
    vectors.push((
        "expiry exclusive",
        rule.clone(),
        waiver.clone(),
        20,
        "WAIVER_INACTIVE",
    ));

    let mut changed = waiver.clone();
    changed.has_authorization = false;
    vectors.push((
        "authorization",
        rule.clone(),
        changed,
        10,
        "WAIVER_AUTHORIZATION_MISSING",
    ));
    let mut changed = waiver.clone();
    changed.has_audit_reference = false;
    vectors.push(("audit", rule.clone(), changed, 10, "WAIVER_AUDIT_MISSING"));
    let mut changed = waiver.clone();
    changed.issuer = "unknown";
    vectors.push((
        "issuer",
        rule.clone(),
        changed,
        10,
        "WAIVER_ISSUER_UNAUTHORIZED",
    ));
    let mut changed = delegated.clone();
    changed.authority_chain[1].0 = "unrelated";
    vectors.push((
        "broken chain",
        rule.clone(),
        changed,
        10,
        "WAIVER_CHAIN_DISCONNECTED",
    ));
    let mut changed = delegated;
    changed.authority_chain[1].1 = "security-team";
    vectors.push(("cycle", rule.clone(), changed, 10, "WAIVER_CHAIN_CYCLE"));

    for (name, rule, waiver, decision, expected) in vectors {
        assert_eq!(authorize(&rule, &waiver, decision), Err(expected), "{name}");
    }
}

#[test]
fn every_weakening_category_has_a_closed_typed_shape() {
    let narrow = Scope::new(Some(&["example/goal.a@0"]), None, None);
    let broad = Scope::new(Some(&["example/goal.a@0", "example/goal.b@0"]), None, None);
    let values = [
        Weakening::RemoveRequirement("example/requirement.review@0"),
        Weakening::AllowProhibitedEffect("bhcp-effect/network@0"),
        Weakening::BroadenCapability {
            effect: "bhcp-effect/fs.read@0",
            from: narrow,
            to: broad,
        },
        Weakening::LoosenLimit {
            dimension: "example/dimension.memory@0",
            unit: "example/unit.byte@0",
            from: 5,
            to: 6,
        },
        Weakening::WeakenTypeMode {
            from: "strict",
            to: "infer-strict",
        },
    ];
    assert_eq!(values.len(), 5);
}

#[test]
fn semantics_schema_fixture_and_threat_model_publish_one_boundary() {
    let semantics = fs::read_to_string("SEMANTICS.md").unwrap();
    let schema = fs::read_to_string("schemas/v0/bhcp-v0.cddl").unwrap();
    let threat_model = fs::read_to_string("THREAT_MODEL.md").unwrap();
    for required in [
        "injected decision time",
        "[not_before, expires_at)",
        "MUST NOT read an ambient clock",
        "exact source-rule identity",
        "invalid waiver aborts",
        "post-waiver effective policy",
        "waiver metadata changes artifact identity",
    ] {
        assert!(semantics.contains(required), "SEMANTICS omitted {required}");
    }
    for required in [
        "waiver-target = {",
        "waiver-weakening =",
        "waiver-delegation = {",
        "\"targets\": [1* waiver-target]",
        "\"authority_chain\": [* waiver-delegation]",
        "\"authorization\": [1* authorization]",
    ] {
        assert!(schema.contains(required), "CDDL omitted {required}");
    }
    assert!(!schema.contains("\"scope\": value"));
    assert!(!schema.contains("\"weakening\": value"));

    let fixture =
        parse_diagnostic(&fs::read_to_string("schemas/v0/examples/waiver.diag").unwrap()).unwrap();
    validate_root(&fixture, "waiver").unwrap();
    assert!(matches!(fixture.get("targets"), Some(Value::Array(values)) if !values.is_empty()));
    assert!(
        matches!(fixture.get("authorization"), Some(Value::Array(values)) if !values.is_empty())
    );
    assert!(fixture.get("semantic_id").is_none());

    for threat in [
        "scope amplification",
        "delegation-chain confusion",
        "clock rollback",
        "audit-reference substitution",
        "non-waivable downgrade",
    ] {
        assert!(
            threat_model.contains(threat),
            "threat model omitted {threat}"
        );
    }
}
