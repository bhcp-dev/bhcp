//! Shared typed model for the v0 analysis, execution, and evidence graphs.

use std::collections::{BTreeMap, BTreeSet};

use crate::cbor::{decode_deterministic, encode_deterministic};
use crate::diagnostic::{Diagnostic, Result};
use crate::hash::{HashAlgorithm, artifact_hash_with, hash_value};
use crate::model::HashId;
use crate::value::Value;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GraphKind {
    Obligation,
    Capability,
    State,
    Execution,
    Evidence,
}

impl GraphKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Obligation => "obligation-graph",
            Self::Capability => "capability-graph",
            Self::State => "state-graph",
            Self::Execution => "execution-graph",
            Self::Evidence => "evidence-bundle",
        }
    }

    fn from_str(kind: &str) -> Result<Self> {
        match kind {
            "obligation-graph" => Ok(Self::Obligation),
            "capability-graph" => Ok(Self::Capability),
            "state-graph" => Ok(Self::State),
            "execution-graph" => Ok(Self::Execution),
            "evidence-bundle" => Ok(Self::Evidence),
            _ => Err(invalid_schema(format!(
                "{kind:?} is not a typed graph root"
            ))),
        }
    }

    fn forbids_cycles(self) -> bool {
        matches!(self, Self::Obligation | Self::Execution)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphNode {
    pub id: String,
    pub kind: String,
    value: Value,
}

impl GraphNode {
    pub fn value(&self) -> &Value {
        &self.value
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphEdge {
    pub id: String,
    pub from: String,
    pub to: String,
    pub kind: String,
    value: Value,
}

impl GraphEdge {
    pub fn value(&self) -> &Value {
        &self.value
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphDocument {
    kind: GraphKind,
    value: Value,
    nodes: Vec<GraphNode>,
    edges: Vec<GraphEdge>,
    semantic_id: Option<HashId>,
    artifact_id: Option<HashId>,
}

impl GraphDocument {
    pub fn from_cbor(bytes: &[u8]) -> Result<Self> {
        Self::from_value(&decode_deterministic(bytes)?)
    }

    pub fn from_value(value: &Value) -> Result<Self> {
        let kind = GraphKind::from_str(
            text_field(value, "kind").ok_or_else(|| invalid_schema("graph kind must be text"))?,
        )?;
        reject_unknown_graph_fields(value, kind)?;
        validate_graph_shape(value, kind)?;

        let mut value = value.clone();
        normalize_document(&mut value, kind)?;
        let nodes = collect_nodes(&value, kind)?;
        let edges = collect_edges(&value)?;
        validate_references(&value, kind, &nodes, &edges)?;

        let semantic_id = optional_hash(&value, "semantic_id")?;
        let artifact_id = optional_hash(&value, "artifact_id")?;
        let document = Self {
            kind,
            value,
            nodes,
            edges,
            semantic_id,
            artifact_id,
        };
        document.validate_identities()?;
        Ok(document)
    }

    pub fn kind(&self) -> GraphKind {
        self.kind
    }

    pub fn nodes(&self) -> &[GraphNode] {
        &self.nodes
    }

    pub fn edges(&self) -> &[GraphEdge] {
        &self.edges
    }

    pub fn semantic_id(&self) -> Option<&HashId> {
        self.semantic_id.as_ref()
    }

    pub fn artifact_id(&self) -> Option<&HashId> {
        self.artifact_id.as_ref()
    }

    pub fn to_value(&self) -> Value {
        self.value.clone()
    }

    pub fn to_cbor(&self) -> Result<Vec<u8>> {
        encode_deterministic(&self.value)
    }

    pub fn compute_semantic_id(&self, algorithm: HashAlgorithm) -> Result<HashId> {
        hash_value(&semantic_projection(&self.value, self.kind), algorithm)
    }

    pub fn compute_artifact_id(&self, algorithm: HashAlgorithm) -> Result<HashId> {
        artifact_hash_with(&self.value, algorithm)
    }

    pub fn materialize_identities(&mut self, algorithm: HashAlgorithm) -> Result<()> {
        remove_field(&mut self.value, "semantic_id");
        remove_field(&mut self.value, "artifact_id");
        let semantic_id = self.compute_semantic_id(algorithm)?;
        insert_field(&mut self.value, "semantic_id", semantic_id.to_value())?;
        self.semantic_id = Some(semantic_id);
        let artifact_id = self.compute_artifact_id(algorithm)?;
        insert_field(&mut self.value, "artifact_id", artifact_id.to_value())?;
        self.artifact_id = Some(artifact_id);
        validate_graph_shape(&self.value, self.kind)?;
        Ok(())
    }

    fn validate_identities(&self) -> Result<()> {
        if let Some(identity) = &self.semantic_id {
            identity
                .validate()
                .map_err(|error| invalid_identity(error.message))?;
            let algorithm = HashAlgorithm::from_id(&identity.algorithm)
                .map_err(|error| invalid_identity(error.message))?;
            if self.compute_semantic_id(algorithm)? != *identity {
                return Err(invalid_identity("graph semantic identity does not match"));
            }
        }
        if let Some(identity) = &self.artifact_id {
            identity
                .validate()
                .map_err(|error| invalid_identity(error.message))?;
            let algorithm = HashAlgorithm::from_id(&identity.algorithm)
                .map_err(|error| invalid_identity(error.message))?;
            if self.compute_artifact_id(algorithm)? != *identity {
                return Err(invalid_identity("graph artifact identity does not match"));
            }
        }
        Ok(())
    }
}

fn validate_graph_shape(value: &Value, kind: GraphKind) -> Result<()> {
    encode_deterministic(value).map_err(|error| {
        invalid_schema(format!("graph wire value is invalid: {}", error.message))
    })?;
    crate::schema::validate_root(value, kind.as_str())
        .map_err(|error| invalid_schema(error.message))?;
    require_text_value(value, "version", "graph header")?;
    if text_field(value, "version") != Some("bhcp/v0") {
        return Err(invalid_schema("graph version must equal \"bhcp/v0\""));
    }
    let features = require_array(value, "features", "graph header")?;
    for feature in features {
        let Value::Text(feature) = feature else {
            return Err(invalid_schema("graph feature must be a symbol-id"));
        };
        if !crate::model::is_symbol(feature) {
            return Err(invalid_schema("graph feature must be a symbol-id"));
        }
    }
    for field in ["semantic_id", "artifact_id"] {
        if let Some(value) = value.get(field) {
            parse_hash(value)?;
        }
    }
    if let Some(provenance) = value.get("provenance") {
        validate_provenance(provenance)?;
    }
    if let Some(Value::Array(authorizations)) = value.get("authorization") {
        for authorization in authorizations {
            validate_authorization(authorization)?;
        }
    } else if value.get("authorization").is_some() {
        return Err(invalid_schema("graph authorization must be an array"));
    }
    validate_content_reference(
        value
            .get("semantic_ir")
            .ok_or_else(|| invalid_schema("graph requires semantic_ir"))?,
    )?;
    match kind {
        GraphKind::Obligation => validate_standard_graph(value, obligation_node_fields, None),
        GraphKind::Capability => validate_standard_graph(value, capability_node_fields, None),
        GraphKind::State => {
            validate_standard_graph(value, state_node_fields, None)?;
            for transition in require_array(value, "transitions", "state graph")? {
                validate_state_transition(transition)?;
            }
            Ok(())
        }
        GraphKind::Execution => {
            validate_standard_graph(value, execution_node_fields, Some(execution_edge_fields))?;
            for entrypoint in require_array(value, "entrypoints", "execution graph")? {
                validate_ref_value(entrypoint, "execution entrypoint")?;
            }
            Ok(())
        }
        GraphKind::Evidence => validate_evidence_shape(value),
    }
}

fn validate_standard_graph(
    value: &Value,
    node_validator: fn(&Value) -> Result<()>,
    edge_validator: Option<fn(&Value) -> Result<()>>,
) -> Result<()> {
    for node in require_array(value, "nodes", "graph")? {
        node_validator(node)?;
    }
    for edge in require_array(value, "edges", "graph")? {
        edge_validator.unwrap_or(graph_edge_fields)(edge)?;
    }
    Ok(())
}

fn obligation_node_fields(value: &Value) -> Result<()> {
    validate_map_fields(
        value,
        &["id", "kind", "clause", "status"],
        &["evidence", "goals", "source_clauses", "recursion", "policy"],
        "obligation node",
    )?;
    required_ref(value, "id")?;
    required_ref(value, "clause")?;
    require_one_of(
        value,
        "kind",
        &[
            "requirement",
            "guarantee",
            "invariant",
            "limit",
            "verification",
            "case",
            "discharge",
        ],
        "obligation node",
    )?;
    require_one_of(
        value,
        "status",
        &["open", "discharged", "refuted", "unresolved"],
        "obligation node",
    )?;
    validate_optional_ref_array(value, "evidence", "obligation node")?;
    validate_nonempty_symbol_array(value, "goals", "obligation node")?;
    validate_optional_nonempty_ref_array(value, "source_clauses", "obligation node")?;
    if let Some(recursion) = value.get("recursion") {
        if text_field(value, "kind") != Some("limit") {
            return Err(invalid_schema(
                "only a limit obligation may carry recursion evidence",
            ));
        }
        validate_recursion_bound(recursion)?;
    }
    if let Some(policy) = value.get("policy") {
        validate_policy_obligation(policy, text_field(value, "kind").unwrap())?;
    }
    Ok(())
}

fn validate_recursion_bound(value: &Value) -> Result<()> {
    match text_field(value, "kind") {
        Some("bounded") => {
            validate_map_fields(value, &["kind", "maximum"], &[], "recursion bound")?;
            require_unsigned(value, "maximum", "recursion bound")?;
            if matches!(value.get("maximum"), Some(Value::Integer(0))) {
                return Err(invalid_schema("recursion maximum must be positive"));
            }
            Ok(())
        }
        Some("well-founded") => {
            validate_map_fields(value, &["kind", "measure"], &[], "recursion bound")?;
            validate_expression(value.get("measure").unwrap())
        }
        _ => Err(invalid_schema("recursion bound has invalid kind")),
    }
}

fn validate_policy_obligation(value: &Value, node_kind: &str) -> Result<()> {
    validate_map_fields(
        value,
        &["category", "effective_rule", "value", "sources"],
        &[],
        "policy obligation",
    )?;
    let category = text_field(value, "category")
        .ok_or_else(|| invalid_schema("policy obligation category must be text"))?;
    let expected_kind = match category {
        "requirement" => "requirement",
        "evidence" => "verification",
        "limit" => "limit",
        _ => return Err(invalid_schema("invalid policy obligation category")),
    };
    if node_kind != expected_kind {
        return Err(invalid_schema(
            "policy obligation category does not match its node kind",
        ));
    }
    require_unsigned(value, "effective_rule", "policy obligation")?;
    crate::policy::validate_obligation_value(category, value.get("value").unwrap())
        .map_err(|error| invalid_schema(error.message))?;
    let sources = require_array(value, "sources", "policy obligation")?;
    if sources.is_empty() {
        return Err(invalid_schema(
            "policy obligation sources must be non-empty",
        ));
    }
    for source in sources {
        validate_map_fields(
            source,
            &["layer", "policy", "rule"],
            &[],
            "policy obligation source",
        )?;
        require_one_of(
            source,
            "layer",
            &["organization", "team", "repository", "user"],
            "policy obligation source",
        )?;
        require_symbol(source, "policy", "policy obligation source")?;
        required_ref(source, "rule")?;
    }
    Ok(())
}

fn capability_node_fields(value: &Value) -> Result<()> {
    validate_map_fields(
        value,
        &["id", "kind"],
        &[
            "goal",
            "request",
            "capability",
            "resource",
            "resources",
            "source_clauses",
            "policy",
            "waiver",
            "gap",
            "payload",
        ],
        "capability node",
    )?;
    required_ref(value, "id")?;
    require_one_of(
        value,
        "kind",
        &[
            "request", "grant", "denial", "resource", "decision", "waiver",
        ],
        "capability node",
    )?;
    if value.get("goal").is_some() {
        require_symbol(value, "goal", "capability node")?;
    }
    if let Some(request) = value.get("request") {
        validate_map_fields(request, &["goal", "effect"], &[], "capability request")?;
        require_symbol(request, "goal", "capability request")?;
        validate_effect(request.get("effect").unwrap(), "capability request effect")?;
    }
    if let Some(capability) = value.get("capability") {
        validate_map_fields(
            capability,
            &["effect", "scope", "decision", "sources"],
            &[],
            "capability",
        )?;
        require_one_of(
            capability,
            "decision",
            &["allow", "deny", "unresolved"],
            "capability",
        )?;
        validate_effect(capability.get("effect").unwrap(), "capability effect")?;
        validate_bhcp_value(capability.get("scope").unwrap(), "capability scope")?;
        validate_nonempty_ref_array(capability, "sources", "capability")?;
    }
    if let Some(resource) = value.get("resource") {
        validate_map_fields(
            resource,
            &["goal", "name", "type"],
            &[],
            "capability resource",
        )?;
        require_symbol(resource, "goal", "capability resource")?;
        require_text_value(resource, "name", "capability resource")?;
        validate_type(resource.get("type").unwrap(), "capability resource type")?;
    }
    validate_optional_nonempty_ref_array(value, "resources", "capability node")?;
    validate_optional_nonempty_ref_array(value, "source_clauses", "capability node")?;
    if let Some(policy) = value.get("policy") {
        validate_capability_policy(policy)?;
    }
    if let Some(waiver) = value.get("waiver") {
        validate_capability_waiver(waiver)?;
    }
    if let Some(gap) = value.get("gap") {
        validate_map_fields(gap, &["kind", "required"], &[], "capability gap")?;
        require_one_of(
            gap,
            "kind",
            &["unsafe", "foreign", "unsupported"],
            "capability gap",
        )?;
        if gap.get("required") != Some(&Value::Bool(true)) {
            return Err(invalid_schema("capability gap must be required"));
        }
    }
    if let Some(payload) = value.get("payload") {
        validate_bhcp_value(payload, "capability payload")?;
    }
    if text_field(value, "kind") == Some("waiver") && value.get("waiver").is_none() {
        return Err(invalid_schema("waiver node requires waiver detail"));
    }
    Ok(())
}

fn validate_capability_policy(value: &Value) -> Result<()> {
    validate_map_fields(
        value,
        &["category", "effective_rule", "value", "sources"],
        &[],
        "capability policy",
    )?;
    require_one_of(
        value,
        "category",
        &["capability", "prohibition"],
        "capability policy",
    )?;
    require_unsigned(value, "effective_rule", "capability policy")?;
    let policy_value = value.get("value").unwrap();
    validate_map_fields(
        policy_value,
        &["effect"],
        &["scope"],
        "capability policy value",
    )?;
    require_symbol(policy_value, "effect", "capability policy value")?;
    if let Some(scope) = policy_value.get("scope") {
        validate_map_fields(
            scope,
            &[],
            &["goals", "resources", "operations"],
            "capability policy scope",
        )?;
        for field in ["goals", "resources", "operations"] {
            validate_nonempty_symbol_array(scope, field, "capability policy scope")?;
        }
    }
    let sources = require_array(value, "sources", "capability policy")?;
    if sources.is_empty() {
        return Err(invalid_schema(
            "capability policy sources must be non-empty",
        ));
    }
    for source in sources {
        validate_map_fields(
            source,
            &["layer", "policy", "rule"],
            &[],
            "capability policy source",
        )?;
        require_one_of(
            source,
            "layer",
            &["organization", "team", "repository", "user"],
            "capability policy source",
        )?;
        require_symbol(source, "policy", "capability policy source")?;
        required_ref(source, "rule")?;
    }
    Ok(())
}

fn validate_capability_waiver(value: &Value) -> Result<()> {
    validate_map_fields(
        value,
        &["waiver", "targets", "decision_time"],
        &[],
        "capability waiver",
    )?;
    validate_content_reference(value.get("waiver").unwrap())?;
    let targets = require_array(value, "targets", "capability waiver")?;
    if targets.is_empty() {
        return Err(invalid_schema(
            "capability waiver targets must be non-empty",
        ));
    }
    for target in targets {
        let Value::Array(parts) = target else {
            return Err(invalid_schema("waiver target must be a two-item array"));
        };
        let [Value::Text(policy), Value::Text(rule)] = parts.as_slice() else {
            return Err(invalid_schema("waiver target must be a two-item array"));
        };
        if !crate::model::is_symbol(policy) || rule.is_empty() || rule.len() > 128 {
            return Err(invalid_schema("waiver target is invalid"));
        }
    }
    validate_timestamp(
        value.get("decision_time").unwrap(),
        "capability waiver decision_time",
    )
}

fn state_node_fields(value: &Value) -> Result<()> {
    validate_map_fields(
        value,
        &["id", "kind"],
        &["cell", "handle", "payload"],
        "state node",
    )?;
    required_ref(value, "id")?;
    require_one_of(
        value,
        "kind",
        &[
            "resource",
            "cell",
            "borrow",
            "ownership",
            "transition",
            "invariant",
        ],
        "state node",
    )?;
    if let Some(cell) = value.get("cell") {
        validate_map_fields(
            cell,
            &["key", "type", "state", "atomic_version"],
            &[],
            "state cell",
        )?;
        require_text_value(cell, "key", "state cell")?;
        validate_type(cell.get("type").unwrap(), "state cell type")?;
        require_unsigned(cell, "atomic_version", "state cell")?;
        validate_state_value(cell.get("state").unwrap())?;
    }
    if let Some(handle) = value.get("handle") {
        validate_type(handle, "state handle")?;
        let Value::Array(parts) = handle else {
            unreachable!("checked type is an array")
        };
        if parts.first() != Some(&Value::Text("handle".to_owned())) {
            return Err(invalid_schema("state handle must be a handle type"));
        }
    }
    if let Some(payload) = value.get("payload") {
        validate_bhcp_value(payload, "state payload")?;
    }
    Ok(())
}

fn execution_node_fields(value: &Value) -> Result<()> {
    validate_map_fields(
        value,
        &[
            "id",
            "kind",
            "executor",
            "inputs",
            "outputs",
            "effects",
            "capability_decisions",
            "budgets",
            "expected_evidence",
            "dependencies",
        ],
        &[],
        "execution node",
    )?;
    required_ref(value, "id")?;
    require_one_of(
        value,
        "kind",
        &[
            "goal",
            "verifier",
            "condition",
            "state",
            "approval",
            "executor",
        ],
        "execution node",
    )?;
    require_symbol(value, "executor", "execution node")?;
    let Value::Map(inputs) = value
        .get("inputs")
        .ok_or_else(|| invalid_schema("execution node requires inputs"))?
    else {
        return Err(invalid_schema("execution inputs must be a map"));
    };
    for (_, input) in inputs {
        validate_map_fields(input, &["ref", "type"], &[], "typed reference")?;
        required_ref(input, "ref")?;
        validate_type(input.get("type").unwrap(), "typed reference type")?;
    }
    let Some(Value::Map(outputs)) = value.get("outputs") else {
        return Err(invalid_schema("execution outputs must be a map"));
    };
    for (_, output) in outputs {
        validate_type(output, "execution output type")?;
    }
    validate_effect_row(value.get("effects").unwrap(), "execution effects")?;
    for field in ["capability_decisions", "dependencies"] {
        validate_ref_array(value, field, "execution node")?;
    }
    for budget in require_array(value, "budgets", "execution node")? {
        validate_budget(budget)?;
    }
    for class in require_array(value, "expected_evidence", "execution node")? {
        let Value::Text(class) = class else {
            return Err(invalid_schema("expected evidence class must be text"));
        };
        validate_evidence_class(class)?;
    }
    Ok(())
}

fn graph_edge_fields(value: &Value) -> Result<()> {
    validate_map_fields(value, &["id", "from", "to", "kind"], &[], "graph edge")?;
    for field in ["id", "from", "to"] {
        required_ref(value, field)?;
    }
    require_text_value(value, "kind", "graph edge")
}

fn execution_edge_fields(value: &Value) -> Result<()> {
    validate_map_fields(
        value,
        &["id", "from", "to", "kind"],
        &["binding"],
        "execution edge",
    )?;
    for field in ["id", "from", "to"] {
        required_ref(value, field)?;
    }
    require_one_of(
        value,
        "kind",
        &["data", "control", "effect", "state", "evidence"],
        "execution edge",
    )?;
    if value.get("binding").is_some() {
        require_text_value(value, "binding", "execution edge")?;
    }
    Ok(())
}

fn validate_state_transition(value: &Value) -> Result<()> {
    validate_map_fields(
        value,
        &[
            "id",
            "cell",
            "from_version",
            "to_version",
            "result",
            "atomic",
        ],
        &[],
        "state transition",
    )?;
    required_ref(value, "id")?;
    required_ref(value, "cell")?;
    require_unsigned(value, "from_version", "state transition")?;
    require_unsigned(value, "to_version", "state transition")?;
    if value.get("atomic") != Some(&Value::Bool(true)) {
        return Err(invalid_schema("state transition atomic must equal true"));
    }
    validate_execution_result(value.get("result").unwrap(), "state transition result")
}

fn validate_evidence_shape(value: &Value) -> Result<()> {
    validate_content_reference(
        value
            .get("execution_graph")
            .ok_or_else(|| invalid_schema("evidence graph requires execution_graph"))?,
    )?;
    for claim in require_array(value, "claims", "evidence graph")? {
        validate_map_fields(
            claim,
            &[
                "id",
                "obligation",
                "polarity",
                "subject",
                "predicate",
                "status",
            ],
            &["execution_instance"],
            "evidence claim",
        )?;
        required_ref(claim, "id")?;
        if claim.get("execution_instance").is_some() {
            required_ref(claim, "execution_instance")?;
        }
        required_ref(claim, "obligation")?;
        require_one_of(
            claim,
            "polarity",
            &["supports", "refutes"],
            "evidence claim",
        )?;
        require_symbol(claim, "predicate", "evidence claim")?;
        require_one_of(
            claim,
            "status",
            &["accepted", "rejected", "stale", "unresolved"],
            "evidence claim",
        )?;
        validate_content_reference(claim.get("subject").unwrap())?;
    }
    for item in require_array(value, "items", "evidence graph")? {
        validate_map_fields(
            item,
            &[
                "id",
                "class",
                "verifier",
                "verifier_artifact",
                "payload",
                "claims",
                "produced_at",
                "provenance",
                "trust",
            ],
            &["execution_instance", "fresh_until"],
            "evidence item",
        )?;
        required_ref(item, "id")?;
        if item.get("execution_instance").is_some() {
            required_ref(item, "execution_instance")?;
        }
        let evidence_class = text_field(item, "class")
            .ok_or_else(|| invalid_schema("evidence item class must be text"))?;
        validate_evidence_class(evidence_class)?;
        require_symbol(item, "verifier", "evidence item")?;
        validate_content_reference(item.get("verifier_artifact").unwrap())?;
        validate_content_reference(item.get("payload").unwrap())?;
        validate_nonempty_ref_array(item, "claims", "evidence item")?;
        validate_timestamp(item.get("produced_at").unwrap(), "evidence produced_at")?;
        if let Some(fresh_until) = item.get("fresh_until") {
            validate_timestamp(fresh_until, "evidence fresh_until")?;
        }
        validate_provenance(item.get("provenance").unwrap())?;
        for trust in require_array(item, "trust", "evidence item")? {
            let Value::Text(trust) = trust else {
                return Err(invalid_schema("evidence trust must be a symbol-id"));
            };
            if !crate::model::is_symbol(trust) {
                return Err(invalid_schema("evidence trust must be a symbol-id"));
            }
        }
    }
    for gap in require_array(value, "gaps", "evidence graph")? {
        validate_map_fields(
            gap,
            &["id", "kind", "obligations", "reason", "required"],
            &["execution_instance"],
            "evidence gap",
        )?;
        required_ref(gap, "id")?;
        if gap.get("execution_instance").is_some() {
            required_ref(gap, "execution_instance")?;
        }
        require_text_value(gap, "kind", "evidence gap")?;
        let gap_kind = text_field(gap, "kind").unwrap();
        if !["unsafe", "foreign", "missing", "stale", "unsupported"].contains(&gap_kind)
            && !crate::model::is_symbol(gap_kind)
        {
            return Err(invalid_schema("invalid evidence gap kind"));
        }
        validate_ref_array(gap, "obligations", "evidence gap")?;
        validate_reason(gap.get("reason").unwrap())?;
        if !matches!(gap.get("required"), Some(Value::Bool(_))) {
            return Err(invalid_schema("evidence gap required must be bool"));
        }
    }
    for edge in require_array(value, "edges", "evidence graph")? {
        validate_map_fields(edge, &["id", "from", "to", "kind"], &[], "evidence edge")?;
        for field in ["id", "from", "to"] {
            required_ref(edge, field)?;
        }
        require_one_of(
            edge,
            "kind",
            &[
                "supports",
                "refutes",
                "derives-from",
                "produces",
                "depends-on",
            ],
            "evidence edge",
        )?;
    }
    let Some(Value::Map(statuses)) = value.get("obligation_status") else {
        return Err(invalid_schema("evidence obligation_status must be a map"));
    };
    for (id, status) in statuses {
        validate_ref(id, "obligation status")?;
        let Value::Text(status) = status else {
            return Err(invalid_schema("obligation status must be text"));
        };
        if !["discharged", "refuted", "unresolved"].contains(&status.as_str()) {
            return Err(invalid_schema("invalid obligation status"));
        }
    }
    if let Some(policy) = value.get("policy_obligations") {
        let Value::Array(policy) = policy else {
            return Err(invalid_schema("policy_obligations must be an array"));
        };
        if policy.is_empty() {
            return Err(invalid_schema("policy_obligations must be non-empty"));
        }
        for obligation in policy {
            validate_map_fields(
                obligation,
                &[
                    "id",
                    "symbol",
                    "classes",
                    "minimum",
                    "effective_rule",
                    "sources",
                ],
                &[],
                "policy evidence obligation",
            )?;
            required_ref(obligation, "id")?;
            require_symbol(obligation, "symbol", "policy evidence obligation")?;
            let classes = require_array(obligation, "classes", "policy evidence obligation")?;
            if classes.is_empty() {
                return Err(invalid_schema("policy evidence classes must be non-empty"));
            }
            for class in classes {
                let Value::Text(class) = class else {
                    return Err(invalid_schema("policy evidence class must be text"));
                };
                validate_evidence_class(class)?;
            }
            require_unsigned(obligation, "minimum", "policy evidence obligation")?;
            if obligation.get("minimum") == Some(&Value::Integer(0)) {
                return Err(invalid_schema("policy evidence minimum must be positive"));
            }
            require_unsigned(obligation, "effective_rule", "policy evidence obligation")?;
            let sources = require_array(obligation, "sources", "policy evidence obligation")?;
            if sources.is_empty() {
                return Err(invalid_schema("policy evidence sources must be non-empty"));
            }
            for source in sources {
                validate_map_fields(
                    source,
                    &["layer", "policy", "rule"],
                    &[],
                    "policy evidence source",
                )?;
                require_one_of(
                    source,
                    "layer",
                    &["organization", "team", "repository", "user"],
                    "policy evidence source",
                )?;
                require_symbol(source, "policy", "policy evidence source")?;
                required_ref(source, "rule")?;
            }
        }
    }
    Ok(())
}

fn validate_content_reference(value: &Value) -> Result<()> {
    validate_map_fields(
        value,
        &["media_type", "size", "digests"],
        &["locations"],
        "content reference",
    )?;
    require_text_value(value, "media_type", "content reference")?;
    require_unsigned(value, "size", "content reference")?;
    let digests = require_array(value, "digests", "content reference")?;
    if digests.is_empty() {
        return Err(invalid_schema(
            "content reference digests must be non-empty",
        ));
    }
    for digest in digests {
        parse_hash(digest)?
            .validate()
            .map_err(|error| invalid_schema(error.message))?;
    }
    if let Some(locations) = value.get("locations") {
        let Value::Array(locations) = locations else {
            return Err(invalid_schema(
                "content reference locations must be an array",
            ));
        };
        if locations
            .iter()
            .any(|location| !matches!(location, Value::Text(_)))
        {
            return Err(invalid_schema("content reference location must be text"));
        }
    }
    Ok(())
}

fn validate_provenance(value: &Value) -> Result<()> {
    validate_map_fields(
        value,
        &["producer", "created_at"],
        &["source", "parents", "annotations"],
        "provenance",
    )?;
    require_symbol(value, "producer", "provenance")?;
    validate_timestamp(value.get("created_at").unwrap(), "provenance created_at")?;
    if let Some(source) = value.get("source") {
        validate_content_reference(source)?;
    }
    if let Some(Value::Array(parents)) = value.get("parents") {
        for parent in parents {
            parse_hash(parent)?
                .validate()
                .map_err(|error| invalid_schema(error.message))?;
        }
    } else if value.get("parents").is_some() {
        return Err(invalid_schema("provenance parents must be an array"));
    }
    if let Some(annotations) = value.get("annotations") {
        let Value::Map(annotations) = annotations else {
            return Err(invalid_schema("provenance annotations must be a map"));
        };
        for (_, annotation) in annotations {
            validate_bhcp_value(annotation, "provenance annotation")?;
        }
    }
    Ok(())
}

fn validate_authorization(value: &Value) -> Result<()> {
    validate_map_fields(
        value,
        &["scheme", "issuer", "subject", "signature"],
        &["expires_at"],
        "authorization",
    )?;
    require_symbol(value, "scheme", "authorization")?;
    require_text_value(value, "issuer", "authorization")?;
    validate_content_reference(value.get("subject").unwrap())?;
    if !matches!(value.get("signature"), Some(Value::Bytes(_))) {
        return Err(invalid_schema("authorization signature must be bytes"));
    }
    if let Some(expires) = value.get("expires_at") {
        validate_timestamp(expires, "authorization expires_at")?;
    }
    Ok(())
}

fn validate_reason(value: &Value) -> Result<()> {
    validate_map_fields(value, &["code", "message"], &["details"], "reason")?;
    require_symbol(value, "code", "reason")?;
    require_text_value(value, "message", "reason")?;
    if let Some(details) = value.get("details") {
        validate_bhcp_value(details, "reason details")?;
    }
    Ok(())
}

fn validate_type(value: &Value, context: &str) -> Result<()> {
    crate::typecheck::CheckedType::from_value(value)
        .map(|_| ())
        .map_err(|error| invalid_schema(format!("{context} is invalid: {}", error.message)))
}

fn validate_bhcp_value(value: &Value, context: &str) -> Result<()> {
    match value {
        Value::Bool(_) | Value::Text(_) | Value::Bytes(_) => Ok(()),
        Value::Array(items) => {
            match items.as_slice() {
                [Value::Text(kind)] if kind == "unit" => return Ok(()),
                [Value::Text(kind), Value::Integer(_)] if kind == "integer" => return Ok(()),
                [
                    Value::Text(kind),
                    Value::Integer(_),
                    Value::Integer(denominator),
                ] if kind == "rational" && *denominator > 0 => {
                    return Ok(());
                }
                [Value::Text(kind), Value::Integer(_), Value::Integer(_)] if kind == "decimal" => {
                    return Ok(());
                }
                [Value::Text(kind), Value::Text(format), Value::Bytes(bytes)]
                    if kind == "machine-float"
                        && matches!(
                            (format.as_str(), bytes.len()),
                            ("binary16", 2) | ("binary32", 4) | ("binary64", 8) | ("binary128", 16)
                        ) =>
                {
                    return Ok(());
                }
                [Value::Text(kind), Value::Text(_), payload] if kind == "variant" => {
                    return validate_bhcp_value(payload, context);
                }
                _ => {}
            }
            for item in items {
                validate_bhcp_value(item, context)?;
            }
            Ok(())
        }
        Value::Map(entries) => {
            for (_, item) in entries {
                validate_bhcp_value(item, context)?;
            }
            Ok(())
        }
        Value::Null | Value::Integer(_) | Value::Tag(_, _) => Err(invalid_schema(format!(
            "{context} is not a canonical BHCP value"
        ))),
    }
}

fn validate_effect(value: &Value, context: &str) -> Result<()> {
    validate_map_fields(value, &["id"], &["resource", "parameters"], context)?;
    require_symbol(value, "id", context)?;
    if value.get("resource").is_some() {
        required_ref(value, "resource")?;
    }
    if let Some(parameters) = value.get("parameters") {
        let Value::Array(parameters) = parameters else {
            return Err(invalid_schema(format!(
                "{context} parameters must be an array"
            )));
        };
        for parameter in parameters {
            validate_bhcp_value(parameter, "effect parameter")?;
        }
    }
    Ok(())
}

fn validate_effect_row(value: &Value, context: &str) -> Result<()> {
    validate_map_fields(value, &["effects"], &["row_variable"], context)?;
    for effect in require_array(value, "effects", context)? {
        validate_effect(effect, "effect row member")?;
    }
    if value.get("row_variable").is_some() {
        require_text_value(value, "row_variable", context)?;
    }
    Ok(())
}

fn validate_budget(value: &Value) -> Result<()> {
    validate_map_fields(
        value,
        &["dimension", "limit", "allocation"],
        &["children"],
        "budget",
    )?;
    require_symbol(value, "dimension", "budget")?;
    validate_exact_number(value.get("limit").unwrap(), "budget limit")?;
    require_one_of(value, "allocation", &["shared", "explicit"], "budget")?;
    if let Some(children) = value.get("children") {
        let Value::Map(children) = children else {
            return Err(invalid_schema("budget children must be a map"));
        };
        for (child, allocation) in children {
            validate_ref(child, "budget child")?;
            validate_exact_number(allocation, "budget child allocation")?;
        }
    }
    Ok(())
}

fn validate_exact_number(value: &Value, context: &str) -> Result<()> {
    let Value::Array(parts) = value else {
        return Err(invalid_schema(format!("{context} must be an exact number")));
    };
    match parts.as_slice() {
        [Value::Text(kind), Value::Integer(_)] if kind == "integer" => Ok(()),
        [
            Value::Text(kind),
            Value::Integer(_),
            Value::Integer(denominator),
        ] if kind == "rational" && *denominator > 0 => Ok(()),
        [Value::Text(kind), Value::Integer(_), Value::Integer(_)] if kind == "decimal" => Ok(()),
        _ => Err(invalid_schema(format!("{context} must be an exact number"))),
    }
}

fn validate_state_value(value: &Value) -> Result<()> {
    let Value::Array(parts) = value else {
        return Err(invalid_schema("state cell state must be an array"));
    };
    match parts.as_slice() {
        [Value::Text(state)] if state == "empty" => Ok(()),
        [
            Value::Text(state),
            captured,
            references,
            provenance,
            timestamp,
            expression,
        ] if state == "captured" => {
            validate_bhcp_value(captured, "captured state value")?;
            let Value::Array(references) = references else {
                return Err(invalid_schema("captured state references must be an array"));
            };
            if references.is_empty() {
                return Err(invalid_schema(
                    "captured state references must be non-empty",
                ));
            }
            for reference in references {
                validate_ref_value(reference, "captured state reference")?;
            }
            validate_provenance(provenance)?;
            validate_timestamp(timestamp, "captured state timestamp")?;
            validate_expression(expression)
        }
        _ => Err(invalid_schema("invalid state cell state")),
    }
}

fn validate_expression(value: &Value) -> Result<()> {
    validate_map_fields(value, &["id", "type", "form"], &[], "expression")?;
    required_ref(value, "id")?;
    validate_type(value.get("type").unwrap(), "expression type")?;
    let Some(Value::Array(form)) = value.get("form") else {
        return Err(invalid_schema("expression form must be an array"));
    };
    let Some(Value::Text(kind)) = form.first() else {
        return Err(invalid_schema("expression form requires a text kind"));
    };
    match (kind.as_str(), form.as_slice()) {
        ("literal", [_, literal]) => validate_bhcp_value(literal, "literal expression"),
        ("reference", [_, reference]) => validate_ref_value(reference, "expression reference"),
        ("record", [_, Value::Map(fields)]) => {
            for (_, field) in fields {
                validate_expression(field)?;
            }
            Ok(())
        }
        ("tuple", [_, Value::Array(items)]) => validate_expressions(items),
        ("variant", [_, Value::Text(_), payload]) => validate_expression(payload),
        ("collection", [_, Value::Text(collection), Value::Array(items)])
            if matches!(collection.as_str(), "list" | "set") =>
        {
            validate_expressions(items)
        }
        ("map", [_, Value::Array(entries)]) => {
            for entry in entries {
                let Value::Array(pair) = entry else {
                    return Err(invalid_schema("expression map entry must be a pair"));
                };
                if pair.len() != 2 {
                    return Err(invalid_schema("expression map entry must be a pair"));
                }
                validate_expression(&pair[0])?;
                validate_expression(&pair[1])?;
            }
            Ok(())
        }
        ("select", [_, subject, selector]) => {
            validate_expression(subject)?;
            match selector {
                Value::Text(_) => Ok(()),
                Value::Integer(number) if *number >= 0 => Ok(()),
                _ => Err(invalid_schema("expression selector must be text or uint")),
            }
        }
        ("unary", [_, Value::Text(_), operand]) => validate_expression(operand),
        ("binary", [_, Value::Text(_), left, right]) => {
            validate_expression(left)?;
            validate_expression(right)
        }
        ("if", [_, condition, consequent, alternate]) => {
            validate_expression(condition)?;
            validate_expression(consequent)?;
            validate_expression(alternate)
        }
        ("let", [_, binding, bound, body]) => {
            validate_binding(binding)?;
            validate_expression(bound)?;
            validate_expression(body)
        }
        ("match", [_, subject, Value::Array(arms)]) if !arms.is_empty() => {
            validate_expression(subject)?;
            for arm in arms {
                validate_match_arm(arm)?;
            }
            Ok(())
        }
        ("call", [_, Value::Text(symbol), Value::Array(arguments)])
            if crate::model::is_symbol(symbol) =>
        {
            validate_expressions(arguments)
        }
        ("quantify", [_, Value::Text(quantifier), binding, domain, predicate])
            if matches!(quantifier.as_str(), "forall" | "exists") =>
        {
            validate_binding(binding)?;
            validate_expression(domain)?;
            validate_expression(predicate)
        }
        (
            "quantify",
            [
                _,
                Value::Text(quantifier),
                binding,
                domain,
                predicate,
                verifier,
            ],
        ) if matches!(quantifier.as_str(), "forall" | "exists") => {
            validate_binding(binding)?;
            validate_expression(domain)?;
            validate_expression(predicate)?;
            validate_verifier_binding(verifier)
        }
        ("cast-dynamic", [_, source, target]) => {
            validate_expression(source)?;
            validate_type(target, "dynamic cast target")
        }
        _ => Err(invalid_schema("invalid expression form")),
    }
}

fn validate_expressions(values: &[Value]) -> Result<()> {
    for value in values {
        validate_expression(value)?;
    }
    Ok(())
}

fn validate_binding(value: &Value) -> Result<()> {
    validate_map_fields(value, &["id", "type"], &["name"], "binding")?;
    required_ref(value, "id")?;
    validate_type(value.get("type").unwrap(), "binding type")?;
    if value.get("name").is_some() {
        require_text_value(value, "name", "binding")?;
    }
    Ok(())
}

fn validate_match_arm(value: &Value) -> Result<()> {
    let Value::Array(parts) = value else {
        return Err(invalid_schema("match arm must be an array"));
    };
    match parts.as_slice() {
        [pattern, body] => {
            validate_pattern(pattern)?;
            validate_expression(body)
        }
        [pattern, guard, body] => {
            validate_pattern(pattern)?;
            validate_expression(guard)?;
            validate_expression(body)
        }
        _ => Err(invalid_schema("match arm has invalid arity")),
    }
}

fn validate_pattern(value: &Value) -> Result<()> {
    let Value::Array(parts) = value else {
        return Err(invalid_schema("pattern must be an array"));
    };
    match parts.as_slice() {
        [Value::Text(kind)] if kind == "wildcard" => Ok(()),
        [Value::Text(kind), literal] if kind == "literal" => {
            validate_bhcp_value(literal, "literal pattern")
        }
        [Value::Text(kind), binding] if kind == "bind" => validate_binding(binding),
        [Value::Text(kind), Value::Text(_), Value::Array(patterns)] if kind == "variant" => {
            for pattern in patterns {
                validate_pattern(pattern)?;
            }
            Ok(())
        }
        [Value::Text(kind), Value::Array(patterns)] if kind == "tuple" => {
            for pattern in patterns {
                validate_pattern(pattern)?;
            }
            Ok(())
        }
        [Value::Text(kind), Value::Map(patterns)] if kind == "record" => {
            for (_, pattern) in patterns {
                validate_pattern(pattern)?;
            }
            Ok(())
        }
        _ => Err(invalid_schema("invalid pattern")),
    }
}

fn validate_verifier_binding(value: &Value) -> Result<()> {
    validate_map_fields(
        value,
        &["verifier", "input", "output"],
        &["configuration", "trust"],
        "verifier binding",
    )?;
    require_symbol(value, "verifier", "verifier binding")?;
    validate_type(value.get("input").unwrap(), "verifier input type")?;
    validate_evidence_type(value.get("output").unwrap(), "verifier output type")?;
    if let Some(configuration) = value.get("configuration") {
        validate_bhcp_value(configuration, "verifier configuration")?;
    }
    if let Some(trust) = value.get("trust") {
        let Value::Array(trust) = trust else {
            return Err(invalid_schema("verifier trust must be an array"));
        };
        for class in trust {
            let Value::Text(class) = class else {
                return Err(invalid_schema("verifier trust class must be text"));
            };
            validate_evidence_class(class)?;
        }
    }
    Ok(())
}

fn validate_execution_result(value: &Value, context: &str) -> Result<()> {
    let Value::Map(_) = value else {
        return Err(invalid_schema(format!("{context} must be a map")));
    };
    match text_field(value, "state") {
        Some("completed") => {
            validate_map_fields(value, &["state", "verdict"], &[], context)?;
            validate_verdict(value.get("verdict").unwrap())
        }
        Some("faulted") => {
            validate_map_fields(value, &["state", "fault"], &[], context)?;
            validate_operational_fault(value.get("fault").unwrap())
        }
        _ => Err(invalid_schema(format!("{context} has invalid state"))),
    }
}

fn validate_verdict(value: &Value) -> Result<()> {
    match text_field(value, "state") {
        Some("satisfied") => {
            validate_map_fields(value, &["state", "output", "evidence"], &[], "verdict")?;
            validate_bhcp_value(value.get("output").unwrap(), "verdict output")?;
            validate_nonempty_ref_array(value, "evidence", "verdict")
        }
        Some("refuted") => {
            validate_map_fields(value, &["state", "counter_evidence"], &[], "verdict")?;
            validate_nonempty_ref_array(value, "counter_evidence", "verdict")
        }
        Some("unresolved") => {
            validate_map_fields(
                value,
                &["state", "reason", "partial_evidence"],
                &[],
                "verdict",
            )?;
            validate_reason(value.get("reason").unwrap())?;
            validate_ref_array(value, "partial_evidence", "verdict")
        }
        _ => Err(invalid_schema("invalid verdict state")),
    }
}

fn validate_operational_fault(value: &Value) -> Result<()> {
    validate_map_fields(value, &["error", "trace"], &[], "operational fault")?;
    validate_reason(value.get("error").unwrap())?;
    for event in require_array(value, "trace", "operational fault")? {
        validate_map_fields(
            event,
            &["sequence", "node", "at", "kind"],
            &["payload"],
            "trace event",
        )?;
        require_unsigned(event, "sequence", "trace event")?;
        required_ref(event, "node")?;
        validate_timestamp(event.get("at").unwrap(), "trace event timestamp")?;
        require_symbol(event, "kind", "trace event")?;
        if let Some(payload) = event.get("payload") {
            validate_bhcp_value(payload, "trace payload")?;
        }
    }
    Ok(())
}

fn validate_map_fields(
    value: &Value,
    required: &[&str],
    optional: &[&str],
    context: &str,
) -> Result<()> {
    let Value::Map(entries) = value else {
        return Err(invalid_schema(format!("{context} must be a map")));
    };
    for field in required {
        if value.get(field).is_none() {
            return Err(invalid_schema(format!("{context} requires {field}")));
        }
    }
    let mut seen = BTreeSet::new();
    for (field, _) in entries {
        if !seen.insert(field) {
            return Err(duplicate(format!(
                "{context} contains duplicate field {field:?}"
            )));
        }
        if !required.contains(&field.as_str()) && !optional.contains(&field.as_str()) {
            return Err(invalid_schema(format!(
                "{context} contains unknown field {field:?}"
            )));
        }
    }
    Ok(())
}

fn require_array<'a>(value: &'a Value, field: &str, context: &str) -> Result<&'a [Value]> {
    match value.get(field) {
        Some(Value::Array(items)) => Ok(items),
        _ => Err(invalid_schema(format!(
            "{context} {field} must be an array"
        ))),
    }
}

fn require_text_value(value: &Value, field: &str, context: &str) -> Result<()> {
    if matches!(value.get(field), Some(Value::Text(_))) {
        Ok(())
    } else {
        Err(invalid_schema(format!("{context} {field} must be text")))
    }
}

fn require_symbol(value: &Value, field: &str, context: &str) -> Result<()> {
    let Some(Value::Text(symbol)) = value.get(field) else {
        return Err(invalid_schema(format!(
            "{context} {field} must be a symbol-id"
        )));
    };
    if crate::model::is_symbol(symbol) {
        Ok(())
    } else {
        Err(invalid_schema(format!(
            "{context} {field} must be a symbol-id"
        )))
    }
}

fn require_unsigned(value: &Value, field: &str, context: &str) -> Result<()> {
    if matches!(value.get(field), Some(Value::Integer(number)) if *number >= 0) {
        Ok(())
    } else {
        Err(invalid_schema(format!(
            "{context} {field} must be unsigned"
        )))
    }
}

fn require_one_of(value: &Value, field: &str, allowed: &[&str], context: &str) -> Result<()> {
    match text_field(value, field) {
        Some(actual) if allowed.contains(&actual) => Ok(()),
        _ => Err(invalid_schema(format!(
            "{context} {field} has an invalid value"
        ))),
    }
}

fn validate_ref(value: &str, context: &str) -> Result<()> {
    if value.is_empty() || value.len() > 128 {
        Err(invalid_schema(format!(
            "{context} must be a 1..128 byte ref-id"
        )))
    } else {
        Ok(())
    }
}

fn validate_ref_value(value: &Value, context: &str) -> Result<()> {
    let Value::Text(reference) = value else {
        return Err(invalid_schema(format!("{context} must be a ref-id")));
    };
    validate_ref(reference, context)
}

fn validate_ref_array(value: &Value, field: &str, context: &str) -> Result<()> {
    for reference in require_array(value, field, context)? {
        validate_ref_value(reference, context)?;
    }
    Ok(())
}

fn validate_nonempty_ref_array(value: &Value, field: &str, context: &str) -> Result<()> {
    let values = require_array(value, field, context)?;
    if values.is_empty() {
        return Err(invalid_schema(format!(
            "{context} {field} must be non-empty"
        )));
    }
    for reference in values {
        validate_ref_value(reference, context)?;
    }
    Ok(())
}

fn validate_optional_ref_array(value: &Value, field: &str, context: &str) -> Result<()> {
    if value.get(field).is_some() {
        validate_ref_array(value, field, context)
    } else {
        Ok(())
    }
}

fn validate_optional_nonempty_ref_array(value: &Value, field: &str, context: &str) -> Result<()> {
    if value.get(field).is_some() {
        validate_nonempty_ref_array(value, field, context)
    } else {
        Ok(())
    }
}

fn validate_nonempty_symbol_array(value: &Value, field: &str, context: &str) -> Result<()> {
    let Some(Value::Array(symbols)) = value.get(field) else {
        return if value.get(field).is_none() {
            Ok(())
        } else {
            Err(invalid_schema(format!(
                "{context} {field} must be an array"
            )))
        };
    };
    if symbols.is_empty() {
        return Err(invalid_schema(format!(
            "{context} {field} must be non-empty"
        )));
    }
    for symbol in symbols {
        let Value::Text(symbol) = symbol else {
            return Err(invalid_schema(format!(
                "{context} {field} must contain symbol IDs"
            )));
        };
        if !crate::model::is_symbol(symbol) {
            return Err(invalid_schema(format!(
                "{context} {field} must contain symbol IDs"
            )));
        }
    }
    Ok(())
}

fn validate_timestamp(value: &Value, context: &str) -> Result<()> {
    if matches!(value, Value::Tag(0, inner) if matches!(inner.as_ref(), Value::Text(_))) {
        Ok(())
    } else {
        Err(invalid_schema(format!(
            "{context} must be tagged timestamp text"
        )))
    }
}

fn validate_evidence_class(value: &str) -> Result<()> {
    if [
        "formal",
        "static",
        "empirical",
        "statistical",
        "model-judged",
        "human-approved",
        "unresolved",
    ]
    .contains(&value)
        || crate::model::is_symbol(value)
    {
        Ok(())
    } else {
        Err(invalid_schema("invalid evidence class"))
    }
}

fn validate_evidence_type(value: &Value, context: &str) -> Result<()> {
    validate_type(value, context)?;
    let Value::Array(parts) = value else {
        return Err(invalid_schema(format!("{context} must be evidence-type")));
    };
    let [Value::Text(kind), Value::Array(classes)] = parts.as_slice() else {
        return Err(invalid_schema(format!("{context} must be evidence-type")));
    };
    if kind != "evidence" {
        return Err(invalid_schema(format!("{context} must be evidence-type")));
    }
    for class in classes {
        let Value::Text(class) = class else {
            return Err(invalid_schema(format!(
                "{context} evidence class must be text"
            )));
        };
        validate_evidence_class(class)?;
    }
    Ok(())
}

fn reject_unknown_graph_fields(value: &Value, kind: GraphKind) -> Result<()> {
    let common = [
        "version",
        "features",
        "semantic_id",
        "artifact_id",
        "provenance",
        "authorization",
        "kind",
    ];
    let root_extra: &[&str] = match kind {
        GraphKind::Obligation | GraphKind::Capability => &["semantic_ir", "nodes", "edges"],
        GraphKind::State => &["semantic_ir", "nodes", "edges", "transitions"],
        GraphKind::Execution => &["semantic_ir", "nodes", "edges", "entrypoints"],
        GraphKind::Evidence => &[
            "semantic_ir",
            "execution_graph",
            "claims",
            "items",
            "gaps",
            "edges",
            "obligation_status",
            "policy_obligations",
        ],
    };
    let Value::Map(entries) = value else {
        return Err(invalid_schema("graph root must be a map"));
    };
    for (field, _) in entries {
        if !common.contains(&field.as_str()) && !root_extra.contains(&field.as_str()) {
            return Err(invalid_schema(format!(
                "graph root contains unknown field {field:?}"
            )));
        }
    }
    Ok(())
}

fn normalize_document(value: &mut Value, kind: GraphKind) -> Result<()> {
    normalize_header(value)?;
    normalize_content_reference(map_field_mut(value, "semantic_ir").unwrap())?;
    match kind {
        GraphKind::Obligation => {
            for node in array_field_mut(value, "nodes")? {
                sort_optional_set_field(node, "evidence")?;
                sort_optional_set_field(node, "goals")?;
                sort_optional_set_field(node, "source_clauses")?;
                if let Some(policy) = map_field_mut(node, "policy") {
                    sort_set_field(policy, "sources")?;
                    if let Some(policy_value) = map_field_mut(policy, "value") {
                        sort_optional_set_field(policy_value, "classes")?;
                        if let Some(scope) = map_field_mut(policy_value, "scope") {
                            for field in ["goals", "resources", "operations"] {
                                sort_optional_set_field(scope, field)?;
                            }
                        }
                    }
                }
            }
            sort_set_field(value, "nodes")?;
            sort_set_field(value, "edges")
        }
        GraphKind::Capability => {
            for node in array_field_mut(value, "nodes")? {
                if let Some(request) = map_field_mut(node, "request") {
                    normalize_effect(map_field_mut(request, "effect").unwrap())?;
                }
                if let Some(capability) = map_field_mut(node, "capability") {
                    normalize_effect(map_field_mut(capability, "effect").unwrap())?;
                    sort_set_field(capability, "sources")?;
                }
                if let Some(resource) = map_field_mut(node, "resource") {
                    normalize_type_field(resource, "type")?;
                }
                sort_optional_set_field(node, "resources")?;
                sort_optional_set_field(node, "source_clauses")?;
                if let Some(policy) = map_field_mut(node, "policy") {
                    sort_set_field(policy, "sources")?;
                    if let Some(policy_value) = map_field_mut(policy, "value")
                        && let Some(scope) = map_field_mut(policy_value, "scope")
                    {
                        for field in ["goals", "resources", "operations"] {
                            sort_optional_set_field(scope, field)?;
                        }
                    }
                }
                if let Some(waiver) = map_field_mut(node, "waiver") {
                    normalize_content_reference(map_field_mut(waiver, "waiver").unwrap())?;
                    sort_set_field(waiver, "targets")?;
                }
            }
            sort_set_field(value, "nodes")?;
            sort_set_field(value, "edges")
        }
        GraphKind::State => {
            for node in array_field_mut(value, "nodes")? {
                if let Some(cell) = map_field_mut(node, "cell") {
                    normalize_type_field(cell, "type")?;
                    normalize_state_value(map_field_mut(cell, "state").unwrap())?;
                }
                if let Some(handle) = map_field_mut(node, "handle") {
                    normalize_type(handle)?;
                }
            }
            for transition in array_field_mut(value, "transitions")? {
                normalize_execution_result(map_field_mut(transition, "result").unwrap())?;
            }
            sort_set_field(value, "nodes")?;
            sort_set_field(value, "edges")?;
            sort_set_field(value, "transitions")
        }
        GraphKind::Execution => {
            for node in array_field_mut(value, "nodes")? {
                if let Some(Value::Map(inputs)) = map_field_mut(node, "inputs") {
                    for (_, input) in inputs {
                        normalize_type_field(input, "type")?;
                    }
                }
                if let Some(Value::Map(outputs)) = map_field_mut(node, "outputs") {
                    for (_, output) in outputs {
                        normalize_type(output)?;
                    }
                }
                normalize_effect_row(map_field_mut(node, "effects").unwrap())?;
                for field in [
                    "capability_decisions",
                    "budgets",
                    "expected_evidence",
                    "dependencies",
                ] {
                    sort_set_field(node, field)?;
                }
            }
            sort_set_field(value, "nodes")?;
            sort_set_field(value, "edges")?;
            sort_set_field(value, "entrypoints")
        }
        GraphKind::Evidence => {
            normalize_content_reference(map_field_mut(value, "execution_graph").unwrap())?;
            for claim in array_field_mut(value, "claims")? {
                normalize_content_reference(map_field_mut(claim, "subject").unwrap())?;
            }
            for item in array_field_mut(value, "items")? {
                normalize_content_reference(map_field_mut(item, "verifier_artifact").unwrap())?;
                normalize_content_reference(map_field_mut(item, "payload").unwrap())?;
                normalize_provenance(map_field_mut(item, "provenance").unwrap())?;
                sort_set_field(item, "claims")?;
                sort_set_field(item, "trust")?;
            }
            for gap in array_field_mut(value, "gaps")? {
                sort_set_field(gap, "obligations")?;
            }
            if let Some(Value::Array(obligations)) = map_field_mut(value, "policy_obligations") {
                for obligation in obligations {
                    sort_set_field(obligation, "classes")?;
                    sort_set_field(obligation, "sources")?;
                }
            }
            for field in ["claims", "items", "gaps", "edges"] {
                sort_set_field(value, field)?;
            }
            sort_optional_set_field(value, "policy_obligations")
        }
    }
}

fn normalize_header(value: &mut Value) -> Result<()> {
    sort_set_field(value, "features")?;
    if let Some(provenance) = map_field_mut(value, "provenance") {
        normalize_provenance(provenance)?;
    }
    if let Some(Value::Array(authorizations)) = map_field_mut(value, "authorization") {
        for authorization in authorizations.iter_mut() {
            normalize_content_reference(map_field_mut(authorization, "subject").unwrap())?;
        }
        sort_semantic_set(authorizations, "authorization")?;
    }
    Ok(())
}

fn normalize_content_reference(value: &mut Value) -> Result<()> {
    sort_set_field(value, "digests")?;
    sort_optional_set_field(value, "locations")
}

fn normalize_provenance(value: &mut Value) -> Result<()> {
    if let Some(source) = map_field_mut(value, "source") {
        normalize_content_reference(source)?;
    }
    sort_optional_set_field(value, "parents")
}

fn normalize_effect_row(value: &mut Value) -> Result<()> {
    for effect in array_field_mut(value, "effects")?.iter_mut() {
        normalize_effect(effect)?;
    }
    sort_set_field(value, "effects")
}

fn normalize_effect(_value: &mut Value) -> Result<()> {
    // Effect parameters are ordered arbitrary BHCP values and remain opaque.
    Ok(())
}

fn normalize_state_value(value: &mut Value) -> Result<()> {
    let Value::Array(parts) = value else {
        unreachable!("validated state value")
    };
    if matches!(parts.first(), Some(Value::Text(kind)) if kind == "captured") {
        let Value::Array(references) = &mut parts[2] else {
            unreachable!("validated captured references")
        };
        sort_semantic_set(references, "captured state references")?;
        normalize_provenance(&mut parts[3])?;
        normalize_expression(&mut parts[5])?;
    }
    Ok(())
}

fn normalize_expression(value: &mut Value) -> Result<()> {
    normalize_type_field(value, "type")?;
    let Some(Value::Array(form)) = map_field_mut(value, "form") else {
        unreachable!("validated expression form")
    };
    let kind = match form.first() {
        Some(Value::Text(kind)) => kind.clone(),
        _ => unreachable!("validated expression kind"),
    };
    match kind.as_str() {
        "record" => {
            let Value::Map(fields) = &mut form[1] else {
                unreachable!()
            };
            for (_, field) in fields {
                normalize_expression(field)?;
            }
        }
        "tuple" => normalize_expression_array(&mut form[1])?,
        "variant" => normalize_expression(&mut form[2])?,
        "collection" => {
            normalize_expression_array(&mut form[2])?;
            if form[1] == Value::Text("set".to_owned()) {
                let Value::Array(elements) = &mut form[2] else {
                    unreachable!()
                };
                sort_semantic_set(elements, "expression set")?;
            }
        }
        "map" => {
            let Value::Array(entries) = &mut form[1] else {
                unreachable!()
            };
            for entry in entries.iter_mut() {
                let Value::Array(pair) = entry else {
                    unreachable!()
                };
                normalize_expression(&mut pair[0])?;
                normalize_expression(&mut pair[1])?;
            }
            sort_semantic_set(entries, "expression map")?;
        }
        "select" | "unary" => {
            let index = if kind == "select" { 1 } else { 2 };
            normalize_expression(&mut form[index])?;
        }
        "binary" => {
            normalize_expression(&mut form[2])?;
            normalize_expression(&mut form[3])?;
        }
        "if" => {
            for expression in &mut form[1..=3] {
                normalize_expression(expression)?;
            }
        }
        "let" => {
            normalize_binding(&mut form[1])?;
            normalize_expression(&mut form[2])?;
            normalize_expression(&mut form[3])?;
        }
        "match" => {
            normalize_expression(&mut form[1])?;
            let Value::Array(arms) = &mut form[2] else {
                unreachable!()
            };
            for arm in arms {
                normalize_match_arm(arm)?;
            }
        }
        "call" => normalize_expression_array(&mut form[2])?,
        "quantify" => {
            normalize_binding(&mut form[2])?;
            normalize_expression(&mut form[3])?;
            normalize_expression(&mut form[4])?;
            if form.len() == 6 {
                normalize_verifier_binding(&mut form[5])?;
            }
        }
        "cast-dynamic" => {
            normalize_expression(&mut form[1])?;
            normalize_type(&mut form[2])?;
        }
        "literal" | "reference" => {}
        _ => unreachable!("validated expression form"),
    }
    Ok(())
}

fn normalize_expression_array(value: &mut Value) -> Result<()> {
    let Value::Array(expressions) = value else {
        unreachable!("validated expression array")
    };
    for expression in expressions {
        normalize_expression(expression)?;
    }
    Ok(())
}

fn normalize_match_arm(value: &mut Value) -> Result<()> {
    let Value::Array(parts) = value else {
        unreachable!("validated match arm")
    };
    normalize_pattern(&mut parts[0])?;
    for expression in &mut parts[1..] {
        normalize_expression(expression)?;
    }
    Ok(())
}

fn normalize_pattern(value: &mut Value) -> Result<()> {
    let Value::Array(parts) = value else {
        unreachable!("validated pattern")
    };
    let kind = match parts.first() {
        Some(Value::Text(kind)) => kind.as_str(),
        _ => unreachable!(),
    };
    match kind {
        "bind" => normalize_binding(&mut parts[1]),
        "variant" => {
            let Value::Array(patterns) = &mut parts[2] else {
                unreachable!()
            };
            for pattern in patterns {
                normalize_pattern(pattern)?;
            }
            Ok(())
        }
        "tuple" => {
            let Value::Array(patterns) = &mut parts[1] else {
                unreachable!()
            };
            for pattern in patterns {
                normalize_pattern(pattern)?;
            }
            Ok(())
        }
        "record" => {
            let Value::Map(patterns) = &mut parts[1] else {
                unreachable!()
            };
            for (_, pattern) in patterns {
                normalize_pattern(pattern)?;
            }
            Ok(())
        }
        "wildcard" | "literal" => Ok(()),
        _ => unreachable!("validated pattern kind"),
    }
}

fn normalize_binding(value: &mut Value) -> Result<()> {
    normalize_type_field(value, "type")
}

fn normalize_verifier_binding(value: &mut Value) -> Result<()> {
    normalize_type_field(value, "input")?;
    normalize_type_field(value, "output")?;
    sort_optional_set_field(value, "trust")
}

fn normalize_execution_result(value: &mut Value) -> Result<()> {
    match text_field(value, "state") {
        Some("completed") => {
            let verdict = map_field_mut(value, "verdict").unwrap();
            match text_field(verdict, "state") {
                Some("satisfied") => sort_set_field(verdict, "evidence"),
                Some("refuted") => sort_set_field(verdict, "counter_evidence"),
                Some("unresolved") => sort_set_field(verdict, "partial_evidence"),
                _ => unreachable!(),
            }
        }
        Some("faulted") => Ok(()),
        _ => unreachable!("validated execution result"),
    }
}

fn normalize_type_field(value: &mut Value, field: &str) -> Result<()> {
    normalize_type(map_field_mut(value, field).unwrap())
}

fn normalize_type(value: &mut Value) -> Result<()> {
    *value = crate::typecheck::CheckedType::from_value(value)
        .map_err(|error| invalid_schema(error.message))?
        .to_value();
    Ok(())
}

fn array_field_mut<'a>(value: &'a mut Value, field: &str) -> Result<&'a mut Vec<Value>> {
    match map_field_mut(value, field) {
        Some(Value::Array(items)) => Ok(items),
        _ => Err(invalid_schema(format!(
            "graph field {field} must be an array"
        ))),
    }
}

fn sort_set_field(value: &mut Value, field: &str) -> Result<()> {
    sort_semantic_set(array_field_mut(value, field)?, field)
}

fn sort_optional_set_field(value: &mut Value, field: &str) -> Result<()> {
    if value.get(field).is_some() {
        sort_set_field(value, field)
    } else {
        Ok(())
    }
}

fn sort_semantic_set(items: &mut Vec<Value>, field: &str) -> Result<()> {
    let mut encoded = items
        .drain(..)
        .map(|item| encode_deterministic(&item).map(|bytes| (bytes, item)))
        .collect::<Result<Vec<_>>>()?;
    encoded.sort_by(|left, right| left.0.cmp(&right.0));
    if encoded.windows(2).any(|pair| pair[0].0 == pair[1].0) {
        return Err(duplicate(format!(
            "graph semantic set {field} contains a duplicate"
        )));
    }
    items.extend(encoded.into_iter().map(|(_, item)| item));
    Ok(())
}

fn collect_nodes(value: &Value, kind: GraphKind) -> Result<Vec<GraphNode>> {
    let fields: &[&str] = match kind {
        GraphKind::Evidence => &["claims", "items", "gaps", "policy_obligations"],
        _ => &["nodes"],
    };
    let mut nodes = Vec::new();
    let mut ids = BTreeSet::new();
    for field in fields {
        let Some(Value::Array(items)) = value.get(field) else {
            if *field == "policy_obligations" {
                continue;
            }
            return Err(invalid_schema(format!(
                "graph field {field} must be an array"
            )));
        };
        for item in items {
            let id = required_ref(item, "id")?.to_owned();
            if !ids.insert(id.clone()) {
                return Err(duplicate(format!("duplicate graph ID {id:?}")));
            }
            let node_kind = evidence_node_kind(field, item)
                .or_else(|| text_field(item, "kind").map(ToOwned::to_owned))
                .ok_or_else(|| invalid_schema(format!("graph node {id:?} has no kind")))?;
            nodes.push(GraphNode {
                id,
                kind: node_kind,
                value: item.clone(),
            });
        }
    }
    Ok(nodes)
}

fn evidence_node_kind(field: &str, value: &Value) -> Option<String> {
    match field {
        "claims" => Some(format!("claim:{}", text_field(value, "polarity")?)),
        "items" => Some(format!("evidence:{}", text_field(value, "class")?)),
        "gaps" => Some(format!("gap:{}", text_field(value, "kind")?)),
        "policy_obligations" => Some("policy-obligation".to_owned()),
        _ => None,
    }
}

fn collect_edges(value: &Value) -> Result<Vec<GraphEdge>> {
    let Some(Value::Array(items)) = value.get("edges") else {
        return Err(invalid_schema("graph edges must be an array"));
    };
    let mut edges = Vec::with_capacity(items.len());
    for item in items {
        edges.push(GraphEdge {
            id: required_ref(item, "id")?.to_owned(),
            from: required_ref(item, "from")?.to_owned(),
            to: required_ref(item, "to")?.to_owned(),
            kind: text_field(item, "kind")
                .ok_or_else(|| invalid_schema("graph edge kind must be text"))?
                .to_owned(),
            value: item.clone(),
        });
    }
    Ok(edges)
}

fn validate_references(
    value: &Value,
    kind: GraphKind,
    nodes: &[GraphNode],
    edges: &[GraphEdge],
) -> Result<()> {
    let mut all_ids = BTreeSet::new();
    for node in nodes {
        if !all_ids.insert(node.id.as_str()) {
            return Err(duplicate(format!("duplicate graph ID {:?}", node.id)));
        }
    }
    for edge in edges {
        if !all_ids.insert(edge.id.as_str()) {
            return Err(duplicate(format!("duplicate graph ID {:?}", edge.id)));
        }
    }

    let mut endpoints: BTreeSet<&str> = nodes.iter().map(|node| node.id.as_str()).collect();
    if kind == GraphKind::Evidence {
        let Value::Map(statuses) = value
            .get("obligation_status")
            .ok_or_else(|| invalid_schema("evidence graph requires obligation_status"))?
        else {
            return Err(invalid_schema("obligation_status must be a map"));
        };
        endpoints.extend(statuses.iter().map(|(id, _)| id.as_str()));
    }
    for edge in edges {
        require_endpoint(&endpoints, &edge.from, &edge.id)?;
        require_endpoint(&endpoints, &edge.to, &edge.id)?;
    }

    match kind {
        GraphKind::State => validate_state_references(value, &endpoints, &mut all_ids)?,
        GraphKind::Execution => validate_execution_references(value, &endpoints)?,
        GraphKind::Evidence => validate_evidence_references(value, &endpoints)?,
        _ => {}
    }
    if kind.forbids_cycles() {
        reject_cycle(value, kind, nodes, edges)?;
    }
    Ok(())
}

fn validate_state_references<'a>(
    value: &'a Value,
    endpoints: &BTreeSet<&'a str>,
    all_ids: &mut BTreeSet<&'a str>,
) -> Result<()> {
    let Some(Value::Array(transitions)) = value.get("transitions") else {
        return Err(invalid_schema("state graph transitions must be an array"));
    };
    for transition in transitions {
        let id = required_ref(transition, "id")?;
        if !all_ids.insert(id) {
            return Err(duplicate(format!("duplicate graph ID {id:?}")));
        }
        require_endpoint(endpoints, required_ref(transition, "cell")?, id)?;
    }
    Ok(())
}

