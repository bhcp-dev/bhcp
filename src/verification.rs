//! Registered verifier dispatch and deterministic v0 evidence bundles.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::panic::{AssertUnwindSafe, catch_unwind};

use crate::adapter::{
    AdapterExecutionRecord, AdapterRequest, CancellationToken, VerifierProcessRunner,
};
use crate::cbor::encode_deterministic;
use crate::diagnostic::{Diagnostic, Result};
use crate::hash::{HashAlgorithm, artifact_hash_with};
use crate::kernel::Reason;
use crate::manifest::VerifierAdapterDeclaration;
use crate::model::{
    BhcpType, ClauseKind, ContentReference, Expression, ExpressionForm, GoalDefinition, HashId,
    VerifierBinding, is_symbol,
};
use crate::pipeline::Compilation;
use crate::schema::validate_root;
use crate::value::Value;

const INVALID_VERIFICATION: &str = "BHCP7001";
const VERIFICATION_FEATURE: &str = "bhcp/feature.verifier-dispatch@0";
const ADAPTER_EVIDENCE_FEATURE: &str = "bhcp/feature.process-adapter-evidence@0";
const EXPRESSION_VERIFIER: &str = "bhcp.verifier/expression@0";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifierEvidence {
    pub evidence_class: String,
    pub predicate: String,
    pub media_type: String,
    pub payload: Vec<u8>,
    pub trust: Vec<String>,
}

impl VerifierEvidence {
    pub fn new(
        evidence_class: impl Into<String>,
        predicate: impl Into<String>,
        media_type: impl Into<String>,
        payload: Vec<u8>,
        trust: Vec<String>,
    ) -> Self {
        Self {
            evidence_class: evidence_class.into(),
            predicate: predicate.into(),
            media_type: media_type.into(),
            payload,
            trust,
        }
    }

