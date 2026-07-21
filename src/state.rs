//! Deterministic construction of analysis-only ownership and persistent-state graphs.

use std::collections::{BTreeMap, HashMap};

use crate::capability::build_capability_graph;
use crate::cbor::encode_deterministic;
use crate::diagnostic::{Diagnostic, Result};
use crate::graph::{GraphDocument, GraphKind};
use crate::hash::{HashAlgorithm, hash_value};
use crate::kernel::ArgumentMode;
use crate::model::{
    BhcpType, ClauseKind, ContentReference, Expression, ExpressionForm, GoalDefinition, HandleType,
};
use crate::obligation::validate_compilation;
use crate::pipeline::Compilation;
use crate::value::Value;

const INVALID_INPUT: &str = "BHCP7401";
const INVALID_CONSTRUCTION: &str = "BHCP7402";
const GRAPH_MISMATCH: &str = "BHCP7403";
pub const STATE_FEATURE: &str = "bhcp/feature.state-graph-builder@0";

#[derive(Clone)]
struct ResourceCoordinate {
    id: String,
    ownership: Option<String>,
    binding: String,
    goal: String,
    name: String,
    value_type: BhcpType,
    handle: Option<HandleType>,
}

pub fn build_state_graph(compilation: &Compilation) -> Result<GraphDocument> {
    let algorithm =
        validate_compilation(compilation).map_err(|error| invalid_input(error.message))?;
    let mut nodes = BTreeMap::new();
    let mut edges = BTreeMap::new();
    let mut transitions = BTreeMap::new();
    let mut resources = HashMap::<(String, String), ResourceCoordinate>::new();
    let capability = build_capability_graph(compilation).map_err(|error| {
        invalid_input(format!("capability graph is invalid: {}", error.message))
    })?;

    let mut goals = compilation.ir.goals.iter().collect::<Vec<_>>();
    goals.sort_by(|left, right| left.symbol.cmp(&right.symbol));
    for goal in &goals {
        add_handle_resources(goal, &mut resources, &mut nodes, &mut edges)?;
    }
    for goal in &goals {
        add_authored_invariants(goal, &mut nodes)?;
        add_ownership_edges(goal, &resources, &mut nodes, &mut edges)?;
        if goal
            .body
            .as_ref()
            .is_some_and(|network| network.reducer.starts_with("bhcp/prelude.retain-reducer-"))
        {
            add_retention_topology(
                goal,
                &goals,
                &capability,
                &mut resources,
                &mut nodes,
                &mut edges,
                &mut transitions,
            )?;
        }
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
            Value::Array(vec![Value::Text(STATE_FEATURE.to_owned())]),
        ),
        ("kind", Value::Text("state-graph".to_owned())),
        ("semantic_ir", semantic_ir.to_value()),
        ("nodes", Value::Array(nodes.into_values().collect())),
        ("edges", Value::Array(edges.into_values().collect())),
        (
            "transitions",
            Value::Array(transitions.into_values().collect()),
        ),
    ]);
    let mut graph = GraphDocument::from_value(&value)?;
    graph.materialize_identities(algorithm)?;
    Ok(graph)
}

pub fn validate_state_graph(compilation: &Compilation, graph: &GraphDocument) -> Result<()> {
    if graph.kind() != GraphKind::State
        || graph.semantic_id().is_none()
        || graph.artifact_id().is_none()
    {
        return Err(mismatch("planning requires a materialized state graph"));
    }
    if graph.to_value() != build_state_graph(compilation)?.to_value() {
        return Err(mismatch(
            "state graph does not match deterministic construction from semantic IR",
        ));
    }
    Ok(())
}

fn add_handle_resources(
    goal: &GoalDefinition,
    resources: &mut HashMap<(String, String), ResourceCoordinate>,
    nodes: &mut BTreeMap<String, Value>,
    edges: &mut BTreeMap<String, Value>,
) -> Result<()> {
    for clause in &goal.clauses {
        let ClauseKind::Fact { binding, .. } = &clause.kind else {
            continue;
        };
        let BhcpType::Handle(_) = &binding.value_type else {
            continue;
        };
        let coordinate =
            resource_coordinate(goal, &binding.id, &binding.name, &binding.value_type)?;
        insert_resource(&coordinate, nodes, edges)?;
        resources.insert((goal.symbol.clone(), binding.name.clone()), coordinate);
    }
    Ok(())
}

