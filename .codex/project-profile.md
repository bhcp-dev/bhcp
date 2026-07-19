# BHCP project-loop profile

Profile version: `1`

This repository-local file is the canonical adapter profile for reusable issue
selection, issue-to-PR delivery, independent review/merge, and retrospective loops.

## Repository identity

- Repository: `bhcp-dev/bhcp`
- Repository URL: `https://github.com/bhcp-dev/bhcp`
- Default branch: `main`
- GitHub adapter: `gh`
- Issue tracker and wiki: GitHub repository features
- Merge method: head-matched squash, automatic branch deletion
- Repository merge settings: `allow_auto_merge = true`,
  `delete_branch_on_merge = true`, `allow_squash_merge = true`,
  `allow_merge_commit = false`, and `allow_rebase_merge = false`
- Work branches: `codex/<issue-slug>` from current `origin/main`
- Worktrees: one fresh isolated worktree per issue

Every live command must pass `--repo bhcp-dev/bhcp` or set the equivalent explicit
`GH_REPO`. Never infer repository identity from the current directory for a write.

## Source and authority policy

`SEMANTICS.md` is normative behavior; `schemas/v0/` is the CDDL wire contract;
conformance fixtures and tests are executable evidence; Rust source implements the
current slice. README/VISION explain maturity and direction. Wiki pages summarize
implemented, measured, decided, and planned work. Issues and milestones authorize
roadmap work but do not silently override normative semantics.

## Roadmap query and dependency policy

Dependency field: native `blockedBy`

Export the complete managed graph with:

```sh
gh issue list --repo bhcp-dev/bhcp --state all --limit 1000 --json number,title,body,state,milestone,labels,blockedBy,blocking
```

Selection uses open issues only, ordered first by blocker state, then
`priority:p0`, `priority:p1`, `priority:p2`, then milestone and issue number. An
issue is claimable only when every native blocker is closed and all concurrency
tokens can be acquired. The visible `Blocked by:` body line is a compatibility
mirror, not authority.

Readiness mapping:

- `status:ready`: open, no open native blocker, eligible for an atomic claim.
- `status:claimed`: issue/resource refs acquired; claim metadata reconciled.
- `status:blocked`: at least one open native blocker.
- `status:review`: implementation delivered; locks retained through review.
- `status:done`: exact reviewed head merged, issue closed, consistency complete.
- `status:stale`: evidence-based recovery is required; never permission to steal.

Each managed issue has exactly one milestone; one status, priority, kind, and risk;
at least one `area:<slug>`; and at least one concurrency declaration. Supported
concurrency labels are `concurrency:parallel`, `mutex:<resource>`, and
`semaphore:<resource>:<capacity>`.

## Atomic claim protocol

Labels, assignments, and comments are mirrors only. Authoritative locks are commits
atomically installed at these remote refs:

- Coordinator: `refs/heads/codex-locks/project-bootstrap`
- Issue: `refs/heads/codex-locks/issues/<number>`
- Mutex: `refs/heads/codex-locks/mutex/<resource>`
- Semaphore: `refs/heads/codex-locks/semaphore/<resource>/<slot>`

Create each lock commit over the current default-branch tree with the base commit as
parent. Its message records repository, issue/token, agent or task, UTC timestamp,
branch, and a unique nonce. While holding the coordinator, acquire required resource
refs in lexical order, a valid semaphore slot in numeric order, and finally the
issue ref. Roll back only refs created by the failed attempt, using fenced release.
After success, replace `status:ready` with `status:claimed`, assign when possible,
and comment with task, time, branch, worktree, scope, every ref/SHA, and every nonce.
Release the coordinator after metadata reconciliation; retain issue/resource refs
through review.

Release an owned ref only with Git's compare-and-delete fence:

```sh
LOCK_REF="refs/heads/codex-locks/issues/123"
LOCK_SHA="<acquired-commit-sha>"
git push --force-with-lease="$LOCK_REF:$LOCK_SHA" origin ":$LOCK_REF"
```

A lease rejection means lost ownership. Do not retry with a newer SHA. Stale
recovery requires inspecting claim metadata, task, branch, worktree, PR, head SHA,
and recent activity, documenting the recovery, and fencing deletion to the inspected
SHA.

## Implementation and local validation

Install the exact Rust toolchain with `mise install`. Canonical local commands are:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo build --release
cargo test --test schema_fixtures
```

Issue-specific focused tests run before the complete gate. Executable changes use
strict red-green-refactor TDD with the initial failure retained in PR evidence.
Rust models, CDDL, SEMANTICS, fixtures, inspection output, README, and wiki claims
must remain aligned when their shared contract changes.

## Remote CI gates

Required checks established by issue #12 and enforced on `main`:

- `Rust quality / Format`
- `Rust quality / Clippy`
- `Rust quality / Tests`
- `Rust quality / Release build`
- `Rust quality / 17-root CDDL fixtures`

The remote workflow uses the repository mise pin and the same commands. A pull
request is not mergeable merely because local checks pass. All configured required
checks must be green for the exact Reviewed head SHA.

Strict required-check mode is enabled on `main`; administrators are included. The
branch must be current before merge, force pushes are disabled, the protected branch
cannot be deleted, and every review conversation must be resolved. This does not
conflict with deleting a merged topic branch. The required contexts are bound to the
GitHub Actions app, not to author-supplied status names.

## Pull request, review, and merge

One issue maps to one focused branch and PR. The PR body includes `Closes #<number>`,
red-to-green evidence, focused/full commands and results, remote check names,
documentation/wiki impact, residual risk, and head SHA. Move the issue to
`status:review` while retaining locks.

The author must not review or merge the PR. GitHub rejects an author approval, and
neither an author comment nor green checks count as approval. A different agent or
task reviews the exact head, records its identity when GitHub accounts are shared,
resolves every actionable comment, reruns affected checks plus the full gate, and
confirms the Reviewed head SHA did not change. Prefer:

```sh
gh pr merge <number> --repo bhcp-dev/bhcp --squash --auto --delete-branch --match-head-commit <reviewed-head-sha>
```

If auto-merge is unavailable, the independent reviewer may omit `--auto` only after
all required gates pass. Never relax branch protection to complete a merge.

## Completion and reconciliation

Issue completion requires all acceptance criteria and evidence, full local and
remote gates, independent review of the unchanged head, squash merge, issue closure,
`status:done`, correct promotion of newly unblocked dependents, and fenced release of
the exact issue/resource refs.

Post-merge consistency audit: compare merged code and tests, code comments, README,
VISION, SEMANTICS, CDDL, conformance fixtures, `AGENTS.md`, this profile, every wiki
page, issue/milestone states, dependencies, readiness labels, and optional research
artifacts. Fix drift or open correctly blocked follow-ups. Roadmap completion means
all milestone acceptance outcomes are demonstrable, every issue is closed/done, and
the final audit reports no hidden contradiction.

## Safety boundaries

Do not expose secrets; force-push shared history; delete unverified refs or user
data; change global Git or Docker configuration; add dependencies without issue
scope and review; run destructive migrations; or regenerate canonical artifacts
without reviewed semantic intent. Preserve unrelated working-tree changes and all
external state outside the claimed issue.
