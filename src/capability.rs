//! Deterministic capability decisions derived from validated effects and authority.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::cbor::encode_deterministic;
use crate::diagnostic::{Diagnostic, Result};
use crate::graph::{GraphDocument, GraphKind};
use crate::hash::{HashAlgorithm, hash_value};
use crate::model::{
    BhcpType, ClauseKind, ContentReference, Effect, Expression, ExpressionForm, GoalDefinition,
};
use crate::obligation::validate_compilation;
use crate::pipeline::Compilation;
use crate::policy::{
    EffectivePolicyDocument, PolicyCategory, PolicyLayer, PolicyScope, RuleProvenance,
};
use crate::value::Value;

const INVALID_INPUT: &str = "BHCP7201";
const INVALID_CONSTRUCTION: &str = "BHCP7202";
const GRAPH_MISMATCH: &str = "BHCP7203";
const CAPABILITY_FEATURE: &str = "bhcp/feature.capability-graph-builder@0";

#[derive(Clone)]
struct ResourceCoordinate {
    id: String,
    source_clause: String,
    goal: String,
    name: String,
    value_type: BhcpType,
}

pub fn build_capability_graph(compilation: &Compilation) -> Result<GraphDocument> {
    let algorithm =
        validate_compilation(compilation).map_err(|error| invalid_input(error.message))?;
    let resources = resource_coordinates(&compilation.ir.goals)?;
    let used_resources = used_resource_bindings(&compilation.ir.goals);
    let mut nodes = BTreeMap::<String, Value>::new();
    let mut edges = BTreeMap::<String, Value>::new();
    add_resource_nodes(&resources, &used_resources, &mut nodes)?;

    let mut goals = compilation.ir.goals.iter().collect::<Vec<_>>();
    goals.sort_by(|left, right| left.symbol.cmp(&right.symbol));
    let mut requests = BTreeMap::<(String, Vec<u8>), String>::new();
    let mut decisions = BTreeMap::<(String, Vec<u8>), String>::new();

    for goal in &goals {
        add_goal_nodes(
            goal,
            compilation.effective_policy.as_ref(),
            &resources,
            &mut requests,
            &mut decisions,
            &mut nodes,
            &mut edges,
        )?;
    }
    add_propagation_edges(&goals, &resources, &requests, &mut edges)?;
    if let Some(policy) = &compilation.effective_policy {
        add_waiver_nodes(policy, &decisions, &mut nodes, &mut edges)?;
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
            Value::Array(vec![Value::Text(CAPABILITY_FEATURE.to_owned())]),
        ),
        ("kind", Value::Text("capability-graph".to_owned())),
        ("semantic_ir", semantic_ir.to_value()),
        ("nodes", Value::Array(nodes.into_values().collect())),
        ("edges", Value::Array(edges.into_values().collect())),
    ]);
    let mut graph = GraphDocument::from_value(&value)?;
    graph.materialize_identities(algorithm)?;
    Ok(graph)
}

pub fn validate_capability_graph(compilation: &Compilation, graph: &GraphDocument) -> Result<()> {
    if graph.kind() != GraphKind::Capability
        || graph.semantic_id().is_none()
        || graph.artifact_id().is_none()
    {
        return Err(mismatch(
            "planning requires a materialized capability graph",
        ));
    }
    if graph.to_value() != build_capability_graph(compilation)?.to_value() {
        return Err(mismatch(
            "capability graph does not match deterministic construction from semantic IR",
        ));
    }
    Ok(())
}

fn resource_coordinates(goals: &[GoalDefinition]) -> Result<HashMap<String, ResourceCoordinate>> {
    let mut resources = HashMap::new();
    for goal in goals {
        for clause in &goal.clauses {
            let ClauseKind::Fact { binding, .. } = &clause.kind else {
                continue;
            };
            if resource_symbol(&binding.value_type).is_none() {
                continue;
            }
            let key = Value::map([
                ("goal", Value::Text(goal.symbol.clone())),
                ("name", Value::Text(binding.name.clone())),
                ("type", binding.value_type.to_value()),
            ]);
            let coordinate = ResourceCoordinate {
                id: structural_id("resource", &key)?,
                source_clause: clause.id.clone(),
                goal: goal.symbol.clone(),
                name: binding.name.clone(),
                value_type: binding.value_type.clone(),
            };
            resources.insert(binding.id.clone(), coordinate);
        }
    }
    Ok(resources)
}

