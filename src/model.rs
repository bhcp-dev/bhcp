use std::collections::{BTreeMap, BTreeSet, HashSet};

use crate::diagnostic::{Diagnostic, Result};
use crate::expression::CheckedExpression;
use crate::hash::{HashAlgorithm, SHA3_512};
use crate::kernel::KernelNetwork;
use crate::policy::TypeMode;
use crate::typecheck::{CheckedType, CheckedTypeDefinition};
use crate::value::Value;

pub const BASE_FEATURE: &str = "bhcp/feature.canonical-simple-goal@0";

pub fn features_for(algorithm: crate::hash::HashAlgorithm) -> Vec<String> {
    vec![
        BASE_FEATURE.to_owned(),
        format!(
            "bhcp/feature.hash-{}@0",
            match algorithm {
                crate::hash::HashAlgorithm::Sha3_512 => "sha3-512",
            }
        ),
    ]
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HashId {
    pub algorithm: String,
    pub digest: Vec<u8>,
}

impl HashId {
    pub fn to_value(&self) -> Value {
        Value::map([
            ("algorithm", Value::Text(self.algorithm.clone())),
            ("digest", Value::Bytes(self.digest.clone())),
        ])
    }
    pub fn validate(&self) -> Result<()> {
        match self.algorithm.as_str() {
            SHA3_512 if self.digest.len() == 64 => Ok(()),
            SHA3_512 => Err(Diagnostic::plain(
                "BHCP4001",
                "hash digest length does not match its algorithm",
            )),
            _ if is_symbol(&self.algorithm) => Ok(()),
            _ => Err(Diagnostic::plain(
                "BHCP4001",
                "hash algorithm must be a symbol-id",
            )),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContentReference {
    pub media_type: String,
    pub size: usize,
    pub digests: Vec<HashId>,
}

impl ContentReference {
    pub fn from_bytes(
        media_type: impl Into<String>,
        bytes: &[u8],
        algorithm: HashAlgorithm,
    ) -> Self {
        Self {
            media_type: media_type.into(),
            size: bytes.len(),
            digests: vec![algorithm.hash(bytes)],
        }
    }

    pub fn to_value(&self) -> Value {
        Value::map([
            ("media_type", Value::Text(self.media_type.clone())),
            ("size", Value::Integer(self.size as i128)),
            (
                "digests",
                Value::Array(self.digests.iter().map(HashId::to_value).collect()),
            ),
        ])
    }
    pub fn validate(&self) -> Result<()> {
        if self.digests.is_empty() {
            return Err(Diagnostic::plain(
                "BHCP4001",
                "content reference requires at least one digest",
            ));
        }
        for digest in &self.digests {
            digest.validate()?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Point {
    pub byte: usize,
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TokenSpan {
    pub source: ContentReference,
    pub start: Point,
    pub end: Point,
}

impl TokenSpan {
    fn to_value(&self) -> Value {
        Value::map([
            ("source", self.source.to_value()),
            ("start_byte", Value::Integer(self.start.byte as i128)),
            ("end_byte", Value::Integer(self.end.byte as i128)),
            ("start_line", Value::Integer(self.start.line as i128)),
            ("start_column", Value::Integer(self.start.column as i128)),
            ("end_line", Value::Integer(self.end.line as i128)),
            ("end_column", Value::Integer(self.end.column as i128)),
        ])
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AstNode {
    pub id: String,
    pub kind: String,
    pub token: Option<String>,
    pub children: Vec<AstNode>,
    pub span: TokenSpan,
    pub attributes: Vec<(String, Value)>,
}

impl AstNode {
    fn to_value(&self) -> Value {
        let mut entries = vec![
            ("id".to_owned(), Value::Text(self.id.clone())),
            ("kind".to_owned(), Value::Text(self.kind.clone())),
            ("span".to_owned(), self.span.to_value()),
        ];
        if let Some(token) = &self.token {
            entries.push(("token".to_owned(), Value::Text(token.clone())));
        }
        if !self.children.is_empty() {
            entries.push((
                "children".to_owned(),
                Value::Array(self.children.iter().map(AstNode::to_value).collect()),
            ));
        }
        if !self.attributes.is_empty() {
            entries.push((
                "attributes".to_owned(),
                Value::owned_map(self.attributes.clone()),
            ));
        }
        Value::owned_map(entries)
    }
    fn validate(&self, ids: &mut HashSet<String>) -> Result<()> {
        add_id(&self.id, ids)?;
        self.span.source.validate()?;
        for child in &self.children {
            child.validate(ids)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalAstDocument {
    pub features: Vec<String>,
    pub profile: String,
    pub root: AstNode,
    pub source: ContentReference,
    pub artifact_id: Option<HashId>,
}

impl CanonicalAstDocument {
    pub fn to_value(&self, include_artifact_id: bool) -> Value {
        let mut entries = header_entries(&self.features);
        entries.extend([
            ("kind".to_owned(), Value::Text("canonical-ast".to_owned())),
            ("profile".to_owned(), Value::Text(self.profile.clone())),
            ("root".to_owned(), self.root.to_value()),
            ("source".to_owned(), self.source.to_value()),
        ]);
        if include_artifact_id && let Some(hash) = &self.artifact_id {
            entries.push(("artifact_id".to_owned(), hash.to_value()));
        }
        Value::owned_map(entries)
    }
    pub fn validate(&self) -> Result<()> {
        validate_features(&self.features)?;
        if self.profile.is_empty() {
            return Err(Diagnostic::plain(
                "BHCP4001",
                "canonical AST profile must be an exact non-empty symbol",
            ));
        }
        self.source.validate()?;
        self.root.validate(&mut HashSet::new())?;
        if let Some(hash) = &self.artifact_id {
            hash.validate()?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BhcpType {
    Primitive(&'static str),
    ExactNumber(&'static str),
    Record(Vec<FieldType>),
    Variant(Vec<VariantCaseType>),
    List(Box<BhcpType>),
    Nominal(String, Vec<BhcpType>),
    Option(Box<BhcpType>),
    Evidence(Vec<String>),
    Verdict(Box<BhcpType>),
    ExecutionResult(Box<BhcpType>),
    Reduction(Box<BhcpType>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FieldType {
    pub name: String,
    pub value_type: BhcpType,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VariantCaseType {
    pub tag: String,
    pub payload: Vec<BhcpType>,
}

impl BhcpType {
    pub fn to_value(&self) -> Value {
        match self {
            Self::Primitive(name) => Value::Array(vec![
                Value::Text("primitive".to_owned()),
                Value::Text((*name).to_owned()),
            ]),
            Self::ExactNumber(name) => Value::Array(vec![
                Value::Text("exact-number".to_owned()),
                Value::Text((*name).to_owned()),
            ]),
            Self::Record(fields) => Value::Array(vec![
                Value::Text("record".to_owned()),
                Value::Bool(false),
                Value::Array(
                    fields
                        .iter()
                        .map(|field| {
                            Value::Array(vec![
                                Value::Text(field.name.clone()),
                                field.value_type.to_value(),
                                Value::Bool(false),
                            ])
                        })
                        .collect(),
                ),
            ]),
            Self::Variant(cases) => Value::Array(vec![
                Value::Text("variant".to_owned()),
                Value::Array(
                    cases
                        .iter()
                        .map(|case| {
                            Value::Array(vec![
                                Value::Text(case.tag.clone()),
                                Value::Array(case.payload.iter().map(BhcpType::to_value).collect()),
                            ])
                        })
                        .collect(),
                ),
            ]),
            Self::List(value) => {
                Value::Array(vec![Value::Text("list".to_owned()), value.to_value()])
            }
            Self::Nominal(symbol, arguments) => Value::Array(vec![
                Value::Text("nominal".to_owned()),
                Value::Text(symbol.clone()),
                Value::Array(arguments.iter().map(BhcpType::to_value).collect()),
            ]),
            Self::Option(value) => {
                Value::Array(vec![Value::Text("option".to_owned()), value.to_value()])
            }
            Self::Evidence(classes) => Value::Array(vec![
                Value::Text("evidence".to_owned()),
                Value::Array(classes.iter().cloned().map(Value::Text).collect()),
            ]),
            Self::Verdict(output) => {
                Value::Array(vec![Value::Text("verdict".to_owned()), output.to_value()])
            }
            Self::ExecutionResult(output) => Value::Array(vec![
                Value::Text("execution-result".to_owned()),
                output.to_value(),
            ]),
            Self::Reduction(output) => {
                Value::Array(vec![Value::Text("reduction".to_owned()), output.to_value()])
            }
        }
    }

    pub fn accepts(&self, value: &Value) -> bool {
        match (value, self) {
            (Value::Bool(_), Self::Primitive("Bool"))
            | (Value::Text(_), Self::Primitive("Text" | "Timestamp" | "Duration"))
            | (Value::Bytes(_), Self::Primitive("Bytes")) => true,
            (Value::Array(value), Self::Primitive("Unit")) => {
                value == &[Value::Text("unit".to_owned())]
            }
            (Value::Array(value), Self::ExactNumber("Integer")) => matches!(
                value.as_slice(),
                [Value::Text(kind), Value::Integer(_)] if kind == "integer"
            ),
            (Value::Array(value), Self::ExactNumber("Rational")) => matches!(
                value.as_slice(),
                [Value::Text(kind), Value::Integer(_), Value::Integer(denominator)]
                    if kind == "rational" && *denominator > 0
            ),
            (Value::Array(value), Self::ExactNumber("Decimal")) => matches!(
                value.as_slice(),
                [Value::Text(kind), Value::Integer(_), Value::Integer(_)] if kind == "decimal"
            ),
            (Value::Array(values), Self::List(element)) => {
                values.iter().all(|value| element.accepts(value))
            }
            (Value::Map(entries), Self::Record(fields)) => {
                entries.len() == fields.len()
                    && fields.iter().all(|field| {
                        entries.iter().any(|(name, value)| {
                            name == &field.name && field.value_type.accepts(value)
                        })
                    })
            }
            (Value::Array(parts), Self::Variant(cases)) => {
                let [Value::Text(kind), Value::Text(tag), payload] = parts.as_slice() else {
                    return false;
                };
                if kind != "variant" {
                    return false;
                }
                let Some(case) = cases.iter().find(|case| case.tag == *tag) else {
                    return false;
                };
                match case.payload.as_slice() {
                    [] => payload == &Value::Array(vec![Value::Text("unit".to_owned())]),
                    [value_type] => value_type.accepts(payload),
                    value_types => match payload {
                        Value::Array(values) if values.len() == value_types.len() => value_types
                            .iter()
                            .zip(values)
                            .all(|(value_type, value)| value_type.accepts(value)),
                        _ => false,
                    },
                }
            }
            _ => false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Binding {
    pub id: String,
    pub name: String,
    pub value_type: BhcpType,
}

impl Binding {
    fn to_value(&self) -> Value {
        Value::map([
            ("id", Value::Text(self.id.clone())),
            ("name", Value::Text(self.name.clone())),
            ("type", self.value_type.to_value()),
        ])
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Expression {
    pub id: String,
    pub value_type: BhcpType,
    pub form: ExpressionForm,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExpressionForm {
    Literal(Value),
    Reference(String),
    Unary(String, Box<Expression>),
    Binary(String, Box<Expression>, Box<Expression>),
    If(Box<Expression>, Box<Expression>, Box<Expression>),
    Call(String, Vec<Expression>),
}

impl Expression {
    pub(crate) fn to_value(&self) -> Value {
        let form = match &self.form {
            ExpressionForm::Literal(value) => {
                Value::Array(vec![Value::Text("literal".to_owned()), value.clone()])
            }
            ExpressionForm::Reference(id) => Value::Array(vec![
                Value::Text("reference".to_owned()),
                Value::Text(id.clone()),
            ]),
            ExpressionForm::Unary(operator, operand) => Value::Array(vec![
                Value::Text("unary".to_owned()),
                Value::Text(operator.clone()),
                operand.to_value(),
            ]),
            ExpressionForm::Binary(operator, left, right) => Value::Array(vec![
                Value::Text("binary".to_owned()),
                Value::Text(operator.clone()),
                left.to_value(),
                right.to_value(),
            ]),
            ExpressionForm::If(condition, consequent, alternative) => Value::Array(vec![
                Value::Text("if".to_owned()),
                condition.to_value(),
                consequent.to_value(),
                alternative.to_value(),
            ]),
            ExpressionForm::Call(function, arguments) => Value::Array(vec![
                Value::Text("call".to_owned()),
                Value::Text(function.clone()),
                Value::Array(arguments.iter().map(Expression::to_value).collect()),
            ]),
        };
        Value::map([
            ("id", Value::Text(self.id.clone())),
            ("type", self.value_type.to_value()),
            ("form", form),
        ])
    }
    fn validate(
        &self,
        ids: &mut HashSet<String>,
        references: &mut Vec<String>,
        calls: &mut Vec<String>,
    ) -> Result<()> {
        add_id(&self.id, ids)?;
        match &self.form {
            ExpressionForm::Literal(_) => {}
            ExpressionForm::Reference(id) => references.push(id.clone()),
            ExpressionForm::Unary(_, operand) => operand.validate(ids, references, calls)?,
            ExpressionForm::Binary(_, left, right) => {
                left.validate(ids, references, calls)?;
                right.validate(ids, references, calls)?;
            }
            ExpressionForm::If(condition, consequent, alternative) => {
                condition.validate(ids, references, calls)?;
                consequent.validate(ids, references, calls)?;
                alternative.validate(ids, references, calls)?;
            }
            ExpressionForm::Call(function, arguments) => {
                if !is_symbol(function) {
                    return Err(Diagnostic::plain(
                        "BHCP4001",
                        "called function is not a symbol-id",
                    ));
                }
                calls.push(function.clone());
                for argument in arguments {
                    argument.validate(ids, references, calls)?;
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Effect {
    pub id: String,
    pub resource: Option<String>,
    pub parameters: Vec<Value>,
}

impl Effect {
    fn to_value(&self) -> Value {
        let mut entries = vec![("id".to_owned(), Value::Text(self.id.clone()))];
        if let Some(resource) = &self.resource {
            entries.push(("resource".to_owned(), Value::Text(resource.clone())));
        }
        if !self.parameters.is_empty() {
            entries.push((
                "parameters".to_owned(),
                Value::Array(self.parameters.clone()),
            ));
        }
        Value::owned_map(entries)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifierBinding {
    pub verifier: String,
    pub input: BhcpType,
    pub output: BhcpType,
    pub trust: Vec<String>,
}

impl VerifierBinding {
    fn to_value(&self) -> Value {
        let mut entries = vec![
            ("verifier".to_owned(), Value::Text(self.verifier.clone())),
            ("input".to_owned(), self.input.to_value()),
            ("output".to_owned(), self.output.to_value()),
        ];
        if !self.trust.is_empty() {
            entries.push((
                "trust".to_owned(),
                Value::Array(self.trust.iter().cloned().map(Value::Text).collect()),
            ));
        }
        Value::owned_map(entries)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Clause {
    pub id: String,
    pub label: Option<String>,
    pub kind: ClauseKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClauseKind {
    Fact {
        kind: &'static str,
        binding: Binding,
    },
    Contract {
        kind: &'static str,
        dimension: Option<String>,
        condition: Expression,
    },
    Authority {
        kind: &'static str,
        effects: Vec<Effect>,
    },
    Preference {
        priority: i64,
        objective: Expression,
    },
    Verify {
        binding: VerifierBinding,
        obligations: Vec<String>,
    },
}

impl Clause {
    fn to_value(&self, include_label: bool) -> Value {
        let mut entries = vec![("id".to_owned(), Value::Text(self.id.clone()))];
        if include_label && let Some(label) = &self.label {
            entries.push(("label".to_owned(), Value::Text(label.clone())));
        }
        match &self.kind {
            ClauseKind::Fact { kind, binding } => {
                entries.push(("kind".to_owned(), Value::Text((*kind).to_owned())));
                entries.push(("binding".to_owned(), binding.to_value()));
            }
            ClauseKind::Contract {
                kind,
                dimension,
                condition,
            } => {
                entries.push(("kind".to_owned(), Value::Text((*kind).to_owned())));
                if let Some(dimension) = dimension {
                    entries.push(("dimension".to_owned(), Value::Text(dimension.clone())));
                }
                entries.push(("condition".to_owned(), condition.to_value()));
            }
            ClauseKind::Authority { kind, effects } => {
                entries.push(("kind".to_owned(), Value::Text((*kind).to_owned())));
                entries.push((
                    "effects".to_owned(),
                    Value::Array(effects.iter().map(Effect::to_value).collect()),
                ));
            }
            ClauseKind::Preference {
                priority,
                objective,
            } => {
                entries.push(("kind".to_owned(), Value::Text("prefer".to_owned())));
                entries.push(("priority".to_owned(), Value::Integer(i128::from(*priority))));
                entries.push(("objective".to_owned(), objective.to_value()));
            }
            ClauseKind::Verify {
                binding,
                obligations,
            } => {
                entries.push(("kind".to_owned(), Value::Text("verify".to_owned())));
                entries.push(("binding".to_owned(), binding.to_value()));
                if !obligations.is_empty() {
                    entries.push((
                        "obligations".to_owned(),
                        Value::Array(obligations.iter().cloned().map(Value::Text).collect()),
                    ));
                }
            }
        }
        Value::owned_map(entries)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectivePolicyReference {
    pub semantic_id: HashId,
    pub artifact_id: HashId,
}

impl EffectivePolicyReference {
    fn to_value(&self, include_artifact_id: bool) -> Value {
        let mut entries = vec![("semantic_id".to_owned(), self.semantic_id.to_value())];
        if include_artifact_id {
            entries.push(("artifact_id".to_owned(), self.artifact_id.to_value()));
        }
        Value::owned_map(entries)
    }

    fn validate(&self) -> Result<()> {
        self.semantic_id.validate()?;
        self.artifact_id.validate()?;
        if self.semantic_id.algorithm != self.artifact_id.algorithm {
            return Err(Diagnostic::plain(
                "BHCP4001",
                "effective policy identities must use the same algorithm",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyDecision {
    pub type_mode: String,
    pub requirements: Vec<usize>,
    pub evidence: Vec<usize>,
    pub prohibitions: Vec<usize>,
    pub capabilities: Vec<usize>,
    pub limits: Vec<usize>,
}

impl PolicyDecision {
    fn to_value(&self) -> Value {
        Value::map([
            ("type_mode", Value::Text(self.type_mode.clone())),
            ("requirements", policy_indices(&self.requirements)),
            ("evidence", policy_indices(&self.evidence)),
            ("prohibitions", policy_indices(&self.prohibitions)),
            ("capabilities", policy_indices(&self.capabilities)),
            ("limits", policy_indices(&self.limits)),
        ])
    }

    fn validate(&self) -> Result<()> {
        if !matches!(
            self.type_mode.as_str(),
            "dynamic" | "gradual" | "infer-strict" | "strict"
        ) || [
            &self.requirements,
            &self.evidence,
            &self.prohibitions,
            &self.capabilities,
            &self.limits,
        ]
        .iter()
        .any(|values| {
            values.iter().any(|index| *index > i64::MAX as usize)
                || !values.windows(2).all(|pair| pair[0] < pair[1])
        }) {
            return Err(Diagnostic::plain(
                "BHCP4001",
                "goal policy decision is not normalized",
            ));
        }
        Ok(())
    }
}

fn policy_indices(indices: &[usize]) -> Value {
    Value::Array(
        indices
            .iter()
            .map(|index| Value::Integer(*index as i128))
            .collect(),
    )
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GoalDefinition {
    pub id: String,
    pub symbol: String,
    pub type_mode: TypeMode,
    pub input: BhcpType,
    pub output: BhcpType,
    pub evidence: BhcpType,
    pub clauses: Vec<Clause>,
    pub policy_decision: Option<PolicyDecision>,
    pub body: Option<KernelNetwork>,
}

impl GoalDefinition {
    fn to_value(&self, include_labels: bool) -> Value {
        let mut entries = vec![
            ("id".to_owned(), Value::Text(self.id.clone())),
            ("symbol".to_owned(), Value::Text(self.symbol.clone())),
            (
                "type_mode".to_owned(),
                Value::Text(self.type_mode.as_str().to_owned()),
            ),
            ("input".to_owned(), self.input.to_value()),
            ("output".to_owned(), self.output.to_value()),
            (
                "effects".to_owned(),
                Value::map([("effects", Value::Array(vec![]))]),
            ),
            ("evidence".to_owned(), self.evidence.to_value()),
            (
                "clauses".to_owned(),
                Value::Array(
                    self.clauses
                        .iter()
                        .map(|clause| clause.to_value(include_labels))
                        .collect(),
                ),
            ),
        ];
        if let Some(body) = &self.body {
            entries.push(("body".to_owned(), body.to_value()));
        }
        if let Some(decision) = &self.policy_decision {
            entries.push(("policy_decision".to_owned(), decision.to_value()));
        }
        Value::owned_map(entries)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionDefinition {
    pub id: String,
    pub symbol: String,
    pub parameters: Vec<Binding>,
    pub result: BhcpType,
    pub definition: Expression,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PureBinding {
    pub(crate) id: String,
    pub(crate) value_type: CheckedType,
}

impl PureBinding {
    fn to_value(&self) -> Value {
        Value::map([
            ("id", Value::Text(self.id.clone())),
            ("type", self.value_type.to_value()),
        ])
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PureFunctionDefinition {
    pub(crate) id: String,
    pub(crate) symbol: String,
    pub(crate) parameters: Vec<PureBinding>,
    pub(crate) result: CheckedType,
    pub(crate) definition: CheckedExpression,
}

impl PureFunctionDefinition {
    fn to_value(&self) -> Value {
        Value::map([
            ("id", Value::Text(self.id.clone())),
            ("symbol", Value::Text(self.symbol.clone())),
            (
                "parameters",
                Value::Array(self.parameters.iter().map(PureBinding::to_value).collect()),
            ),
            ("result", self.result.to_value()),
            ("definition", self.definition.to_value()),
        ])
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PredicateVerifierBinding {
    pub(crate) verifier: String,
    pub(crate) input: CheckedType,
    pub(crate) output: CheckedType,
    pub(crate) configuration: Option<Value>,
    pub(crate) trust: Vec<String>,
}

impl PredicateVerifierBinding {
    fn to_value(&self) -> Value {
        let mut entries = vec![
            ("verifier".to_owned(), Value::Text(self.verifier.clone())),
            ("input".to_owned(), self.input.to_value()),
            ("output".to_owned(), self.output.to_value()),
        ];
        if let Some(configuration) = &self.configuration {
            entries.push(("configuration".to_owned(), configuration.clone()));
        }
        if !self.trust.is_empty() {
            entries.push((
                "trust".to_owned(),
                Value::Array(self.trust.iter().cloned().map(Value::Text).collect()),
            ));
        }
        Value::owned_map(entries)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PredicateDefinition {
    pub(crate) id: String,
    pub(crate) symbol: String,
    pub(crate) parameters: Vec<PureBinding>,
    pub(crate) definition: Option<CheckedExpression>,
    pub(crate) verifier: Option<PredicateVerifierBinding>,
}

impl PredicateDefinition {
    fn to_value(&self) -> Value {
        let mut entries = vec![
            ("id".to_owned(), Value::Text(self.id.clone())),
            ("symbol".to_owned(), Value::Text(self.symbol.clone())),
            (
                "parameters".to_owned(),
                Value::Array(self.parameters.iter().map(PureBinding::to_value).collect()),
            ),
            (
                "result".to_owned(),
                Value::Array(vec![
                    Value::Text("primitive".to_owned()),
                    Value::Text("Bool".to_owned()),
                ]),
            ),
        ];
        if let Some(definition) = &self.definition {
            entries.push(("definition".to_owned(), definition.to_value()));
        }
        if let Some(verifier) = &self.verifier {
            entries.push(("verifier".to_owned(), verifier.to_value()));
        }
        Value::owned_map(entries)
    }
}

impl FunctionDefinition {
    fn to_value(&self) -> Value {
        Value::map([
            ("id", Value::Text(self.id.clone())),
            ("symbol", Value::Text(self.symbol.clone())),
            (
                "parameters",
                Value::Array(self.parameters.iter().map(Binding::to_value).collect()),
            ),
            ("result", self.result.to_value()),
            ("definition", self.definition.to_value()),
        ])
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticIrDocument {
    pub features: Vec<String>,
    pub type_mode: TypeMode,
    pub types: Vec<CheckedTypeDefinition>,
    pub functions: Vec<FunctionDefinition>,
    pub pure_functions: Vec<PureFunctionDefinition>,
    pub predicates: Vec<PredicateDefinition>,
    pub goals: Vec<GoalDefinition>,
    pub entrypoints: Vec<String>,
    pub effective_policy: Option<EffectivePolicyReference>,
    pub semantic_id: Option<HashId>,
    pub artifact_id: Option<HashId>,
}

impl SemanticIrDocument {
    pub fn to_value(&self, include_artifact_id: bool) -> Value {
        self.value(true, true, include_artifact_id)
    }
    pub fn semantic_value(&self) -> Value {
        self.value(false, false, false)
    }
    fn value(
        &self,
        include_labels: bool,
        include_semantic_id: bool,
        include_artifact_id: bool,
    ) -> Value {
        let mut entries = header_entries(&self.features);
        entries.extend([
            ("kind".to_owned(), Value::Text("semantic-ir".to_owned())),
            (
                "type_mode".to_owned(),
                Value::Text(self.type_mode.as_str().to_owned()),
            ),
            (
                "types".to_owned(),
                Value::Array(
                    self.types
                        .iter()
                        .map(CheckedTypeDefinition::to_value)
                        .collect(),
                ),
            ),
            (
                "functions".to_owned(),
                Value::Array(
                    self.functions
                        .iter()
                        .map(FunctionDefinition::to_value)
                        .chain(
                            self.pure_functions
                                .iter()
                                .map(PureFunctionDefinition::to_value),
                        )
                        .collect(),
                ),
            ),
            (
                "predicates".to_owned(),
                Value::Array(
                    self.predicates
                        .iter()
                        .map(PredicateDefinition::to_value)
                        .collect(),
                ),
            ),
            (
                "goals".to_owned(),
                Value::Array(
                    self.goals
                        .iter()
                        .map(|goal| goal.to_value(include_labels))
                        .collect(),
                ),
            ),
            ("extensions".to_owned(), Value::Array(vec![])),
            (
                "entrypoints".to_owned(),
                Value::Array(self.entrypoints.iter().cloned().map(Value::Text).collect()),
            ),
        ]);
        if let Some(policy) = &self.effective_policy {
            entries.push((
                "effective_policy".to_owned(),
                policy.to_value(include_labels),
            ));
        }
        if include_semantic_id && let Some(hash) = &self.semantic_id {
            entries.push(("semantic_id".to_owned(), hash.to_value()));
        }
        if include_artifact_id && let Some(hash) = &self.artifact_id {
            entries.push(("artifact_id".to_owned(), hash.to_value()));
        }
        Value::owned_map(entries)
    }
    pub fn validate(&self) -> Result<()> {
        validate_features(&self.features)?;
        if let Some(policy) = &self.effective_policy {
            policy.validate()?;
        }
        let mut ids = HashSet::new();
        let mut references = Vec::new();
        let mut function_calls = Vec::new();
        let mut goals = HashSet::new();
        let mut child_goals = Vec::new();
        let mut function_symbols = HashSet::new();
        let mut reducers = Vec::new();
        let mut type_symbols = HashSet::new();
        for definition in &self.types {
            add_id(&definition.id, &mut ids)?;
            if !type_symbols.insert(definition.symbol.clone()) {
                return Err(Diagnostic::plain(
                    "BHCP4001",
                    "type definition symbols must be unique",
                ));
            }
            definition.validate()?;
            for parameter in &definition.parameters {
                add_id(&parameter.id, &mut ids)?;
            }
        }
        for function in &self.functions {
            add_id(&function.id, &mut ids)?;
            if !is_symbol(&function.symbol) || !function_symbols.insert(function.symbol.clone()) {
                return Err(Diagnostic::plain(
                    "BHCP4001",
                    "function symbols must be unique symbol-ids",
                ));
            }
            for parameter in &function.parameters {
                add_id(&parameter.id, &mut ids)?;
            }
            if function.definition.value_type != function.result {
                return Err(Diagnostic::plain(
                    "BHCP4001",
                    "function definition type does not match its result type",
                ));
            }
            function
                .definition
                .validate(&mut ids, &mut references, &mut function_calls)?;
        }
        let mut pure_dependencies = BTreeMap::<String, BTreeSet<String>>::new();
        for function in &self.pure_functions {
            add_id(&function.id, &mut ids)?;
            if !is_symbol(&function.symbol) || !function_symbols.insert(function.symbol.clone()) {
                return Err(Diagnostic::plain(
                    "BHCP4001",
                    "function symbols must be unique symbol-ids",
                ));
            }
            for parameter in &function.parameters {
                add_id(&parameter.id, &mut ids)?;
            }
            if function.definition.value_type() != &function.result {
                return Err(Diagnostic::plain(
                    "BHCP4001",
                    "pure function definition type does not match its result type",
                ));
            }
            let mut calls = BTreeSet::new();
            collect_checked_expression(
                &function.definition.to_value(),
                &mut ids,
                &mut references,
                &mut calls,
            )?;
            pure_dependencies.insert(function.symbol.clone(), calls);
        }
        let bool_type = CheckedType::from_value(&Value::Array(vec![
            Value::Text("primitive".to_owned()),
            Value::Text("Bool".to_owned()),
        ]))?;
        for predicate in &self.predicates {
            add_id(&predicate.id, &mut ids)?;
            if !is_symbol(&predicate.symbol) || !function_symbols.insert(predicate.symbol.clone()) {
                return Err(Diagnostic::plain(
                    "BHCP4001",
                    "predicate symbols must be unique symbol-ids",
                ));
            }
            for parameter in &predicate.parameters {
                add_id(&parameter.id, &mut ids)?;
            }
            if predicate.definition.is_none() && predicate.verifier.is_none() {
                return Err(Diagnostic::plain(
                    "BHCP4001",
                    "predicate requires a definition or verifier binding",
                ));
            }
            let mut calls = BTreeSet::new();
            if let Some(definition) = &predicate.definition {
                if definition.value_type() != &bool_type {
                    return Err(Diagnostic::plain(
                        "BHCP4001",
                        "predicate definition must return Bool",
                    ));
                }
                collect_checked_expression(
                    &definition.to_value(),
                    &mut ids,
                    &mut references,
                    &mut calls,
                )?;
            }
            if let Some(binding) = &predicate.verifier
                && (!is_symbol(&binding.verifier)
                    || !matches!(binding.output.to_value(), Value::Array(ref values) if values.first() == Some(&Value::Text("evidence".to_owned())))
                    || binding.trust.iter().any(|class| !is_evidence_class(class))
                    || !is_normalized(&binding.trust))
            {
                return Err(Diagnostic::plain(
                    "BHCP4001",
                    "predicate verifier binding has an invalid typed interface",
                ));
            }
            if let Some(binding) = &predicate.verifier {
                validate_predicate_verifier_configuration(
                    binding,
                    &mut ids,
                    &mut references,
                    &mut calls,
                )?;
            }
            if predicate.definition.is_some() {
                pure_dependencies.insert(predicate.symbol.clone(), calls);
            }
        }
        for dependencies in pure_dependencies.values() {
            if dependencies
                .iter()
                .any(|symbol| !pure_dependencies.contains_key(symbol))
            {
                return Err(Diagnostic::plain(
                    "BHCP4001",
                    "pure definition call does not resolve to a retained definition",
                ));
            }
        }
        validate_acyclic_definitions(&pure_dependencies)?;
        for goal in &self.goals {
            add_id(&goal.id, &mut ids)?;
            goals.insert(goal.id.clone());
            if !is_symbol(&goal.symbol) {
                return Err(Diagnostic::plain(
                    "BHCP4001",
                    "goal symbol is not a symbol-id",
                ));
            }
            if self.effective_policy.is_some() != goal.policy_decision.is_some() {
                return Err(Diagnostic::plain(
                    "BHCP4001",
                    "effective policy and goal policy decisions must appear together",
                ));
            }
            if let Some(decision) = &goal.policy_decision {
                decision.validate()?;
            }
            let contract_ids: HashSet<_> = goal
                .clauses
                .iter()
                .filter_map(|clause| {
                    matches!(clause.kind, ClauseKind::Contract { .. }).then_some(clause.id.clone())
                })
                .collect();
            for clause in &goal.clauses {
                add_id(&clause.id, &mut ids)?;
                match &clause.kind {
                    ClauseKind::Fact { binding, .. } => add_id(&binding.id, &mut ids)?,
                    ClauseKind::Contract {
                        kind,
                        dimension,
                        condition,
                        ..
                    } => {
                        if dimension
                            .as_ref()
                            .is_some_and(|value| *kind != "limit" || !is_symbol(value))
                        {
                            return Err(Diagnostic::plain(
                                "BHCP4001",
                                "only a contract limit may carry a symbol-id dimension",
                            ));
                        }
                        condition.validate(&mut ids, &mut references, &mut function_calls)?
                    }
                    ClauseKind::Authority { effects, .. } => {
                        for effect in effects {
                            if !is_symbol(&effect.id) {
                                return Err(Diagnostic::plain(
                                    "BHCP4001",
                                    "effect ID is not a symbol-id",
                                ));
                            }
                            if let Some(resource) = &effect.resource {
                                references.push(resource.clone());
                            }
                        }
                    }
                    ClauseKind::Preference { objective, .. } => {
                        objective.validate(&mut ids, &mut references, &mut function_calls)?
                    }
                    ClauseKind::Verify {
                        binding,
                        obligations,
                    } => {
                        if !is_symbol(&binding.verifier) {
                            return Err(Diagnostic::plain(
                                "BHCP4001",
                                "verifier is not a symbol-id",
                            ));
                        }
                        let expected_input = BhcpType::Record(vec![
                            FieldType {
                                name: "input".to_owned(),
                                value_type: goal.input.clone(),
                            },
                            FieldType {
                                name: "output".to_owned(),
                                value_type: goal.output.clone(),
                            },
                        ]);
                        if binding.input != expected_input
                            || !matches!(binding.output, BhcpType::Evidence(_))
                            || binding.trust.iter().any(|class| !is_evidence_class(class))
                            || !is_normalized(&binding.trust)
                            || obligations
                                .iter()
                                .any(|obligation| !contract_ids.contains(obligation))
                            || !is_normalized(obligations)
                        {
                            return Err(Diagnostic::plain(
                                "BHCP4001",
                                "goal verifier binding has an invalid typed interface",
                            ));
                        }
                        references.extend(obligations.iter().cloned());
                    }
                }
            }
            if let Some(body) = &goal.body {
                body.validate()?;
                add_id(&body.id, &mut ids)?;
                reducers.push(body.reducer.clone());
                for child in &body.children {
                    add_id(&child.id, &mut ids)?;
                    child_goals.push(child.goal.clone());
                    for argument in &child.arguments {
                        argument
                            .value
                            .validate(&mut ids, &mut references, &mut function_calls)?;
                    }
                }
            }
        }
        if references.iter().any(|reference| !ids.contains(reference)) {
            return Err(Diagnostic::plain(
                "BHCP4001",
                "IR reference does not resolve to a structural ID",
            ));
        }
        if function_calls
            .iter()
            .any(|call| !function_symbols.contains(call) && !is_registered_kernel_primitive(call))
        {
            return Err(Diagnostic::plain(
                "BHCP4001",
                "IR function call does not resolve to a retained definition or closed kernel primitive",
            ));
        }
        if self
            .entrypoints
            .iter()
            .any(|entrypoint| !goals.contains(entrypoint))
        {
            return Err(Diagnostic::plain(
                "BHCP4001",
                "entrypoint does not reference a goal",
            ));
        }
        if child_goals.iter().any(|goal| !goals.contains(goal)) {
            return Err(Diagnostic::plain(
                "BHCP4001",
                "kernel child does not reference a goal",
            ));
        }
        if reducers
            .iter()
            .any(|reducer| !function_symbols.contains(reducer))
        {
            return Err(Diagnostic::plain(
                "BHCP4001",
                "kernel reducer does not resolve to a function",
            ));
        }
        for parent in &self.goals {
            let Some(network) = &parent.body else {
                continue;
            };
            let reducer = self
                .functions
                .iter()
                .find(|function| function.symbol == network.reducer)
                .expect("reducer resolution was checked above");
            validate_network_arguments(parent, network, &self.goals)?;
            let mut observation_fields = Vec::with_capacity(network.children.len());
            for child in &network.children {
                let child_goal = self
                    .goals
                    .iter()
                    .find(|goal| goal.id == child.goal)
                    .expect("child goal resolution was checked above");
                observation_fields.push(FieldType {
                    name: child.tag.clone(),
                    value_type: BhcpType::Option(Box::new(BhcpType::ExecutionResult(Box::new(
                        child_goal.output.clone(),
                    )))),
                });
            }
            observation_fields.sort_by(|left, right| left.name.cmp(&right.name));
            let expected_observations = BhcpType::Record(observation_fields);
            let valid_parameters = reducer.parameters.len() == 2
                && reducer.parameters[0].value_type == parent.input
                && reducer.parameters[1].value_type == expected_observations;
            let expected_result = BhcpType::Reduction(Box::new(parent.output.clone()));
            if !valid_parameters || reducer.result != expected_result {
                return Err(Diagnostic::plain(
                    "BHCP4001",
                    "kernel reducer signature must be (parent input, monomorphized observations) -> Reduction<parent output>",
                ));
            }
        }
        if let Some(hash) = &self.semantic_id {
            hash.validate()?;
        }
        if let Some(hash) = &self.artifact_id {
            hash.validate()?;
        }
        Ok(())
    }
}

fn collect_checked_expression(
    value: &Value,
    ids: &mut HashSet<String>,
    references: &mut Vec<String>,
    calls: &mut BTreeSet<String>,
) -> Result<()> {
    if let Value::Map(entries) = value
        && value.get("type").is_some()
        && let Some(Value::Text(id)) = value.get("id")
    {
        add_id(id, ids)?;
        for (_, nested) in entries {
            collect_checked_expression(nested, ids, references, calls)?;
        }
        return Ok(());
    }
    if let Value::Array(values) = value {
        match values.as_slice() {
            [Value::Text(tag), Value::Text(reference)] if tag == "reference" => {
                references.push(reference.clone());
            }
            [Value::Text(tag), Value::Text(symbol), Value::Array(_)] if tag == "call" => {
                calls.insert(symbol.clone());
            }
            _ => {}
        }
        for nested in values {
            collect_checked_expression(nested, ids, references, calls)?;
        }
    } else if let Value::Map(entries) = value {
        for (_, nested) in entries {
            collect_checked_expression(nested, ids, references, calls)?;
        }
    } else if let Value::Tag(_, nested) = value {
        collect_checked_expression(nested, ids, references, calls)?;
    }
    Ok(())
}

fn validate_predicate_verifier_configuration(
    binding: &PredicateVerifierBinding,
    ids: &mut HashSet<String>,
    references: &mut Vec<String>,
    calls: &mut BTreeSet<String>,
) -> Result<()> {
    let arguments = match &binding.configuration {
        None => &[][..],
        Some(Value::Array(arguments)) => arguments.as_slice(),
        Some(_) => {
            return Err(Diagnostic::plain(
                "BHCP4001",
                "predicate verifier configuration must be a canonical argument array",
            ));
        }
    };
    let mut previous = None;
    let mut fields = Vec::with_capacity(arguments.len());
    for argument in arguments {
        let Value::Map(entries) = argument else {
            return Err(Diagnostic::plain(
                "BHCP4001",
                "predicate verifier argument must be a closed map",
            ));
        };
        let (Some(Value::Text(name)), Some(Value::Text(mode)), Some(value)) = (
            argument.get("name"),
            argument.get("mode"),
            argument.get("value"),
        ) else {
            return Err(Diagnostic::plain(
                "BHCP4001",
                "predicate verifier argument is incomplete",
            ));
        };
        if entries.len() != 3
            || name.is_empty()
            || !matches!(mode.as_str(), "value" | "move" | "borrow" | "share")
            || previous.is_some_and(|previous: &str| previous >= name.as_str())
        {
            return Err(Diagnostic::plain(
                "BHCP4001",
                "predicate verifier arguments are invalid or noncanonical",
            ));
        }
        previous = Some(name.as_str());
        let Some(value_type) = value.get("type") else {
            return Err(Diagnostic::plain(
                "BHCP4001",
                "predicate verifier argument expression omits its type",
            ));
        };
        CheckedType::from_value(value_type)?;
        collect_checked_expression(value, ids, references, calls)?;
        fields.push(Value::Array(vec![
            Value::Text(name.clone()),
            value_type.clone(),
            Value::Bool(false),
        ]));
    }
    let expected_input = Value::Array(vec![
        Value::Text("record".to_owned()),
        Value::Bool(false),
        Value::Array(fields),
    ]);
    if binding.input.to_value() != expected_input {
        return Err(Diagnostic::plain(
            "BHCP4001",
            "predicate verifier input does not match its canonical arguments",
        ));
    }
    Ok(())
}

fn validate_acyclic_definitions(dependencies: &BTreeMap<String, BTreeSet<String>>) -> Result<()> {
    fn visit(
        symbol: &str,
        dependencies: &BTreeMap<String, BTreeSet<String>>,
        visiting: &mut BTreeSet<String>,
        visited: &mut BTreeSet<String>,
    ) -> Result<()> {
        if visited.contains(symbol) {
            return Ok(());
        }
        if !visiting.insert(symbol.to_owned()) {
            return Err(Diagnostic::plain(
                "BHCP4001",
                "pure definition dependency graph contains a cycle",
            ));
        }
        for dependency in &dependencies[symbol] {
            visit(dependency, dependencies, visiting, visited)?;
        }
        visiting.remove(symbol);
        visited.insert(symbol.to_owned());
        Ok(())
    }

    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    for symbol in dependencies.keys() {
        visit(symbol, dependencies, &mut visiting, &mut visited)?;
    }
    Ok(())
}

fn validate_network_arguments(
    parent: &GoalDefinition,
    network: &KernelNetwork,
    goals: &[GoalDefinition],
) -> Result<()> {
    for (child_index, child) in network.children.iter().enumerate() {
        let goal = goals
            .iter()
            .find(|goal| goal.id == child.goal)
            .expect("child goal resolution was checked before argument validation");
        let BhcpType::Record(fields) = &goal.input else {
            return Err(Diagnostic::plain(
                "BHCP4001",
                "kernel child goal input must be a record",
            ));
        };
        if child.arguments.len() != fields.len() {
            return Err(Diagnostic::plain(
                "BHCP4001",
                "kernel child arguments must exactly cover its typed input fields",
            ));
        }
        for field in fields {
            let Some(argument) = child
                .arguments
                .iter()
                .find(|argument| argument.name == field.name)
            else {
                return Err(Diagnostic::plain(
                    "BHCP4001",
                    "kernel child argument does not name a typed input field",
                ));
            };
            if argument.value.value_type != field.value_type {
                return Err(Diagnostic::plain(
                    "BHCP4001",
                    "kernel child argument does not preserve its input field type",
                ));
            }
            let ExpressionForm::Call(symbol, parameters) = &argument.value.form else {
                return Err(Diagnostic::plain(
                    "BHCP4001",
                    "kernel child argument is not a checked data-edge expression",
                ));
            };
            let [parameter] = parameters.as_slice() else {
                return Err(Diagnostic::plain(
                    "BHCP4001",
                    "observed-output data edge must name exactly one predecessor tag",
                ));
            };
            let ExpressionForm::Literal(Value::Text(tag)) = &parameter.form else {
                return Err(Diagnostic::plain(
                    "BHCP4001",
                    "observed-output data edge must use a literal predecessor tag",
                ));
            };
            if symbol == "bhcp/kernel.parent-field@0" {
                let BhcpType::Record(parent_fields) = &parent.input else {
                    unreachable!("goal inputs are records in the implemented source slice")
                };
                let Some(parent_field) = parent_fields.iter().find(|field| field.name == *tag)
                else {
                    return Err(Diagnostic::plain(
                        "BHCP4001",
                        "parent-field data edge must name a parent input field",
                    ));
                };
                if parameter.value_type != BhcpType::Primitive("Text")
                    || argument.value.value_type != parent_field.value_type
                {
                    return Err(Diagnostic::plain(
                        "BHCP4001",
                        "parent-field data edge has an invalid field type",
                    ));
                }
                continue;
            }
            let Some(predecessor) = network.children[..child_index]
                .iter()
                .find(|candidate| candidate.tag == *tag)
            else {
                return Err(Diagnostic::plain(
                    "BHCP4001",
                    "observed-output data edge must name an earlier network child",
                ));
            };
            let predecessor_goal = goals
                .iter()
                .find(|goal| goal.id == predecessor.goal)
                .expect("predecessor child goal resolves");
            if symbol != "bhcp/kernel.observed-output@0"
                || parameter.value_type != BhcpType::Primitive("Text")
                || argument.value.value_type != predecessor_goal.output
            {
                return Err(Diagnostic::plain(
                    "BHCP4001",
                    "observed-output data edge has an invalid symbol or output type",
                ));
            }
        }
    }
    Ok(())
}

fn header_entries(features: &[String]) -> Vec<(String, Value)> {
    vec![
        ("version".to_owned(), Value::Text("bhcp/v0".to_owned())),
        (
            "features".to_owned(),
            Value::Array(features.iter().cloned().map(Value::Text).collect()),
        ),
    ]
}

fn validate_features(features: &[String]) -> Result<()> {
    if features.iter().all(|feature| is_symbol(feature)) {
        Ok(())
    } else {
        Err(Diagnostic::plain("BHCP4001", "feature is not a symbol-id"))
    }
}

fn add_id(id: &str, ids: &mut HashSet<String>) -> Result<()> {
    if !id.is_empty() && id.len() <= 128 && ids.insert(id.to_owned()) {
        Ok(())
    } else {
        Err(Diagnostic::plain(
            "BHCP4001",
            "structural IDs must be unique non-empty ref-ids",
        ))
    }
}

pub fn is_symbol(value: &str) -> bool {
    let Some((path, version)) = value.rsplit_once('@') else {
        return false;
    };
    !version.is_empty()
        && path.split('/').count() >= 2
        && path.split('/').all(valid_symbol_component)
        && valid_symbol_component(version)
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

fn is_registered_kernel_primitive(value: &str) -> bool {
    matches!(
        value,
        "bhcp/kernel.has-refuted@0"
            | "bhcp/kernel.has-missing@0"
            | "bhcp/kernel.has-faulted@0"
            | "bhcp/kernel.has-unresolved@0"
            | "bhcp/kernel.has-satisfied@0"
            | "bhcp/kernel.all-refuted@0"
            | "bhcp/kernel.missing-tags@0"
            | "bhcp/kernel.first-missing-tag@0"
            | "bhcp/kernel.first-counter-evidence@0"
            | "bhcp/kernel.all-counter-evidence@0"
            | "bhcp/kernel.first-satisfied-evidence@0"
            | "bhcp/kernel.partial-evidence@0"
            | "bhcp/kernel.satisfied-evidence@0"
            | "bhcp/kernel.first-fault@0"
            | "bhcp/kernel.first-unresolved-reason@0"
            | "bhcp/kernel.satisfied-record@0"
            | "bhcp/kernel.first-satisfied-output@0"
            | "bhcp/kernel.last-satisfied-output@0"
            | "bhcp/kernel.last-satisfied-output-or-unit@0"
            | "bhcp/kernel.first-satisfied-winner@0"
            | "bhcp/kernel.included@0"
            | "bhcp/kernel.excluded@0"
            | "bhcp/kernel.unit@0"
            | "bhcp/kernel.pending@0"
            | "bhcp/kernel.refuted@0"
            | "bhcp/kernel.faulted@0"
            | "bhcp/kernel.unresolved@0"
            | "bhcp/kernel.satisfied@0"
            | "bhcp/kernel.conclude@0"
            | "bhcp/kernel.observed-output@0"
            | "bhcp/kernel.parent-field@0"
    )
}

fn is_normalized(values: &[String]) -> bool {
    values.windows(2).all(|pair| pair[0] < pair[1])
}

fn valid_symbol_component(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}
