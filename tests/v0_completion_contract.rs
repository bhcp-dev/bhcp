use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

use bhcp::hash::HashAlgorithm;
use bhcp::model::ContentReference;
use bhcp::pipeline::{compile_source, parse_policy_source};
use bhcp::policy::{ExactNumber, WaiverDocument, WaiverWeakening, apply_waiver, compose_policies};
use bhcp::profile::PresentationDocument;
use bhcp::schema::{parse_diagnostic, validate_root, validate_schema_inventory};
use bhcp::value::Value;

const ISSUE_KEYS: &[(u64, &str)] = &[
    (99, "v0-completion-contract"),
    (100, "definition-source-parser"),
    (101, "goal-source-parser"),
    (102, "governance-source-parser"),
    (103, "complete-type-checker"),
    (104, "expression-calculus"),
    (105, "function-predicate-elaboration"),
    (106, "ownership-analysis"),
    (107, "effect-authority-budget-analysis"),
    (108, "policy-waiver-lowering"),
    (109, "profile-source-lowering"),
    (110, "extension-lowering"),
    (111, "recursion-retention-lowering"),
    (112, "frontend-completion-audit"),
    (113, "graph-core-model"),
    (114, "obligation-graph-builder"),
    (115, "capability-graph-builder"),
    (116, "state-graph-builder"),
    (117, "obligation-proof-checker"),
    (118, "graph-completion-audit"),
    (119, "planner-boundary"),
    (120, "conflict-scheduler"),
    (121, "budget-retry-planner"),
    (122, "execution-graph-builder"),
    (123, "capability-executor-runtime"),
    (124, "state-cas-runtime"),
    (125, "execution-lifecycle-runtime"),
    (126, "evidence-graph-assembly"),
    (127, "runtime-policy-enforcement"),
    (128, "coding-agent-backend"),
    (129, "rust-sdk"),
    (130, "complete-cli"),
    (131, "complete-v0-conformance"),
    (132, "reference-program-e2e"),
    (133, "v0-security-audit"),
    (134, "v0-completion-audit"),
];

const STAGES: &[&str] = &[
    "source",
    "checking",
    "graphs",
    "planning",
    "execution",
    "evidence",
    "sdk",
    "cli",
    "conformance",
    "certification",
];

const ARTIFACTS: &[&str] = &[
    "program",
    "program-profiled",
    "program-contract",
    "registry",
    "policy",
    "waiver",
    "waiver-projection",
    "syntax",
    "syntax-projection",
    "profile",
    "profile-projection",
    "extension",
    "extension-projection",
    "extension-type-rule",
    "extension-effect-rule",
    "extension-policy-rule",
    "extension-normalization-rule",
    "extension-evidence-rule",
    "policy-evidence-registry",
    "planner-input",
    "execution-input",
    "expected-obligations",
    "outcome-matrix",
];

const FEATURES: &[&str] = &[
    "typed-functions-and-predicates",
    "nested-goals-and-recursion",
    "effects-and-ownership",
    "policy-boundary",
    "waiver-boundary",
    "custom-profile",
    "derived-extension",
    "planning",
    "execution",
    "per-obligation-evidence",
];

const NON_GOALS: &[&str] = &[
    "full-theorem-proving",
    "unrestricted-macros-or-grammar-plugins",
    "comprehensive-temporal-or-reactive-logic",
    "universal-workflow-synthesis",
];

const REGISTRY: &[(&str, &str)] = &[
    ("canonical-program", "program.bhcp"),
    ("alternate-program", "program.words.bhcp"),
    ("program-contract", "program-contract.txt"),
    ("syntax-source", "syntax.bhcp"),
    ("syntax-document", "syntax.diag"),
    ("syntax-symbol", "bhcp.reference/words@0"),
    ("profile-source", "profile.bhcp"),
    ("profile-document", "profile.diag"),
    ("profile-symbol", "bhcp.reference/review-profile@0"),
    ("policy-source", "policy.bhcp"),
    (
        "organization-policy",
        "bhcp.reference/organization-policy@0",
    ),
    ("repository-policy", "bhcp.reference/repository-policy@0"),
    ("waiver-source", "waiver.bhcp"),
    ("waiver-document", "waiver.diag"),
    ("waiver-symbol", "bhcp.reference/offline-emergency-waiver@0"),
    ("waiver-decision-at", "2026-01-01T00:30:00Z"),
    ("extension-source", "extension.bhcp"),
    ("extension-document", "extension.diag"),
    ("extension-symbol", "bhcp.reference/review@0"),
    ("extension-lowering", "bhcp.reference/lowerReview@0"),
    ("extension-type-rule", "extension-type.rule"),
    ("extension-effect-rule", "extension-effect.rule"),
    ("extension-policy-rule", "extension-policy.rule"),
    (
        "extension-normalization-rule",
        "extension-normalization.rule",
    ),
    ("extension-evidence-rule", "extension-evidence.rule"),
    ("policy-evidence-registry", "policy-evidence-registry.txt"),
    ("planner-input", "planner-input.txt"),
    ("execution-input", "execution-input.txt"),
    ("expected-obligations", "expected-obligations.txt"),
    ("outcome-matrix", "outcome-matrix.txt"),
];

const OBLIGATIONS: &[&str] = &[
    "source|bhcp.reference/DeliverChange@0:attempts|limit|formal|open",
    "source|bhcp.reference/DeliverChange@0:non-empty-digest|contract|formal|open",
    "source|bhcp.reference/DeliverChange@0:tree-depth-matches|contract|formal|open",
    "source|bhcp.reference/Approve@0:high-risk|contract|human-approved|open",
    "source|bhcp.reference/Persist@0:safe-result|contract|static|open",
    "source|bhcp.reference/Persist@0:stored|contract|formal|open",
    "source|bhcp.reference/WalkTree@0:depth|limit|formal|open",
    "source|bhcp.reference/WalkTree@0:leaf-at-zero|contract|formal|open",
    "source|bhcp.reference/WalkTree@0:non-negative-depth|contract|formal|open",
    "policy|bhcp.reference/limit.attempts@0|limit|formal|open",
    "policy|bhcp.reference/obligation.static-analysis@0|evidence-demand|static|open",
    "policy|bhcp.reference/obligation.human-approval@0|evidence-demand|human-approved|open",
];

const OUTCOMES: &[&str] = &[
    "satisfied|execution|completed|satisfied",
    "refuted|execution|completed|refuted",
    "unresolved|execution|completed|unresolved:missing-evidence",
    "policy-denied|planning|refused|policy-denied",
    "budget-refused|planning|refused|budget-exhausted",
    "stale|execution|completed|unresolved:stale-evidence",
    "cancelled|execution|completed|unresolved:cancelled",
    "faulted|execution|faulted|operational-fault",
];

const POLICY_EVIDENCE_BINDINGS: &[&str] = &[
    "bhcp.reference/obligation.human-approval@0|bhcp.verifier/human-approval@0|bhcp.reference/Approve@0:high-risk",
    "bhcp.reference/obligation.static-analysis@0|bhcp.verifier/static-analysis@0|bhcp.reference/Persist@0:safe-result",
];