fn validate_execution_references(value: &Value, endpoints: &BTreeSet<&str>) -> Result<()> {
    let Some(Value::Array(entrypoints)) = value.get("entrypoints") else {
        return Err(invalid_schema(
            "execution graph entrypoints must be an array",
        ));
    };
    for entrypoint in entrypoints {
        let Value::Text(entrypoint) = entrypoint else {
            return Err(invalid_schema(
                "execution graph entrypoint must be a ref-id",
            ));
        };
        require_endpoint(endpoints, entrypoint, "entrypoints")?;
    }
    let Some(Value::Array(nodes)) = value.get("nodes") else {
        return Err(invalid_schema("execution graph nodes must be an array"));
    };
    for node in nodes {
        let id = required_ref(node, "id")?;
        let Some(Value::Array(dependencies)) = node.get("dependencies") else {
            return Err(invalid_schema("execution dependencies must be an array"));
        };
        for dependency in dependencies {
            let Value::Text(dependency) = dependency else {
                return Err(invalid_schema("execution dependency must be a ref-id"));
            };
            require_endpoint(endpoints, dependency, id)?;
        }
    }
    Ok(())
}

fn validate_evidence_references(value: &Value, endpoints: &BTreeSet<&str>) -> Result<()> {
    for (field, references) in [
        ("claims", "obligation"),
        ("items", "claims"),
        ("gaps", "obligations"),
    ] {
        let Some(Value::Array(items)) = value.get(field) else {
            return Err(invalid_schema(format!(
                "evidence graph {field} must be an array"
            )));
        };
        for item in items {
            let id = required_ref(item, "id")?;
            match item.get(references) {
                Some(Value::Text(reference)) => require_endpoint(endpoints, reference, id)?,
                Some(Value::Array(values)) => {
                    for reference in values {
                        let Value::Text(reference) = reference else {
                            return Err(invalid_schema("evidence reference must be a ref-id"));
                        };
                        require_endpoint(endpoints, reference, id)?;
                    }
                }
                _ => {
                    return Err(invalid_schema(format!(
                        "evidence {field} requires {references}"
                    )));
                }
            }
        }
    }
    Ok(())
}

