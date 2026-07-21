//! Generic proof checking across reconstructed obligation graphs and sealed evidence.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use crate::cbor::decode_deterministic;
use crate::diagnostic::{Diagnostic, Result};
use crate::graph::{GraphDocument, GraphNode};
use crate::hash::HashAlgorithm;
use crate::kernel::{ChildObservation, ExecutionResult, KernelRuntime, Reduction, Verdict};
use crate::model::{BhcpType, ClauseKind, ContentReference, GoalDefinition, VerifierBinding};
use crate::obligation::{contract_target_map, validate_compilation, validate_obligation_graph};
use crate::pipeline::Compilation;
use crate::value::Value;
use crate::verification::{
    EvidenceBundle, EvidenceClaim, EvidenceItem, PayloadArtifact, VerifierRegistry,
};

const INVALID_PROOF_INPUT: &str = "BHCP7301";
const INVALID_PROOF: &str = "BHCP7302";
const EXPRESSION_VERIFIER: &str = "bhcp.verifier/expression@0";
const VERIFIER_FAULT_GAP: &str = "bhcp.evidence-gap/verifier-fault@0";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProofState {
    Satisfied,
    Refuted,
    Unresolved,
    Faulted,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProofReport {
    pub state: ProofState,
    pub obligation_status: BTreeMap<String, String>,
}

pub struct ObligationProofRequest<'a> {
    pub compilation: &'a Compilation,
    pub obligation_graph: &'a GraphDocument,
    pub network: &'a str,
    pub parent: &'a Value,
    pub observations: &'a [ChildObservation],
    pub claimed: &'a Reduction,
    pub evidence: &'a EvidenceBundle,
    pub payloads: &'a [PayloadArtifact],
    pub evaluation_contexts: &'a [ProofEvaluationContext],
    pub verifier_registry: &'a VerifierRegistry,
    pub candidate: &'a ContentReference,
    pub candidate_bytes: &'a [u8],
    pub produced_at: &'a str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProofEvaluationContext {
    pub goal: String,
    pub input: Value,
    pub output: Value,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CheckedStatus {
    Discharged,
    Refuted,
    Unresolved,
    Faulted,
}

pub fn verify_obligation_proof(request: ObligationProofRequest<'_>) -> Result<ProofReport> {
    let algorithm = validate_compilation(request.compilation)
        .map_err(|error| invalid_input(format!("invalid compilation: {}", error.message)))?;
    validate_obligation_graph(request.compilation, request.obligation_graph)
        .map_err(|error| invalid_input(format!("invalid obligation graph: {}", error.message)))?;
    request
        .evidence
        .validate()
        .map_err(|error| invalid_input(format!("invalid evidence bundle: {}", error.message)))?;
    if request.evidence.artifact_id.is_none() {
        return Err(invalid_input(
            "proof checking requires an identity-bound evidence bundle",
        ));
    }
    validate_content(request.candidate, request.candidate_bytes, "candidate")?;
    let semantic_ir =
        ContentReference::from_bytes("application/cbor", &request.compilation.ir_bytes, algorithm);
    if request.evidence.semantic_ir != semantic_ir {
        return Err(invalid_input(
            "evidence bundle does not bind the exact semantic IR artifact",
        ));
    }

    let (goal, _) = resolve_network(request.compilation, request.network)?;
    let closure = obligation_closure(request.obligation_graph, &goal.symbol)?;
    let evidence_targets = closure
        .iter()
        .filter(|id| node(request.obligation_graph, id).kind != "discharge")
        .cloned()
        .collect::<BTreeSet<_>>();
    if request
        .evidence
        .obligation_status
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>()
        != evidence_targets
    {
        return Err(invalid_input(
            "evidence statuses do not match the exact obligation closure",
        ));
    }

    let payloads = validate_payloads(request.evidence, request.payloads)?;
    validate_evidence_bindings(&request, goal, &evidence_targets, &payloads)?;

    KernelRuntime::new(&request.compilation.ir)
        .verify(
            request.network,
            request.parent.clone(),
            request.observations,
            request.claimed,
        )
        .map_err(|error| {
            invalid_proof(format!("reducer re-evaluation failed: {}", error.message))
        })?;
    let Reduction::Concluded { result, derivation } = request.claimed else {
        return Err(invalid_proof(
            "a pending reduction is not a completed obligation proof",
        ));
    };
    validate_result_reason(result, request.evidence)?;
    if evidence_identity_collision(request.evidence, &derivation.id) {
        return Err(invalid_input(
            "the derived proof identity collides with supplied evidence",
        ));
    }

    let claims = request
        .evidence
        .claims
        .iter()
        .map(|claim| (claim.id.as_str(), claim))
        .collect::<HashMap<_, _>>();
    validate_premises(
        result,
        derivation.premises.as_slice(),
        &claims,
        request.observations,
    )?;

    let mut statuses = checked_leaf_statuses(request.evidence, &evidence_targets)?;
    derive_dependency_statuses(request.obligation_graph, &closure, &mut statuses)?;
    validate_observation_statuses(
        request.observations,
        request.obligation_graph,
        &closure,
        &statuses,
    )?;
    validate_dependency_premises(
        derivation.premises.as_slice(),
        &claims,
        request.observations,
        request.evidence,
        request.obligation_graph,
        &closure,
    )?;
    let state = validate_outcome(
        result,
        request.obligation_graph,
        &goal.symbol,
        &closure,
        &statuses,
    )?;
    let obligation_status = statuses
        .into_iter()
        .map(|(id, status)| (id, status_name(status).to_owned()))
        .collect();
    Ok(ProofReport {
        state,
        obligation_status,
    })
}

fn resolve_network<'a>(
    compilation: &'a Compilation,
    network: &str,
) -> Result<(&'a GoalDefinition, &'a crate::kernel::KernelNetwork)> {
    compilation
        .ir
        .goals
        .iter()
        .find_map(|goal| {
            goal.body
                .as_ref()
                .filter(|body| body.id == network)
                .map(|body| (goal, body))
        })
        .ok_or_else(|| invalid_input("proof network does not resolve in semantic IR"))
}

