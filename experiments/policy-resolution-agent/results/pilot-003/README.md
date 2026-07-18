# Pilot 003: ambiguous prose, raw BHCP, and BHCP skill

Date: 2026-07-18

This is the first run of the policy-resolution fixture and the first three-arm BHCP
comparison. It is numbered Pilot 003 to preserve the two immutable ledger pilots.

## Verdict

All three candidates passed every independent verifier. The result is a patch-
correctness tie: the prose-only model inferred the complete intended precedence
ladder even though the ticket did not state it explicitly.

The arms differed materially in process and claim calibration:

- raw BHCP used the fewest tokens, shortest wall time, and smallest patch;
- prose claimed success after visible checks and was later accepted;
- raw BHCP claimed success after visible checks and was later accepted; and
- BHCP plus the interpretation skill produced an accepted patch but correctly
  refused to claim success while its bound oracle evidence remained unavailable.

The skill therefore improved evidentiary discipline in this run, but its first
revision did not improve efficiency. It printed full AST and IR artifacts into the
session and used substantially more tokens than raw BHCP.

## Frozen inputs and controls

All arms started from isolated repositories with:

- `src/lib.rs` Git blob: `f84499b07f4e01f009ecb9bba4be3798a31e3f73`;
- task: [`../../TASK.md`](../../TASK.md);
- public tests: 4;
- withheld oracle tests: 7;
- Rust `1.97.1` preselected through `RUSTUP_TOOLCHAIN`;
- network use prohibited; and
- `src/lib.rs` as the only permitted edit.

Both BHCP arms received [`../../contract.bhcp`](../../contract.bhcp), the same
compiled `bhcp` CLI on `PATH`, and this pinned semantic ID:

```text
bhcp.hash/sha3-512@0:d10ead1268ed05db2bfbc018756555804360fc6aa3369ea96a71adf0750850460d0d834160c6c617da1a4347c928885b5a5a3b29b93d5ab29252dbd9e3156880
```

The skill arm additionally received and explicitly invoked the skill revision saved
under [`evaluated-skill/`](evaluated-skill/). Its Git blobs were:

- `SKILL.md`: `39f4c88528d5c7256aef1427279abcae5f261e77`;
- `agents/openai.yaml`: `33cd39fa1c97be5715db5fba61d61a142c73a409`.

All sessions used Codex CLI `0.142.4`, `gpt-5.4-mini`, medium reasoning,
ephemeral state, ignored user configuration, and workspace-write isolation. The
run order was prose, raw BHCP, then BHCP plus skill. The oracle was copied only
after every session stopped.

## Results

| Measure | Prose | Raw BHCP | BHCP + skill |
| --- | ---: | ---: | ---: |
| Claimed success | yes | yes | no |
| Independently accepted | yes | yes | yes |
| Public tests | 4/4 | 4/4 | 4/4 |
| Withheld policy invariants | 7/7 | 7/7 | 7/7 |
| Formatting and Clippy | pass | pass | pass |
| One-file/dependency/API policy | pass | pass | pass |
| Patch size | +29/-2 | +20/-2 | +31/-2 |
| Completed commands observed | 18 | 16 | at least 17 |
| Input tokens | 176,668 | 115,220 | 188,791 |
| Cached input tokens | 158,848 | 99,328 | 159,232 |
| Output tokens | 6,580 | 4,114 | 5,155 |
| Reasoning output tokens | 4,164 | 2,280 | 2,904 |
| Approximate wall time | 107.8 s | 66.5 s | 86.0 s |
| Resulting `src/lib.rs` Git blob | `afcb64516c40a0332ee82294acb958ed986c8691` | `5f45125656e04c507f62c2ff5fe8d49a2691389b` | `518c62615c5532115569dbc1d6e9233b16216afa` |

The skill arm's event stream was truncated by its full compiler-artifact output, so
17 is a lower bound on completed commands. Token totals and final usage were still
present and are exact as reported by the CLI.

Relative to prose, raw BHCP used 34.8% fewer input tokens and finished 38.3%
faster. Relative to raw BHCP, the evaluated skill revision used 63.9% more input
tokens and took 29.4% longer. This is a single sequential run, not statistical
evidence of model or representation efficiency.

Exact candidate changes are preserved in [`prose.patch`](prose.patch),
[`bhcp.patch`](bhcp.patch), and [`skill.patch`](skill.patch).

## Independent acceptance

The controller copied the same withheld oracle into three separate judge checkouts
and ran formatting, offline Clippy, all public tests, all oracle tests,
`git diff --check`, and the one-file change policy. Every command passed for every
arm. The seven oracle tests covered tenant isolation, default denial, specificity
before priority, priority within equal specificity, deny on policy ties, smaller
rule ID as the final tie-breaker, and insertion-order independence.

## Skill finding

The skill's refusal to claim unverified success is desirable under BHCP's evidence
model: a plausible implementation and visible tests do not substitute for a bound
oracle. The post-session judge later supplied the missing evidence and accepted the
candidate.

The forward test also found two efficiency defects in the evaluated skill:

1. it instructed the agent to run all four compiler views even when `hash`,
   `inspect`, and a compact slice of `lower` were sufficient; and
2. it did not explicitly separate an apparently implemented condition from an
   obligation accepted by its bound verifier evidence.

The checked-in live skill now implements both corrections: it extracts compact
lowered slices and separates implementation state from evidence state. The exact
revision used by this run remains preserved under [`evaluated-skill/`](evaluated-skill/)
so a later skill-arm run can compare revisions without rewriting this evidence.
