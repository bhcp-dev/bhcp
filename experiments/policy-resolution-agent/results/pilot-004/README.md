# Pilot 004: refined ambiguity and canonical inspection

Date: 2026-07-18

This round repeats the three-arm policy-resolution experiment with the merged
Rust/CBOR interpretation skill and a more concise production ticket. The subject,
canonical contract, visible tests, and withheld oracle are unchanged from Pilot
003. The refined prose names the relevant policy concepts but does not disclose
their precedence.

## Verdict

This is the first pilot in which canonical BHCP intent changed independent
acceptance:

- prose produced a plausible priority-first comparator, claimed success, and failed
  the withheld specificity invariant;
- raw BHCP produced the canonical specificity-first comparator, claimed success,
  and passed every independent check; and
- BHCP plus the merged skill produced the canonical comparator, passed every
  independent check, and correctly refused to claim success while the bound
  adapters were unavailable to the session.

The result demonstrates the intended advantage on this frozen case: canonical
intent prevented a reasonable prose interpretation from silently changing policy.
It remains one sequential run of an intentionally information-asymmetric
experiment, not general evidence that BHCP improves every coding task.

## Frozen inputs and controls

All arms started from isolated repositories with:

- BHCP main commit: `af90e976d227d7e21503fad42cf09ab348c30173`;
- `src/lib.rs` Git blob: `f84499b07f4e01f009ecb9bba4be3798a31e3f73`;
- refined task: [`refined-task.md`](refined-task.md), Git blob
  `5b30594b6099af66291a19abfab3b62bd5db7db1`;
- public tests: 4;
- withheld oracle tests: 7;
- Rust `1.97.1` available through the repository pin;
- network use prohibited; and
- `src/lib.rs` as the only permitted edit.

Both BHCP arms received [`../../contract.bhcp`](../../contract.bhcp), the same
compiled `bhcp` CLI on `PATH`, and this pinned semantic ID:

```text
bhcp.hash/sha3-512@0:d10ead1268ed05db2bfbc018756555804360fc6aa3369ea96a71adf0750850460d0d834160c6c617da1a4347c928885b5a5a3b29b93d5ab29252dbd9e3156880
```

The skill arm additionally received and explicitly invoked the merged revision
saved under [`evaluated-skill/`](evaluated-skill/). Its Git blobs were:

- `SKILL.md`: `c516e2e71dbe843280f953fa1dca182a18dcd4fe`;
- `agents/openai.yaml`: `5d4e7c9a119e934760b058792b36b491a460320b`.

All sessions used Codex CLI `0.142.4`, `gpt-5.4-mini`, medium reasoning,
ephemeral state, ignored user configuration, and workspace-write isolation. The
run order was skill, raw BHCP, then prose. The oracle was absent until every agent
stopped. Controller event records remained outside the repository; only the frozen
inputs, candidate patches, and derived report are checked in.

## Prompts

The shared prompt was:

```text
Work autonomously on the coding task in TASK.md. Meet every stated requirement; an independent oracle is withheld until you stop. Do not use the network. Edit only src/lib.rs and preserve the public API. Run all visible formatting, Clippy, and test checks before reporting success. Do not ask questions. In your final response, state whether you claim success, name every changed file, and list the checks you actually ran.
```

The raw BHCP arm additionally received:

```text
The canonical machine contract is contract.bhcp and its pinned identity is contract.semantic-id. Treat every targeted obligation and verifier binding in that contract as required for acceptance.
```

The skill arm began with:

```text
Use $interpret-bhcp-contract to work autonomously on the coding task in TASK.md.
```

It received the same BHCP authority statement plus an explicit instruction to
report obligation evidence only when its bound producer ran.

## Results

| Measure | Prose | Raw BHCP | BHCP + skill |
| --- | ---: | ---: | ---: |
| Claimed success | yes | yes | no |
| Independently accepted | **no** | **yes** | **yes** |
| Public tests | 4/4 | 4/4 | 4/4 |
| Withheld policy invariants | 6/7 | 7/7 | 7/7 |
| Formatting and Clippy | pass | pass | pass |
| One-file/dependency/API policy | pass | pass | pass |
| Patch size | +31/-2 | +28/-2 | +27/-2 |
| Completed commands | 9 | 18 | 19 |
| Failed intermediate commands | 0 | 2 | 2 |
| Input tokens | 76,136 | 141,372 | 210,920 |
| Cached input tokens | 66,304 | 127,360 | 192,000 |
| Output tokens | 3,634 | 5,057 | 6,856 |
| Reasoning output tokens | 2,220 | 2,904 | 3,223 |
| Approximate wall time | 59 s | 94 s | 112 s |
| Resulting `src/lib.rs` Git blob | `08ad6554f5c464bd4e23964880ced4f7b20e3e85` | `e026e38f3fc9c39528221b0a5ae21c793fcfb80d` | `6fb8b4dab61606438a5a2748bec5efdc2b440b27` |

Exact candidate changes are preserved in [`prose.patch`](prose.patch),
[`bhcp.patch`](bhcp.patch), and [`skill.patch`](skill.patch).

## Independent acceptance

The controller copied the withheld oracle into three fresh judge directories and
ran formatting, offline Clippy, all public tests, all oracle tests,
`git diff --check`, and the one-file change policy.

The prose candidate ordered rules by priority, then specificity, denial, and rule
ID. It passed every visible check and six independent invariants but failed
`specificity_dominates_numeric_priority`: a broad high-priority denial displaced an
exact low-priority permission.

Both BHCP candidates ordered by specificity, priority, denial, and smaller rule ID.
They passed tenant isolation, default denial, both precedence invariants, denial and
ID tie-breaking, and insertion-order independence.

## Skill finding

The merged skill corrected the concrete Pilot 003 process defects:

- it ran exactly one `bhcp hash` and one `bhcp inspect`;
- it never emitted or reread raw AST or IR;
- it mapped verifier arrows to obligation IDs before editing; and
- it distinguished implemented conditions from accepted verifier evidence.

The skill agent therefore returned the right patch and the best-calibrated claim.
It marked the policy obligations unresolved during the session, then the external
judge supplied the missing adapter-equivalent evidence and accepted them.

Efficiency remains an open issue. Relative to raw BHCP, the skill used 49.2% more
input tokens and 19.1% more wall time. Those premiums are smaller than Pilot 003's
63.9% and 29.4%, respectively, and the event record was no longer truncated by
compiler artifacts. The absolute token count was still higher than the prior skill
run, so this single changed challenge supports a process improvement, not a stable
efficiency trend.

The next useful boundary is executable verifier adapters available inside the agent
session. That would let the same structural matrix advance from `implemented` to
`accepted` without a manual post-session judge, and would test whether the skill can
gain evidentiary rigor without retaining its current reporting overhead.
