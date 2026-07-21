//! Deterministic construction and exact validation of structural obligation graphs.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::cbor::encode_deterministic;
use crate::diagnostic::{Diagnostic, Result};
use crate::graph::{GraphDocument, GraphKind};
use crate::hash::{HashAlgorithm, artifact_hash_with, hash_value};
use crate::model::ContentReference;
use crate::model::{ClauseKind, Expression, ExpressionForm, GoalDefinition};
use crate::pipeline::Compilation;
use crate::policy::{
    EffectivePolicyDocument, PolicyCategory, PolicyDocument, PolicyLayer, RuleProvenance,
};
use crate::value::Value;

const INVALID_INPUT: &str = "BHCP7101";
const INVALID_CONSTRUCTION: &str = "BHCP7102";
const GRAPH_MISMATCH: &str = "BHCP7103";
const OBLIGATION_FEATURE: &str = "bhcp/feature.obligation-graph-builder@0";

#[derive(Clone)]
struct ContractIndex {
    by_source: BTreeMap<String, String>,
    by_id: BTreeMap<String, String>,
}

pub fn build_obligation_graph(compilation: &Compilation) -> Result<GraphDocument> {
    let algorithm = validate_compilation(compilation)?;
    let mut nodes = BTreeMap::<String, Value>::new();
    let mut edges = BTreeMap::<String, Value>::new();
    let mut contracts = BTreeMap::<String, ContractIndex>::new();

    let mut goals = compilation.ir.goals.iter().collect::<Vec<_>>();
    goals.sort_by(|left, right| left.symbol.cmp(&right.symbol));
    for goal in &goals {
        let index = add_contract_nodes(goal, &mut nodes)?;
        add_case_nodes(goal, &mut nodes)?;
        add_verifier_nodes(goal, &index, &mut nodes, &mut edges)?;
        contracts.insert(goal.symbol.clone(), index);
    }

    add_parent_child_dependencies(&goals, &contracts, &mut nodes, &mut edges)?;
    if let Some(policy) = &compilation.effective_policy {
        add_policy_nodes(&goals, policy, &mut nodes)?;
    }

    let semantic_bytes = encode_deterministic(&compilation.ir.semantic_value())?;
    let semantic_ir = ContentReference::from_bytes(
        "application/vnd.bhcp.semantic-ir+cbor",
        &semantic_bytes,
        algorithm,
    );
    if semantic_ir.digests.as_slice() != [compilation.semantic_hash.clone()] {
        return Err(invalid_input(
            "semantic IR projection does not match the compilation semantic identity",
        ));
    }
    let value = Value::map([
        ("version", Value::Text("bhcp/v0".to_owned())),
        (
            "features",
            Value::Array(vec![Value::Text(OBLIGATION_FEATURE.to_owned())]),
        ),
        ("kind", Value::Text("obligation-graph".to_owned())),
        ("semantic_ir", semantic_ir.to_value()),
        ("nodes", Value::Array(nodes.into_values().collect())),
        ("edges", Value::Array(edges.into_values().collect())),
    ]);
    let mut graph = GraphDocument::from_value(&value)?;
    graph.materialize_identities(algorithm)?;
    Ok(graph)
}

pub fn validate_obligation_graph(compilation: &Compilation, graph: &GraphDocument) -> Result<()> {
    if graph.kind() != GraphKind::Obligation
        || graph.semantic_id().is_none()
        || graph.artifact_id().is_none()
    {
        return Err(mismatch(
            "planning requires a materialized obligation graph",
        ));
    }
    let expected = build_obligation_graph(compilation)?;
    if graph.to_value() != expected.to_value() {
        return Err(mismatch(
            "obligation graph does not match deterministic construction from semantic IR",
        ));
    }
    Ok(())
}

pub(crate) fn contract_target_map(goal: &GoalDefinition) -> Result<BTreeMap<String, String>> {
    Ok(contract_index(goal)?.by_source)
}

pub(crate) fn policy_obligation_id(category: PolicyCategory, value: &Value) -> Result<String> {
    structural_id(
        "policy",
        &Value::map([
            ("category", Value::Text(category.as_str().to_owned())),
            ("value", value.clone()),
        ]),
    )
}

