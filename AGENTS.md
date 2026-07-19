# BHCP repository operating contract

## Purpose and authority

BHCP makes outcomes, authority, limits, and required evidence part of a typed
semantic program. This repository is the normative v0 foundation and a set of
focused executable Rust slices; it is not yet a complete parser, checker, planner,
runtime, SDK, or proof that BHCP outperforms conventional languages.

Use this authority order when claims conflict: `SEMANTICS.md` for normative
behavior; `schemas/v0/` for the wire contract; checked-in conformance fixtures and
tests for executable evidence; implementation code; `README.md` and `VISION.md`;
the wiki; then live issues and milestones for planned work. Record contradictions
instead of silently choosing a convenient source. The repository-owned stack is
safe Rust only. Do not add another project language runtime without an explicit,
reviewed roadmap decision.

## Commands and meaningful TDD

Install the exact toolchain with `mise install`. The canonical local gate is:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo build --release
cargo test --test schema_fixtures
```

Start executable changes with a focused failing test or deterministic artifact
validator, confirm that it fails for the intended reason, implement the smallest
coherent behavior, and rerun the focused check before the complete gate. Test
observable invariants, boundaries, invalid inputs, error categories, state
transitions, identity, and determinism as applicable. Do not treat coverage,
existence-only tests, or self-reported success as acceptance evidence.

## Issue-to-merge workflow

Work from a fresh worktree based on current `origin/main`, on a `codex/<slug>`
branch, with one issue per branch and pull request. Preserve the user's shared
checkout and unrelated changes. Select only an open `status:ready` issue whose
native blockers are all closed and whose concurrency tokens are available.

The pull request must link the issue and record red-to-green evidence, focused and
full validation, documentation impact, residual risk, and the exact head SHA under
review. Its body uses `Closes #<number>` so the reviewed merge closes exactly its
issue. Do not self-review or self-merge. GitHub rejects an author approval; an author
comment or green checks are not substitute approval. A different agent or task must
record its identity when a GitHub account is shared, resolve all actionable comments,
and verify the unchanged reviewed head.

The repository permits squash merge only and uses automatic branch deletion. Queue
the exact reviewed head when required checks are still running:

```sh
gh pr merge <number> --repo bhcp-dev/bhcp --squash --auto --delete-branch --match-head-commit <reviewed-head-sha>
```

If every required check already passes, the reviewer may omit `--auto`; never omit
the head match or weaken repository protection to make a merge pass.

## Atomic claims and concurrency

Native `blockedBy` relations are authoritative; issue-body dependency text is a
human-readable compatibility mirror. Status, priority, kind, risk, area, and
concurrency labels must match the live issue state. Labels, assignees, and comments are coordination metadata, never locks.

Cross-machine exclusion uses atomically created remote Git refs. Serialize claim
selection with `refs/heads/codex-locks/project-bootstrap`; acquire resource tokens
in lexical order; then acquire `refs/heads/codex-locks/issues/<number>`. Resource
refs are `refs/heads/codex-locks/mutex/<resource>` or numbered
`refs/heads/codex-locks/semaphore/<resource>/<slot>` refs within declared capacity.
Every lock commit has a unique nonce, task, timestamp, branch, and base parent.
Mirror acquired ref paths and exact SHAs in the claim comment and retain them
through review.

Release only a ref owned by the task, fenced to the acquired SHA:

```sh
git push --force-with-lease="$LOCK_REF:$LOCK_SHA" origin ":$LOCK_REF"
```

If the lease fails, ownership changed: do not retry against the newer SHA. Recover a
stale claim only after inspecting its task, comment, branch, worktree, PR, timestamps,
and recent activity and documenting the decision.

## Consistency and safety

After every merge, perform a Post-merge consistency audit across merged code and
tests, code comments, README, VISION, SEMANTICS, schemas, conformance, wiki, issues,
milestones, this file, and `.codex/project-profile.md`. Update affected claims or
open a correctly blocked follow-up; never hide drift in celebratory prose.

Never expose secrets, rewrite shared history, delete user data, run destructive
migrations, add unreviewed dependencies, regenerate canonical fixtures incidentally,
or mutate external state beyond the claimed issue. Preserve deterministic CBOR,
stable diagnostics, semantic/artifact identity boundaries, and the distinction
between satisfied, refuted, unresolved, and faulted results.