fn obligation_closure(graph: &GraphDocument, goal: &str) -> Result<BTreeSet<String>> {
    let mut closure = graph
        .nodes()
        .iter()
        .filter(|node| is_obligation(node) && node_goals(node).contains(goal))
        .map(|node| node.id.clone())
        .collect::<BTreeSet<_>>();
    let mut changed = true;
    while changed {
        changed = false;
        for edge in graph
            .edges()
            .iter()
            .filter(|edge| edge.kind == "depends-on")
        {
            if closure.contains(&edge.from) && closure.insert(edge.to.clone()) {
                changed = true;
            }
        }
    }
    if closure.is_empty() {
        return Err(invalid_input(
            "proof network goal has no structural obligation closure",
        ));
    }
    Ok(closure)
}

fn is_obligation(node: &GraphNode) -> bool {
    node.kind != "case" && (node.kind != "verification" || node.value().get("policy").is_some())
}

fn node_goals(node: &GraphNode) -> BTreeSet<&str> {
    match node.value().get("goals") {
        Some(Value::Array(goals)) => goals
            .iter()
            .filter_map(|goal| match goal {
                Value::Text(goal) => Some(goal.as_str()),
                _ => None,
            })
            .collect(),
        _ => BTreeSet::new(),
    }
}

fn node<'a>(graph: &'a GraphDocument, id: &str) -> &'a GraphNode {
    graph
        .nodes()
        .iter()
        .find(|node| node.id == id)
        .expect("validated graph edge and closure references resolve")
}

fn validate_content(reference: &ContentReference, bytes: &[u8], name: &str) -> Result<()> {
    reference
        .validate()
        .map_err(|error| invalid_input(format!("invalid {name} reference: {}", error.message)))?;
    if reference.size != bytes.len()
        || reference.digests.iter().any(|digest| {
            HashAlgorithm::from_id(&digest.algorithm)
                .map(|algorithm| algorithm.hash(bytes) != *digest)
                .unwrap_or(true)
        })
    {
        return Err(invalid_input(format!(
            "{name} bytes do not match their content reference"
        )));
    }
    Ok(())
}

fn validate_payloads<'a>(
    evidence: &EvidenceBundle,
    payloads: &'a [PayloadArtifact],
) -> Result<HashMap<String, &'a [u8]>> {
    let mut indexed = HashMap::<String, &'a [u8]>::new();
    for payload in payloads {
        validate_content(&payload.reference, &payload.bytes, "evidence payload")?;
        let key = content_key(&payload.reference)?;
        if indexed
            .insert(key.clone(), &payload.bytes)
            .is_some_and(|previous| previous != payload.bytes)
        {
            return Err(invalid_input(
                "one evidence payload reference resolves to different bytes",
            ));
        }
        if !evidence
            .items
            .iter()
            .any(|item| item.payload == payload.reference)
        {
            return Err(invalid_input(
                "proof input contains an unreferenced evidence payload",
            ));
        }
    }
    for item in &evidence.items {
        let key = content_key(&item.payload)?;
        if !indexed.contains_key(&key) {
            return Err(invalid_input(
                "evidence item payload bytes are not sealed with the proof input",
            ));
        }
    }
    Ok(indexed)
}

