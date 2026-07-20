# Evidence-generalization study preregistration

Registered on 2026-07-20 against `main` squash merge
`d5ef5ac29a12dabe2fe2af3f0ec35437204d29c8`. No model turn occurred before this registration.
The closed machine record is [`preregistration.txt`](preregistration.txt); its exact
artifact pins, schedule, limits, and analysis rules are normative for these studies.

This protocol authorizes two finite follow-ups: positive registered-adapter use in
[#92](https://github.com/bhcp-dev/bhcp/issues/92) and a symmetric representation
comparison in [#93](https://github.com/bhcp-dev/bhcp/issues/93). It does not alter or
reinterpret the completed [Phase 2 audit](../phase-2-evidence-audit.md).

## Population and task-selection rule

The population is the repository fixture frame at the registration base, not an
external software or developer population. A task is eligible only when that base
contains all of the following before registration: a dependency-free safe-Rust
subject, a checked-in implementation task, canonical BHCP contract and semantic ID,
deterministic public checks, a withheld deterministic oracle, and a one-file change
boundary. The selection is a census of the four eligible fixtures, stratified by the
failure shape each fixture exercises:

| Task | Stratum | Shared BHCP-arm task | Equal-information prose treatment |
| --- | --- | --- | --- |
| Atomic batch | State transaction | [task](../minimal-coding-agent/TASK.md) | [prose](tasks/atomic-batch-prose.md) |
| Tenant policy | Authorization specificity | [task](../policy-resolution-agent/TASK.md) | [prose](tasks/tenant-policy-prose.md) |
| Contextual policy | Ordered context | [task](../contextual-policy-agent/TASK.md) | [prose](tasks/contextual-policy-prose.md) |
| Evidence readiness | Evidence-gated repair | [task](../in-session-evidence-agent/TASK.md) | [prose](tasks/in-session-evidence-prose.md) |

The machine record pins each starter, shared task, prose treatment, contract,
semantic ID, and oracle. The prose treatments enumerate the same required outcome
boundary as the associated contracts; semantic equivalence remains a reviewed study
design claim rather than something a substring test can prove.

## Model, sessions, and arms

Every session uses Codex CLI `0.142.4`, `gpt-5.4-mini`, medium reasoning, Rust
`1.97.1`, and `workspace-write/no-network/read-confined`. The one registered arm
uses the exact checked-in `interpret-bhcp-contract` skill pinned in the machine
record. A seed label denotes a fresh isolated model session; it does not claim the
hosted model exposes or honors a deterministic random seed.

There are three fixed seed labels per task and no replacement:

- #92 runs twelve registered-evidence sessions: one `bhcp-registered` arm for every
  task × seed block. The model receives the shared task, canonical contract, pinned
  skill, and canonical registered-verifier command.
- #93 runs twelve paired comparative blocks: `prose-control` receives only the
  equal-information prose treatment, while `bhcp-contract` receives the shared task
  and canonical contract. Both comparative arms are forbidden from receiving the
  skill or project registry. Position 1 alternates across tasks and seeds, with each
  arm first in six blocks.

The studies are separate: #92 measures registry uptake; #93 estimates a bounded
prose-versus-contract representation contrast. Results are not substituted across
the two studies.

## Outcomes and analysis

For #92, the two primary binary outcomes over completed model turns are:

1. **Positive registry use:** the session invokes the canonical registry and retains
   a parseable evidence bundle bound to its exact candidate.
2. **In-session acceptance:** every mandatory registered target is accepted before
   the final claim and the independent controller later accepts the same candidate.

Report counts, proportions, and two-sided 95% Clopper–Pearson intervals overall and
by task. Claim calibration is secondary: success without complete accepted evidence
is an overclaim; complete accepted evidence with a negative claim is an underclaim.

For #93, the primary outcome is independent all-judge acceptance. The primary point
estimate is the paired risk difference (`bhcp-contract` minus `prose-control`) across
the twelve task × seed blocks. Report the two discordant counts and the two-sided
exact McNemar result as uncertainty evidence. Claim calibration uses the same paired
analysis with calibrated success/non-success as the positive category. Input,
cached-input, output, reasoning-token, command, and wall-time distributions are
secondary and reported by median and interquartile range. `alpha=descriptive-only`
means no result is promoted to a confirmatory population claim by a threshold.

Every task-level record and unfavorable result remains visible. No endpoint,
contrast, interval, exclusion, or arm may be changed after observing a model turn.

## Exclusions and stopping

A completed model turn is included even if it makes no edit, omits the registry,
fails a judge, returns rejected/unresolved/faulted evidence, or miscalibrates its
claim. An interrupted pre-model launch, identity drift, read-boundary failure,
contamination, incomplete closed result record, or unavailable required sandbox is
an infrastructure exclusion. Exclusions are never replaced.

For a comparative block, an infrastructure exclusion in either arm removes that
block from the paired estimate, while every completed arm remains in the descriptive
ledger. Counts and reasons are always reported. There is no efficacy or futility
stopping. A safety, identity, sandbox, or resource-ceiling failure stops subsequent
launches and retains all completed records; it cannot trigger a changed task, arm,
seed, or replacement.

## Resource and authorization decision

Merging the reviewed #91 preregistration authorizes at most 36 model sessions, 540
aggregate model-minutes, 12,000,000 reported input tokens, 500,000 output tokens,
500,000 reasoning tokens, and two concurrent experiment slots. The authorization
uses only the repository owner's existing Codex entitlement and sets incremental
pay-as-you-go spend to **USD 0**. It does not authorize pay-as-you-go spend: if a
launch requires an incremental charge or would exceed any ceiling, stop before that
launch and request a new reviewed authorization. Unused capacity does not justify
extra or replacement sessions.

## Preflight closure before execution

Issues #92 and #93 must each freeze their exact runner, project manifests, adapters,
prompts, controller plan, executables, toolchain files, and judge order in a Git
commit before the first model turn. Infrastructure-only smoke checks may prove that
the staged prompt is readable, the original oracle is unreadable, and the sandbox
and registry launch. A smoke cannot edit a study candidate or produce a research
observation. After the first model turn, infrastructure repairs require a new run ID
and registration; earlier records retain their original classification.

## Inference boundary

The design measures four repository-owned Rust fixtures under one model and three
fresh sessions per task. It cannot establish a population rate, causal language
effect, model-wide effect, developer-productivity effect, or general BHCP advantage.
A null, unfavorable, incomplete, or invalid result is a conforming deliverable when
reported under this frozen protocol.