fn reject_cycle(
    value: &Value,
    kind: GraphKind,
    nodes: &[GraphNode],
    edges: &[GraphEdge],
) -> Result<()> {
    let mut adjacency: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for node in nodes {
        adjacency.entry(&node.id).or_default();
    }
    for edge in edges {
        adjacency.entry(&edge.from).or_default().push(&edge.to);
    }
    if kind == GraphKind::Execution
        && let Some(Value::Array(execution_nodes)) = value.get("nodes")
    {
        for node in execution_nodes {
            let id = required_ref(node, "id")?;
            let Some(Value::Array(dependencies)) = node.get("dependencies") else {
                continue;
            };
            for dependency in dependencies {
                let Value::Text(dependency) = dependency else {
                    continue;
                };
                adjacency.entry(dependency).or_default().push(id);
            }
        }
    }
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    for node in adjacency.keys().copied().collect::<Vec<_>>() {
        if visit(node, &adjacency, &mut visiting, &mut visited) {
            return Err(Diagnostic::plain(
                "BHCP7004",
                "graph contains a cycle in an acyclic graph kind",
            ));
        }
    }
    Ok(())
}

fn visit<'a>(
    node: &'a str,
    adjacency: &BTreeMap<&'a str, Vec<&'a str>>,
    visiting: &mut BTreeSet<&'a str>,
    visited: &mut BTreeSet<&'a str>,
) -> bool {
    if visited.contains(node) {
        return false;
    }
    if !visiting.insert(node) {
        return true;
    }
    if adjacency
        .get(node)
        .into_iter()
        .flatten()
        .any(|next| visit(next, adjacency, visiting, visited))
    {
        return true;
    }
    visiting.remove(node);
    visited.insert(node);
    false
}