const EXTENSION_RULES: &[(&str, &str, &str)] = &[
    (
        "type_rule",
        "extension-type-rule",
        "extension: bhcp.reference/review@0\ninput: bhcp.reference/Risk@0\noutput: Unit\nchildren: []\n",
    ),
    (
        "effect_rule",
        "extension-effect-rule",
        "extension: bhcp.reference/review@0\neffects: []\n",
    ),
    (
        "policy_rule",
        "extension-policy-rule",
        "extension: bhcp.reference/review@0\npolicy: inherit-enclosing-without-override\n",
    ),
    (
        "normalization_rule",
        "extension-normalization-rule",
        "extension: bhcp.reference/review@0\nnormalization: lower-completely-to-kernel-network\nreducer: bhcp.reference/reviewReducer@0\n",
    ),
    (
        "evidence_rule",
        "extension-evidence-rule",
        "extension: bhcp.reference/review@0\nevidence: checked-kernel-derivation\n",
    ),
];

const PROGRAM_SEMANTICS: &[&str] = &[
    "type|bhcp.reference/StartDelivery@0|output|token|Text",
    "type|bhcp.reference/ConfirmDelivery@0|input|token|Text",
    "type|bhcp.reference/ConfirmDelivery@0|output|confirmation|Text",
    "type|bhcp.reference/Approve@0|input|risk|bhcp.reference/Risk@0",
    "type|bhcp.reference/Approve@0|output|approval|Text",
    "type|bhcp.reference/Persist@0|input|patch|owned affine bhcp.reference/Patch@0",
    "type|bhcp.reference/Persist@0|resource|repository|owned linear bhcp.reference/Repository@0",
    "type|bhcp.reference/Persist@0|output|receipt|Result<bhcp.reference/Receipt@0,bhcp.reference/DeliveryError@0>",
    "type|bhcp.reference/WalkTree@0|input|node|bhcp.reference/Node@0",
    "type|bhcp.reference/WalkTree@0|input|remaining|Integer",
    "type|bhcp.reference/WalkTree@0|output|result|Unit",
    "type|bhcp.reference/DeliverChange@0|input|patch|owned affine bhcp.reference/Patch@0",
    "type|bhcp.reference/DeliverChange@0|input|risk|bhcp.reference/Risk@0",
    "type|bhcp.reference/DeliverChange@0|input|tree|bhcp.reference/Node@0",
    "type|bhcp.reference/DeliverChange@0|input|tree_depth|Integer",
    "type|bhcp.reference/DeliverChange@0|resource|repository|owned linear bhcp.reference/Repository@0",
    "type|bhcp.reference/DeliverChange@0|state|attempts|Integer",
    "type|bhcp.reference/DeliverChange@0|output|outcome|bhcp.reference/Delivery@0",
    "type|bhcp.reference/review@0|input|risk|bhcp.reference/Risk@0",
    "type|bhcp.reference/review@0|output|result|Unit",
    "clause|bhcp.reference/WalkTree@0|non-negative-depth|requires|Bool|0 <= remaining",
    "clause|bhcp.reference/WalkTree@0|leaf-at-zero|requires|Bool|remaining != 0 || node.children == []",
    "clause|bhcp.reference/DeliverChange@0|non-empty-digest|requires|Bool|bhcp.reference/nonEmpty@0(patch.digest)",
    "clause|bhcp.reference/DeliverChange@0|tree-depth-matches|requires|Bool|tree.depth == tree_depth",
    "clause|bhcp.reference/Persist@0|safe-result|ensures|Bool|match receipt",
    "limit|bhcp.reference/WalkTree@0|depth|bhcp.reference/limit.depth@0|remaining|64|Bool",
    "limit|bhcp.reference/DeliverChange@0|attempts|bhcp.reference/limit.attempts@0|attempts|3|Bool",
    "effect|bhcp.reference/Persist@0|allow|bhcp-effect/fs.read@0|resource.repository",
    "effect|bhcp.reference/Persist@0|allow|bhcp-effect/fs.write@0|resource.repository",
    "effect|bhcp.reference/Persist@0|forbid|bhcp-effect/network@0|-",
    "effect|bhcp.reference/DeliverChange@0|allow|bhcp-effect/fs.read@0|resource.repository",
    "effect|bhcp.reference/DeliverChange@0|allow|bhcp-effect/fs.write@0|resource.repository",
    "effect|bhcp.reference/DeliverChange@0|allow|bhcp-effect/process@0|literal.cargo",
    "effect|bhcp.reference/DeliverChange@0|forbid|bhcp-effect/network@0|-",
    "recursion|bhcp.reference/WalkTree@0|children.*|well-founded|remaining|remaining - 1|0 <= remaining|remaining != 0 || node.children == []",
    "chain|bhcp.reference/DeliverChange@0|sequence|sequence.1.started|bhcp.reference/StartDelivery@0|input-free|-",
    "chain|bhcp.reference/DeliverChange@0|sequence|sequence.2.confirmed|bhcp.reference/ConfirmDelivery@0|predecessor-whole|step.sequence.1.started",
    "reducer|bhcp.reference/reviewReducer@0|bhcp.reference/Risk@0|{}|Reduction<Unit>|pending-or-concluded",
    "lowerer|bhcp.reference/lowerReview@0|Meta<DerivedForm,bhcp.reference/Risk@0,Unit>|Meta<NetworkShape,bhcp.reference/Risk@0,Unit>|bhcp/meta.network-shape@0",
    "extension-shape|bhcp.reference/review@0|bhcp.reference/Risk@0|Unit|no-children",
];

const PROGRAM_DEFINITIONS: &[(&str, &str)] = &[
    ("type", "bhcp.reference/NonEmptyText@0"),
    ("type", "bhcp.reference/Risk@0"),
    ("type", "bhcp.reference/Patch@0"),
    ("type", "bhcp.reference/Repository@0"),
    ("type", "bhcp.reference/Receipt@0"),
    ("type", "bhcp.reference/DeliveryError@0"),
    ("type", "bhcp.reference/Node@0"),
    ("type", "bhcp.reference/WalkInput@0"),
    ("type", "bhcp.reference/Delivery@0"),
    ("function", "bhcp.reference/isHighRisk@0"),
    ("predicate", "bhcp.reference/nonEmpty@0"),
    ("goal", "bhcp.reference/StartDelivery@0"),
    ("goal", "bhcp.reference/ConfirmDelivery@0"),
    ("goal", "bhcp.reference/Approve@0"),
    ("goal", "bhcp.reference/Persist@0"),
    ("goal", "bhcp.reference/WalkTree@0"),
    ("goal", "bhcp.reference/DeliverChange@0"),
    ("function", "bhcp.reference/reviewReducer@0"),
    ("function", "bhcp.reference/lowerReview@0"),
    ("extension", "bhcp.reference/review@0"),
];

#[derive(Debug)]
struct Stage {
    owners: Vec<String>,
    outcome: String,
}

#[derive(Debug)]
struct Feature {
    artifact: String,
    owners: Vec<String>,
    needles: Vec<String>,
}

#[derive(Debug, Default)]
struct CompletionContract {
    version: Option<String>,
    issues: BTreeMap<u64, String>,
    scenarios: BTreeMap<String, String>,
    roots: BTreeMap<String, String>,
    stages: BTreeMap<String, Stage>,
    artifacts: BTreeMap<String, PathBuf>,
    features: BTreeMap<String, Feature>,
    non_goals: BTreeSet<String>,
}