fn resource_coordinate(
    goal: &GoalDefinition,
    binding: &str,
    name: &str,
    value_type: &BhcpType,
) -> Result<ResourceCoordinate> {
    let id = structural_id(
        "resource",
        &Value::map([
            ("goal", Value::Text(goal.symbol.clone())),
            ("binding", Value::Text(binding.to_owned())),
            ("type", value_type.to_value()),
        ]),
    )?;
    let handle = match value_type {
        BhcpType::Handle(handle) => Some(handle.as_ref().clone()),
        _ => None,
    };
    let ownership = handle
        .as_ref()
        .map(|_| {
            structural_id(
                "ownership",
                &Value::map([
                    ("resource", Value::Text(id.clone())),
                    ("handle", value_type.to_value()),
                ]),
            )
        })
        .transpose()?;
    Ok(ResourceCoordinate {
        id,
        ownership,
        binding: binding.to_owned(),
        goal: goal.symbol.clone(),
        name: name.to_owned(),
        value_type: value_type.clone(),
        handle,
    })
}

fn insert_resource(
    resource: &ResourceCoordinate,
    nodes: &mut BTreeMap<String, Value>,
    edges: &mut BTreeMap<String, Value>,
) -> Result<()> {
    insert_node(
        nodes,
        resource.id.clone(),
        Value::map([
            ("id", Value::Text(resource.id.clone())),
            ("kind", Value::Text("resource".to_owned())),
            (
                "payload",
                Value::map([
                    ("goal", Value::Text(resource.goal.clone())),
                    ("binding", Value::Text(resource.binding.clone())),
                    ("name", Value::Text(resource.name.clone())),
                    ("type", resource.value_type.to_value()),
                ]),
            ),
        ]),
    )?;
    if let (Some(handle), Some(ownership)) = (&resource.handle, &resource.ownership) {
        insert_node(
            nodes,
            ownership.clone(),
            Value::map([
                ("id", Value::Text(ownership.clone())),
                ("kind", Value::Text("ownership".to_owned())),
                (
                    "handle",
                    BhcpType::Handle(Box::new(handle.clone())).to_value(),
                ),
                (
                    "payload",
                    Value::map([
                        ("goal", Value::Text(resource.goal.clone())),
                        ("binding", Value::Text(resource.binding.clone())),
                        ("resource", Value::Text(resource.id.clone())),
                    ]),
                ),
            ]),
        )?;
        add_edge(edges, &resource.id, ownership, "owns")?;
    }
    Ok(())
}

fn add_authored_invariants(
    goal: &GoalDefinition,
    nodes: &mut BTreeMap<String, Value>,
) -> Result<()> {
    for clause in &goal.clauses {
        let ClauseKind::Contract {
            kind: "invariant",
            condition,
            ..
        } = &clause.kind
        else {
            continue;
        };
        let id = structural_id(
            "invariant",
            &Value::map([
                ("goal", Value::Text(goal.symbol.clone())),
                ("clause", Value::Text(clause.id.clone())),
            ]),
        )?;
        insert_node(
            nodes,
            id.clone(),
            Value::map([
                ("id", Value::Text(id)),
                ("kind", Value::Text("invariant".to_owned())),
                (
                    "payload",
                    Value::map([
                        ("goal", Value::Text(goal.symbol.clone())),
                        ("clause", Value::Text(clause.id.clone())),
                        ("condition", condition.to_value()),
                    ]),
                ),
            ]),
        )?;
    }
    Ok(())
}