fn semantic_projection(value: &Value, kind: GraphKind) -> Value {
    let mut value = value.clone();
    for field in ["semantic_id", "artifact_id", "provenance", "authorization"] {
        remove_field(&mut value, field);
    }

    project_content_reference_field(&mut value, "semantic_ir");
    match kind {
        GraphKind::Obligation => {
            if let Some(Value::Array(nodes)) = map_field_mut(&mut value, "nodes") {
                for node in nodes {
                    remove_field(node, "source_clauses");
                    if let Some(policy) = map_field_mut(node, "policy") {
                        remove_field(policy, "sources");
                    }
                }
            }
        }
        GraphKind::Capability => {
            let mut waiver_ids = BTreeSet::new();
            if let Some(Value::Array(nodes)) = map_field_mut(&mut value, "nodes") {
                for node in nodes.iter() {
                    if text_field(node, "kind") == Some("waiver")
                        && let Some(Value::Text(id)) = node.get("id")
                    {
                        waiver_ids.insert(id.clone());
                    }
                }
                nodes.retain(|node| text_field(node, "kind") != Some("waiver"));
                for node in nodes {
                    remove_field(node, "source_clauses");
                    let kind = text_field(node, "kind").map(ToOwned::to_owned);
                    if matches!(kind.as_deref(), Some("grant" | "denial"))
                        && let Some(capability) = map_field_mut(node, "capability")
                    {
                        remove_field(capability, "sources");
                    }
                    if let Some(policy) = map_field_mut(node, "policy") {
                        remove_field(policy, "effective_rule");
                        remove_field(policy, "sources");
                    }
                }
            }
            if let Some(Value::Array(edges)) = map_field_mut(&mut value, "edges") {
                edges.retain(|edge| {
                    !matches!(edge.get("from"), Some(Value::Text(id)) if waiver_ids.contains(id))
                        && !matches!(edge.get("to"), Some(Value::Text(id)) if waiver_ids.contains(id))
                });
            }
        }
        GraphKind::Execution => {}
        GraphKind::State => {
            if let Some(Value::Array(nodes)) = map_field_mut(&mut value, "nodes") {
                for node in nodes {
                    let Some(cell) = map_field_mut(node, "cell") else {
                        continue;
                    };
                    let Some(Value::Array(parts)) = map_field_mut(cell, "state") else {
                        continue;
                    };
                    if matches!(parts.first(), Some(Value::Text(state)) if state == "captured") {
                        parts[3] = Value::Null;
                    }
                }
            }
        }
        GraphKind::Evidence => {
            project_content_reference_field(&mut value, "execution_graph");
            if let Some(Value::Array(claims)) = map_field_mut(&mut value, "claims") {
                for claim in claims {
                    project_content_reference_field(claim, "subject");
                }
            }
            if let Some(Value::Array(items)) = map_field_mut(&mut value, "items") {
                for item in items {
                    project_content_reference_field(item, "verifier_artifact");
                    project_content_reference_field(item, "payload");
                    for field in ["produced_at", "fresh_until", "provenance"] {
                        remove_field(item, field);
                    }
                }
            }
        }
    }
    value
}

