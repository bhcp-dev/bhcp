//! Deterministic checking and monomorphization for parsed pure definitions.
//!
//! Source registration order is discarded before resolution. Generic templates are
//! validated even when unreachable, while only concrete inferred specializations
//! enter semantic IR. Calls close over retained definitions and never host callbacks.

use std::collections::{BTreeMap, BTreeSet};

use crate::cbor::encode_deterministic;
use crate::diagnostic::{Diagnostic, Result};
use crate::expression::ExpressionContext;
use crate::hash::HashAlgorithm;
use crate::model::{
    Point, PredicateDefinition, PredicateVerifierBinding, PureBinding, PureFunctionDefinition,
};
use crate::parser::{
    ParsedProgram, SurfaceArgumentMode, SurfaceExpression, SurfaceFunction, SurfaceLiteral,
    SurfacePredicate, SurfaceType,
};
use crate::typecheck::{CheckedType, CheckedTypeProgram, TypeRelations, surface_type};
use crate::value::Value;

const INVALID_DEFINITION: &str = "BHCP4301";

#[derive(Clone)]
enum Template {
    Function(SurfaceFunction),
    Predicate(SurfacePredicate),
}

impl Template {
    fn symbol(&self) -> &str {
        match self {
            Self::Function(definition) => &definition.symbol,
            Self::Predicate(definition) => &definition.symbol,
        }
    }

    fn type_parameters(&self) -> &[String] {
        match self {
            Self::Function(definition) => &definition.type_parameters,
            Self::Predicate(definition) => &definition.type_parameters,
        }
    }

    fn type_parameter_bounds(&self) -> &[Option<SurfaceType>] {
        match self {
            Self::Function(definition) => &definition.type_parameter_bounds,
            Self::Predicate(definition) => &definition.type_parameter_bounds,
        }
    }

    fn parameters(&self) -> &[crate::parser::SurfaceParameter] {
        match self {
            Self::Function(definition) => &definition.parameters,
            Self::Predicate(definition) => &definition.parameters,
        }
    }

    fn result(&self) -> Option<&SurfaceType> {
        match self {
            Self::Function(definition) => Some(&definition.result),
            Self::Predicate(_) => None,
        }
    }