fn content_key(reference: &ContentReference) -> Result<String> {
    let bytes = crate::cbor::encode_deterministic(&reference.to_value())?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn validate_evidence_bindings(
    request: &ObligationProofRequest<'_>,
    parent_goal: &GoalDefinition,
    targets: &BTreeSet<String>,
    payloads: &HashMap<String, &[u8]>,
) -> Result<()> {
    if request
        .evidence
        .claims
        .iter()
        .any(|claim| claim.subject != *request.candidate || !targets.contains(&claim.obligation))
    {
        return Err(invalid_input(
            "evidence claim does not bind the exact candidate and obligation target",
        ));
    }
    if request
        .evidence
        .items
        .iter()
        .any(|item| item.produced_at != request.produced_at || item.producer != item.verifier)
    {
        return Err(invalid_input(
            "evidence producer or decision time does not match the sealed proof input",
        ));
    }

    let claims = request
        .evidence
        .claims
        .iter()
        .map(|claim| (claim.id.as_str(), claim))
        .collect::<HashMap<_, _>>();
    let contexts = goal_contexts(request, parent_goal, targets)?;
    let items = request
        .evidence
        .items
        .iter()
        .map(|item| (item.id.as_str(), item))
        .collect::<HashMap<_, _>>();
    let mut produced = HashSet::new();
    for edge in &request.evidence.edges {
        if !produced.insert((edge.from.as_str(), edge.to.as_str())) {
            return Err(invalid_input("evidence production edge is duplicated"));
        }
        let item = items
            .get(edge.from.as_str())
            .expect("evidence bundle validation resolved edge source");
        if !item.claims.contains(&edge.to) {
            return Err(invalid_input(
                "evidence production edge is not declared by its source item",
            ));
        }
    }
    for item in &request.evidence.items {
        for claim in &item.claims {
            if request
                .evidence
                .edges
                .iter()
                .filter(|edge| edge.from == item.id && edge.to == *claim)
                .count()
                != 1
            {
                return Err(invalid_input(
                    "evidence item claim does not have one exact production edge",
                ));
            }
        }
    }
    for claim in request
        .evidence
        .claims
        .iter()
        .filter(|claim| claim.status == "accepted")
    {
        let producers = request
            .evidence
            .edges
            .iter()
            .filter(|edge| edge.to == claim.id)
            .filter_map(|edge| items.get(edge.from.as_str()).copied())
            .collect::<Vec<_>>();
        if producers.is_empty() {
            return Err(invalid_input(
                "accepted evidence claim has no sealed producing item",
            ));
        }
        for item in producers {
            validate_claim_channel(
                request.compilation,
                request.obligation_graph,
                claim,
                item,
                payloads,
                &contexts,
                request.verifier_registry,
            )?;
        }
    }
    for item in &request.evidence.items {
        if item.claims.iter().any(|claim| {
            !claims.contains_key(claim.as_str())
                || !request
                    .evidence
                    .edges
                    .iter()
                    .any(|edge| edge.from == item.id && edge.to == *claim)
        }) {
            return Err(invalid_input(
                "evidence item claim is not paired with its production edge",
            ));
        }
    }
    validate_policy_minima(request.evidence, request.obligation_graph, targets, &items)?;
    Ok(())
}

fn validate_claim_channel(
    compilation: &Compilation,
    graph: &GraphDocument,
    claim: &EvidenceClaim,
    item: &EvidenceItem,
    payloads: &HashMap<String, &[u8]>,
    contexts: &HashMap<String, (Value, Value)>,
    verifier_registry: &VerifierRegistry,
) -> Result<()> {
    if compilation
        .ir
        .extensions
        .iter()
        .any(|extension| extension.extension == item.verifier)
    {
        return Err(invalid_input(
            "native extension nodes cannot act as proof callbacks",
        ));
    }
    let target = node(graph, &claim.obligation);
    if let Some(policy) = target.value().get("policy") {
        let expected = policy_predicate(policy)?;
        if claim.predicate != expected {
            return Err(invalid_input(
                "policy evidence predicate does not match its structural obligation",
            ));
        }
        if text(policy, "category") == Some("evidence") {
            let value = policy.get("value").expect("validated policy value");
            let classes = texts(value, "classes");
            if !classes.contains(&item.evidence_class.as_str()) {
                return Err(invalid_input(
                    "policy evidence class is outside the retained rule",
                ));
            }
            if !verifier_registry.validates_evidence_authority(Some(expected), item)? {
                return Err(invalid_input(
                    "policy evidence producer is not registered and bound to the retained obligation",
                ));
            }
        }
        return Ok(());
    }

    if item.verifier == EXPRESSION_VERIFIER {
        if item.evidence_class != "static"
            || claim.predicate != EXPRESSION_VERIFIER
            || item.verifier_artifact
                != ContentReference::from_bytes(
                    "application/vnd.bhcp.verifier",
                    EXPRESSION_VERIFIER.as_bytes(),
                    HashAlgorithm::default(),
                )
        {
            return Err(invalid_input(
                "total-condition evidence does not match the built-in verifier",
            ));
        }
        let key = content_key(&item.payload)?;
        validate_expression_payload(
            payloads.get(&key).expect("payload coverage checked"),
            target,
            claim,
            compilation,
            contexts,
        )?;
        return Ok(());
    }

    let bindings = verifier_bindings(compilation, &claim.obligation)?;
    let Some(binding) = bindings
        .into_iter()
        .find(|binding| binding.verifier == item.verifier)
    else {
        return Err(invalid_input(
            "evidence producer is not retained for this structural obligation",
        ));
    };
    let BhcpType::Evidence(classes) = &binding.output else {
        return Err(invalid_input("retained verifier output is not Evidence"));
    };
    if (!classes.is_empty() && !classes.contains(&item.evidence_class))
        || (!binding.trust.is_empty() && !binding.trust.contains(&item.evidence_class))
    {
        return Err(invalid_input(
            "evidence class is outside the retained verifier type or trust declaration",
        ));
    }
    if !verifier_registry.validates_evidence_authority(None, item)? {
        return Err(invalid_input(
            "evidence producer is not bound to the trusted verifier registry",
        ));
    }
    Ok(())
}

fn validate_expression_payload(
    bytes: &[u8],
    target: &GraphNode,
    claim: &EvidenceClaim,
    compilation: &Compilation,
    contexts: &HashMap<String, (Value, Value)>,
) -> Result<()> {
    let value = decode_deterministic(bytes).map_err(|error| {
        invalid_input(format!("invalid expression evidence: {}", error.message))
    })?;
    let expected_result = match (claim.status.as_str(), claim.polarity.as_str()) {
        ("accepted", "supports") => true,
        ("accepted", "refutes") => false,
        _ => return Err(invalid_input("expression evidence disposition is invalid")),
    };
    let target_goal = node_goals(target)
        .into_iter()
        .next()
        .ok_or_else(|| invalid_input("contract obligation has no goal"))?;
    let goal = compilation
        .ir
        .goals
        .iter()
        .find(|goal| goal.symbol == target_goal)
        .ok_or_else(|| invalid_input("contract obligation goal does not resolve"))?;
    if text(&value, "obligation") != Some(target.id.as_str())
        || value.get("result") != Some(&Value::Bool(expected_result))
        || text(&value, "goal") != Some(goal.id.as_str())
    {
        return Err(invalid_input(
            "expression evidence payload does not match its structural target",
        ));
    }
    let (input, output) = contexts.get(&goal.symbol).ok_or_else(|| {
        invalid_input("expression evidence has no sealed input and output evaluation context")
    })?;
    if value.get("input") != Some(input) || value.get("output") != Some(output) {
        return Err(invalid_input(
            "expression evidence payload does not match its sealed evaluation context",
        ));
    }
    for source in texts(target.value(), "source_clauses") {
        let actual = crate::verification::evaluate_contract_condition(goal, source, input, output)?;
        if actual != expected_result {
            return Err(invalid_input(
                "total-condition evidence disagrees with deterministic re-evaluation",
            ));
        }
    }
    Ok(())
}

fn goal_contexts(
    request: &ObligationProofRequest<'_>,
    parent_goal: &GoalDefinition,
    targets: &BTreeSet<String>,
) -> Result<HashMap<String, (Value, Value)>> {
    let (_, network) = resolve_network(request.compilation, request.network)?;
    let mut allowed = targets
        .iter()
        .flat_map(|target| node_goals(node(request.obligation_graph, target)))
        .collect::<BTreeSet<_>>();
    allowed.insert(parent_goal.symbol.as_str());
    let mut contexts = HashMap::new();
    for context in request.evaluation_contexts {
        if !allowed.contains(context.goal.as_str())
            || contexts
                .insert(
                    context.goal.clone(),
                    (context.input.clone(), context.output.clone()),
                )
                .is_some()
        {
            return Err(invalid_input(
                "proof evaluation context is duplicated or outside the network closure",
            ));
        }
    }
    if contexts
        .get(&parent_goal.symbol)
        .is_some_and(|(input, _)| input != request.parent)
    {
        return Err(invalid_input(
            "parent evaluation context does not match the sealed network input",
        ));
    }
    if let Reduction::Concluded {
        result: ExecutionResult::Completed(Verdict::Satisfied { output, .. }),
        ..
    } = request.claimed
    {
        insert_or_match_context(
            &mut contexts,
            parent_goal,
            request.parent.clone(),
            output.clone(),
        )?;
    }
    for observation in request.observations {
        let child = network
            .children
            .iter()
            .find(|child| child.id == observation.child)
            .expect("kernel re-evaluation resolved every observation");
        let child_goal = request
            .compilation
            .ir
            .goals
            .iter()
            .find(|goal| goal.id == child.goal)
            .expect("validated semantic IR resolves every child goal");
        let input = child_input(network, child, request.parent, request.observations)?;
        if contexts
            .get(&child_goal.symbol)
            .is_some_and(|(expected, _)| expected != &input)
        {
            return Err(invalid_input(
                "child evaluation context does not match its retained data edges",
            ));
        }
        if let ExecutionResult::Completed(Verdict::Satisfied { output, .. }) = &observation.result {
            insert_or_match_context(&mut contexts, child_goal, input, output.clone())?;
        }
    }
    Ok(contexts)
}

fn insert_or_match_context(
    contexts: &mut HashMap<String, (Value, Value)>,
    goal: &GoalDefinition,
    input: Value,
    output: Value,
) -> Result<()> {
    if let Some(expected) = contexts.get(&goal.symbol) {
        if expected != &(input, output) {
            return Err(invalid_input(
                "proof evaluation context does not match the reconstructed execution value",
            ));
        }
    } else {
        contexts.insert(goal.symbol.clone(), (input, output));
    }
    Ok(())
}

fn child_input(
    network: &crate::kernel::KernelNetwork,
    child: &crate::kernel::KernelChild,
    parent: &Value,
    observations: &[ChildObservation],
) -> Result<Value> {
    let mut fields = Vec::new();
    for argument in &child.arguments {
        let crate::model::ExpressionForm::Call(symbol, parameters) = &argument.value.form else {
            return Err(invalid_input(
                "child proof input is not a retained data edge",
            ));
        };
        let [parameter] = parameters.as_slice() else {
            return Err(invalid_input("child proof data edge has invalid arity"));
        };
        let crate::model::ExpressionForm::Literal(Value::Text(coordinate)) = &parameter.form else {
            return Err(invalid_input("child proof data edge is not literal"));
        };
        let value = match symbol.as_str() {
            "bhcp/kernel.parent-field@0" => parent
                .get(coordinate)
                .cloned()
                .ok_or_else(|| invalid_input("parent proof input field is missing"))?,
            "bhcp/kernel.observed-output@0" => {
                let predecessor = network
                    .children
                    .iter()
                    .find(|candidate| candidate.tag == *coordinate)
                    .expect("validated data edge resolves predecessor tag");
                let observation = observations
                    .iter()
                    .find(|observation| observation.child == predecessor.id)
                    .ok_or_else(|| invalid_input("proof data-edge observation is missing"))?;
                let ExecutionResult::Completed(Verdict::Satisfied { output, .. }) =
                    &observation.result
                else {
                    return Err(invalid_input(
                        "proof data-edge predecessor is not satisfied",
                    ));
                };
                output.clone()
            }
            _ => return Err(invalid_input("child proof data-edge symbol is unsupported")),
        };
        fields.push((argument.name.clone(), value));
    }
    Ok(Value::owned_map(fields))
}

fn verifier_bindings<'a>(
    compilation: &'a Compilation,
    target: &str,
) -> Result<Vec<&'a VerifierBinding>> {
    let mut bindings = Vec::new();
    for goal in &compilation.ir.goals {
        let targets = contract_target_map(goal)?;
        for clause in &goal.clauses {
            let ClauseKind::Verify {
                binding,
                obligations,
            } = &clause.kind
            else {
                continue;
            };
            let selected = if obligations.is_empty() {
                targets.values().any(|candidate| candidate == target)
            } else {
                obligations.iter().any(|source| {
                    targets
                        .get(source)
                        .is_some_and(|candidate| candidate == target)
                })
            };
            if selected {
                bindings.push(binding);
            }
        }
    }
    Ok(bindings)
}

