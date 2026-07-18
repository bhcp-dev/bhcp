//! Minimal outcome-aware network kernel used by self-hosted BHCP behavior.

use std::collections::HashSet;

use crate::diagnostic::{Diagnostic, Result};
use crate::model::{BhcpType, Expression, is_symbol};
use crate::value::Value;

const INVALID_KERNEL: &str = "BHCP4101";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Reason {
    pub code: String,
    pub message: String,
    pub details: Option<Value>,
}

impl Reason {
    fn to_value(&self) -> Value {
        let mut entries = vec![
            ("code".to_owned(), Value::Text(self.code.clone())),
            ("message".to_owned(), Value::Text(self.message.clone())),
        ];
        if let Some(details) = &self.details {
            entries.push(("details".to_owned(), details.clone()));
        }
        Value::owned_map(entries)
    }

    fn validate(&self) -> Result<()> {
        if !is_symbol(&self.code) {
            return Err(invalid("reason code must be a symbol-id"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceEvent {
    pub sequence: usize,
    pub node: String,
    pub at: String,
    pub kind: String,
    pub payload: Option<Value>,
}

impl TraceEvent {
    fn to_value(&self) -> Value {
        let mut entries = vec![
            ("sequence".to_owned(), Value::Integer(self.sequence as i64)),
            ("node".to_owned(), Value::Text(self.node.clone())),
            ("at".to_owned(), Value::Text(self.at.clone())),
            ("kind".to_owned(), Value::Text(self.kind.clone())),
        ];
        if let Some(payload) = &self.payload {
            entries.push(("payload".to_owned(), payload.clone()));
        }
        Value::owned_map(entries)
    }

    fn validate(&self) -> Result<()> {
        validate_ref(&self.node)?;
        if !is_symbol(&self.kind) {
            return Err(invalid("trace kind must be a symbol-id"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Verdict {
    Satisfied {
        output: Value,
        evidence: Vec<String>,
    },
    Refuted {
        counter_evidence: Vec<String>,
    },
    Unresolved {
        reason: Reason,
        partial_evidence: Vec<String>,
    },
}

impl Verdict {
    fn to_value(&self) -> Value {
        match self {
            Self::Satisfied { output, evidence } => Value::map([
                ("state", Value::Text("satisfied".to_owned())),
                ("output", output.clone()),
                ("evidence", refs_value(evidence)),
            ]),
            Self::Refuted { counter_evidence } => Value::map([
                ("state", Value::Text("refuted".to_owned())),
                ("counter_evidence", refs_value(counter_evidence)),
            ]),
            Self::Unresolved {
                reason,
                partial_evidence,
            } => Value::map([
                ("state", Value::Text("unresolved".to_owned())),
                ("reason", reason.to_value()),
                ("partial_evidence", refs_value(partial_evidence)),
            ]),
        }
    }

    fn validate(&self) -> Result<()> {
        match self {
            Self::Satisfied { evidence, .. } => validate_nonempty_refs(evidence, "evidence"),
            Self::Refuted { counter_evidence } => {
                validate_nonempty_refs(counter_evidence, "counter-evidence")
            }
            Self::Unresolved {
                reason,
                partial_evidence,
            } => {
                reason.validate()?;
                validate_refs(partial_evidence)
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationalFault {
    pub error: Reason,
    pub trace: Vec<TraceEvent>,
}

impl OperationalFault {
    fn to_value(&self) -> Value {
        Value::map([
            ("error", self.error.to_value()),
            (
                "trace",
                Value::Array(self.trace.iter().map(TraceEvent::to_value).collect()),
            ),
        ])
    }

    fn validate(&self) -> Result<()> {
        self.error.validate()?;
        for event in &self.trace {
            event.validate()?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExecutionResult {
    Completed(Verdict),
    Faulted(OperationalFault),
}

impl ExecutionResult {
    pub fn to_value(&self) -> Value {
        match self {
            Self::Completed(verdict) => Value::map([
                ("state", Value::Text("completed".to_owned())),
                ("verdict", verdict.to_value()),
            ]),
            Self::Faulted(fault) => Value::map([
                ("state", Value::Text("faulted".to_owned())),
                ("fault", fault.to_value()),
            ]),
        }
    }

    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Completed(verdict) => verdict.validate(),
            Self::Faulted(fault) => fault.validate(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Derivation {
    pub id: String,
    pub premises: Vec<String>,
}

impl Derivation {
    fn to_value(&self) -> Value {
        Value::map([
            ("id", Value::Text(self.id.clone())),
            ("premises", refs_value(&self.premises)),
        ])
    }

    fn validate(&self) -> Result<()> {
        validate_ref(&self.id)?;
        validate_refs(&self.premises)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KernelChild {
    pub id: String,
    pub tag: String,
    pub goal: String,
    pub arguments: Vec<KernelArgument>,
}

impl KernelChild {
    fn to_value(&self) -> Value {
        Value::map([
            ("id", Value::Text(self.id.clone())),
            ("tag", Value::Text(self.tag.clone())),
            ("goal", Value::Text(self.goal.clone())),
            (
                "arguments",
                Value::owned_map(
                    self.arguments
                        .iter()
                        .map(|argument| (argument.name.clone(), argument.to_value()))
                        .collect(),
                ),
            ),
        ])
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArgumentMode {
    Value,
    Move,
    Borrow,
    Share,
}

impl ArgumentMode {
    fn name(self) -> &'static str {
        match self {
            Self::Value => "value",
            Self::Move => "move",
            Self::Borrow => "borrow",
            Self::Share => "share",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KernelArgument {
    pub name: String,
    pub mode: ArgumentMode,
    pub value: Expression,
}

impl KernelArgument {
    fn to_value(&self) -> Value {
        Value::map([
            ("mode", Value::Text(self.mode.name().to_owned())),
            ("value", self.value.to_value()),
        ])
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChildObservation {
    pub child: String,
    pub result: ExecutionResult,
}

impl ChildObservation {
    pub fn to_value(&self) -> Value {
        Value::map([
            ("child", Value::Text(self.child.clone())),
            ("result", self.result.to_value()),
        ])
    }

    pub fn validate(&self, network: &KernelNetwork) -> Result<()> {
        if !network.child_ids().contains(self.child.as_str()) {
            return Err(invalid("observation must name a known child"));
        }
        self.result.validate()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KernelNetwork {
    pub id: String,
    pub output: BhcpType,
    pub children: Vec<KernelChild>,
    pub reducer: String,
}

impl KernelNetwork {
    pub fn to_value(&self) -> Value {
        Value::map([
            ("id", Value::Text(self.id.clone())),
            ("output", self.output.to_value()),
            (
                "children",
                Value::Array(self.children.iter().map(KernelChild::to_value).collect()),
            ),
            ("reducer", Value::Text(self.reducer.clone())),
        ])
    }

    pub fn validate(&self) -> Result<()> {
        validate_ref(&self.id)?;
        if !is_symbol(&self.reducer) {
            return Err(invalid("network reducer must be a symbol-id"));
        }
        let mut ids = HashSet::new();
        let mut tags = HashSet::new();
        for child in &self.children {
            validate_ref(&child.id)?;
            validate_ref(&child.goal)?;
            if !ids.insert(child.id.as_str()) {
                return Err(invalid("network child IDs must be unique"));
            }
            if child.tag.is_empty() || !tags.insert(child.tag.as_str()) {
                return Err(invalid("network child tags must be unique and non-empty"));
            }
            let mut arguments = HashSet::new();
            if child.arguments.iter().any(|argument| {
                argument.name.is_empty() || !arguments.insert(argument.name.as_str())
            }) {
                return Err(invalid(
                    "network argument names must be unique and non-empty",
                ));
            }
        }
        Ok(())
    }

    fn child_ids(&self) -> HashSet<&str> {
        self.children
            .iter()
            .map(|child| child.id.as_str())
            .collect()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Reduction {
    Pending {
        required: Vec<String>,
    },
    Concluded {
        result: ExecutionResult,
        derivation: Derivation,
    },
}

impl Reduction {
    pub fn to_value(&self) -> Value {
        match self {
            Self::Pending { required } => Value::map([
                ("state", Value::Text("pending".to_owned())),
                ("required", refs_value(required)),
            ]),
            Self::Concluded { result, derivation } => Value::map([
                ("state", Value::Text("concluded".to_owned())),
                ("result", result.to_value()),
                ("derivation", derivation.to_value()),
            ]),
        }
    }

    pub fn validate(&self, network: &KernelNetwork, observed: &HashSet<String>) -> Result<()> {
        match self {
            Self::Pending { required } => {
                if required.is_empty() {
                    return Err(invalid("pending reduction requires at least one child"));
                }
                let children = network.child_ids();
                let mut unique = HashSet::new();
                for child in required {
                    if !children.contains(child.as_str())
                        || observed.contains(child)
                        || !unique.insert(child.as_str())
                    {
                        return Err(invalid(
                            "pending reduction must name unique, known, unobserved children",
                        ));
                    }
                }
                Ok(())
            }
            Self::Concluded { result, derivation } => {
                result.validate()?;
                derivation.validate()?;
                match result {
                    ExecutionResult::Completed(Verdict::Satisfied { evidence, .. })
                        if !evidence.contains(&derivation.id) =>
                    {
                        Err(invalid(
                            "a concluded satisfied verdict must reference its checked derivation",
                        ))
                    }
                    ExecutionResult::Completed(Verdict::Refuted { counter_evidence })
                        if !counter_evidence.contains(&derivation.id) =>
                    {
                        Err(invalid(
                            "a concluded refuted verdict must reference its checked derivation",
                        ))
                    }
                    _ => Ok(()),
                }
            }
        }
    }
}

fn refs_value(refs: &[String]) -> Value {
    Value::Array(refs.iter().cloned().map(Value::Text).collect())
}

fn validate_nonempty_refs(refs: &[String], name: &str) -> Result<()> {
    if refs.is_empty() {
        return Err(invalid(format!("{name} must not be empty")));
    }
    validate_refs(refs)
}

fn validate_refs(refs: &[String]) -> Result<()> {
    let mut unique = HashSet::new();
    for reference in refs {
        validate_ref(reference)?;
        if !unique.insert(reference) {
            return Err(invalid("references must be unique"));
        }
    }
    Ok(())
}

fn validate_ref(reference: &str) -> Result<()> {
    if reference.is_empty() || reference.len() > 128 {
        Err(invalid("reference must be a non-empty ref-id"))
    } else {
        Ok(())
    }
}

fn invalid(message: impl Into<String>) -> Diagnostic {
    Diagnostic::plain(INVALID_KERNEL, message)
}
