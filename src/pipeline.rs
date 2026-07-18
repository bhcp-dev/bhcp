use std::collections::{HashMap, HashSet};

use crate::cbor::encode_deterministic;
use crate::diagnostic::{Diagnostic, Result};
use crate::hash::{HashAlgorithm, artifact_hash_with, semantic_hash_with};
use crate::model::{
    BhcpType, Binding, CanonicalAstDocument, Clause, ClauseKind, ContentReference, Effect,
    Expression, ExpressionForm, FieldType, GoalDefinition, HashId, SemanticIrDocument,
    VerifierBinding, features_for,
};
use crate::parser::{
    ParsedProgram, SurfaceClauseKind, SurfaceEffect, SurfaceExpression, SurfaceGoal,
    SurfaceLiteral, SurfaceType, parse_canonical,
};
use crate::value::Value;

#[derive(Clone, Debug)]
pub struct Compilation {
    pub ast: CanonicalAstDocument,
    pub ir: SemanticIrDocument,
    pub ast_bytes: Vec<u8>,
    pub ir_bytes: Vec<u8>,
    pub ast_hash: HashId,
    pub semantic_hash: HashId,
    pub ir_hash: HashId,
}

pub fn parse_source(source: &str, source_name: &str) -> Result<CanonicalAstDocument> {
    parse_source_with_algorithm(source, source_name, HashAlgorithm::default())
}

pub fn parse_source_with_algorithm(
    source: &str,
    source_name: &str,
    algorithm: HashAlgorithm,
) -> Result<CanonicalAstDocument> {
    Ok(parse_internal(source, source_name, algorithm)?.0)
}

pub fn compile_source(source: &str, source_name: &str) -> Result<Compilation> {
    compile_source_with_algorithm(source, source_name, HashAlgorithm::default())
}

pub fn compile_source_with_algorithm(
    source: &str,
    source_name: &str,
    algorithm: HashAlgorithm,
) -> Result<Compilation> {
    let (ast, program) = parse_internal(source, source_name, algorithm)?;
    let mut ir = elaborate(&program, source_name, algorithm)?;
    let semantic_hash = semantic_hash_with(&ir, algorithm)?;
    ir.semantic_id = Some(semantic_hash.clone());
    let ir_hash = artifact_hash_with(&ir.to_value(false), algorithm)?;
    ir.artifact_id = Some(ir_hash.clone());
    ir.validate()?;
    let ast_bytes = encode_deterministic(&ast.to_value(true))?;
    let ir_bytes = encode_deterministic(&ir.to_value(true))?;
    let ast_hash = ast.artifact_id.clone().expect("validated AST has identity");
    Ok(Compilation {
        ast_hash,
        ast,
        ir,
        ast_bytes,
        ir_bytes,
        semantic_hash,
        ir_hash,
    })
}

fn parse_internal(
    source: &str,
    source_name: &str,
    algorithm: HashAlgorithm,
) -> Result<(CanonicalAstDocument, ParsedProgram)> {
    let bytes = source.as_bytes();
    let source_ref = ContentReference {
        media_type: "text/bhcp;profile=bhcp%2Fcanonical%400".to_owned(),
        size: bytes.len(),
        digests: vec![algorithm.hash(bytes)],
    };
    let program = parse_canonical(source, source_name, source_ref.clone())?;
    let mut ast = CanonicalAstDocument {
        features: features_for(algorithm),
        root: program.ast.clone(),
        source: source_ref,
        artifact_id: None,
    };
    ast.artifact_id = Some(artifact_hash_with(&ast.to_value(false), algorithm)?);
    ast.validate()?;
    Ok((ast, program))
}

struct Ids {
    counters: HashMap<&'static str, usize>,
}
impl Ids {
    fn new() -> Self {
        Self {
            counters: HashMap::new(),
        }
    }
    fn next(&mut self, kind: &'static str) -> String {
        let counter = self.counters.entry(kind).or_default();
        *counter += 1;
        format!("{kind}-{counter}")
    }
}

fn elaborate(
    program: &ParsedProgram,
    source_name: &str,
    algorithm: HashAlgorithm,
) -> Result<SemanticIrDocument> {
    let mut symbols = HashSet::new();
    for goal in &program.goals {
        if !symbols.insert(&goal.symbol) {
            return Err(error(
                "BHCP2002",
                format!("duplicate goal symbol {}", goal.symbol),
                source_name,
                &goal.at,
            ));
        }
    }
    let mut ids = Ids::new();
    let mut goals = Vec::new();
    for (index, goal) in program.goals.iter().enumerate() {
        goals.push(lower_goal(goal, index, source_name, &mut ids)?);
    }
    let entrypoints = goals.iter().map(|goal| goal.id.clone()).collect();
    Ok(SemanticIrDocument {
        features: features_for(algorithm),
        functions: vec![],
        goals,
        entrypoints,
        semantic_id: None,
        artifact_id: None,
    })
}