fn repository() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn insert_unique<K: Ord, V>(
    map: &mut BTreeMap<K, V>,
    key: K,
    value: V,
    kind: &str,
) -> Result<(), String> {
    if map.insert(key, value).is_some() {
        return Err(format!("duplicate {kind}"));
    }
    Ok(())
}

fn comma_list(field: &str) -> Vec<String> {
    field.split(',').map(str::to_owned).collect()
}

fn parse_contract(text: &str) -> Result<CompletionContract, String> {
    let mut contract = CompletionContract::default();
    for (index, line) in text.lines().enumerate() {
        let line_number = index + 1;
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields = line.split('|').collect::<Vec<_>>();
        match fields.as_slice() {
            ["version", version] => {
                if contract.version.replace((*version).to_owned()).is_some() {
                    return Err("duplicate version".to_owned());
                }
            }
            ["issue", number, key] => {
                let number = number
                    .parse::<u64>()
                    .map_err(|_| format!("line {line_number}: invalid issue number"))?;
                insert_unique(&mut contract.issues, number, (*key).to_owned(), "issue")?;
            }
            ["scenario", id, owner] => insert_unique(
                &mut contract.scenarios,
                (*id).to_owned(),
                (*owner).to_owned(),
                "scenario",
            )?,
            ["root", kind, owner] => insert_unique(
                &mut contract.roots,
                (*kind).to_owned(),
                (*owner).to_owned(),
                "root",
            )?,
            ["stage", id, owners, outcome] => insert_unique(
                &mut contract.stages,
                (*id).to_owned(),
                Stage {
                    owners: comma_list(owners),
                    outcome: (*outcome).to_owned(),
                },
                "stage",
            )?,
            ["artifact", id, path] => insert_unique(
                &mut contract.artifacts,
                (*id).to_owned(),
                PathBuf::from(path),
                "artifact",
            )?,
            ["feature", id, artifact, owners, needles] => insert_unique(
                &mut contract.features,
                (*id).to_owned(),
                Feature {
                    artifact: (*artifact).to_owned(),
                    owners: comma_list(owners),
                    needles: comma_list(needles),
                },
                "feature",
            )?,
            ["non-goal", id] => {
                if !contract.non_goals.insert((*id).to_owned()) {
                    return Err("duplicate non-goal".to_owned());
                }
            }
            _ => return Err(format!("line {line_number}: unknown or malformed record")),
        }
    }
    Ok(contract)
}

fn markdown_scenarios(readme: &str) -> BTreeSet<String> {
    let mut scenarios = BTreeSet::new();
    for line in readme.lines().filter(|line| line.starts_with("| ")) {
        let Some(id) = line.split('|').nth(1).map(str::trim) else {
            continue;
        };
        if id == "ID" || !id.as_bytes().first().is_some_and(u8::is_ascii_uppercase) {
            continue;
        }
        if let Some(behavior) = id.strip_prefix("ALG-") {
            for verdict in ["S", "R", "U", "F"] {
                scenarios.insert(format!("ALG-{behavior}-{verdict}"));
            }
        } else {
            scenarios.insert(id.to_owned());
        }
    }
    scenarios
}

fn schema_roots(example_manifest: &str) -> BTreeSet<String> {
    example_manifest
        .lines()
        .filter_map(|line| line.split_whitespace().nth(1))
        .map(str::to_owned)
        .collect()
}

fn expected_set(values: &[&str]) -> BTreeSet<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

fn reference_directory(root: &Path) -> PathBuf {
    root.join("conformance/v0/reference-program")
}

fn read_reference(root: &Path, name: &str) -> Result<String, String> {
    fs::read_to_string(reference_directory(root).join(name))
        .map_err(|error| format!("cannot read reference artifact {name}: {error}"))
}

fn parse_registry(text: &str) -> Result<BTreeMap<String, String>, String> {
    let mut registry = BTreeMap::new();
    for (index, line) in text.lines().enumerate() {
        let fields = line.split('|').collect::<Vec<_>>();
        let [key, value] = fields.as_slice() else {
            return Err(format!("registry line {} is malformed", index + 1));
        };
        insert_unique(
            &mut registry,
            (*key).to_owned(),
            (*value).to_owned(),
            "registry key",
        )?;
    }
    Ok(registry)
}

fn value_text<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    match value.get(key) {
        Some(Value::Text(text)) => Ok(text),
        _ => Err(format!("projection field {key} is not text")),
    }
}

#[derive(Clone, Debug)]
struct DataCall {
    parent: String,
    step: String,
    callee: String,
    argument: String,
    mode: String,
    source: String,
}

#[derive(Debug, Default)]
struct ProgramContract {
    definitions: BTreeSet<(String, String)>,
    facts: BTreeMap<(String, String, String), String>,
    consumes: BTreeSet<(String, String, String)>,
    calls: Vec<DataCall>,
    calls0: Vec<(String, String, String)>,
    semantics: BTreeSet<String>,
}

fn parse_program_contract(text: &str) -> Result<ProgramContract, String> {
    let mut contract = ProgramContract::default();
    for (index, line) in text.lines().enumerate() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields = line.split('|').collect::<Vec<_>>();
        match fields.as_slice() {
            ["definition", kind, symbol] => {
                if !contract
                    .definitions
                    .insert(((*kind).to_owned(), (*symbol).to_owned()))
                {
                    return Err("duplicate program definition".to_owned());
                }
            }
            ["fact", owner, kind, name, mode] => insert_unique(
                &mut contract.facts,
                ((*owner).to_owned(), (*kind).to_owned(), (*name).to_owned()),
                (*mode).to_owned(),
                "program fact",
            )?,
            ["consume", owner, source, destination] => {
                if !contract.consumes.insert((
                    (*owner).to_owned(),
                    (*source).to_owned(),
                    (*destination).to_owned(),
                )) {
                    return Err("duplicate program consumption".to_owned());
                }
            }
            ["call", parent, step, callee, argument, mode, source] => {
                contract.calls.push(DataCall {
                    parent: (*parent).to_owned(),
                    step: (*step).to_owned(),
                    callee: (*callee).to_owned(),
                    argument: (*argument).to_owned(),
                    mode: (*mode).to_owned(),
                    source: (*source).to_owned(),
                });
            }
            ["call0", parent, step, callee] => contract.calls0.push((
                (*parent).to_owned(),
                (*step).to_owned(),
                (*callee).to_owned(),
            )),
            [kind, ..]
                if matches!(
                    *kind,
                    "type"
                        | "clause"
                        | "limit"
                        | "effect"
                        | "recursion"
                        | "chain"
                        | "reducer"
                        | "lowerer"
                        | "extension-shape"
                ) =>
            {
                if !contract.semantics.insert(line.to_owned()) {
                    return Err("duplicate semantic projection row".to_owned());
                }
            }
            _ => {
                return Err(format!("program contract line {} is malformed", index + 1));
            }
        }
    }
    Ok(contract)
}

