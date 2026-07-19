use std::fs;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: &str) -> String {
    fs::read_to_string(root().join(path)).unwrap_or_else(|error| panic!("{path}: {error}"))
}

#[test]
fn agents_contract_names_authority_workflow_and_safety_invariants() {
    let contract = read("AGENTS.md");
    for required in [
        "## Purpose and authority",
        "## Commands and meaningful TDD",
        "## Issue-to-merge workflow",
        "## Atomic claims and concurrency",
        "## Consistency and safety",
        "cargo fmt --check",
        "cargo clippy --all-targets -- -D warnings",
        "cargo test --all-targets",
        "cargo build --release",
        "cargo test --test schema_fixtures",
        "one issue per branch and pull request",
        "Do not self-review or self-merge",
        "Native `blockedBy` relations are authoritative",
        "Labels, assignees, and comments are coordination metadata, never locks",
        "git push --force-with-lease=\"$LOCK_REF:$LOCK_SHA\" origin \":$LOCK_REF\"",
        "Post-merge consistency audit",
        "squash merge only",
        "automatic branch deletion",
        "Closes #<number>",
        "--match-head-commit <reviewed-head-sha>",
        "GitHub rejects an author approval",
        "README",
        "VISION",
        "SEMANTICS",
        "schemas",
        "conformance",
        "wiki",
        "issues",
        "milestones",
    ] {
        assert!(contract.contains(required), "AGENTS.md omitted {required}");
    }
}

#[test]
fn project_profile_is_complete_for_the_reusable_delivery_loop() {
    let profile = read(".codex/project-profile.md");
    for required in [
        "Profile version: `1`",
        "Repository: `bhcp-dev/bhcp`",
        "Default branch: `main`",
        "GitHub adapter: `gh`",
        "Dependency field: native `blockedBy`",
        "status:ready",
        "status:claimed",
        "status:blocked",
        "status:review",
        "status:done",
        "refs/heads/codex-locks/project-bootstrap",
        "refs/heads/codex-locks/issues/<number>",
        "refs/heads/codex-locks/mutex/<resource>",
        "refs/heads/codex-locks/semaphore/<resource>/<slot>",
        "git push --force-with-lease=\"$LOCK_REF:$LOCK_SHA\" origin \":$LOCK_REF\"",
        "Rust quality / Format",
        "Rust quality / Clippy",
        "Rust quality / Tests",
        "Rust quality / Release build",
        "Rust quality / 17-root CDDL fixtures",
        "Strict required-check mode",
        "administrators are included",
        "force pushes and branch deletion are disabled",
        "allow_auto_merge = true",
        "delete_branch_on_merge = true",
        "allow_squash_merge = true",
        "allow_merge_commit = false",
        "allow_rebase_merge = false",
        "Closes #<number>",
        "--match-head-commit <reviewed-head-sha>",
        "Post-merge consistency audit",
        "Reviewed head SHA",
    ] {
        assert!(
            profile.contains(required),
            "project profile omitted {required}"
        );
    }

    assert!(profile.contains("gh issue list --repo bhcp-dev/bhcp"));
    assert!(profile.contains("--json number,title,body,state,milestone,labels,blockedBy,blocking"));
}

#[test]
fn contributor_entrypoint_links_to_both_canonical_contracts() {
    let readme = read("README.md");
    assert!(readme.contains("## Contributing and autonomous delivery"));
    assert!(readme.contains("[AGENTS.md](AGENTS.md)"));
    assert!(readme.contains("[project-loop profile](.codex/project-profile.md)"));
}
