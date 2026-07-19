//! Strongly typed v0 source and effective policy documents.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashSet};

use crate::cbor::{decode_deterministic, encode_deterministic};
use crate::diagnostic::{Diagnostic, Result};
use crate::hash::{HashAlgorithm, artifact_hash_with, hash_value};
use crate::model::{HashId, is_symbol};
use crate::value::Value;

const INVALID_POLICY: &str = "BHCP8001";
const CAPABILITY_WIDENING: &str = "BHCP8101";
const LIMIT_LOOSENING: &str = "BHCP8102";
const TYPE_MODE_WEAKENING: &str = "BHCP8103";
const REQUIREMENT_REMOVAL: &str = "BHCP8104";
const EVIDENCE_REMOVAL: &str = "BHCP8105";
const ALLOW_OVER_DENY: &str = "BHCP8106";
const INCOMPATIBLE_LIMIT_UNITS: &str = "BHCP8107";
const INVALID_COMPOSITION_TOPOLOGY: &str = "BHCP8110";
const INVALID_WAIVER: &str = "BHCP8301";
const WAIVER_TARGET_MISMATCH: &str = "BHCP8302";
const WAIVER_CHANGE_MISMATCH: &str = "BHCP8303";
const WAIVER_INACTIVE: &str = "BHCP8304";
const WAIVER_UNAUTHORIZED: &str = "BHCP8305";
const WAIVER_NONWAIVABLE: &str = "BHCP8306";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyHeader {
    pub features: Vec<String>,
    pub semantic_id: Option<HashId>,
    pub artifact_id: Option<HashId>,
    pub provenance: Option<Value>,
    pub authorization: Option<Vec<Value>>,
}

impl PolicyHeader {
    fn from_entries(entries: &[(String, Value)]) -> Result<Self> {
        require_exact_text(entries, "version", "bhcp/v0", "policy document")?;
        let features = parse_symbol_array(
            required(entries, "features", "policy document")?,
            "policy features",
            true,
        )?;
        let semantic_id = optional(entries, "semantic_id")
            .map(parse_hash_id)
            .transpose()?;
        let artifact_id = optional(entries, "artifact_id")
            .map(parse_hash_id)
            .transpose()?;
        let provenance = optional(entries, "provenance").cloned();
        if provenance
            .as_ref()
            .is_some_and(|value| !matches!(value, Value::Map(_)))
        {
            return Err(invalid("policy header provenance must be a map"));
        }
        let authorization = optional(entries, "authorization")
            .map(|value| {
                let values = array_values(value, "policy header authorization")?;
                if values.iter().any(|value| !matches!(value, Value::Map(_))) {
                    return Err(invalid("policy authorization entries must be maps"));
                }
                Ok(values.to_vec())
            })
            .transpose()?;
        Ok(Self {
            features,
            semantic_id,
            artifact_id,
            provenance,
            authorization,
        })
    }

    fn entries(&self, include_artifact_id: bool) -> Vec<(String, Value)> {
        let mut entries = vec![
            ("version".to_owned(), text("bhcp/v0")),
            (
                "features".to_owned(),
                Value::Array(self.features.iter().cloned().map(Value::Text).collect()),
            ),
        ];
        if let Some(semantic_id) = &self.semantic_id {
            entries.push(("semantic_id".to_owned(), semantic_id.to_value()));
        }
        if include_artifact_id && let Some(artifact_id) = &self.artifact_id {
            entries.push(("artifact_id".to_owned(), artifact_id.to_value()));
        }
        if let Some(provenance) = &self.provenance {
            entries.push(("provenance".to_owned(), provenance.clone()));
        }
        if let Some(authorization) = &self.authorization {
            entries.push((
                "authorization".to_owned(),
                Value::Array(authorization.clone()),
            ));
        }
        entries
    }