pub(crate) fn validate_compilation(compilation: &Compilation) -> Result<HashAlgorithm> {
    compilation
        .ir
        .validate()
        .map_err(|error| invalid_input(format!("invalid semantic IR: {}", error.message)))?;
    let encoded = encode_deterministic(&compilation.ir.to_value(true))?;
    if encoded != compilation.ir_bytes {
        return Err(invalid_input(
            "semantic IR bytes do not match the retained typed document",
        ));
    }
    let algorithm = HashAlgorithm::from_id(&compilation.semantic_hash.algorithm)
        .map_err(|error| invalid_input(error.message))?;
    let semantic_hash = hash_value(&compilation.ir.semantic_value(), algorithm)?;
    if semantic_hash != compilation.semantic_hash
        || compilation.ir.semantic_id.as_ref() != Some(&compilation.semantic_hash)
    {
        return Err(invalid_input(
            "semantic IR identity does not match the compilation envelope",
        ));
    }
    let artifact_algorithm = HashAlgorithm::from_id(&compilation.ir_hash.algorithm)
        .map_err(|error| invalid_input(error.message))?;
    let artifact_hash = artifact_hash_with(&compilation.ir.to_value(false), artifact_algorithm)?;
    if artifact_algorithm != algorithm
        || artifact_hash != compilation.ir_hash
        || compilation.ir.artifact_id.as_ref() != Some(&compilation.ir_hash)
    {
        return Err(invalid_input(
            "semantic IR artifact identity does not match the compilation envelope",
        ));
    }

    match (
        &compilation.effective_policy,
        &compilation.ir.effective_policy,
    ) {
        (None, None) => {
            if compilation
                .ir
                .goals
                .iter()
                .any(|goal| goal.policy_decision.is_some())
            {
                return Err(invalid_input(
                    "policy decisions require the retained effective policy document",
                ));
            }
        }
        (Some(policy), Some(reference)) => {
            PolicyDocument::Effective(policy.clone())
                .validate()
                .map_err(|error| invalid_input(format!("invalid policy: {}", error.message)))?;
            if policy.header.semantic_id.as_ref() != Some(&reference.semantic_id)
                || policy.header.artifact_id.as_ref() != Some(&reference.artifact_id)
            {
                return Err(invalid_input(
                    "retained effective policy does not match semantic IR identities",
                ));
            }
            for goal in &compilation.ir.goals {
                validate_policy_decision(goal, policy)?;
            }
        }
        _ => {
            return Err(invalid_input(
                "effective policy envelope is incomplete or inconsistent",
            ));
        }
    }
    Ok(algorithm)
}

pub(crate) fn validate_policy_decision(
    goal: &GoalDefinition,
    policy: &EffectivePolicyDocument,
) -> Result<()> {
    let decision = goal
        .policy_decision
        .as_ref()
        .ok_or_else(|| invalid_input("effective policy requires a goal policy decision"))?;
    let requirements = applicable_indices(&policy.effective.requirements, &goal.symbol, |rule| {
        rule.value.scope.as_ref()
    });
    let evidence = applicable_indices(&policy.effective.evidence, &goal.symbol, |rule| {
        rule.value.scope.as_ref()
    });
    let prohibitions = applicable_indices(&policy.effective.prohibitions, &goal.symbol, |rule| {
        rule.value.scope.as_ref()
    });
    let capabilities = applicable_indices(&policy.effective.capabilities, &goal.symbol, |rule| {
        rule.value.scope.as_ref()
    });
    let limits = applicable_indices(&policy.effective.limits, &goal.symbol, |rule| {
        rule.value.scope.as_ref()
    });
    if goal.type_mode < policy.effective.type_mode.value
        || decision.type_mode != goal.type_mode.as_str()
        || decision.requirements != requirements
        || decision.evidence != evidence
        || decision.prohibitions != prohibitions
        || decision.capabilities != capabilities
        || decision.limits != limits
    {
        return Err(invalid_input(
            "goal policy decision does not match retained effective policy applicability",
        ));
    }
    Ok(())
}

fn applicable_indices<T, F>(rules: &[T], goal: &str, scope: F) -> Vec<usize>
where
    F: Fn(&T) -> Option<&crate::policy::PolicyScope>,
{
    rules
        .iter()
        .enumerate()
        .filter_map(|(index, rule)| {
            scope(rule)
                .is_none_or(|scope| {
                    scope
                        .goals
                        .as_ref()
                        .is_none_or(|goals| goals.iter().any(|candidate| candidate == goal))
                })
                .then_some(index)
        })
        .collect()
}

