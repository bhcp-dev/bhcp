//! Strongly typed v0 source and effective policy documents.

use std::cmp::Ordering;
use std::collections::HashSet;

use crate::cbor::{decode_deterministic, encode_deterministic};
use crate::diagnostic::{Diagnostic, Result};
use crate::hash::{HashAlgorithm, artifact_hash_with, hash_value};
use crate::model::{HashId, is_symbol};
use crate::value::Value;

const INVALID_POLICY: &str = "BHCP8001";

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

    fn as_str(self) -> &'static str {
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

    fn to_value(&self) -> Value {
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
    fn from_value(value: &Value) -> Result<Self> {
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
pub struct EffectivePolicyDocument {
    pub header: PolicyHeader,
    pub effective: EffectivePolicy,
    pub source_layers: Vec<PolicySourceLayer>,
    pub rule_provenance: Vec<RuleProvenance>,
    pub waivers: Option<Vec<ArtifactReference>>,
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
                        .map(ArtifactReference::from_value)
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
                Value::Array(waivers.iter().map(ArtifactReference::to_value).collect()),
            ));
        }
        Value::owned_map(entries)
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
                    .map(ArtifactReference::to_value)
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
        if hash_value(&self.effective.to_value(), algorithm)? != *semantic_id {
            return Err(invalid(
                "effective policy semantic_id does not match effective meaning",
            ));
        }
        validate_artifact_id(
            &self.header,
            &self.to_value(false),
            "effective policy artifact_id does not match document",
        )
    }
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

fn text(value: &str) -> Value {
    Value::Text(value.to_owned())
}

fn policy_error(diagnostic: Diagnostic) -> Diagnostic {
    invalid(diagnostic.message)
}

fn invalid(message: impl Into<String>) -> Diagnostic {
    Diagnostic::plain(INVALID_POLICY, message)
}