fn lower_goal(
    goal: &SurfaceGoal,
    index: usize,
    source_name: &str,
    ids: &mut Ids,
) -> Result<GoalDefinition> {
    let mut environment = HashMap::new();
    let mut bindings = HashMap::new();
    let mut input_fields = Vec::new();
    let mut output_fields = Vec::new();
    for (clause_index, clause) in goal.clauses.iter().enumerate() {
        if let SurfaceClauseKind::Fact {
            kind,
            name,
            value_type,
        } = &clause.kind
        {
            if environment.contains_key(name) {
                return Err(error(
                    "BHCP2002",
                    format!("duplicate observable name {name:?}"),
                    source_name,
                    &clause.at,
                ));
            }
            let value_type = lower_type(value_type);
            let binding = Binding {
                id: ids.next("binding"),
                name: name.clone(),
                value_type: value_type.clone(),
            };
            environment.insert(name.clone(), binding.clone());
            bindings.insert(clause_index, binding);
            let field = FieldType {
                name: name.clone(),
                value_type,
            };
            if *kind == "input" {
                input_fields.push(field);
            } else {
                output_fields.push(field);
            }
        }
    }
    input_fields.sort_by(|left, right| left.name.cmp(&right.name));
    output_fields.sort_by(|left, right| left.name.cmp(&right.name));
    let input = BhcpType::Record(input_fields);
    let output = BhcpType::Record(output_fields);
    let mut clauses = Vec::new();
    for (clause_index, surface) in goal.clauses.iter().enumerate() {
        let kind = match &surface.kind {
            SurfaceClauseKind::Fact { kind, .. } => ClauseKind::Fact {
                kind,
                binding: bindings[&clause_index].clone(),
            },
            SurfaceClauseKind::Contract { kind, condition } => {
                let condition = lower_expression(condition, &environment, source_name, ids)?;
                if condition.value_type != BhcpType::Primitive("Bool") {
                    return Err(error(
                        "BHCP2003",
                        format!("{kind} condition must have type Bool"),
                        source_name,
                        &surface.at,
                    ));
                }
                ClauseKind::Contract { kind, condition }
            }
            SurfaceClauseKind::Authority { kind, effects } => {
                let mut effects: Vec<_> = effects
                    .iter()
                    .map(|effect| lower_effect(effect, &environment, source_name))
                    .collect::<Result<_>>()?;
                effects.sort_by(|left, right| left.id.cmp(&right.id));
                ClauseKind::Authority { kind, effects }
            }
            SurfaceClauseKind::Preference {
                priority,
                objective,
            } => ClauseKind::Preference {
                priority: *priority,
                objective: lower_expression(objective, &environment, source_name, ids)?,
            },
            SurfaceClauseKind::Verify { verifier } => ClauseKind::Verify {
                binding: VerifierBinding {
                    verifier: verifier.clone(),
                    input: input.clone(),
                    output: BhcpType::Evidence(vec!["static".to_owned()]),
                },
            },
        };
        clauses.push(Clause {
            id: ids.next("clause"),
            label: surface.label.clone(),
            kind,
        });
    }
    let evidence = if clauses
        .iter()
        .any(|clause| matches!(clause.kind, ClauseKind::Verify { .. }))
    {
        "static"
    } else {
        "unresolved"
    };
    Ok(GoalDefinition {
        id: format!("goal-{}", index + 1),
        symbol: goal.symbol.clone(),
        input,
        output,
        evidence: BhcpType::Evidence(vec![evidence.to_owned()]),
        clauses,
        body: None,
    })
}

fn lower_type(value: &SurfaceType) -> BhcpType {
    match value {
        SurfaceType::Primitive(name) => BhcpType::Primitive(name),
        SurfaceType::Exact(name) => BhcpType::ExactNumber(name),
    }
}

