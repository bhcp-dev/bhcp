use std::collections::{BTreeMap, BTreeSet};

use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Signed, ToPrimitive, Zero};

use crate::cbor::encode_deterministic;
use crate::diagnostic::{Diagnostic, Result};
use crate::model::is_symbol;
use crate::typecheck::{CheckedType, RefinementEvidence};
use crate::value::Value;

const INVALID_EXPRESSION: &str = "BHCP4201";
const EXPRESSION_FAULT: &str = "BHCP4202";
const MAX_EXECUTABLE_DECIMAL_EXPONENT: u32 = 4096;

#[derive(Clone, Debug, Default)]
pub struct ExpressionContext {
    bindings: BTreeMap<String, CheckedType>,
    functions: BTreeMap<String, CheckedFunction>,
    types: BTreeMap<String, RegisteredType>,
}

#[derive(Clone, Debug)]
struct RegisteredType {
    parameter_count: usize,
    definition: CheckedType,
}

#[derive(Clone, Debug, Default)]
pub struct EvaluationContext {
    bindings: BTreeMap<String, Value>,
    quantifier_witnesses: BTreeMap<Vec<u8>, QuantifierWitness>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedExpression {
    value: Value,
    value_type: CheckedType,
    node: Node,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Node {
    Literal(Value),
    Reference(String),
    Record(Vec<(String, CheckedExpression)>),
    Tuple(Vec<CheckedExpression>),
    Variant(String, Box<CheckedExpression>),
    Collection(CollectionKind, Vec<CheckedExpression>),
    Map(Vec<(CheckedExpression, CheckedExpression)>),
    Select(Box<CheckedExpression>, Selector),
    Unary(String, Box<CheckedExpression>),
    Binary(String, Box<CheckedExpression>, Box<CheckedExpression>),
    If {
        condition: Box<CheckedExpression>,
        then_branch: Box<CheckedExpression>,
        else_branch: Box<CheckedExpression>,
    },
    Let {
        binding: String,
        initializer: Box<CheckedExpression>,
        body: Box<CheckedExpression>,
    },
    Match {
        subject: Box<CheckedExpression>,
        arms: Vec<MatchArm>,
    },
    Quantify {
        quantifier: Quantifier,
        binding: String,
        domain: Box<CheckedExpression>,
        predicate: Box<CheckedExpression>,
        verifier: Option<VerifierRequirement>,
    },
    Call {
        parameters: Vec<String>,
        arguments: Vec<CheckedExpression>,
        definition: Box<CheckedExpression>,
    },
    CastDynamic(Box<CheckedExpression>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CheckedFunction {
    parameters: Vec<(String, CheckedType)>,
    result: CheckedType,
    definition: CheckedExpression,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct VerifierRequirement {
    verifier: String,
    output: CheckedType,
    trust: BTreeSet<String>,
}

#[derive(Clone, Debug)]
struct QuantifierWitness {
    verifier: String,
    evidence: Value,
    finite_domain: Vec<Value>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CollectionKind {
    List,
    Set,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Selector {
    Field(String),
    Index(usize),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct MatchArm {
    pattern: CheckedPattern,
    guard: Option<CheckedExpression>,
    body: CheckedExpression,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CheckedPattern {
    node: PatternNode,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum PatternNode {
    Wildcard,
    Literal(Value),
    Bind(String),
    Variant(String, Vec<CheckedPattern>),
    Tuple(Vec<CheckedPattern>),
    Record(Vec<(String, CheckedPattern)>),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Quantifier {
    ForAll,
    Exists,
}

impl ExpressionContext {
    pub fn define_type(
        mut self,
        symbol: impl Into<String>,
        parameter_count: usize,
        definition: CheckedType,
    ) -> Result<Self> {
        let symbol = symbol.into();
        if !is_symbol(&symbol)
            || self
                .types
                .insert(
                    symbol,
                    RegisteredType {
                        parameter_count,
                        definition,
                    },
                )
                .is_some()
        {
            return Err(invalid("type symbol is invalid or duplicated"));
        }
        Ok(self)
    }

    pub fn bind(mut self, id: impl Into<String>, value_type: CheckedType) -> Result<Self> {
        let id = id.into();
        if id.is_empty() || self.bindings.insert(id, value_type).is_some() {
            return Err(invalid(
                "expression binding identity is empty or duplicated",
            ));
        }
        Ok(self)
    }

    pub fn define(
        self,
        symbol: impl Into<String>,
        parameters: Vec<(String, CheckedType)>,
        result: CheckedType,
        definition: &Value,
    ) -> Result<Self> {
        self.define_checked(symbol, parameters, result, definition)
            .map(|(context, _)| context)
    }

    pub(crate) fn define_checked(
        mut self,
        symbol: impl Into<String>,
        parameters: Vec<(String, CheckedType)>,
        result: CheckedType,
        definition: &Value,
    ) -> Result<(Self, CheckedExpression)> {
        let symbol = symbol.into();
        if !is_symbol(&symbol) || self.functions.contains_key(&symbol) {
            return Err(invalid("pure function symbol is invalid or duplicated"));
        }
        let mut bindings = BTreeMap::new();
        for (id, value_type) in &parameters {
            if id.is_empty() || bindings.insert(id.clone(), value_type.clone()).is_some() {
                return Err(invalid(
                    "pure function parameter identity is empty or duplicated",
                ));
            }
        }
        let checked = check_expression(
            definition,
            &bindings,
            &self.functions,
            &self.types,
            &mut BTreeSet::new(),
        )?;
        require_type(&checked.value_type, &result, "pure function result")?;
        self.functions.insert(
            symbol,
            CheckedFunction {
                parameters,
                result,
                definition: checked.clone(),
            },
        );
        Ok((self, checked))
    }
}

impl EvaluationContext {
    pub fn bind(mut self, id: impl Into<String>, value: Value) -> Result<Self> {
        let id = id.into();
        if id.is_empty() || self.bindings.insert(id, value).is_some() {
            return Err(invalid(
                "evaluation binding identity is empty or duplicated",
            ));
        }
        Ok(self)
    }

    pub fn accept_quantifier_witness(
        mut self,
        expression: &CheckedExpression,
        verifier: impl Into<String>,
        evidence: Value,
        finite_domain: Vec<Value>,
    ) -> Result<Self> {
        let Node::Quantify {
            verifier: Some(requirement),
            ..
        } = &expression.node
        else {
            return Err(invalid(
                "accepted quantifier evidence requires a verifier-backed checked expression",
            ));
        };
        let verifier = verifier.into();
        if verifier != requirement.verifier {
            return Err(invalid(
                "quantifier evidence came from a different verifier",
            ));
        }
        requirement
            .output
            .validate_value(&evidence, &RefinementEvidence::default())?;
        let Value::Array(classes) = &evidence else {
            return Err(invalid(
                "quantifier evidence must retain its accepted classes",
            ));
        };
        if requirement
            .trust
            .iter()
            .any(|class| !classes.contains(&Value::Text(class.clone())))
        {
            return Err(invalid(
                "quantifier evidence does not satisfy its trust restriction",
            ));
        }
        let expression_key = encode_deterministic(&expression.value)?;
        if self
            .quantifier_witnesses
            .insert(
                expression_key,
                QuantifierWitness {
                    verifier,
                    evidence,
                    finite_domain,
                },
            )
            .is_some()
        {
            return Err(invalid(
                "quantifier witness identity is empty or duplicated",
            ));
        }
        Ok(self)
    }
}

impl CheckedExpression {
    pub fn check(value: &Value, context: &ExpressionContext) -> Result<Self> {
        check_expression(
            value,
            &context.bindings,
            &context.functions,
            &context.types,
            &mut BTreeSet::new(),
        )
    }

    pub fn to_value(&self) -> Value {
        self.value.clone()
    }

    pub fn value_type(&self) -> &CheckedType {
        &self.value_type
    }

    pub fn evaluate(&self, context: &EvaluationContext) -> Result<Value> {
        evaluate(self, &context.bindings, context)
    }
}

fn check_expression(
    value: &Value,
    bindings: &BTreeMap<String, CheckedType>,
    functions: &BTreeMap<String, CheckedFunction>,
    types: &BTreeMap<String, RegisteredType>,
    expression_ids: &mut BTreeSet<String>,
) -> Result<CheckedExpression> {
    let Value::Map(entries) = value else {
        return Err(invalid("expression must be a closed map"));
    };
    let Some(Value::Text(id)) = value.get("id") else {
        return Err(invalid("expression identity must be non-empty text"));
    };
    if entries.len() != 3
        || id.is_empty()
        || value.get("type").is_none()
        || value.get("form").is_none()
    {
        return Err(invalid(
            "expression must contain exactly id, type, and form",
        ));
    }
    if !expression_ids.insert(id.clone()) {
        return Err(invalid(format!("duplicate expression identity {id:?}")));
    }
    let value_type = CheckedType::from_value(value.get("type").unwrap())?;
    let Value::Array(form) = value.get("form").unwrap() else {
        return Err(invalid("expression form must be an array"));
    };
    let node = match form.as_slice() {
        [Value::Text(tag), literal] if tag == "literal" => {
            value_type.validate_value(literal, &RefinementEvidence::default())?;
            Node::Literal(literal.clone())
        }
        [Value::Text(tag), Value::Text(reference)] if tag == "reference" => {
            let Some(binding_type) = bindings.get(reference) else {
                return Err(invalid(format!(
                    "unbound expression reference {reference:?}"
                )));
            };
            require_type(binding_type, &value_type, "expression reference")?;
            Node::Reference(reference.clone())
        }
        [Value::Text(tag), Value::Map(fields)] if tag == "record" => {
            let declared = record_fields(&value_type, types)?;
            let open = record_is_open(&value_type, types)?;
            let mut checked = Vec::with_capacity(fields.len());
            for (name, field) in fields {
                let field = check_expression(field, bindings, functions, types, expression_ids)?;
                if let Some((field_type, _)) = declared.get(name) {
                    require_type(&field.value_type, field_type, "record field")?;
                } else if !open {
                    return Err(invalid(format!(
                        "record expression has undeclared field {name:?}"
                    )));
                }
                checked.push((name.clone(), field));
            }
            for (name, (_, optional)) in &declared {
                if !optional && !fields.iter().any(|(candidate, _)| candidate == name) {
                    return Err(invalid(format!(
                        "record expression is missing field {name:?}"
                    )));
                }
            }
            Node::Record(checked)
        }
        [Value::Text(tag), Value::Array(elements)] if tag == "tuple" => {
            let declared = tuple_elements(&value_type, types)?;
            if elements.len() != declared.len() {
                return Err(invalid("tuple expression has the wrong arity"));
            }
            let mut checked = Vec::with_capacity(elements.len());
            for (element, element_type) in elements.iter().zip(&declared) {
                let element =
                    check_expression(element, bindings, functions, types, expression_ids)?;
                require_type(&element.value_type, element_type, "tuple element")?;
                checked.push(element);
            }
            Node::Tuple(checked)
        }
        [Value::Text(tag), Value::Text(case), payload] if tag == "variant" => {
            let payload_types = variant_payload(&value_type, case, types)?;
            let payload = check_expression(payload, bindings, functions, types, expression_ids)?;
            let expected = variant_payload_type(&payload_types)?;
            require_type(&payload.value_type, &expected, "variant payload")?;
            Node::Variant(case.clone(), Box::new(payload))
        }
        [Value::Text(tag), Value::Text(kind), Value::Array(elements)]
            if tag == "collection" && matches!(kind.as_str(), "list" | "set") =>
        {
            let element_type = collection_element(&value_type, kind)?;
            let mut checked = Vec::with_capacity(elements.len());
            for element in elements {
                let element =
                    check_expression(element, bindings, functions, types, expression_ids)?;
                require_type(&element.value_type, &element_type, "collection element")?;
                checked.push(element);
            }
            Node::Collection(
                if kind == "list" {
                    CollectionKind::List
                } else {
                    CollectionKind::Set
                },
                checked,
            )
        }
        [Value::Text(tag), Value::Array(entries)] if tag == "map" => {
            let (key_type, element_type) = map_types(&value_type)?;
            let mut checked = Vec::with_capacity(entries.len());
            for entry in entries {
                let [key, element] = entry.as_array()? else {
                    return Err(invalid("map expression entry must contain a key and value"));
                };
                let key = check_expression(key, bindings, functions, types, expression_ids)?;
                let element =
                    check_expression(element, bindings, functions, types, expression_ids)?;
                require_type(&key.value_type, &key_type, "map key")?;
                require_type(&element.value_type, &element_type, "map value")?;
                checked.push((key, element));
            }
            Node::Map(checked)
        }
        [Value::Text(tag), subject, selector] if tag == "select" => {
            let subject = check_expression(subject, bindings, functions, types, expression_ids)?;
            let selector = check_selector(selector)?;
            let selected_type = selected_type(&subject.value_type, &selector, types)?;
            require_type(&selected_type, &value_type, "selection result")?;
            Node::Select(Box::new(subject), selector)
        }
        [Value::Text(tag), Value::Text(operator), operand] if tag == "unary" => {
            let operand = check_expression(operand, bindings, functions, types, expression_ids)?;
            validate_unary(operator, &operand.value_type, &value_type)?;
            Node::Unary(operator.clone(), Box::new(operand))
        }
        [Value::Text(tag), Value::Text(operator), left, right] if tag == "binary" => {
            let left = check_expression(left, bindings, functions, types, expression_ids)?;
            let right = check_expression(right, bindings, functions, types, expression_ids)?;
            validate_binary(operator, &left.value_type, &right.value_type, &value_type)?;
            Node::Binary(operator.clone(), Box::new(left), Box::new(right))
        }
        [Value::Text(tag), condition, then_branch, else_branch] if tag == "if" => {
            let condition =
                check_expression(condition, bindings, functions, types, expression_ids)?;
            require_type(&condition.value_type, &bool_type()?, "if condition")?;
            let then_branch =
                check_expression(then_branch, bindings, functions, types, expression_ids)?;
            let else_branch =
                check_expression(else_branch, bindings, functions, types, expression_ids)?;
            require_type(&then_branch.value_type, &value_type, "then branch")?;
            require_type(&else_branch.value_type, &value_type, "else branch")?;
            Node::If {
                condition: Box::new(condition),
                then_branch: Box::new(then_branch),
                else_branch: Box::new(else_branch),
            }
        }
        [Value::Text(tag), binding, initializer, body] if tag == "let" => {
            let (binding_id, binding_type) = checked_binding(binding)?;
            let initializer =
                check_expression(initializer, bindings, functions, types, expression_ids)?;
            require_type(&initializer.value_type, &binding_type, "let initializer")?;
            let nested = nested_bindings(bindings, &binding_id, binding_type)?;
            let body = check_expression(body, &nested, functions, types, expression_ids)?;
            require_type(&body.value_type, &value_type, "let body")?;
            Node::Let {
                binding: binding_id,
                initializer: Box::new(initializer),
                body: Box::new(body),
            }
        }
        [Value::Text(tag), subject, Value::Array(arms)] if tag == "match" && !arms.is_empty() => {
            let subject = check_expression(subject, bindings, functions, types, expression_ids)?;
            let mut checked_arms = Vec::with_capacity(arms.len());
            for arm in arms {
                let (pattern, guard, body) = match arm.as_array()? {
                    [pattern, body] => (pattern, None, body),
                    [pattern, guard, body] => (pattern, Some(guard), body),
                    _ => {
                        return Err(invalid(
                            "match arm must contain pattern, optional guard, and body",
                        ));
                    }
                };
                let mut pattern_bindings = BTreeMap::new();
                let pattern =
                    check_pattern(pattern, &subject.value_type, &mut pattern_bindings, types)?;
                let mut nested = bindings.clone();
                for (id, value_type) in pattern_bindings {
                    if nested.insert(id, value_type).is_some() {
                        return Err(invalid("match pattern captures an existing identity"));
                    }
                }
                let guard = guard
                    .map(|guard| check_expression(guard, &nested, functions, types, expression_ids))
                    .transpose()?;
                if let Some(guard) = &guard {
                    require_type(&guard.value_type, &bool_type()?, "match guard")?;
                }
                let body = check_expression(body, &nested, functions, types, expression_ids)?;
                require_type(&body.value_type, &value_type, "match arm body")?;
                checked_arms.push(MatchArm {
                    pattern,
                    guard,
                    body,
                });
            }
            validate_exhaustive(&subject.value_type, &checked_arms, types)?;
            Node::Match {
                subject: Box::new(subject),
                arms: checked_arms,
            }
        }
        [
            Value::Text(tag),
            Value::Text(quantifier),
            binding,
            domain,
            predicate,
        ] if tag == "quantify" => {
            let quantifier = checked_quantifier(quantifier)?;
            require_type(&value_type, &bool_type()?, "quantifier result")?;
            let (binding_id, binding_type) = checked_binding(binding)?;
            let domain = check_expression(domain, bindings, functions, types, expression_ids)?;
            require_type(
                &collection_element_any(&domain.value_type)?,
                &binding_type,
                "quantifier binding",
            )?;
            let nested = nested_bindings(bindings, &binding_id, binding_type)?;
            let predicate = check_expression(predicate, &nested, functions, types, expression_ids)?;
            require_type(&predicate.value_type, &bool_type()?, "quantifier predicate")?;
            Node::Quantify {
                quantifier,
                binding: binding_id,
                domain: Box::new(domain),
                predicate: Box::new(predicate),
                verifier: None,
            }
        }
        [
            Value::Text(tag),
            Value::Text(quantifier),
            binding,
            domain,
            predicate,
            verifier,
        ] if tag == "quantify" => {
            let quantifier = checked_quantifier(quantifier)?;
            require_type(&value_type, &bool_type()?, "quantifier result")?;
            let (binding_id, binding_type) = checked_binding(binding)?;
            let domain = check_expression(domain, bindings, functions, types, expression_ids)?;
            require_type(
                &collection_element_any(&domain.value_type)?,
                &binding_type,
                "quantifier binding",
            )?;
            let verifier = checked_verifier_binding(verifier, &domain.value_type)?;
            let nested = nested_bindings(bindings, &binding_id, binding_type)?;
            let predicate = check_expression(predicate, &nested, functions, types, expression_ids)?;
            require_type(&predicate.value_type, &bool_type()?, "quantifier predicate")?;
            Node::Quantify {
                quantifier,
                binding: binding_id,
                domain: Box::new(domain),
                predicate: Box::new(predicate),
                verifier: Some(verifier),
            }
        }
        [
            Value::Text(tag),
            Value::Text(symbol),
            Value::Array(arguments),
        ] if tag == "call" => {
            let Some(function) = functions.get(symbol) else {
                return Err(invalid(
                    "expression call has no closed checked total-pure definition",
                ));
            };
            if arguments.len() != function.parameters.len() {
                return Err(invalid("pure function call has the wrong arity"));
            }
            require_type(&function.result, &value_type, "pure function call result")?;
            let mut checked = Vec::with_capacity(arguments.len());
            for (argument, (_, parameter_type)) in arguments.iter().zip(&function.parameters) {
                let argument =
                    check_expression(argument, bindings, functions, types, expression_ids)?;
                require_type(
                    &argument.value_type,
                    parameter_type,
                    "pure function argument",
                )?;
                checked.push(argument);
            }
            Node::Call {
                parameters: function
                    .parameters
                    .iter()
                    .map(|(id, _)| id.clone())
                    .collect(),
                arguments: checked,
                definition: Box::new(function.definition.clone()),
            }
        }
        [Value::Text(tag), source, target] if tag == "cast-dynamic" => {
            let source = check_expression(source, bindings, functions, types, expression_ids)?;
            require_type(&source.value_type, &dynamic_type()?, "dynamic cast source")?;
            let target = CheckedType::from_value(target)?;
            require_type(&target, &value_type, "dynamic cast target")?;
            Node::CastDynamic(Box::new(source))
        }
        _ => return Err(invalid("unsupported or malformed expression form")),
    };
    Ok(CheckedExpression {
        value: value.clone(),
        value_type,
        node,
    })
}

fn evaluate(
    expression: &CheckedExpression,
    bindings: &BTreeMap<String, Value>,
    context: &EvaluationContext,
) -> Result<Value> {
    let value = match &expression.node {
        Node::Literal(value) => value.clone(),
        Node::Reference(reference) => bindings
            .get(reference)
            .cloned()
            .ok_or_else(|| fault(format!("missing evaluated binding {reference:?}")))?,
        Node::Record(fields) => Value::owned_map(
            fields
                .iter()
                .map(|(name, value)| Ok((name.clone(), evaluate(value, bindings, context)?)))
                .collect::<Result<Vec<_>>>()?,
        ),
        Node::Tuple(elements) => Value::Array(
            elements
                .iter()
                .map(|element| evaluate(element, bindings, context))
                .collect::<Result<_>>()?,
        ),
        Node::Variant(case, payload) => Value::Array(vec![
            Value::Text("variant".to_owned()),
            Value::Text(case.clone()),
            evaluate(payload, bindings, context)?,
        ]),
        Node::Collection(kind, elements) => {
            let mut values = elements
                .iter()
                .map(|element| evaluate(element, bindings, context))
                .collect::<Result<Vec<_>>>()?;
            if *kind == CollectionKind::Set {
                values.sort_by_key(|value| encode_deterministic(value).unwrap_or_default());
                if values.windows(2).any(|pair| pair[0] == pair[1]) {
                    return Err(fault("set expression evaluates to duplicate elements"));
                }
            }
            Value::Array(values)
        }
        Node::Map(entries) => {
            let mut values = Vec::with_capacity(entries.len());
            for (key, element) in entries {
                values.push((
                    evaluate(key, bindings, context)?,
                    evaluate(element, bindings, context)?,
                ));
            }
            if map_key_is_text(&expression.value_type)? {
                let mut text_values = Vec::with_capacity(values.len());
                for (key, value) in values {
                    let Value::Text(key) = key else {
                        return Err(fault("text-keyed map received a non-Text key"));
                    };
                    if text_values.iter().any(|(candidate, _)| candidate == &key) {
                        return Err(fault(format!(
                            "map expression evaluates to duplicate key {key:?}"
                        )));
                    }
                    text_values.push((key, value));
                }
                Value::owned_map(text_values)
            } else {
                let mut encoded = values
                    .into_iter()
                    .map(|(key, value)| Ok((encode_deterministic(&key)?, key, value)))
                    .collect::<Result<Vec<_>>>()?;
                encoded.sort_by(|left, right| left.0.cmp(&right.0));
                if encoded.windows(2).any(|pair| pair[0].0 == pair[1].0) {
                    return Err(fault("map expression evaluates to a duplicate generic key"));
                }
                Value::Array(
                    encoded
                        .into_iter()
                        .map(|(_, key, value)| Value::Array(vec![key, value]))
                        .collect(),
                )
            }
        }
        Node::Select(subject, selector) => {
            evaluate_selection(evaluate(subject, bindings, context)?, selector)?
        }
        Node::Unary(operator, operand) => evaluate_unary(
            operator,
            evaluate(operand, bindings, context)?,
            &expression.value_type,
        )?,
        Node::Binary(operator, left, right) => evaluate_binary(
            operator,
            evaluate(left, bindings, context)?,
            evaluate(right, bindings, context)?,
            &expression.value_type,
        )?,
        Node::If {
            condition,
            then_branch,
            else_branch,
        } => {
            let Value::Bool(condition) = evaluate(condition, bindings, context)? else {
                return Err(fault("if condition did not evaluate to Bool"));
            };
            evaluate(
                if condition { then_branch } else { else_branch },
                bindings,
                context,
            )?
        }
        Node::Let {
            binding,
            initializer,
            body,
        } => {
            let value = evaluate(initializer, bindings, context)?;
            let mut nested = bindings.clone();
            nested.insert(binding.clone(), value);
            evaluate(body, &nested, context)?
        }
        Node::Match { subject, arms } => {
            let subject = evaluate(subject, bindings, context)?;
            let mut selected = None;
            for arm in arms {
                let Some(captures) = match_pattern(&arm.pattern, &subject)? else {
                    continue;
                };
                let mut nested = bindings.clone();
                nested.extend(captures);
                if let Some(guard) = &arm.guard {
                    let Value::Bool(guard) = evaluate(guard, &nested, context)? else {
                        return Err(fault("match guard did not evaluate to Bool"));
                    };
                    if !guard {
                        continue;
                    }
                }
                selected = Some(evaluate(&arm.body, &nested, context)?);
                break;
            }
            selected.ok_or_else(|| fault("exhaustive match selected no arm"))?
        }
        Node::Quantify {
            quantifier,
            binding,
            domain,
            predicate,
            verifier,
        } => {
            let Value::Array(mut values) = evaluate(domain, bindings, context)? else {
                return Err(fault(
                    "finite quantifier domain did not evaluate to a collection",
                ));
            };
            if let Some(requirement) = verifier {
                let key = encode_deterministic(&expression.value)?;
                let witness = context.quantifier_witnesses.get(&key).ok_or_else(|| {
                    fault(format!(
                        "verifier-backed quantifier has no accepted finite-domain evidence from {:?}",
                        requirement.verifier
                    ))
                })?;
                if witness.verifier != requirement.verifier {
                    return Err(fault(
                        "verifier-backed quantifier evidence identity changed after acceptance",
                    ));
                }
                requirement
                    .output
                    .validate_value(&witness.evidence, &RefinementEvidence::default())
                    .map_err(|error| {
                        fault(format!(
                            "verifier-backed quantifier evidence became invalid: {}",
                            error.message
                        ))
                    })?;
                if witness.finite_domain != values {
                    return Err(fault(
                        "verifier-backed quantifier witness differs from its evaluated finite domain",
                    ));
                }
                values.clone_from(&witness.finite_domain);
            }
            let mut result = *quantifier == Quantifier::ForAll;
            for value in values {
                let mut nested = bindings.clone();
                nested.insert(binding.clone(), value);
                let Value::Bool(predicate) = evaluate(predicate, &nested, context)? else {
                    return Err(fault("quantifier predicate did not evaluate to Bool"));
                };
                match quantifier {
                    Quantifier::ForAll if !predicate => {
                        result = false;
                        break;
                    }
                    Quantifier::Exists if predicate => {
                        result = true;
                        break;
                    }
                    Quantifier::ForAll => result = true,
                    Quantifier::Exists => result = false,
                }
            }
            Value::Bool(result)
        }
        Node::Call {
            parameters,
            arguments,
            definition,
        } => {
            let mut nested = BTreeMap::new();
            for (parameter, argument) in parameters.iter().zip(arguments) {
                nested.insert(parameter.clone(), evaluate(argument, bindings, context)?);
            }
            evaluate(definition, &nested, context)?
        }
        Node::CastDynamic(source) => evaluate(source, bindings, context)?,
    };
    expression
        .value_type
        .validate_value(&value, &RefinementEvidence::default())
        .map_err(|error| {
            fault(format!(
                "evaluated value violates its checked type: {}",
                error.message
            ))
        })?;
    Ok(value)
}

fn check_pattern(
    value: &Value,
    subject_type: &CheckedType,
    bindings: &mut BTreeMap<String, CheckedType>,
    types: &BTreeMap<String, RegisteredType>,
) -> Result<CheckedPattern> {
    let Value::Array(parts) = value else {
        return Err(invalid("pattern must be an array"));
    };
    let node = match parts.as_slice() {
        [Value::Text(tag)] if tag == "wildcard" => PatternNode::Wildcard,
        [Value::Text(tag), literal] if tag == "literal" => {
            subject_type.validate_value(literal, &RefinementEvidence::default())?;
            PatternNode::Literal(literal.clone())
        }
        [Value::Text(tag), binding] if tag == "bind" => {
            let (id, value_type) = checked_binding(binding)?;
            require_type(&value_type, subject_type, "pattern binding")?;
            if bindings.insert(id.clone(), value_type).is_some() {
                return Err(invalid("pattern binding identity is duplicated"));
            }
            PatternNode::Bind(id)
        }
        [Value::Text(tag), Value::Text(case), Value::Array(patterns)] if tag == "variant" => {
            let payload_types = variant_payload(subject_type, case, types)?;
            if payload_types.len() != patterns.len() {
                return Err(invalid("variant pattern has the wrong arity"));
            }
            PatternNode::Variant(
                case.clone(),
                patterns
                    .iter()
                    .zip(&payload_types)
                    .map(|(pattern, value_type)| {
                        check_pattern(pattern, value_type, bindings, types)
                    })
                    .collect::<Result<_>>()?,
            )
        }
        [Value::Text(tag), Value::Array(patterns)] if tag == "tuple" => {
            let element_types = tuple_elements(subject_type, types)?;
            if element_types.len() != patterns.len() {
                return Err(invalid("tuple pattern has the wrong arity"));
            }
            PatternNode::Tuple(
                patterns
                    .iter()
                    .zip(&element_types)
                    .map(|(pattern, value_type)| {
                        check_pattern(pattern, value_type, bindings, types)
                    })
                    .collect::<Result<_>>()?,
            )
        }
        [Value::Text(tag), Value::Map(patterns)] if tag == "record" => {
            let field_types = record_fields(subject_type, types)?;
            let mut checked = Vec::with_capacity(patterns.len());
            for (name, pattern) in patterns {
                let Some((field_type, _)) = field_types.get(name) else {
                    return Err(invalid(format!(
                        "record pattern has undeclared field {name:?}"
                    )));
                };
                checked.push((
                    name.clone(),
                    check_pattern(pattern, field_type, bindings, types)?,
                ));
            }
            PatternNode::Record(checked)
        }
        _ => return Err(invalid("unsupported or malformed pattern")),
    };
    Ok(CheckedPattern { node })
}

fn match_pattern(
    pattern: &CheckedPattern,
    value: &Value,
) -> Result<Option<BTreeMap<String, Value>>> {
    let mut captures = BTreeMap::new();
    let matched = match &pattern.node {
        PatternNode::Wildcard => true,
        PatternNode::Literal(literal) => literal == value,
        PatternNode::Bind(id) => {
            captures.insert(id.clone(), value.clone());
            true
        }
        PatternNode::Variant(case, patterns) => {
            let Value::Array(parts) = value else {
                return Ok(None);
            };
            let [Value::Text(tag), Value::Text(actual), payload] = parts.as_slice() else {
                return Ok(None);
            };
            if tag != "variant" || actual != case {
                false
            } else {
                let payloads: Vec<&Value> = match patterns.len() {
                    0 => Vec::new(),
                    1 => vec![payload],
                    _ => match payload {
                        Value::Array(values) => values.iter().collect(),
                        _ => return Ok(None),
                    },
                };
                match_pattern_sequence(patterns, payloads, &mut captures)?
            }
        }
        PatternNode::Tuple(patterns) => match value {
            Value::Array(values) if values.len() == patterns.len() => {
                match_pattern_sequence(patterns, values.iter().collect(), &mut captures)?
            }
            _ => false,
        },
        PatternNode::Record(patterns) => match value {
            Value::Map(values) => {
                let mut matched = true;
                for (name, pattern) in patterns {
                    let Some((_, value)) = values.iter().find(|(candidate, _)| candidate == name)
                    else {
                        matched = false;
                        break;
                    };
                    let Some(nested) = match_pattern(pattern, value)? else {
                        matched = false;
                        break;
                    };
                    captures.extend(nested);
                }
                matched
            }
            _ => false,
        },
    };
    Ok(matched.then_some(captures))
}

fn match_pattern_sequence(
    patterns: &[CheckedPattern],
    values: Vec<&Value>,
    captures: &mut BTreeMap<String, Value>,
) -> Result<bool> {
    if patterns.len() != values.len() {
        return Ok(false);
    }
    for (pattern, value) in patterns.iter().zip(values) {
        let Some(nested) = match_pattern(pattern, value)? else {
            return Ok(false);
        };
        captures.extend(nested);
    }
    Ok(true)
}

fn validate_exhaustive(
    subject_type: &CheckedType,
    arms: &[MatchArm],
    types: &BTreeMap<String, RegisteredType>,
) -> Result<()> {
    if arms.iter().any(|arm| {
        arm.guard.is_none()
            && matches!(
                arm.pattern.node,
                PatternNode::Wildcard | PatternNode::Bind(_)
            )
    }) {
        return Ok(());
    }
    let subject_type = resolve_registered_type(subject_type, types)?;
    let Value::Array(parts) = subject_type.to_value() else {
        unreachable!()
    };
    if matches!(parts.as_slice(), [Value::Text(tag), Value::Text(name)] if tag == "primitive" && name == "Bool")
    {
        let covered = arms
            .iter()
            .filter(|arm| arm.guard.is_none())
            .filter_map(|arm| match &arm.pattern.node {
                PatternNode::Literal(Value::Bool(value)) => Some(*value),
                _ => None,
            })
            .collect::<BTreeSet<_>>();
        if covered.len() == 2 {
            return Ok(());
        }
    }
    let declared = match parts.as_slice() {
        [Value::Text(tag), Value::Array(cases)] if tag == "variant" => cases
            .iter()
            .filter_map(|case| match case.as_array() {
                Ok([Value::Text(name), _]) => Some(name.clone()),
                _ => None,
            })
            .collect::<BTreeSet<_>>(),
        [Value::Text(tag), _, _] if tag == "result" => {
            BTreeSet::from(["Err".to_owned(), "Ok".to_owned()])
        }
        [Value::Text(tag), _] if tag == "option" => {
            BTreeSet::from(["None".to_owned(), "Some".to_owned()])
        }
        _ => BTreeSet::new(),
    };
    if !declared.is_empty() {
        let covered = arms
            .iter()
            .filter(|arm| arm.guard.is_none())
            .filter_map(|arm| match &arm.pattern.node {
                PatternNode::Variant(name, patterns) => variant_payload(&subject_type, name, types)
                    .ok()
                    .filter(|payload| {
                        patterns.len() == payload.len()
                            && patterns.iter().zip(payload).all(|(pattern, value_type)| {
                                pattern_is_irrefutable(pattern, value_type, types)
                            })
                    })
                    .map(|_| name.clone()),
                _ => None,
            })
            .collect::<BTreeSet<_>>();
        if declared == covered {
            return Ok(());
        }
    }
    Err(invalid("match expression is not statically exhaustive"))
}

fn pattern_is_irrefutable(
    pattern: &CheckedPattern,
    value_type: &CheckedType,
    types: &BTreeMap<String, RegisteredType>,
) -> bool {
    match &pattern.node {
        PatternNode::Wildcard | PatternNode::Bind(_) => true,
        PatternNode::Literal(_) | PatternNode::Variant(_, _) => false,
        PatternNode::Tuple(patterns) => tuple_elements(value_type, types).is_ok_and(|elements| {
            patterns.len() == elements.len()
                && patterns.iter().zip(elements).all(|(pattern, value_type)| {
                    pattern_is_irrefutable(pattern, &value_type, types)
                })
        }),
        PatternNode::Record(patterns) => record_fields(value_type, types).is_ok_and(|fields| {
            patterns.iter().all(|(name, pattern)| {
                fields.get(name).is_some_and(|(value_type, optional)| {
                    !optional && pattern_is_irrefutable(pattern, value_type, types)
                })
            })
        }),
    }
}

fn validate_unary(operator: &str, operand: &CheckedType, result: &CheckedType) -> Result<()> {
    let valid = match operator {
        "!" => operand == &bool_type()? && result == operand,
        "-" => numeric_kind(operand).is_some() && result == operand,
        _ => false,
    };
    valid.then_some(()).ok_or_else(|| {
        invalid(format!(
            "unsupported or ill-typed unary operator {operator:?}"
        ))
    })
}

fn evaluate_unary(operator: &str, operand: Value, result_type: &CheckedType) -> Result<Value> {
    match operator {
        "!" => match operand {
            Value::Bool(value) => Ok(Value::Bool(!value)),
            _ => Err(fault("Boolean negation received a non-Boolean value")),
        },
        "-" => {
            let (numerator, denominator) = exact_ratio(&operand)?;
            exact_value_for_type(-numerator, denominator, result_type)
        }
        _ => Err(fault("checked unary operator has no evaluator")),
    }
}

fn validate_binary(
    operator: &str,
    left: &CheckedType,
    right: &CheckedType,
    result: &CheckedType,
) -> Result<()> {
    let boolean = bool_type()?;
    let numeric = numeric_kind(left).is_some() && left == right;
    let valid = match operator {
        "==" | "!=" => left == right && result == &boolean,
        "<" | "<=" | ">" | ">=" => numeric && result == &boolean,
        "+" | "-" | "*" | "/" | "%" => numeric && result == left,
        "&&" | "||" => left == &boolean && right == &boolean && result == &boolean,
        "in" => {
            collection_element_any(right).is_ok_and(|element| element == *left)
                && result == &boolean
        }
        _ => false,
    };
    valid.then_some(()).ok_or_else(|| {
        invalid(format!(
            "unsupported or ill-typed binary operator {operator:?}"
        ))
    })
}

fn evaluate_binary(
    operator: &str,
    left: Value,
    right: Value,
    result_type: &CheckedType,
) -> Result<Value> {
    match operator {
        "==" => Ok(Value::Bool(left == right)),
        "!=" => Ok(Value::Bool(left != right)),
        "in" => match right {
            Value::Array(values) => Ok(Value::Bool(values.contains(&left))),
            _ => Err(fault("membership received a non-collection domain")),
        },
        "&&" | "||" => {
            let (Value::Bool(left), Value::Bool(right)) = (left, right) else {
                return Err(fault("Boolean operation received non-Boolean values"));
            };
            Ok(Value::Bool(if operator == "&&" {
                left && right
            } else {
                left || right
            }))
        }
        "<" | "<=" | ">" | ">=" => {
            let (left_numerator, left_denominator) = exact_ratio(&left)?;
            let (right_numerator, right_denominator) = exact_ratio(&right)?;
            let left = left_numerator * right_denominator;
            let right = right_numerator * left_denominator;
            Ok(Value::Bool(match operator {
                "<" => left < right,
                "<=" => left <= right,
                ">" => left > right,
                ">=" => left >= right,
                _ => unreachable!(),
            }))
        }
        "+" | "-" | "*" | "/" | "%" => {
            let (left_numerator, left_denominator) = exact_ratio(&left)?;
            let (right_numerator, right_denominator) = exact_ratio(&right)?;
            if matches!(operator, "/" | "%") && right_numerator.is_zero() {
                return Err(fault("division by zero"));
            }
            let (numerator, denominator) = match operator {
                "+" | "-" => {
                    let left = left_numerator * &right_denominator;
                    let right = right_numerator * &left_denominator;
                    (
                        if operator == "+" {
                            left + right
                        } else {
                            left - right
                        },
                        left_denominator * right_denominator,
                    )
                }
                "*" => (
                    left_numerator * right_numerator,
                    left_denominator * right_denominator,
                ),
                "/" => (
                    left_numerator * right_denominator,
                    left_denominator * right_numerator,
                ),
                "%" if left_denominator.is_one() && right_denominator.is_one() => {
                    (left_numerator % right_numerator, BigInt::one())
                }
                "%" => return Err(fault("remainder is defined only for integral exact values")),
                _ => unreachable!(),
            };
            exact_value_for_type(numerator, denominator, result_type)
        }
        _ => Err(fault("checked binary operator has no evaluator")),
    }
}

fn evaluate_selection(subject: Value, selector: &Selector) -> Result<Value> {
    match (subject, selector) {
        (Value::Map(entries), Selector::Field(field)) => entries
            .into_iter()
            .find_map(|(name, value)| (name == *field).then_some(value))
            .ok_or_else(|| fault(format!("selection has no field {field:?}"))),
        (Value::Array(values), Selector::Index(index)) => values
            .get(*index)
            .cloned()
            .ok_or_else(|| fault(format!("selection index {index} is out of bounds"))),
        _ => Err(fault("selection received a value with the wrong shape")),
    }
}

fn check_selector(value: &Value) -> Result<Selector> {
    match value {
        Value::Text(field) => Ok(Selector::Field(field.clone())),
        Value::Integer(index) if *index >= 0 => usize::try_from(*index)
            .map(Selector::Index)
            .map_err(|_| invalid("selection index exceeds the executable domain")),
        _ => Err(invalid("selection requires a text field or unsigned index")),
    }
}

fn selected_type(
    subject: &CheckedType,
    selector: &Selector,
    types: &BTreeMap<String, RegisteredType>,
) -> Result<CheckedType> {
    let subject = resolve_registered_type(subject, types)?;
    let subject = match subject.to_value() {
        Value::Array(parts) if matches!(parts.first(), Some(Value::Text(tag)) if tag == "handle") =>
        {
            let value_type = parts
                .get(5)
                .ok_or_else(|| invalid("selected handle type is malformed"))?;
            resolve_registered_type(&CheckedType::from_value(value_type)?, types)?
        }
        _ => subject,
    };
    let Value::Array(parts) = subject.to_value() else {
        unreachable!()
    };
    match (parts.as_slice(), selector) {
        ([Value::Text(tag), _, Value::Array(fields)], Selector::Field(field))
            if tag == "record" =>
        {
            fields
                .iter()
                .find_map(|entry| match entry.as_array() {
                    Ok([Value::Text(name), value_type, _]) if name == field => {
                        Some(CheckedType::from_value(value_type))
                    }
                    _ => None,
                })
                .transpose()?
                .ok_or_else(|| invalid(format!("record type has no field {field:?}")))
        }
        ([Value::Text(tag), key, value], Selector::Field(_)) if tag == "map" => {
            require_type(
                &CheckedType::from_value(key)?,
                &text_type()?,
                "selected map key",
            )?;
            CheckedType::from_value(value)
        }
        ([Value::Text(tag), Value::Array(elements)], Selector::Index(index)) if tag == "tuple" => {
            elements
                .get(*index)
                .map(CheckedType::from_value)
                .transpose()?
                .ok_or_else(|| invalid(format!("tuple selection index {index} is out of bounds")))
        }
        ([Value::Text(tag), element], Selector::Index(_))
            if matches!(tag.as_str(), "list" | "set") =>
        {
            CheckedType::from_value(element)
        }
        _ => Err(invalid(
            "selector is incompatible with the selected expression type",
        )),
    }
}

fn checked_binding(value: &Value) -> Result<(String, CheckedType)> {
    let Value::Map(entries) = value else {
        return Err(invalid("binding must be a closed map"));
    };
    if !matches!(entries.len(), 2 | 3)
        || !matches!(value.get("id"), Some(Value::Text(id)) if !id.is_empty())
        || value.get("type").is_none()
        || (entries.len() == 3
            && !matches!(value.get("name"), Some(Value::Text(name)) if !name.is_empty()))
    {
        return Err(invalid("binding must contain id, optional name, and type"));
    }
    let Value::Text(id) = value.get("id").unwrap() else {
        unreachable!()
    };
    Ok((
        id.clone(),
        CheckedType::from_value(value.get("type").unwrap())?,
    ))
}

fn checked_verifier_binding(
    value: &Value,
    domain_type: &CheckedType,
) -> Result<VerifierRequirement> {
    let Value::Map(entries) = value else {
        return Err(invalid("verifier binding must be a closed map"));
    };
    if !(3..=5).contains(&entries.len())
        || entries.iter().any(|(key, _)| {
            !matches!(
                key.as_str(),
                "verifier" | "input" | "output" | "configuration" | "trust"
            )
        })
    {
        return Err(invalid(
            "verifier binding contains unknown or missing fields",
        ));
    }
    let Some(Value::Text(verifier)) = value.get("verifier") else {
        return Err(invalid("verifier binding requires a verifier symbol"));
    };
    if !is_symbol(verifier) {
        return Err(invalid("verifier binding symbol is invalid"));
    }
    let input = value
        .get("input")
        .ok_or_else(|| invalid("verifier binding requires an input type"))?;
    require_type(
        &CheckedType::from_value(input)?,
        domain_type,
        "verifier input",
    )?;
    let output_value = value
        .get("output")
        .ok_or_else(|| invalid("verifier binding requires an evidence output type"))?;
    if !matches!(output_value, Value::Array(parts) if matches!(parts.as_slice(), [Value::Text(tag), Value::Array(_)] if tag == "evidence"))
    {
        return Err(invalid(
            "verifier binding output must be a closed evidence type",
        ));
    }
    let output = CheckedType::from_canonical_value(output_value)?;
    if let Some(configuration) = value.get("configuration") {
        CheckedType::validate_untyped_value(configuration)?;
    }
    let mut retained_trust = BTreeSet::new();
    if let Some(trust) = value.get("trust") {
        let Value::Array(classes) = trust else {
            return Err(invalid("verifier trust restriction must be an array"));
        };
        for class in classes {
            let Value::Text(class) = class else {
                return Err(invalid(
                    "verifier trust restriction contains an invalid class",
                ));
            };
            if !is_evidence_class(class) || !retained_trust.insert(class.clone()) {
                return Err(invalid(
                    "verifier trust restriction contains an invalid or duplicate class",
                ));
            }
        }
    }
    Ok(VerifierRequirement {
        verifier: verifier.clone(),
        output,
        trust: retained_trust,
    })
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

fn nested_bindings(
    bindings: &BTreeMap<String, CheckedType>,
    id: &str,
    value_type: CheckedType,
) -> Result<BTreeMap<String, CheckedType>> {
    let mut nested = bindings.clone();
    if nested.insert(id.to_owned(), value_type).is_some() {
        return Err(invalid("binding captures an existing identity"));
    }
    Ok(nested)
}

fn checked_quantifier(value: &str) -> Result<Quantifier> {
    match value {
        "forall" => Ok(Quantifier::ForAll),
        "exists" => Ok(Quantifier::Exists),
        _ => Err(invalid("unknown expression quantifier")),
    }
}

fn require_type(actual: &CheckedType, expected: &CheckedType, context: &str) -> Result<()> {
    (actual == expected)
        .then_some(())
        .ok_or_else(|| invalid(format!("{context} type differs from its declaration")))
}

fn record_fields(
    value_type: &CheckedType,
    types: &BTreeMap<String, RegisteredType>,
) -> Result<BTreeMap<String, (CheckedType, bool)>> {
    let value_type = resolve_registered_type(value_type, types)?;
    let Value::Array(parts) = value_type.to_value() else {
        unreachable!()
    };
    let [Value::Text(tag), _, Value::Array(fields)] = parts.as_slice() else {
        return Err(invalid("record form requires a record type"));
    };
    if tag != "record" {
        return Err(invalid("record form requires a record type"));
    }
    fields
        .iter()
        .map(|field| {
            let [Value::Text(name), value_type, Value::Bool(optional)] = field.as_array()? else {
                return Err(invalid("record field type lost its shape"));
            };
            Ok((
                name.clone(),
                (CheckedType::from_value(value_type)?, *optional),
            ))
        })
        .collect()
}

fn record_is_open(
    value_type: &CheckedType,
    types: &BTreeMap<String, RegisteredType>,
) -> Result<bool> {
    let value_type = resolve_registered_type(value_type, types)?;
    let Value::Array(parts) = value_type.to_value() else {
        unreachable!()
    };
    let [Value::Text(tag), Value::Bool(open), Value::Array(_)] = parts.as_slice() else {
        return Err(invalid("record form requires a record type"));
    };
    if tag != "record" {
        return Err(invalid("record form requires a record type"));
    }
    Ok(*open)
}

fn tuple_elements(
    value_type: &CheckedType,
    types: &BTreeMap<String, RegisteredType>,
) -> Result<Vec<CheckedType>> {
    let value_type = resolve_registered_type(value_type, types)?;
    let Value::Array(parts) = value_type.to_value() else {
        unreachable!()
    };
    let [Value::Text(tag), Value::Array(elements)] = parts.as_slice() else {
        return Err(invalid("tuple form requires a tuple type"));
    };
    if tag != "tuple" {
        return Err(invalid("tuple form requires a tuple type"));
    }
    elements.iter().map(CheckedType::from_value).collect()
}

fn variant_payload(
    value_type: &CheckedType,
    case: &str,
    types: &BTreeMap<String, RegisteredType>,
) -> Result<Vec<CheckedType>> {
    let value_type = resolve_registered_type(value_type, types)?;
    let Value::Array(parts) = value_type.to_value() else {
        unreachable!()
    };
    if let [Value::Text(tag), ok, error] = parts.as_slice()
        && tag == "result"
    {
        return match case {
            "Ok" => Ok(vec![CheckedType::from_value(ok)?]),
            "Err" => Ok(vec![CheckedType::from_value(error)?]),
            _ => Err(invalid(format!("result type has no case {case:?}"))),
        };
    }
    if let [Value::Text(tag), element] = parts.as_slice()
        && tag == "option"
    {
        return match case {
            "Some" => Ok(vec![CheckedType::from_value(element)?]),
            "None" => Ok(vec![]),
            _ => Err(invalid(format!("option type has no case {case:?}"))),
        };
    }
    let [Value::Text(tag), Value::Array(cases)] = parts.as_slice() else {
        return Err(invalid(
            "variant form requires a variant, Result, or Option type",
        ));
    };
    if tag != "variant" {
        return Err(invalid(
            "variant form requires a variant, Result, or Option type",
        ));
    }
    let payload = cases
        .iter()
        .find_map(|entry| match entry.as_array() {
            Ok([Value::Text(name), Value::Array(payload)]) if name == case => Some(payload),
            _ => None,
        })
        .ok_or_else(|| invalid(format!("variant type has no case {case:?}")))?;
    payload.iter().map(CheckedType::from_value).collect()
}

fn resolve_registered_type(
    value_type: &CheckedType,
    types: &BTreeMap<String, RegisteredType>,
) -> Result<CheckedType> {
    let Value::Array(parts) = value_type.to_value() else {
        return Ok(value_type.clone());
    };
    let [
        Value::Text(tag),
        Value::Text(symbol),
        Value::Array(arguments),
    ] = parts.as_slice()
    else {
        return Ok(value_type.clone());
    };
    if tag != "nominal" {
        return Ok(value_type.clone());
    }
    let registered = types.get(symbol).ok_or_else(|| {
        invalid(format!(
            "nominal expression type {symbol:?} is not registered"
        ))
    })?;
    if registered.parameter_count != arguments.len() {
        return Err(invalid("nominal expression type has the wrong arity"));
    }
    let arguments = arguments
        .iter()
        .map(CheckedType::from_value)
        .collect::<Result<Vec<_>>>()?;
    substitute_type_parameters(&registered.definition, &arguments)
}

fn substitute_type_parameters(
    value_type: &CheckedType,
    arguments: &[CheckedType],
) -> Result<CheckedType> {
    fn substitute(value: &Value, arguments: &[CheckedType]) -> Result<Value> {
        if let Value::Array(parts) = value
            && let [Value::Text(tag), Value::Integer(index)] = parts.as_slice()
            && tag == "parameter"
        {
            let index = usize::try_from(*index)
                .map_err(|_| invalid("generic type parameter index is invalid"))?;
            return arguments
                .get(index)
                .map(CheckedType::to_value)
                .ok_or_else(|| invalid("generic type parameter does not resolve"));
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
    CheckedType::from_value(&substitute(&value_type.to_value(), arguments)?)
}

fn variant_payload_type(payload: &[CheckedType]) -> Result<CheckedType> {
    match payload {
        [] => unit_type(),
        [value_type] => Ok(value_type.clone()),
        values => CheckedType::from_value(&Value::Array(vec![
            Value::Text("tuple".to_owned()),
            Value::Array(values.iter().map(CheckedType::to_value).collect()),
        ])),
    }
}

fn map_types(value_type: &CheckedType) -> Result<(CheckedType, CheckedType)> {
    let Value::Array(parts) = value_type.to_value() else {
        unreachable!()
    };
    let [Value::Text(tag), key, value] = parts.as_slice() else {
        return Err(invalid("map form requires a map type"));
    };
    if tag != "map" {
        return Err(invalid("map form requires a map type"));
    }
    Ok((
        CheckedType::from_value(key)?,
        CheckedType::from_value(value)?,
    ))
}

fn map_key_is_text(value_type: &CheckedType) -> Result<bool> {
    let (key, _) = map_types(value_type)?;
    Ok(key == text_type()?)
}

fn collection_element(value_type: &CheckedType, kind: &str) -> Result<CheckedType> {
    let Value::Array(parts) = value_type.to_value() else {
        unreachable!()
    };
    if matches!(parts.as_slice(), [Value::Text(tag), _] if tag == kind) {
        CheckedType::from_value(&parts[1])
    } else {
        Err(invalid("collection form differs from its declared type"))
    }
}

fn collection_element_any(value_type: &CheckedType) -> Result<CheckedType> {
    let Value::Array(parts) = value_type.to_value() else {
        unreachable!()
    };
    if matches!(parts.as_slice(), [Value::Text(tag), _] if matches!(tag.as_str(), "list" | "set")) {
        CheckedType::from_value(&parts[1])
    } else {
        Err(invalid(
            "operation requires a statically finite list or set",
        ))
    }
}

fn simple_type(tag: &str, name: &str) -> Result<CheckedType> {
    CheckedType::from_value(&Value::Array(vec![
        Value::Text(tag.to_owned()),
        Value::Text(name.to_owned()),
    ]))
}

fn bool_type() -> Result<CheckedType> {
    simple_type("primitive", "Bool")
}
fn text_type() -> Result<CheckedType> {
    simple_type("primitive", "Text")
}
fn unit_type() -> Result<CheckedType> {
    simple_type("primitive", "Unit")
}
fn dynamic_type() -> Result<CheckedType> {
    simple_type("special", "Dynamic")
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NumericKind {
    Integer,
    Rational,
    Decimal,
    MachineInteger,
}

fn numeric_kind(value_type: &CheckedType) -> Option<NumericKind> {
    let Value::Array(parts) = value_type.to_value() else {
        return None;
    };
    match parts.as_slice() {
        [Value::Text(tag), Value::Text(name)] if tag == "exact-number" => match name.as_str() {
            "Integer" => Some(NumericKind::Integer),
            "Rational" => Some(NumericKind::Rational),
            "Decimal" => Some(NumericKind::Decimal),
            _ => None,
        },
        [Value::Text(tag), _, _] if tag == "machine-integer" => Some(NumericKind::MachineInteger),
        _ => None,
    }
}

fn exact_ratio(value: &Value) -> Result<(BigInt, BigInt)> {
    let Value::Array(parts) = value else {
        return Err(fault("exact operation received a non-exact value"));
    };
    match parts.as_slice() {
        [Value::Text(tag), Value::Integer(value)] if tag == "integer" => {
            Ok((BigInt::from(*value), BigInt::one()))
        }
        [
            Value::Text(tag),
            Value::Integer(numerator),
            Value::Integer(denominator),
        ] if tag == "rational" => Ok((BigInt::from(*numerator), BigInt::from(*denominator))),
        [
            Value::Text(tag),
            Value::Integer(coefficient),
            Value::Integer(exponent),
        ] if tag == "decimal" => {
            let magnitude = u32::try_from(exponent.unsigned_abs())
                .map_err(|_| fault("decimal exponent exceeds the executable number domain"))?;
            if *coefficient == 0 {
                return Ok((BigInt::zero(), BigInt::one()));
            }
            if magnitude > MAX_EXECUTABLE_DECIMAL_EXPONENT {
                return Err(fault(format!(
                    "decimal exponent exceeds the bounded evaluation ceiling of {MAX_EXECUTABLE_DECIMAL_EXPONENT}"
                )));
            }
            let power = BigInt::from(10_u8).pow(magnitude);
            if *exponent >= 0 {
                Ok((BigInt::from(*coefficient) * power, BigInt::one()))
            } else {
                Ok((BigInt::from(*coefficient), power))
            }
        }
        _ => Err(fault("exact operation received a non-exact value")),
    }
}

fn exact_value_for_type(
    numerator: BigInt,
    denominator: BigInt,
    result_type: &CheckedType,
) -> Result<Value> {
    let (numerator, denominator) = reduced_ratio(numerator, denominator)?;
    match numeric_kind(result_type) {
        Some(NumericKind::Integer | NumericKind::MachineInteger) => {
            if !denominator.is_one() {
                return Err(fault("operation has no integral result"));
            }
            Ok(exact_integer_value(executable_integer(&numerator)?))
        }
        Some(NumericKind::Rational) => Ok(Value::Array(vec![
            Value::Text("rational".to_owned()),
            Value::Integer(executable_integer(&numerator)?),
            Value::Integer(executable_integer(&denominator)?),
        ])),
        Some(NumericKind::Decimal) => exact_decimal_value(numerator, denominator),
        None => Err(fault("checked operation has a non-numeric result type")),
    }
}

fn reduced_ratio(numerator: BigInt, denominator: BigInt) -> Result<(BigInt, BigInt)> {
    if denominator.is_zero() {
        return Err(fault("division by zero"));
    }
    let (numerator, denominator) = if denominator.is_negative() {
        (-numerator, -denominator)
    } else {
        (numerator, denominator)
    };
    let divisor = numerator.gcd(&denominator);
    Ok((numerator / &divisor, denominator / divisor))
}

fn exact_decimal_value(numerator: BigInt, denominator: BigInt) -> Result<Value> {
    let mut denominator = denominator;
    let mut twos = 0_u32;
    let mut fives = 0_u32;
    while (&denominator % 2_u8).is_zero() {
        denominator /= 2;
        twos += 1;
    }
    while (&denominator % 5_u8).is_zero() {
        denominator /= 5;
        fives += 1;
    }
    if !denominator.is_one() {
        return Err(fault("exact quotient has no finite Decimal representation"));
    }
    let scale = twos.max(fives);
    let mut coefficient = numerator;
    coefficient *= BigInt::from(2_u8).pow(scale - twos);
    coefficient *= BigInt::from(5_u8).pow(scale - fives);
    let mut exponent = -(scale as i128);
    while !coefficient.is_zero() && (&coefficient % 10_u8).is_zero() {
        coefficient /= 10;
        exponent += 1;
    }
    Ok(Value::Array(vec![
        Value::Text("decimal".to_owned()),
        Value::Integer(executable_integer(&coefficient)?),
        Value::Integer(exponent),
    ]))
}

fn executable_integer(value: &BigInt) -> Result<i128> {
    value
        .to_i128()
        .ok_or_else(|| fault("exact result exceeds the executable deterministic-CBOR domain"))
}

fn exact_integer_value(value: i128) -> Value {
    Value::Array(vec![
        Value::Text("integer".to_owned()),
        Value::Integer(value),
    ])
}

trait ValueArray {
    fn as_array(&self) -> Result<&[Value]>;
}

impl ValueArray for Value {
    fn as_array(&self) -> Result<&[Value]> {
        match self {
            Value::Array(values) => Ok(values),
            _ => Err(invalid("value must be an array")),
        }
    }
}

fn invalid(message: impl Into<String>) -> Diagnostic {
    Diagnostic::plain(INVALID_EXPRESSION, message)
}
fn fault(message: impl Into<String>) -> Diagnostic {
    Diagnostic::plain(EXPRESSION_FAULT, message)
}