fn fact_mode<'a>(
    contract: &'a ProgramContract,
    owner: &str,
    kind: &str,
    name: &str,
) -> Result<&'a str, String> {
    contract
        .facts
        .get(&(owner.to_owned(), kind.to_owned(), name.to_owned()))
        .map(String::as_str)
        .ok_or_else(|| format!("unknown {kind} {owner}:{name}"))
}

fn definition_block<'a>(source: &'a str, kind: &str, symbol: &str) -> Result<&'a str, String> {
    let marker = format!("§{kind} {symbol}");
    let start = source
        .find(&marker)
        .ok_or_else(|| format!("source omits {marker}"))?;
    let tail = &source[start..];
    let end = tail[marker.len()..]
        .find("\n§")
        .map(|offset| marker.len() + offset)
        .unwrap_or(tail.len());
    Ok(&tail[..end])
}

fn compact(text: &str) -> String {
    text.chars()
        .filter(|character| !character.is_whitespace())
        .collect()
}

fn validate_program_projection_sources(
    text: &str,
    canonical: &str,
    extension_source: &str,
) -> Result<(), String> {
    let projection = parse_program_contract(text)?;
    if projection.semantics != expected_set(PROGRAM_SEMANTICS) {
        return Err("program semantic projection mismatch".to_owned());
    }
    let expected_definitions = PROGRAM_DEFINITIONS
        .iter()
        .map(|(kind, symbol)| ((*kind).to_owned(), (*symbol).to_owned()))
        .collect::<BTreeSet<_>>();
    if projection.definitions != expected_definitions {
        return Err("program definition projection mismatch".to_owned());
    }
    let definition_symbols = projection
        .definitions
        .iter()
        .map(|(_, symbol)| symbol.as_str())
        .collect::<BTreeSet<_>>();

    for (kind, symbol) in &projection.definitions {
        if !matches!(
            kind.as_str(),
            "type" | "function" | "predicate" | "goal" | "extension"
        ) {
            return Err(format!("unknown definition kind {kind}"));
        }
        let marker = format!("§{kind} {symbol}");
        let occurrences =
            canonical.matches(&marker).count() + extension_source.matches(&marker).count();
        if occurrences != 1 {
            return Err(format!(
                "definition marker {marker:?} occurs {occurrences} times"
            ));
        }
    }

    for row in projection
        .semantics
        .iter()
        .filter(|row| row.starts_with("type|"))
    {
        let fields = row.splitn(6, '|').collect::<Vec<_>>();
        let [_, owner, kind, name, value_type] = fields.as_slice() else {
            return Err("typed projection row is malformed".to_owned());
        };
        if *owner == "bhcp.reference/review@0"
            || (*owner == "bhcp.reference/WalkTree@0" && *kind == "output")
        {
            continue;
        }
        let block = definition_block(canonical, "goal", owner)?;
        let marker = format!("§{kind} {name}: {value_type};");
        if !compact(block).contains(&compact(&marker)) {
            return Err(format!("{owner} omits owner-scoped typed fact {marker}"));
        }
    }

    for row in projection
        .semantics
        .iter()
        .filter(|row| row.starts_with("clause|"))
    {
        let fields = row.splitn(6, '|').collect::<Vec<_>>();
        let [_, owner, label, contract, _, expression] = fields.as_slice() else {
            return Err("clause projection row is malformed".to_owned());
        };
        let block = definition_block(canonical, "goal", owner)?;
        let marker = format!("§{contract} \"{label}\": {expression}");
        if !compact(block).contains(&compact(&marker)) {
            return Err(format!("{owner} omits owner-scoped clause {marker}"));
        }
    }

    let deliver = definition_block(canonical, "goal", "bhcp.reference/DeliverChange@0")?;
    let persist = definition_block(canonical, "goal", "bhcp.reference/Persist@0")?;
    let walk = definition_block(canonical, "goal", "bhcp.reference/WalkTree@0")?;
    for (block, marker) in [
        (
            walk,
            "§limit \"depth\": bhcp.reference/limit.depth@0: remaining <= 64;",
        ),
        (walk, "remaining = remaining - 1"),
        (
            persist,
            "§allows bhcp-effect/fs.read@0(repository), bhcp-effect/fs.write@0(repository);",
        ),
        (persist, "§forbids bhcp-effect/network@0;"),
        (
            deliver,
            "§limit \"attempts\": bhcp.reference/limit.attempts@0: attempts <= 3;",
        ),
        (
            deliver,
            "§allows bhcp-effect/fs.read@0(repository), bhcp-effect/fs.write@0(repository), bhcp-effect/process@0(\"cargo\");",
        ),
        (deliver, "§forbids bhcp-effect/network@0;"),
        (deliver, "started = bhcp.reference/StartDelivery@0();"),
        (
            deliver,
            "confirmed = bhcp.reference/ConfirmDelivery@0(token = started);",
        ),
    ] {
        if !compact(block).contains(&compact(marker)) {
            return Err(format!("owner-scoped source omits {marker}"));
        }
    }

    let mut steps = BTreeMap::<(String, String), String>::new();
    let mut uses = BTreeMap::<(String, String), usize>::new();
    for (parent, step, callee) in &projection.calls0 {
        if !definition_symbols.contains(parent.as_str())
            || !definition_symbols.contains(callee.as_str())
        {
            return Err("input-free call has an unknown definition".to_owned());
        }
        if projection.facts.keys().any(|(owner, kind, _)| {
            owner == callee && matches!(kind.as_str(), "input" | "resource")
        }) {
            return Err(format!("first chain child {callee} is not input-free"));
        }
        if steps
            .insert((parent.clone(), step.clone()), callee.clone())
            .is_some()
        {
            return Err(format!("duplicate input-free step {step}"));
        }
        let block = definition_block(canonical, "goal", parent)?;
        if !compact(block).contains(&compact(&format!("{callee}();"))) {
            return Err(format!("source omits input-free call {callee}"));
        }
    }
    for call in &projection.calls {
        if !definition_symbols.contains(call.parent.as_str()) {
            return Err(format!("call has unknown parent {}", call.parent));
        }
        if !definition_symbols.contains(call.callee.as_str()) {
            return Err(format!("call has unknown callee {}", call.callee));
        }
        let target_kind = if projection.facts.contains_key(&(
            call.callee.clone(),
            "input".to_owned(),
            call.argument.clone(),
        )) {
            "input"
        } else {
            "resource"
        };
        fact_mode(&projection, &call.callee, target_kind, &call.argument)?;
        if !matches!(call.mode.as_str(), "copy" | "move") {
            return Err(format!("call {} has unknown transfer mode", call.step));
        }

        let source_mode = if let Some(name) = call.source.strip_prefix("input.") {
            fact_mode(&projection, &call.parent, "input", name)?
        } else if let Some(name) = call.source.strip_prefix("resource.") {
            fact_mode(&projection, &call.parent, "resource", name)?
        } else if let Some(step) = call.source.strip_prefix("step.") {
            let callee = steps
                .get(&(call.parent.clone(), step.to_owned()))
                .ok_or_else(|| format!("call {} reads unknown or later step {step}", call.step))?;
            projection
                .facts
                .iter()
                .find_map(|((owner, kind, _), mode)| {
                    (owner == callee && kind == "output").then_some(mode.as_str())
                })
                .ok_or_else(|| format!("step {step} has no declared output"))?
        } else if call.source == "quantifier.child" || call.source.starts_with("expression.") {
            "unrestricted"
        } else {
            return Err(format!(
                "call {} has unknown source {}",
                call.step, call.source
            ));
        };
        if source_mode.starts_with("owned-") != (call.mode == "move") {
            return Err(format!(
                "call {} must move owned values and copy unrestricted values",
                call.step
            ));
        }
        *uses
            .entry((call.parent.clone(), call.source.clone()))
            .or_default() += 1;

        let step_key = (call.parent.clone(), call.step.clone());
        if let Some(existing) = steps.get(&step_key) {
            if existing != &call.callee {
                return Err(format!("step {} names multiple callees", call.step));
            }
        } else {
            steps.insert(step_key, call.callee.clone());
        }

        let call_marker = format!("{}(", call.callee);
        if !canonical.contains(&call_marker) {
            return Err(format!("source omits projected call {call_marker}"));
        }
        let source_name = call
            .source
            .strip_prefix("expression.")
            .unwrap_or_else(|| call.source.rsplit('.').next().unwrap_or(&call.source));
        let argument_marker = if call.mode == "move" {
            format!("{} = move {source_name}", call.argument)
        } else {
            format!("{} = {source_name}", call.argument)
        };
        if !canonical.contains(&argument_marker) {
            return Err(format!("source omits projected argument {argument_marker}"));
        }
    }

    for marker in [
        "§type bhcp.reference/Risk@0 = variant { Low, High };",
        "§type bhcp.reference/Patch@0 = { bytes: Bytes, digest: Text };",
        "§type bhcp.reference/Repository@0 = { root: Text };",
        "§type bhcp.reference/WalkInput@0 = { node: bhcp.reference/Node@0, remaining: Integer };",
        "§input patch: owned affine bhcp.reference/Patch@0;",
        "§output token: Text;",
        "§input token: Text;",
        "§resource repository: owned linear bhcp.reference/Repository@0;",
        "§input remaining: Integer;",
        "§input tree_depth: Integer;",
        "§state attempts: Integer;",
        "§output outcome: bhcp.reference/Delivery@0;",
        "§requires \"non-negative-depth\": 0 <= remaining;",
        "§requires \"leaf-at-zero\": remaining != 0 || node.children == [];",
        "§requires \"tree-depth-matches\": tree.depth == tree_depth;",
        "§limit \"depth\": bhcp.reference/limit.depth@0: remaining <= 64;",
        "§limit \"attempts\": bhcp.reference/limit.attempts@0: attempts <= 3;",
        "remaining = remaining - 1",
        "§allows bhcp-effect/fs.read@0(repository), bhcp-effect/fs.write@0(repository);",
        "§allows bhcp-effect/fs.read@0(repository), bhcp-effect/fs.write@0(repository), bhcp-effect/process@0(\"cargo\");",
    ] {
        if !canonical.contains(marker) {
            return Err(format!("source omits semantic projection marker {marker}"));
        }
    }
    for marker in [
        "parent: bhcp.reference/Risk@0",
        "observations: {}",
        "): Reduction<Unit> =",
        "bhcp/kernel.pending@0",
        "bhcp/kernel.conclude@0",
        "bhcp/meta.network-shape@0(",
        "Meta<DerivedForm, bhcp.reference/Risk@0, Unit>",
        "Meta<NetworkShape, bhcp.reference/Risk@0, Unit>",
    ] {
        if !extension_source.contains(marker) {
            return Err(format!(
                "extension omits semantic projection marker {marker}"
            ));
        }
    }

    for (owner, kind, name) in projection
        .facts
        .keys()
        .filter(|(_, kind, _)| matches!(kind.as_str(), "input" | "resource"))
    {
        let mode = fact_mode(&projection, owner, kind, name)?;
        if !mode.starts_with("owned-") {
            continue;
        }
        let source = format!("{kind}.{name}");
        let call_uses = uses
            .get(&(owner.clone(), source.clone()))
            .copied()
            .unwrap_or_default();
        let local_uses = projection
            .consumes
            .iter()
            .filter(|(candidate_owner, candidate_source, _)| {
                candidate_owner == owner && candidate_source == &source
            })
            .count();
        if call_uses + local_uses != 1 {
            return Err(format!(
                "owned value {owner}:{source} must be consumed exactly once"
            ));
        }
    }

    for ((parent, step), callee) in &steps {
        let Some(output_mode) = projection
            .facts
            .iter()
            .find_map(|((owner, kind, _), mode)| {
                (owner == callee && kind == "output").then_some(mode)
            })
        else {
            return Err(format!("step {step} has no output fact"));
        };
        if output_mode.starts_with("owned-") {
            let source = format!("step.{step}");
            if uses.get(&(parent.clone(), source)).copied() != Some(1) {
                return Err(format!(
                    "owned step output {parent}:{step} is not moved exactly once"
                ));
            }
        }
    }
    Ok(())
}