fn project_content_reference_field(value: &mut Value, field: &str) {
    if let Some(reference) = map_field_mut(value, field) {
        remove_field(reference, "locations");
    }
}

fn optional_hash(value: &Value, field: &str) -> Result<Option<HashId>> {
    value.get(field).map(parse_hash).transpose()
}

fn parse_hash(value: &Value) -> Result<HashId> {
    let algorithm = text_field(value, "algorithm")
        .ok_or_else(|| invalid_identity("identity algorithm must be text"))?;
    let Some(Value::Bytes(digest)) = value.get("digest") else {
        return Err(invalid_identity("identity digest must be bytes"));
    };
    Ok(HashId {
        algorithm: algorithm.to_owned(),
        digest: digest.clone(),
    })
}

fn required_ref<'a>(value: &'a Value, field: &str) -> Result<&'a str> {
    let reference = text_field(value, field)
        .ok_or_else(|| invalid_schema(format!("graph {field} must be a ref-id")))?;
    if reference.is_empty() || reference.len() > 128 {
        return Err(invalid_schema(format!(
            "graph {field} must contain between 1 and 128 bytes"
        )));
    }
    Ok(reference)
}

fn require_endpoint(endpoints: &BTreeSet<&str>, reference: &str, owner: &str) -> Result<()> {
    if endpoints.contains(reference) {
        Ok(())
    } else {
        Err(Diagnostic::plain(
            "BHCP7003",
            format!("graph reference {reference:?} from {owner:?} is dangling"),
        ))
    }
}