fn validate_policy_minima(
    evidence: &EvidenceBundle,
    graph: &GraphDocument,
    targets: &BTreeSet<String>,
    items: &HashMap<&str, &EvidenceItem>,
) -> Result<()> {
    let expected_metadata = targets
        .iter()
        .filter(|target| {
            node(graph, target)
                .value()
                .get("policy")
                .is_some_and(|policy| text(policy, "category") == Some("evidence"))
        })
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let actual_metadata = evidence
        .policy_obligations
        .iter()
        .map(|obligation| obligation.id.as_str())
        .collect::<BTreeSet<_>>();
    if actual_metadata != expected_metadata {
        return Err(invalid_input(
            "policy evidence metadata does not match the exact obligation closure",
        ));
    }
    for target in targets {
        let node = node(graph, target);
        let Some(policy) = node.value().get("policy") else {
            continue;
        };
        if text(policy, "category") != Some("evidence") {
            continue;
        }
        let value = policy.get("value").expect("validated policy value");
        let minimum = unsigned(value, "minimum")?;
        let symbol = text(value, "obligation").expect("validated obligation symbol");
        let metadata = evidence
            .policy_obligations
            .iter()
            .find(|obligation| obligation.id == *target)
            .ok_or_else(|| invalid_input("policy evidence metadata is missing"))?;
        if metadata.symbol != symbol
            || metadata.minimum != minimum
            || metadata.effective_rule != unsigned(policy, "effective_rule")? as usize
            || metadata
                .classes
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                != texts(value, "classes")
            || !policy_sources_match(policy, metadata)
        {
            return Err(invalid_input(
                "policy evidence metadata does not match the retained rule",
            ));
        }
        let producers = evidence
            .claims
            .iter()
            .filter(|claim| {
                claim.obligation == *target
                    && claim.status == "accepted"
                    && claim.polarity == "supports"
            })
            .flat_map(|claim| {
                evidence
                    .edges
                    .iter()
                    .filter(move |edge| edge.to == claim.id)
                    .filter_map(|edge| {
                        items
                            .get(edge.from.as_str())
                            .map(|item| item.producer.as_str())
                    })
            })
            .collect::<BTreeSet<_>>();
        if evidence.obligation_status.get(target).map(String::as_str) == Some("discharged")
            && producers.len() < minimum as usize
        {
            return Err(invalid_input(
                "policy evidence minimum is not met by distinct producers",
            ));
        }
    }
    Ok(())
}