    fn validate(&self) -> Result<()> {
        if !is_evidence_class(&self.evidence_class) {
            return Err(invalid("verifier evidence class is not registered"));
        }
        if !is_symbol(&self.predicate) {
            return Err(invalid("verifier evidence predicate is not a symbol-id"));
        }
        if self.media_type.is_empty() {
            return Err(invalid("verifier evidence media type must not be empty"));
        }
        if self.trust.iter().any(|trust| !is_symbol(trust)) {
            return Err(invalid("verifier evidence trust entry is not a symbol-id"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VerifierConclusion {
    Accepted(VerifierEvidence),
    Rejected(VerifierEvidence),
    Unresolved {
        reason: Reason,
        evidence: Option<VerifierEvidence>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VerifierExecution {
    Completed(VerifierConclusion),
    Faulted(Reason),
}

pub struct VerifierContext<'a> {
    pub goal: &'a GoalDefinition,
    pub input: &'a Value,
    pub output: &'a Value,
    pub subject: &'a ContentReference,
    pub obligations: &'a [String],
}

pub trait Verifier {
    fn symbol(&self) -> &str;
    fn artifact(&self) -> ContentReference;
    fn verify(&self, context: &VerifierContext<'_>) -> VerifierExecution;
}

#[derive(Default)]
pub struct VerifierRegistry {
    verifiers: BTreeMap<String, RegisteredVerifier>,
}

enum RegisteredVerifier {
    InProcess {
        implementation: Box<dyn Verifier>,
        artifact: ContentReference,
    },
    Adapter {
        runner: VerifierProcessRunner,
        declaration: VerifierAdapterDeclaration,
        effect_ceiling: Vec<String>,
        cancellation: CancellationToken,
    },
}

impl VerifierRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<V: Verifier + 'static>(&mut self, verifier: V) -> Result<()> {
        let symbol = verifier.symbol().to_owned();
        if !is_symbol(&symbol) {
            return Err(invalid("registered verifier name is not a symbol-id"));
        }
        if self.verifiers.contains_key(&symbol) {
            return Err(invalid(format!(
                "verifier {symbol:?} is already registered"
            )));
        }
        let artifact = verifier.artifact();
        artifact.validate()?;
        self.verifiers.insert(
            symbol,
            RegisteredVerifier::InProcess {
                implementation: Box::new(verifier),
                artifact,
            },
        );
        Ok(())
    }

    pub fn register_adapter(
        &mut self,
        runner: VerifierProcessRunner,
        declaration: VerifierAdapterDeclaration,
        mut effect_ceiling: Vec<String>,
        cancellation: CancellationToken,
    ) -> Result<()> {
        let symbol = declaration.symbol.clone();
        if !is_symbol(&symbol) {
            return Err(invalid("registered adapter name is not a symbol-id"));
        }
        if self.verifiers.contains_key(&symbol) {
            return Err(invalid(format!(
                "verifier {symbol:?} is already registered"
            )));
        }
        effect_ceiling.sort();
        effect_ceiling.dedup();
        if effect_ceiling
            .iter()
            .any(|effect| !is_adapter_effect(effect))
        {
            return Err(invalid(
                "adapter effect ceiling contains an unsupported effect",
            ));
        }
        self.verifiers.insert(
            symbol,
            RegisteredVerifier::Adapter {
                runner,
                declaration,
                effect_ceiling,
                cancellation,
            },
        );
        Ok(())
    }

    pub fn verify(&self, request: VerificationRequest<'_>) -> Result<VerificationReport> {
        verify(self, request)
    }
}

pub struct VerificationRequest<'a> {
    pub compilation: &'a Compilation,
    pub goal: &'a str,
    pub input: &'a Value,
    pub output: &'a Value,
    pub subject: ContentReference,
    pub execution_graph: ContentReference,
    pub produced_at: &'a str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VerificationDecision {
    Accepted,
    Rejected,
    Unresolved,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VerificationState {
    Completed(VerificationDecision),
    Faulted(Vec<Reason>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayloadArtifact {
    pub reference: ContentReference,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceClaim {
    pub id: String,
    pub obligation: String,
    pub polarity: String,
    pub subject: ContentReference,
    pub predicate: String,
    pub status: String,
}

impl EvidenceClaim {
    fn to_value(&self) -> Value {
        Value::map([
            ("id", Value::Text(self.id.clone())),
            ("obligation", Value::Text(self.obligation.clone())),
            ("polarity", Value::Text(self.polarity.clone())),
            ("subject", self.subject.to_value()),
            ("predicate", Value::Text(self.predicate.clone())),
            ("status", Value::Text(self.status.clone())),
        ])
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceItem {
    pub id: String,
    pub evidence_class: String,
    pub verifier: String,
    pub verifier_artifact: ContentReference,
    pub payload: ContentReference,
    pub claims: Vec<String>,
    pub produced_at: String,
    pub producer: String,
    pub provenance_source: Option<ContentReference>,
    pub trust: Vec<String>,
}

impl EvidenceItem {
    fn to_value(&self) -> Value {
        let mut provenance = vec![
            ("producer".to_owned(), Value::Text(self.producer.clone())),
            ("created_at".to_owned(), timestamp_value(&self.produced_at)),
        ];
        if let Some(source) = &self.provenance_source {
            provenance.push(("source".to_owned(), source.to_value()));
        }
        Value::map([
            ("id", Value::Text(self.id.clone())),
            ("class", Value::Text(self.evidence_class.clone())),
            ("verifier", Value::Text(self.verifier.clone())),
            ("verifier_artifact", self.verifier_artifact.to_value()),
            ("payload", self.payload.to_value()),
            (
                "claims",
                Value::Array(self.claims.iter().cloned().map(Value::Text).collect()),
            ),
            ("produced_at", timestamp_value(&self.produced_at)),
            ("provenance", Value::owned_map(provenance)),
            (
                "trust",
                Value::Array(self.trust.iter().cloned().map(Value::Text).collect()),
            ),
        ])
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceGap {
    pub id: String,
    pub kind: String,
    pub obligations: Vec<String>,
    pub reason: Reason,
    pub required: bool,
}

impl EvidenceGap {
    fn to_value(&self) -> Value {
        Value::map([
            ("id", Value::Text(self.id.clone())),
            ("kind", Value::Text(self.kind.clone())),
            (
                "obligations",
                Value::Array(self.obligations.iter().cloned().map(Value::Text).collect()),
            ),
            ("reason", reason_value(&self.reason)),
            ("required", Value::Bool(self.required)),
        ])
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceEdge {
    pub id: String,
    pub from: String,
    pub to: String,
    pub kind: String,
}

impl EvidenceEdge {
    fn to_value(&self) -> Value {
        Value::map([
            ("id", Value::Text(self.id.clone())),
            ("from", Value::Text(self.from.clone())),
            ("to", Value::Text(self.to.clone())),
            ("kind", Value::Text(self.kind.clone())),
        ])
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceBundle {
    pub features: Vec<String>,
    pub semantic_ir: ContentReference,
    pub execution_graph: ContentReference,
    pub claims: Vec<EvidenceClaim>,
    pub items: Vec<EvidenceItem>,
    pub gaps: Vec<EvidenceGap>,
    pub edges: Vec<EvidenceEdge>,
    pub obligation_status: BTreeMap<String, String>,
    pub artifact_id: Option<HashId>,
}

impl EvidenceBundle {
    pub fn to_value(&self, include_artifact_id: bool) -> Value {
        let mut entries = vec![
            ("version".to_owned(), Value::Text("bhcp/v0".to_owned())),
            (
                "features".to_owned(),
                Value::Array(self.features.iter().cloned().map(Value::Text).collect()),
            ),
            ("kind".to_owned(), Value::Text("evidence-bundle".to_owned())),
            ("semantic_ir".to_owned(), self.semantic_ir.to_value()),
            (
                "execution_graph".to_owned(),
                self.execution_graph.to_value(),
            ),
            (
                "claims".to_owned(),
                Value::Array(self.claims.iter().map(EvidenceClaim::to_value).collect()),
            ),
            (
                "items".to_owned(),
                Value::Array(self.items.iter().map(EvidenceItem::to_value).collect()),
            ),
            (
                "gaps".to_owned(),
                Value::Array(self.gaps.iter().map(EvidenceGap::to_value).collect()),
            ),
            (
                "edges".to_owned(),
                Value::Array(self.edges.iter().map(EvidenceEdge::to_value).collect()),
            ),
            (
                "obligation_status".to_owned(),
                Value::owned_map(
                    self.obligation_status
                        .iter()
                        .map(|(id, status)| (id.clone(), Value::Text(status.clone())))
                        .collect(),
                ),
            ),
        ];
        if include_artifact_id && let Some(artifact_id) = &self.artifact_id {
            entries.push(("artifact_id".to_owned(), artifact_id.to_value()));
        }
        Value::owned_map(entries)
    }

    pub fn validate(&self) -> Result<()> {
        let mut features = HashSet::new();
        for feature in &self.features {
            if !is_symbol(feature) || !features.insert(feature) {
                return Err(invalid("evidence features must be unique feature IDs"));
            }
        }
        if !normalized(&self.features) {
            return Err(invalid("evidence features are not normalized"));
        }
        self.semantic_ir.validate()?;
        self.execution_graph.validate()?;
        let obligations: HashSet<_> = self.obligation_status.keys().cloned().collect();
        if obligations.iter().any(|obligation| !is_ref(obligation)) {
            return Err(invalid("evidence obligation is not a ref-id"));
        }
        if self
            .obligation_status
            .values()
            .any(|status| !matches!(status.as_str(), "discharged" | "refuted" | "unresolved"))
        {
            return Err(invalid("evidence obligation status is invalid"));
        }

        let mut ids = HashSet::new();
        let mut claims = HashSet::new();
        for claim in &self.claims {
            add_id(&claim.id, &mut ids)?;
            claims.insert(claim.id.clone());
            if !obligations.contains(&claim.obligation)
                || !matches!(claim.polarity.as_str(), "supports" | "refutes")
                || !matches!(
                    claim.status.as_str(),
                    "accepted" | "rejected" | "stale" | "unresolved"
                )
                || !is_symbol(&claim.predicate)
            {
                return Err(invalid("evidence claim is invalid"));
            }
            claim.subject.validate()?;
        }
        let mut items = HashSet::new();
        for item in &self.items {
            add_id(&item.id, &mut ids)?;
            items.insert(item.id.clone());
            if !is_evidence_class(&item.evidence_class)
                || !is_symbol(&item.verifier)
                || !is_symbol(&item.producer)
                || item.trust.iter().any(|trust| !is_symbol(trust))
                || !normalized(&item.trust)
                || item.claims.is_empty()
                || item.claims.iter().any(|claim| !claims.contains(claim))
            {
                return Err(invalid("evidence item is invalid"));
            }
            validate_timestamp(&item.produced_at)?;
            item.verifier_artifact.validate()?;
            item.payload.validate()?;
            if let Some(source) = &item.provenance_source {
                source.validate()?;
            }
        }
        let mut gaps = HashSet::new();
        for gap in &self.gaps {
            add_id(&gap.id, &mut ids)?;
            gaps.insert(gap.id.clone());
            if !is_gap_kind(&gap.kind)
                || gap.obligations.is_empty()
                || gap
                    .obligations
                    .iter()
                    .any(|obligation| !obligations.contains(obligation))
                || !normalized(&gap.obligations)
            {
                return Err(invalid("evidence gap is invalid"));
            }
            validate_reason(&gap.reason)?;
        }
        for edge in &self.edges {
            add_id(&edge.id, &mut ids)?;
            if !items.contains(&edge.from) || !claims.contains(&edge.to) || edge.kind != "produces"
            {
                return Err(invalid("evidence edge is invalid"));
            }
        }

        for (obligation, status) in &self.obligation_status {
            let matching: Vec<_> = self
                .claims
                .iter()
                .filter(|claim| &claim.obligation == obligation)
                .collect();
            let supports = matching
                .iter()
                .any(|claim| claim.status == "accepted" && claim.polarity == "supports");
            let refutes = matching
                .iter()
                .any(|claim| claim.status == "accepted" && claim.polarity == "refutes");
            let unresolved = matching.iter().any(|claim| claim.status == "unresolved")
                || self
                    .gaps
                    .iter()
                    .any(|gap| gap.required && gap.obligations.iter().any(|id| id == obligation));
            let valid = match status.as_str() {
                "discharged" => supports && !refutes && !unresolved,
                "refuted" => refutes,
                "unresolved" => unresolved,
                _ => false,
            };
            if !valid {
                return Err(invalid("evidence status is not justified by its claims"));
            }
        }
        if let Some(artifact_id) = &self.artifact_id {
            artifact_id.validate()?;
            let algorithm = HashAlgorithm::from_id(&artifact_id.algorithm)?;
            if artifact_hash_with(&self.to_value(false), algorithm)? != *artifact_id {
                return Err(invalid("evidence bundle artifact identity does not match"));
            }
        }
        validate_root(&self.to_value(true), "evidence-bundle")?;
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationReport {
    pub state: VerificationState,
    pub bundle: EvidenceBundle,
    pub bundle_bytes: Vec<u8>,
    pub bundle_hash: HashId,
    pub payloads: Vec<PayloadArtifact>,
    pub adapter_records: Vec<AdapterExecutionRecord>,
}

#[derive(Clone, Copy)]
enum Marker {
    Supports,
    Refutes,
    Unresolved,
}

#[derive(Clone, Copy)]
enum ClaimDisposition {
    Supports,
    Refutes,
    Unresolved,
}

impl ClaimDisposition {
    fn fields(self) -> (&'static str, &'static str, Marker) {
        match self {
            Self::Supports => ("supports", "accepted", Marker::Supports),
            Self::Refutes => ("refutes", "accepted", Marker::Refutes),
            Self::Unresolved => ("supports", "unresolved", Marker::Unresolved),
        }
    }
}

struct Builder {
    produced_at: String,
    subject: ContentReference,
    claims: Vec<EvidenceClaim>,
    items: Vec<EvidenceItem>,
    gaps: Vec<EvidenceGap>,
    edges: Vec<EvidenceEdge>,
    payloads: Vec<PayloadArtifact>,
    markers: BTreeMap<String, Vec<Marker>>,
}

impl Builder {
    fn new(
        produced_at: &str,
        subject: ContentReference,
        obligations: impl IntoIterator<Item = String>,
    ) -> Self {
        Self {
            produced_at: produced_at.to_owned(),
            subject,
            claims: vec![],
            items: vec![],
            gaps: vec![],
            edges: vec![],
            payloads: vec![],
            markers: obligations.into_iter().map(|id| (id, vec![])).collect(),
        }
    }

    fn evidence(
        &mut self,
        verifier: &str,
        verifier_artifact: ContentReference,
        evidence: VerifierEvidence,
        obligations: &[String],
        disposition: ClaimDisposition,
        provenance_source: Option<ContentReference>,
    ) -> Result<()> {
        let mut evidence = evidence;
        evidence.trust.sort();
        evidence.trust.dedup();
        evidence.validate()?;
        let (polarity, claim_status, marker) = disposition.fields();
        let payload = ContentReference::from_bytes(
            evidence.media_type.clone(),
            &evidence.payload,
            HashAlgorithm::default(),
        );
        self.payloads.push(PayloadArtifact {
            reference: payload.clone(),
            bytes: evidence.payload,
        });
        let mut claim_ids = Vec::new();
        for obligation in obligations {
            let claim_id = format!("claim-{}", self.claims.len() + 1);
            self.claims.push(EvidenceClaim {
                id: claim_id.clone(),
                obligation: obligation.clone(),
                polarity: polarity.to_owned(),
                subject: self.subject.clone(),
                predicate: evidence.predicate.clone(),
                status: claim_status.to_owned(),
            });
            self.markers
                .get_mut(obligation)
                .expect("resolved obligation")
                .push(marker);
            claim_ids.push(claim_id);
        }
        let item_id = format!("evidence-{}", self.items.len() + 1);
        self.items.push(EvidenceItem {
            id: item_id.clone(),
            evidence_class: evidence.evidence_class,
            verifier: verifier.to_owned(),
            verifier_artifact,
            payload,
            claims: claim_ids.clone(),
            produced_at: self.produced_at.clone(),
            producer: verifier.to_owned(),
            provenance_source,
            trust: evidence.trust,
        });
        for claim_id in claim_ids {
            self.edges.push(EvidenceEdge {
                id: format!("edge-{}", self.edges.len() + 1),
                from: item_id.clone(),
                to: claim_id,
                kind: "produces".to_owned(),
            });
        }
        Ok(())
    }

    fn gap(&mut self, kind: &str, obligations: &[String], reason: Reason) {
        for obligation in obligations {
            self.markers
                .get_mut(obligation)
                .expect("resolved obligation")
                .push(Marker::Unresolved);
        }
        self.gaps.push(EvidenceGap {
            id: format!("gap-{}", self.gaps.len() + 1),
            kind: kind.to_owned(),
            obligations: obligations.to_vec(),
            reason,
            required: true,
        });
    }
}

fn verify(
    registry: &VerifierRegistry,
    request: VerificationRequest<'_>,
) -> Result<VerificationReport> {
    request.compilation.ir.validate().map_err(|diagnostic| {
        invalid(format!(
            "verification received invalid semantic IR: {}",
            diagnostic.message
        ))
    })?;
    if encode_deterministic(&request.compilation.ir.to_value(true))? != request.compilation.ir_bytes
    {
        return Err(invalid(
            "verification semantic IR bytes do not match the typed document",
        ));
    }
    validate_timestamp(request.produced_at)?;
    request.subject.validate()?;
    request.execution_graph.validate()?;
    let goal = request
        .compilation
        .ir
        .goals
        .iter()
        .find(|goal| goal.symbol == request.goal || goal.id == request.goal)
        .ok_or_else(|| {
            invalid(format!(
                "verification goal {:?} does not exist",
                request.goal
            ))
        })?;
    if !goal.input.accepts(request.input) {
        return Err(invalid(
            "verification input does not match the goal input type",
        ));
    }
    if !goal.output.accepts(request.output) {
        return Err(invalid(
            "verification output does not match the goal output type",
        ));
    }

    let mut obligations: Vec<_> = goal
        .clauses
        .iter()
        .filter_map(|clause| {
            matches!(clause.kind, ClauseKind::Contract { .. }).then_some(clause.id.clone())
        })
        .collect();
    obligations.sort();
    if obligations.is_empty() {
        return Err(invalid(
            "verification requires at least one implemented contract obligation",
        ));
    }
    let obligation_set: HashSet<_> = obligations.iter().cloned().collect();
    let bindings = fact_bindings(goal, request.input, request.output)?;
    let mut builder = Builder::new(
        request.produced_at,
        request.subject.clone(),
        obligations.clone(),
    );
    let expression_artifact = ContentReference::from_bytes(
        "application/vnd.bhcp.verifier",
        EXPRESSION_VERIFIER.as_bytes(),
        HashAlgorithm::default(),
    );

    for clause in &goal.clauses {
        let ClauseKind::Contract { condition, .. } = &clause.kind else {
            continue;
        };
        let Value::Bool(accepted) = evaluate(condition, &bindings)? else {
            return Err(invalid("contract condition did not evaluate to Bool"));
        };
        let payload_value = Value::map([
            ("goal", Value::Text(goal.id.clone())),
            ("obligation", Value::Text(clause.id.clone())),
            ("result", Value::Bool(accepted)),
        ]);
        builder.evidence(
            EXPRESSION_VERIFIER,
            expression_artifact.clone(),
            VerifierEvidence::new(
                "static",
                EXPRESSION_VERIFIER,
                "application/cbor",
                encode_deterministic(&payload_value)?,
                vec![],
            ),
            std::slice::from_ref(&clause.id),
            if accepted {
                ClaimDisposition::Supports
            } else {
                ClaimDisposition::Refutes
            },
            None,
        )?;
    }

    let mut faults = Vec::new();
    let mut adapter_records = Vec::new();
    let mut used_adapter = false;
    for clause in &goal.clauses {
        let ClauseKind::Verify {
            binding,
            obligations: targeted,
        } = &clause.kind
        else {
            continue;
        };
        let targeted = if targeted.is_empty() {
            obligations.clone()
        } else {
            if targeted
                .iter()
                .any(|obligation| !obligation_set.contains(obligation))
            {
                return Err(invalid("verifier targets an unknown obligation"));
            }
            targeted.clone()
        };
        let Some(verifier) = registry.verifiers.get(&binding.verifier) else {
            builder.gap(
                "unsupported",
                &targeted,
                Reason {
                    code: "bhcp.reason/verifier-unregistered@0".to_owned(),
                    message: format!("verifier {} is not registered", binding.verifier),
                    details: None,
                },
            );
            continue;
        };
        let context = VerifierContext {
            goal,
            input: request.input,
            output: request.output,
            subject: &request.subject,
            obligations: &targeted,
        };
        let (execution, verifier_artifact, provenance_source) = match verifier {
            RegisteredVerifier::InProcess {
                implementation,
                artifact,
            } => (
                catch_unwind(AssertUnwindSafe(|| implementation.verify(&context))).unwrap_or_else(
                    |_| {
                        VerifierExecution::Faulted(Reason {
                            code: "bhcp.fault/verifier-panic@0".to_owned(),
                            message: "registered verifier panicked".to_owned(),
                            details: None,
                        })
                    },
                ),
                Some(artifact.clone()),
                None,
            ),
            RegisteredVerifier::Adapter {
                runner,
                declaration,
                effect_ceiling,
                cancellation,
            } => {
                used_adapter = true;
                let candidate = encode_deterministic(&Value::map([
                    ("input", request.input.clone()),
                    ("output", request.output.clone()),
                ]))?;
                match catch_unwind(AssertUnwindSafe(|| {
                    runner.run(
                        declaration,
                        AdapterRequest {
                            verifier: &binding.verifier,
                            obligations: &targeted,
                            payload: &candidate,
                            effect_ceiling,
                        },
                        cancellation,
                    )
                })) {
                    Ok(Ok(run)) => {
                        let artifact = run
                            .record
                            .executable_artifact
                            .clone()
                            .unwrap_or_else(|| run.record.registration_artifact.clone());
                        let provenance = Some(run.record.registration_artifact.clone());
                        let execution = run.execution;
                        adapter_records.push(run.record);
                        (execution, Some(artifact), provenance)
                    }
                    Ok(Err(diagnostic)) => (
                        VerifierExecution::Faulted(Reason {
                            code: "bhcp.fault/adapter-boundary@0".to_owned(),
                            message: diagnostic.to_string(),
                            details: None,
                        }),
                        None,
                        None,
                    ),
                    Err(_) => (
                        VerifierExecution::Faulted(Reason {
                            code: "bhcp.fault/adapter-runner-panic@0".to_owned(),
                            message: "registered adapter runner panicked".to_owned(),
                            details: None,
                        }),
                        None,
                        None,
                    ),
                }
            }
        };
        match execution {
            VerifierExecution::Completed(VerifierConclusion::Accepted(evidence)) => {
                validate_declared_evidence(binding, &evidence)?;
                builder.evidence(
                    &binding.verifier,
                    verifier_artifact
                        .clone()
                        .expect("accepted verifier execution retains an artifact"),
                    evidence,
                    &targeted,
                    ClaimDisposition::Supports,
                    provenance_source.clone(),
                )?;
            }
            VerifierExecution::Completed(VerifierConclusion::Rejected(evidence)) => {
                validate_declared_evidence(binding, &evidence)?;
                builder.evidence(
                    &binding.verifier,
                    verifier_artifact
                        .clone()
                        .expect("rejected verifier execution retains an artifact"),
                    evidence,
                    &targeted,
                    ClaimDisposition::Refutes,
                    provenance_source.clone(),
                )?;
            }
            VerifierExecution::Completed(VerifierConclusion::Unresolved { reason, evidence }) => {
                validate_reason(&reason)?;
                if let Some(evidence) = evidence {
                    validate_declared_evidence(binding, &evidence)?;
                    builder.evidence(
                        &binding.verifier,
                        verifier_artifact
                            .clone()
                            .expect("unresolved verifier evidence retains an artifact"),
                        evidence,
                        &targeted,
                        ClaimDisposition::Unresolved,
                        provenance_source.clone(),
                    )?;
                }
                builder.gap("missing", &targeted, reason);
            }
            VerifierExecution::Faulted(reason) => {
                validate_reason(&reason)?;
                builder.gap(
                    "bhcp.evidence-gap/verifier-fault@0",
                    &targeted,
                    reason.clone(),
                );
                faults.push(reason);
            }
        }
    }

    let mut obligation_status = BTreeMap::new();
    for (obligation, markers) in &builder.markers {
        let status = if markers
            .iter()
            .any(|marker| matches!(marker, Marker::Refutes))
        {
            "refuted"
        } else if markers
            .iter()
            .any(|marker| matches!(marker, Marker::Unresolved))
        {
            "unresolved"
        } else if markers
            .iter()
            .any(|marker| matches!(marker, Marker::Supports))
        {
            "discharged"
        } else {
            return Err(invalid("contract obligation has no verification path"));
        };
        obligation_status.insert(obligation.clone(), status.to_owned());
    }
    let decision = if obligation_status.values().any(|status| status == "refuted") {
        VerificationDecision::Rejected
    } else if obligation_status
        .values()
        .all(|status| status == "discharged")
    {
        VerificationDecision::Accepted
    } else {
        VerificationDecision::Unresolved
    };
    let state = if faults.is_empty() {
        VerificationState::Completed(decision)
    } else {
        VerificationState::Faulted(faults)
    };

    let semantic_ir = ContentReference::from_bytes(
        "application/cbor",
        &request.compilation.ir_bytes,
        HashAlgorithm::default(),
    );
    let mut features = vec![VERIFICATION_FEATURE.to_owned()];
    if used_adapter {
        features.push(ADAPTER_EVIDENCE_FEATURE.to_owned());
        features.sort();
    }
    let mut bundle = EvidenceBundle {
        features,
        semantic_ir,
        execution_graph: request.execution_graph,
        claims: builder.claims,
        items: builder.items,
        gaps: builder.gaps,
        edges: builder.edges,
        obligation_status,
        artifact_id: None,
    };
    let bundle_hash = artifact_hash_with(&bundle.to_value(false), HashAlgorithm::default())?;
    bundle.artifact_id = Some(bundle_hash.clone());
    bundle.validate()?;
    let bundle_bytes = encode_deterministic(&bundle.to_value(true))?;
    Ok(VerificationReport {
        state,
        bundle,
        bundle_bytes,
        bundle_hash,
        payloads: builder.payloads,
        adapter_records,
    })
}

fn fact_bindings(
    goal: &GoalDefinition,
    input: &Value,
    output: &Value,
) -> Result<HashMap<String, Value>> {
    let mut bindings = HashMap::new();
    for clause in &goal.clauses {
        let ClauseKind::Fact { kind, binding } = &clause.kind else {
            continue;
        };
        let record = if *kind == "input" { input } else { output };
        let value = record.get(&binding.name).ok_or_else(|| {
            invalid(format!(
                "verification value is missing binding {:?}",
                binding.name
            ))
        })?;
        bindings.insert(binding.id.clone(), value.clone());
    }
    Ok(bindings)
}

fn evaluate(expression: &Expression, bindings: &HashMap<String, Value>) -> Result<Value> {
    match &expression.form {
        ExpressionForm::Literal(value) => Ok(value.clone()),
        ExpressionForm::Reference(reference) => bindings
            .get(reference)
            .cloned()
            .ok_or_else(|| invalid(format!("unbound expression reference {reference:?}"))),
        ExpressionForm::Unary(operator, operand) => {
            let operand = evaluate(operand, bindings)?;
            match (operator.as_str(), operand) {
                ("!", Value::Bool(value)) => Ok(Value::Bool(!value)),
                ("-", value) => integer(value)
                    .and_then(|value| {
                        value
                            .checked_neg()
                            .ok_or_else(|| invalid("integer overflow"))
                    })
                    .map(integer_value),
                _ => Err(invalid("ill-typed unary expression reached verification")),
            }
        }
        ExpressionForm::Binary(operator, left, right) => {
            let left = evaluate(left, bindings)?;
            let right = evaluate(right, bindings)?;
            match operator.as_str() {
                "==" => Ok(Value::Bool(left == right)),
                "!=" => Ok(Value::Bool(left != right)),
                "&&" => Ok(Value::Bool(boolean(left)? && boolean(right)?)),
                "||" => Ok(Value::Bool(boolean(left)? || boolean(right)?)),
                "<" | "<=" | ">" | ">=" => {
                    let left = integer(left)?;
                    let right = integer(right)?;
                    Ok(Value::Bool(match operator.as_str() {
                        "<" => left < right,
                        "<=" => left <= right,
                        ">" => left > right,
                        ">=" => left >= right,
                        _ => unreachable!(),
                    }))
                }
                "+" => match (left, right) {
                    (Value::Text(left), Value::Text(right)) => {
                        Ok(Value::Text(format!("{left}{right}")))
                    }
                    (left, right) => integer(left)
                        .and_then(|left| {
                            integer(right).and_then(|right| {
                                left.checked_add(right)
                                    .ok_or_else(|| invalid("integer overflow"))
                            })
                        })
                        .map(integer_value),
                },
                _ => Err(invalid(
                    "expression operator is not executable by verification",
                )),
            }
        }
        ExpressionForm::If(_, _, _) | ExpressionForm::Call(_, _) => Err(invalid(
            "expression form is outside the executable verification slice",
        )),
    }
}

fn integer(value: Value) -> Result<i64> {
    match value {
        Value::Array(values) if matches!(values.as_slice(), [Value::Text(kind), Value::Integer(_)] if kind == "integer") =>
        {
            let Value::Integer(value) = values[1] else {
                unreachable!()
            };
            Ok(value)
        }
        _ => Err(invalid("expression value is not an Integer")),
    }
}

fn integer_value(value: i64) -> Value {
    Value::Array(vec![
        Value::Text("integer".to_owned()),
        Value::Integer(value),
    ])
}

fn boolean(value: Value) -> Result<bool> {
    match value {
        Value::Bool(value) => Ok(value),
        _ => Err(invalid("expression value is not Bool")),
    }
}

fn validate_timestamp(value: &str) -> Result<()> {
    let bytes = value.as_bytes();
    let punctuation = [
        (4, b'-'),
        (7, b'-'),
        (10, b'T'),
        (13, b':'),
        (16, b':'),
        (19, b'Z'),
    ];
    let valid_shape = bytes.len() == 20
        && punctuation
            .iter()
            .all(|(index, expected)| bytes[*index] == *expected)
        && bytes.iter().enumerate().all(|(index, byte)| {
            punctuation.iter().any(|(position, _)| *position == index) || byte.is_ascii_digit()
        });
    if !valid_shape {
        return Err(invalid(
            "implemented evidence timestamps require canonical UTC second precision",
        ));
    }
    let number = |range: std::ops::Range<usize>| -> u32 {
        value[range]
            .parse()
            .expect("timestamp shape contains only ASCII digits")
    };
    let year = number(0..4);
    let month = number(5..7);
    let day = number(8..10);
    let hour = number(11..13);
    let minute = number(14..16);
    let second = number(17..19);
    let leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => 0,
    };
    if day == 0 || day > days || hour > 23 || minute > 59 || second > 60 {
        return Err(invalid("evidence timestamp is not a valid UTC date-time"));
    }
    Ok(())
}

fn timestamp_value(value: &str) -> Value {
    Value::Tag(0, Box::new(Value::Text(value.to_owned())))
}

fn validate_reason(reason: &Reason) -> Result<()> {
    if is_symbol(&reason.code) {
        Ok(())
    } else {
        Err(invalid("verification reason code is not a symbol-id"))
    }
}

fn reason_value(reason: &Reason) -> Value {
    let mut entries = vec![
        ("code".to_owned(), Value::Text(reason.code.clone())),
        ("message".to_owned(), Value::Text(reason.message.clone())),
    ];
    if let Some(details) = &reason.details {
        entries.push(("details".to_owned(), details.clone()));
    }
    Value::owned_map(entries)
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

fn is_adapter_effect(value: &str) -> bool {
    matches!(
        value,
        "bhcp-effect/clock@0"
            | "bhcp-effect/fs.read@0"
            | "bhcp-effect/fs.write@0"
            | "bhcp-effect/process@0"
    )
}

fn validate_declared_evidence(
    binding: &VerifierBinding,
    evidence: &VerifierEvidence,
) -> Result<()> {
    let BhcpType::Evidence(classes) = &binding.output else {
        return Err(invalid("verifier output is not an evidence type"));
    };
    if !classes.is_empty() && !classes.contains(&evidence.evidence_class) {
        return Err(invalid(
            "verifier returned an evidence class outside its declared output",
        ));
    }
    if !binding.trust.is_empty() && !binding.trust.contains(&evidence.evidence_class) {
        return Err(invalid(
            "verifier returned evidence outside its declared trust classes",
        ));
    }
    Ok(())
}

fn is_gap_kind(value: &str) -> bool {
    matches!(
        value,
        "unsafe" | "foreign" | "missing" | "stale" | "unsupported"
    ) || is_symbol(value)
}

fn normalized(values: &[String]) -> bool {
    values.windows(2).all(|pair| pair[0] < pair[1])
}

fn is_ref(value: &str) -> bool {
    !value.is_empty() && value.len() <= 128
}

fn add_id(value: &str, ids: &mut HashSet<String>) -> Result<()> {
    if is_ref(value) && ids.insert(value.to_owned()) {
        Ok(())
    } else {
        Err(invalid("evidence IDs must be unique non-empty ref-ids"))
    }
}

fn invalid(message: impl Into<String>) -> Diagnostic {
    Diagnostic::plain(INVALID_VERIFICATION, message)
}