    fn validate(&self) -> Result<()> {
        validate_normalized_symbols(&self.features, "policy features", true)?;
        if let Some(semantic_id) = &self.semantic_id {
            semantic_id.validate().map_err(policy_error)?;
        }
        if let Some(artifact_id) = &self.artifact_id {
            artifact_id.validate().map_err(policy_error)?;
        }
        if self
            .provenance
            .as_ref()
            .is_some_and(|value| !matches!(value, Value::Map(_)))
        {
            return Err(invalid("policy header provenance must be a map"));
        }
        if self
            .authorization
            .as_ref()
            .is_some_and(|values| values.iter().any(|value| !matches!(value, Value::Map(_))))
        {
            return Err(invalid("policy authorization entries must be maps"));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum PolicyLayer {
    Organization,
    Team,
    Repository,
    User,
}

impl PolicyLayer {
    fn parse(value: &Value) -> Result<Self> {
        match text_value(value, "policy layer")? {
            "organization" => Ok(Self::Organization),
            "team" => Ok(Self::Team),
            "repository" => Ok(Self::Repository),
            "user" => Ok(Self::User),
            _ => Err(invalid(
                "policy layer must be organization, team, repository, or user",
            )),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Organization => "organization",
            Self::Team => "team",
            Self::Repository => "repository",
            Self::User => "user",
        }
    }
}

/// A weakening that cannot be expressed by the closed additive source-policy grammar.
///
/// The waiver boundary uses this typed representation instead of silently editing an
/// earlier rule. Until a valid waiver is applied, every attempt is rejected.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PolicyWeakeningAttempt {
    RemoveRequirement {
        layer: PolicyLayer,
        policy: String,
        rule: String,
        requirement: String,
        earlier: SourceRuleIdentity,
        earlier_layer: PolicyLayer,
    },
    RemoveEvidence {
        layer: PolicyLayer,
        policy: String,
        rule: String,
        obligation: String,
        earlier: SourceRuleIdentity,
        earlier_layer: PolicyLayer,
    },
    AllowDeniedEffect {
        layer: PolicyLayer,
        policy: String,
        rule: String,
        effect: String,
        earlier: SourceRuleIdentity,
        earlier_layer: PolicyLayer,
    },
}

/// Rejects an explicit weakening until the waiver boundary authorizes it.
pub fn reject_policy_weakening(attempt: PolicyWeakeningAttempt) -> Result<()> {
    let (code, layer, policy, rule, action, earlier, earlier_layer) = match attempt {
        PolicyWeakeningAttempt::RemoveRequirement {
            layer,
            policy,
            rule,
            requirement,
            earlier,
            earlier_layer,
        } => (
            REQUIREMENT_REMOVAL,
            layer,
            policy,
            rule,
            format!("removes requirement {requirement}"),
            earlier,
            earlier_layer,
        ),
        PolicyWeakeningAttempt::RemoveEvidence {
            layer,
            policy,
            rule,
            obligation,
            earlier,
            earlier_layer,
        } => (
            EVIDENCE_REMOVAL,
            layer,
            policy,
            rule,
            format!("removes evidence {obligation}"),
            earlier,
            earlier_layer,
        ),
        PolicyWeakeningAttempt::AllowDeniedEffect {
            layer,
            policy,
            rule,
            effect,
            earlier,
            earlier_layer,
        } => (
            ALLOW_OVER_DENY,
            layer,
            policy,
            rule,
            format!("allows denied effect {effect}"),
            earlier,
            earlier_layer,
        ),
    };
    Err(weakening_diagnostic(
        code,
        layer,
        &policy,
        &rule,
        &action,
        earlier_layer,
        &earlier,
    ))
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum PolicyCategory {
    Requirement,
    Evidence,
    Prohibition,
    Capability,
    Limit,
    TypeMode,
}

impl PolicyCategory {
    fn parse(value: &Value) -> Result<Self> {
        match text_value(value, "policy category")? {
            "requirement" => Ok(Self::Requirement),
            "evidence" => Ok(Self::Evidence),
            "prohibition" => Ok(Self::Prohibition),
            "capability" => Ok(Self::Capability),
            "limit" => Ok(Self::Limit),
            "type-mode" => Ok(Self::TypeMode),
            category => Err(invalid(format!("unknown policy category {category:?}"))),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Requirement => "requirement",
            Self::Evidence => "evidence",
            Self::Prohibition => "prohibition",
            Self::Capability => "capability",
            Self::Limit => "limit",
            Self::TypeMode => "type-mode",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum TypeMode {
    Dynamic,
    Gradual,
    InferStrict,
    Strict,
}

impl TypeMode {
    fn parse(value: &Value) -> Result<Self> {
        match text_value(value, "type-mode policy value")? {
            "dynamic" => Ok(Self::Dynamic),
            "gradual" => Ok(Self::Gradual),
            "infer-strict" => Ok(Self::InferStrict),
            "strict" => Ok(Self::Strict),
            _ => Err(invalid(
                "type-mode policy value must be dynamic, gradual, infer-strict, or strict",
            )),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dynamic => "dynamic",
            Self::Gradual => "gradual",
            Self::InferStrict => "infer-strict",
            Self::Strict => "strict",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyScope {
    pub goals: Option<Vec<String>>,
    pub resources: Option<Vec<String>>,
    pub operations: Option<Vec<String>>,
}

impl PolicyScope {
    fn from_value(value: &Value) -> Result<Self> {
        let entries = map_entries(value, "policy scope")?;
        ensure_fields(
            entries,
            &["goals", "resources", "operations"],
            "policy scope",
        )?;
        Ok(Self {
            goals: optional(entries, "goals")
                .map(|value| parse_symbol_array(value, "policy scope goals", true))
                .transpose()?,
            resources: optional(entries, "resources")
                .map(|value| parse_symbol_array(value, "policy scope resources", true))
                .transpose()?,
            operations: optional(entries, "operations")
                .map(|value| parse_symbol_array(value, "policy scope operations", true))
                .transpose()?,
        })
    }

    fn to_value(&self) -> Value {
        let mut entries = Vec::new();
        for (key, values) in [
            ("goals", &self.goals),
            ("resources", &self.resources),
            ("operations", &self.operations),
        ] {
            if let Some(values) = values {
                entries.push((
                    key.to_owned(),
                    Value::Array(values.iter().cloned().map(Value::Text).collect()),
                ));
            }
        }
        Value::owned_map(entries)
    }

    fn validate(&self) -> Result<()> {
        for (label, values) in [
            ("policy scope goals", &self.goals),
            ("policy scope resources", &self.resources),
            ("policy scope operations", &self.operations),
        ] {
            if let Some(values) = values {
                validate_normalized_symbols(values, label, true)?;
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExactNumber {
    Integer(i64),
    Rational { numerator: i64, denominator: i64 },
    Decimal { coefficient: i64, exponent: i64 },
}

impl ExactNumber {
    fn parse_non_negative(value: &Value) -> Result<Self> {
        let Value::Array(parts) = value else {
            return Err(invalid("limit maximum must be a non-negative exact number"));
        };
        let parsed = match parts.as_slice() {
            [Value::Text(kind), Value::Integer(integer)] if kind == "integer" => {
                Self::Integer(*integer)
            }
            [
                Value::Text(kind),
                Value::Integer(numerator),
                Value::Integer(denominator),
            ] if kind == "rational" && *denominator > 0 => Self::Rational {
                numerator: *numerator,
                denominator: *denominator,
            },
            [
                Value::Text(kind),
                Value::Integer(coefficient),
                Value::Integer(exponent),
            ] if kind == "decimal" => Self::Decimal {
                coefficient: *coefficient,
                exponent: *exponent,
            },
            _ => return Err(invalid("limit maximum must be a non-negative exact number")),
        };
        if match parsed {
            Self::Integer(value) => value < 0,
            Self::Rational { numerator, .. } => numerator < 0,
            Self::Decimal { coefficient, .. } => coefficient < 0,
        } {
            return Err(invalid("limit maximum must be a non-negative exact number"));
        }
        Ok(parsed)
    }

    pub fn to_value(&self) -> Value {
        match self {
            Self::Integer(value) => Value::Array(vec![text("integer"), Value::Integer(*value)]),
            Self::Rational {
                numerator,
                denominator,
            } => Value::Array(vec![
                text("rational"),
                Value::Integer(*numerator),
                Value::Integer(*denominator),
            ]),
            Self::Decimal {
                coefficient,
                exponent,
            } => Value::Array(vec![
                text("decimal"),
                Value::Integer(*coefficient),
                Value::Integer(*exponent),
            ]),
        }
    }

    pub fn compare(&self, other: &Self) -> Ordering {
        exact_number_cmp(self, other)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequirementPolicyValue {
    pub requirement: String,
    pub scope: Option<PolicyScope>,
    pub parameters: Option<Value>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidencePolicyValue {
    pub obligation: String,
    pub classes: Vec<String>,
    pub minimum: u64,
    pub scope: Option<PolicyScope>,
    pub parameters: Option<Value>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityPolicyValue {
    pub effect: String,
    pub scope: Option<PolicyScope>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LimitPolicyValue {
    pub dimension: String,
    pub unit: String,
    pub maximum: ExactNumber,
    pub scope: Option<PolicyScope>,
}

impl RequirementPolicyValue {
    fn from_value(value: &Value) -> Result<Self> {
        let entries = map_entries(value, "requirement policy value")?;
        ensure_fields(
            entries,
            &["requirement", "scope", "parameters"],
            "requirement policy value",
        )?;
        let requirement = required_symbol(entries, "requirement", "requirement policy value")?;
        let scope = optional(entries, "scope")
            .map(PolicyScope::from_value)
            .transpose()?;
        Ok(Self {
            requirement,
            scope,
            parameters: optional(entries, "parameters").cloned(),
        })
    }

    fn to_value(&self) -> Value {
        let mut entries = vec![("requirement".to_owned(), text(&self.requirement))];
        push_scope_and_parameters(&mut entries, &self.scope, &self.parameters);
        Value::owned_map(entries)
    }

    fn validate(&self) -> Result<()> {
        validate_symbol(&self.requirement, "requirement policy symbol")?;
        if let Some(scope) = &self.scope {
            scope.validate()?;
        }
        Ok(())
    }
}

impl EvidencePolicyValue {
    fn from_value(value: &Value) -> Result<Self> {
        let entries = map_entries(value, "evidence policy value")?;
        ensure_fields(
            entries,
            &["obligation", "classes", "minimum", "scope", "parameters"],
            "evidence policy value",
        )?;
        let obligation = required_symbol(entries, "obligation", "evidence policy value")?;
        let classes =
            parse_evidence_classes(required(entries, "classes", "evidence policy value")?)?;
        let minimum = positive_u64(
            required(entries, "minimum", "evidence policy value")?,
            "evidence minimum must be a positive integer",
        )?;
        let scope = optional(entries, "scope")
            .map(PolicyScope::from_value)
            .transpose()?;
        Ok(Self {
            obligation,
            classes,
            minimum,
            scope,
            parameters: optional(entries, "parameters").cloned(),
        })
    }

    fn to_value(&self) -> Value {
        let mut entries = vec![
            ("obligation".to_owned(), text(&self.obligation)),
            (
                "classes".to_owned(),
                Value::Array(self.classes.iter().cloned().map(Value::Text).collect()),
            ),
            ("minimum".to_owned(), Value::Integer(self.minimum as i64)),
        ];
        push_scope_and_parameters(&mut entries, &self.scope, &self.parameters);
        Value::owned_map(entries)
    }

    fn validate(&self) -> Result<()> {
        validate_symbol(&self.obligation, "evidence obligation")?;
        if self.minimum == 0 || self.minimum > i64::MAX as u64 {
            return Err(invalid("evidence minimum must be a positive integer"));
        }
        validate_normalized_evidence_classes(&self.classes)?;
        if let Some(scope) = &self.scope {
            scope.validate()?;
        }
        Ok(())
    }
}

impl CapabilityPolicyValue {
    fn from_value(value: &Value) -> Result<Self> {
        let entries = map_entries(value, "capability policy value")?;
        ensure_fields(entries, &["effect", "scope"], "capability policy value")?;
        Ok(Self {
            effect: required_symbol(entries, "effect", "capability policy value")?,
            scope: optional(entries, "scope")
                .map(PolicyScope::from_value)
                .transpose()?,
        })
    }

    fn to_value(&self) -> Value {
        let mut entries = vec![("effect".to_owned(), text(&self.effect))];
        if let Some(scope) = &self.scope {
            entries.push(("scope".to_owned(), scope.to_value()));
        }
        Value::owned_map(entries)
    }

    fn validate(&self) -> Result<()> {
        validate_symbol(&self.effect, "capability effect")?;
        if let Some(scope) = &self.scope {
            scope.validate()?;
        }
        Ok(())
    }
}

impl LimitPolicyValue {
    fn from_value(value: &Value) -> Result<Self> {
        let entries = map_entries(value, "limit policy value")?;
        ensure_fields(
            entries,
            &["dimension", "unit", "maximum", "scope"],
            "limit policy value",
        )?;
        Ok(Self {
            dimension: required_symbol(entries, "dimension", "limit policy value")?,
            unit: required_symbol(entries, "unit", "limit policy value")?,
            maximum: ExactNumber::parse_non_negative(required(
                entries,
                "maximum",
                "limit policy value",
            )?)?,
            scope: optional(entries, "scope")
                .map(PolicyScope::from_value)
                .transpose()?,
        })
    }

    fn to_value(&self) -> Value {
        let mut entries = vec![
            ("dimension".to_owned(), text(&self.dimension)),
            ("unit".to_owned(), text(&self.unit)),
            ("maximum".to_owned(), self.maximum.to_value()),
        ];
        if let Some(scope) = &self.scope {
            entries.push(("scope".to_owned(), scope.to_value()));
        }
        Value::owned_map(entries)
    }

    fn validate(&self) -> Result<()> {
        validate_symbol(&self.dimension, "limit dimension")?;
        validate_symbol(&self.unit, "limit unit")?;
        if match self.maximum {
            ExactNumber::Integer(value) => value < 0,
            ExactNumber::Rational {
                numerator,
                denominator,
            } => numerator < 0 || denominator <= 0,
            ExactNumber::Decimal { coefficient, .. } => coefficient < 0,
        } {
            return Err(invalid("limit maximum must be a non-negative exact number"));
        }
        if let Some(scope) = &self.scope {
            scope.validate()?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyRuleCommon {
    pub id: String,
    pub waivable: bool,
    pub authorized_issuers: Vec<String>,
}

impl PolicyRuleCommon {
    fn from_entries(entries: &[(String, Value)]) -> Result<Self> {
        let id = required_text(entries, "id", "policy rule")?;
        let waivable = required_bool(entries, "waivable", "policy rule")?;
        let issuers_present = optional(entries, "authorized_issuers");
        let authorized_issuers = issuers_present
            .map(|value| parse_text_array(value, "authorized issuers", false))
            .transpose()?
            .unwrap_or_default();
        if waivable && authorized_issuers.is_empty() {
            return Err(invalid("waivable policy rule requires authorized issuers"));
        }
        if !waivable && issuers_present.is_some() {
            return Err(invalid(
                "non-waivable policy rule must not authorize issuers",
            ));
        }
        let common = Self {
            id,
            waivable,
            authorized_issuers,
        };
        common.validate()?;
        Ok(common)
    }

    fn entries(&self) -> Vec<(String, Value)> {
        let mut entries = vec![
            ("id".to_owned(), text(&self.id)),
            ("waivable".to_owned(), Value::Bool(self.waivable)),
        ];
        if self.waivable {
            entries.push((
                "authorized_issuers".to_owned(),
                Value::Array(
                    self.authorized_issuers
                        .iter()
                        .cloned()
                        .map(Value::Text)
                        .collect(),
                ),
            ));
        }
        entries
    }

    fn validate(&self) -> Result<()> {
        if self.id.is_empty() || self.id.len() > 128 {
            return Err(invalid("policy rule ID must be a non-empty ref-id"));
        }
        if self.waivable {
            validate_normalized_text(&self.authorized_issuers, "authorized issuers", false)?;
        } else if !self.authorized_issuers.is_empty() {
            return Err(invalid(
                "non-waivable policy rule must not authorize issuers",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PolicyRule {
    Requirement {
        common: PolicyRuleCommon,
        value: RequirementPolicyValue,
    },
    Evidence {
        common: PolicyRuleCommon,
        value: EvidencePolicyValue,
    },
    Prohibition {
        common: PolicyRuleCommon,
        value: CapabilityPolicyValue,
    },
    Capability {
        common: PolicyRuleCommon,
        value: CapabilityPolicyValue,
    },
    Limit {
        common: PolicyRuleCommon,
        value: LimitPolicyValue,
    },
    TypeMode {
        common: PolicyRuleCommon,
        value: TypeMode,
    },
}

impl PolicyRule {
    pub(crate) fn from_value(value: &Value) -> Result<Self> {
        let entries = map_entries(value, "policy rule")?;
        let category = PolicyCategory::parse(required(entries, "category", "policy rule")?)?;
        let context = format!("{} policy rule", category.as_str());
        ensure_fields(
            entries,
            &[
                "id",
                "category",
                "operation",
                "value",
                "waivable",
                "authorized_issuers",
            ],
            &context,
        )?;
        let operation = required_text(entries, "operation", &context)?;
        let expected_operation = match category {
            PolicyCategory::Requirement | PolicyCategory::Evidence => "add",
            PolicyCategory::Prohibition => "deny",
            PolicyCategory::Capability => "narrow",
            PolicyCategory::Limit => "tighten",
            PolicyCategory::TypeMode => "strengthen",
        };
        if operation != expected_operation {
            return Err(invalid(format!(
                "policy category {:?} requires operation {:?}",
                category.as_str(),
                expected_operation
            )));
        }
        let common = PolicyRuleCommon::from_entries(entries)?;
        let rule_value = required(entries, "value", &context)?;
        let rule = match category {
            PolicyCategory::Requirement => Self::Requirement {
                common,
                value: RequirementPolicyValue::from_value(rule_value)?,
            },
            PolicyCategory::Evidence => Self::Evidence {
                common,
                value: EvidencePolicyValue::from_value(rule_value)?,
            },
            PolicyCategory::Prohibition => Self::Prohibition {
                common,
                value: CapabilityPolicyValue::from_value(rule_value)?,
            },
            PolicyCategory::Capability => Self::Capability {
                common,
                value: CapabilityPolicyValue::from_value(rule_value)?,
            },
            PolicyCategory::Limit => Self::Limit {
                common,
                value: LimitPolicyValue::from_value(rule_value)?,
            },
            PolicyCategory::TypeMode => Self::TypeMode {
                common,
                value: TypeMode::parse(rule_value)?,
            },
        };
        rule.validate()?;
        Ok(rule)
    }

    fn category(&self) -> PolicyCategory {
        match self {
            Self::Requirement { .. } => PolicyCategory::Requirement,
            Self::Evidence { .. } => PolicyCategory::Evidence,
            Self::Prohibition { .. } => PolicyCategory::Prohibition,
            Self::Capability { .. } => PolicyCategory::Capability,
            Self::Limit { .. } => PolicyCategory::Limit,
            Self::TypeMode { .. } => PolicyCategory::TypeMode,
        }
    }

    fn operation(&self) -> &'static str {
        match self {
            Self::Requirement { .. } | Self::Evidence { .. } => "add",
            Self::Prohibition { .. } => "deny",
            Self::Capability { .. } => "narrow",
            Self::Limit { .. } => "tighten",
            Self::TypeMode { .. } => "strengthen",
        }
    }

    pub fn id(&self) -> &str {
        self.common().id.as_str()
    }

    fn common(&self) -> &PolicyRuleCommon {
        match self {
            Self::Requirement { common, .. }
            | Self::Evidence { common, .. }
            | Self::Prohibition { common, .. }
            | Self::Capability { common, .. }
            | Self::Limit { common, .. }
            | Self::TypeMode { common, .. } => common,
        }
    }

    fn to_value(&self) -> Value {
        let mut entries = self.common().entries();
        entries.extend([
            ("category".to_owned(), text(self.category().as_str())),
            ("operation".to_owned(), text(self.operation())),
            (
                "value".to_owned(),
                match self {
                    Self::Requirement { value, .. } => value.to_value(),
                    Self::Evidence { value, .. } => value.to_value(),
                    Self::Prohibition { value, .. } | Self::Capability { value, .. } => {
                        value.to_value()
                    }
                    Self::Limit { value, .. } => value.to_value(),
                    Self::TypeMode { value, .. } => text(value.as_str()),
                },
            ),
        ]);
        Value::owned_map(entries)
    }

    fn validate(&self) -> Result<()> {
        self.common().validate()?;
        match self {
            Self::Requirement { value, .. } => value.validate(),
            Self::Evidence { value, .. } => value.validate(),
            Self::Prohibition { value, .. } | Self::Capability { value, .. } => value.validate(),
            Self::Limit { value, .. } => value.validate(),
            Self::TypeMode { .. } => Ok(()),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourcePolicyDocument {
    pub header: PolicyHeader,
    pub symbol: String,
    pub layer: PolicyLayer,
    pub extends: Option<String>,
    pub rules: Vec<PolicyRule>,
}

impl SourcePolicyDocument {
    fn from_entries(entries: &[(String, Value)]) -> Result<Self> {
        ensure_fields(
            entries,
            &[
                "version",
                "features",
                "semantic_id",
                "artifact_id",
                "provenance",
                "authorization",
                "kind",
                "form",
                "symbol",
                "layer",
                "extends",
                "rules",
            ],
            "source policy document",
        )?;
        require_exact_text(entries, "kind", "policy", "source policy document")?;
        require_exact_text(entries, "form", "source", "source policy document")?;
        let header = PolicyHeader::from_entries(entries)?;
        let symbol = required_symbol(entries, "symbol", "source policy document")?;
        let layer = PolicyLayer::parse(required(entries, "layer", "source policy document")?)?;
        let extends = optional(entries, "extends")
            .map(|value| symbol_value(value, "source policy extends"))
            .transpose()?;
        let rules = array_values(
            required(entries, "rules", "source policy document")?,
            "source policy rules",
        )?
        .iter()
        .map(PolicyRule::from_value)
        .collect::<Result<Vec<_>>>()?;
        let document = Self {
            header,
            symbol,
            layer,
            extends,
            rules,
        };
        document.validate()?;
        Ok(document)
    }

    fn to_value(&self, include_artifact_id: bool) -> Value {
        let mut entries = self.header.entries(include_artifact_id);
        entries.extend([
            ("kind".to_owned(), text("policy")),
            ("form".to_owned(), text("source")),
            ("symbol".to_owned(), text(&self.symbol)),
            ("layer".to_owned(), text(self.layer.as_str())),
        ]);
        if let Some(extends) = &self.extends {
            entries.push(("extends".to_owned(), text(extends)));
        }
        entries.push((
            "rules".to_owned(),
            Value::Array(self.rules.iter().map(PolicyRule::to_value).collect()),
        ));
        Value::owned_map(entries)
    }

    fn validate(&self) -> Result<()> {
        self.header.validate()?;
        validate_symbol(&self.symbol, "source policy symbol")?;
        if let Some(extends) = &self.extends {
            validate_symbol(extends, "source policy extends")?;
            if extends == &self.symbol {
                return Err(invalid("source policy must not extend itself"));
            }
        }
        for rule in &self.rules {
            rule.validate()?;
        }
        if !self
            .rules
            .windows(2)
            .all(|pair| pair[0].id() < pair[1].id())
        {
            return Err(invalid(
                "source policy rules must be sorted by unique rule ID",
            ));
        }
        validate_artifact_id(
            &self.header,
            &self.to_value(false),
            "source policy artifact_id does not match document",
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectiveRule<T> {
    pub waivable: bool,
    pub authorized_issuers: Vec<String>,
    pub value: T,
}

impl<T> EffectiveRule<T> {
    fn common_from_entries(
        entries: &[(String, Value)],
        context: &str,
    ) -> Result<(bool, Vec<String>)> {
        let waivable = required_bool(entries, "waivable", context)?;
        let issuers_present = optional(entries, "authorized_issuers");
        let issuers = issuers_present
            .map(|value| parse_text_array(value, "effective authorized issuers", false))
            .transpose()?
            .unwrap_or_default();
        if waivable && issuers.is_empty() {
            return Err(invalid(
                "waivable effective policy rule requires authorized issuers",
            ));
        }
        if !waivable && issuers_present.is_some() {
            return Err(invalid(
                "non-waivable effective policy rule must not authorize issuers",
            ));
        }
        Ok((waivable, issuers))
    }

    fn common_entries(&self) -> Vec<(String, Value)> {
        let mut entries = vec![("waivable".to_owned(), Value::Bool(self.waivable))];
        if self.waivable {
            entries.push((
                "authorized_issuers".to_owned(),
                Value::Array(
                    self.authorized_issuers
                        .iter()
                        .cloned()
                        .map(Value::Text)
                        .collect(),
                ),
            ));
        }
        entries
    }

    fn validate_common(&self) -> Result<()> {
        if self.waivable {
            validate_normalized_text(
                &self.authorized_issuers,
                "effective authorized issuers",
                false,
            )
        } else if self.authorized_issuers.is_empty() {
            Ok(())
        } else {
            Err(invalid(
                "non-waivable effective policy rule must not authorize issuers",
            ))
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectivePolicy {
    pub requirements: Vec<EffectiveRule<RequirementPolicyValue>>,
    pub evidence: Vec<EffectiveRule<EvidencePolicyValue>>,
    pub prohibitions: Vec<EffectiveRule<CapabilityPolicyValue>>,
    pub capabilities: Vec<EffectiveRule<CapabilityPolicyValue>>,
    pub limits: Vec<EffectiveRule<LimitPolicyValue>>,
    pub type_mode: EffectiveRule<TypeMode>,
}

impl EffectivePolicy {
    fn from_value(value: &Value) -> Result<Self> {
        let entries = map_entries(value, "effective policy")?;
        ensure_fields(
            entries,
            &[
                "requirements",
                "evidence",
                "prohibitions",
                "capabilities",
                "limits",
                "type_mode",
            ],
            "effective policy",
        )?;
        let policy = Self {
            requirements: parse_effective_array(
                required(entries, "requirements", "effective policy")?,
                "effective requirement rule",
                RequirementPolicyValue::from_value,
            )?,
            evidence: parse_effective_array(
                required(entries, "evidence", "effective policy")?,
                "effective evidence rule",
                EvidencePolicyValue::from_value,
            )?,
            prohibitions: parse_effective_array(
                required(entries, "prohibitions", "effective policy")?,
                "effective prohibition rule",
                CapabilityPolicyValue::from_value,
            )?,
            capabilities: parse_effective_array(
                required(entries, "capabilities", "effective policy")?,
                "effective capability rule",
                CapabilityPolicyValue::from_value,
            )?,
            limits: parse_effective_array(
                required(entries, "limits", "effective policy")?,
                "effective limit rule",
                LimitPolicyValue::from_value,
            )?,
            type_mode: parse_effective_rule(
                required(entries, "type_mode", "effective policy")?,
                "effective type-mode rule",
                TypeMode::parse,
            )?,
        };
        policy.validate()?;
        Ok(policy)
    }

    pub fn to_value(&self) -> Value {
        Value::map([
            (
                "requirements",
                Value::Array(
                    self.requirements
                        .iter()
                        .map(|rule| effective_rule_value(rule, RequirementPolicyValue::to_value))
                        .collect(),
                ),
            ),
            (
                "evidence",
                Value::Array(
                    self.evidence
                        .iter()
                        .map(|rule| effective_rule_value(rule, EvidencePolicyValue::to_value))
                        .collect(),
                ),
            ),
            (
                "prohibitions",
                Value::Array(
                    self.prohibitions
                        .iter()
                        .map(|rule| effective_rule_value(rule, CapabilityPolicyValue::to_value))
                        .collect(),
                ),
            ),
            (
                "capabilities",
                Value::Array(
                    self.capabilities
                        .iter()
                        .map(|rule| effective_rule_value(rule, CapabilityPolicyValue::to_value))
                        .collect(),
                ),
            ),
            (
                "limits",
                Value::Array(
                    self.limits
                        .iter()
                        .map(|rule| effective_rule_value(rule, LimitPolicyValue::to_value))
                        .collect(),
                ),
            ),
            (
                "type_mode",
                effective_rule_value(&self.type_mode, |mode| text(mode.as_str())),
            ),
        ])
    }

    fn validate(&self) -> Result<()> {
        validate_effective_rules(
            &self.requirements,
            RequirementPolicyValue::validate,
            RequirementPolicyValue::to_value,
            "effective requirements",
        )?;
        validate_effective_rules(
            &self.evidence,
            EvidencePolicyValue::validate,
            EvidencePolicyValue::to_value,
            "effective evidence",
        )?;
        validate_effective_rules(
            &self.prohibitions,
            CapabilityPolicyValue::validate,
            CapabilityPolicyValue::to_value,
            "effective prohibitions",
        )?;
        validate_effective_rules(
            &self.capabilities,
            CapabilityPolicyValue::validate,
            CapabilityPolicyValue::to_value,
            "effective capabilities",
        )?;
        validate_effective_rules(
            &self.limits,
            LimitPolicyValue::validate,
            LimitPolicyValue::to_value,
            "effective limits",
        )?;
        self.type_mode.validate_common()?;
        let mut effects = HashSet::new();
        for rule in &self.capabilities {
            if !effects.insert(rule.value.effect.as_str()) {
                return Err(invalid(
                    "effective capabilities require at most one ceiling per effect",
                ));
            }
        }
        Ok(())
    }

    fn category_len(&self, category: PolicyCategory) -> usize {
        match category {
            PolicyCategory::Requirement => self.requirements.len(),
            PolicyCategory::Evidence => self.evidence.len(),
            PolicyCategory::Prohibition => self.prohibitions.len(),
            PolicyCategory::Capability => self.capabilities.len(),
            PolicyCategory::Limit => self.limits.len(),
            PolicyCategory::TypeMode => 1,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactReference {
    pub media_type: String,
    pub size: u64,
    pub digests: Vec<HashId>,
    pub locations: Option<Vec<String>>,
}

impl ArtifactReference {
    fn from_value(value: &Value) -> Result<Self> {
        let entries = map_entries(value, "policy artifact reference")?;
        ensure_fields(
            entries,
            &["media_type", "size", "digests", "locations"],
            "policy artifact reference",
        )?;
        let size = non_negative_u64(
            required(entries, "size", "policy artifact reference")?,
            "policy artifact size must be a non-negative integer",
        )?;
        let digests = array_values(
            required(entries, "digests", "policy artifact reference")?,
            "policy artifact digests",
        )?
        .iter()
        .map(parse_hash_id)
        .collect::<Result<Vec<_>>>()?;
        let locations = optional(entries, "locations")
            .map(|value| parse_text_array(value, "policy artifact locations", true))
            .transpose()?;
        let reference = Self {
            media_type: required_text(entries, "media_type", "policy artifact reference")?,
            size,
            digests,
            locations,
        };
        reference.validate()?;
        Ok(reference)
    }

    fn to_value(&self) -> Value {
        let mut entries = vec![
            ("media_type".to_owned(), text(&self.media_type)),
            ("size".to_owned(), Value::Integer(self.size as i64)),
            (
                "digests".to_owned(),
                Value::Array(self.digests.iter().map(HashId::to_value).collect()),
            ),
        ];
        if let Some(locations) = &self.locations {
            entries.push((
                "locations".to_owned(),
                Value::Array(locations.iter().cloned().map(Value::Text).collect()),
            ));
        }
        Value::owned_map(entries)
    }

    fn validate(&self) -> Result<()> {
        if self.media_type.is_empty() || self.size > i64::MAX as u64 || self.digests.is_empty() {
            return Err(invalid("invalid policy artifact reference"));
        }
        for digest in &self.digests {
            digest.validate().map_err(policy_error)?;
        }
        if !self
            .digests
            .windows(2)
            .all(|pair| pair[0].algorithm < pair[1].algorithm)
        {
            return Err(invalid(
                "policy artifact digests must use unique sorted algorithms",
            ));
        }
        if let Some(locations) = &self.locations {
            validate_normalized_text(locations, "policy artifact locations", true)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicySource {
    pub symbol: String,
    pub artifact: ArtifactReference,
}

impl PolicySource {
    fn from_value(value: &Value) -> Result<Self> {
        let entries = map_entries(value, "policy source")?;
        ensure_fields(entries, &["symbol", "artifact"], "policy source")?;
        Ok(Self {
            symbol: required_symbol(entries, "symbol", "policy source")?,
            artifact: ArtifactReference::from_value(required(
                entries,
                "artifact",
                "policy source",
            )?)?,
        })
    }

    fn to_value(&self) -> Value {
        Value::map([
            ("symbol", text(&self.symbol)),
            ("artifact", self.artifact.to_value()),
        ])
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicySourceLayer {
    pub layer: PolicyLayer,
    pub policies: Vec<PolicySource>,
}

impl PolicySourceLayer {
    fn from_value(value: &Value) -> Result<Self> {
        let entries = map_entries(value, "policy source layer")?;
        ensure_fields(entries, &["layer", "policies"], "policy source layer")?;
        let layer = Self {
            layer: PolicyLayer::parse(required(entries, "layer", "policy source layer")?)?,
            policies: array_values(
                required(entries, "policies", "policy source layer")?,
                "policy source layer policies",
            )?
            .iter()
            .map(PolicySource::from_value)
            .collect::<Result<Vec<_>>>()?,
        };
        layer.validate()?;
        Ok(layer)
    }

    fn to_value(&self) -> Value {
        Value::map([
            ("layer", text(self.layer.as_str())),
            (
                "policies",
                Value::Array(self.policies.iter().map(PolicySource::to_value).collect()),
            ),
        ])
    }

    fn validate(&self) -> Result<()> {
        if self.policies.is_empty() {
            return Err(invalid("policy source layer requires at least one policy"));
        }
        if !self
            .policies
            .windows(2)
            .all(|pair| pair[0].symbol < pair[1].symbol)
        {
            return Err(invalid(
                "policy sources must be sorted by unique policy symbol",
            ));
        }
        for source in &self.policies {
            validate_symbol(&source.symbol, "policy source symbol")?;
            source.artifact.validate()?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceRuleIdentity {
    pub policy: String,
    pub rule: String,
}

impl SourceRuleIdentity {
    fn from_value(value: &Value) -> Result<Self> {
        let Value::Array(values) = value else {
            return Err(invalid("source rule identity must be a two-item array"));
        };
        let [policy, rule] = values.as_slice() else {
            return Err(invalid("source rule identity must be a two-item array"));
        };
        let identity = Self {
            policy: symbol_value(policy, "source rule policy")?,
            rule: text_value(rule, "source rule ID")?.to_owned(),
        };
        if identity.rule.is_empty() || identity.rule.len() > 128 {
            return Err(invalid("source rule identity contains an invalid ref-id"));
        }
        Ok(identity)
    }

    fn to_value(&self) -> Value {
        Value::Array(vec![text(&self.policy), text(&self.rule)])
    }
}

impl Ord for SourceRuleIdentity {
    fn cmp(&self, other: &Self) -> Ordering {
        self.policy
            .cmp(&other.policy)
            .then_with(|| self.rule.cmp(&other.rule))
    }
}

impl PartialOrd for SourceRuleIdentity {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuleProvenance {
    pub category: PolicyCategory,
    pub effective_rule: usize,
    pub sources: Vec<SourceRuleIdentity>,
}

impl RuleProvenance {
    fn from_value(value: &Value) -> Result<Self> {
        let entries = map_entries(value, "policy rule provenance")?;
        ensure_fields(
            entries,
            &["category", "effective_rule", "sources"],
            "policy rule provenance",
        )?;
        let effective_rule = non_negative_u64(
            required(entries, "effective_rule", "policy rule provenance")?,
            "effective rule index must be a non-negative integer",
        )?;
        let provenance = Self {
            category: PolicyCategory::parse(required(
                entries,
                "category",
                "policy rule provenance",
            )?)?,
            effective_rule: usize::try_from(effective_rule)
                .map_err(|_| invalid("effective rule index exceeds platform range"))?,
            sources: array_values(
                required(entries, "sources", "policy rule provenance")?,
                "policy rule provenance sources",
            )?
            .iter()
            .map(SourceRuleIdentity::from_value)
            .collect::<Result<Vec<_>>>()?,
        };
        if provenance.sources.is_empty()
            || !provenance.sources.windows(2).all(|pair| pair[0] < pair[1])
        {
            return Err(invalid(
                "policy rule provenance sources must be a non-empty sorted set",
            ));
        }
        Ok(provenance)
    }

    fn to_value(&self) -> Value {
        Value::map([
            ("category", text(self.category.as_str())),
            ("effective_rule", Value::Integer(self.effective_rule as i64)),
            (
                "sources",
                Value::Array(
                    self.sources
                        .iter()
                        .map(SourceRuleIdentity::to_value)
                        .collect(),
                ),
            ),
        ])
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppliedWaiver {
    pub waiver: ArtifactReference,
    pub targets: Vec<SourceRuleIdentity>,
    pub decision_time: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WaiverWeakening {
    RemoveRequirement(RequirementPolicyValue),
    RemoveEvidence(EvidencePolicyValue),
    AllowProhibition(CapabilityPolicyValue),
    BroadenCapability {
        from: CapabilityPolicyValue,
        to: CapabilityPolicyValue,
    },
    LoosenLimit {
        from: LimitPolicyValue,
        to: LimitPolicyValue,
    },
    WeakenTypeMode {
        from: TypeMode,
        to: TypeMode,
    },
}

impl WaiverWeakening {
    fn from_value(value: &Value) -> Result<Self> {
        let entries = map_entries(value, "waiver weakening")?;
        let category = required_text(entries, "category", "waiver weakening")?;
        let operation = required_text(entries, "operation", "waiver weakening")?;
        match (category.as_str(), operation.as_str()) {
            ("requirement", "remove") => {
                ensure_fields(
                    entries,
                    &["category", "operation", "value"],
                    "waiver weakening",
                )?;
                Ok(Self::RemoveRequirement(RequirementPolicyValue::from_value(
                    required(entries, "value", "waiver weakening")?,
                )?))
            }
            ("evidence", "remove") => {
                ensure_fields(
                    entries,
                    &["category", "operation", "value"],
                    "waiver weakening",
                )?;
                Ok(Self::RemoveEvidence(EvidencePolicyValue::from_value(
                    required(entries, "value", "waiver weakening")?,
                )?))
            }
            ("prohibition", "allow") => {
                ensure_fields(
                    entries,
                    &["category", "operation", "value"],
                    "waiver weakening",
                )?;
                Ok(Self::AllowProhibition(CapabilityPolicyValue::from_value(
                    required(entries, "value", "waiver weakening")?,
                )?))
            }
            ("capability", "broaden") => {
                ensure_fields(
                    entries,
                    &["category", "operation", "from", "to"],
                    "waiver weakening",
                )?;
                Ok(Self::BroadenCapability {
                    from: CapabilityPolicyValue::from_value(required(
                        entries,
                        "from",
                        "waiver weakening",
                    )?)?,
                    to: CapabilityPolicyValue::from_value(required(
                        entries,
                        "to",
                        "waiver weakening",
                    )?)?,
                })
            }
            ("limit", "loosen") => {
                ensure_fields(
                    entries,
                    &["category", "operation", "from", "to"],
                    "waiver weakening",
                )?;
                Ok(Self::LoosenLimit {
                    from: LimitPolicyValue::from_value(required(
                        entries,
                        "from",
                        "waiver weakening",
                    )?)?,
                    to: LimitPolicyValue::from_value(required(entries, "to", "waiver weakening")?)?,
                })
            }
            ("type-mode", "weaken") => {
                ensure_fields(
                    entries,
                    &["category", "operation", "from", "to"],
                    "waiver weakening",
                )?;
                Ok(Self::WeakenTypeMode {
                    from: TypeMode::parse(required(entries, "from", "waiver weakening")?)?,
                    to: TypeMode::parse(required(entries, "to", "waiver weakening")?)?,
                })
            }
            _ => Err(waiver_invalid(
                "waiver weakening requires a registered category and operation",
            )),
        }
    }

    fn to_value(&self) -> Value {
        match self {
            Self::RemoveRequirement(value) => {
                weakening_value("requirement", "remove", value.to_value())
            }
            Self::RemoveEvidence(value) => weakening_value("evidence", "remove", value.to_value()),
            Self::AllowProhibition(value) => {
                weakening_value("prohibition", "allow", value.to_value())
            }
            Self::BroadenCapability { from, to } => {
                from_to_weakening_value("capability", "broaden", from.to_value(), to.to_value())
            }
            Self::LoosenLimit { from, to } => {
                from_to_weakening_value("limit", "loosen", from.to_value(), to.to_value())
            }
            Self::WeakenTypeMode { from, to } => from_to_weakening_value(
                "type-mode",
                "weaken",
                text(from.as_str()),
                text(to.as_str()),
            ),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WaiverTarget {
    pub rule: SourceRuleIdentity,
    pub scope: Option<PolicyScope>,
    pub weakening: WaiverWeakening,
}

impl WaiverTarget {
    fn from_value(value: &Value) -> Result<Self> {
        let entries = map_entries(value, "waiver target")?;
        ensure_fields(entries, &["rule", "scope", "weakening"], "waiver target")?;
        Ok(Self {
            rule: SourceRuleIdentity::from_value(required(entries, "rule", "waiver target")?)?,
            scope: optional(entries, "scope")
                .map(PolicyScope::from_value)
                .transpose()?,
            weakening: WaiverWeakening::from_value(required(
                entries,
                "weakening",
                "waiver target",
            )?)?,
        })
    }

    fn to_value(&self) -> Value {
        let mut entries = vec![
            ("rule".to_owned(), self.rule.to_value()),
            ("weakening".to_owned(), self.weakening.to_value()),
        ];
        if let Some(scope) = &self.scope {
            entries.push(("scope".to_owned(), scope.to_value()));
        }
        Value::owned_map(entries)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WaiverDelegation {
    pub delegator: String,
    pub delegate: String,
    pub authorization: ArtifactReference,
}

impl WaiverDelegation {
    fn from_value(value: &Value) -> Result<Self> {
        let entries = map_entries(value, "waiver delegation")?;
        ensure_fields(
            entries,
            &["delegator", "delegate", "authorization"],
            "waiver delegation",
        )?;
        Ok(Self {
            delegator: required_text(entries, "delegator", "waiver delegation")?,
            delegate: required_text(entries, "delegate", "waiver delegation")?,
            authorization: ArtifactReference::from_value(required(
                entries,
                "authorization",
                "waiver delegation",
            )?)?,
        })
    }

    fn to_value(&self) -> Value {
        Value::map([
            ("delegator", text(&self.delegator)),
            ("delegate", text(&self.delegate)),
            ("authorization", self.authorization.to_value()),
        ])
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WaiverDocument {
    pub features: Vec<String>,
    pub artifact_id: Option<HashId>,
    pub provenance: Option<Value>,
    pub authorization: Vec<Value>,
    pub symbol: String,
    pub targets: Vec<WaiverTarget>,
    pub justification: String,
    pub issuer: String,
    pub authority_chain: Vec<WaiverDelegation>,
    pub issued_at: String,
    pub not_before: String,
    pub expires_at: String,
    pub audit_reference: ArtifactReference,
}

impl WaiverDocument {
    pub fn from_value(value: &Value) -> Result<Self> {
        let entries = map_entries(value, "waiver document")?;
        ensure_fields(
            entries,
            &[
                "version",
                "features",
                "artifact_id",
                "provenance",
                "authorization",
                "kind",
                "symbol",
                "targets",
                "justification",
                "issuer",
                "authority_chain",
                "issued_at",
                "not_before",
                "expires_at",
                "audit_reference",
            ],
            "waiver document",
        )?;
        require_exact_text(entries, "version", "bhcp/v0", "waiver document")?;
        require_exact_text(entries, "kind", "waiver", "waiver document")?;
        let authorization = array_values(
            required(entries, "authorization", "waiver document")?,
            "waiver authorization",
        )?
        .to_vec();
        let document = Self {
            features: parse_symbol_array(
                required(entries, "features", "waiver document")?,
                "waiver features",
                true,
            )?,
            artifact_id: optional(entries, "artifact_id")
                .map(parse_hash_id)
                .transpose()?,
            provenance: optional(entries, "provenance").cloned(),
            authorization,
            symbol: required_symbol(entries, "symbol", "waiver document")?,
            targets: array_values(
                required(entries, "targets", "waiver document")?,
                "waiver targets",
            )?
            .iter()
            .map(WaiverTarget::from_value)
            .collect::<Result<Vec<_>>>()?,
            justification: required_text(entries, "justification", "waiver document")?,
            issuer: required_text(entries, "issuer", "waiver document")?,
            authority_chain: array_values(
                required(entries, "authority_chain", "waiver document")?,
                "waiver authority chain",
            )?
            .iter()
            .map(WaiverDelegation::from_value)
            .collect::<Result<Vec<_>>>()?,
            issued_at: policy_timestamp(
                required(entries, "issued_at", "waiver document")?,
                "waiver issued_at",
            )?,
            not_before: policy_timestamp(
                required(entries, "not_before", "waiver document")?,
                "waiver not_before",
            )?,
            expires_at: policy_timestamp(
                required(entries, "expires_at", "waiver document")?,
                "waiver expires_at",
            )?,
            audit_reference: ArtifactReference::from_value(required(
                entries,
                "audit_reference",
                "waiver document",
            )?)?,
        };
        document.validate()?;
        Ok(document)
    }

    pub fn to_value(&self, include_artifact_id: bool) -> Value {
        let mut entries = vec![
            ("version".to_owned(), text("bhcp/v0")),
            (
                "features".to_owned(),
                Value::Array(self.features.iter().cloned().map(Value::Text).collect()),
            ),
            (
                "authorization".to_owned(),
                Value::Array(self.authorization.clone()),
            ),
            ("kind".to_owned(), text("waiver")),
            ("symbol".to_owned(), text(&self.symbol)),
            (
                "targets".to_owned(),
                Value::Array(self.targets.iter().map(WaiverTarget::to_value).collect()),
            ),
            ("justification".to_owned(), text(&self.justification)),
            ("issuer".to_owned(), text(&self.issuer)),
            (
                "authority_chain".to_owned(),
                Value::Array(
                    self.authority_chain
                        .iter()
                        .map(WaiverDelegation::to_value)
                        .collect(),
                ),
            ),
            (
                "issued_at".to_owned(),
                Value::Tag(0, Box::new(text(&self.issued_at))),
            ),
            (
                "not_before".to_owned(),
                Value::Tag(0, Box::new(text(&self.not_before))),
            ),
            (
                "expires_at".to_owned(),
                Value::Tag(0, Box::new(text(&self.expires_at))),
            ),
            (
                "audit_reference".to_owned(),
                self.audit_reference.to_value(),
            ),
        ];
        if include_artifact_id && let Some(artifact_id) = &self.artifact_id {
            entries.push(("artifact_id".to_owned(), artifact_id.to_value()));
        }
        if let Some(provenance) = &self.provenance {
            entries.push(("provenance".to_owned(), provenance.clone()));
        }
        Value::owned_map(entries)
    }

    fn validate(&self) -> Result<()> {
        validate_normalized_symbols(&self.features, "waiver features", true)?;
        validate_symbol(&self.symbol, "waiver symbol")?;
        if self.authorization.is_empty()
            || self
                .authorization
                .iter()
                .any(|value| !matches!(value, Value::Map(_)))
        {
            return Err(waiver_invalid(
                "waiver authorization must be a non-empty array of maps",
            ));
        }
        if self.targets.is_empty() {
            return Err(waiver_invalid("waiver targets must be non-empty"));
        }
        validate_values_sorted_unique(
            &self
                .targets
                .iter()
                .map(WaiverTarget::to_value)
                .collect::<Vec<_>>(),
            "waiver targets",
        )
        .map_err(|diagnostic| waiver_invalid(diagnostic.message))?;
        if self.justification.is_empty() || self.issuer.is_empty() {
            return Err(waiver_invalid(
                "waiver justification and issuer must be non-empty",
            ));
        }
        self.audit_reference
            .validate()
            .map_err(|diagnostic| waiver_invalid(diagnostic.message))?;
        validate_policy_timestamp(&self.issued_at, "waiver issued_at")?;
        validate_policy_timestamp(&self.not_before, "waiver not_before")?;
        validate_policy_timestamp(&self.expires_at, "waiver expires_at")?;
        if self.issued_at > self.not_before || self.not_before >= self.expires_at {
            return Err(waiver_time(
                "waiver interval must satisfy issued_at <= not_before < expires_at",
            ));
        }
        let mut expected = None::<&str>;
        let mut principals = BTreeSet::new();
        for delegation in &self.authority_chain {
            if delegation.delegator.is_empty() || delegation.delegate.is_empty() {
                return Err(waiver_authority(
                    "waiver delegation principals must be non-empty",
                ));
            }
            if expected.is_some_and(|value| value != delegation.delegator) {
                return Err(waiver_authority("waiver authority chain is disconnected"));
            }
            if !principals.insert(delegation.delegator.as_str())
                || !principals.insert(delegation.delegate.as_str())
            {
                return Err(waiver_authority(
                    "waiver authority chain repeats a principal",
                ));
            }
            delegation
                .authorization
                .validate()
                .map_err(|diagnostic| waiver_authority(diagnostic.message))?;
            expected = Some(&delegation.delegate);
        }
        if expected.is_some_and(|value| value != self.issuer) {
            return Err(waiver_authority(
                "waiver authority chain does not end at the issuer",
            ));
        }
        if let Some(artifact_id) = &self.artifact_id {
            let algorithm = HashAlgorithm::from_id(&artifact_id.algorithm).map_err(policy_error)?;
            if artifact_hash_with(&self.to_value(false), algorithm)? != *artifact_id {
                return Err(waiver_invalid(
                    "waiver artifact_id does not match the document",
                ));
            }
        }
        Ok(())
    }

    fn artifact_reference(&self, algorithm: HashAlgorithm) -> Result<ArtifactReference> {
        let mut materialized = self.clone();
        materialized.artifact_id = Some(artifact_hash_with(
            &materialized.to_value(false),
            algorithm,
        )?);
        let bytes = encode_deterministic(&materialized.to_value(true))?;
        Ok(ArtifactReference {
            media_type: "application/vnd.bhcp.waiver+cbor".to_owned(),
            size: bytes.len() as u64,
            digests: vec![algorithm.hash(&bytes)],
            locations: None,
        })
    }
}

impl AppliedWaiver {
    fn from_value(value: &Value) -> Result<Self> {
        let entries = map_entries(value, "applied waiver")?;
        ensure_fields(
            entries,
            &["waiver", "targets", "decision_time"],
            "applied waiver",
        )?;
        let applied = Self {
            waiver: ArtifactReference::from_value(required(entries, "waiver", "applied waiver")?)?,
            targets: array_values(
                required(entries, "targets", "applied waiver")?,
                "applied waiver targets",
            )?
            .iter()
            .map(SourceRuleIdentity::from_value)
            .collect::<Result<Vec<_>>>()?,
            decision_time: policy_timestamp(
                required(entries, "decision_time", "applied waiver")?,
                "applied waiver decision_time",
            )?,
        };
        applied.validate()?;
        Ok(applied)
    }

    fn to_value(&self) -> Value {
        Value::map([
            ("waiver", self.waiver.to_value()),
            (
                "targets",
                Value::Array(
                    self.targets
                        .iter()
                        .map(SourceRuleIdentity::to_value)
                        .collect(),
                ),
            ),
            (
                "decision_time",
                Value::Tag(0, Box::new(text(&self.decision_time))),
            ),
        ])
    }

    fn validate(&self) -> Result<()> {
        self.waiver.validate()?;
        if self.targets.is_empty() || !self.targets.windows(2).all(|pair| pair[0] < pair[1]) {
            return Err(invalid(
                "applied waiver targets must be a non-empty sorted set",
            ));
        }
        validate_policy_timestamp(&self.decision_time, "applied waiver decision_time")
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectivePolicyDocument {
    pub header: PolicyHeader,
    pub effective: EffectivePolicy,
    pub source_layers: Vec<PolicySourceLayer>,
    pub rule_provenance: Vec<RuleProvenance>,
    pub waivers: Option<Vec<AppliedWaiver>>,
}

impl EffectivePolicyDocument {
    fn from_entries(entries: &[(String, Value)]) -> Result<Self> {
        ensure_fields(
            entries,
            &[
                "version",
                "features",
                "semantic_id",
                "artifact_id",
                "provenance",
                "authorization",
                "kind",
                "form",
                "effective",
                "source_layers",
                "rule_provenance",
                "waivers",
            ],
            "effective policy document",
        )?;
        require_exact_text(entries, "kind", "policy", "effective policy document")?;
        require_exact_text(entries, "form", "effective", "effective policy document")?;
        let header = PolicyHeader::from_entries(entries)?;
        if header.semantic_id.is_none() {
            return Err(invalid("effective policy document requires semantic_id"));
        }
        let document = Self {
            header,
            effective: EffectivePolicy::from_value(required(
                entries,
                "effective",
                "effective policy document",
            )?)?,
            source_layers: array_values(
                required(entries, "source_layers", "effective policy document")?,
                "effective policy source layers",
            )?
            .iter()
            .map(PolicySourceLayer::from_value)
            .collect::<Result<Vec<_>>>()?,
            rule_provenance: array_values(
                required(entries, "rule_provenance", "effective policy document")?,
                "effective policy rule provenance",
            )?
            .iter()
            .map(RuleProvenance::from_value)
            .collect::<Result<Vec<_>>>()?,
            waivers: optional(entries, "waivers")
                .map(|value| {
                    array_values(value, "effective policy waivers")?
                        .iter()
                        .map(AppliedWaiver::from_value)
                        .collect::<Result<Vec<_>>>()
                })
                .transpose()?,
        };
        document.validate()?;
        Ok(document)
    }

    fn to_value(&self, include_artifact_id: bool) -> Value {
        let mut entries = self.header.entries(include_artifact_id);
        entries.extend([
            ("kind".to_owned(), text("policy")),
            ("form".to_owned(), text("effective")),
            ("effective".to_owned(), self.effective.to_value()),
            (
                "source_layers".to_owned(),
                Value::Array(
                    self.source_layers
                        .iter()
                        .map(PolicySourceLayer::to_value)
                        .collect(),
                ),
            ),
            (
                "rule_provenance".to_owned(),
                Value::Array(
                    self.rule_provenance
                        .iter()
                        .map(RuleProvenance::to_value)
                        .collect(),
                ),
            ),
        ]);
        if let Some(waivers) = &self.waivers {
            entries.push((
                "waivers".to_owned(),
                Value::Array(waivers.iter().map(AppliedWaiver::to_value).collect()),
            ));
        }
        Value::owned_map(entries)
    }

    /// Computes identity from normalized effective policy meaning only.
    pub fn compute_semantic_id(&self, algorithm: HashAlgorithm) -> Result<HashId> {
        hash_value(&self.effective.to_value(), algorithm)
    }

    /// Computes identity from the complete effective policy artifact except its ID.
    pub fn compute_artifact_id(&self, algorithm: HashAlgorithm) -> Result<HashId> {
        artifact_hash_with(&self.to_value(false), algorithm)
    }

    fn materialize_identities(&mut self, algorithm: HashAlgorithm) -> Result<()> {
        self.header.semantic_id = Some(self.compute_semantic_id(algorithm)?);
        self.header.artifact_id = Some(self.compute_artifact_id(algorithm)?);
        Ok(())
    }

    fn validate(&self) -> Result<()> {
        self.header.validate()?;
        self.effective.validate()?;
        if !self
            .source_layers
            .windows(2)
            .all(|pair| pair[0].layer < pair[1].layer)
        {
            return Err(invalid(
                "effective policy source layers must follow organization, team, repository, user order",
            ));
        }
        for layer in &self.source_layers {
            layer.validate()?;
        }
        if !self.rule_provenance.windows(2).all(|pair| {
            (pair[0].category, pair[0].effective_rule) < (pair[1].category, pair[1].effective_rule)
        }) {
            return Err(invalid(
                "effective policy rule provenance must be sorted by unique category and index",
            ));
        }
        for provenance in &self.rule_provenance {
            if provenance.effective_rule >= self.effective.category_len(provenance.category) {
                return Err(invalid(
                    "policy rule provenance references an unknown effective rule",
                ));
            }
            if provenance.sources.is_empty()
                || !provenance.sources.windows(2).all(|pair| pair[0] < pair[1])
            {
                return Err(invalid(
                    "policy rule provenance sources must be a non-empty sorted set",
                ));
            }
        }
        if let Some(waivers) = &self.waivers {
            for waiver in waivers {
                waiver.validate()?;
            }
            validate_values_sorted_unique(
                &waivers
                    .iter()
                    .map(AppliedWaiver::to_value)
                    .collect::<Vec<_>>(),
                "effective policy waivers",
            )?;
        }
        let semantic_id = self
            .header
            .semantic_id
            .as_ref()
            .ok_or_else(|| invalid("effective policy document requires semantic_id"))?;
        let algorithm = HashAlgorithm::from_id(&semantic_id.algorithm).map_err(policy_error)?;
        if self.compute_semantic_id(algorithm)? != *semantic_id {
            return Err(invalid(
                "effective policy semantic_id does not match effective meaning",
            ));
        }
        if let Some(artifact_id) = &self.header.artifact_id {
            let algorithm = HashAlgorithm::from_id(&artifact_id.algorithm).map_err(policy_error)?;
            if self.compute_artifact_id(algorithm)? != *artifact_id {
                return Err(invalid(
                    "effective policy artifact_id does not match document",
                ));
            }
        }
        Ok(())
    }
}

#[derive(Clone)]
struct ComposedRule<T> {
    effective: EffectiveRule<T>,
    sources: Vec<SourceRuleIdentity>,
}

#[derive(Default)]
struct Composition {
    requirements: Vec<ComposedRule<RequirementPolicyValue>>,
    evidence: Vec<ComposedRule<EvidencePolicyValue>>,
    prohibitions: Vec<ComposedRule<CapabilityPolicyValue>>,
    capabilities: Vec<ComposedRule<CapabilityPolicyValue>>,
    limits: Vec<ComposedRule<LimitPolicyValue>>,
    type_mode: Option<ComposedRule<TypeMode>>,
}

/// Validates and atomically applies one waiver to a pre-waiver effective policy.
pub fn apply_waiver(
    policy: &EffectivePolicyDocument,
    waiver: &WaiverDocument,
    decision_time: &str,
    algorithm: HashAlgorithm,
) -> Result<EffectivePolicyDocument> {
    PolicyDocument::Effective(policy.clone()).validate()?;
    waiver.validate()?;
    validate_policy_timestamp(decision_time, "waiver decision_time")?;
    if decision_time < waiver.not_before.as_str() || decision_time >= waiver.expires_at.as_str() {
        return Err(waiver_time(
            "waiver is inactive at the injected decision time",
        ));
    }

    let authority_root = waiver
        .authority_chain
        .first()
        .map_or(waiver.issuer.as_str(), |delegation| {
            delegation.delegator.as_str()
        });
    let mut resolved = Vec::with_capacity(waiver.targets.len());
    for target in &waiver.targets {
        let Some(provenance) = policy
            .rule_provenance
            .iter()
            .find(|entry| entry.sources.binary_search(&target.rule).is_ok())
            .cloned()
        else {
            return Err(waiver_target(format!(
                "waiver target {}:{} does not resolve to an effective rule",
                target.rule.policy, target.rule.rule
            )));
        };
        let targeted_sources = waiver
            .targets
            .iter()
            .filter(|candidate| provenance.sources.binary_search(&candidate.rule).is_ok())
            .map(|candidate| candidate.rule.clone())
            .collect::<BTreeSet<_>>();
        if targeted_sources.len() != provenance.sources.len() {
            return Err(waiver_target(
                "waiver must target every source contributing to the effective rule",
            ));
        }
        resolved.push((target, provenance));
    }
    resolved.sort_by(|(_, left), (_, right)| {
        left.category
            .cmp(&right.category)
            .then_with(|| right.effective_rule.cmp(&left.effective_rule))
    });

    let mut output = policy.clone();
    let mut processed =
        BTreeMap::<(PolicyCategory, usize), (Option<PolicyScope>, WaiverWeakening)>::new();
    for (target, provenance) in resolved {
        let key = (provenance.category, provenance.effective_rule);
        if let Some((scope, weakening)) = processed.get(&key) {
            if scope != &normalized_scope(&target.scope) || weakening != &target.weakening {
                return Err(waiver_change(
                    "targets for one effective rule must request the same exact weakening",
                ));
            }
            continue;
        }
        processed.insert(
            key,
            (normalized_scope(&target.scope), target.weakening.clone()),
        );
        match (&target.weakening, provenance.category) {
            (WaiverWeakening::RemoveRequirement(expected), PolicyCategory::Requirement) => {
                let rule = output
                    .effective
                    .requirements
                    .get(provenance.effective_rule)
                    .ok_or_else(|| waiver_target("waiver requirement target index is invalid"))?;
                authorize_effective_rule(rule, authority_root)?;
                if expected != &rule.value
                    || normalized_scope(&target.scope) != normalized_scope(&rule.value.scope)
                {
                    return Err(waiver_change(
                        "waiver requirement removal does not match the exact effective rule",
                    ));
                }
                output
                    .effective
                    .requirements
                    .remove(provenance.effective_rule);
                remove_rule_provenance(
                    &mut output.rule_provenance,
                    PolicyCategory::Requirement,
                    provenance.effective_rule,
                );
            }
            (WaiverWeakening::RemoveEvidence(expected), PolicyCategory::Evidence) => {
                let rule = output
                    .effective
                    .evidence
                    .get(provenance.effective_rule)
                    .ok_or_else(|| waiver_target("waiver evidence target index is invalid"))?;
                authorize_effective_rule(rule, authority_root)?;
                if expected != &rule.value
                    || normalized_scope(&target.scope) != normalized_scope(&rule.value.scope)
                {
                    return Err(waiver_change(
                        "waiver evidence removal does not match the exact effective rule",
                    ));
                }
                output.effective.evidence.remove(provenance.effective_rule);
                remove_rule_provenance(
                    &mut output.rule_provenance,
                    PolicyCategory::Evidence,
                    provenance.effective_rule,
                );
            }
            (WaiverWeakening::AllowProhibition(expected), PolicyCategory::Prohibition) => {
                let rule = output
                    .effective
                    .prohibitions
                    .get(provenance.effective_rule)
                    .ok_or_else(|| waiver_target("waiver prohibition target index is invalid"))?;
                authorize_effective_rule(rule, authority_root)?;
                if expected != &rule.value
                    || normalized_scope(&target.scope) != normalized_scope(&rule.value.scope)
                {
                    return Err(waiver_change(
                        "waiver prohibition allowance does not match the exact effective rule",
                    ));
                }
                output
                    .effective
                    .prohibitions
                    .remove(provenance.effective_rule);
                remove_rule_provenance(
                    &mut output.rule_provenance,
                    PolicyCategory::Prohibition,
                    provenance.effective_rule,
                );
            }
            (WaiverWeakening::WeakenTypeMode { from, to }, PolicyCategory::TypeMode) => {
                let rule = &mut output.effective.type_mode;
                authorize_effective_rule(rule, authority_root)?;
                if normalized_scope(&target.scope).is_some() {
                    return Err(waiver_change(
                        "type-mode waiver target must not declare a scope",
                    ));
                }
                if from != &rule.value || to >= from {
                    return Err(waiver_change(
                        "waiver type-mode change is not an exact weakening",
                    ));
                }
                rule.value = *to;
            }
            (WaiverWeakening::BroadenCapability { from, to }, PolicyCategory::Capability) => {
                let rule = output
                    .effective
                    .capabilities
                    .get_mut(provenance.effective_rule)
                    .ok_or_else(|| waiver_target("waiver capability target index is invalid"))?;
                authorize_effective_rule(rule, authority_root)?;
                let target_scope = normalized_scope(&target.scope);
                if target_scope != normalized_scope(&rule.value.scope)
                    || target_scope != normalized_scope(&from.scope)
                {
                    return Err(waiver_change(
                        "capability waiver does not match the exact target scope",
                    ));
                }
                if from != &rule.value
                    || from.effect != to.effect
                    || !scope_subset(&from.scope, &to.scope)
                    || normalized_scope(&from.scope) == normalized_scope(&to.scope)
                {
                    return Err(waiver_change(
                        "waiver capability change is not an exact broadening",
                    ));
                }
                rule.value = to.clone();
            }
            (WaiverWeakening::LoosenLimit { from, to }, PolicyCategory::Limit) => {
                let rule = output
                    .effective
                    .limits
                    .get_mut(provenance.effective_rule)
                    .ok_or_else(|| waiver_target("waiver limit target index is invalid"))?;
                authorize_effective_rule(rule, authority_root)?;
                let target_scope = normalized_scope(&target.scope);
                if target_scope != normalized_scope(&rule.value.scope)
                    || target_scope != normalized_scope(&from.scope)
                    || target_scope != normalized_scope(&to.scope)
                {
                    return Err(waiver_change(
                        "implemented waiver application requires one exact representable scope",
                    ));
                }
                if from != &rule.value
                    || from.dimension != to.dimension
                    || from.unit != to.unit
                    || exact_number_cmp(&to.maximum, &from.maximum) != Ordering::Greater
                {
                    return Err(waiver_change(
                        "waiver limit change does not match the exact effective restriction",
                    ));
                }
                rule.value = to.clone();
            }
            _ => {
                return Err(waiver_change(
                    "waiver change category does not match the effective rule",
                ));
            }
        }
    }

    let mut targets = waiver
        .targets
        .iter()
        .map(|target| target.rule.clone())
        .collect::<Vec<_>>();
    targets.sort();
    targets.dedup();
    let applied = AppliedWaiver {
        waiver: waiver.artifact_reference(algorithm)?,
        targets,
        decision_time: decision_time.to_owned(),
    };
    let waivers = output.waivers.get_or_insert_with(Vec::new);
    waivers.push(applied);
    waivers.sort_by_cached_key(|entry| {
        encode_deterministic(&entry.to_value()).expect("validated applied waiver encodes")
    });
    output.materialize_identities(algorithm)?;
    output.validate()?;
    Ok(output)
}

fn authorize_effective_rule<T>(rule: &EffectiveRule<T>, authority_root: &str) -> Result<()> {
    if !rule.waivable {
        return Err(Diagnostic::plain(
            WAIVER_NONWAIVABLE,
            "waiver target is non-waivable",
        ));
    }
    if rule
        .authorized_issuers
        .binary_search_by(|candidate| candidate.as_str().cmp(authority_root))
        .is_err()
    {
        return Err(waiver_authority(
            "waiver issuer is not authorized for the target rule",
        ));
    }
    Ok(())
}

fn remove_rule_provenance(
    provenance: &mut Vec<RuleProvenance>,
    category: PolicyCategory,
    index: usize,
) {
    provenance.retain(|entry| !(entry.category == category && entry.effective_rule == index));
    for entry in provenance
        .iter_mut()
        .filter(|entry| entry.category == category && entry.effective_rule > index)
    {
        entry.effective_rule -= 1;
    }
}

/// Composes validated source documents in organization-to-user order.
pub fn compose_policies(
    sources: &[SourcePolicyDocument],
    algorithm: HashAlgorithm,
) -> Result<EffectivePolicyDocument> {
    for source in sources {
        PolicyDocument::Source(source.clone()).validate()?;
    }
    let mut ordered = sources.iter().collect::<Vec<_>>();
    ordered.sort_by(|left, right| left.symbol.cmp(&right.symbol));
    if let Some(pair) = ordered
        .windows(2)
        .find(|pair| pair[0].symbol == pair[1].symbol)
    {
        return Err(composition_invalid(format!(
            "duplicate policy source {}",
            pair[0].symbol
        )));
    }
    validate_inheritance(&ordered)?;

    let mut by_layer = BTreeMap::<PolicyLayer, Vec<&SourcePolicyDocument>>::new();
    let mut features = BTreeSet::new();
    for source in &ordered {
        by_layer.entry(source.layer).or_default().push(*source);
        features.extend(source.header.features.iter().cloned());
    }

    let mut composition = Composition::default();
    for (layer, documents) in &by_layer {
        validate_layer_monotonicity(*layer, documents, &composition, &ordered)?;
        for document in documents {
            for rule in &document.rules {
                composition.add_source_rule(document, rule)?;
            }
        }
    }
    composition.finish(ordered, features.into_iter().collect(), algorithm)
}

fn validate_inheritance(sources: &[&SourcePolicyDocument]) -> Result<()> {
    for source in sources {
        if let Some(parent) = &source.extends {
            let Some(parent) = sources.iter().find(|candidate| candidate.symbol == *parent) else {
                return Err(composition_invalid(format!(
                    "policy {} extends missing policy {}",
                    source.symbol, parent
                )));
            };
            if parent.layer != source.layer {
                return Err(composition_invalid(format!(
                    "policy {} extends a policy in another layer",
                    source.symbol
                )));
            }
        }
    }
    let mut states = vec![0_u8; sources.len()];
    for index in 0..sources.len() {
        visit_inheritance(index, sources, &mut states)?;
    }
    Ok(())
}

fn visit_inheritance(
    index: usize,
    sources: &[&SourcePolicyDocument],
    states: &mut [u8],
) -> Result<()> {
    if states[index] == 2 {
        return Ok(());
    }
    states[index] = 1;
    if let Some(parent) = &sources[index].extends {
        let parent_index = sources
            .iter()
            .position(|candidate| candidate.symbol == *parent)
            .expect("inheritance target was validated");
        if states[parent_index] == 1 {
            return Err(composition_invalid(format!(
                "policy inheritance cycle includes {}",
                sources[parent_index].symbol
            )));
        }
        visit_inheritance(parent_index, sources, states)?;
    }
    states[index] = 2;
    Ok(())
}

fn validate_layer_monotonicity(
    layer: PolicyLayer,
    documents: &[&SourcePolicyDocument],
    earlier: &Composition,
    sources: &[&SourcePolicyDocument],
) -> Result<()> {
    let mut layer_limits =
        Vec::<(&SourcePolicyDocument, &PolicyRuleCommon, &LimitPolicyValue)>::new();
    for document in documents {
        for rule in &document.rules {
            match rule {
                PolicyRule::Capability { common, value } => {
                    if let Some(ceiling) = earlier
                        .capabilities
                        .iter()
                        .find(|candidate| candidate.effective.value.effect == value.effect)
                        && !scope_subset(&value.scope, &ceiling.effective.value.scope)
                    {
                        let (earlier_layer, earlier_source) =
                            earlier_authority_where(ceiling, sources, |rule| {
                                matches!(
                                    rule,
                                    PolicyRule::Capability { value: prior, .. }
                                        if prior.effect == value.effect
                                            && !scope_subset(&value.scope, &prior.scope)
                                )
                            });
                        return Err(weakening(
                            CAPABILITY_WIDENING,
                            layer,
                            document,
                            common,
                            format!(
                                "broadens capability {} from {} to {}",
                                value.effect,
                                display_scope(&ceiling.effective.value.scope),
                                display_scope(&value.scope)
                            ),
                            earlier_layer,
                            &earlier_source,
                        ));
                    }
                }
                PolicyRule::Limit { common, value } => {
                    for (prior_document, prior_common, prior) in
                        layer_limits.iter().filter(|(_, _, prior)| {
                            prior.dimension == value.dimension
                                && scopes_overlap(&prior.scope, &value.scope)
                        })
                    {
                        if prior.unit != value.unit {
                            let prior_identity = SourceRuleIdentity {
                                policy: prior_document.symbol.clone(),
                                rule: prior_common.id.clone(),
                            };
                            return Err(unit_conflict(
                                layer,
                                document,
                                common,
                                value,
                                layer,
                                prior,
                                &prior_identity,
                            ));
                        }
                    }
                    for prior in earlier.limits.iter().filter(|candidate| {
                        candidate.effective.value.dimension == value.dimension
                            && scopes_overlap(&candidate.effective.value.scope, &value.scope)
                    }) {
                        if prior.effective.value.unit != value.unit {
                            let (earlier_layer, earlier_source) = earlier_authority(prior, sources);
                            return Err(unit_conflict(
                                layer,
                                document,
                                common,
                                value,
                                earlier_layer,
                                &prior.effective.value,
                                &earlier_source,
                            ));
                        }
                        if exact_number_cmp(&value.maximum, &prior.effective.value.maximum)
                            == Ordering::Greater
                        {
                            let (earlier_layer, earlier_source) =
                                earlier_authority_where(prior, sources, |rule| {
                                    matches!(
                                        rule,
                                        PolicyRule::Limit { value: prior, .. }
                                            if prior.dimension == value.dimension
                                                && scopes_overlap(&prior.scope, &value.scope)
                                                && exact_number_cmp(
                                                    &value.maximum,
                                                    &prior.maximum,
                                                ) == Ordering::Greater
                                    )
                                });
                            return Err(weakening(
                                LIMIT_LOOSENING,
                                layer,
                                document,
                                common,
                                format!(
                                    "loosens limit {} from {} to {}",
                                    value.dimension,
                                    display_exact_number(&prior.effective.value.maximum),
                                    display_exact_number(&value.maximum)
                                ),
                                earlier_layer,
                                &earlier_source,
                            ));
                        }
                    }
                    layer_limits.push((document, common, value));
                }
                PolicyRule::TypeMode { common, value } => {
                    if let Some(prior) = &earlier.type_mode
                        && value < &prior.effective.value
                    {
                        let (earlier_layer, earlier_source) =
                            earlier_authority_where(prior, sources, |rule| {
                                matches!(
                                    rule,
                                    PolicyRule::TypeMode { value: prior, .. } if value < prior
                                )
                            });
                        return Err(weakening(
                            TYPE_MODE_WEAKENING,
                            layer,
                            document,
                            common,
                            format!(
                                "weakens type mode from {} to {}",
                                prior.effective.value.as_str(),
                                value.as_str()
                            ),
                            earlier_layer,
                            &earlier_source,
                        ));
                    }
                }
                _ => {}
            }
        }
    }
    Ok(())
}

fn weakening(
    code: &'static str,
    layer: PolicyLayer,
    document: &SourcePolicyDocument,
    common: &PolicyRuleCommon,
    action: String,
    earlier_layer: PolicyLayer,
    earlier: &SourceRuleIdentity,
) -> Diagnostic {
    weakening_diagnostic(
        code,
        layer,
        &document.symbol,
        &common.id,
        &action,
        earlier_layer,
        earlier,
    )
}

fn weakening_diagnostic(
    code: &'static str,
    layer: PolicyLayer,
    policy: &str,
    rule: &str,
    action: &str,
    earlier_layer: PolicyLayer,
    earlier: &SourceRuleIdentity,
) -> Diagnostic {
    Diagnostic::plain(
        code,
        format!(
            "{} policy {} rule {} {}; earlier {} authority {}:{}; waiver required",
            layer.as_str(),
            policy,
            rule,
            action,
            earlier_layer.as_str(),
            earlier.policy,
            earlier.rule
        ),
    )
}

fn earlier_authority<T>(
    rule: &ComposedRule<T>,
    sources: &[&SourcePolicyDocument],
) -> (PolicyLayer, SourceRuleIdentity) {
    earlier_authority_where(rule, sources, |_| true)
}

fn earlier_authority_where<T>(
    rule: &ComposedRule<T>,
    sources: &[&SourcePolicyDocument],
    mut qualifies: impl FnMut(&PolicyRule) -> bool,
) -> (PolicyLayer, SourceRuleIdentity) {
    let identity = rule
        .sources
        .iter()
        .find(|identity| {
            let source = sources
                .iter()
                .find(|source| source.symbol == identity.policy)
                .expect("composed rule source belongs to the validated source set");
            let source_rule = source
                .rules
                .iter()
                .find(|rule| rule.id() == identity.rule)
                .expect("composed rule identity belongs to its validated source");
            qualifies(source_rule)
        })
        .expect("a weakening has at least one governing earlier source")
        .clone();
    let layer = sources
        .iter()
        .find(|source| source.symbol == identity.policy)
        .expect("composed rule source belongs to the validated source set")
        .layer;
    (layer, identity)
}

fn unit_conflict(
    layer: PolicyLayer,
    document: &SourcePolicyDocument,
    common: &PolicyRuleCommon,
    attempted: &LimitPolicyValue,
    earlier_layer: PolicyLayer,
    earlier: &LimitPolicyValue,
    earlier_source: &SourceRuleIdentity,
) -> Diagnostic {
    weakening_diagnostic(
        INCOMPATIBLE_LIMIT_UNITS,
        layer,
        &document.symbol,
        &common.id,
        &format!(
            "uses incompatible unit {} for overlapping limit {}; earlier unit {}",
            attempted.unit, attempted.dimension, earlier.unit
        ),
        earlier_layer,
        earlier_source,
    )
}

fn display_scope(scope: &Option<PolicyScope>) -> String {
    let Some(scope) = scope else {
        return "universe".to_owned();
    };
    let mut dimensions = Vec::new();
    for (name, values) in [
        ("goals", &scope.goals),
        ("resources", &scope.resources),
        ("operations", &scope.operations),
    ] {
        if let Some(values) = values {
            dimensions.push(format!("{name}=[{}]", values.join(",")));
        }
    }
    if dimensions.is_empty() {
        "universe".to_owned()
    } else {
        dimensions.join(" ")
    }
}

fn display_exact_number(value: &ExactNumber) -> String {
    match value {
        ExactNumber::Integer(value) => format!("integer({value})"),
        ExactNumber::Rational {
            numerator,
            denominator,
        } => format!("rational({numerator},{denominator})"),
        ExactNumber::Decimal {
            coefficient,
            exponent,
        } => format!("decimal({coefficient},{exponent})"),
    }
}

impl Composition {
    fn add_source_rule(
        &mut self,
        document: &SourcePolicyDocument,
        rule: &PolicyRule,
    ) -> Result<()> {
        let source = SourceRuleIdentity {
            policy: document.symbol.clone(),
            rule: rule.id().to_owned(),
        };
        match rule {
            PolicyRule::Requirement { common, value } => {
                let mut value = value.clone();
                value.scope = normalized_scope(&value.scope);
                merge_additive(&mut self.requirements, common, value, source)
            }
            PolicyRule::Evidence { common, value } => {
                let mut value = value.clone();
                value.scope = normalized_scope(&value.scope);
                merge_additive(&mut self.evidence, common, value, source)
            }
            PolicyRule::Prohibition { common, value } => {
                let mut value = value.clone();
                value.scope = normalized_scope(&value.scope);
                merge_additive(&mut self.prohibitions, common, value, source)
            }
            PolicyRule::Capability { common, value } => {
                let mut value = value.clone();
                value.scope = normalized_scope(&value.scope);
                if let Some(existing) = self
                    .capabilities
                    .iter_mut()
                    .find(|candidate| candidate.effective.value.effect == value.effect)
                {
                    existing.effective.value.scope =
                        intersect_scope(&existing.effective.value.scope, &value.scope);
                    merge_governance(&mut existing.effective, common);
                    insert_source(&mut existing.sources, source);
                } else {
                    self.capabilities.push(composed(common, value, source));
                }
            }
            PolicyRule::Limit { common, value } => {
                let mut value = value.clone();
                value.scope = normalized_scope(&value.scope);
                if let Some(existing) = self.limits.iter_mut().find(|candidate| {
                    candidate.effective.value.dimension == value.dimension
                        && candidate.effective.value.unit == value.unit
                        && candidate.effective.value.scope == value.scope
                }) {
                    match exact_number_cmp(&value.maximum, &existing.effective.value.maximum) {
                        Ordering::Less => {
                            *existing = composed(common, value, source);
                        }
                        Ordering::Equal => {
                            if exact_number_encoding(&value.maximum)
                                < exact_number_encoding(&existing.effective.value.maximum)
                            {
                                existing.effective.value.maximum = value.maximum.clone();
                            }
                            merge_governance(&mut existing.effective, common);
                            insert_source(&mut existing.sources, source);
                        }
                        Ordering::Greater => {}
                    }
                } else {
                    self.limits.push(composed(common, value, source));
                }
            }
            PolicyRule::TypeMode { common, value } => match &mut self.type_mode {
                Some(existing) if value > &existing.effective.value => {
                    *existing = composed(common, *value, source);
                }
                Some(existing) if value == &existing.effective.value => {
                    merge_governance(&mut existing.effective, common);
                    insert_source(&mut existing.sources, source);
                }
                Some(_) => {}
                None => self.type_mode = Some(composed(common, *value, source)),
            },
        }
        Ok(())
    }

    fn finish(
        mut self,
        sources: Vec<&SourcePolicyDocument>,
        features: Vec<String>,
        algorithm: HashAlgorithm,
    ) -> Result<EffectivePolicyDocument> {
        sort_composed(&mut self.requirements, RequirementPolicyValue::to_value)?;
        sort_composed(&mut self.evidence, EvidencePolicyValue::to_value)?;
        sort_composed(&mut self.prohibitions, CapabilityPolicyValue::to_value)?;
        sort_composed(&mut self.capabilities, CapabilityPolicyValue::to_value)?;
        sort_composed(&mut self.limits, LimitPolicyValue::to_value)?;

        let mut provenance = Vec::new();
        append_provenance(
            &mut provenance,
            PolicyCategory::Requirement,
            &self.requirements,
        );
        append_provenance(&mut provenance, PolicyCategory::Evidence, &self.evidence);
        append_provenance(
            &mut provenance,
            PolicyCategory::Prohibition,
            &self.prohibitions,
        );
        append_provenance(
            &mut provenance,
            PolicyCategory::Capability,
            &self.capabilities,
        );
        append_provenance(&mut provenance, PolicyCategory::Limit, &self.limits);

        let mut type_mode = self.type_mode.unwrap_or(ComposedRule {
            effective: EffectiveRule {
                waivable: false,
                authorized_issuers: vec![],
                value: TypeMode::Dynamic,
            },
            sources: vec![],
        });
        if type_mode.effective.value == TypeMode::Dynamic {
            type_mode.effective.waivable = false;
            type_mode.effective.authorized_issuers.clear();
        }
        if !type_mode.sources.is_empty() {
            provenance.push(RuleProvenance {
                category: PolicyCategory::TypeMode,
                effective_rule: 0,
                sources: type_mode.sources.clone(),
            });
        }

        let effective = EffectivePolicy {
            requirements: take_effective(self.requirements),
            evidence: take_effective(self.evidence),
            prohibitions: take_effective(self.prohibitions),
            capabilities: take_effective(self.capabilities),
            limits: take_effective(self.limits),
            type_mode: type_mode.effective,
        };
        let source_layers = build_source_layers(&sources, algorithm)?;
        let mut document = EffectivePolicyDocument {
            header: PolicyHeader {
                features,
                semantic_id: None,
                artifact_id: None,
                provenance: None,
                authorization: None,
            },
            effective,
            source_layers,
            rule_provenance: provenance,
            waivers: None,
        };
        document.materialize_identities(algorithm)?;
        document.validate()?;
        Ok(document)
    }
}

fn composed<T: Clone>(
    common: &PolicyRuleCommon,
    value: T,
    source: SourceRuleIdentity,
) -> ComposedRule<T> {
    ComposedRule {
        effective: EffectiveRule {
            waivable: common.waivable,
            authorized_issuers: common.authorized_issuers.clone(),
            value,
        },
        sources: vec![source],
    }
}

fn merge_additive<T: Clone + Eq>(
    rules: &mut Vec<ComposedRule<T>>,
    common: &PolicyRuleCommon,
    value: T,
    source: SourceRuleIdentity,
) {
    if let Some(existing) = rules
        .iter_mut()
        .find(|candidate| candidate.effective.value == value)
    {
        merge_governance(&mut existing.effective, common);
        insert_source(&mut existing.sources, source);
    } else {
        rules.push(composed(common, value, source));
    }
}

fn merge_governance<T>(effective: &mut EffectiveRule<T>, common: &PolicyRuleCommon) {
    if !effective.waivable || !common.waivable {
        effective.waivable = false;
        effective.authorized_issuers.clear();
        return;
    }
    effective.authorized_issuers =
        intersect_sorted(&effective.authorized_issuers, &common.authorized_issuers);
    if effective.authorized_issuers.is_empty() {
        effective.waivable = false;
    }
}

fn insert_source(sources: &mut Vec<SourceRuleIdentity>, source: SourceRuleIdentity) {
    match sources.binary_search(&source) {
        Ok(_) => {}
        Err(index) => sources.insert(index, source),
    }
}

fn intersect_sorted<T: Ord + Clone>(left: &[T], right: &[T]) -> Vec<T> {
    left.iter()
        .filter(|value| right.binary_search(value).is_ok())
        .cloned()
        .collect()
}

fn scope_subset(left: &Option<PolicyScope>, right: &Option<PolicyScope>) -> bool {
    let left = left.as_ref();
    let right = right.as_ref();
    dimension_subset(
        left.and_then(|scope| scope.goals.as_ref()),
        right.and_then(|scope| scope.goals.as_ref()),
    ) && dimension_subset(
        left.and_then(|scope| scope.resources.as_ref()),
        right.and_then(|scope| scope.resources.as_ref()),
    ) && dimension_subset(
        left.and_then(|scope| scope.operations.as_ref()),
        right.and_then(|scope| scope.operations.as_ref()),
    )
}

fn dimension_subset(left: Option<&Vec<String>>, right: Option<&Vec<String>>) -> bool {
    match (left, right) {
        (_, None) => true,
        (None, Some(_)) => false,
        (Some(left), Some(right)) => left.iter().all(|value| right.binary_search(value).is_ok()),
    }
}

fn intersect_scope(left: &Option<PolicyScope>, right: &Option<PolicyScope>) -> Option<PolicyScope> {
    let scope = PolicyScope {
        goals: intersect_dimension(
            left.as_ref().and_then(|value| value.goals.as_ref()),
            right.as_ref().and_then(|value| value.goals.as_ref()),
        ),
        resources: intersect_dimension(
            left.as_ref().and_then(|value| value.resources.as_ref()),
            right.as_ref().and_then(|value| value.resources.as_ref()),
        ),
        operations: intersect_dimension(
            left.as_ref().and_then(|value| value.operations.as_ref()),
            right.as_ref().and_then(|value| value.operations.as_ref()),
        ),
    };
    if scope.goals.is_none() && scope.resources.is_none() && scope.operations.is_none() {
        None
    } else {
        Some(scope)
    }
}

fn normalized_scope(scope: &Option<PolicyScope>) -> Option<PolicyScope> {
    match scope {
        Some(scope)
            if scope.goals.is_none() && scope.resources.is_none() && scope.operations.is_none() =>
        {
            None
        }
        value => value.clone(),
    }
}

fn intersect_dimension(
    left: Option<&Vec<String>>,
    right: Option<&Vec<String>>,
) -> Option<Vec<String>> {
    match (left, right) {
        (None, None) => None,
        (Some(value), None) | (None, Some(value)) => Some(value.clone()),
        (Some(left), Some(right)) => Some(intersect_sorted(left, right)),
    }
}

fn scopes_overlap(left: &Option<PolicyScope>, right: &Option<PolicyScope>) -> bool {
    let left = left.as_ref();
    let right = right.as_ref();
    dimension_overlaps(
        left.and_then(|scope| scope.goals.as_ref()),
        right.and_then(|scope| scope.goals.as_ref()),
    ) && dimension_overlaps(
        left.and_then(|scope| scope.resources.as_ref()),
        right.and_then(|scope| scope.resources.as_ref()),
    ) && dimension_overlaps(
        left.and_then(|scope| scope.operations.as_ref()),
        right.and_then(|scope| scope.operations.as_ref()),
    )
}

fn dimension_overlaps(left: Option<&Vec<String>>, right: Option<&Vec<String>>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => left.iter().any(|value| right.binary_search(value).is_ok()),
        (Some(value), None) | (None, Some(value)) => !value.is_empty(),
        (None, None) => true,
    }
}

fn exact_number_cmp(left: &ExactNumber, right: &ExactNumber) -> Ordering {
    let (left_num, left_den, left_exp) = exact_parts(left);
    let (right_num, right_den, right_exp) = exact_parts(right);
    scaled_integer_cmp(
        left_num * right_den,
        left_exp,
        right_num * left_den,
        right_exp,
    )
}

fn exact_number_encoding(value: &ExactNumber) -> Vec<u8> {
    encode_deterministic(&value.to_value()).expect("validated exact number encodes")
}

fn exact_parts(value: &ExactNumber) -> (u128, u128, i64) {
    match value {
        ExactNumber::Integer(value) => (*value as u128, 1, 0),
        ExactNumber::Rational {
            numerator,
            denominator,
        } => (*numerator as u128, *denominator as u128, 0),
        ExactNumber::Decimal {
            coefficient,
            exponent,
        } => (*coefficient as u128, 1, *exponent),
    }
}

fn scaled_integer_cmp(left: u128, left_exp: i64, right: u128, right_exp: i64) -> Ordering {
    if left == 0 || right == 0 {
        return left.cmp(&right);
    }
    let left_text = left.to_string();
    let right_text = right.to_string();
    let left_magnitude = left_text.len() as i128 + left_exp as i128;
    let right_magnitude = right_text.len() as i128 + right_exp as i128;
    match left_magnitude.cmp(&right_magnitude) {
        Ordering::Equal => {
            let width = left_text.len().max(right_text.len());
            (0..width)
                .map(|index| left_text.as_bytes().get(index).copied().unwrap_or(b'0'))
                .cmp(
                    (0..width)
                        .map(|index| right_text.as_bytes().get(index).copied().unwrap_or(b'0')),
                )
        }
        ordering => ordering,
    }
}

fn sort_composed<T>(rules: &mut [ComposedRule<T>], value: fn(&T) -> Value) -> Result<()> {
    rules.sort_by_cached_key(|rule| {
        encode_deterministic(&effective_rule_value(&rule.effective, value))
            .expect("validated effective rule encodes")
    });
    Ok(())
}

fn append_provenance<T>(
    output: &mut Vec<RuleProvenance>,
    category: PolicyCategory,
    rules: &[ComposedRule<T>],
) {
    output.extend(
        rules
            .iter()
            .enumerate()
            .map(|(effective_rule, rule)| RuleProvenance {
                category,
                effective_rule,
                sources: rule.sources.clone(),
            }),
    );
}

fn take_effective<T>(rules: Vec<ComposedRule<T>>) -> Vec<EffectiveRule<T>> {
    rules.into_iter().map(|rule| rule.effective).collect()
}

fn build_source_layers(
    sources: &[&SourcePolicyDocument],
    algorithm: HashAlgorithm,
) -> Result<Vec<PolicySourceLayer>> {
    let mut layers = BTreeMap::<PolicyLayer, Vec<PolicySource>>::new();
    for source in sources {
        let bytes = PolicyDocument::Source((*source).clone()).to_cbor(false)?;
        layers.entry(source.layer).or_default().push(PolicySource {
            symbol: source.symbol.clone(),
            artifact: ArtifactReference {
                media_type: "application/cbor".to_owned(),
                size: bytes.len() as u64,
                digests: vec![algorithm.hash(&bytes)],
                locations: None,
            },
        });
    }
    Ok(layers
        .into_iter()
        .map(|(layer, mut policies)| {
            policies.sort_by(|left, right| left.symbol.cmp(&right.symbol));
            PolicySourceLayer { layer, policies }
        })
        .collect())
}

fn composition_invalid(message: impl Into<String>) -> Diagnostic {
    Diagnostic::plain(INVALID_COMPOSITION_TOPOLOGY, message)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PolicyDocument {
    Source(SourcePolicyDocument),
    Effective(EffectivePolicyDocument),
}

impl PolicyDocument {
    pub fn from_value(value: &Value) -> Result<Self> {
        let entries = map_entries(value, "policy document")?;
        let form = required_text(entries, "form", "policy document")?;
        match form.as_str() {
            "source" => Ok(Self::Source(SourcePolicyDocument::from_entries(entries)?)),
            "effective" => Ok(Self::Effective(EffectivePolicyDocument::from_entries(
                entries,
            )?)),
            _ => Err(invalid("policy form must be source or effective")),
        }
    }

    pub fn from_cbor(bytes: &[u8]) -> Result<Self> {
        let value = decode_deterministic(bytes)?;
        Self::from_value(&value)
    }

    pub fn to_value(&self, include_artifact_id: bool) -> Value {
        match self {
            Self::Source(document) => document.to_value(include_artifact_id),
            Self::Effective(document) => document.to_value(include_artifact_id),
        }
    }

    pub fn to_cbor(&self, include_artifact_id: bool) -> Result<Vec<u8>> {
        self.validate()?;
        encode_deterministic(&self.to_value(include_artifact_id))
    }

    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Source(document) => document.validate(),
            Self::Effective(document) => document.validate(),
        }
    }
}

fn parse_effective_array<T>(
    value: &Value,
    context: &str,
    parse: fn(&Value) -> Result<T>,
) -> Result<Vec<EffectiveRule<T>>> {
    array_values(value, context)?
        .iter()
        .map(|value| parse_effective_rule(value, context, parse))
        .collect()
}

fn parse_effective_rule<T>(
    value: &Value,
    context: &str,
    parse: fn(&Value) -> Result<T>,
) -> Result<EffectiveRule<T>> {
    let entries = map_entries(value, context)?;
    ensure_fields(
        entries,
        &["waivable", "authorized_issuers", "value"],
        context,
    )?;
    let (waivable, authorized_issuers) = EffectiveRule::<T>::common_from_entries(entries, context)?;
    Ok(EffectiveRule {
        waivable,
        authorized_issuers,
        value: parse(required(entries, "value", context)?)?,
    })
}

fn effective_rule_value<T>(rule: &EffectiveRule<T>, value: fn(&T) -> Value) -> Value {
    let mut entries = rule.common_entries();
    entries.push(("value".to_owned(), value(&rule.value)));
    Value::owned_map(entries)
}

fn validate_effective_rules<T>(
    rules: &[EffectiveRule<T>],
    validate: fn(&T) -> Result<()>,
    value: fn(&T) -> Value,
    context: &str,
) -> Result<()> {
    let mut values = Vec::with_capacity(rules.len());
    for rule in rules {
        rule.validate_common()?;
        validate(&rule.value)?;
        values.push(effective_rule_value(rule, value));
    }
    validate_values_sorted_unique(&values, context)
}

fn validate_values_sorted_unique(values: &[Value], context: &str) -> Result<()> {
    let encoded = values
        .iter()
        .map(encode_deterministic)
        .collect::<Result<Vec<_>>>()?;
    if encoded.windows(2).all(|pair| pair[0] < pair[1]) {
        Ok(())
    } else {
        Err(invalid(format!(
            "{context} must be in unique deterministic order"
        )))
    }
}

fn push_scope_and_parameters(
    entries: &mut Vec<(String, Value)>,
    scope: &Option<PolicyScope>,
    parameters: &Option<Value>,
) {
    if let Some(scope) = scope {
        entries.push(("scope".to_owned(), scope.to_value()));
    }
    if let Some(parameters) = parameters {
        entries.push(("parameters".to_owned(), parameters.clone()));
    }
}

fn validate_artifact_id(
    header: &PolicyHeader,
    value_without_artifact_id: &Value,
    message: &str,
) -> Result<()> {
    let Some(artifact_id) = &header.artifact_id else {
        return Ok(());
    };
    let algorithm = HashAlgorithm::from_id(&artifact_id.algorithm).map_err(policy_error)?;
    if artifact_hash_with(value_without_artifact_id, algorithm)? == *artifact_id {
        Ok(())
    } else {
        Err(invalid(message))
    }
}

fn parse_hash_id(value: &Value) -> Result<HashId> {
    let entries = map_entries(value, "policy hash ID")?;
    ensure_fields(entries, &["algorithm", "digest"], "policy hash ID")?;
    let hash = HashId {
        algorithm: required_symbol(entries, "algorithm", "policy hash ID")?,
        digest: match required(entries, "digest", "policy hash ID")? {
            Value::Bytes(bytes) => bytes.clone(),
            _ => return Err(invalid("policy hash digest must be bytes")),
        },
    };
    hash.validate().map_err(policy_error)?;
    Ok(hash)
}

fn parse_evidence_classes(value: &Value) -> Result<Vec<String>> {
    let values = parse_text_array(value, "evidence classes", false)?;
    validate_normalized_evidence_classes(&values)?;
    Ok(values)
}

fn validate_normalized_evidence_classes(values: &[String]) -> Result<()> {
    if values.is_empty()
        || values.iter().any(|value| !is_evidence_class(value))
        || !values.windows(2).all(|pair| pair[0] < pair[1])
    {
        Err(invalid(
            "evidence classes must be a non-empty sorted set of registered classes",
        ))
    } else {
        Ok(())
    }
}

fn is_evidence_class(value: &str) -> bool {
    matches!(
        value,
        "formal"
            | "static"
            | "empirical"
            | "statistical"
            | "model-judged"
            | "human-approved"
            | "unresolved"
    ) || is_symbol(value)
}

fn parse_symbol_array(value: &Value, context: &str, allow_empty: bool) -> Result<Vec<String>> {
    let values = parse_text_array(value, context, allow_empty)?;
    validate_normalized_symbols(&values, context, allow_empty)?;
    Ok(values)
}

fn validate_normalized_symbols(values: &[String], context: &str, allow_empty: bool) -> Result<()> {
    if (!allow_empty && values.is_empty())
        || values.iter().any(|value| !is_symbol(value))
        || !values.windows(2).all(|pair| pair[0] < pair[1])
    {
        Err(invalid(format!(
            "{context} must be a {}sorted set of symbol IDs",
            if allow_empty { "" } else { "non-empty " }
        )))
    } else {
        Ok(())
    }
}

fn parse_text_array(value: &Value, context: &str, allow_empty: bool) -> Result<Vec<String>> {
    let values = array_values(value, context)?;
    let parsed = values
        .iter()
        .map(|value| text_value(value, context).map(str::to_owned))
        .collect::<Result<Vec<_>>>()?;
    validate_normalized_text(&parsed, context, allow_empty)?;
    Ok(parsed)
}

fn validate_normalized_text(values: &[String], context: &str, allow_empty: bool) -> Result<()> {
    if (!allow_empty && values.is_empty()) || !values.windows(2).all(|pair| pair[0] < pair[1]) {
        Err(invalid(format!(
            "{context} must be a {}sorted set",
            if allow_empty { "" } else { "non-empty " }
        )))
    } else {
        Ok(())
    }
}

fn required_symbol(entries: &[(String, Value)], key: &str, context: &str) -> Result<String> {
    symbol_value(
        required(entries, key, context)?,
        &format!("{context} {key}"),
    )
}

fn symbol_value(value: &Value, context: &str) -> Result<String> {
    let value = text_value(value, context)?;
    validate_symbol(value, context)?;
    Ok(value.to_owned())
}

fn validate_symbol(value: &str, context: &str) -> Result<()> {
    if is_symbol(value) {
        Ok(())
    } else {
        Err(invalid(format!("{context} must be a symbol-id")))
    }
}

fn ensure_fields(entries: &[(String, Value)], allowed: &[&str], context: &str) -> Result<()> {
    let mut seen = HashSet::new();
    for (key, _) in entries {
        if !seen.insert(key.as_str()) {
            return Err(invalid(format!("duplicate {context} field {key:?}")));
        }
        if !allowed.contains(&key.as_str()) {
            return Err(invalid(format!("unknown {context} field {key:?}")));
        }
    }
    Ok(())
}

fn map_entries<'a>(value: &'a Value, context: &str) -> Result<&'a [(String, Value)]> {
    match value {
        Value::Map(entries) => Ok(entries),
        _ => Err(invalid(format!("{context} must be a map"))),
    }
}

fn array_values<'a>(value: &'a Value, context: &str) -> Result<&'a [Value]> {
    match value {
        Value::Array(values) => Ok(values),
        _ => Err(invalid(format!("{context} must be an array"))),
    }
}

fn required<'a>(entries: &'a [(String, Value)], key: &str, context: &str) -> Result<&'a Value> {
    optional(entries, key).ok_or_else(|| invalid(format!("{context} requires {key}")))
}

fn optional<'a>(entries: &'a [(String, Value)], key: &str) -> Option<&'a Value> {
    entries
        .iter()
        .find_map(|(candidate, value)| (candidate == key).then_some(value))
}

fn required_text(entries: &[(String, Value)], key: &str, context: &str) -> Result<String> {
    Ok(text_value(
        required(entries, key, context)?,
        &format!("{context} {key}"),
    )?
    .to_owned())
}

fn require_exact_text(
    entries: &[(String, Value)],
    key: &str,
    expected: &str,
    context: &str,
) -> Result<()> {
    if text_value(
        required(entries, key, context)?,
        &format!("{context} {key}"),
    )? == expected
    {
        Ok(())
    } else {
        Err(invalid(format!("{context} {key} must equal {expected:?}")))
    }
}

fn text_value<'a>(value: &'a Value, context: &str) -> Result<&'a str> {
    match value {
        Value::Text(value) => Ok(value),
        _ => Err(invalid(format!("{context} must be text"))),
    }
}

fn policy_timestamp(value: &Value, context: &str) -> Result<String> {
    let Value::Tag(0, item) = value else {
        return Err(invalid(format!(
            "{context} must be a tag-0 RFC 3339 timestamp"
        )));
    };
    let timestamp = text_value(item, context)?.to_owned();
    validate_policy_timestamp(&timestamp, context)?;
    Ok(timestamp)
}

fn validate_policy_timestamp(value: &str, context: &str) -> Result<()> {
    let bytes = value.as_bytes();
    let punctuation = [
        (4, b'-'),
        (7, b'-'),
        (10, b'T'),
        (13, b':'),
        (16, b':'),
        (19, b'Z'),
    ];
    let valid_shape = bytes.len() == 20
        && punctuation
            .iter()
            .all(|(index, expected)| bytes[*index] == *expected)
        && bytes.iter().enumerate().all(|(index, byte)| {
            punctuation.iter().any(|(position, _)| *position == index) || byte.is_ascii_digit()
        });
    if !valid_shape {
        return Err(invalid(format!(
            "{context} must use canonical UTC second precision"
        )));
    }
    let number = |range: std::ops::Range<usize>| -> u32 {
        value[range]
            .parse()
            .expect("timestamp shape contains only ASCII digits")
    };
    let year = number(0..4);
    let month = number(5..7);
    let day = number(8..10);
    let hour = number(11..13);
    let minute = number(14..16);
    let second = number(17..19);
    let leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => 0,
    };
    if day == 0 || day > days || hour > 23 || minute > 59 || second > 60 {
        return Err(invalid(format!("{context} is not a valid UTC date-time")));
    }
    Ok(())
}

fn required_bool(entries: &[(String, Value)], key: &str, context: &str) -> Result<bool> {
    match required(entries, key, context)? {
        Value::Bool(value) => Ok(*value),
        _ => Err(invalid(format!("{context} {key} must be a boolean"))),
    }
}

fn positive_u64(value: &Value, message: &str) -> Result<u64> {
    match value {
        Value::Integer(value) if *value > 0 => Ok(*value as u64),
        _ => Err(invalid(message)),
    }
}

fn non_negative_u64(value: &Value, message: &str) -> Result<u64> {
    match value {
        Value::Integer(value) if *value >= 0 => Ok(*value as u64),
        _ => Err(invalid(message)),
    }
}

fn weakening_value(category: &str, operation: &str, value: Value) -> Value {
    Value::map([
        ("category", text(category)),
        ("operation", text(operation)),
        ("value", value),
    ])
}

fn from_to_weakening_value(category: &str, operation: &str, from: Value, to: Value) -> Value {
    Value::map([
        ("category", text(category)),
        ("operation", text(operation)),
        ("from", from),
        ("to", to),
    ])
}

fn text(value: &str) -> Value {
    Value::Text(value.to_owned())
}

fn policy_error(diagnostic: Diagnostic) -> Diagnostic {
    invalid(diagnostic.message)
}

fn invalid(message: impl Into<String>) -> Diagnostic {
    Diagnostic::plain(INVALID_POLICY, message)
}

fn waiver_invalid(message: impl Into<String>) -> Diagnostic {
    Diagnostic::plain(INVALID_WAIVER, message)
}

fn waiver_target(message: impl Into<String>) -> Diagnostic {
    Diagnostic::plain(WAIVER_TARGET_MISMATCH, message)
}

fn waiver_change(message: impl Into<String>) -> Diagnostic {
    Diagnostic::plain(WAIVER_CHANGE_MISMATCH, message)
}

fn waiver_time(message: impl Into<String>) -> Diagnostic {
    Diagnostic::plain(WAIVER_INACTIVE, message)
}

fn waiver_authority(message: impl Into<String>) -> Diagnostic {
    Diagnostic::plain(WAIVER_UNAUTHORIZED, message)
}