fn policy_sources_match(
    policy: &Value,
    metadata: &crate::verification::PolicyEvidenceObligation,
) -> bool {
    let Some(Value::Array(sources)) = policy.get("sources") else {
        return false;
    };
    sources.len() == metadata.sources.len()
        && sources
            .iter()
            .zip(&metadata.sources)
            .all(|(expected, actual)| {
                text(expected, "layer") == Some(actual.layer.as_str())
                    && text(expected, "policy") == Some(actual.policy.as_str())
                    && text(expected, "rule") == Some(actual.rule.as_str())
            })
}

fn policy_predicate(policy: &Value) -> Result<&str> {
    let category = text(policy, "category").expect("validated policy category");
    let value = policy.get("value").expect("validated policy value");
    let field = match category {
        "requirement" => "requirement",
        "evidence" => "obligation",
        "limit" => "dimension",
        _ => return Err(invalid_input("unsupported policy proof category")),
    };
    text(value, field).ok_or_else(|| invalid_input("policy proof predicate is missing"))
}

fn validate_premises(
    result: &ExecutionResult,
    premises: &[String],
    claims: &HashMap<&str, &EvidenceClaim>,
    observations: &[ChildObservation],
) -> Result<()> {
    let observed = observation_tokens(observations);
    for premise in premises {
        let valid = if let Some(claim) = claims.get(premise.as_str()) {
            match result {
                ExecutionResult::Completed(Verdict::Satisfied { .. })
                | ExecutionResult::Completed(Verdict::Refuted { .. }) => {
                    claim.status == "accepted"
                        && matches!(claim.polarity.as_str(), "supports" | "refutes")
                }
                ExecutionResult::Completed(Verdict::Unresolved { .. }) => {
                    claim.status == "accepted" || claim.status == "unresolved"
                }
                ExecutionResult::Faulted(_) => false,
            }
        } else {
            premise.starts_with("derivation-") && observed.contains(premise.as_str())
        };
        if !valid {
            return Err(invalid_proof(
                "derivation premise disposition cannot justify the claimed outcome",
            ));
        }
    }
    Ok(())
}