fn add_contract_nodes(
    goal: &GoalDefinition,
    nodes: &mut BTreeMap<String, Value>,
) -> Result<ContractIndex> {
    let facts = fact_coordinates(goal);
    let mut by_source = BTreeMap::new();
    let mut by_id = BTreeMap::new();
    let mut source_sets = BTreeMap::<String, BTreeSet<String>>::new();

    for clause in &goal.clauses {
        let ClauseKind::Contract {
            kind,
            dimension,
            condition,
        } = &clause.kind
        else {
            continue;
        };
        let node_kind = contract_kind(kind)?;
        let key = Value::map([
            ("goal", Value::Text(goal.symbol.clone())),
            ("kind", Value::Text(node_kind.to_owned())),
            (
                "dimension",
                dimension.clone().map_or(Value::Null, Value::Text),
            ),
            ("condition", expression_key(condition, &facts)),
        ]);
        let id = structural_id("contract", &key)?;
        by_source.insert(clause.id.clone(), id.clone());
        by_id.insert(id.clone(), node_kind.to_owned());
        source_sets
            .entry(id.clone())
            .or_default()
            .insert(clause.id.clone());
        nodes.entry(id.clone()).or_insert_with(|| {
            Value::map([
                ("id", Value::Text(id.clone())),
                ("kind", Value::Text(node_kind.to_owned())),
                ("clause", Value::Text(id.clone())),
                ("status", Value::Text("open".to_owned())),
                (
                    "goals",
                    Value::Array(vec![Value::Text(goal.symbol.clone())]),
                ),
                ("source_clauses", Value::Array(Vec::new())),
            ])
        });
    }
    for (id, sources) in source_sets {
        replace_field(
            nodes.get_mut(&id).expect("inserted contract node"),
            "source_clauses",
            Value::Array(sources.into_iter().map(Value::Text).collect()),
        )?;
    }
    Ok(ContractIndex { by_source, by_id })
}

fn contract_index(goal: &GoalDefinition) -> Result<ContractIndex> {
    add_contract_nodes(goal, &mut BTreeMap::new())
}

fn add_case_nodes(goal: &GoalDefinition, nodes: &mut BTreeMap<String, Value>) -> Result<()> {
    let mut source_sets = BTreeMap::<String, BTreeSet<String>>::new();
    for clause in &goal.clauses {
        let ClauseKind::Case { inputs, expected } = &clause.kind else {
            continue;
        };
        let key = Value::map([
            ("goal", Value::Text(goal.symbol.clone())),
            (
                "inputs",
                Value::owned_map(inputs.clone().into_iter().collect()),
            ),
            ("expected", expected.to_value()),
        ]);
        let id = structural_id("case", &key)?;
        let node = Value::map([
            ("id", Value::Text(id.clone())),
            ("kind", Value::Text("case".to_owned())),
            ("clause", Value::Text(id.clone())),
            ("status", Value::Text("open".to_owned())),
            (
                "goals",
                Value::Array(vec![Value::Text(goal.symbol.clone())]),
            ),
            (
                "source_clauses",
                Value::Array(vec![Value::Text(clause.id.clone())]),
            ),
        ]);
        source_sets
            .entry(id.clone())
            .or_default()
            .insert(clause.id.clone());
        nodes.entry(id).or_insert(node);
    }
    for (id, sources) in source_sets {
        replace_field(
            nodes.get_mut(&id).expect("inserted case node"),
            "source_clauses",
            Value::Array(sources.into_iter().map(Value::Text).collect()),
        )?;
    }
    Ok(())
}

