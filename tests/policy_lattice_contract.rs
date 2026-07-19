use std::fs;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn policy_rules_have_closed_category_operation_value_shapes() {
    let schema = fs::read_to_string(root().join("schemas/v0/bhcp-v0.cddl")).unwrap();

    for rule in [
        "policy-rule = requirement-policy-rule / evidence-policy-rule",
        "requirement-policy-rule = {",
        "\"category\": \"requirement\"",
        "\"operation\": \"add\"",
        "\"value\": requirement-policy-value",
        "evidence-policy-rule = {",
        "prohibition-policy-rule = {",
        "capability-policy-rule = {",
        "limit-policy-rule = {",
        "type-mode-policy-rule = {",
        "source-policy-document = {",
        "effective-policy-document = {",
        "\"form\": \"effective\"",
        "\"source_layers\": [* policy-source-layer]",
        "\"rule_provenance\": [* policy-rule-provenance]",
        "\"targets\": [1* waiver-target]",
        "\"weakening\": waiver-weakening",
        "waiver-weakening = remove-requirement-waiver / remove-evidence-waiver",
    ] {
        assert!(
            schema.contains(rule),
            "CDDL policy boundary is missing {rule}"
        );
    }

    assert!(
        !schema.contains(
            "\"category\": \"requirement\" / \"limit\" / \"type-mode\" / \"evidence\"
            / \"prohibition\" / \"capability\","
        ),
        "policy categories must not share an unrestricted value slot"
    );
    assert!(
        !schema.contains("\"value\": value,\n  \"waivable\": bool"),
        "policy values must be category-specific"
    );
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Restriction {
    requirements: u8,
    evidence: u8,
    prohibitions: u8,
    capabilities: u8,
    limit: u8,
    mode: u8,
}

impl Restriction {
    fn compose(self, later: Self) -> Self {
        Self {
            requirements: self.requirements | later.requirements,
            evidence: self.evidence | later.evidence,
            prohibitions: self.prohibitions | later.prohibitions,
            capabilities: self.capabilities & later.capabilities,
            limit: self.limit.min(later.limit),
            mode: self.mode.max(later.mode),
        }
    }

    fn at_least_as_restrictive_as(self, earlier: Self) -> bool {
        self.requirements | earlier.requirements == self.requirements
            && self.evidence | earlier.evidence == self.evidence
            && self.prohibitions | earlier.prohibitions == self.prohibitions
            && self.capabilities & earlier.capabilities == self.capabilities
            && self.limit <= earlier.limit
            && self.mode >= earlier.mode
    }
}

fn finite_model() -> Vec<Restriction> {
    let mut values = Vec::new();
    for requirements in 0..=1 {
        for evidence in 0..=1 {
            for prohibitions in 0..=1 {
                for capabilities in 0..=1 {
                    for limit in 0..=2 {
                        for mode in 0..=3 {
                            values.push(Restriction {
                                requirements,
                                evidence,
                                prohibitions,
                                capabilities,
                                limit,
                                mode,
                            });
                        }
                    }
                }
            }
        }
    }
    values
}

#[test]
fn strict_restriction_relation_is_acyclic_in_the_finite_model() {
    let values = finite_model();
    for &left in &values {
        assert!(left.at_least_as_restrictive_as(left));
        for &right in &values {
            if left.at_least_as_restrictive_as(right) && right.at_least_as_restrictive_as(left) {
                assert_eq!(left, right, "antisymmetry excludes strict cycles");
            }
        }
    }

    for &earlier in &values {
        for &middle in &values {
            if !middle.at_least_as_restrictive_as(earlier) {
                continue;
            }
            for &later in &values {
                if later.at_least_as_restrictive_as(middle) {
                    assert!(
                        later.at_least_as_restrictive_as(earlier),
                        "transitivity plus antisymmetry excludes longer strict cycles"
                    );
                }
            }
        }
    }
}

#[test]
fn policy_composition_is_associative_coordinate_by_coordinate() {
    for a in 0_u8..=3 {
        for b in 0_u8..=3 {
            for c in 0_u8..=3 {
                assert_eq!((a | b) | c, a | (b | c));
                assert_eq!((a & b) & c, a & (b & c));
                assert_eq!(a.min(b).min(c), a.min(b.min(c)));
                assert_eq!(a.max(b).max(c), a.max(b.max(c)));
            }
        }
    }

    let values = finite_model();
    for &earlier in &values {
        for &later in &values {
            let composed = earlier.compose(later);
            assert!(composed.at_least_as_restrictive_as(earlier));
            assert!(composed.at_least_as_restrictive_as(later));
        }
    }
}