fn observation_tokens(observations: &[ChildObservation]) -> HashSet<&str> {
    observations
        .iter()
        .flat_map(|observation| match &observation.result {
            ExecutionResult::Completed(Verdict::Satisfied { evidence, .. }) => evidence.as_slice(),
            ExecutionResult::Completed(Verdict::Refuted { counter_evidence }) => {
                counter_evidence.as_slice()
            }
            ExecutionResult::Completed(Verdict::Unresolved {
                partial_evidence, ..
            }) => partial_evidence.as_slice(),
            ExecutionResult::Faulted(_) => &[] as &[String],
        })
        .map(String::as_str)
        .collect()
}

fn validate_result_reason(result: &ExecutionResult, evidence: &EvidenceBundle) -> Result<()> {
    let matches_gap = |reason: &crate::kernel::Reason, kind: Option<&str>| {
        evidence.gaps.iter().any(|gap| {
            gap.required && kind.is_none_or(|kind| gap.kind == kind) && gap.reason == *reason
        })
    };
    let valid = match result {
        ExecutionResult::Completed(Verdict::Unresolved { reason, .. }) => matches_gap(reason, None),
        ExecutionResult::Faulted(fault) => matches_gap(&fault.error, Some(VERIFIER_FAULT_GAP)),
        _ => true,
    };
    if valid {
        Ok(())
    } else {
        Err(invalid_proof(
            "unresolved or faulted result does not bind an exact sealed evidence gap",
        ))
    }
}

fn checked_leaf_statuses(
    evidence: &EvidenceBundle,
    targets: &BTreeSet<String>,
) -> Result<BTreeMap<String, CheckedStatus>> {
    let faulted = evidence
        .gaps
        .iter()
        .filter(|gap| gap.kind == VERIFIER_FAULT_GAP)
        .flat_map(|gap| gap.obligations.iter().cloned())
        .collect::<HashSet<_>>();
    targets
        .iter()
        .map(|target| {
            let status = if faulted.contains(target) {
                CheckedStatus::Faulted
            } else {
                match evidence.obligation_status.get(target).map(String::as_str) {
                    Some("discharged") => CheckedStatus::Discharged,
                    Some("refuted") => CheckedStatus::Refuted,
                    Some("unresolved") => CheckedStatus::Unresolved,
                    _ => return Err(invalid_input("invalid obligation evidence status")),
                }
            };
            Ok((target.clone(), status))
        })
        .collect()
}