fn add_verifier_nodes(
    goal: &GoalDefinition,
    contracts: &ContractIndex,
    nodes: &mut BTreeMap<String, Value>,
    edges: &mut BTreeMap<String, Value>,
) -> Result<()> {
    let mut source_sets = BTreeMap::<String, BTreeSet<String>>::new();
    for clause in &goal.clauses {
        let ClauseKind::Verify {
            binding,
            obligations,
        } = &clause.kind
        else {
            continue;
        };
        let targets = if obligations.is_empty() {
            contracts.by_id.keys().cloned().collect::<Vec<_>>()
        } else {
            obligations
                .iter()
                .map(|source| {
                    contracts.by_source.get(source).cloned().ok_or_else(|| {
                        invalid_construction(format!(
                            "verifier target {source:?} is not a structural contract obligation"
                        ))
                    })
                })
                .collect::<Result<Vec<_>>>()?
        };
        let mut targets = targets;
        targets.sort();
        targets.dedup();
        let key = Value::map([
            ("goal", Value::Text(goal.symbol.clone())),
            ("verifier", Value::Text(binding.verifier.clone())),
            ("input", binding.input.to_value()),
            ("output", binding.output.to_value()),
            (
                "trust",
                Value::Array(binding.trust.iter().cloned().map(Value::Text).collect()),
            ),
            (
                "targets",
                Value::Array(targets.iter().cloned().map(Value::Text).collect()),
            ),
        ]);
        let id = structural_id("verification", &key)?;
        source_sets
            .entry(id.clone())
            .or_default()
            .insert(clause.id.clone());
        nodes.entry(id.clone()).or_insert_with(|| {
            Value::map([
                ("id", Value::Text(id.clone())),
                ("kind", Value::Text("verification".to_owned())),
                ("clause", Value::Text(id.clone())),
                ("status", Value::Text("open".to_owned())),
                (
                    "goals",
                    Value::Array(vec![Value::Text(goal.symbol.clone())]),
                ),
                ("source_clauses", Value::Array(Vec::new())),
            ])
        });
        for target in targets {
            add_edge(edges, "verifies", &id, &target)?;
        }
    }
    for (id, sources) in source_sets {
        replace_field(
            nodes.get_mut(&id).expect("inserted verifier node"),
            "source_clauses",
            Value::Array(sources.into_iter().map(Value::Text).collect()),
        )?;
    }
    Ok(())
}

fn add_parent_child_dependencies(
    goals: &[&GoalDefinition],
    contracts: &BTreeMap<String, ContractIndex>,
    nodes: &mut BTreeMap<String, Value>,
    edges: &mut BTreeMap<String, Value>,
) -> Result<()> {
    let mut goal_by_reference = HashMap::new();
    for goal in goals {
        goal_by_reference.insert(goal.symbol.as_str(), *goal);
        goal_by_reference.insert(goal.id.as_str(), *goal);
    }
    for parent in goals {
        let Some(network) = &parent.body else {
            continue;
        };
        for child in &network.children {
            let child_goal = goal_by_reference
                .get(child.goal.as_str())
                .copied()
                .ok_or_else(|| invalid_construction("network child goal is missing"))?;
            let child_contracts = contracts
                .get(&child_goal.symbol)
                .expect("every goal has a contract index");
            for (requirement, kind) in &child_contracts.by_id {
                if kind != "requirement" {
                    continue;
                }
                let key = Value::map([
                    ("parent", Value::Text(parent.symbol.clone())),
                    ("child_tag", Value::Text(child.tag.clone())),
                    ("child_goal", Value::Text(child_goal.symbol.clone())),
                    ("requirement", Value::Text(requirement.clone())),
                ]);
                let id = structural_id("discharge", &key)?;
                nodes.entry(id.clone()).or_insert_with(|| {
                    Value::map([
                        ("id", Value::Text(id.clone())),
                        ("kind", Value::Text("discharge".to_owned())),
                        ("clause", Value::Text(id.clone())),
                        ("status", Value::Text("open".to_owned())),
                        (
                            "goals",
                            Value::Array(vec![Value::Text(parent.symbol.clone())]),
                        ),
                        (
                            "source_clauses",
                            Value::Array(vec![Value::Text(child.id.clone())]),
                        ),
                    ])
                });
                add_edge(edges, "depends-on", &id, requirement)?;
            }
        }
    }
    Ok(())
}