    fn at(&self) -> &Point {
        match self {
            Self::Function(definition) => &definition.at,
            Self::Predicate(definition) => &definition.at,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedCall {
    pub symbol: String,
    pub result: CheckedType,
}

#[derive(Clone, Debug)]
struct LoweredExpression {
    value: Value,
    value_type: CheckedType,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct InstantiationKey {
    base_symbol: String,
    type_arguments: Vec<u8>,
}

impl InstantiationKey {
    fn new(base_symbol: &str, type_arguments: &[CheckedType]) -> Result<Self> {
        Ok(Self {
            base_symbol: base_symbol.to_owned(),
            type_arguments: encode_deterministic(&Value::Array(
                type_arguments.iter().map(CheckedType::to_value).collect(),
            ))?,
        })
    }
}

pub(crate) struct DefinitionElaborator {
    templates: BTreeMap<String, Template>,
    relations: TypeRelations,
    source_name: String,
    context: ExpressionContext,
    visiting: BTreeSet<String>,
    completed: BTreeMap<InstantiationKey, ResolvedCall>,
    emitted_symbols: BTreeMap<String, InstantiationKey>,
    functions: BTreeMap<String, PureFunctionDefinition>,
    predicates: BTreeMap<String, PredicateDefinition>,
    next_function: usize,
    next_predicate: usize,
    next_parameter: usize,
    next_expression: usize,
}

impl DefinitionElaborator {
    pub(crate) fn new(
        program: &ParsedProgram,
        checked_types: &CheckedTypeProgram,
        source_name: &str,
    ) -> Result<Self> {
        let mut templates = BTreeMap::new();
        for definition in &program.functions {
            templates.insert(
                definition.symbol.clone(),
                Template::Function(definition.clone()),
            );
        }
        for definition in &program.predicates {
            if definition.definition.is_none() && definition.verifier.is_none() {
                return Err(invalid_at(
                    "predicate requires a definition or verifier binding",
                    source_name,
                    &definition.at,
                ));
            }
            templates.insert(
                definition.symbol.clone(),
                Template::Predicate(definition.clone()),
            );
        }
        Ok(Self {
            templates,
            relations: checked_types.relations.clone(),
            source_name: source_name.to_owned(),
            context: ExpressionContext::default(),
            visiting: BTreeSet::new(),
            completed: BTreeMap::new(),
            emitted_symbols: BTreeMap::new(),
            functions: BTreeMap::new(),
            predicates: BTreeMap::new(),
            next_function: 0,
            next_predicate: 0,
            next_parameter: 0,
            next_expression: 0,
        })
    }

    pub(crate) fn elaborate_roots(&mut self) -> Result<()> {
        let roots = self
            .templates
            .values()
            .filter(|template| template.type_parameters().is_empty())
            .map(|template| template.symbol().to_owned())
            .collect::<Vec<_>>();
        for symbol in roots {
            let template = self.templates[&symbol].clone();
            let argument_types = template
                .parameters()
                .iter()
                .map(|parameter| surface_type(&parameter.value_type, &[]))
                .collect::<Result<Vec<_>>>()?;
            self.instantiate(&symbol, &argument_types, template.at())?;
        }
        let generic_roots = self
            .templates
            .values()
            .filter(|template| !template.type_parameters().is_empty())
            .map(|template| template.symbol().to_owned())
            .collect::<Vec<_>>();
        for symbol in generic_roots {
            let template = self.templates[&symbol].clone();
            let type_arguments = self.validation_type_arguments(&template)?;
            let arguments = template
                .parameters()
                .iter()
                .map(|parameter| {
                    surface_type(&parameter.value_type, template.type_parameters())
                        .and_then(|value| substitute_checked_type(&value, &type_arguments))
                })
                .collect::<Result<Vec<_>>>()?;
            let mut validator = self.validation_fork();
            validator.instantiate(&symbol, &arguments, template.at())?;
        }
        Ok(())
    }

    pub(crate) fn resolve_call(
        &mut self,
        symbol: &str,
        argument_types: &[CheckedType],
        at: &Point,
    ) -> Result<ResolvedCall> {
        match self.templates.get(symbol) {
            None => {
                return Err(invalid_at(
                    format!("unresolved pure definition {symbol:?}"),
                    &self.source_name,
                    at,
                ));
            }
            Some(Template::Predicate(predicate)) if predicate.definition.is_none() => {
                return Err(invalid_at(
                    format!("predicate {symbol:?} has no retained pure body"),
                    &self.source_name,
                    at,
                ));
            }
            Some(_) => {}
        }
        self.instantiate(symbol, argument_types, at)
    }

    pub(crate) fn finish(self) -> (Vec<PureFunctionDefinition>, Vec<PredicateDefinition>) {
        (
            self.functions.into_values().collect(),
            self.predicates.into_values().collect(),
        )
    }

    fn instantiate(
        &mut self,
        base_symbol: &str,
        argument_types: &[CheckedType],
        call_at: &Point,
    ) -> Result<ResolvedCall> {
        let template = self.templates.get(base_symbol).cloned().ok_or_else(|| {
            invalid_at(
                format!("unresolved pure definition {base_symbol:?}"),
                &self.source_name,
                call_at,
            )
        })?;
        if argument_types.len() != template.parameters().len() {
            return Err(invalid_at(
                format!("pure definition {base_symbol:?} has the wrong call arity"),
                &self.source_name,
                call_at,
            ));
        }
        let type_arguments = self.infer_type_arguments(&template, argument_types, call_at)?;
        let key = InstantiationKey::new(base_symbol, &type_arguments)?;
        let symbol = specialized_symbol(base_symbol, &type_arguments)?;
        if let Some(completed) = self.completed.get(&key) {
            return Ok(completed.clone());
        }
        if (symbol != base_symbol && self.templates.contains_key(&symbol))
            || self
                .emitted_symbols
                .get(&symbol)
                .is_some_and(|owner| owner != &key)
        {
            return Err(invalid_at(
                format!("specialization symbol collision for {symbol:?}"),
                &self.source_name,
                call_at,
            ));
        }
        self.emitted_symbols.insert(symbol.clone(), key.clone());
        if !self.visiting.insert(base_symbol.to_owned()) {
            return Err(invalid_at(
                format!("recursive function cycle reaches {base_symbol:?}"),
                &self.source_name,
                call_at,
            ));
        }

        let result = self.instantiate_template_type(
            template.result(),
            template.type_parameters(),
            &type_arguments,
        )?;
        let result = result.unwrap_or(bool_type()?);
        let mut environment = BTreeMap::new();
        let mut parameters = Vec::with_capacity(template.parameters().len());
        for (parameter, argument_type) in template.parameters().iter().zip(argument_types) {
            self.next_parameter += 1;
            let binding = PureBinding {
                id: format!("pure-parameter-{}", self.next_parameter),
                value_type: argument_type.clone(),
            };
            environment.insert(parameter.name.clone(), binding.clone());
            parameters.push(binding);
        }

        let lowered = match &template {
            Template::Function(definition) => {
                let lowered = self.lower_expression(&definition.definition, &environment)?;
                require_same_type(
                    &lowered.value_type,
                    &result,
                    "function result",
                    &self.source_name,
                    &definition.at,
                )?;
                Some(lowered)
            }
            Template::Predicate(definition) => definition
                .definition
                .as_ref()
                .map(|expression| self.lower_expression(expression, &environment))
                .transpose()?
                .map(|lowered| {
                    require_same_type(
                        &lowered.value_type,
                        &bool_type()?,
                        "predicate result",
                        &self.source_name,
                        &definition.at,
                    )?;
                    Ok(lowered)
                })
                .transpose()?,
        };
        let checked = if let Some(lowered) = &lowered {
            let (context, checked) = self
                .context
                .clone()
                .define_checked(
                    symbol.clone(),
                    parameters
                        .iter()
                        .map(|parameter| (parameter.id.clone(), parameter.value_type.clone()))
                        .collect(),
                    result.clone(),
                    &lowered.value,
                )
                .map_err(|diagnostic| {
                    invalid_at(
                        format!("definition expression is invalid: {}", diagnostic.message),
                        &self.source_name,
                        template.at(),
                    )
                })?;
            self.context = context;
            Some(checked)
        } else {
            None
        };

        match template {
            Template::Function(_) => {
                self.next_function += 1;
                self.functions.insert(
                    symbol.clone(),
                    PureFunctionDefinition {
                        id: format!("pure-function-{}", self.next_function),
                        symbol: symbol.clone(),
                        parameters,
                        result: result.clone(),
                        definition: checked.expect("functions always have a definition"),
                    },
                );
            }
            Template::Predicate(definition) => {
                let verifier = definition
                    .verifier
                    .as_ref()
                    .map(|binding| self.lower_verifier(binding, &environment))
                    .transpose()?;
                self.next_predicate += 1;
                self.predicates.insert(
                    symbol.clone(),
                    PredicateDefinition {
                        id: format!("predicate-{}", self.next_predicate),
                        symbol: symbol.clone(),
                        parameters,
                        definition: checked,
                        verifier,
                    },
                );
            }
        }
        self.visiting.remove(base_symbol);
        let resolved = ResolvedCall { symbol, result };
        self.completed.insert(key, resolved.clone());
        Ok(resolved)
    }

    fn infer_type_arguments(
        &self,
        template: &Template,
        argument_types: &[CheckedType],
        at: &Point,
    ) -> Result<Vec<CheckedType>> {
        let type_parameters = template.type_parameters();
        let mut inferred = vec![None; type_parameters.len()];
        for (parameter, actual) in template.parameters().iter().zip(argument_types) {
            let expected = surface_type(&parameter.value_type, type_parameters)?;
            unify_type(&expected.to_value(), &actual.to_value(), &mut inferred)
                .map_err(|message| invalid_at(message, &self.source_name, at))?;
        }
        let inferred = inferred
            .into_iter()
            .enumerate()
            .map(|(index, argument)| {
                argument.ok_or_else(|| {
                    invalid_at(
                        format!(
                            "generic parameter {:?} cannot be soundly inferred",
                            type_parameters[index]
                        ),
                        &self.source_name,
                        at,
                    )
                })
            })
            .collect::<Result<Vec<_>>>()?;
        for (index, bound) in template.type_parameter_bounds().iter().enumerate() {
            let Some(bound) = bound else {
                continue;
            };
            let bound = surface_type(bound, type_parameters)?;
            let bound = substitute_checked_type(&bound, &inferred)?;
            if !is_dynamic(&bound) && !inferred[index].is_subtype_of(&bound, &self.relations) {
                return Err(invalid_at(
                    format!(
                        "generic argument does not satisfy bound for parameter {:?}",
                        type_parameters[index]
                    ),
                    &self.source_name,
                    at,
                ));
            }
        }
        Ok(inferred)
    }

    fn instantiate_template_type(
        &self,
        value: Option<&SurfaceType>,
        parameters: &[String],
        arguments: &[CheckedType],
    ) -> Result<Option<CheckedType>> {
        value
            .map(|value| {
                surface_type(value, parameters)
                    .and_then(|value| substitute_checked_type(&value, arguments))
            })
            .transpose()
    }

    fn validation_type_arguments(&self, template: &Template) -> Result<Vec<CheckedType>> {
        let mut arguments = Vec::with_capacity(template.type_parameters().len());
        for bound in template.type_parameter_bounds() {
            let argument = bound
                .as_ref()
                .map(|bound| {
                    surface_type(bound, template.type_parameters())
                        .and_then(|bound| substitute_checked_type(&bound, &arguments))
                })
                .transpose()?
                .unwrap_or(dynamic_type()?);
            arguments.push(argument);
        }
        Ok(arguments)
    }

    fn validation_fork(&self) -> Self {
        Self {
            templates: self.templates.clone(),
            relations: self.relations.clone(),
            source_name: self.source_name.clone(),
            context: ExpressionContext::default(),
            visiting: BTreeSet::new(),
            completed: BTreeMap::new(),
            emitted_symbols: BTreeMap::new(),
            functions: BTreeMap::new(),
            predicates: BTreeMap::new(),
            next_function: 0,
            next_predicate: 0,
            next_parameter: 0,
            next_expression: 0,
        }
    }

    fn lower_expression(
        &mut self,
        expression: &SurfaceExpression,
        environment: &BTreeMap<String, PureBinding>,
    ) -> Result<LoweredExpression> {
        let (value_type, form) = match expression {
            SurfaceExpression::Literal { value, .. } => match value {
                SurfaceLiteral::Bool(value) => (
                    bool_type()?,
                    Value::Array(vec![Value::Text("literal".to_owned()), Value::Bool(*value)]),
                ),
                SurfaceLiteral::Text(value) => (
                    primitive_type("Text")?,
                    Value::Array(vec![
                        Value::Text("literal".to_owned()),
                        Value::Text(value.clone()),
                    ]),
                ),
                SurfaceLiteral::Integer(value) => (
                    exact_type("Integer")?,
                    Value::Array(vec![
                        Value::Text("literal".to_owned()),
                        Value::Array(vec![
                            Value::Text("integer".to_owned()),
                            Value::Integer(i128::from(*value)),
                        ]),
                    ]),
                ),
            },
            SurfaceExpression::Reference { name, at } => {
                let binding = environment.get(name).ok_or_else(|| {
                    invalid_at(
                        format!("unresolved definition binding {name:?}"),
                        &self.source_name,
                        at,
                    )
                })?;
                (
                    binding.value_type.clone(),
                    Value::Array(vec![
                        Value::Text("reference".to_owned()),
                        Value::Text(binding.id.clone()),
                    ]),
                )
            }
            SurfaceExpression::Unary {
                operator,
                operand,
                at,
            } => {
                let operand = self.lower_expression(operand, environment)?;
                let valid = (operator == "!" && operand.value_type == bool_type()?)
                    || (operator == "-" && is_exact_number(&operand.value_type));
                if !valid {
                    return Err(invalid_at(
                        format!("definition unary operator {operator:?} is ill-typed"),
                        &self.source_name,
                        at,
                    ));
                }
                (
                    operand.value_type.clone(),
                    Value::Array(vec![
                        Value::Text("unary".to_owned()),
                        Value::Text(operator.clone()),
                        operand.value,
                    ]),
                )
            }
            SurfaceExpression::Binary {
                operator,
                left,
                right,
                at,
            } => {
                let left = self.lower_expression(left, environment)?;
                let right = self.lower_expression(right, environment)?;
                require_same_type(
                    &left.value_type,
                    &right.value_type,
                    "binary operands",
                    &self.source_name,
                    at,
                )?;
                let value_type = match operator.as_str() {
                    "==" | "!=" => bool_type()?,
                    "<" | "<=" | ">" | ">=" if is_ordered(&left.value_type) => bool_type()?,
                    "&&" | "||" if left.value_type == bool_type()? => bool_type()?,
                    "+" if left.value_type == primitive_type("Text")?
                        || is_exact_number(&left.value_type) =>
                    {
                        left.value_type.clone()
                    }
                    "-" | "*" | "/" | "%" if is_exact_number(&left.value_type) => {
                        left.value_type.clone()
                    }
                    _ => {
                        return Err(invalid_at(
                            format!("definition binary operator {operator:?} is ill-typed"),
                            &self.source_name,
                            at,
                        ));
                    }
                };
                (
                    value_type,
                    Value::Array(vec![
                        Value::Text("binary".to_owned()),
                        Value::Text(operator.clone()),
                        left.value,
                        right.value,
                    ]),
                )
            }
            SurfaceExpression::Call {
                function,
                arguments,
                at,
            } => {
                let arguments = arguments
                    .iter()
                    .map(|argument| self.lower_expression(argument, environment))
                    .collect::<Result<Vec<_>>>()?;
                let argument_types = arguments
                    .iter()
                    .map(|argument| argument.value_type.clone())
                    .collect::<Vec<_>>();
                let resolved = self.resolve_call(function, &argument_types, at)?;
                (
                    resolved.result,
                    Value::Array(vec![
                        Value::Text("call".to_owned()),
                        Value::Text(resolved.symbol),
                        Value::Array(
                            arguments
                                .into_iter()
                                .map(|argument| argument.value)
                                .collect(),
                        ),
                    ]),
                )
            }
            SurfaceExpression::If {
                condition,
                consequent,
                alternative,
                at,
            } => {
                let condition = self.lower_expression(condition, environment)?;
                require_same_type(
                    &condition.value_type,
                    &bool_type()?,
                    "if condition",
                    &self.source_name,
                    at,
                )?;
                let consequent = self.lower_expression(consequent, environment)?;
                let alternative = self.lower_expression(alternative, environment)?;
                require_same_type(
                    &consequent.value_type,
                    &alternative.value_type,
                    "if branches",
                    &self.source_name,
                    at,
                )?;
                (
                    consequent.value_type.clone(),
                    Value::Array(vec![
                        Value::Text("if".to_owned()),
                        condition.value,
                        consequent.value,
                        alternative.value,
                    ]),
                )
            }
        };
        self.next_expression += 1;
        Ok(LoweredExpression {
            value: Value::map([
                (
                    "id",
                    Value::Text(format!("pure-expression-{}", self.next_expression)),
                ),
                ("type", value_type.to_value()),
                ("form", form),
            ]),
            value_type,
        })
    }

    fn lower_verifier(
        &mut self,
        verifier: &crate::parser::SurfaceVerifierBinding,
        environment: &BTreeMap<String, PureBinding>,
    ) -> Result<PredicateVerifierBinding> {
        let mut source_arguments = verifier.arguments.iter().collect::<Vec<_>>();
        source_arguments.sort_by(|left, right| left.name.cmp(&right.name));
        let arguments = source_arguments
            .into_iter()
            .map(|argument| {
                let lowered = self.lower_expression(&argument.value, environment)?;
                Ok((argument, lowered))
            })
            .collect::<Result<Vec<_>>>()?;
        let input = CheckedType::from_value(&Value::Array(vec![
            Value::Text("record".to_owned()),
            Value::Bool(false),
            Value::Array(
                arguments
                    .iter()
                    .map(|(argument, lowered)| {
                        Value::Array(vec![
                            Value::Text(argument.name.clone()),
                            lowered.value_type.to_value(),
                            Value::Bool(false),
                        ])
                    })
                    .collect(),
            ),
        ]))?;
        let configuration = (!arguments.is_empty()).then(|| {
            Value::Array(
                arguments
                    .into_iter()
                    .map(|(argument, lowered)| {
                        Value::map([
                            ("name", Value::Text(argument.name.clone())),
                            (
                                "mode",
                                Value::Text(
                                    match argument.mode {
                                        SurfaceArgumentMode::Value => "value",
                                        SurfaceArgumentMode::Move => "move",
                                        SurfaceArgumentMode::Borrow => "borrow",
                                        SurfaceArgumentMode::Share => "share",
                                    }
                                    .to_owned(),
                                ),
                            ),
                            ("value", lowered.value),
                        ])
                    })
                    .collect(),
            )
        });
        Ok(PredicateVerifierBinding {
            verifier: verifier.symbol.clone(),
            input,
            output: evidence_type()?,
            configuration,
            trust: vec![],
        })
    }
}

fn specialized_symbol(base: &str, arguments: &[CheckedType]) -> Result<String> {
    if arguments.is_empty() {
        return Ok(base.to_owned());
    }
    let (path, version) = base
        .rsplit_once('@')
        .ok_or_else(|| invalid_plain("pure definition is not a versioned semantic name"))?;
    let value = Value::Array(arguments.iter().map(CheckedType::to_value).collect());
    let digest = HashAlgorithm::default()
        .hash(&encode_deterministic(&value)?)
        .digest;
    let suffix = digest[..16]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Ok(format!("{path}-{suffix}@{version}"))
}

fn substitute_checked_type(value: &CheckedType, arguments: &[CheckedType]) -> Result<CheckedType> {
    fn substitute(value: &Value, arguments: &[CheckedType]) -> Result<Value> {
        if let Value::Array(parts) = value
            && let [Value::Text(tag), Value::Integer(index)] = parts.as_slice()
            && tag == "parameter"
        {
            let index = usize::try_from(*index)
                .map_err(|_| invalid_plain("generic parameter index is invalid"))?;
            return arguments
                .get(index)
                .map(CheckedType::to_value)
                .ok_or_else(|| invalid_plain("generic parameter does not resolve"));
        }
        Ok(match value {
            Value::Array(values) => Value::Array(
                values
                    .iter()
                    .map(|value| substitute(value, arguments))
                    .collect::<Result<_>>()?,
            ),
            Value::Map(entries) => Value::owned_map(
                entries
                    .iter()
                    .map(|(key, value)| Ok((key.clone(), substitute(value, arguments)?)))
                    .collect::<Result<Vec<_>>>()?,
            ),
            Value::Tag(tag, nested) => Value::Tag(*tag, Box::new(substitute(nested, arguments)?)),
            other => other.clone(),
        })
    }
    CheckedType::from_value(&substitute(&value.to_value(), arguments)?)
}

fn unify_type(
    expected: &Value,
    actual: &Value,
    inferred: &mut [Option<CheckedType>],
) -> std::result::Result<(), String> {
    if let Value::Array(parts) = expected
        && let [Value::Text(tag), Value::Integer(index)] = parts.as_slice()
        && tag == "parameter"
    {
        let index =
            usize::try_from(*index).map_err(|_| "generic parameter index is invalid".to_owned())?;
        let checked = CheckedType::from_value(actual).map_err(|error| error.message)?;
        let slot = inferred
            .get_mut(index)
            .ok_or_else(|| "generic parameter does not resolve".to_owned())?;
        if slot.as_ref().is_some_and(|existing| existing != &checked) {
            return Err("generic parameter inference is inconsistent".to_owned());
        }
        *slot = Some(checked);
        return Ok(());
    }
    match (expected, actual) {
        (Value::Array(expected), Value::Array(actual)) if expected.len() == actual.len() => {
            for (expected, actual) in expected.iter().zip(actual) {
                unify_type(expected, actual, inferred)?;
            }
            Ok(())
        }
        (Value::Map(expected), Value::Map(actual)) if expected.len() == actual.len() => {
            for ((expected_key, expected), (actual_key, actual)) in expected.iter().zip(actual) {
                if expected_key != actual_key {
                    return Err(
                        "pure function argument type does not match its parameter".to_owned()
                    );
                }
                unify_type(expected, actual, inferred)?;
            }
            Ok(())
        }
        _ if expected == actual => Ok(()),
        _ => Err("pure function argument type does not match its parameter".to_owned()),
    }
}

fn require_same_type(
    actual: &CheckedType,
    expected: &CheckedType,
    context: &str,
    source_name: &str,
    at: &Point,
) -> Result<()> {
    if actual == expected {
        Ok(())
    } else {
        Err(invalid_at(
            format!("{context} type does not match its declaration"),
            source_name,
            at,
        ))
    }
}

fn primitive_type(name: &str) -> Result<CheckedType> {
    CheckedType::from_value(&Value::Array(vec![
        Value::Text("primitive".to_owned()),
        Value::Text(name.to_owned()),
    ]))
}

fn bool_type() -> Result<CheckedType> {
    primitive_type("Bool")
}

fn exact_type(name: &str) -> Result<CheckedType> {
    CheckedType::from_value(&Value::Array(vec![
        Value::Text("exact-number".to_owned()),
        Value::Text(name.to_owned()),
    ]))
}

fn evidence_type() -> Result<CheckedType> {
    CheckedType::from_value(&Value::Array(vec![
        Value::Text("evidence".to_owned()),
        Value::Array(vec![]),
    ]))
}

fn dynamic_type() -> Result<CheckedType> {
    CheckedType::from_value(&Value::Array(vec![
        Value::Text("special".to_owned()),
        Value::Text("Dynamic".to_owned()),
    ]))
}

fn is_exact_number(value: &CheckedType) -> bool {
    matches!(value.to_value(), Value::Array(ref parts) if matches!(parts.as_slice(), [Value::Text(tag), Value::Text(_)] if tag == "exact-number"))
}

fn is_dynamic(value: &CheckedType) -> bool {
    matches!(value.to_value(), Value::Array(ref parts) if parts == &[Value::Text("special".to_owned()), Value::Text("Dynamic".to_owned())])
}

fn is_ordered(value: &CheckedType) -> bool {
    is_exact_number(value) || value == &primitive_type("Text").expect("Text is a valid type")
}

fn invalid_at(message: impl Into<String>, source_name: &str, at: &Point) -> Diagnostic {
    Diagnostic::new(INVALID_DEFINITION, message, source_name, at.line, at.column)
}

fn invalid_plain(message: impl Into<String>) -> Diagnostic {
    Diagnostic::plain(INVALID_DEFINITION, message)
}
