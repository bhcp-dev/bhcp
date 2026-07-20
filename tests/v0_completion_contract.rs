use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use bhcp::pipeline::compile_source;
use bhcp::schema::validate_schema_inventory;

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
    "policy",
    "waiver",
    "syntax",
    "profile",
    "extension",
    "planner-input",
    "execution-input",
    "expected-obligations",
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

fn insert_unique<K: Ord + std::fmt::Display, V>(
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

    for (id, relative) in &contract.artifacts {
        let path = root.join(relative);
        if !path.is_file() {
            return Err(format!(
                "reference artifact {id} does not exist: {relative:?}"
            ));
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
