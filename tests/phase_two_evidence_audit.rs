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
        assert!(
            output.status.success(),
            "cannot hash pinned artifact: {path}"
        );
        assert_eq!(
            String::from_utf8(output.stdout).unwrap().trim(),
            expected,
            "artifact drifted: {path}",
        );
    }
}

fn single_pinned_path(specification: &str) -> &str {
    assert!(
        !specification.contains(','),
        "expected one pinned artifact: {specification}"
    );
    specification
        .rsplit_once('@')
        .unwrap_or_else(|| panic!("artifact pin lacks @<git-blob>: {specification}"))
        .0
}

fn expected_delivery(identifier: &str) -> (&str, &str, &str) {
    match identifier {
        "pilot-001" | "pilot-002" | "pilot-003" => {
            ("none", "#6", "93e3cc6b892bd4373fc112e74cd52de75fe82594")
        }
        "pilot-004" => ("none", "#8", "98092552efda108cd3ce02e3787ad38239e09066"),
        "pilot-005" => ("none", "#9", "64b5d164e4083041da0bbb09f10d5840a04f35d8"),
        "pilot-006" => ("#21", "#86", "b227ce10e6e7e20c610c4d061a8cdb4fd15fd10c"),
        "contextual-policy-multiseed-001"
        | "contextual-policy-multiseed-002"
        | "contextual-policy-multiseed-003"
        | "contextual-policy-multiseed-004" => {
            ("#26", "#88", "44bf1a1cf61f1829f3fbf839aea4067e06cb4a6c")
        }
        "in-session-evidence-forward-001" => {
            ("#27", "#89", "ee7ee62649daa31b9216379b562e7f43442231da")
        }
        _ => panic!("unknown delivery identity: {identifier}"),
    }
}

#[test]
fn every_phase_two_experiment_has_exact_identity_and_executable_evidence() {
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
            result_specification,
            evidence_specification,
            evidence_function,
            issue,
            pull_request,
            merge,
        ] = fields.as_slice()
        else {
            unreachable!()
        };

        assert!(
            identifiers.insert(*identifier),
            "duplicate experiment: {line}"
        );
        for pin in [
            source,
            task,
            contract,
            skill,
            oracle,
            result_specification,
            evidence_specification,
        ] {
            assert_blob(&repository, pin);
        }

        let semantic_pin = read(repository.join(semantic_id_path)).trim().to_owned();
        assert_eq!(
            &semantic_pin, semantic_id,
            "semantic identity drifted: {line}"
        );
        assert!(semantic_id.starts_with("bhcp.hash/sha3-512@0:"), "{line}");
        let result_path = single_pinned_path(result_specification);
        let evidence_path = single_pinned_path(evidence_specification);
        assert!(
            !read(repository.join(result_path)).trim().is_empty(),
            "{line}"
        );

        let evidence = read(repository.join(evidence_path));
        assert!(
            evidence.contains(&format!("#[test]\nfn {evidence_function}() {{")),
            "missing executable evidence: {line}",
        );
        let expected = expected_delivery(identifier);
        assert_eq!((*issue, *pull_request, *merge), expected, "{line}");

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

    assert!(
        local_links.len() >= 12,
        "audit must link broad local evidence"
    );
    for document in [&report, &readme, &vision] {
        let document = document.to_lowercase();
        assert!(document.contains("bhcp v0 is not complete"));
        assert!(document.contains("no bhcp-versus-prose advantage"));
        assert!(document.contains("positive in-session acceptance remains unproven"));
    }
    assert!(report.contains("Pilot 001 cannot be reproduced at the model layer"));
    assert!(report.contains("0/5 accepted"));
    assert!(report.contains("0/1 accepted"));
    for provenance in [
        "[#6](https://github.com/bhcp-dev/bhcp/pull/6) | `93e3cc6b892bd4373fc112e74cd52de75fe82594`",
        "[#8](https://github.com/bhcp-dev/bhcp/pull/8) | `98092552efda108cd3ce02e3787ad38239e09066`",
        "[#9](https://github.com/bhcp-dev/bhcp/pull/9) | `64b5d164e4083041da0bbb09f10d5840a04f35d8`",
        "[#86](https://github.com/bhcp-dev/bhcp/pull/86) | `b227ce10e6e7e20c610c4d061a8cdb4fd15fd10c`",
        "[#87](https://github.com/bhcp-dev/bhcp/pull/87) | `56ae59f8fa8a8584891958f101d6a902767352fa`",
        "[#88](https://github.com/bhcp-dev/bhcp/pull/88) | `44bf1a1cf61f1829f3fbf839aea4067e06cb4a6c`",
        "[#89](https://github.com/bhcp-dev/bhcp/pull/89) | `ee7ee62649daa31b9216379b562e7f43442231da`",
    ] {
        assert!(
            report.contains(provenance),
            "missing delivery provenance: {provenance}"
        );
    }
    for follow_up in ["issues/91", "issues/92", "issues/93"] {
        assert!(
            report.contains(follow_up),
            "missing residual follow-up: {follow_up}"
        );
    }
    assert!(agents.contains("not yet a complete parser, checker, planner"));
    assert!(profile.contains("all milestone acceptance outcomes are demonstrable"));
}
