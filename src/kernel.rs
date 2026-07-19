//! Minimal outcome-aware network kernel used by self-hosted BHCP behavior.

use std::collections::HashSet;

use crate::cbor::encode_deterministic;
use crate::diagnostic::{Diagnostic, Result};
use crate::hash::HashAlgorithm;
use crate::model::{
    BhcpType, Expression, ExpressionForm, FieldType, FunctionDefinition, SemanticIrDocument,
    is_symbol,
};
use crate::value::Value;

const INVALID_KERNEL: &str = "BHCP4101";
const INVALID_DERIVATION: &str = "BHCP4102";

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

    fn child_tags(&self) -> HashSet<&str> {
        self.children
            .iter()
            .map(|child| child.tag.as_str())
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
                    return Err(invalid("pending reduction requires at least one child tag"));
                }
                let children = network.child_tags();
                let mut unique = HashSet::new();
                for child in required {
                    if !children.contains(child.as_str())
                        || observed.contains(child)
                        || !unique.insert(child.as_str())
                    {
                        return Err(invalid(
                            "pending reduction must name unique, known, unobserved child tags",
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

#[derive(Clone)]
struct ObservationSlot {
    tag: String,
    result: Option<ExecutionResult>,
}

#[derive(Clone)]
enum RuntimeValue {
    Data(Value),
    Observations(Vec<ObservationSlot>),
    Bool(bool),
    Texts(Vec<String>),
    Reason(Reason),
    Fault(OperationalFault),
    Result(ExecutionResult),
    Reduction(Reduction),
}

/// Executes and generically re-checks total pure reducer definitions retained in semantic IR.
pub struct KernelRuntime<'a> {
    ir: &'a SemanticIrDocument,
}

impl<'a> KernelRuntime<'a> {
    pub fn new(ir: &'a SemanticIrDocument) -> Self {
        Self { ir }
    }

    pub fn reduce(
        &self,
        network_id: &str,
        parent: Value,
        observations: &[ChildObservation],
    ) -> Result<Reduction> {
        self.ir.validate()?;
        let (parent_goal, network, reducer) = self.resolve(network_id)?;
        if !parent_goal.input.accepts(&parent) {
            return Err(invalid(
                "reducer parent input does not match the goal input type",
            ));
        }
        validate_reducer_expression(&reducer.definition, reducer)?;
        let slots = self.observation_slots(network, observations)?;
        let derivation_id = derive_derivation_id(network, &parent, &slots)?;
        let value = evaluate_expression(
            &reducer.definition,
            reducer,
            &derivation_id,
            parent,
            slots.clone(),
        )?;
        let RuntimeValue::Reduction(reduction) = value else {
            return Err(invalid("reducer did not return a reduction"));
        };
        let observed_tags = slots
            .iter()
            .filter_map(|slot| slot.result.as_ref().map(|_| slot.tag.clone()))
            .collect();
        reduction.validate(network, &observed_tags)?;
        if let Reduction::Concluded {
            result: ExecutionResult::Completed(Verdict::Satisfied { output, .. }),
            ..
        } = &reduction
            && !network.output.accepts(output)
        {
            return Err(invalid(
                "reducer conclusion output does not match the network output type",
            ));
        }
        Ok(reduction)
    }

    pub fn verify(
        &self,
        network_id: &str,
        parent: Value,
        observations: &[ChildObservation],
        claimed: &Reduction,
    ) -> Result<()> {
        let expected = self.reduce(network_id, parent, observations)?;
        if &expected != claimed {
            return Err(invalid_derivation(
                "reducer result does not match re-evaluation",
            ));
        }
        if let Reduction::Concluded { derivation, .. } = claimed {
            let accepted = accepted_premises(observations);
            if derivation
                .premises
                .iter()
                .any(|premise| !accepted.contains(premise))
            {
                return Err(invalid_derivation(
                    "derivation references an unaccepted premise",
                ));
            }
        }
        Ok(())
    }

    fn resolve(
        &self,
        network_id: &str,
    ) -> Result<(
        &crate::model::GoalDefinition,
        &KernelNetwork,
        &FunctionDefinition,
    )> {
        let (goal, network) = self
            .ir
            .goals
            .iter()
            .find_map(|goal| {
                goal.body
                    .as_ref()
                    .filter(|network| network.id == network_id)
                    .map(|network| (goal, network))
            })
            .ok_or_else(|| invalid("kernel network does not resolve"))?;
        let reducer = self
            .ir
            .functions
            .iter()
            .find(|function| function.symbol == network.reducer)
            .ok_or_else(|| invalid("kernel reducer does not resolve"))?;
        Ok((goal, network, reducer))
    }

    fn observation_slots(
        &self,
        network: &KernelNetwork,
        observations: &[ChildObservation],
    ) -> Result<Vec<ObservationSlot>> {
        let mut supplied = std::collections::HashMap::new();
        for observation in observations {
            observation.validate(network)?;
            if supplied
                .insert(observation.child.as_str(), &observation.result)
                .is_some()
            {
                return Err(invalid("child observations must be unique"));
            }
        }
        network
            .children
            .iter()
            .map(|child| {
                let result = supplied.get(child.id.as_str()).copied();
                if let Some(ExecutionResult::Completed(Verdict::Satisfied { output, .. })) = result
                {
                    let child_goal = self
                        .ir
                        .goals
                        .iter()
                        .find(|goal| goal.id == child.goal)
                        .expect("IR validation resolves child goals");
                    if !child_goal.output.accepts(output) {
                        return Err(invalid(
                            "child output does not match the declared goal output type",
                        ));
                    }
                }
                Ok(ObservationSlot {
                    tag: child.tag.clone(),
                    result: result.cloned(),
                })
            })
            .collect()
    }
}

fn evaluate_expression(
    expression: &Expression,
    function: &FunctionDefinition,
    derivation_id: &str,
    parent: Value,
    observations: Vec<ObservationSlot>,
) -> Result<RuntimeValue> {
    match &expression.form {
        ExpressionForm::Literal(value) => match (&expression.value_type, value) {
            (BhcpType::Primitive("Bool"), Value::Bool(value)) => Ok(RuntimeValue::Bool(*value)),
            (BhcpType::List(element), Value::Array(values))
                if element.as_ref() == &BhcpType::Primitive("Text") =>
            {
                Ok(RuntimeValue::Texts(
                    values
                        .iter()
                        .map(|value| {
                            let Value::Text(value) = value else {
                                unreachable!("literal type was statically validated")
                            };
                            value.clone()
                        })
                        .collect(),
                ))
            }
            _ => Ok(RuntimeValue::Data(value.clone())),
        },
        ExpressionForm::Reference(reference) if reference == &function.parameters[0].id => {
            Ok(RuntimeValue::Data(parent))
        }
        ExpressionForm::Reference(reference) if reference == &function.parameters[1].id => {
            Ok(RuntimeValue::Observations(observations))
        }
        ExpressionForm::If(condition, consequent, alternative) => {
            let condition = evaluate_expression(
                condition,
                function,
                derivation_id,
                parent.clone(),
                observations.clone(),
            )?;
            match condition {
                RuntimeValue::Bool(true) => {
                    evaluate_expression(consequent, function, derivation_id, parent, observations)
                }
                RuntimeValue::Bool(false) => {
                    evaluate_expression(alternative, function, derivation_id, parent, observations)
                }
                _ => Err(invalid("reducer condition did not evaluate to Bool")),
            }
        }
        ExpressionForm::Unary(operator, operand) => {
            let operand =
                evaluate_expression(operand, function, derivation_id, parent, observations)?;
            match (operator.as_str(), operand) {
                ("!", RuntimeValue::Bool(value)) => Ok(RuntimeValue::Bool(!value)),
                _ => Err(invalid("reducer unary operation violated its checked type")),
            }
        }
        ExpressionForm::Binary(operator, left, right) => {
            let left = evaluate_expression(
                left,
                function,
                derivation_id,
                parent.clone(),
                observations.clone(),
            )?;
            let right = evaluate_expression(right, function, derivation_id, parent, observations)?;
            match operator.as_str() {
                "==" => runtime_equal(&left, &right).map(RuntimeValue::Bool),
                "!=" => runtime_equal(&left, &right).map(|equal| RuntimeValue::Bool(!equal)),
                "&&" => match (left, right) {
                    (RuntimeValue::Bool(left), RuntimeValue::Bool(right)) => {
                        Ok(RuntimeValue::Bool(left && right))
                    }
                    _ => Err(invalid(
                        "reducer boolean operation violated its checked type",
                    )),
                },
                "||" => match (left, right) {
                    (RuntimeValue::Bool(left), RuntimeValue::Bool(right)) => {
                        Ok(RuntimeValue::Bool(left || right))
                    }
                    _ => Err(invalid(
                        "reducer boolean operation violated its checked type",
                    )),
                },
                _ => Err(invalid(format!(
                    "unsupported total pure reducer binary operation {operator:?}"
                ))),
            }
        }
        ExpressionForm::Call(symbol, arguments) => {
            let mut values = Vec::with_capacity(arguments.len());
            for argument in arguments {
                values.push(evaluate_expression(
                    argument,
                    function,
                    derivation_id,
                    parent.clone(),
                    observations.clone(),
                )?);
            }
            let value = evaluate_primitive(symbol, derivation_id, values)?;
            if !runtime_value_matches_type(&value, &expression.value_type) {
                return Err(invalid(
                    "reducer primitive result does not match its declared expression type",
                ));
            }
            Ok(value)
        }
        _ => Err(invalid(
            "reducer used an expression outside the executable total-pure slice",
        )),
    }
}

fn runtime_value_matches_type(value: &RuntimeValue, value_type: &BhcpType) -> bool {
    match value {
        RuntimeValue::Data(value) => value_type.accepts(value),
        RuntimeValue::Observations(_) => matches!(value_type, BhcpType::Record(_)),
        RuntimeValue::Bool(_) => value_type == &BhcpType::Primitive("Bool"),
        RuntimeValue::Texts(_) => {
            value_type == &BhcpType::List(Box::new(BhcpType::Primitive("Text")))
        }
        RuntimeValue::Reason(_) => {
            value_type == &BhcpType::Nominal("bhcp/kernel.reason@0".to_owned(), vec![])
        }
        RuntimeValue::Fault(_) => {
            value_type == &BhcpType::Nominal("bhcp/kernel.fault@0".to_owned(), vec![])
        }
        RuntimeValue::Result(_) => matches!(value_type, BhcpType::ExecutionResult(_)),
        RuntimeValue::Reduction(_) => matches!(value_type, BhcpType::Reduction(_)),
    }
}

fn validate_reducer_expression(
    expression: &Expression,
    function: &FunctionDefinition,
) -> Result<()> {
    match &expression.form {
        ExpressionForm::Literal(value) => {
            if !expression.value_type.accepts(value) {
                return Err(invalid(
                    "reducer literal does not match its declared value type",
                ));
            }
        }
        ExpressionForm::Reference(reference) => {
            let Some(binding) = function
                .parameters
                .iter()
                .find(|binding| binding.id == *reference)
            else {
                return Err(invalid("reducer reference is not a parameter binding"));
            };
            if expression.value_type != binding.value_type {
                return Err(invalid(
                    "reducer reference does not preserve its parameter type",
                ));
            }
        }
        ExpressionForm::Unary(operator, operand) => {
            validate_reducer_expression(operand, function)?;
            if operator != "!"
                || operand.value_type != BhcpType::Primitive("Bool")
                || expression.value_type != BhcpType::Primitive("Bool")
            {
                return Err(invalid(format!(
                    "unsupported or ill-typed total pure reducer unary operation {operator:?}"
                )));
            }
        }
        ExpressionForm::Binary(operator, left, right) => {
            validate_reducer_expression(left, function)?;
            validate_reducer_expression(right, function)?;
            let valid = match operator.as_str() {
                "==" | "!=" => {
                    left.value_type == right.value_type
                        && expression.value_type == BhcpType::Primitive("Bool")
                        && left.value_type != function.parameters[1].value_type
                        && !matches!(
                            left.value_type,
                            BhcpType::ExecutionResult(_) | BhcpType::Reduction(_)
                        )
                }
                "&&" | "||" => {
                    left.value_type == BhcpType::Primitive("Bool")
                        && right.value_type == BhcpType::Primitive("Bool")
                        && expression.value_type == BhcpType::Primitive("Bool")
                }
                _ => false,
            };
            if !valid {
                return Err(invalid(format!(
                    "unsupported or ill-typed total pure reducer binary operation {operator:?}"
                )));
            }
        }
        ExpressionForm::If(condition, consequent, alternative) => {
            validate_reducer_expression(condition, function)?;
            validate_reducer_expression(consequent, function)?;
            validate_reducer_expression(alternative, function)?;
            if condition.value_type != BhcpType::Primitive("Bool")
                || consequent.value_type != alternative.value_type
                || expression.value_type != consequent.value_type
            {
                return Err(invalid(
                    "reducer conditional is not total and consistently typed",
                ));
            }
        }
        ExpressionForm::Call(symbol, arguments) => {
            for argument in arguments {
                validate_reducer_expression(argument, function)?;
            }
            validate_primitive_signature(expression, symbol, arguments, function)?;
        }
    }
    Ok(())
}

fn validate_primitive_signature(
    expression: &Expression,
    symbol: &str,
    arguments: &[Expression],
    function: &FunctionDefinition,
) -> Result<()> {
    let observations = &function.parameters[1].value_type;
    let BhcpType::Reduction(output) = &function.result else {
        return Err(invalid("reducer function does not return Reduction"));
    };
    let refs = BhcpType::List(Box::new(BhcpType::Primitive("Text")));
    let fault = BhcpType::Nominal("bhcp/kernel.fault@0".to_owned(), vec![]);
    let reason = BhcpType::Nominal("bhcp/kernel.reason@0".to_owned(), vec![]);
    let execution = BhcpType::ExecutionResult(output.clone());
    let argument_types = arguments
        .iter()
        .map(|argument| argument.value_type.clone())
        .collect::<Vec<_>>();
    let expected = match symbol {
        "bhcp/kernel.has-refuted@0"
        | "bhcp/kernel.has-missing@0"
        | "bhcp/kernel.has-faulted@0"
        | "bhcp/kernel.has-unresolved@0"
        | "bhcp/kernel.has-satisfied@0"
        | "bhcp/kernel.all-refuted@0"
            if argument_types == [observations.clone()] =>
        {
            BhcpType::Primitive("Bool")
        }
        "bhcp/kernel.missing-tags@0"
        | "bhcp/kernel.first-missing-tag@0"
        | "bhcp/kernel.first-counter-evidence@0"
        | "bhcp/kernel.all-counter-evidence@0"
        | "bhcp/kernel.first-satisfied-evidence@0"
        | "bhcp/kernel.partial-evidence@0"
        | "bhcp/kernel.satisfied-evidence@0"
            if argument_types == [observations.clone()] =>
        {
            refs.clone()
        }
        "bhcp/kernel.first-fault@0" if argument_types == [observations.clone()] => fault,
        "bhcp/kernel.first-unresolved-reason@0" if argument_types == [observations.clone()] => {
            reason
        }
        "bhcp/kernel.satisfied-record@0"
        | "bhcp/kernel.first-satisfied-output@0"
        | "bhcp/kernel.last-satisfied-output@0"
            if argument_types == [observations.clone()] =>
        {
            output.as_ref().clone()
        }
        "bhcp/kernel.first-satisfied-winner@0"
            if argument_types == [observations.clone()]
                && winner_type_matches(observations, output) =>
        {
            output.as_ref().clone()
        }
        "bhcp/kernel.unit@0" if argument_types.is_empty() => BhcpType::Primitive("Unit"),
        "bhcp/kernel.pending@0" if argument_types == [refs.clone()] => function.result.clone(),
        "bhcp/kernel.refuted@0" if argument_types == [refs.clone()] => execution.clone(),
        "bhcp/kernel.faulted@0" if argument_types == [fault] => execution.clone(),
        "bhcp/kernel.unresolved@0" if argument_types == [reason, refs.clone()] => execution.clone(),
        "bhcp/kernel.satisfied@0" if argument_types == [output.as_ref().clone(), refs] => {
            execution.clone()
        }
        "bhcp/kernel.conclude@0" if argument_types == [execution] => function.result.clone(),
        _ => {
            return Err(invalid(format!(
                "reducer call is not a registered total pure kernel primitive: {symbol}"
            )));
        }
    };
    if expression.value_type != expected {
        return Err(invalid(format!(
            "reducer primitive {symbol} does not preserve its declared result type"
        )));
    }
    Ok(())
}

fn winner_type_matches(observations: &BhcpType, output: &BhcpType) -> bool {
    let BhcpType::Record(fields) = observations else {
        return false;
    };
    let mut child_outputs = fields.iter().map(|field| match &field.value_type {
        BhcpType::Option(result) => match result.as_ref() {
            BhcpType::ExecutionResult(output) => Some(output.as_ref()),
            _ => None,
        },
        _ => None,
    });
    let Some(first) = child_outputs.next() else {
        return true;
    };
    let Some(first) = first else {
        return false;
    };
    if child_outputs.any(|candidate| candidate != Some(first)) {
        return false;
    }
    output
        == &BhcpType::Record(vec![
            FieldType {
                name: "output".to_owned(),
                value_type: first.clone(),
            },
            FieldType {
                name: "tag".to_owned(),
                value_type: BhcpType::Primitive("Text"),
            },
        ])
}

fn runtime_equal(left: &RuntimeValue, right: &RuntimeValue) -> Result<bool> {
    match (left, right) {
        (RuntimeValue::Data(left), RuntimeValue::Data(right)) => Ok(left == right),
        (RuntimeValue::Bool(left), RuntimeValue::Bool(right)) => Ok(left == right),
        (RuntimeValue::Texts(left), RuntimeValue::Texts(right)) => Ok(left == right),
        _ => Err(invalid("reducer equality compared sealed or unlike values")),
    }
}

fn evaluate_primitive(
    symbol: &str,
    derivation_id: &str,
    arguments: Vec<RuntimeValue>,
) -> Result<RuntimeValue> {
    match symbol {
        "bhcp/kernel.has-refuted@0" => with_observations(arguments, |observations| {
            RuntimeValue::Bool(observations.iter().any(|slot| {
                matches!(
                    &slot.result,
                    Some(ExecutionResult::Completed(Verdict::Refuted { .. }))
                )
            }))
        }),
        "bhcp/kernel.has-missing@0" => with_observations(arguments, |observations| {
            RuntimeValue::Bool(observations.iter().any(|slot| slot.result.is_none()))
        }),
        "bhcp/kernel.has-faulted@0" => with_observations(arguments, |observations| {
            RuntimeValue::Bool(
                observations
                    .iter()
                    .any(|slot| matches!(&slot.result, Some(ExecutionResult::Faulted(_)))),
            )
        }),
        "bhcp/kernel.has-unresolved@0" => with_observations(arguments, |observations| {
            RuntimeValue::Bool(observations.iter().any(|slot| {
                matches!(
                    &slot.result,
                    Some(ExecutionResult::Completed(Verdict::Unresolved { .. }))
                )
            }))
        }),
        "bhcp/kernel.has-satisfied@0" => with_observations(arguments, |observations| {
            RuntimeValue::Bool(observations.iter().any(|slot| {
                matches!(
                    &slot.result,
                    Some(ExecutionResult::Completed(Verdict::Satisfied { .. }))
                )
            }))
        }),
        "bhcp/kernel.all-refuted@0" => with_observations(arguments, |observations| {
            RuntimeValue::Bool(observations.iter().all(|slot| {
                matches!(
                    &slot.result,
                    Some(ExecutionResult::Completed(Verdict::Refuted { .. }))
                )
            }))
        }),
        "bhcp/kernel.missing-tags@0" => with_observations(arguments, |observations| {
            RuntimeValue::Texts(
                observations
                    .iter()
                    .filter(|slot| slot.result.is_none())
                    .map(|slot| slot.tag.clone())
                    .collect(),
            )
        }),
        "bhcp/kernel.first-missing-tag@0" => with_observations(arguments, |observations| {
            RuntimeValue::Texts(
                observations
                    .iter()
                    .find(|slot| slot.result.is_none())
                    .map(|slot| vec![slot.tag.clone()])
                    .unwrap_or_default(),
            )
        }),
        "bhcp/kernel.first-counter-evidence@0" => {
            let observations = take_observations(arguments)?;
            let evidence = observations
                .iter()
                .find_map(|slot| match &slot.result {
                    Some(ExecutionResult::Completed(Verdict::Refuted { counter_evidence })) => {
                        Some(counter_evidence.clone())
                    }
                    _ => None,
                })
                .ok_or_else(|| invalid("no refuted observation supplies counter-evidence"))?;
            Ok(RuntimeValue::Texts(evidence))
        }
        "bhcp/kernel.all-counter-evidence@0" => with_observations(arguments, |observations| {
            RuntimeValue::Texts(unique_refs(observations.iter().flat_map(
                |slot| match &slot.result {
                    Some(ExecutionResult::Completed(Verdict::Refuted { counter_evidence })) => {
                        counter_evidence.clone()
                    }
                    _ => vec![],
                },
            )))
        }),
        "bhcp/kernel.first-fault@0" => {
            let observations = take_observations(arguments)?;
            let fault = observations
                .iter()
                .find_map(|slot| match &slot.result {
                    Some(ExecutionResult::Faulted(fault)) => Some(fault.clone()),
                    _ => None,
                })
                .ok_or_else(|| invalid("no faulted observation supplies a fault"))?;
            Ok(RuntimeValue::Fault(fault))
        }
        "bhcp/kernel.first-unresolved-reason@0" => {
            let observations = take_observations(arguments)?;
            let reason = observations
                .iter()
                .find_map(|slot| match &slot.result {
                    Some(ExecutionResult::Completed(Verdict::Unresolved { reason, .. })) => {
                        Some(reason.clone())
                    }
                    _ => None,
                })
                .ok_or_else(|| invalid("no unresolved observation supplies a reason"))?;
            Ok(RuntimeValue::Reason(reason))
        }
        "bhcp/kernel.partial-evidence@0" => with_observations(arguments, |observations| {
            RuntimeValue::Texts(unique_refs(observations.iter().flat_map(
                |slot| match &slot.result {
                    Some(ExecutionResult::Completed(Verdict::Satisfied { evidence, .. })) => {
                        evidence.clone()
                    }
                    Some(ExecutionResult::Completed(Verdict::Unresolved {
                        partial_evidence,
                        ..
                    })) => partial_evidence.clone(),
                    _ => vec![],
                },
            )))
        }),
        "bhcp/kernel.satisfied-record@0" => {
            let observations = take_observations(arguments)?;
            let mut outputs = Vec::new();
            for slot in observations {
                let Some(ExecutionResult::Completed(Verdict::Satisfied { output, .. })) =
                    slot.result
                else {
                    return Err(invalid(
                        "satisfied-record requires every observation to be satisfied",
                    ));
                };
                outputs.push((slot.tag, output));
            }
            Ok(RuntimeValue::Data(Value::owned_map(outputs)))
        }
        "bhcp/kernel.first-satisfied-output@0" => {
            let observations = take_observations(arguments)?;
            let output = observations
                .iter()
                .find_map(|slot| match &slot.result {
                    Some(ExecutionResult::Completed(Verdict::Satisfied { output, .. })) => {
                        Some(output.clone())
                    }
                    _ => None,
                })
                .ok_or_else(|| invalid("no satisfied observation supplies an output"))?;
            Ok(RuntimeValue::Data(output))
        }
        "bhcp/kernel.last-satisfied-output@0" => {
            let observations = take_observations(arguments)?;
            let output = observations
                .iter()
                .rev()
                .find_map(|slot| match &slot.result {
                    Some(ExecutionResult::Completed(Verdict::Satisfied { output, .. })) => {
                        Some(output.clone())
                    }
                    _ => None,
                })
                .ok_or_else(|| invalid("no satisfied observation supplies an output"))?;
            Ok(RuntimeValue::Data(output))
        }
        "bhcp/kernel.first-satisfied-winner@0" => {
            let observations = take_observations(arguments)?;
            let (tag, output) = observations
                .iter()
                .find_map(|slot| match &slot.result {
                    Some(ExecutionResult::Completed(Verdict::Satisfied { output, .. })) => {
                        Some((slot.tag.clone(), output.clone()))
                    }
                    _ => None,
                })
                .ok_or_else(|| invalid("no satisfied observation supplies a winner"))?;
            Ok(RuntimeValue::Data(Value::map([
                ("output", output),
                ("tag", Value::Text(tag)),
            ])))
        }
        "bhcp/kernel.first-satisfied-evidence@0" => {
            let observations = take_observations(arguments)?;
            let evidence = observations
                .iter()
                .find_map(|slot| match &slot.result {
                    Some(ExecutionResult::Completed(Verdict::Satisfied { evidence, .. })) => {
                        Some(evidence.clone())
                    }
                    _ => None,
                })
                .ok_or_else(|| invalid("no satisfied observation supplies evidence"))?;
            Ok(RuntimeValue::Texts(evidence))
        }
        "bhcp/kernel.satisfied-evidence@0" => with_observations(arguments, |observations| {
            RuntimeValue::Texts(unique_refs(observations.iter().flat_map(
                |slot| match &slot.result {
                    Some(ExecutionResult::Completed(Verdict::Satisfied { evidence, .. })) => {
                        evidence.clone()
                    }
                    _ => vec![],
                },
            )))
        }),
        "bhcp/kernel.unit@0" if arguments.is_empty() => {
            Ok(RuntimeValue::Data(Value::Array(vec![Value::Text(
                "unit".to_owned(),
            )])))
        }
        "bhcp/kernel.pending@0" => match arguments.as_slice() {
            [RuntimeValue::Texts(tags)] if !tags.is_empty() => {
                Ok(RuntimeValue::Reduction(Reduction::Pending {
                    required: tags.clone(),
                }))
            }
            _ => Err(invalid("pending requires a non-empty list of child tags")),
        },
        "bhcp/kernel.refuted@0" => match arguments.as_slice() {
            [RuntimeValue::Texts(evidence)] => Ok(RuntimeValue::Result(
                ExecutionResult::Completed(Verdict::Refuted {
                    counter_evidence: evidence.clone(),
                }),
            )),
            _ => Err(invalid("refuted requires a counter-evidence list")),
        },
        "bhcp/kernel.faulted@0" => match arguments.as_slice() {
            [RuntimeValue::Fault(fault)] => Ok(RuntimeValue::Result(ExecutionResult::Faulted(
                fault.clone(),
            ))),
            _ => Err(invalid("faulted requires an operational fault")),
        },
        "bhcp/kernel.unresolved@0" => match arguments.as_slice() {
            [RuntimeValue::Reason(reason), RuntimeValue::Texts(evidence)] => Ok(
                RuntimeValue::Result(ExecutionResult::Completed(Verdict::Unresolved {
                    reason: reason.clone(),
                    partial_evidence: evidence.clone(),
                })),
            ),
            _ => Err(invalid("unresolved requires a reason and partial evidence")),
        },
        "bhcp/kernel.satisfied@0" => match arguments.as_slice() {
            [RuntimeValue::Data(output), RuntimeValue::Texts(evidence)] => Ok(
                RuntimeValue::Result(ExecutionResult::Completed(Verdict::Satisfied {
                    output: output.clone(),
                    evidence: evidence.clone(),
                })),
            ),
            _ => Err(invalid("satisfied requires an output and evidence")),
        },
        "bhcp/kernel.conclude@0" => match arguments.as_slice() {
            [RuntimeValue::Result(result)] => {
                let mut result = result.clone();
                let premises = match &result {
                    ExecutionResult::Completed(Verdict::Satisfied { evidence, .. }) => {
                        evidence.clone()
                    }
                    ExecutionResult::Completed(Verdict::Refuted { counter_evidence }) => {
                        counter_evidence.clone()
                    }
                    ExecutionResult::Completed(Verdict::Unresolved {
                        partial_evidence, ..
                    }) => partial_evidence.clone(),
                    ExecutionResult::Faulted(_) => vec![],
                };
                let derivation = Derivation {
                    id: derivation_id.to_owned(),
                    premises: unique_refs(premises),
                };
                match &mut result {
                    ExecutionResult::Completed(Verdict::Satisfied { evidence, .. }) => {
                        evidence.push(derivation.id.clone());
                    }
                    ExecutionResult::Completed(Verdict::Refuted { counter_evidence }) => {
                        counter_evidence.push(derivation.id.clone());
                    }
                    _ => {}
                }
                Ok(RuntimeValue::Reduction(Reduction::Concluded {
                    result,
                    derivation,
                }))
            }
            _ => Err(invalid("conclude requires one execution result")),
        },
        _ => Err(invalid(format!("unknown kernel primitive {symbol}"))),
    }
}

fn with_observations(
    arguments: Vec<RuntimeValue>,
    operation: impl FnOnce(&[ObservationSlot]) -> RuntimeValue,
) -> Result<RuntimeValue> {
    let observations = take_observations(arguments)?;
    Ok(operation(&observations))
}

fn take_observations(arguments: Vec<RuntimeValue>) -> Result<Vec<ObservationSlot>> {
    let mut arguments = arguments.into_iter();
    match (arguments.next(), arguments.next()) {
        (Some(RuntimeValue::Observations(observations)), None) => Ok(observations),
        _ => Err(invalid(
            "observation operation requires one sealed observation record",
        )),
    }
}

fn derive_derivation_id(
    network: &KernelNetwork,
    parent: &Value,
    observations: &[ObservationSlot],
) -> Result<String> {
    let value = Value::Array(vec![
        network.to_value(),
        parent.clone(),
        Value::Array(
            observations
                .iter()
                .map(|slot| {
                    Value::Array(vec![
                        Value::Text(slot.tag.clone()),
                        slot.result
                            .as_ref()
                            .map(ExecutionResult::to_value)
                            .unwrap_or(Value::Null),
                    ])
                })
                .collect(),
        ),
    ]);
    let bytes = encode_deterministic(&value)?;
    let digest = HashAlgorithm::default().hash(&bytes).digest;
    let suffix: String = digest[..32]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    Ok(format!("derivation-{suffix}"))
}

fn unique_refs(values: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut seen = HashSet::new();
    values
        .into_iter()
        .filter(|value| seen.insert(value.clone()))
        .collect()
}

fn accepted_premises(observations: &[ChildObservation]) -> HashSet<String> {
    observations
        .iter()
        .flat_map(|observation| match &observation.result {
            ExecutionResult::Completed(Verdict::Satisfied { evidence, .. }) => evidence.clone(),
            ExecutionResult::Completed(Verdict::Refuted { counter_evidence }) => {
                counter_evidence.clone()
            }
            ExecutionResult::Completed(Verdict::Unresolved {
                partial_evidence, ..
            }) => partial_evidence.clone(),
            ExecutionResult::Faulted(_) => vec![],
        })
        .collect()
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

fn invalid_derivation(message: impl Into<String>) -> Diagnostic {
    Diagnostic::plain(INVALID_DERIVATION, message)
}