fn derive_dependency_statuses(
    graph: &GraphDocument,
    closure: &BTreeSet<String>,
    statuses: &mut BTreeMap<String, CheckedStatus>,
) -> Result<()> {
    let discharge = closure
        .iter()
        .filter(|id| node(graph, id).kind == "discharge")
        .cloned()
        .collect::<BTreeSet<_>>();
    while statuses.keys().filter(|id| discharge.contains(*id)).count() < discharge.len() {
        let mut changed = false;
        for id in &discharge {
            if statuses.contains_key(id) {
                continue;
            }
            let dependencies = graph
                .edges()
                .iter()
                .filter(|edge| edge.kind == "depends-on" && edge.from == *id)
                .map(|edge| edge.to.as_str())
                .collect::<Vec<_>>();
            if dependencies.is_empty() {
                return Err(invalid_input(
                    "discharge obligation has no structural prerequisite",
                ));
            }
            let Some(values) = dependencies
                .iter()
                .map(|dependency| statuses.get(*dependency).copied())
                .collect::<Option<Vec<_>>>()
            else {
                continue;
            };
            let status = if values.contains(&CheckedStatus::Refuted) {
                CheckedStatus::Refuted
            } else if values.contains(&CheckedStatus::Faulted) {
                CheckedStatus::Faulted
            } else if values.contains(&CheckedStatus::Unresolved) {
                CheckedStatus::Unresolved
            } else {
                CheckedStatus::Discharged
            };
            statuses.insert(id.clone(), status);
            changed = true;
        }
        if !changed {
            return Err(invalid_input(
                "obligation dependencies do not form a complete acyclic closure",
            ));
        }
    }
    Ok(())
}

fn validate_observation_statuses(
    observations: &[ChildObservation],
    graph: &GraphDocument,
    closure: &BTreeSet<String>,
    statuses: &BTreeMap<String, CheckedStatus>,
) -> Result<()> {
    let discharge = closure
        .iter()
        .filter(|id| node(graph, id).kind == "discharge")
        .collect::<Vec<_>>();
    for observation in observations {
        let matches = discharge
            .iter()
            .filter(|id| {
                texts(node(graph, id).value(), "source_clauses")
                    .contains(&observation.child.as_str())
            })
            .collect::<Vec<_>>();
        let [discharge] = matches.as_slice() else {
            return Err(invalid_input(
                "child observation does not have one exact structural discharge",
            ));
        };
        let dependencies = graph
            .edges()
            .iter()
            .filter(|edge| edge.kind == "depends-on" && edge.from.as_str() == discharge.as_str())
            .map(|edge| {
                statuses
                    .get(&edge.to)
                    .copied()
                    .ok_or_else(|| invalid_input("child obligation status is missing"))
            })
            .collect::<Result<Vec<_>>>()?;
        if dependencies.is_empty() || !result_matches_statuses(&observation.result, &dependencies) {
            return Err(invalid_proof(
                "child observation does not match its aggregate obligation evidence",
            ));
        }
    }
    Ok(())
}

fn result_matches_statuses(result: &ExecutionResult, statuses: &[CheckedStatus]) -> bool {
    let has = |status| statuses.contains(&status);
    match result {
        ExecutionResult::Completed(Verdict::Satisfied { .. }) => statuses
            .iter()
            .all(|status| *status == CheckedStatus::Discharged),
        ExecutionResult::Completed(Verdict::Refuted { .. }) => has(CheckedStatus::Refuted),
        ExecutionResult::Completed(Verdict::Unresolved { .. }) => {
            !has(CheckedStatus::Refuted)
                && !has(CheckedStatus::Faulted)
                && has(CheckedStatus::Unresolved)
        }
        ExecutionResult::Faulted(_) => !has(CheckedStatus::Refuted) && has(CheckedStatus::Faulted),
    }
}

fn validate_dependency_premises(
    premises: &[String],
    claims: &HashMap<&str, &EvidenceClaim>,
    observations: &[ChildObservation],
    evidence: &EvidenceBundle,
    graph: &GraphDocument,
    closure: &BTreeSet<String>,
) -> Result<()> {
    let dependencies = graph
        .edges()
        .iter()
        .filter(|edge| edge.kind == "depends-on" && closure.contains(&edge.from))
        .map(|edge| (edge.to.as_str(), node(graph, &edge.from)))
        .collect::<Vec<_>>();
    for premise in premises {
        let Some(claim) = claims.get(premise.as_str()) else {
            continue;
        };
        let Some((_, discharge)) = dependencies
            .iter()
            .find(|(target, _)| *target == claim.obligation)
        else {
            return Err(invalid_proof(
                "reducer premise does not target a structural child dependency",
            ));
        };
        let child_ids = texts(discharge.value(), "source_clauses");
        let valid = observations.iter().any(|observation| {
            child_ids.contains(&observation.child.as_str())
                && observation_claim_matches(&observation.result, premise, claim)
        });
        if !valid {
            return Err(invalid_proof(
                "reducer premise is not sealed by the matching child dependency",
            ));
        }
    }
    for (target, discharge) in dependencies {
        let child_ids = texts(discharge.value(), "source_clauses");
        for observation in observations
            .iter()
            .filter(|observation| child_ids.contains(&observation.child.as_str()))
        {
            let reason = match &observation.result {
                ExecutionResult::Completed(Verdict::Unresolved { reason, .. }) => {
                    Some((reason, false))
                }
                ExecutionResult::Faulted(fault) => Some((&fault.error, true)),
                _ => None,
            };
            if let Some((reason, faulted)) = reason
                && !evidence.gaps.iter().any(|gap| {
                    gap.obligations
                        .iter()
                        .any(|obligation| obligation == target)
                        && (gap.kind == VERIFIER_FAULT_GAP) == faulted
                        && gap.reason == *reason
                })
            {
                return Err(invalid_proof(
                    "child unresolved or fault reason is not sealed by its obligation gap",
                ));
            }
        }
    }
    Ok(())
}

