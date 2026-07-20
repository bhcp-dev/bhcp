use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: impl AsRef<Path>) -> String {
    fs::read_to_string(path.as_ref())
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.as_ref().display()))
}

fn assert_blob(repository: &Path, specification: &str) {
    if specification == "none" {
        return;
    }

    for item in specification.split(',') {
        let (path, expected) = item
            .rsplit_once('@')
            .unwrap_or_else(|| panic!("artifact pin lacks @<git-blob>: {item}"));
        assert_eq!(expected.len(), 40, "artifact pin is not a Git blob: {item}");
        let absolute = repository.join(path);
        assert!(absolute.is_file(), "missing pinned artifact: {path}");
        let output = Command::new("git")
            .args(["hash-object", path])
            .current_dir(repository)
            .output()
            .unwrap_or_else(|error| panic!("cannot hash {path}: {error}"));
        assert!(output.status.success(), "cannot hash pinned artifact: {path}");
        assert_eq!(
            String::from_utf8(output.stdout).unwrap().trim(),
            expected,
            "artifact drifted: {path}",
        );
    }
}

#[test]
fn every_phase_two_experiment_has_exact_identity_and_replay_evidence() {
    let repository = root();
    let manifest = read(repository.join("experiments/phase-2-evidence-audit.txt"));
    let report = read(repository.join("experiments/phase-2-evidence-audit.md"));
    let mut identifiers = BTreeSet::new();

    for line in manifest.lines().filter(|line| !line.is_empty()) {
        let fields = line.split('|').collect::<Vec<_>>();
        assert_eq!(fields.len(), 17, "invalid audit record: {line}");
        let [
            identifier,
            admissibility,
            result,
            source,
            task,
            contract,
            semantic_id_path,
            semantic_id,
            skill,
            model,
            oracle,
            result_path,
            evidence_path,
            evidence_function,
            issue,
            pull_request,
            merge,
        ] = fields.as_slice()
        else {
            unreachable!()
        };

        assert!(identifiers.insert(*identifier), "duplicate experiment: {line}");
        for pin in [source, task, contract, skill, oracle] {
            assert_blob(&repository, pin);
        }

        let semantic_pin = read(repository.join(semantic_id_path)).trim().to_owned();
        assert_eq!(&semantic_pin, semantic_id, "semantic identity drifted: {line}");
        assert!(semantic_id.starts_with("bhcp.hash/sha3-512@0:"), "{line}");
        assert!(repository.join(result_path).is_file(), "missing result: {line}");

        let evidence = read(repository.join(evidence_path));
        assert!(
            evidence.contains(&format!("fn {evidence_function}()")),
            "missing executable replay evidence: {line}",
        );
        assert!(issue.starts_with('#'), "missing issue pin: {line}");
        assert!(pull_request.starts_with('#'), "missing PR pin: {line}");
        assert_eq!(merge.len(), 40, "missing squash-merge pin: {line}");

        assert!(
            report.lines().any(|report_line| {
                report_line.starts_with(&format!("| `{identifier}` |"))
                    && report_line.contains(&format!("| {admissibility} | {result} |"))
            }),
            "human report omits or reclassifies: {line}",
        );

        if *admissibility == "historical-unreproducible" {
            assert!(model.contains("model=unrecorded-default"), "{line}");
        } else {
            assert!(model.contains("model=gpt-5.4-mini"), "{line}");
        }
        assert!(model.contains("codex-cli=0.142.4"), "{line}");
        assert!(model.contains("reasoning=medium"), "{line}");
    }

    assert_eq!(
        identifiers,
        BTreeSet::from([
            "contextual-policy-multiseed-001",
            "contextual-policy-multiseed-002",
            "contextual-policy-multiseed-003",
            "contextual-policy-multiseed-004",
            "in-session-evidence-forward-001",
            "pilot-001",
            "pilot-002",
            "pilot-003",
            "pilot-004",
            "pilot-005",
            "pilot-006",
        ]),
    );
}

#[test]
fn report_links_and_public_maturity_claims_agree() {
    let repository = root();
    let report_path = repository.join("experiments/phase-2-evidence-audit.md");
    let report = read(&report_path);
    let readme = read(repository.join("README.md"));
    let vision = read(repository.join("VISION.md"));
    let agents = read(repository.join("AGENTS.md"));
    let profile = read(repository.join(".codex/project-profile.md"));
    let mut local_links = BTreeSet::new();
    let mut remaining = report.as_str();

    while let Some(start) = remaining.find("](") {
        remaining = &remaining[start + 2..];
        let end = remaining.find(')').expect("unterminated Markdown link");
        let target = &remaining[..end];
        remaining = &remaining[end + 1..];
        if target.starts_with("https://") || target.starts_with('#') {
            continue;
        }
        let path = target.split('#').next().unwrap();
        assert!(!path.is_empty(), "empty local report link");
        assert!(
            report_path.parent().unwrap().join(path).exists(),
            "broken local report link: {target}",
        );
        local_links.insert(target.to_owned());
    }

    assert!(local_links.len() >= 12, "audit must link broad local evidence");
    for document in [&report, &readme, &vision] {
        assert!(document.contains("BHCP v0 is not complete"));
        assert!(document.contains("no BHCP-versus-prose advantage"));
        assert!(document.contains("positive in-session acceptance remains unproven"));
    }
    assert!(report.contains("Pilot 001 cannot be reproduced at the model layer"));
    assert!(report.contains("0/5 accepted"));
    assert!(report.contains("0/1 accepted"));
    assert!(agents.contains("not yet a complete parser, checker, planner"));
    assert!(profile.contains("all milestone acceptance outcomes are demonstrable"));
}
