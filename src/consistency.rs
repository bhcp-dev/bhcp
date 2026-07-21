//! Exact reconstruction and correlation of the completed v0 analysis graphs.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::capability::{build_capability_graph, validate_capability_graph};
use crate::cbor::{decode_deterministic, encode_deterministic};
use crate::diagnostic::{Diagnostic, Result};
use crate::graph::{GraphDocument, GraphKind, GraphNode};
use crate::model::{ClauseKind, Effect};
use crate::obligation::{build_obligation_graph, validate_compilation, validate_obligation_graph};
use crate::pipeline::Compilation;
use crate::state::{STATE_FEATURE, build_state_graph, validate_state_graph};
use crate::value::Value;

pub const OBLIGATION_FEATURE: &str = "bhcp/feature.obligation-graph-builder@0";
pub const CAPABILITY_FEATURE: &str = "bhcp/feature.capability-graph-builder@0";

const INVALID_INPUT: &str = "BHCP7501";
const GRAPH_MISMATCH: &str = "BHCP7502";
const UNSUPPORTED_FEATURE: &str = "BHCP7503";
const CROSS_REFERENCE: &str = "BHCP7504";
const DECISION_MISMATCH: &str = "BHCP7505";
const OBLIGATION_MISMATCH: &str = "BHCP7506";
const STATE_MISMATCH: &str = "BHCP7507";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnalysisGraphs {
    pub obligation: GraphDocument,
    pub capability: GraphDocument,
    pub state: GraphDocument,
}