fn add_ownership_edges(
    goal: &GoalDefinition,
    resources: &HashMap<(String, String), ResourceCoordinate>,
    nodes: &mut BTreeMap<String, Value>,
    edges: &mut BTreeMap<String, Value>,
) -> Result<()> {
    let Some(network) = &goal.body else {
        return Ok(());
    };
    let mut by_resource = BTreeMap::<String, Vec<(String, String)>>::new();
    for child in &network.children {
        for argument in &child.arguments {
            if argument.mode == ArgumentMode::Value {
                continue;
            }
            let Some(name) = parent_field_name(&argument.value) else {
                continue;
            };
            let Some(resource) = resources.get(&(goal.symbol.clone(), name.to_owned())) else {
                continue;
            };
            let Some(ownership) = &resource.ownership else {
                continue;
            };
            let (kind, edge_kind) = match argument.mode {
                ArgumentMode::Borrow | ArgumentMode::Share => ("borrow", "borrows"),
                ArgumentMode::Move => ("transition", "moves"),
                ArgumentMode::Value => unreachable!(),
            };
            let mode = argument_mode(argument.mode);
            let id = structural_id(
                "ownership-edge",
                &Value::map([
                    ("goal", Value::Text(goal.symbol.clone())),
                    ("child", Value::Text(child.id.clone())),
                    ("argument", Value::Text(argument.name.clone())),
                    ("resource", Value::Text(resource.id.clone())),
                    ("mode", Value::Text(mode.to_owned())),
                ]),
            )?;
            let handle = resource
                .handle
                .as_ref()
                .expect("ownership coordinate has a handle");
            insert_node(
                nodes,
                id.clone(),
                Value::map([
                    ("id", Value::Text(id.clone())),
                    ("kind", Value::Text(kind.to_owned())),
                    (
                        "handle",
                        BhcpType::Handle(Box::new(handle.clone())).to_value(),
                    ),
                    (
                        "payload",
                        Value::map([
                            ("goal", Value::Text(goal.symbol.clone())),
                            ("child", Value::Text(child.id.clone())),
                            ("argument", Value::Text(argument.name.clone())),
                            ("resource", Value::Text(resource.id.clone())),
                            ("ownership", Value::Text(ownership.clone())),
                            ("mode", Value::Text(mode.to_owned())),
                        ]),
                    ),
                ]),
            )?;
            add_edge(edges, ownership, &id, edge_kind)?;
            by_resource
                .entry(resource.id.clone())
                .or_default()
                .push((id, handle.access.clone()));
        }
    }
    for (resource, uses) in by_resource {
        let ids = uses
            .iter()
            .map(|(id, _)| Value::Text(id.clone()))
            .collect::<Vec<_>>();
        let invariant = structural_id(
            "conflict-free",
            &Value::map([
                ("goal", Value::Text(goal.symbol.clone())),
                ("resource", Value::Text(resource.clone())),
                ("subjects", Value::Array(ids.clone())),
            ]),
        )?;
        insert_node(
            nodes,
            invariant.clone(),
            Value::map([
                ("id", Value::Text(invariant.clone())),
                ("kind", Value::Text("invariant".to_owned())),
                (
                    "payload",
                    Value::map([
                        ("goal", Value::Text(goal.symbol.clone())),
                        ("rule", Value::Text("bhcp/state.conflict-free@0".to_owned())),
                        ("subjects", Value::Array(ids)),
                    ]),
                ),
            ]),
        )?;
        for (id, _) in &uses {
            add_edge(edges, &invariant, id, "guards")?;
        }
        for left in 0..uses.len() {
            for right in left + 1..uses.len() {
                if uses[left].1 == "read" && uses[right].1 == "read" {
                    add_edge(edges, &uses[left].0, &uses[right].0, "compatible")?;
                }
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn add_retention_topology(
    goal: &GoalDefinition,
    goals: &[&GoalDefinition],
    capability: &GraphDocument,
    resources: &mut HashMap<(String, String), ResourceCoordinate>,
    nodes: &mut BTreeMap<String, Value>,
    edges: &mut BTreeMap<String, Value>,
    transitions: &mut BTreeMap<String, Value>,
) -> Result<()> {
    let network = goal.body.as_ref().expect("retention goal has a network");
    let resource_binding = goal
        .clauses
        .iter()
        .find_map(|clause| match &clause.kind {
            ClauseKind::Fact { binding, .. } if binding.name == "resource" => Some(binding),
            _ => None,
        })
        .ok_or_else(|| {
            invalid_construction("retention graph requires the exact resource binding")
        })?;
    let key = (goal.symbol.clone(), resource_binding.name.clone());
    if !resources.contains_key(&key) {
        let coordinate = resource_coordinate(
            goal,
            &resource_binding.id,
            &resource_binding.name,
            &resource_binding.value_type,
        )?;
        insert_resource(&coordinate, nodes, edges)?;
        resources.insert(key.clone(), coordinate);
    }
    let resource = resources.get(&key).unwrap();
    let role = |name: &str| {
        network
            .children
            .iter()
            .find(|child| child.tag == name)
            .ok_or_else(|| invalid_construction(format!("retention graph omits {name}")))
    };
    let read = role("state-read")?;
    let candidate = role("candidate")?;
    let cas = role("compare-and-swap")?;
    let candidate_type = goals
        .iter()
        .find(|item| item.id == candidate.goal)
        .map(|item| item.output.clone())
        .ok_or_else(|| invalid_construction("retention candidate goal is missing"))?;
    let mut role_ids = BTreeMap::new();
    for (name, child) in [
        ("state-read", read),
        ("candidate", candidate),
        ("compare-and-swap", cas),
    ] {
        let id = structural_id(
            "retention-role",
            &Value::map([
                ("goal", Value::Text(goal.symbol.clone())),
                ("child", Value::Text(child.id.clone())),
                ("role", Value::Text(name.to_owned())),
            ]),
        )?;
        insert_node(
            nodes,
            id.clone(),
            Value::map([
                ("id", Value::Text(id.clone())),
                ("kind", Value::Text("transition".to_owned())),
                (
                    "payload",
                    Value::map([
                        ("goal", Value::Text(goal.symbol.clone())),
                        ("child", Value::Text(child.id.clone())),
                        ("role", Value::Text(name.to_owned())),
                        ("resource", Value::Text(resource.id.clone())),
                    ]),
                ),
            ]),
        )?;
        role_ids.insert(name, id);
    }
    let cell = structural_id(
        "cell",
        &Value::map([
            ("goal", Value::Text(goal.symbol.clone())),
            ("resource", Value::Text(resource.id.clone())),
            ("type", candidate_type.to_value()),
        ]),
    )?;
    insert_node(
        nodes,
        cell.clone(),
        Value::map([
            ("id", Value::Text(cell.clone())),
            ("kind", Value::Text("cell".to_owned())),
            (
                "cell",
                Value::map([
                    ("key", Value::Text(resource.id.clone())),
                    ("type", candidate_type.to_value()),
                    ("state", Value::Array(vec![Value::Text("empty".to_owned())])),
                    ("atomic_version", Value::Integer(0)),
                ]),
            ),
            (
                "payload",
                Value::map([
                    ("goal", Value::Text(goal.symbol.clone())),
                    ("network", Value::Text(network.id.clone())),
                    ("resource", Value::Text(resource.id.clone())),
                    ("read", Value::Text(role_ids["state-read"].clone())),
                    ("candidate", Value::Text(role_ids["candidate"].clone())),
                    (
                        "compare_and_swap",
                        Value::Text(role_ids["compare-and-swap"].clone()),
                    ),
                ]),
            ),
        ]),
    )?;
    let capability_decision = capability
        .nodes()
        .iter()
        .find(|node| {
            node.kind == "decision"
                && node.value().get("goal") == Some(&Value::Text(goal.symbol.clone()))
                && node
                    .value()
                    .get("capability")
                    .and_then(|value| value.get("effect"))
                    .and_then(|value| value.get("id"))
                    == Some(&Value::Text(
                        "bhcp-effect/state.compare-and-swap@0".to_owned(),
                    ))
        })
        .ok_or_else(|| {
            invalid_construction(
                "mutable retention transition requires the exact compare-and-swap capability decision",
            )
        })?;
    let authority = structural_id(
        "authority",
        &Value::map([
            ("goal", Value::Text(goal.symbol.clone())),
            (
                "operation",
                Value::Text("bhcp/state.compare-and-swap@0".to_owned()),
            ),
            ("resource", Value::Text(resource.id.clone())),
        ]),
    )?;
    insert_node(
        nodes,
        authority.clone(),
        Value::map([
            ("id", Value::Text(authority.clone())),
            ("kind", Value::Text("authority".to_owned())),
            (
                "payload",
                Value::map([
                    ("goal", Value::Text(goal.symbol.clone())),
                    (
                        "operation",
                        Value::Text("bhcp/state.compare-and-swap@0".to_owned()),
                    ),
                    ("decision", Value::Text(capability_decision.id.clone())),
                    ("resource", Value::Text(resource.id.clone())),
                ]),
            ),
        ]),
    )?;
    let freshness = structural_id(
        "freshness",
        &Value::map([
            ("goal", Value::Text(goal.symbol.clone())),
            ("subject", Value::Text(resource.id.clone())),
            ("content", Value::Text(role_ids["state-read"].clone())),
        ]),
    )?;
    insert_node(
        nodes,
        freshness.clone(),
        Value::map([
            ("id", Value::Text(freshness.clone())),
            ("kind", Value::Text("freshness".to_owned())),
            (
                "payload",
                Value::map([
                    ("goal", Value::Text(goal.symbol.clone())),
                    ("subject", Value::Text(resource.id.clone())),
                    ("content", Value::Text(role_ids["state-read"].clone())),
                    ("provenance", Value::Text(goal.symbol.clone())),
                    (
                        "capture_time",
                        Value::Text("bhcp/state.capture-time@0".to_owned()),
                    ),
                    (
                        "rule",
                        Value::Text("bhcp/freshness.required-at-read@0".to_owned()),
                    ),
                    (
                        "stale",
                        Value::Text("bhcp.reason/stale-evidence@0".to_owned()),
                    ),
                    (
                        "fault",
                        Value::Text("bhcp.fault/policy-required-stale-evidence@0".to_owned()),
                    ),
                ]),
            ),
        ]),
    )?;
    let transition = structural_id(
        "atomic-transition",
        &Value::map([
            ("cell", Value::Text(cell.clone())),
            ("resource", Value::Text(resource.id.clone())),
            ("read", Value::Text(role_ids["state-read"].clone())),
            ("candidate", Value::Text(role_ids["candidate"].clone())),
            ("cas", Value::Text(role_ids["compare-and-swap"].clone())),
        ]),
    )?;
    let mut invariant_ids = goal
        .clauses
        .iter()
        .filter_map(|clause| match &clause.kind {
            ClauseKind::Contract {
                kind: "invariant", ..
            } => structural_id(
                "invariant",
                &Value::map([
                    ("goal", Value::Text(goal.symbol.clone())),
                    ("clause", Value::Text(clause.id.clone())),
                ]),
            )
            .ok(),
            _ => None,
        })
        .collect::<Vec<_>>();
    for rule in ["prior-version-match", "satisfied-candidate"] {
        let id = structural_id(
            rule,
            &Value::map([
                ("goal", Value::Text(goal.symbol.clone())),
                ("cell", Value::Text(cell.clone())),
                ("resource", Value::Text(resource.id.clone())),
                ("transition", Value::Text(transition.clone())),
            ]),
        )?;
        insert_node(
            nodes,
            id.clone(),
            Value::map([
                ("id", Value::Text(id.clone())),
                ("kind", Value::Text("invariant".to_owned())),
                (
                    "payload",
                    Value::map([
                        ("goal", Value::Text(goal.symbol.clone())),
                        ("rule", Value::Text(format!("bhcp/state.{rule}@0"))),
                        ("resource", Value::Text(resource.id.clone())),
                        ("transition", Value::Text(transition.clone())),
                        (
                            "subjects",
                            Value::Array(vec![
                                Value::Text(cell.clone()),
                                Value::Text(role_ids["state-read"].clone()),
                                Value::Text(role_ids["candidate"].clone()),
                            ]),
                        ),
                    ]),
                ),
            ]),
        )?;
        invariant_ids.push(id);
    }
    invariant_ids.sort();
    invariant_ids.dedup();
    add_edge(edges, &cell, &role_ids["state-read"], "reads")?;
    add_edge(
        edges,
        &role_ids["state-read"],
        &role_ids["candidate"],
        "prior-state",
    )?;
    add_edge(
        edges,
        &role_ids["candidate"],
        &role_ids["compare-and-swap"],
        "candidate",
    )?;
    add_edge(
        edges,
        &role_ids["candidate"],
        &role_ids["compare-and-swap"],
        "candidate-evidence",
    )?;
    add_edge(
        edges,
        &role_ids["state-read"],
        &role_ids["compare-and-swap"],
        "expected-version",
    )?;
    add_edge(
        edges,
        &authority,
        &role_ids["compare-and-swap"],
        "requires-authority",
    )?;
    add_edge(
        edges,
        &freshness,
        &role_ids["state-read"],
        "freshness-guard",
    )?;
    for invariant in &invariant_ids {
        add_edge(edges, invariant, &role_ids["compare-and-swap"], "guards")?;
    }
    transitions.insert(
        transition.clone(),
        Value::map([
            ("id", Value::Text(transition)),
            ("cell", Value::Text(cell)),
            ("from_version", Value::Integer(0)),
            ("to_version", Value::Integer(1)),
            ("read", Value::Text(role_ids["state-read"].clone())),
            ("candidate", Value::Text(role_ids["candidate"].clone())),
            (
                "compare_and_swap",
                Value::Text(role_ids["compare-and-swap"].clone()),
            ),
            ("authority", Value::Array(vec![Value::Text(authority)])),
            (
                "invariants",
                Value::Array(invariant_ids.into_iter().map(Value::Text).collect()),
            ),
            ("freshness", Value::Text(freshness)),
            (
                "conflict",
                Value::Text("bhcp.reason/compare-and-swap-conflict@0".to_owned()),
            ),
            ("atomic", Value::Bool(true)),
        ]),
    );
    Ok(())
}

fn parent_field_name(expression: &Expression) -> Option<&str> {
    let ExpressionForm::Call(symbol, parameters) = &expression.form else {
        return None;
    };
    if symbol != "bhcp/kernel.parent-field@0" {
        return None;
    }
    let [parameter] = parameters.as_slice() else {
        return None;
    };
    let ExpressionForm::Literal(Value::Text(name)) = &parameter.form else {
        return None;
    };
    Some(name)
}

fn argument_mode(mode: ArgumentMode) -> &'static str {
    match mode {
        ArgumentMode::Value => "value",
        ArgumentMode::Move => "move",
        ArgumentMode::Borrow => "borrow",
        ArgumentMode::Share => "share",
    }
}

fn add_edge(edges: &mut BTreeMap<String, Value>, from: &str, to: &str, kind: &str) -> Result<()> {
    if from == to {
        return Err(invalid_construction(
            "state graph edge cannot be self-referential",
        ));
    }
    let id = structural_id(
        "edge",
        &Value::map([
            ("from", Value::Text(from.to_owned())),
            ("to", Value::Text(to.to_owned())),
            ("kind", Value::Text(kind.to_owned())),
        ]),
    )?;
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

fn insert_node(nodes: &mut BTreeMap<String, Value>, id: String, node: Value) -> Result<()> {
    if nodes.insert(id, node).is_some() {
        return Err(invalid_construction("state node structural ID collision"));
    }
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
    id.push_str("s-");
    for byte in digest.digest.iter().take(32) {
        use std::fmt::Write as _;
        write!(id, "{byte:02x}").expect("writing to String cannot fail");
    }
    Ok(id)
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
