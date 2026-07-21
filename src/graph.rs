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
        &["evidence"],
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
    validate_optional_ref_array(value, "evidence", "obligation node")
}

fn capability_node_fields(value: &Value) -> Result<()> {
    validate_map_fields(
        value,
        &["id", "kind"],
        &["capability", "payload"],
        "capability node",
    )?;
    required_ref(value, "id")?;
    require_one_of(
        value,
        "kind",
        &["request", "grant", "denial", "resource", "decision"],
        "capability node",
    )?;
    if let Some(capability) = value.get("capability") {
        validate_map_fields(
            capability,
            &["effect", "scope", "decision", "sources"],
            &[],
            "capability",
        )?;
        require_one_of(capability, "decision", &["allow", "deny"], "capability")?;
        validate_nonempty_ref_array(capability, "sources", "capability")?;
    }
    Ok(())
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
        require_unsigned(cell, "atomic_version", "state cell")?;
        require_array(cell, "state", "state cell")?;
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
    }
    if !matches!(value.get("outputs"), Some(Value::Map(_))) {
        return Err(invalid_schema("execution outputs must be a map"));
    }
    for field in ["capability_decisions", "dependencies"] {
        validate_ref_array(value, field, "execution node")?;
    }
    for field in ["budgets", "expected_evidence"] {
        require_array(value, field, "execution node")?;
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
    Ok(())
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
            &[],
            "evidence claim",
        )?;
        required_ref(claim, "id")?;
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
            &["fresh_until"],
            "evidence item",
        )?;
        required_ref(item, "id")?;
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
            &[],
            "evidence gap",
        )?;
        required_ref(gap, "id")?;
        require_text_value(gap, "kind", "evidence gap")?;
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
            parse_hash(parent)?;
        }
    } else if value.get("parents").is_some() {
        return Err(invalid_schema("provenance parents must be an array"));
    }
    if value.get("annotations").is_some()
        && !matches!(value.get("annotations"), Some(Value::Map(_)))
    {
        return Err(invalid_schema("provenance annotations must be a map"));
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
    require_text_value(value, "message", "reason")
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
    sort_text_array(value, "features")?;
    match kind {
        GraphKind::Obligation | GraphKind::Capability => {
            sort_id_array(value, "nodes")?;
            sort_id_array(value, "edges")?;
        }
        GraphKind::State => {
            sort_id_array(value, "nodes")?;
            sort_id_array(value, "edges")?;
            sort_id_array(value, "transitions")?;
        }
        GraphKind::Execution => {
            sort_id_array(value, "nodes")?;
            sort_id_array(value, "edges")?;
            sort_text_array(value, "entrypoints")?;
        }
        GraphKind::Evidence => {
            for field in ["claims", "items", "gaps", "edges", "policy_obligations"] {
                if value.get(field).is_some() {
                    sort_id_array(value, field)?;
                }
            }
        }
    }
    normalize_nested_sets(value, kind)
}

fn normalize_nested_sets(value: &mut Value, kind: GraphKind) -> Result<()> {
    let fields: &[(&str, &[&str])] = match kind {
        GraphKind::Obligation => &[("nodes", &["evidence"])],
        GraphKind::Capability => &[("nodes", &["sources"])],
        GraphKind::State => &[],
        GraphKind::Execution => &[(
            "nodes",
            &["capability_decisions", "expected_evidence", "dependencies"],
        )],
        GraphKind::Evidence => &[
            ("items", &["claims", "trust"]),
            ("gaps", &["obligations"]),
            ("policy_obligations", &["classes"]),
        ],
    };
    for (array_field, nested) in fields {
        let Some(Value::Array(items)) = map_field_mut(value, array_field) else {
            continue;
        };
        for item in items {
            for field in *nested {
                if item.get(field).is_some() {
                    sort_text_array(item, field)?;
                }
            }
        }
    }
    Ok(())
}

fn sort_id_array(value: &mut Value, field: &str) -> Result<()> {
    let Some(Value::Array(items)) = map_field_mut(value, field) else {
        return Err(invalid_schema(format!(
            "graph field {field} must be an array"
        )));
    };
    items.sort_by(|left, right| text_field(left, "id").cmp(&text_field(right, "id")));
    Ok(())
}

fn sort_text_array(value: &mut Value, field: &str) -> Result<()> {
    let Some(Value::Array(items)) = map_field_mut(value, field) else {
        return Err(invalid_schema(format!(
            "graph field {field} must be an array"
        )));
    };
    if items.iter().any(|item| !matches!(item, Value::Text(_))) {
        return Err(invalid_schema(format!(
            "graph field {field} must contain text references"
        )));
    }
    items.sort_by(|left, right| match (left, right) {
        (Value::Text(left), Value::Text(right)) => left.as_bytes().cmp(right.as_bytes()),
        _ => unreachable!(),
    });
    if items.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(duplicate(format!(
            "graph semantic set {field} contains a duplicate"
        )));
    }
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
    nodes.sort_by(|left, right| left.id.cmp(&right.id));
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
    edges.sort_by(|left, right| left.id.cmp(&right.id));
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
    remove_locations(&mut value);
    if kind == GraphKind::Evidence
        && let Some(Value::Array(items)) = map_field_mut(&mut value, "items")
    {
        for item in items {
            for field in ["produced_at", "fresh_until", "provenance"] {
                remove_field(item, field);
            }
        }
    }
    value
}

fn remove_locations(value: &mut Value) {
    match value {
        Value::Array(items) => items.iter_mut().for_each(remove_locations),
        Value::Map(entries) => {
            entries.retain(|(key, _)| key != "locations");
            for (_, item) in entries {
                remove_locations(item);
            }
        }
        Value::Tag(_, item) => remove_locations(item),
        _ => {}
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
