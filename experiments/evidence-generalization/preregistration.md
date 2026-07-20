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
boundary. The positive-use selection is a census of the four eligible fixtures,
stratified by the failure shape each fixture exercises. The comparative selection is
the three fixtures whose obligations can be represented without the registry:

| Task | Stratum | Study frame | Shared BHCP-arm task | Prose treatment |
| --- | --- | --- | --- | --- |
| Atomic batch | State transaction | Positive use + comparative | [task](../minimal-coding-agent/TASK.md) | [equal-information prose](tasks/atomic-batch-prose.md) |
| Tenant policy | Authorization specificity | Positive use + comparative | [task](../policy-resolution-agent/TASK.md) | [equal-information prose](tasks/tenant-policy-prose.md) |
| Contextual policy | Ordered context | Positive use + comparative | [task](../contextual-policy-agent/TASK.md) | [equal-information prose](tasks/contextual-policy-prose.md) |
| Evidence readiness | Evidence-gated repair | Positive use only | [task](../in-session-evidence-agent/TASK.md) | [descriptive prose](tasks/in-session-evidence-prose.md) |

The machine record pins each starter, shared task, prose treatment, contract,
semantic ID, and oracle. The prose treatments enumerate the same required outcome
boundary as the associated contracts. For the three comparative fixtures, the prose
also states the contract's minimal-change preference; equal information remains a
reviewed study-design claim rather than something a substring test can prove. The
evidence-readiness task is not comparative because its required registry observation
would contradict the comparative arms' registry-forbidden boundary.

## Model, sessions, and arms

Every session uses Codex CLI `0.142.4`, `gpt-5.4-mini`, medium reasoning, Rust
`1.97.1`, and `workspace-write/no-network/read-confined`. The one registered arm
uses the exact checked-in `interpret-bhcp-contract` skill pinned in the machine
record. A seed label denotes a fresh isolated model session; it does not claim the
hosted model exposes or honors a deterministic random seed.

The positive-use study has three fixed seed labels for each of its four tasks. The
comparative study has four fixed seed labels for each of its three tasks. There is no
replacement:

- #92 runs twelve registered-evidence sessions: one `bhcp-registered` arm for every
  task × seed block. The model receives the shared task, canonical contract, pinned
  skill, and canonical registered-verifier command.
- #93 runs twelve paired comparative blocks: `prose-control` receives only the
  equal-information prose treatment, while `bhcp-contract` receives the shared task
  and canonical contract. Both comparative arms are forbidden from receiving the
  skill or project registry. Evidence readiness is excluded because requiring its
  registry evidence would contradict that rule. Position 1 alternates across tasks
  and seeds, with each arm first in six blocks.

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
analysis with calibrated success/non-success as the positive category. For this
endpoint, claimed success exactly equals all-judge acceptance. Input,
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
stopping. A safety, identity, sandbox, or enforceable-resource failure stops
subsequent launches and retains all completed records; it cannot trigger a changed
task, arm, seed, or replacement. Monitored usage thresholds are checked after each
completed session, so one completed session can overshoot a threshold before later
launches stop.

## Resource and authorization decision

Merging the reviewed #91 preregistration authorizes the enforceable resource budget:
at most 36 model sessions, 15 minutes per session, 540 aggregate model-minutes, two
concurrent experiment slots, and **USD 0** incremental pay-as-you-go spend. Before
the first launch, the runner must reject API-key environment variables and API-base
overrides and require a recorded owner billing attestation that the existing Codex
entitlement is being used; absent attestation stops execution.
It does not authorize pay-as-you-go spend.

Reported usage has monitoring stop thresholds of 12,000,000 input tokens, 500,000
output tokens, and 500,000 reasoning tokens. These are not hard caps because the
controller observes usage only after completion: one completed session can
overshoot. Crossing a threshold stops later launches and requires new reviewed
authorization. A launch that requires incremental billing also stops before the
model turn. Unused capacity does not justify extra or replacement sessions.

## Preflight closure before execution

Issues #92 and #93 must each freeze their exact runner, project manifests, adapters,
prompts, controller plan, executables, toolchain files, judge order, per-session
timeout, post-session usage monitor, and billing preflight in a Git commit before the
first model turn. Infrastructure-only smoke checks may prove that the staged prompt
is readable, the original oracle is unreadable, and the sandbox and registry launch.
A smoke cannot edit a study candidate or produce a research observation. After the
first model turn, infrastructure repairs require a new run ID and registration;
earlier records retain their original classification.

## Inference boundary

The design measures four repository-owned Rust fixtures in positive use and three in
the representation comparison, under one model and the frozen seed schedules. It
cannot establish a population rate, causal language effect, model-wide effect,
developer-productivity effect, or general BHCP advantage. A null, unfavorable,
incomplete, or invalid result is a conforming deliverable when reported under this
frozen protocol.