fn observation_claim_matches(
    result: &ExecutionResult,
    premise: &str,
    claim: &EvidenceClaim,
) -> bool {
    match result {
        ExecutionResult::Completed(Verdict::Satisfied { evidence, .. }) => {
            evidence.iter().any(|token| token == premise)
                && claim.status == "accepted"
                && claim.polarity == "supports"
        }
        ExecutionResult::Completed(Verdict::Refuted { counter_evidence }) => {
            counter_evidence.iter().any(|token| token == premise)
                && claim.status == "accepted"
                && claim.polarity == "refutes"
        }
        ExecutionResult::Completed(Verdict::Unresolved {
            partial_evidence, ..
        }) => {
            partial_evidence.iter().any(|token| token == premise)
                && matches!(claim.status.as_str(), "accepted" | "unresolved")
        }
        ExecutionResult::Faulted(_) => false,
    }
}

fn validate_outcome(
    result: &ExecutionResult,
    graph: &GraphDocument,
    goal: &str,
    closure: &BTreeSet<String>,
    statuses: &BTreeMap<String, CheckedStatus>,
) -> Result<ProofState> {
    let has = |status| statuses.values().any(|candidate| *candidate == status);
    let local = closure
        .iter()
        .filter(|id| {
            let node = node(graph, id);
            node.kind != "discharge" && node_goals(node).contains(goal)
        })
        .filter_map(|id| statuses.get(id))
        .copied()
        .collect::<Vec<_>>();
    let state = match result {
        ExecutionResult::Completed(Verdict::Satisfied { .. })
            if local
                .iter()
                .all(|status| *status == CheckedStatus::Discharged) =>
        {
            ProofState::Satisfied
        }
        ExecutionResult::Completed(Verdict::Refuted { .. }) if has(CheckedStatus::Refuted) => {
            ProofState::Refuted
        }
        ExecutionResult::Completed(Verdict::Unresolved { .. })
            if !has(CheckedStatus::Refuted)
                && !has(CheckedStatus::Faulted)
                && has(CheckedStatus::Unresolved) =>
        {
            ProofState::Unresolved
        }
        ExecutionResult::Faulted(_)
            if !has(CheckedStatus::Refuted) && has(CheckedStatus::Faulted) =>
        {
            ProofState::Faulted
        }
        _ => {
            return Err(invalid_proof(
                "claimed execution outcome does not match checked obligation dispositions",
            ));
        }
    };
    Ok(state)
}

fn evidence_identity_collision(evidence: &EvidenceBundle, derivation: &str) -> bool {
    evidence
        .claims
        .iter()
        .any(|value| value.id == derivation || value.id.starts_with("derivation-"))
        || evidence
            .items
            .iter()
            .any(|value| value.id == derivation || value.id.starts_with("derivation-"))
        || evidence
            .gaps
            .iter()
            .any(|value| value.id == derivation || value.id.starts_with("derivation-"))
        || evidence
            .edges
            .iter()
            .any(|value| value.id == derivation || value.id.starts_with("derivation-"))
}

fn text<'a>(value: &'a Value, field: &str) -> Option<&'a str> {
    match value.get(field) {
        Some(Value::Text(value)) => Some(value),
        _ => None,
    }
}

fn texts<'a>(value: &'a Value, field: &str) -> Vec<&'a str> {
    match value.get(field) {
        Some(Value::Array(values)) => values
            .iter()
            .filter_map(|value| match value {
                Value::Text(value) => Some(value.as_str()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn unsigned(value: &Value, field: &str) -> Result<u64> {
    match value.get(field) {
        Some(Value::Integer(value)) if *value >= 0 => {
            u64::try_from(*value).map_err(|_| invalid_input("integer is out of range"))
        }
        _ => Err(invalid_input("required unsigned integer is missing")),
    }
}

fn status_name(status: CheckedStatus) -> &'static str {
    match status {
        CheckedStatus::Discharged => "discharged",
        CheckedStatus::Refuted => "refuted",
        CheckedStatus::Unresolved => "unresolved",
        CheckedStatus::Faulted => "faulted",
    }
}

fn invalid_input(message: impl Into<String>) -> Diagnostic {
    Diagnostic::plain(INVALID_PROOF_INPUT, message)
}

fn invalid_proof(message: impl Into<String>) -> Diagnostic {
    Diagnostic::plain(INVALID_PROOF, message)
}