fn lower_expression(
    surface: &SurfaceExpression,
    environment: &HashMap<String, Binding>,
    source_name: &str,
    ids: &mut Ids,
) -> Result<Expression> {
    let (value_type, form) = match surface {
        SurfaceExpression::Literal { value, .. } => match value {
            SurfaceLiteral::Bool(value) => (
                BhcpType::Primitive("Bool"),
                ExpressionForm::Literal(Value::Bool(*value)),
            ),
            SurfaceLiteral::Text(value) => (
                BhcpType::Primitive("Text"),
                ExpressionForm::Literal(Value::Text(value.clone())),
            ),
            SurfaceLiteral::Integer(value) => (
                BhcpType::ExactNumber("Integer"),
                ExpressionForm::Literal(Value::Array(vec![
                    Value::Text("integer".to_owned()),
                    Value::Integer(*value),
                ])),
            ),
        },
        SurfaceExpression::Reference { name, at } => {
            let binding = environment.get(name).ok_or_else(|| {
                error(
                    "BHCP2001",
                    format!("unresolved name {name:?}"),
                    source_name,
                    at,
                )
            })?;
            (
                binding.value_type.clone(),
                ExpressionForm::Reference(binding.id.clone()),
            )
        }
        SurfaceExpression::Unary {
            operator,
            operand,
            at,
        } => {
            let operand = lower_expression(operand, environment, source_name, ids)?;
            let accepted = (operator == "!" && operand.value_type == BhcpType::Primitive("Bool"))
                || (operator == "-" && operand.value_type == BhcpType::ExactNumber("Integer"));
            if !accepted {
                return Err(error(
                    "BHCP2003",
                    format!("operator {operator} has an incompatible operand"),
                    source_name,
                    at,
                ));
            }
            let value_type = operand.value_type.clone();
            (
                value_type,
                ExpressionForm::Unary(operator.clone(), Box::new(operand)),
            )
        }
        SurfaceExpression::Binary {
            operator,
            left,
            right,
            at,
        } => {
            let left = lower_expression(left, environment, source_name, ids)?;
            let right = lower_expression(right, environment, source_name, ids)?;
            if left.value_type != right.value_type {
                return Err(error(
                    "BHCP2003",
                    format!("operator {operator} has incompatible operand types"),
                    source_name,
                    at,
                ));
            }
            let value_type = match operator.as_str() {
                "==" | "!=" => BhcpType::Primitive("Bool"),
                "<" | "<=" | ">" | ">=" if left.value_type == BhcpType::ExactNumber("Integer") => {
                    BhcpType::Primitive("Bool")
                }
                "+" if matches!(
                    left.value_type,
                    BhcpType::Primitive("Text") | BhcpType::ExactNumber("Integer")
                ) =>
                {
                    left.value_type.clone()
                }
                "&&" | "||" if left.value_type == BhcpType::Primitive("Bool") => {
                    BhcpType::Primitive("Bool")
                }
                "-" | "*" | "/" | "%" => {
                    return Err(error(
                        "BHCP2004",
                        format!(
                            "binary operator {operator} is outside the implemented expression slice"
                        ),
                        source_name,
                        at,
                    ));
                }
                _ => {
                    return Err(error(
                        "BHCP2003",
                        format!("operator {operator} has incompatible operand types"),
                        source_name,
                        at,
                    ));
                }
            };
            (
                value_type,
                ExpressionForm::Binary(operator.clone(), Box::new(left), Box::new(right)),
            )
        }
    };
    Ok(Expression {
        id: ids.next("expr"),
        value_type,
        form,
    })
}

fn lower_effect(
    surface: &SurfaceEffect,
    environment: &HashMap<String, Binding>,
    source_name: &str,
) -> Result<Effect> {
    let mut resource = None;
    let mut parameters = Vec::new();
    for argument in &surface.arguments {
        match argument {
            SurfaceExpression::Reference { name, at } => {
                let binding = environment.get(name).ok_or_else(|| {
                    error(
                        "BHCP2001",
                        format!("unresolved name {name:?}"),
                        source_name,
                        at,
                    )
                })?;
                if resource.is_some() {
                    return Err(error(
                        "BHCP2004",
                        "an effect atom supports one resource reference in this slice",
                        source_name,
                        at,
                    ));
                }
                resource = Some(binding.id.clone());
            }
            SurfaceExpression::Literal { value, .. } => parameters.push(match value {
                SurfaceLiteral::Bool(value) => Value::Bool(*value),
                SurfaceLiteral::Text(value) => Value::Text(value.clone()),
                SurfaceLiteral::Integer(value) => Value::Array(vec![
                    Value::Text("integer".to_owned()),
                    Value::Integer(*value),
                ]),
            }),
            _ => {
                return Err(error(
                    "BHCP2004",
                    "effect parameters must be literals or direct resource references in this slice",
                    source_name,
                    surface.arguments[0].at(),
                ));
            }
        }
    }
    Ok(Effect {
        id: surface.symbol.clone(),
        resource,
        parameters,
    })
}

fn error(
    code: &'static str,
    message: impl Into<String>,
    source: &str,
    point: &crate::model::Point,
) -> Diagnostic {
    Diagnostic::new(code, message, source, point.line, point.column)
}