fn validate_program_projection(root: &Path, text: &str) -> Result<(), String> {
    validate_program_projection_sources(
        text,
        &read_reference(root, "program.bhcp")?,
        &read_reference(root, "extension.bhcp")?,
    )
}

fn validate_program_contract(root: &Path) -> Result<(), String> {
    validate_program_projection(root, &read_reference(root, "program-contract.txt")?)
}

fn validate_reference_policy(text: &str) -> Result<(), String> {
    let parsed_policy =
        parse_policy_source(text, "policy.bhcp").map_err(|error| error.to_string())?;
    let policy_symbols = parsed_policy
        .documents
        .iter()
        .map(|document| document.symbol.as_str())
        .collect::<BTreeSet<_>>();
    if policy_symbols
        != BTreeSet::from([
            "bhcp.reference/organization-policy@0",
            "bhcp.reference/repository-policy@0",
        ])
    {
        return Err("reference policy symbol inventory mismatch".to_owned());
    }
    let effective_policy = compose_policies(&parsed_policy.documents, HashAlgorithm::default())
        .map_err(|error| error.to_string())?;
    let capability_effects = effective_policy
        .effective
        .capabilities
        .iter()
        .map(|rule| rule.value.effect.as_str())
        .collect::<BTreeSet<_>>();
    let capability_goals = effective_policy
        .effective
        .capabilities
        .iter()
        .map(|rule| {
            (
                rule.value.effect.as_str(),
                rule.value
                    .scope
                    .as_ref()
                    .and_then(|scope| scope.goals.as_ref())
                    .cloned(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    if capability_effects
        != BTreeSet::from([
            "bhcp-effect/fs.read@0",
            "bhcp-effect/fs.write@0",
            "bhcp-effect/process@0",
        ])
        || capability_goals.get("bhcp-effect/fs.read@0")
            != Some(&Some(vec![
                "bhcp.reference/DeliverChange@0".to_owned(),
                "bhcp.reference/Persist@0".to_owned(),
            ]))
        || capability_goals.get("bhcp-effect/fs.write@0")
            != Some(&Some(vec![
                "bhcp.reference/DeliverChange@0".to_owned(),
                "bhcp.reference/Persist@0".to_owned(),
            ]))
        || capability_goals.get("bhcp-effect/process@0")
            != Some(&Some(vec!["bhcp.reference/DeliverChange@0".to_owned()]))
        || !effective_policy
            .effective
            .prohibitions
            .iter()
            .any(|rule| rule.value.effect == "bhcp-effect/network@0")
        || !effective_policy.effective.limits.iter().any(|rule| {
            rule.value.dimension == "bhcp.reference/limit.attempts@0"
                && rule.value.maximum == ExactNumber::Integer(2)
        })
    {
        return Err(
            "reference policy does not authorize the declared effects and base limit".to_owned(),
        );
    }
    Ok(())
}

fn validate_policy_evidence_bindings(canonical: &str, text: &str) -> Result<(), String> {
    let bindings = text.lines().map(str::to_owned).collect::<BTreeSet<_>>();
    if bindings != expected_set(POLICY_EVIDENCE_BINDINGS) {
        return Err("policy evidence producer registry mismatch".to_owned());
    }
    for binding in &bindings {
        let fields = binding.split('|').collect::<Vec<_>>();
        let [obligation, producer, target] = fields.as_slice() else {
            return Err("policy evidence producer binding is malformed".to_owned());
        };
        let (goal, label) = target
            .split_once(':')
            .ok_or_else(|| "policy evidence target is malformed".to_owned())?;
        if !OBLIGATIONS
            .iter()
            .any(|row| row.starts_with(&format!("policy|{obligation}|evidence-demand|")))
            || !canonical.contains(&format!("with {producer}"))
            || !canonical.contains(&format!("§goal {goal}"))
            || !canonical.contains(&format!("\"{label}\""))
        {
            return Err("policy evidence producer binding is disconnected".to_owned());
        }
    }
    Ok(())
}

fn validate_reference_semantics(root: &Path) -> Result<(), String> {
    let directory = reference_directory(root);
    let registry = parse_registry(&read_reference(root, "registry.txt")?)?;
    let expected_registry = REGISTRY
        .iter()
        .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
        .collect::<BTreeMap<_, _>>();
    if registry != expected_registry {
        return Err("reference registry mismatch".to_owned());
    }
    for (key, value) in &registry {
        if (key.ends_with("program")
            || key.ends_with("source")
            || key.ends_with("document")
            || matches!(
                key.as_str(),
                "program-contract"
                    | "planner-input"
                    | "execution-input"
                    | "expected-obligations"
                    | "outcome-matrix"
                    | "policy-evidence-registry"
            )
            || key.ends_with("-rule"))
            && !directory.join(value).is_file()
        {
            return Err(format!("registry path {key} does not exist"));
        }
    }

    let canonical = read_reference(root, "program.bhcp")?;
    let alternate = read_reference(root, "program.words.bhcp")?;
    let (preamble, alternate_body) = alternate
        .split_once('\n')
        .ok_or_else(|| "alternate program has no preamble".to_owned())?;
    if preamble != "#!bhcp-profile bhcp.reference/review-profile@0" {
        return Err("alternate program selects the wrong profile".to_owned());
    }
    if alternate_body.replace("§intent", "§goal") != canonical {
        return Err("canonical and alternate source structures differ".to_owned());
    }

    validate_reference_policy(&read_reference(root, "policy.bhcp")?)?;

    let syntax_value = parse_diagnostic(&read_reference(root, "syntax.diag")?)
        .map_err(|error| error.to_string())?;
    validate_root(&syntax_value, "syntax").map_err(|error| error.to_string())?;
    let PresentationDocument::Syntax(syntax) =
        PresentationDocument::from_value(&syntax_value).map_err(|error| error.to_string())?
    else {
        return Err("syntax projection has the wrong root".to_owned());
    };
    if syntax.symbol != "bhcp.reference/words@0"
        || syntax.mappings.len() != 1
        || syntax.mappings[0].canonical != "goal"
        || syntax.mappings[0].surface != "intent"
    {
        return Err("syntax projection does not define the alternate source mapping".to_owned());
    }
    let syntax_source = read_reference(root, "syntax.bhcp")?;
    for marker in [
        &syntax.symbol,
        &syntax.mappings[0].canonical,
        &syntax.mappings[0].surface,
    ] {
        if !syntax_source.contains(marker) {
            return Err(format!("syntax source omits projected value {marker}"));
        }
    }

    let profile_value = parse_diagnostic(&read_reference(root, "profile.diag")?)
        .map_err(|error| error.to_string())?;
    validate_root(&profile_value, "profile").map_err(|error| error.to_string())?;
    let PresentationDocument::Profile(profile) =
        PresentationDocument::from_value(&profile_value).map_err(|error| error.to_string())?
    else {
        return Err("profile projection has the wrong root".to_owned());
    };
    if profile.symbol != "bhcp.reference/review-profile@0"
        || profile.syntax != syntax.symbol
        || profile.policy_overlays
            != [
                "bhcp.reference/organization-policy@0",
                "bhcp.reference/repository-policy@0",
            ]
    {
        return Err("profile projection is disconnected from syntax or policy".to_owned());
    }
    let profile_source = read_reference(root, "profile.bhcp")?;
    for marker in std::iter::once(profile.symbol.as_str())
        .chain(std::iter::once(profile.syntax.as_str()))
        .chain(profile.policy_overlays.iter().map(String::as_str))
    {
        if !profile_source.contains(marker) {
            return Err(format!("profile source omits projected value {marker}"));
        }
    }

    let waiver_value = parse_diagnostic(&read_reference(root, "waiver.diag")?)
        .map_err(|error| error.to_string())?;
    validate_root(&waiver_value, "waiver").map_err(|error| error.to_string())?;
    let waiver = WaiverDocument::from_value(&waiver_value).map_err(|error| error.to_string())?;
    if waiver.symbol != "bhcp.reference/offline-emergency-waiver@0"
        || waiver.targets.len() != 1
        || waiver.targets[0].rule.policy != "bhcp.reference/repository-policy@0"
        || waiver.targets[0].rule.rule != "d-attempts"
        || registry["waiver-decision-at"] <= waiver.not_before
        || registry["waiver-decision-at"] >= waiver.expires_at
    {
        return Err("waiver projection is disconnected or inactive".to_owned());
    }
    let WaiverWeakening::LoosenLimit { from, to } = &waiver.targets[0].weakening else {
        return Err("reference waiver does not loosen the attempt limit".to_owned());
    };
    if from.dimension != "bhcp.reference/limit.attempts@0"
        || from.maximum != ExactNumber::Integer(2)
        || to.dimension != from.dimension
        || to.unit != from.unit
        || to.maximum != ExactNumber::Integer(3)
        || !canonical
            .contains("§limit \"attempts\": bhcp.reference/limit.attempts@0: attempts <= 3;")
        || !read_reference(root, "planner-input.txt")?
            .contains("budget = { attempts: 3, wall-time: duration \"PT10M\", processes: 4 }")
    {
        return Err(
            "reference waiver does not materially authorize the frozen attempt budget".to_owned(),
        );
    }
    let waiver_policy = parse_policy_source(&read_reference(root, "policy.bhcp")?, "policy.bhcp")
        .map_err(|error| error.to_string())?;
    let base_policy = compose_policies(&waiver_policy.documents, HashAlgorithm::default())
        .map_err(|error| error.to_string())?;
    let waived_policy = apply_waiver(
        &base_policy,
        &waiver,
        &registry["waiver-decision-at"],
        HashAlgorithm::default(),
    )
    .map_err(|error| error.to_string())?;
    if !waived_policy.effective.limits.iter().any(|rule| {
        rule.value.dimension == "bhcp.reference/limit.attempts@0"
            && rule.value.maximum == ExactNumber::Integer(3)
    }) {
        return Err("active reference waiver did not produce the attempt ceiling".to_owned());
    }
    let waiver_source = read_reference(root, "waiver.bhcp")?;
    for marker in [
        waiver.symbol.as_str(),
        waiver.targets[0].rule.policy.as_str(),
        waiver.targets[0].rule.rule.as_str(),
        waiver.not_before.as_str(),
        waiver.expires_at.as_str(),
    ] {
        if !waiver_source.contains(marker) {
            return Err(format!("waiver source omits projected value {marker}"));
        }
    }

    let extension_value = parse_diagnostic(&read_reference(root, "extension.diag")?)
        .map_err(|error| error.to_string())?;
    validate_root(&extension_value, "extension-descriptor").map_err(|error| error.to_string())?;
    if value_text(&extension_value, "symbol")? != "bhcp.reference/review@0"
        || value_text(&extension_value, "lowering")? != "bhcp.reference/lowerReview@0"
        || !canonical.contains("bhcp.reference/review@0(risk = risk)")
    {
        return Err("derived extension is disconnected from the reference program".to_owned());
    }
    let extension_source = read_reference(root, "extension.bhcp")?;
    for marker in [
        value_text(&extension_value, "symbol")?,
        value_text(&extension_value, "lowering")?,
        "bhcp.reference/reviewReducer@0",
    ] {
        if !extension_source.contains(marker) {
            return Err(format!("extension source omits projected value {marker}"));
        }
    }
    for (field, registry_key, expected_rule) in EXTENSION_RULES {
        let bytes = fs::read(directory.join(&registry[*registry_key]))
            .map_err(|error| format!("cannot read {registry_key}: {error}"))?;
        if bytes != expected_rule.as_bytes() {
            return Err(format!("extension {field} reviewed rule mismatch"));
        }
        let expected =
            ContentReference::from_bytes("text/plain", &bytes, HashAlgorithm::default()).to_value();
        if extension_value.get(field) != Some(&expected) {
            return Err(format!(
                "extension {field} does not bind its reviewed rule bytes"
            ));
        }
    }

    validate_policy_evidence_bindings(
        &canonical,
        &read_reference(root, "policy-evidence-registry.txt")?,
    )?;

    let obligations = read_reference(root, "expected-obligations.txt")?
        .lines()
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    if obligations != expected_set(OBLIGATIONS) {
        return Err("reference obligation inventory mismatch".to_owned());
    }
    for line in &obligations {
        let fields = line.split('|').collect::<Vec<_>>();
        if fields.len() != 5
            || !matches!(fields[4], "open" | "discharged" | "refuted" | "unresolved")
        {
            return Err("invalid obligation state".to_owned());
        }
    }

    let outcomes = read_reference(root, "outcome-matrix.txt")?
        .lines()
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    if outcomes != expected_set(OUTCOMES) {
        return Err("reference outcome matrix mismatch".to_owned());
    }
    let execution_input = read_reference(root, "execution-input.txt")?;
    for marker in [
        "patch = { bytes: h'00', digest: \"reference-patch\" }",
        "repository = { root: \"reference-workspace\" }",
        "risk = High",
        "tree = { name: \"repository\", depth: 0, children: [] }",
        "tree_depth = 0",
        "attempts = 0",
        "expected-output = bhcp.reference/Delivery@0",
    ] {
        if !execution_input.contains(marker) {
            return Err(format!("execution input omits {marker}"));
        }
    }
    let cases = execution_input
        .lines()
        .filter_map(|line| line.strip_prefix("case = "))
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    let expected_cases = OUTCOMES
        .iter()
        .filter_map(|line| line.split('|').next())
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    if cases != expected_cases {
        return Err("execution inputs do not cover the outcome matrix".to_owned());
    }

    let planner = read_reference(root, "planner-input.txt")?;
    for (key, value) in &registry {
        if matches!(
            key.as_str(),
            "canonical-program"
                | "alternate-program"
                | "program-contract"
                | "syntax-document"
                | "profile-document"
                | "policy-source"
                | "waiver-document"
                | "waiver-decision-at"
                | "extension-document"
                | "policy-evidence-registry"
                | "execution-input"
                | "expected-obligations"
                | "outcome-matrix"
        ) && !planner.contains(value)
        {
            return Err(format!("planner input omits registry link {key}"));
        }
    }

    validate_program_contract(root)
}

fn validate_contract(root: &Path, text: &str) -> Result<(), String> {
    let contract = parse_contract(text)?;
    if contract.version.as_deref() != Some("bhcp-v0-completion@0") {
        return Err("missing or unknown completion contract version".to_owned());
    }

    let expected_issues = ISSUE_KEYS
        .iter()
        .map(|(number, key)| (*number, (*key).to_owned()))
        .collect::<BTreeMap<_, _>>();
    if contract.issues != expected_issues {
        return Err("issue inventory mismatch".to_owned());
    }
    let issue_keys = contract.issues.values().cloned().collect::<BTreeSet<_>>();

    let readme = fs::read_to_string(root.join("conformance/v0/README.md"))
        .map_err(|error| format!("cannot read conformance README: {error}"))?;
    let expected_scenarios = markdown_scenarios(&readme);
    if contract.scenarios.keys().cloned().collect::<BTreeSet<_>>() != expected_scenarios {
        return Err("scenario inventory mismatch".to_owned());
    }

    let example_manifest = fs::read_to_string(root.join("schemas/v0/examples/manifest.txt"))
        .map_err(|error| format!("cannot read schema fixture manifest: {error}"))?;
    let expected_roots = schema_roots(&example_manifest);
    if expected_roots.len() != 17 {
        return Err("wire root inventory no longer contains 17 kinds".to_owned());
    }
    if contract.roots.keys().cloned().collect::<BTreeSet<_>>() != expected_roots {
        return Err("wire root inventory mismatch".to_owned());
    }
    let schema = fs::read_to_string(root.join("schemas/v0/bhcp-v0.cddl"))
        .map_err(|error| format!("cannot read v0 schema: {error}"))?;
    let root_refs = expected_roots
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    validate_schema_inventory(&schema, &root_refs).map_err(|error| error.to_string())?;

    if contract.stages.keys().cloned().collect::<BTreeSet<_>>() != expected_set(STAGES) {
        return Err("pipeline stage inventory mismatch".to_owned());
    }
    if contract.artifacts.keys().cloned().collect::<BTreeSet<_>>() != expected_set(ARTIFACTS) {
        return Err("reference artifact inventory mismatch".to_owned());
    }
    if contract.features.keys().cloned().collect::<BTreeSet<_>>() != expected_set(FEATURES) {
        return Err("reference feature inventory mismatch".to_owned());
    }
    if contract.non_goals != expected_set(NON_GOALS) {
        return Err("v0 non-goal inventory mismatch".to_owned());
    }

    for owner in contract
        .scenarios
        .values()
        .chain(contract.roots.values())
        .chain(
            contract
                .stages
                .values()
                .flat_map(|stage| stage.owners.iter()),
        )
        .chain(
            contract
                .features
                .values()
                .flat_map(|feature| feature.owners.iter()),
        )
    {
        if !issue_keys.contains(owner) {
            return Err(format!("unknown issue key {owner}"));
        }
    }
    if contract
        .stages
        .values()
        .any(|stage| stage.outcome.is_empty())
    {
        return Err("pipeline stage has no observable outcome".to_owned());
    }

    let canonical_root = fs::canonicalize(root)
        .map_err(|error| format!("cannot resolve repository root: {error}"))?;
    for (id, relative) in &contract.artifacts {
        if relative.is_absolute()
            || relative
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err(format!("reference artifact {id} has an unsafe path"));
        }
        let path = root.join(relative);
        if !path.is_file() {
            return Err(format!(
                "reference artifact {id} does not exist: {relative:?}"
            ));
        }
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| format!("cannot inspect reference artifact {id}: {error}"))?;
        let resolved = fs::canonicalize(&path)
            .map_err(|error| format!("cannot resolve reference artifact {id}: {error}"))?;
        if metadata.file_type().is_symlink() || !resolved.starts_with(&canonical_root) {
            return Err(format!("reference artifact {id} escapes the repository"));
        }
    }
    for (id, feature) in &contract.features {
        let relative = contract
            .artifacts
            .get(&feature.artifact)
            .ok_or_else(|| format!("feature {id} names unknown artifact {}", feature.artifact))?;
        let body = fs::read_to_string(root.join(relative))
            .map_err(|error| format!("cannot read feature artifact {id}: {error}"))?;
        for needle in &feature.needles {
            if needle.is_empty() || !body.contains(needle) {
                return Err(format!("feature {id} is missing marker {needle:?}"));
            }
        }
    }

    validate_reference_semantics(root)?;

    Ok(())
}

fn manifest_text() -> String {
    fs::read_to_string(repository().join("conformance/v0/completion-manifest.txt"))
        .expect("the v0 completion manifest must be checked in")
}

#[test]
fn completion_manifest_closes_the_normative_inventory() {
    let text = manifest_text();
    validate_contract(&repository(), &text).unwrap();
}

#[test]
fn completion_manifest_rejects_omitted_duplicate_and_unknown_records() {
    let text = manifest_text();
    let omitted = text.replacen("scenario|SYN-01|profile-source-lowering\n", "", 1);
    assert_eq!(
        validate_contract(&repository(), &omitted).unwrap_err(),
        "scenario inventory mismatch"
    );

    let duplicate = format!("{text}scenario|SYN-01|profile-source-lowering\n");
    assert_eq!(
        validate_contract(&repository(), &duplicate).unwrap_err(),
        "duplicate scenario"
    );

    let unknown = text.replacen(
        "scenario|SYN-01|profile-source-lowering",
        "scenario|SYN-99|profile-source-lowering",
        1,
    );
    assert_eq!(
        validate_contract(&repository(), &unknown).unwrap_err(),
        "scenario inventory mismatch"
    );

    let unknown_owner = text.replacen(
        "scenario|SYN-01|profile-source-lowering",
        "scenario|SYN-01|not-a-roadmap-issue",
        1,
    );
    assert_eq!(
        validate_contract(&repository(), &unknown_owner).unwrap_err(),
        "unknown issue key not-a-roadmap-issue"
    );
}

#[test]
fn reference_program_fails_closed_until_the_complete_front_end_exists() {
    let source =
        fs::read_to_string(repository().join("conformance/v0/reference-program/program.bhcp"))
            .unwrap();
    let diagnostic = compile_source(&source, "reference-program/program.bhcp").unwrap_err();
    assert_eq!(diagnostic.code, "BHCP1004");
    assert!(
        diagnostic
            .message
            .contains("outside the implemented vertical slice"),
        "unexpected boundary: {diagnostic}"
    );
}

#[test]
fn reference_validators_reject_invalid_policy_shapes_and_ownership() {
    let root = repository();
    let policy = read_reference(&root, "policy.bhcp").unwrap().replacen(
        "dimension: bhcp.reference/limit.attempts@0",
        "dimension: attempts",
        1,
    );
    let diagnostic = parse_policy_source(&policy, "invalid-policy.bhcp").unwrap_err();
    assert_eq!(diagnostic.code, "BHCP8001");
    assert!(diagnostic.message.contains("dimension must be a symbol-id"));

    let projection = read_reference(&root, "program-contract.txt")
        .unwrap()
        .replacen("|patch|move|input.patch", "|patch|copy|input.patch", 1);
    assert!(
        validate_program_projection(&root, &projection)
            .unwrap_err()
            .contains("must move owned values")
    );

    let policy_scope = read_reference(&root, "policy.bhcp").unwrap().replacen(
        "[bhcp.reference/DeliverChange@0, bhcp.reference/Persist@0]",
        "[bhcp.reference/DeliverChange@0]",
        1,
    );
    let scope_error = validate_reference_policy(&policy_scope).unwrap_err();
    assert!(
        scope_error.contains("BHCP8101")
            || scope_error
                == "reference policy does not authorize the declared effects and base limit"
    );

    let typed_projection = read_reference(&root, "program-contract.txt").unwrap();
    for (from, to) in [
        ("|attempts|3|Bool", "|attempts|3|Integer"),
        ("|remaining|remaining - 1|", "|remaining|remaining + 1|"),
        ("|bhcp-effect/fs.read@0|", "|fs.read|"),
        ("|{}|Reduction<Unit>|", "|{}|Unit|"),
        ("|input-free|-", "|parent-input|input.patch"),
    ] {
        let mutation = typed_projection.replacen(from, to, 1);
        assert_eq!(
            validate_program_projection(&root, &mutation).unwrap_err(),
            "program semantic projection mismatch"
        );
    }

    let canonical = read_reference(&root, "program.bhcp").unwrap();
    let owner_type_drift = canonical.replacen(
        "§input risk: bhcp.reference/Risk@0;\n    §input tree: bhcp.reference/Node@0;",
        "§input risk: Text;\n    §input tree: bhcp.reference/Node@0;",
        1,
    );
    let owner_error = validate_program_projection_sources(
        &typed_projection,
        &owner_type_drift,
        &read_reference(&root, "extension.bhcp").unwrap(),
    )
    .unwrap_err();
    assert!(owner_error.contains("owner-scoped typed fact"));

    let undefined_repository = canonical.replacen(
        "§type bhcp.reference/Repository@0 = { root: Text };\n",
        "",
        1,
    );
    assert!(
        validate_program_projection_sources(
            &typed_projection,
            &undefined_repository,
            &read_reference(&root, "extension.bhcp").unwrap(),
        )
        .unwrap_err()
        .contains("occurs 0 times")
    );

    let bindings = read_reference(&root, "policy-evidence-registry.txt")
        .unwrap()
        .replacen(
            "bhcp.verifier/static-analysis@0",
            "bhcp.verifier/missing@0",
            1,
        );
    assert_eq!(
        validate_policy_evidence_bindings(
            &read_reference(&root, "program.bhcp").unwrap(),
            &bindings,
        )
        .unwrap_err(),
        "policy evidence producer registry mismatch"
    );
}
