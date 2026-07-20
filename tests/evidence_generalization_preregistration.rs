use std::collections::{BTreeMap, BTreeSet};
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

fn assert_blob<'a>(repository: &Path, specification: &'a str) -> &'a str {
    let (path, expected) = specification
        .rsplit_once('@')
        .unwrap_or_else(|| panic!("artifact pin lacks @<git-blob>: {specification}"));
    assert_eq!(expected.len(), 40, "artifact pin is not a Git blob");
    assert!(repository.join(path).is_file(), "missing artifact: {path}");
    let output = Command::new("git")
        .args(["hash-object", path])
        .current_dir(repository)
        .output()
        .unwrap_or_else(|error| panic!("cannot hash {path}: {error}"));
    assert!(output.status.success(), "cannot hash artifact: {path}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        expected,
        "artifact drifted: {path}",
    );
    path
}

#[test]
fn protocol_freezes_population_arms_schedule_analysis_and_resource_authority() {
    let repository = root();
    let manifest = read(repository.join("experiments/evidence-generalization/preregistration.txt"));
    let report = read(repository.join("experiments/evidence-generalization/preregistration.md"));

    for exact in [
        "version|bhcp-evidence-generalization@0",
        "base|d5ef5ac29a12dabe2fe2af3f0ec35437204d29c8",
        "model|codex-cli=0.142.4|model=gpt-5.4-mini|reasoning=medium|rust=1.97.1|sandbox=workspace-write/no-network/read-confined",
        "authorization|decision=approved-on-merge|incremental-usd=0|existing-entitlement-only=true|overage=stop-before-launch",
        "resource|sessions=36|model-minutes=540|input-tokens=12000000|output-tokens=500000|reasoning-tokens=500000|concurrency=2",
        "stopping|no-efficacy-stop|no-futility-stop|no-replacement|stop-on-safety-or-identity-failure|retain-completed-records",
        "analysis|positive-use=clopper-pearson-95|comparative=paired-risk-difference+exact-mcnemar|resources=median+iqr|alpha=descriptive-only",
        "inference|repository-fixture-frame-only|single-model|no-population-causal-model-wide-or-general-language-claim",
    ] {
        assert!(
            manifest.lines().any(|line| line == exact),
            "missing frozen protocol record: {exact}"
        );
    }

    let expected_tasks = BTreeMap::from([
        (
            "atomic-batch",
            (
                "state-transaction",
                BTreeSet::from([
                    "atomic-rollback",
                    "exact-idempotent-replay",
                    "conflicting-replay",
                    "failed-id-retry",
                    "conservation-and-checked-receipt",
                ]),
            ),
        ),
        (
            "tenant-policy",
            (
                "authorization-specificity",
                BTreeSet::from([
                    "tenant-isolation",
                    "default-deny",
                    "specificity-before-priority",
                    "priority-before-deny",
                    "deny-before-rule-id",
                    "stable-rule-id",
                    "insertion-order-independence",
                ]),
            ),
        ),
        (
            "contextual-policy",
            (
                "ordered-context",
                BTreeSet::from([
                    "tenant-isolation",
                    "default-deny",
                    "resource-before-subject",
                    "subject-before-action",
                    "action-before-priority",
                    "priority-before-deny",
                    "deny-before-rule-id",
                    "stable-rule-id",
                    "insertion-order-independence",
                    "disabled-rule-exclusion",
                ]),
            ),
        ),
        (
            "in-session-evidence",
            (
                "evidence-gated-repair",
                BTreeSet::from([
                    "public-readiness",
                    "oracle-readiness",
                    "policy-readiness",
                    "one-file-change-policy",
                    "accepted-evidence-before-success",
                ]),
            ),
        ),
    ]);

    let mut tasks = BTreeSet::new();
    for line in manifest.lines().filter(|line| line.starts_with("task|")) {
        let fields = line.split('|').collect::<Vec<_>>();
        assert_eq!(fields.len(), 11, "invalid task record: {line}");
        let [
            _,
            identifier,
            class,
            source,
            shared_task,
            prose,
            contract,
            semantic_path,
            semantic_id,
            oracle,
            obligations,
        ] = fields.as_slice()
        else {
            unreachable!()
        };
        let (expected_class, expected_obligations) = expected_tasks
            .get(identifier)
            .unwrap_or_else(|| panic!("unexpected task: {identifier}"));
        assert_eq!(class, expected_class, "wrong task class: {line}");
        assert!(tasks.insert(*identifier), "duplicate task: {identifier}");
        for pin in [source, shared_task, prose, contract, oracle] {
            assert_blob(&repository, pin);
        }
        let prose_path = assert_blob(&repository, prose);
        let prose_text = read(repository.join(prose_path));
        let actual_obligations = obligations.split(',').collect::<BTreeSet<_>>();
        assert_eq!(&actual_obligations, expected_obligations, "{line}");
        for obligation in actual_obligations {
            assert!(
                prose_text.contains(&format!("`{obligation}`")),
                "prose treatment omits {obligation}: {prose_path}"
            );
        }
        assert_eq!(
            read(repository.join(semantic_path)).trim(),
            *semantic_id,
            "semantic identity drifted: {line}"
        );
        assert!(semantic_id.starts_with("bhcp.hash/sha3-512@0:"), "{line}");
    }
    assert_eq!(tasks, expected_tasks.keys().copied().collect());

    let expected_arms = BTreeSet::from([
        "arm|positive-use|bhcp-registered|shared-task+contract+skill|registry-required",
        "arm|comparative|prose-control|prose-treatment|registry-forbidden",
        "arm|comparative|bhcp-contract|shared-task+contract|registry-forbidden",
    ]);
    assert_eq!(
        manifest
            .lines()
            .filter(|line| line.starts_with("arm|"))
            .collect::<BTreeSet<_>>(),
        expected_arms,
    );

    let seeds = BTreeSet::from(["seed-01", "seed-02", "seed-03"]);
    let task_order = [
        "atomic-batch",
        "tenant-policy",
        "contextual-policy",
        "in-session-evidence",
    ];
    let mut sessions = BTreeSet::new();
    let mut comparative_first = BTreeMap::<&str, usize>::new();
    for line in manifest.lines().filter(|line| line.starts_with("session|")) {
        let fields = line.split('|').collect::<Vec<_>>();
        assert_eq!(fields.len(), 6, "invalid session record: {line}");
        let [_, study, task, seed, position, arm] = fields.as_slice() else {
            unreachable!()
        };
        assert!(tasks.contains(task), "unknown task: {line}");
        assert!(seeds.contains(seed), "unknown seed: {line}");
        assert!(sessions.insert((*study, *task, *seed, *arm)), "{line}");
        match (*study, *arm, *position) {
            ("positive-use", "bhcp-registered", "1") => {}
            ("comparative", "prose-control" | "bhcp-contract", "1" | "2") => {
                if *position == "1" {
                    *comparative_first.entry(arm).or_default() += 1;
                }
            }
            _ => panic!("invalid session assignment: {line}"),
        }
    }
    assert_eq!(sessions.len(), 36);
    for task in task_order {
        for seed in &seeds {
            assert!(sessions.contains(&("positive-use", task, *seed, "bhcp-registered")));
            assert!(sessions.contains(&("comparative", task, *seed, "prose-control")));
            assert!(sessions.contains(&("comparative", task, *seed, "bhcp-contract")));
        }
    }
    assert_eq!(comparative_first.get("prose-control"), Some(&6));
    assert_eq!(comparative_first.get("bhcp-contract"), Some(&6));

    for required in [
        "[#92](https://github.com/bhcp-dev/bhcp/issues/92)",
        "[#93](https://github.com/bhcp-dev/bhcp/issues/93)",
        "No model turn occurred before this registration",
        "does not authorize pay-as-you-go spend",
        "twelve paired comparative blocks",
        "twelve registered-evidence sessions",
    ] {
        assert!(report.contains(required), "report omits: {required}");
    }
}
