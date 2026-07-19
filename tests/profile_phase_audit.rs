use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: impl AsRef<Path>) -> String {
    fs::read_to_string(path.as_ref())
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.as_ref().display()))
}

#[test]
fn every_phase_four_acceptance_claim_names_executable_evidence() {
    let repository = root();
    let manifest = read(repository.join("conformance/v0/profile-phase-audit.txt"));
    let report = read(repository.join("conformance/v0/profile-phase-audit.md"));
    let mut claims_by_issue: BTreeMap<u8, BTreeSet<String>> = BTreeMap::new();

    for line in manifest.lines().filter(|line| !line.is_empty()) {
        let fields = line.split('|').collect::<Vec<_>>();
        assert_eq!(fields.len(), 4, "{line}");
        let issue = fields[0].parse::<u8>().unwrap();
        assert!((41..=49).contains(&issue), "{line}");
        assert!(
            claims_by_issue
                .entry(issue)
                .or_default()
                .insert(fields[1].to_owned()),
            "duplicate acceptance claim: {line}",
        );
        let test_path = repository.join(fields[2]);
        assert!(test_path.is_file(), "missing evidence path: {line}");
        let test_source = read(&test_path);
        assert!(
            test_source.contains(&format!("fn {}()", fields[3])),
            "missing evidence function: {line}",
        );
        assert!(
            report.contains(&format!("`{}`", fields[3])),
            "report omits evidence function: {line}",
        );
    }

    assert_eq!(
        claims_by_issue.keys().copied().collect::<Vec<_>>(),
        (41..=49).collect::<Vec<_>>()
    );
    assert!(
        claims_by_issue.iter().all(|(_, claims)| claims.len() == 3),
        "every Phase 4 issue must expose exactly three acceptance claims",
    );
}

#[test]
fn phase_four_report_local_links_resolve() {
    let repository = root();
    let report_path = repository.join("conformance/v0/profile-phase-audit.md");
    let report = read(&report_path);
    let mut links = BTreeSet::new();
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
        assert!(!path.is_empty(), "empty local link target");
        let resolved = report_path.parent().unwrap().join(path);
        assert!(resolved.exists(), "broken local link: {target}");
        links.insert(target.to_owned());
    }

    assert!(links.len() >= 12, "audit must link broad local evidence");
}

#[test]
fn maturity_and_closed_profile_non_goals_remain_consistent() {
    let repository = root();
    let readme = read(repository.join("README.md"));
    let vision = read(repository.join("VISION.md"));
    let semantics = read(repository.join("SEMANTICS.md"));
    let conformance = read(repository.join("conformance/v0/README.md"));
    let agents = read(repository.join("AGENTS.md"));
    let profile = read(repository.join(".codex/project-profile.md"));
    let report = read(repository.join("conformance/v0/profile-phase-audit.md"));

    assert!(readme.contains("not yet a complete v0"));
    assert!(vision.contains("current Rust slice"));
    assert!(agents.contains("not yet a complete parser, checker, planner"));
    assert!(profile.contains("Roadmap completion means"));
    assert!(profile.contains("all milestone acceptance outcomes are demonstrable"));
    assert!(conformance.contains("does not yet claim general obligation-graph construction"));

    for document in [&readme, &vision, &semantics, &conformance, &report] {
        assert!(
            document.contains("unrestricted macros"),
            "profile non-goal drifted from a public maturity document",
        );
    }
    assert!(semantics.contains("Arbitrary grammars, executable macros, parser plugins"));
    assert!(report.contains("Phase 4 presentation layer is complete"));
    assert!(report.contains("BHCP v0 is not complete"));
}