fn add_policy_nodes(
    goals: &[&GoalDefinition],
    policy: &EffectivePolicyDocument,
    nodes: &mut BTreeMap<String, Value>,
) -> Result<()> {
    let mut requirements = BTreeMap::<usize, BTreeSet<String>>::new();
    let mut evidence = BTreeMap::<usize, BTreeSet<String>>::new();
    let mut limits = BTreeMap::<usize, BTreeSet<String>>::new();
    for goal in goals {
        let Some(decision) = &goal.policy_decision else {
            continue;
        };
        for (indices, destination) in [
            (&decision.requirements, &mut requirements),
            (&decision.evidence, &mut evidence),
            (&decision.limits, &mut limits),
        ] {
            for index in indices {
                destination
                    .entry(*index)
                    .or_default()
                    .insert(goal.symbol.clone());
            }
        }
    }

    for (index, goals) in requirements {
        let value = policy
            .effective
            .requirements
            .get(index)
            .ok_or_else(|| invalid_input("policy requirement index is out of range"))?
            .value
            .to_value();
        add_policy_node(
            PolicyCategory::Requirement,
            index,
            value,
            goals,
            policy,
            nodes,
        )?;
    }
    for (index, goals) in evidence {
        let value = policy
            .effective
            .evidence
            .get(index)
            .ok_or_else(|| invalid_input("policy evidence index is out of range"))?
            .value
            .to_value();
        add_policy_node(PolicyCategory::Evidence, index, value, goals, policy, nodes)?;
    }
    for (index, goals) in limits {
        let value = policy
            .effective
            .limits
            .get(index)
            .ok_or_else(|| invalid_input("policy limit index is out of range"))?
            .value
            .to_value();
        add_policy_node(PolicyCategory::Limit, index, value, goals, policy, nodes)?;
    }
    Ok(())
}

fn add_policy_node(
    category: PolicyCategory,
    effective_rule: usize,
    value: Value,
    goals: BTreeSet<String>,
    policy: &EffectivePolicyDocument,
    nodes: &mut BTreeMap<String, Value>,
) -> Result<()> {
    let id = policy_obligation_id(category, &value)?;
    let node_kind = match category {
        PolicyCategory::Requirement => "requirement",
        PolicyCategory::Evidence => "verification",
        PolicyCategory::Limit => "limit",
        _ => {
            return Err(invalid_construction(
                "unsupported policy obligation category",
            ));
        }
    };
    let sources = policy_sources(policy, category, effective_rule)?;
    let node = Value::map([
        ("id", Value::Text(id.clone())),
        ("kind", Value::Text(node_kind.to_owned())),
        ("clause", Value::Text(id.clone())),
        ("status", Value::Text("open".to_owned())),
        (
            "goals",
            Value::Array(goals.into_iter().map(Value::Text).collect()),
        ),
        (
            "policy",
            Value::map([
                ("category", Value::Text(category.as_str().to_owned())),
                ("effective_rule", Value::Integer(effective_rule as i128)),
                ("value", value),
                ("sources", Value::Array(sources)),
            ]),
        ),
    ]);
    if nodes.insert(id, node).is_some() {
        return Err(invalid_construction(
            "policy obligation structural ID collides with an existing node",
        ));
    }
    Ok(())
}

fn policy_sources(
    policy: &EffectivePolicyDocument,
    category: PolicyCategory,
    effective_rule: usize,
) -> Result<Vec<Value>> {
    let provenance = policy
        .rule_provenance
        .iter()
        .find(|entry| entry.category == category && entry.effective_rule == effective_rule)
        .ok_or_else(|| invalid_input("effective policy obligation has no source provenance"))?;
    provenance_sources(policy, provenance)
}

fn provenance_sources(
    policy: &EffectivePolicyDocument,
    provenance: &RuleProvenance,
) -> Result<Vec<Value>> {
    let mut sources = Vec::new();
    for source in &provenance.sources {
        let layer = policy
            .source_layers
            .iter()
            .find(|layer| {
                layer
                    .policies
                    .iter()
                    .any(|candidate| candidate.symbol == source.policy)
            })
            .ok_or_else(|| invalid_input("policy source provenance has no source layer"))?;
        sources.push(Value::map([
            ("layer", Value::Text(layer_name(layer.layer).to_owned())),
            ("policy", Value::Text(source.policy.clone())),
            ("rule", Value::Text(source.rule.clone())),
        ]));
    }
    Ok(sources)
}

fn layer_name(layer: PolicyLayer) -> &'static str {
    layer.as_str()
}