fn text_field<'a>(value: &'a Value, field: &str) -> Option<&'a str> {
    match value.get(field) {
        Some(Value::Text(value)) => Some(value),
        _ => None,
    }
}

fn map_field_mut<'a>(value: &'a mut Value, field: &str) -> Option<&'a mut Value> {
    let Value::Map(entries) = value else {
        return None;
    };
    entries
        .iter_mut()
        .find_map(|(key, value)| (key == field).then_some(value))
}

fn remove_field(value: &mut Value, field: &str) {
    if let Value::Map(entries) = value {
        entries.retain(|(key, _)| key != field);
    }
}

fn insert_field(value: &mut Value, field: &str, inserted: Value) -> Result<()> {
    let Value::Map(entries) = value else {
        return Err(invalid_schema("graph root must be a map"));
    };
    entries.push((field.to_owned(), inserted));
    *value = Value::owned_map(std::mem::take(entries));
    Ok(())
}

fn invalid_schema(message: impl Into<String>) -> Diagnostic {
    Diagnostic::plain("BHCP7001", message)
}

fn duplicate(message: impl Into<String>) -> Diagnostic {
    Diagnostic::plain("BHCP7002", message)
}

fn invalid_identity(message: impl Into<String>) -> Diagnostic {
    Diagnostic::plain("BHCP7005", message)
}