impl AnalysisGraphs {
    pub fn iter(&self) -> impl Iterator<Item = &GraphDocument> {
        [&self.obligation, &self.capability, &self.state].into_iter()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphConsistencyReport {
    features: BTreeSet<String>,
    checker_obligations: BTreeMap<String, BTreeSet<String>>,
}

impl GraphConsistencyReport {
    pub fn features(&self) -> &BTreeSet<String> {
        &self.features
    }

    pub fn checker_obligations(&self) -> &BTreeMap<String, BTreeSet<String>> {
        &self.checker_obligations
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct ResourceCoordinate {
    goal: String,
    binding: String,
    source_clause: String,
    name: String,
    value_type: Vec<u8>,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct EffectCoordinate {
    goal: String,
    effect: Vec<u8>,
}

pub fn build_analysis_graphs(compilation: &Compilation) -> Result<AnalysisGraphs> {
    validate_compilation(compilation).map_err(|error| invalid_input(error.message))?;
    let graphs = AnalysisGraphs {
        obligation: build_obligation_graph(compilation)
            .map_err(|error| invalid_input(format!("obligation graph: {}", error.message)))?,
        capability: build_capability_graph(compilation)
            .map_err(|error| invalid_input(format!("capability graph: {}", error.message)))?,
        state: build_state_graph(compilation)
            .map_err(|error| invalid_input(format!("state graph: {}", error.message)))?,
    };
    correlate(compilation, &graphs, &graphs).map(|_| graphs)
}

pub fn validate_analysis_graphs(
    compilation: &Compilation,
    graphs: &AnalysisGraphs,
) -> Result<GraphConsistencyReport> {
    validate_compilation(compilation).map_err(|error| invalid_input(error.message))?;

    // Every received set is compared with three graphs rebuilt from the same
    // exact compilation before any received graph can authorize another.
    let expected = AnalysisGraphs {
        obligation: build_obligation_graph(compilation)
            .map_err(|error| invalid_input(format!("obligation graph: {}", error.message)))?,
        capability: build_capability_graph(compilation)
            .map_err(|error| invalid_input(format!("capability graph: {}", error.message)))?,
        state: build_state_graph(compilation)
            .map_err(|error| invalid_input(format!("state graph: {}", error.message)))?,
    };
    let report = correlate(compilation, graphs, &expected)?;

    validate_obligation_graph(compilation, &graphs.obligation)
        .map_err(|error| mismatch(format!("obligation graph: {}", error.message)))?;
    validate_capability_graph(compilation, &graphs.capability)
        .map_err(|error| mismatch(format!("capability graph: {}", error.message)))?;
    validate_state_graph(compilation, &graphs.state)
        .map_err(|error| mismatch(format!("state graph: {}", error.message)))?;
    if graphs != &expected {
        return Err(mismatch(
            "analysis graph set does not match deterministic reconstruction",
        ));
    }
    Ok(report)
}

fn correlate(
    compilation: &Compilation,
    graphs: &AnalysisGraphs,
    expected: &AnalysisGraphs,
) -> Result<GraphConsistencyReport> {
    require_kind(&graphs.obligation, GraphKind::Obligation)?;
    require_kind(&graphs.capability, GraphKind::Capability)?;
    require_kind(&graphs.state, GraphKind::State)?;

    let expected_value = expected.obligation.to_value();
    let expected_semantic_ir = expected_value
        .get("semantic_ir")
        .expect("constructed graph has semantic_ir")
        .clone();
    let mut features = BTreeSet::new();
    for graph in graphs.iter() {
        let value = graph.to_value();
        if value.get("semantic_ir") != Some(&expected_semantic_ir) {
            return Err(mismatch(
                "every analysis graph must bind the exact reconstructed semantic IR",
            ));
        }
        let Some(Value::Array(emitted)) = value.get("features") else {
            return Err(unsupported_feature(
                "graph feature inventory must be an array",
            ));
        };
        let emitted = emitted
            .iter()
            .map(|feature| match feature {
                Value::Text(feature) => Ok(feature.clone()),
                _ => Err(unsupported_feature("graph feature must be text")),
            })
            .collect::<Result<BTreeSet<_>>>()?;
        let required = match graph.kind() {
            GraphKind::Obligation => OBLIGATION_FEATURE,
            GraphKind::Capability => CAPABILITY_FEATURE,
            GraphKind::State => STATE_FEATURE,
            _ => unreachable!("analysis set kinds were checked"),
        };
        if emitted != BTreeSet::from([required.to_owned()]) {
            return Err(unsupported_feature(format!(
                "{} must emit only {required}",
                graph.kind().as_str()
            )));
        }
        features.extend(emitted);
    }

    let checker_obligations =
        correlate_obligations(compilation, &graphs.obligation, &expected.obligation)?;
    let (decisions, capability_resources) =
        correlate_capabilities(compilation, &graphs.capability)?;
    correlate_state(
        compilation,
        &graphs.state,
        &decisions,
        &capability_resources,
    )?;
    Ok(GraphConsistencyReport {
        features,
        checker_obligations,
    })
}

fn require_kind(graph: &GraphDocument, expected: GraphKind) -> Result<()> {
    if graph.kind() != expected || graph.semantic_id().is_none() || graph.artifact_id().is_none() {
        return Err(invalid_input(format!(
            "analysis graph set requires a materialized {}",
            expected.as_str()
        )));
    }
    Ok(())
}

fn correlate_obligations(
    compilation: &Compilation,
    graph: &GraphDocument,
    expected: &GraphDocument,
) -> Result<BTreeMap<String, BTreeSet<String>>> {
    let meanings = graph
        .nodes()
        .iter()
        .map(obligation_meaning)
        .collect::<Result<BTreeSet<_>>>()?;
    let expected_meanings = expected
        .nodes()
        .iter()
        .map(obligation_meaning)
        .collect::<Result<BTreeSet<_>>>()?;
    if meanings != expected_meanings || meanings.len() != graph.nodes().len() {
        return Err(obligation_mismatch(
            "obligation nodes do not match structural source meaning and goal sets",
        ));
    }

    let goals = compilation
        .ir
        .goals
        .iter()
        .map(|goal| goal.symbol.as_str())
        .collect::<BTreeSet<_>>();
    let mut checker = BTreeMap::<String, BTreeSet<String>>::new();
    for node in graph.nodes() {
        let node_goals = texts(node.value(), "goals")?;
        if node_goals.is_empty() || node_goals.iter().any(|goal| !goals.contains(goal.as_str())) {
            return Err(obligation_mismatch(
                "obligation node has no exact semantic-IR goal coordinate",
            ));
        }
        let checker_claim = node.kind != "case"
            && (node.kind != "verification" || node.value().get("policy").is_some());
        if checker_claim {
            for goal in node_goals {
                checker.entry(goal).or_default().insert(node.id.clone());
            }
        }
    }
    Ok(checker)
}

fn obligation_meaning(node: &GraphNode) -> Result<Vec<u8>> {
    let mut fields = vec![
        ("kind".to_owned(), Value::Text(node.kind.clone())),
        (
            "goals".to_owned(),
            Value::Array(
                texts(node.value(), "goals")?
                    .into_iter()
                    .map(Value::Text)
                    .collect(),
            ),
        ),
    ];
    for field in ["clause", "recursion"] {
        if let Some(value) = node.value().get(field) {
            fields.push((field.to_owned(), value.clone()));
        }
    }
    if let Some(policy) = node.value().get("policy") {
        let mut policy = policy.clone();
        remove_field(&mut policy, "effective_rule");
        remove_field(&mut policy, "sources");
        fields.push(("policy".to_owned(), policy));
    }
    encode_deterministic(&Value::owned_map(fields))
}

fn correlate_capabilities(
    compilation: &Compilation,
    graph: &GraphDocument,
) -> Result<(
    BTreeMap<String, EffectCoordinate>,
    BTreeMap<String, ResourceCoordinate>,
)> {
    let bindings = binding_coordinates(compilation)?;
    let mut resources = BTreeMap::<ResourceCoordinate, String>::new();
    let mut resources_by_id = BTreeMap::<String, ResourceCoordinate>::new();
    for node in graph.nodes().iter().filter(|node| node.kind == "resource") {
        let coordinate = capability_resource(node, &bindings)?;
        if resources
            .insert(coordinate.clone(), node.id.clone())
            .is_some()
            || resources_by_id
                .insert(node.id.clone(), coordinate)
                .is_some()
        {
            return Err(decision_mismatch(
                "capability resource coordinate is not unique",
            ));
        }
    }

    let expected = compilation
        .ir
        .goals
        .iter()
        .flat_map(|goal| {
            goal.effects
                .effects
                .iter()
                .map(|effect| expected_effect(goal.symbol.as_str(), effect, &bindings, &resources))
        })
        .collect::<Result<BTreeSet<_>>>()?;
    let mut requests = BTreeMap::<EffectCoordinate, String>::new();
    let mut decisions_by_effect = BTreeMap::<EffectCoordinate, String>::new();
    let mut decisions_by_id = BTreeMap::<String, EffectCoordinate>::new();
    for node in graph.nodes() {
        match node.kind.as_str() {
            "request" => {
                let request = node
                    .value()
                    .get("request")
                    .ok_or_else(|| decision_mismatch("capability request has no payload"))?;
                let coordinate = effect_coordinate(request, "effect")?;
                if requests.insert(coordinate, node.id.clone()).is_some() {
                    return Err(decision_mismatch(
                        "execution-eligible effect has more than one request",
                    ));
                }
            }
            "decision" => {
                let goal = text(node.value(), "goal")?;
                let capability = node
                    .value()
                    .get("capability")
                    .ok_or_else(|| decision_mismatch("capability decision has no payload"))?;
                if text(capability, "decision")? != "allow" {
                    return Err(decision_mismatch(
                        "execution-eligible effect does not have an allow decision",
                    ));
                }
                let effect = capability
                    .get("effect")
                    .ok_or_else(|| decision_mismatch("capability decision has no effect"))?;
                let coordinate = EffectCoordinate {
                    goal,
                    effect: encode_deterministic(effect)?,
                };
                if decisions_by_effect
                    .insert(coordinate.clone(), node.id.clone())
                    .is_some()
                    || decisions_by_id
                        .insert(node.id.clone(), coordinate)
                        .is_some()
                {
                    return Err(decision_mismatch(
                        "execution-eligible effect has more than one allow decision",
                    ));
                }
            }
            _ => {}
        }
    }
    if requests.keys().cloned().collect::<BTreeSet<_>>() != expected
        || decisions_by_effect.keys().cloned().collect::<BTreeSet<_>>() != expected
    {
        return Err(decision_mismatch(
            "requests and allow decisions are not a bijection over execution-eligible effects",
        ));
    }
    for coordinate in &expected {
        let request = &requests[coordinate];
        let decision = &decisions_by_effect[coordinate];
        if !graph
            .edges()
            .iter()
            .any(|edge| edge.kind == "requests" && edge.from == *request && edge.to == *decision)
        {
            return Err(decision_mismatch(
                "capability request does not join its exact allow decision",
            ));
        }
        if let Some(Value::Text(resource)) =
            decode_deterministic(&coordinate.effect)?.get("resource")
            && !resources_by_id.contains_key(resource)
        {
            return Err(cross_reference(
                "capability effect resource does not name a capability resource node",
            ));
        }
    }
    Ok((decisions_by_id, resources_by_id))
}

fn correlate_state(
    compilation: &Compilation,
    graph: &GraphDocument,
    decisions: &BTreeMap<String, EffectCoordinate>,
    capability_resources: &BTreeMap<String, ResourceCoordinate>,
) -> Result<()> {
    let bindings = binding_coordinates(compilation)?;
    let nodes = graph
        .nodes()
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect::<HashMap<_, _>>();
    let mut state_resources = BTreeMap::<String, ResourceCoordinate>::new();
    for node in graph.nodes().iter().filter(|node| node.kind == "resource") {
        let coordinate = state_resource(node, &bindings)?;
        if state_resources
            .insert(node.id.clone(), coordinate)
            .is_some()
        {
            return Err(state_mismatch("state resource ID is not unique"));
        }
    }
    let mut authorities = BTreeSet::new();
    for node in graph.nodes().iter().filter(|node| node.kind == "authority") {
        correlate_authority(node, decisions, capability_resources, &state_resources)?;
        authorities.insert(node.id.clone());
    }

    let value = graph.to_value();
    let Some(Value::Array(transitions)) = value.get("transitions") else {
        return Err(state_mismatch("state transitions must be an array"));
    };
    let mut joined_authorities = BTreeSet::new();
    for transition in transitions {
        let cell = text(transition, "cell")?;
        require_state_kind(&nodes, &cell, "cell", "transition cell")?;
        let read = text(transition, "read")?;
        let candidate = text(transition, "candidate")?;
        let cas = text(transition, "compare_and_swap")?;
        require_state_role(&nodes, &read, "state-read")?;
        require_state_role(&nodes, &candidate, "candidate")?;
        require_state_role(&nodes, &cas, "compare-and-swap")?;
        if transition.get("from_version") != Some(&Value::Integer(0))
            || transition.get("to_version") != Some(&Value::Integer(1))
            || transition.get("atomic") != Some(&Value::Bool(true))
        {
            return Err(state_mismatch(
                "state transition must retain exact pre/post version and atomicity",
            ));
        }
        let authority = texts(transition, "authority")?;
        if authority.is_empty() {
            return Err(state_mismatch(
                "mutable state transition has no capability authority",
            ));
        }
        for id in authority {
            require_state_kind(&nodes, &id, "authority", "transition authority")?;
            joined_authorities.insert(id.clone());
            require_edge(graph, &id, &cas, "requires-authority")?;
        }
        for invariant in texts(transition, "invariants")? {
            require_state_kind(&nodes, &invariant, "invariant", "transition invariant")?;
            require_edge(graph, &invariant, &cas, "guards")?;
        }
        let freshness = text(transition, "freshness")?;
        require_state_kind(&nodes, &freshness, "freshness", "transition freshness")?;
        require_edge(graph, &cell, &read, "reads")?;
        require_edge(graph, &read, &candidate, "prior-state")?;
        require_edge(graph, &candidate, &cas, "candidate")?;
        require_edge(graph, &candidate, &cas, "candidate-evidence")?;
        require_edge(graph, &read, &cas, "expected-version")?;
        require_edge(graph, &freshness, &read, "freshness-guard")?;
    }
    if joined_authorities != authorities {
        return Err(state_mismatch(
            "state authority nodes do not bijectively guard atomic transitions",
        ));
    }
    Ok(())
}

fn correlate_authority(
    node: &GraphNode,
    decisions: &BTreeMap<String, EffectCoordinate>,
    capability_resources: &BTreeMap<String, ResourceCoordinate>,
    state_resources: &BTreeMap<String, ResourceCoordinate>,
) -> Result<()> {
    let payload = node
        .value()
        .get("payload")
        .ok_or_else(|| state_mismatch("state authority has no payload"))?;
    let decision = text(payload, "decision")?;
    let Some(coordinate) = decisions.get(&decision) else {
        return Err(cross_reference(
            "state authority decision does not name a capability decision node",
        ));
    };
    let effect = payload
        .get("effect")
        .ok_or_else(|| state_mismatch("state authority has no exact effect"))?;
    if encode_deterministic(effect)? != coordinate.effect
        || effect.get("id") != payload.get("operation")
        || payload.get("goal") != Some(&Value::Text(coordinate.goal.clone()))
    {
        return Err(state_mismatch(
            "state authority does not match the exact capability decision effect",
        ));
    }
    let state_resource = text(payload, "resource")?;
    let Some(state_coordinate) = state_resources.get(&state_resource) else {
        return Err(cross_reference(
            "state authority resource does not name a state resource node",
        ));
    };
    let Some(Value::Text(capability_resource)) = effect.get("resource") else {
        return Ok(());
    };
    let Some(capability_coordinate) = capability_resources.get(capability_resource) else {
        return Err(cross_reference(
            "state authority effect does not name a capability resource node",
        ));
    };
    if state_coordinate != capability_coordinate {
        return Err(state_mismatch(
            "state and capability resources differ at their full typed source coordinate",
        ));
    }
    Ok(())
}

fn binding_coordinates(compilation: &Compilation) -> Result<BTreeMap<String, ResourceCoordinate>> {
    let mut output = BTreeMap::new();
    for goal in &compilation.ir.goals {
        for clause in &goal.clauses {
            let ClauseKind::Fact { binding, .. } = &clause.kind else {
                continue;
            };
            let coordinate = ResourceCoordinate {
                goal: goal.symbol.clone(),
                binding: binding.id.clone(),
                source_clause: clause.id.clone(),
                name: binding.name.clone(),
                value_type: encode_deterministic(&binding.value_type.to_value())?,
            };
            if output.insert(binding.id.clone(), coordinate).is_some() {
                return Err(invalid_input("semantic IR fact binding is not unique"));
            }
        }
    }
    Ok(output)
}

fn capability_resource(
    node: &GraphNode,
    bindings: &BTreeMap<String, ResourceCoordinate>,
) -> Result<ResourceCoordinate> {
    let value = node
        .value()
        .get("resource")
        .ok_or_else(|| decision_mismatch("capability resource has no typed payload"))?;
    let goal = text(value, "goal")?;
    let name = text(value, "name")?;
    let value_type = encode_deterministic(
        value
            .get("type")
            .ok_or_else(|| decision_mismatch("capability resource has no type"))?,
    )?;
    let source_clauses = texts(node.value(), "source_clauses")?;
    let matches = bindings
        .values()
        .filter(|binding| {
            binding.goal == goal
                && binding.name == name
                && binding.value_type == value_type
                && source_clauses == BTreeSet::from([binding.source_clause.clone()])
        })
        .cloned()
        .collect::<Vec<_>>();
    let [coordinate] = matches.as_slice() else {
        return Err(decision_mismatch(
            "capability resource does not resolve to one full typed source coordinate",
        ));
    };
    Ok(coordinate.clone())
}

fn state_resource(
    node: &GraphNode,
    bindings: &BTreeMap<String, ResourceCoordinate>,
) -> Result<ResourceCoordinate> {
    let payload = node
        .value()
        .get("payload")
        .ok_or_else(|| state_mismatch("state resource has no typed payload"))?;
    let binding = text(payload, "binding")?;
    let Some(coordinate) = bindings.get(&binding) else {
        return Err(state_mismatch(
            "state resource binding is not retained by semantic IR",
        ));
    };
    let value_type = encode_deterministic(
        payload
            .get("type")
            .ok_or_else(|| state_mismatch("state resource has no type"))?,
    )?;
    if payload.get("goal") != Some(&Value::Text(coordinate.goal.clone()))
        || payload.get("name") != Some(&Value::Text(coordinate.name.clone()))
        || value_type != coordinate.value_type
    {
        return Err(state_mismatch(
            "state resource differs from its full typed source coordinate",
        ));
    }
    Ok(coordinate.clone())
}

fn expected_effect(
    goal: &str,
    effect: &Effect,
    bindings: &BTreeMap<String, ResourceCoordinate>,
    resources: &BTreeMap<ResourceCoordinate, String>,
) -> Result<EffectCoordinate> {
    let mut fields = vec![("id".to_owned(), Value::Text(effect.id.clone()))];
    if let Some(binding) = &effect.resource {
        let coordinate = bindings
            .get(binding)
            .ok_or_else(|| decision_mismatch("effect resource binding is missing"))?;
        let resource = resources.get(coordinate).ok_or_else(|| {
            decision_mismatch("effect has no capability resource at its typed coordinate")
        })?;
        fields.push(("resource".to_owned(), Value::Text(resource.clone())));
    }
    if !effect.parameters.is_empty() {
        fields.push((
            "parameters".to_owned(),
            Value::Array(effect.parameters.clone()),
        ));
    }
    Ok(EffectCoordinate {
        goal: goal.to_owned(),
        effect: encode_deterministic(&Value::owned_map(fields))?,
    })
}

fn effect_coordinate(value: &Value, field: &str) -> Result<EffectCoordinate> {
    Ok(EffectCoordinate {
        goal: text(value, "goal")?,
        effect: encode_deterministic(
            value
                .get(field)
                .ok_or_else(|| decision_mismatch("capability coordinate has no effect"))?,
        )?,
    })
}

fn require_state_kind(
    nodes: &HashMap<&str, &GraphNode>,
    id: &str,
    kind: &str,
    context: &str,
) -> Result<()> {
    let Some(node) = nodes.get(id) else {
        return Err(cross_reference(format!("{context} is dangling")));
    };
    if node.kind != kind {
        return Err(cross_reference(format!(
            "{context} names {} instead of {kind}",
            node.kind
        )));
    }
    Ok(())
}

fn require_state_role(nodes: &HashMap<&str, &GraphNode>, id: &str, role: &str) -> Result<()> {
    require_state_kind(nodes, id, "transition", role)?;
    if nodes[id]
        .value()
        .get("payload")
        .and_then(|value| value.get("role"))
        != Some(&Value::Text(role.to_owned()))
    {
        return Err(state_mismatch(format!(
            "state transition role {id} is not {role}"
        )));
    }
    Ok(())
}

fn require_edge(graph: &GraphDocument, from: &str, to: &str, kind: &str) -> Result<()> {
    if !graph
        .edges()
        .iter()
        .any(|edge| edge.from == from && edge.to == to && edge.kind == kind)
    {
        return Err(state_mismatch(format!(
            "state topology omits {kind} edge {from} -> {to}"
        )));
    }
    Ok(())
}

fn text(value: &Value, field: &str) -> Result<String> {
    match value.get(field) {
        Some(Value::Text(value)) => Ok(value.clone()),
        _ => Err(invalid_input(format!("{field} must be text"))),
    }
}

fn texts(value: &Value, field: &str) -> Result<BTreeSet<String>> {
    match value.get(field) {
        Some(Value::Array(values)) => values
            .iter()
            .map(|value| match value {
                Value::Text(value) => Ok(value.clone()),
                _ => Err(invalid_input(format!("{field} member must be text"))),
            })
            .collect(),
        None => Ok(BTreeSet::new()),
        _ => Err(invalid_input(format!("{field} must be an array"))),
    }
}

fn remove_field(value: &mut Value, field: &str) {
    let Value::Map(entries) = value else {
        return;
    };
    entries.retain(|(name, _)| name != field);
}

fn invalid_input(message: impl Into<String>) -> Diagnostic {
    Diagnostic::plain(INVALID_INPUT, message)
}

fn mismatch(message: impl Into<String>) -> Diagnostic {
    Diagnostic::plain(GRAPH_MISMATCH, message)
}

fn unsupported_feature(message: impl Into<String>) -> Diagnostic {
    Diagnostic::plain(UNSUPPORTED_FEATURE, message)
}

fn cross_reference(message: impl Into<String>) -> Diagnostic {
    Diagnostic::plain(CROSS_REFERENCE, message)
}

fn decision_mismatch(message: impl Into<String>) -> Diagnostic {
    Diagnostic::plain(DECISION_MISMATCH, message)
}

fn obligation_mismatch(message: impl Into<String>) -> Diagnostic {
    Diagnostic::plain(OBLIGATION_MISMATCH, message)
}

fn state_mismatch(message: impl Into<String>) -> Diagnostic {
    Diagnostic::plain(STATE_MISMATCH, message)
}