fn fact_coordinates(goal: &GoalDefinition) -> HashMap<String, Value> {
    goal.clauses
        .iter()
        .filter_map(|clause| {
            let ClauseKind::Fact { kind, binding } = &clause.kind else {
                return None;
            };
            Some((
                binding.id.clone(),
                Value::map([
                    ("kind", Value::Text((*kind).to_owned())),
                    ("name", Value::Text(binding.name.clone())),
                    ("type", binding.value_type.to_value()),
                ]),
            ))
        })
        .collect()
}

fn expression_key(expression: &Expression, facts: &HashMap<String, Value>) -> Value {
    let form = match &expression.form {
        ExpressionForm::Literal(value) => {
            Value::Array(vec![Value::Text("literal".to_owned()), value.clone()])
        }
        ExpressionForm::Reference(reference) => Value::Array(vec![
            Value::Text("reference".to_owned()),
            facts
                .get(reference)
                .cloned()
                .unwrap_or_else(|| Value::Text(reference.clone())),
        ]),
        ExpressionForm::Unary(operator, operand) => Value::Array(vec![
            Value::Text("unary".to_owned()),
            Value::Text(operator.clone()),
            expression_key(operand, facts),
        ]),
        ExpressionForm::Binary(operator, left, right) => Value::Array(vec![
            Value::Text("binary".to_owned()),
            Value::Text(operator.clone()),
            expression_key(left, facts),
            expression_key(right, facts),
        ]),
        ExpressionForm::If(condition, consequent, alternative) => Value::Array(vec![
            Value::Text("if".to_owned()),
            expression_key(condition, facts),
            expression_key(consequent, facts),
            expression_key(alternative, facts),
        ]),
        ExpressionForm::Call(function, arguments) => Value::Array(vec![
            Value::Text("call".to_owned()),
            Value::Text(function.clone()),
            Value::Array(
                arguments
                    .iter()
                    .map(|argument| expression_key(argument, facts))
                    .collect(),
            ),
        ]),
    };
    Value::map([("type", expression.value_type.to_value()), ("form", form)])
}

fn contract_kind(kind: &str) -> Result<&'static str> {
    match kind {
        "requires" => Ok("requirement"),
        "ensures" => Ok("guarantee"),
        "invariant" => Ok("invariant"),
        "limit" => Ok("limit"),
        _ => Err(invalid_construction(format!(
            "unsupported contract kind {kind:?}"
        ))),
    }
}

fn add_edge(edges: &mut BTreeMap<String, Value>, kind: &str, from: &str, to: &str) -> Result<()> {
    let key = Value::map([
        ("kind", Value::Text(kind.to_owned())),
        ("from", Value::Text(from.to_owned())),
        ("to", Value::Text(to.to_owned())),
    ]);
    let id = structural_id("edge", &key)?;
    edges.entry(id.clone()).or_insert_with(|| {
        Value::map([
            ("id", Value::Text(id)),
            ("from", Value::Text(from.to_owned())),
            ("to", Value::Text(to.to_owned())),
            ("kind", Value::Text(kind.to_owned())),
        ])
    });
    Ok(())
}

fn structural_id(domain: &str, key: &Value) -> Result<String> {
    let digest = hash_value(
        &Value::map([
            ("domain", Value::Text(domain.to_owned())),
            ("key", key.clone()),
        ]),
        HashAlgorithm::default(),
    )?;
    let mut id = String::with_capacity(66);
    id.push_str("o-");
    for byte in digest.digest.iter().take(32) {
        use std::fmt::Write as _;
        write!(id, "{byte:02x}").expect("writing to String cannot fail");
    }
    Ok(id)
}

fn replace_field(value: &mut Value, field: &str, replacement: Value) -> Result<()> {
    let Value::Map(entries) = value else {
        return Err(invalid_construction("graph node is not a map"));
    };
    let Some((_, value)) = entries.iter_mut().find(|(name, _)| name == field) else {
        return Err(invalid_construction(format!(
            "graph node is missing field {field:?}"
        )));
    };
    *value = replacement;
    Ok(())
}

fn invalid_input(message: impl Into<String>) -> Diagnostic {
    Diagnostic::plain(INVALID_INPUT, message)
}

fn invalid_construction(message: impl Into<String>) -> Diagnostic {
    Diagnostic::plain(INVALID_CONSTRUCTION, message)
}

fn mismatch(message: impl Into<String>) -> Diagnostic {
    Diagnostic::plain(GRAPH_MISMATCH, message)
}
