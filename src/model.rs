use std::collections::HashSet;

use crate::diagnostic::{Diagnostic, Result};
use crate::hash::SHA3_512;
use crate::kernel::KernelNetwork;
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
    pub fn to_value(&self) -> Value {
        Value::map([
            ("media_type", Value::Text(self.media_type.clone())),
            ("size", Value::Integer(self.size as i64)),
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
            ("start_byte", Value::Integer(self.start.byte as i64)),
            ("end_byte", Value::Integer(self.end.byte as i64)),
            ("start_line", Value::Integer(self.start.line as i64)),
            ("start_column", Value::Integer(self.start.column as i64)),
            ("end_line", Value::Integer(self.end.line as i64)),
            ("end_column", Value::Integer(self.end.column as i64)),
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
    pub root: AstNode,
    pub source: ContentReference,
    pub artifact_id: Option<HashId>,
}

impl CanonicalAstDocument {
    pub fn to_value(&self, include_artifact_id: bool) -> Value {
        let mut entries = header_entries(&self.features);
        entries.extend([
            ("kind".to_owned(), Value::Text("canonical-ast".to_owned())),
            (
                "profile".to_owned(),
                Value::Text("bhcp/canonical@0".to_owned()),
            ),
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
        };
        Value::map([
            ("id", Value::Text(self.id.clone())),
            ("type", self.value_type.to_value()),
            ("form", form),
        ])
    }
    fn validate(&self, ids: &mut HashSet<String>, references: &mut Vec<String>) -> Result<()> {
        add_id(&self.id, ids)?;
        match &self.form {
            ExpressionForm::Literal(_) => {}
            ExpressionForm::Reference(id) => references.push(id.clone()),
            ExpressionForm::Unary(_, operand) => operand.validate(ids, references)?,
            ExpressionForm::Binary(_, left, right) => {
                left.validate(ids, references)?;
                right.validate(ids, references)?;
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
}

impl VerifierBinding {
    fn to_value(&self) -> Value {
        Value::map([
            ("verifier", Value::Text(self.verifier.clone())),
            ("input", self.input.to_value()),
            ("output", self.output.to_value()),
            (
                "trust",
                Value::Array(vec![Value::Text("static".to_owned())]),
            ),
        ])
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
            ClauseKind::Contract { kind, condition } => {
                entries.push(("kind".to_owned(), Value::Text((*kind).to_owned())));
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
                entries.push(("priority".to_owned(), Value::Integer(*priority)));
                entries.push(("objective".to_owned(), objective.to_value()));
            }
            ClauseKind::Verify { binding } => {
                entries.push(("kind".to_owned(), Value::Text("verify".to_owned())));
                entries.push(("binding".to_owned(), binding.to_value()));
            }
        }
        Value::owned_map(entries)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GoalDefinition {
    pub id: String,
    pub symbol: String,
    pub input: BhcpType,
    pub output: BhcpType,
    pub evidence: BhcpType,
    pub clauses: Vec<Clause>,
    pub body: Option<KernelNetwork>,
}

impl GoalDefinition {
    fn to_value(&self, include_labels: bool) -> Value {
        let mut entries = vec![
            ("id".to_owned(), Value::Text(self.id.clone())),
            ("symbol".to_owned(), Value::Text(self.symbol.clone())),
            (
                "type_mode".to_owned(),
                Value::Text("infer-strict".to_owned()),
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
    pub functions: Vec<FunctionDefinition>,
    pub goals: Vec<GoalDefinition>,
    pub entrypoints: Vec<String>,
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
                Value::Text("infer-strict".to_owned()),
            ),
            ("types".to_owned(), Value::Array(vec![])),
            (
                "functions".to_owned(),
                Value::Array(
                    self.functions
                        .iter()
                        .map(FunctionDefinition::to_value)
                        .collect(),
                ),
            ),
            ("predicates".to_owned(), Value::Array(vec![])),
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
        let mut ids = HashSet::new();
        let mut references = Vec::new();
        let mut goals = HashSet::new();
        let mut child_goals = Vec::new();
        let mut function_symbols = HashSet::new();
        let mut reducers = Vec::new();
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
            function.definition.validate(&mut ids, &mut references)?;
        }
        for goal in &self.goals {
            add_id(&goal.id, &mut ids)?;
            goals.insert(goal.id.clone());
            if !is_symbol(&goal.symbol) {
                return Err(Diagnostic::plain(
                    "BHCP4001",
                    "goal symbol is not a symbol-id",
                ));
            }
            for clause in &goal.clauses {
                add_id(&clause.id, &mut ids)?;
                match &clause.kind {
                    ClauseKind::Fact { binding, .. } => add_id(&binding.id, &mut ids)?,
                    ClauseKind::Contract { condition, .. } => {
                        condition.validate(&mut ids, &mut references)?
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
                        objective.validate(&mut ids, &mut references)?
                    }
                    ClauseKind::Verify { binding } => {
                        if !is_symbol(&binding.verifier) {
                            return Err(Diagnostic::plain(
                                "BHCP4001",
                                "verifier is not a symbol-id",
                            ));
                        }
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
                }
            }
        }
        if references.iter().any(|reference| !ids.contains(reference)) {
            return Err(Diagnostic::plain(
                "BHCP4001",
                "IR reference does not resolve to a structural ID",
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
        if let Some(hash) = &self.semantic_id {
            hash.validate()?;
        }
        if let Some(hash) = &self.artifact_id {
            hash.validate()?;
        }
        Ok(())
    }
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

fn valid_symbol_component(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}
