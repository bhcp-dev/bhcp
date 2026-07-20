use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repository() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: impl AsRef<Path>) -> String {
    fs::read_to_string(path.as_ref())
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.as_ref().display()))
}

#[test]
fn comparative_study_is_frozen_before_any_model_turn() {
    let root = repository();
    for relative in [
        "experiments/evidence-generalization/comparative-registration.md",
        "experiments/evidence-generalization/comparative-registration.txt",
        "experiments/evidence-generalization/COMPARATIVE_BHCP_PROMPT.md",
        "experiments/evidence-generalization/COMPARATIVE_PROSE_PROMPT.md",
        "src/bin/evidence_generalization_comparative_policy.rs",
        "src/bin/evidence_generalization_comparative.rs",
    ] {
        assert!(
            root.join(relative).is_file(),
            "missing frozen comparative input: {relative}"
        );
    }

    let registration =
        read(root.join("experiments/evidence-generalization/comparative-registration.txt"));
    for exact in [
        "version|bhcp-evidence-generalization-comparative-registration@0",
        "authority|issue=93|owner-attestation-merge=c327d80308dbf6010321ca05ef498493b04350e7|incremental-usd=0|existing-entitlement-only=true",
        "design|tasks=3|seeds=4|paired-blocks=12|sessions=24|concurrency=1|no-replacement",
        "pins|codex-cli=0.142.4|model=gpt-5.4-mini|reasoning=medium|rust=1.97.1|sandbox=workspace-write/no-network/read-confined",
        "analysis|acceptance=paired-risk-difference+exact-mcnemar|calibration=paired-risk-difference+exact-mcnemar|resources=median+iqr|alpha=descriptive-only",
        "exclusion|pair-if-either-arm|retain-completed|stop-later-launches|no-replacement",
        "usage-monitor|prior-input=1961148|prior-output=40755|prior-reasoning=31916|aggregate-input-stop-after=12000000|aggregate-output-stop-after=500000|aggregate-reasoning-stop-after=500000",
    ] {
        assert!(registration.lines().any(|line| line == exact), "{exact}");
    }
    assert_eq!(
        registration
            .lines()
            .filter(|line| line.starts_with("artifact|"))
            .count(),
        8
    );
    for line in registration
        .lines()
        .filter(|line| line.starts_with("artifact|"))
    {
        let fields = line.split('|').collect::<Vec<_>>();
        let [_, path, digest] = fields.as_slice() else {
            panic!("invalid artifact record: {line}");
        };
        let expected = digest.strip_prefix("gitblob=").expect("missing gitblob=");
        let output = Command::new("git")
            .current_dir(&root)
            .args(["hash-object", path])
            .output()
            .expect("cannot hash comparative artifact");
        assert!(output.status.success(), "cannot hash {path}");
        assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), expected);
    }

    let mut blocks = BTreeMap::<(String, String), Vec<(usize, String)>>::new();
    let mut arms = BTreeMap::<String, usize>::new();
    let mut first = BTreeMap::<String, usize>::new();
    let mut registered_schedule = Vec::new();
    for line in registration
        .lines()
        .filter(|line| line.starts_with("session|"))
    {
        let fields = line.split('|').collect::<Vec<_>>();
        assert_eq!(fields.len(), 8, "invalid session record: {line}");
        let task = fields[1].to_owned();
        let seed = fields[2].to_owned();
        let position = fields[3]
            .strip_prefix("position=")
            .unwrap()
            .parse::<usize>()
            .unwrap();
        let arm = fields[4].strip_prefix("arm=").unwrap().to_owned();
        assert!(fields[5].starts_with("bhcp.hash/sha3-512@0:"));
        assert!(fields[6].starts_with("bhcp.hash/sha3-512@0:"));
        assert_eq!(fields[7], arm);
        assert!(matches!(arm.as_str(), "prose-control" | "bhcp-contract"));
        *arms.entry(arm.clone()).or_default() += 1;
        if position == 1 {
            *first.entry(arm.clone()).or_default() += 1;
        }
        blocks
            .entry((task, seed))
            .or_default()
            .push((position, arm));
        registered_schedule.push(format!(
            "{}|{}|{}|{}",
            fields[1], fields[2], position, fields[7]
        ));
    }
    let parent_schedule =
        read(root.join("experiments/evidence-generalization/preregistration.txt"))
            .lines()
            .filter_map(|line| line.strip_prefix("session|comparative|"))
            .map(str::to_owned)
            .collect::<Vec<_>>();
    assert_eq!(registered_schedule, parent_schedule);
    assert_eq!(blocks.len(), 12);
    assert_eq!(
        arms,
        BTreeMap::from([
            ("bhcp-contract".to_owned(), 12),
            ("prose-control".to_owned(), 12)
        ])
    );
    assert_eq!(
        first,
        BTreeMap::from([
            ("bhcp-contract".to_owned(), 6),
            ("prose-control".to_owned(), 6)
        ])
    );
    for block in blocks.values_mut() {
        block.sort();
        assert_eq!(block.len(), 2);
        assert_eq!(
            block.iter().map(|entry| entry.0).collect::<Vec<_>>(),
            [1, 2]
        );
        assert_eq!(
            block
                .iter()
                .map(|entry| entry.1.as_str())
                .collect::<BTreeSet<_>>(),
            BTreeSet::from(["bhcp-contract", "prose-control"])
        );
    }

    let bhcp_prompt =
        read(root.join("experiments/evidence-generalization/COMPARATIVE_BHCP_PROMPT.md"));
    let prose_prompt =
        read(root.join("experiments/evidence-generalization/COMPARATIVE_PROSE_PROMPT.md"));
    assert!(bhcp_prompt.contains("Do not use a BHCP skill, project registry"));
    assert!(prose_prompt.contains("Do not use a BHCP contract, BHCP skill, project registry"));
    assert!(!prose_prompt.contains("contract.bhcp"));
}