fn add_resource_nodes(
    resources: &HashMap<String, ResourceCoordinate>,
    used_resources: &BTreeSet<String>,
    nodes: &mut BTreeMap<String, Value>,
) -> Result<()> {
    for (source_id, resource) in resources {
        if !used_resources.contains(source_id) {
            continue;
        }
        let node = Value::map([
            ("id", Value::Text(resource.id.clone())),
            ("kind", Value::Text("resource".to_owned())),
            (
                "resource",
                Value::map([
                    ("goal", Value::Text(resource.goal.clone())),
                    ("name", Value::Text(resource.name.clone())),
                    ("type", resource.value_type.to_value()),
                ]),
            ),
            (
                "source_clauses",
                Value::Array(vec![Value::Text(resource.source_clause.clone())]),
            ),
        ]);
        if nodes.insert(resource.id.clone(), node).is_some() {
            return Err(invalid_construction(
                "resource structural ID collides with an existing resource",
            ));
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn add_goal_nodes(
    goal: &GoalDefinition,
    policy: Option<&EffectivePolicyDocument>,
    resources: &HashMap<String, ResourceCoordinate>,
    requests: &mut BTreeMap<(String, Vec<u8>), String>,
    decisions: &mut BTreeMap<(String, Vec<u8>), String>,
    nodes: &mut BTreeMap<String, Value>,
    edges: &mut BTreeMap<String, Value>,
) -> Result<()> {
    let authored = authored_nodes(goal, resources, nodes)?;
    let policy_context = policy_nodes(goal, policy, nodes)?;
    for effect in &goal.effects.effects {
        let graph_effect = graph_effect(effect, resources)?;
        let effect_bytes = encode_deterministic(&graph_effect)?;
        let request_key = Value::map([
            ("goal", Value::Text(goal.symbol.clone())),
            ("effect", graph_effect.clone()),
        ]);
        let request_id = structural_id("request", &request_key)?;
        let mut request_entries = vec![
            ("id".to_owned(), Value::Text(request_id.clone())),
            ("kind".to_owned(), Value::Text("request".to_owned())),
            (
                "request".to_owned(),
                Value::map([
                    ("goal", Value::Text(goal.symbol.clone())),
                    ("effect", graph_effect.clone()),
                ]),
            ),
        ];
        if let Some(resource) = graph_effect.get("resource") {
            request_entries.push(("resources".to_owned(), Value::Array(vec![resource.clone()])));
        }
        nodes.insert(request_id.clone(), Value::owned_map(request_entries));
        requests.insert(
            (goal.symbol.clone(), effect_bytes.clone()),
            request_id.clone(),
        );

        let mut grant_sources = authored
            .grants
            .iter()
            .filter(|grant| effect_covers(&grant.effect, effect))
            .map(|grant| grant.id.clone())
            .collect::<BTreeSet<_>>();
        if grant_sources.is_empty() && !authored.has_ceiling {
            let id = add_propagated_grant(goal, &graph_effect, nodes)?;
            grant_sources.insert(id);
        }
        if grant_sources.is_empty() {
            return Err(invalid_input(format!(
                "goal {} has an execution effect outside its retained authored ceiling",
                goal.symbol
            )));
        }

        if policy.is_some() {
            let matching = policy_context
                .grants
                .iter()
                .filter(|grant| {
                    grant.effect.id == effect.id
                        && policy_request_within_scope(effect, &grant.scope, resources)
                })
                .map(|grant| grant.id.clone())
                .collect::<Vec<_>>();
            if matching.is_empty() {
                return Err(invalid_input(format!(
                    "goal {} has an execution effect without retained policy authority",
                    goal.symbol
                )));
            }
            grant_sources.extend(matching);
        }

        let decision_key = Value::map([
            ("goal", Value::Text(goal.symbol.clone())),
            ("effect", graph_effect.clone()),
        ]);
        let decision_id = structural_id("decision", &decision_key)?;
        let mut decision_entries = vec![
            ("id".to_owned(), Value::Text(decision_id.clone())),
            ("kind".to_owned(), Value::Text("decision".to_owned())),
            ("goal".to_owned(), Value::Text(goal.symbol.clone())),
            (
                "capability".to_owned(),
                capability_value(
                    graph_effect,
                    "allow",
                    grant_sources.iter().cloned().collect(),
                ),
            ),
        ];
        if let Some(gap) = effect_gap(effect) {
            decision_entries.push((
                "gap".to_owned(),
                Value::map([
                    ("kind", Value::Text(gap.to_owned())),
                    ("required", Value::Bool(true)),
                ]),
            ));
        }
        nodes.insert(decision_id.clone(), Value::owned_map(decision_entries));
        decisions.insert((goal.symbol.clone(), effect_bytes), decision_id.clone());
        add_edge(edges, "requests", &request_id, &decision_id)?;
        for source in grant_sources {
            add_edge(edges, "authorizes", &source, &decision_id)?;
        }
        for denial in authored.denials.iter().chain(&policy_context.denials) {
            add_edge(edges, "constrains", &denial.id, &decision_id)?;
        }
        if let Some(Value::Text(resource)) = request_key
            .get("effect")
            .and_then(|value| value.get("resource"))
        {
            add_edge(edges, "scopes", resource, &request_id)?;
        }
    }
    Ok(())
}

#[derive(Default)]
struct AuthorityNodes {
    grants: Vec<AuthorityNode>,
    denials: Vec<AuthorityNode>,
    has_ceiling: bool,
}

struct AuthorityNode {
    id: String,
    effect: Effect,
    scope: Option<PolicyScope>,
}

fn authored_nodes(
    goal: &GoalDefinition,
    resources: &HashMap<String, ResourceCoordinate>,
    nodes: &mut BTreeMap<String, Value>,
) -> Result<AuthorityNodes> {
    let mut output = AuthorityNodes::default();
    let mut clauses = BTreeMap::<(String, Vec<u8>), BTreeSet<String>>::new();
    let mut effects = BTreeMap::<(String, Vec<u8>), Effect>::new();
    for clause in &goal.clauses {
        let ClauseKind::Authority {
            kind,
            effects: values,
        } = &clause.kind
        else {
            continue;
        };
        if *kind == "allows" {
            output.has_ceiling = true;
        }
        for effect in values {
            let graph_effect = graph_effect(effect, resources)?;
            let bytes = encode_deterministic(&graph_effect)?;
            let key = ((*kind).to_owned(), bytes.clone());
            clauses
                .entry(key.clone())
                .or_default()
                .insert(clause.id.clone());
            effects.insert(key, effect.clone());
        }
    }
    for ((kind, bytes), source_clauses) in clauses {
        let effect = effects.remove(&(kind.clone(), bytes)).unwrap();
        let graph_effect = graph_effect(&effect, resources)?;
        let node_kind = if kind == "allows" { "grant" } else { "denial" };
        let key = Value::map([
            ("goal", Value::Text(goal.symbol.clone())),
            ("kind", Value::Text(node_kind.to_owned())),
            ("effect", graph_effect.clone()),
        ]);
        let id = structural_id("authored-authority", &key)?;
        let node = Value::map([
            ("id", Value::Text(id.clone())),
            ("kind", Value::Text(node_kind.to_owned())),
            ("goal", Value::Text(goal.symbol.clone())),
            (
                "capability",
                capability_value(
                    graph_effect,
                    if kind == "allows" { "allow" } else { "deny" },
                    vec![stable_source_id("authored", &key)?],
                ),
            ),
            (
                "source_clauses",
                Value::Array(source_clauses.into_iter().map(Value::Text).collect()),
            ),
        ]);
        nodes.insert(id.clone(), node);
        let authority = AuthorityNode {
            id,
            effect,
            scope: None,
        };
        if kind == "allows" {
            output.grants.push(authority);
        } else {
            output.denials.push(authority);
        }
    }
    Ok(output)
}

fn add_propagated_grant(
    goal: &GoalDefinition,
    graph_effect: &Value,
    nodes: &mut BTreeMap<String, Value>,
) -> Result<String> {
    let key = Value::map([
        ("goal", Value::Text(goal.symbol.clone())),
        ("effect", graph_effect.clone()),
        ("basis", Value::Text("propagated".to_owned())),
    ]);
    let id = structural_id("propagated-authority", &key)?;
    let source = stable_source_id("propagated", &key)?;
    nodes.entry(id.clone()).or_insert_with(|| {
        Value::map([
            ("id", Value::Text(id.clone())),
            ("kind", Value::Text("grant".to_owned())),
            ("goal", Value::Text(goal.symbol.clone())),
            (
                "capability",
                capability_value(graph_effect.clone(), "allow", vec![source]),
            ),
            (
                "payload",
                Value::map([(
                    "basis",
                    Value::Text("propagated-effect-analysis".to_owned()),
                )]),
            ),
        ])
    });
    Ok(id)
}

fn policy_nodes(
    goal: &GoalDefinition,
    policy: Option<&EffectivePolicyDocument>,
    nodes: &mut BTreeMap<String, Value>,
) -> Result<AuthorityNodes> {
    let Some(policy) = policy else {
        return Ok(AuthorityNodes::default());
    };
    let decision = goal
        .policy_decision
        .as_ref()
        .ok_or_else(|| invalid_input("retained policy requires a goal decision"))?;
    let mut output = AuthorityNodes::default();
    for (category, indices, rules, node_kind, decision_name) in [
        (
            PolicyCategory::Capability,
            &decision.capabilities,
            &policy.effective.capabilities,
            "grant",
            "allow",
        ),
        (
            PolicyCategory::Prohibition,
            &decision.prohibitions,
            &policy.effective.prohibitions,
            "denial",
            "deny",
        ),
    ] {
        for index in indices {
            let rule = rules
                .get(*index)
                .ok_or_else(|| invalid_input("policy authority index is out of range"))?;
            let value = capability_policy_value(&rule.value);
            let key = Value::map([
                ("goal", Value::Text(goal.symbol.clone())),
                ("category", Value::Text(category.as_str().to_owned())),
                ("value", value.clone()),
            ]);
            let id = structural_id("policy-authority", &key)?;
            let sources = policy_sources(policy, category, *index)?;
            let source_refs = sources
                .iter()
                .map(|source| stable_source_id("policy", source))
                .collect::<Result<Vec<_>>>()?;
            let effect = Effect {
                id: rule.value.effect.clone(),
                resource: None,
                parameters: Vec::new(),
            };
            let node = Value::map([
                ("id", Value::Text(id.clone())),
                ("kind", Value::Text(node_kind.to_owned())),
                ("goal", Value::Text(goal.symbol.clone())),
                (
                    "capability",
                    capability_value(effect.to_value(), decision_name, source_refs),
                ),
                (
                    "policy",
                    Value::map([
                        ("category", Value::Text(category.as_str().to_owned())),
                        ("effective_rule", Value::Integer(*index as i128)),
                        ("value", value),
                        ("sources", Value::Array(sources)),
                    ]),
                ),
            ]);
            nodes.insert(id.clone(), node);
            let authority = AuthorityNode {
                id,
                effect,
                scope: rule.value.scope.clone(),
            };
            if category == PolicyCategory::Capability {
                output.grants.push(authority);
            } else {
                output.denials.push(authority);
            }
        }
    }
    Ok(output)
}

fn add_waiver_nodes(
    policy: &EffectivePolicyDocument,
    decisions: &BTreeMap<(String, Vec<u8>), String>,
    nodes: &mut BTreeMap<String, Value>,
    edges: &mut BTreeMap<String, Value>,
) -> Result<()> {
    let Some(waivers) = &policy.waivers else {
        return Ok(());
    };
    for waiver in waivers {
        let targets = waiver
            .targets
            .iter()
            .map(|target| {
                Value::Array(vec![
                    Value::Text(target.policy.clone()),
                    Value::Text(target.rule.clone()),
                ])
            })
            .collect::<Vec<_>>();
        let detail = Value::map([
            ("waiver", waiver.waiver.to_value()),
            ("targets", Value::Array(targets)),
            (
                "decision_time",
                Value::Tag(0, Box::new(Value::Text(waiver.decision_time.clone()))),
            ),
        ]);
        let id = structural_id("waiver", &detail)?;
        nodes.insert(
            id.clone(),
            Value::map([
                ("id", Value::Text(id.clone())),
                ("kind", Value::Text("waiver".to_owned())),
                ("waiver", detail),
            ]),
        );
        for decision in decisions.values() {
            add_edge(edges, "waiver-context", &id, decision)?;
        }
    }
    Ok(())
}

fn add_propagation_edges(
    goals: &[&GoalDefinition],
    resources: &HashMap<String, ResourceCoordinate>,
    requests: &BTreeMap<(String, Vec<u8>), String>,
    edges: &mut BTreeMap<String, Value>,
) -> Result<()> {
    let by_id = goals
        .iter()
        .map(|goal| (goal.id.as_str(), *goal))
        .collect::<HashMap<_, _>>();
    for parent in goals {
        let Some(body) = &parent.body else {
            continue;
        };
        for child in &body.children {
            let child_goal = by_id
                .get(child.goal.as_str())
                .ok_or_else(|| invalid_input("network child goal is missing"))?;
            for child_effect in &child_goal.effects.effects {
                let child_key = (
                    child_goal.symbol.clone(),
                    encode_deterministic(&graph_effect(child_effect, resources)?)?,
                );
                let Some(child_request) = requests.get(&child_key) else {
                    continue;
                };
                let projected =
                    project_resource(child_effect, child_goal, parent, &child.arguments);
                let parent_key = (
                    parent.symbol.clone(),
                    encode_deterministic(&graph_effect(&projected, resources)?)?,
                );
                if let Some(parent_request) = requests.get(&parent_key) {
                    add_edge(edges, "propagates-to", child_request, parent_request)?;
                }
            }
        }
    }
    Ok(())
}

fn used_resource_bindings(goals: &[GoalDefinition]) -> BTreeSet<String> {
    goals
        .iter()
        .flat_map(|goal| {
            goal.effects
                .effects
                .iter()
                .chain(goal.clauses.iter().flat_map(|clause| match &clause.kind {
                    ClauseKind::Authority { effects, .. } => effects.as_slice(),
                    _ => &[],
                }))
        })
        .filter_map(|effect| effect.resource.clone())
        .collect()
}

fn project_resource(
    effect: &Effect,
    child_goal: &GoalDefinition,
    parent_goal: &GoalDefinition,
    arguments: &[crate::kernel::KernelArgument],
) -> Effect {
    let Some(resource) = &effect.resource else {
        return effect.clone();
    };
    let Some(name) = child_goal
        .clauses
        .iter()
        .find_map(|clause| match &clause.kind {
            ClauseKind::Fact {
                kind: "input",
                binding,
            } if binding.id == *resource => Some(binding.name.as_str()),
            _ => None,
        })
    else {
        return effect.clone();
    };
    let Some(argument) = arguments.iter().find(|argument| argument.name == name) else {
        return effect.clone();
    };
    let parent_resource = match &argument.value.form {
        ExpressionForm::Reference(parent_resource) => parent_resource.clone(),
        ExpressionForm::Call(function, arguments)
            if function == "bhcp/kernel.parent-field@0"
                && matches!(
                    arguments.as_slice(),
                    [Expression {
                        form: ExpressionForm::Literal(Value::Text(_)),
                        ..
                    }]
                ) =>
        {
            let [
                Expression {
                    form: ExpressionForm::Literal(Value::Text(parent_name)),
                    ..
                },
            ] = arguments.as_slice()
            else {
                unreachable!()
            };
            let Some(binding) = parent_goal
                .clauses
                .iter()
                .find_map(|clause| match &clause.kind {
                    ClauseKind::Fact {
                        kind: "input",
                        binding,
                    } if binding.name == *parent_name => Some(binding),
                    _ => None,
                })
            else {
                return effect.clone();
            };
            binding.id.clone()
        }
        _ => return effect.clone(),
    };
    Effect {
        id: effect.id.clone(),
        resource: Some(parent_resource),
        parameters: effect.parameters.clone(),
    }
}

fn graph_effect(effect: &Effect, resources: &HashMap<String, ResourceCoordinate>) -> Result<Value> {
    let mut entries = vec![("id".to_owned(), Value::Text(effect.id.clone()))];
    if let Some(resource) = &effect.resource {
        let coordinate = resources.get(resource).ok_or_else(|| {
            invalid_input("effect resource does not resolve to a retained typed binding")
        })?;
        entries.push(("resource".to_owned(), Value::Text(coordinate.id.clone())));
    }
    if !effect.parameters.is_empty() {
        entries.push((
            "parameters".to_owned(),
            Value::Array(effect.parameters.clone()),
        ));
    }
    Ok(Value::owned_map(entries))
}

fn capability_value(effect: Value, decision: &str, sources: Vec<String>) -> Value {
    Value::map([
        ("effect", effect),
        ("scope", Value::Map(Vec::new())),
        ("decision", Value::Text(decision.to_owned())),
        (
            "sources",
            Value::Array(sources.into_iter().map(Value::Text).collect()),
        ),
    ])
}

fn capability_policy_value(value: &crate::policy::CapabilityPolicyValue) -> Value {
    let mut entries = vec![("effect".to_owned(), Value::Text(value.effect.clone()))];
    if let Some(scope) = &value.scope {
        entries.push(("scope".to_owned(), scope.to_value()));
    }
    Value::owned_map(entries)
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
        .ok_or_else(|| invalid_input("effective policy authority has no source provenance"))?;
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

fn resource_symbol(value_type: &BhcpType) -> Option<&str> {
    match value_type {
        BhcpType::Handle(handle) => resource_symbol(&handle.value_type),
        BhcpType::Nominal(symbol, _) => Some(symbol),
        _ => None,
    }
}

fn effect_covers(rule: &Effect, request: &Effect) -> bool {
    rule.id == request.id
        && rule
            .resource
            .as_ref()
            .is_none_or(|resource| request.resource.as_ref() == Some(resource))
        && (rule.parameters.is_empty() || rule.parameters == request.parameters)
}

fn policy_request_within_scope(
    effect: &Effect,
    scope: &Option<PolicyScope>,
    resources: &HashMap<String, ResourceCoordinate>,
) -> bool {
    let Some(scope) = scope else {
        return true;
    };
    let resource_allowed = scope.resources.as_ref().is_none_or(|allowed| {
        effect
            .resource
            .as_ref()
            .and_then(|resource| resources.get(resource))
            .and_then(|resource| resource_symbol(&resource.value_type))
            .is_some_and(|resource| allowed.iter().any(|candidate| candidate == resource))
    });
    let operation_allowed = scope.operations.as_ref().is_none_or(|allowed| {
        effect
            .parameters
            .iter()
            .any(|parameter| matches!(parameter, Value::Text(value) if allowed.contains(value)))
    });
    resource_allowed && operation_allowed
}

fn effect_gap(effect: &Effect) -> Option<&'static str> {
    if effect.id == "bhcp-effect/unsafe@0" {
        Some("unsafe")
    } else if effect.id == "bhcp-effect/foreign@0" {
        Some("foreign")
    } else if !effect.id.starts_with("bhcp-effect/") {
        Some("unsupported")
    } else {
        None
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

fn stable_source_id(domain: &str, key: &Value) -> Result<String> {
    structural_id(&format!("source:{domain}"), key)
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
    id.push_str("c-");
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
