use std::collections::{HashMap, HashSet};

use crate::cbor::encode_deterministic;
use crate::diagnostic::{Diagnostic, Result};
use crate::hash::{HashAlgorithm, artifact_hash_with, semantic_hash_with};
use crate::kernel::{KernelChild, KernelNetwork};
use crate::model::{
    BhcpType, Binding, CanonicalAstDocument, Clause, ClauseKind, ContentReference, Effect,
    Expression, ExpressionForm, FieldType, FunctionDefinition, GoalDefinition, HashId,
    SemanticIrDocument, VerifierBinding, features_for,
};
use crate::parser::{
    ParsedProgram, SurfaceClauseKind, SurfaceComposition, SurfaceEffect, SurfaceExpression,
    SurfaceFunction, SurfaceGoal, SurfaceLiteral, SurfaceType, parse_canonical,
};
use crate::policy::SourcePolicyDocument;
use crate::prelude::{
    ALL_FEATURE, ALL_LOWERER, ALL_REDUCER, ANY_FEATURE, ANY_LOWERER, ANY_REDUCER, DerivedChild,
    DerivedForm, NetworkShape, Prelude,
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

#[derive(Clone, Debug)]
pub struct ParsedPolicySource {
    pub ast: CanonicalAstDocument,
    pub documents: Vec<SourcePolicyDocument>,
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

pub fn parse_policy_source(source: &str, source_name: &str) -> Result<ParsedPolicySource> {
    parse_policy_source_with_algorithm(source, source_name, HashAlgorithm::default())
}

pub fn parse_policy_source_with_algorithm(
    source: &str,
    source_name: &str,
    algorithm: HashAlgorithm,
) -> Result<ParsedPolicySource> {
    let (ast, program) = parse_internal(source, source_name, algorithm)?;
    if program.policies.is_empty() {
        return Err(Diagnostic::new(
            "BHCP1001",
            "policy source must contain at least one §policy definition",
            source_name,
            1,
            1,
        ));
    }
    Ok(ParsedPolicySource {
        ast,
        documents: program
            .policies
            .into_iter()
            .map(|policy| policy.document)
            .collect(),
    })
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
    let mut features = features_for(algorithm);
    if uses_self_hosted_all(&program) {
        features.push(ALL_FEATURE.to_owned());
    }
    if uses_self_hosted_any(&program) {
        features.push(ANY_FEATURE.to_owned());
    }
    let mut ast = CanonicalAstDocument {
        features,
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
    if !program.policies.is_empty() {
        return Err(error(
            "BHCP2004",
            "policy definitions lower to policy documents, not executable goal IR",
            source_name,
            &program.policies[0].at,
        ));
    }
    if !program.functions.is_empty() {
        return Err(error(
            "BHCP2004",
            "project function definitions are outside the implemented executable slice",
            source_name,
            &program.functions[0].at,
        ));
    }
    if program.goals.is_empty() {
        return Err(Diagnostic::new(
            "BHCP1001",
            "an executable source file must contain at least one goal",
            source_name,
            1,
            1,
        ));
    }
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
    let mut signatures = HashMap::new();
    for (index, goal) in program.goals.iter().enumerate() {
        let signature = goal_signature(goal, index, source_name)?;
        signatures.insert(goal.symbol.clone(), signature);
    }
    let prelude = Prelude::load()?;
    let mut ids = Ids::new();
    let mut goals = Vec::new();
    let mut functions = Vec::new();
    for (index, goal) in program.goals.iter().enumerate() {
        goals.push(lower_goal(
            goal,
            index,
            source_name,
            &signatures,
            &prelude,
            &mut functions,
            &mut ids,
        )?);
    }
    let entrypoints = goals.iter().map(|goal| goal.id.clone()).collect();
    let mut features = features_for(algorithm);
    if uses_self_hosted_all(program) {
        features.push(ALL_FEATURE.to_owned());
    }
    if uses_self_hosted_any(program) {
        features.push(ANY_FEATURE.to_owned());
    }
    Ok(SemanticIrDocument {
        features,
        functions,
        goals,
        entrypoints,
        semantic_id: None,
        artifact_id: None,
    })
}

fn uses_self_hosted_all(program: &ParsedProgram) -> bool {
    program.goals.iter().any(|goal| match &goal.body {
        Some(SurfaceComposition::DerivedAll { .. }) => true,
        Some(SurfaceComposition::Compose { reducer, .. }) => reducer == ALL_REDUCER,
        _ => false,
    })
}

fn uses_self_hosted_any(program: &ParsedProgram) -> bool {
    program.goals.iter().any(|goal| match &goal.body {
        Some(SurfaceComposition::DerivedAny { .. }) => true,
        Some(SurfaceComposition::Compose { reducer, .. }) => reducer == ANY_REDUCER,
        _ => false,
    })
}

#[derive(Clone)]
struct GoalSignature {
    id: String,
    input: BhcpType,
    output: BhcpType,
}

fn goal_signature(goal: &SurfaceGoal, index: usize, source_name: &str) -> Result<GoalSignature> {
    let mut names = HashSet::new();
    let mut input_fields = Vec::new();
    let mut output_fields = Vec::new();
    for clause in &goal.clauses {
        let SurfaceClauseKind::Fact {
            kind,
            name,
            value_type,
        } = &clause.kind
        else {
            continue;
        };
        if !names.insert(name.clone()) {
            return Err(error(
                "BHCP2002",
                format!("duplicate observable name {name:?}"),
                source_name,
                &clause.at,
            ));
        }
        let field = FieldType {
            name: name.clone(),
            value_type: lower_type(value_type, source_name, &clause.at)?,
        };
        if *kind == "input" {
            input_fields.push(field);
        } else {
            output_fields.push(field);
        }
    }
    input_fields.sort_by(|left, right| left.name.cmp(&right.name));
    output_fields.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(GoalSignature {
        id: format!("goal-{}", index + 1),
        input: BhcpType::Record(input_fields),
        output: BhcpType::Record(output_fields),
    })
}

fn lower_goal(
    goal: &SurfaceGoal,
    index: usize,
    source_name: &str,
    signatures: &HashMap<String, GoalSignature>,
    prelude: &Prelude,
    functions: &mut Vec<FunctionDefinition>,
    ids: &mut Ids,
) -> Result<GoalDefinition> {
    let clause_ids: Vec<_> = goal.clauses.iter().map(|_| ids.next("clause")).collect();
    let mut labels = HashMap::new();
    for (index, clause) in goal.clauses.iter().enumerate() {
        let Some(label) = &clause.label else {
            continue;
        };
        if labels
            .insert(label.clone(), (clause_ids[index].clone(), index))
            .is_some()
        {
            return Err(error(
                "BHCP2002",
                format!("duplicate clause label {label:?}"),
                source_name,
                &clause.at,
            ));
        }
    }
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
            let value_type = lower_type(value_type, source_name, &clause.at)?;
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
            SurfaceClauseKind::Verify {
                verifier,
                obligation_labels,
            } => {
                let mut obligations = obligation_labels
                    .iter()
                    .map(|label| {
                        let Some((id, target_index)) = labels.get(label) else {
                            return Err(error(
                                "BHCP2001",
                                format!("unresolved obligation label {label:?}"),
                                source_name,
                                &surface.at,
                            ));
                        };
                        if !matches!(
                            goal.clauses[*target_index].kind,
                            SurfaceClauseKind::Contract { .. }
                        ) {
                            return Err(error(
                                "BHCP2003",
                                format!("label {label:?} does not name a contract obligation"),
                                source_name,
                                &surface.at,
                            ));
                        }
                        Ok(id.clone())
                    })
                    .collect::<Result<Vec<_>>>()?;
                obligations.sort();
                obligations.dedup();
                ClauseKind::Verify {
                    binding: VerifierBinding {
                        verifier: verifier.clone(),
                        input: BhcpType::Record(vec![
                            FieldType {
                                name: "input".to_owned(),
                                value_type: input.clone(),
                            },
                            FieldType {
                                name: "output".to_owned(),
                                value_type: output.clone(),
                            },
                        ]),
                        output: BhcpType::Evidence(vec![]),
                        trust: vec![],
                    },
                    obligations,
                }
            }
        };
        clauses.push(Clause {
            id: clause_ids[clause_index].clone(),
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
    let signature = &signatures[&goal.symbol];
    debug_assert_eq!(signature.input, input);
    debug_assert_eq!(signature.output, output);
    let body = goal
        .body
        .as_ref()
        .map(|composition| {
            lower_composition(
                composition,
                signature,
                source_name,
                signatures,
                prelude,
                functions,
                ids,
            )
        })
        .transpose()?;
    Ok(GoalDefinition {
        id: format!("goal-{}", index + 1),
        symbol: goal.symbol.clone(),
        input,
        output,
        evidence: BhcpType::Evidence(vec![evidence.to_owned()]),
        clauses,
        body,
    })
}

fn lower_composition(
    composition: &SurfaceComposition,
    parent: &GoalSignature,
    source_name: &str,
    signatures: &HashMap<String, GoalSignature>,
    prelude: &Prelude,
    functions: &mut Vec<FunctionDefinition>,
    ids: &mut Ids,
) -> Result<KernelNetwork> {
    let mut tags = HashSet::new();
    let mut children = Vec::new();
    for branch in composition.branches() {
        if !tags.insert(branch.tag.clone()) {
            return Err(error(
                "BHCP2002",
                format!("duplicate composition tag {:?}", branch.tag),
                source_name,
                &branch.at,
            ));
        }
        let child = signatures.get(&branch.goal).ok_or_else(|| {
            error(
                "BHCP2001",
                format!("unresolved goal {}", branch.goal),
                source_name,
                &branch.at,
            )
        })?;
        if child.input != BhcpType::Record(vec![]) {
            return Err(error(
                "BHCP2004",
                "composition children with inputs require goal-call arguments, which are outside this slice",
                source_name,
                &branch.at,
            ));
        }
        children.push(DerivedChild {
            tag: branch.tag.clone(),
            goal: child.id.clone(),
            output: child.output.clone(),
            arguments: vec![],
        });
    }
    children.sort_by(|left, right| left.tag.cmp(&right.tag));
    let shape = match composition {
        SurfaceComposition::DerivedAll { .. } => prelude.lower(
            ALL_LOWERER,
            DerivedForm {
                input: parent.input.clone(),
                output: parent.output.clone(),
                children,
            },
        )?,
        SurfaceComposition::DerivedAny { .. } => prelude.lower(
            ANY_LOWERER,
            DerivedForm {
                input: parent.input.clone(),
                output: parent.output.clone(),
                children,
            },
        )?,
        SurfaceComposition::Compose { reducer, .. } => NetworkShape {
            output: parent.output.clone(),
            children,
            reducer: reducer.clone(),
        },
    };
    if shape.output != parent.output {
        return Err(error(
            "BHCP2003",
            "composition output does not match the parent goal output",
            source_name,
            composition.at(),
        ));
    }

    let mut observation_fields = Vec::new();
    for child in &shape.children {
        observation_fields.push(FieldType {
            name: child.tag.clone(),
            value_type: BhcpType::Option(Box::new(BhcpType::ExecutionResult(Box::new(
                child.output.clone(),
            )))),
        });
    }
    observation_fields.sort_by(|left, right| left.name.cmp(&right.name));
    let observations = BhcpType::Record(observation_fields);
    let reducer_source = prelude.reducer(&shape.reducer).map_err(|_| {
        error(
            "BHCP2004",
            format!(
                "network reducer {} is outside the implemented prelude slice",
                shape.reducer
            ),
            source_name,
            composition.at(),
        )
    })?;
    let reducer_symbol =
        specialized_reducer_symbol(&shape.reducer, &parent.input, &observations, &parent.output)?;
    if !functions
        .iter()
        .any(|function| function.symbol == reducer_symbol)
    {
        functions.push(instantiate_reducer(
            reducer_source,
            reducer_symbol.clone(),
            parent.input.clone(),
            observations,
            parent.output.clone(),
            source_name,
            ids,
        )?);
    }

    Ok(KernelNetwork {
        id: ids.next("network"),
        output: parent.output.clone(),
        children: shape
            .children
            .into_iter()
            .map(|child| KernelChild {
                id: ids.next("child"),
                tag: child.tag,
                goal: child.goal,
                arguments: child.arguments,
            })
            .collect(),
        reducer: reducer_symbol,
    })
}

fn specialized_reducer_symbol(
    base: &str,
    input: &BhcpType,
    observations: &BhcpType,
    output: &BhcpType,
) -> Result<String> {
    let signature = Value::Array(vec![
        Value::Text(base.to_owned()),
        input.to_value(),
        observations.to_value(),
        output.to_value(),
    ]);
    let bytes = encode_deterministic(&signature)?;
    let digest = HashAlgorithm::default().hash(&bytes).digest;
    let suffix: String = digest[..16]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let (path, version) = base.rsplit_once('@').ok_or_else(|| {
        Diagnostic::plain(
            "BHCP3001",
            "prelude reducer is not a versioned semantic name",
        )
    })?;
    Ok(format!("{path}-{suffix}@{version}"))
}

fn instantiate_reducer(
    source: &SurfaceFunction,
    symbol: String,
    input: BhcpType,
    observations: BhcpType,
    output: BhcpType,
    source_name: &str,
    ids: &mut Ids,
) -> Result<FunctionDefinition> {
    let parent = Binding {
        id: ids.next("parameter"),
        name: source.parameters[0].name.clone(),
        value_type: input,
    };
    let observed = Binding {
        id: ids.next("parameter"),
        name: source.parameters[1].name.clone(),
        value_type: observations,
    };
    let environment = HashMap::from([
        (parent.name.clone(), parent.clone()),
        (observed.name.clone(), observed.clone()),
    ]);
    let result = BhcpType::Reduction(Box::new(output));
    let definition =
        lower_reducer_expression(&source.definition, &environment, &result, source_name, ids)?;
    if definition.value_type != result {
        return Err(Diagnostic::plain(
            "BHCP3001",
            "specialized reducer body does not match its result type",
        ));
    }
    Ok(FunctionDefinition {
        id: ids.next("function"),
        symbol,
        parameters: vec![parent, observed],
        result,
        definition,
    })
}

fn lower_reducer_expression(
    surface: &SurfaceExpression,
    environment: &HashMap<String, Binding>,
    result_type: &BhcpType,
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
                    "BHCP3001",
                    format!("unresolved prelude binding {name:?}"),
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
            let operand =
                lower_reducer_expression(operand, environment, result_type, source_name, ids)?;
            if operator != "!" || operand.value_type != BhcpType::Primitive("Bool") {
                return Err(error(
                    "BHCP3001",
                    format!("unsupported or ill-typed pure reducer unary operation {operator}"),
                    source_name,
                    at,
                ));
            }
            (
                BhcpType::Primitive("Bool"),
                ExpressionForm::Unary(operator.clone(), Box::new(operand)),
            )
        }
        SurfaceExpression::Binary {
            operator,
            left,
            right,
            at,
        } => {
            let left = lower_reducer_expression(left, environment, result_type, source_name, ids)?;
            let right =
                lower_reducer_expression(right, environment, result_type, source_name, ids)?;
            let valid = match operator.as_str() {
                "==" | "!=" => {
                    left.value_type == right.value_type
                        && environment
                            .get("observations")
                            .is_none_or(|binding| binding.value_type != left.value_type)
                }
                "&&" | "||" => {
                    left.value_type == BhcpType::Primitive("Bool")
                        && right.value_type == BhcpType::Primitive("Bool")
                }
                _ => false,
            };
            if !valid {
                return Err(error(
                    "BHCP3001",
                    format!("unsupported or ill-typed pure reducer binary operation {operator}"),
                    source_name,
                    at,
                ));
            }
            (
                BhcpType::Primitive("Bool"),
                ExpressionForm::Binary(operator.clone(), Box::new(left), Box::new(right)),
            )
        }
        SurfaceExpression::Call {
            function,
            arguments,
            at,
        } => {
            let arguments = arguments
                .iter()
                .map(|argument| {
                    lower_reducer_expression(argument, environment, result_type, source_name, ids)
                })
                .collect::<Result<Vec<_>>>()?;
            let observations_type = environment
                .get("observations")
                .map(|binding| &binding.value_type)
                .ok_or_else(|| Diagnostic::plain("BHCP3001", "missing observations parameter"))?;
            let BhcpType::Reduction(output_type) = result_type else {
                return Err(Diagnostic::plain(
                    "BHCP3001",
                    "reducer specialization requires a Reduction result",
                ));
            };
            let refs_type = BhcpType::List(Box::new(BhcpType::Primitive("Text")));
            let fault_type = BhcpType::Nominal("bhcp/kernel.fault@0".to_owned(), vec![]);
            let reason_type = BhcpType::Nominal("bhcp/kernel.reason@0".to_owned(), vec![]);
            let execution_type = BhcpType::ExecutionResult(output_type.clone());
            let argument_types: Vec<_> = arguments
                .iter()
                .map(|argument| argument.value_type.clone())
                .collect();
            let value_type = match function.as_str() {
                "bhcp/kernel.has-refuted@0"
                | "bhcp/kernel.has-missing@0"
                | "bhcp/kernel.has-faulted@0"
                | "bhcp/kernel.has-unresolved@0"
                | "bhcp/kernel.has-satisfied@0"
                | "bhcp/kernel.all-refuted@0"
                    if argument_types == [observations_type.clone()] =>
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
                    if argument_types == [observations_type.clone()] =>
                {
                    refs_type.clone()
                }
                "bhcp/kernel.first-fault@0" if argument_types == [observations_type.clone()] => {
                    fault_type.clone()
                }
                "bhcp/kernel.first-unresolved-reason@0"
                    if argument_types == [observations_type.clone()] =>
                {
                    reason_type.clone()
                }
                "bhcp/kernel.satisfied-record@0"
                | "bhcp/kernel.first-satisfied-output@0"
                | "bhcp/kernel.last-satisfied-output@0"
                    if argument_types == [observations_type.clone()] =>
                {
                    output_type.as_ref().clone()
                }
                "bhcp/kernel.first-satisfied-winner@0"
                    if argument_types == [observations_type.clone()]
                        && winner_type_matches(observations_type, output_type) =>
                {
                    output_type.as_ref().clone()
                }
                "bhcp/kernel.unit@0" if argument_types.is_empty() => BhcpType::Primitive("Unit"),
                "bhcp/kernel.pending@0" if argument_types == [refs_type.clone()] => {
                    result_type.clone()
                }
                "bhcp/kernel.refuted@0" if argument_types == [refs_type.clone()] => {
                    execution_type.clone()
                }
                "bhcp/kernel.faulted@0" if argument_types == [fault_type] => execution_type.clone(),
                "bhcp/kernel.unresolved@0"
                    if argument_types == [reason_type, refs_type.clone()] =>
                {
                    execution_type.clone()
                }
                "bhcp/kernel.satisfied@0"
                    if argument_types == [output_type.as_ref().clone(), refs_type] =>
                {
                    execution_type.clone()
                }
                "bhcp/kernel.conclude@0" if argument_types == [execution_type] => {
                    result_type.clone()
                }
                _ => {
                    return Err(error(
                        "BHCP3001",
                        format!("unregistered or ill-typed pure kernel primitive {function}"),
                        source_name,
                        at,
                    ));
                }
            };
            (
                value_type,
                ExpressionForm::Call(function.clone(), arguments),
            )
        }
        SurfaceExpression::If {
            condition,
            consequent,
            alternative,
            at,
        } => {
            let condition =
                lower_reducer_expression(condition, environment, result_type, source_name, ids)?;
            let consequent =
                lower_reducer_expression(consequent, environment, result_type, source_name, ids)?;
            let alternative =
                lower_reducer_expression(alternative, environment, result_type, source_name, ids)?;
            if condition.value_type != BhcpType::Primitive("Bool")
                || consequent.value_type != alternative.value_type
            {
                return Err(error(
                    "BHCP3001",
                    "prelude if expression is not total and consistently typed",
                    source_name,
                    at,
                ));
            }
            let value_type = consequent.value_type.clone();
            (
                value_type,
                ExpressionForm::If(
                    Box::new(condition),
                    Box::new(consequent),
                    Box::new(alternative),
                ),
            )
        }
    };
    Ok(Expression {
        id: ids.next("expr"),
        value_type,
        form,
    })
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

fn lower_type(
    value: &SurfaceType,
    source_name: &str,
    at: &crate::model::Point,
) -> Result<BhcpType> {
    Ok(match value {
        SurfaceType::Primitive(name) => BhcpType::Primitive(name),
        SurfaceType::Exact(name) => BhcpType::ExactNumber(name),
        SurfaceType::Record(fields) => {
            let mut lowered = fields
                .iter()
                .map(|field| {
                    Ok(FieldType {
                        name: field.name.clone(),
                        value_type: lower_type(&field.value_type, source_name, at)?,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            lowered.sort_by(|left, right| left.name.cmp(&right.name));
            BhcpType::Record(lowered)
        }
        SurfaceType::Reduction(output) => {
            BhcpType::Reduction(Box::new(lower_type(output, source_name, at)?))
        }
        SurfaceType::Parameter(_) | SurfaceType::Dynamic | SurfaceType::Meta { .. } => {
            return Err(error(
                "BHCP2004",
                "compile-time and generic types are not permitted in executable goal facts",
                source_name,
                at,
            ));
        }
    })
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
        SurfaceExpression::Call { at, .. } | SurfaceExpression::If { at, .. } => {
            return Err(error(
                "BHCP2004",
                "function calls and conditionals are outside the implemented goal-expression slice",
                source_name,
                at,
            ));
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
