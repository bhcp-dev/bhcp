# Phase 2 coding-agent evidence-loop audit

Status: the bounded Phase 2 evidence-loop implementation is complete through
[#27](https://github.com/bhcp-dev/bhcp/issues/27). This report closes the evidence
audit when [#28](https://github.com/bhcp-dev/bhcp/issues/28) merges.
BHCP v0 is not complete: the repository still provides focused safe-Rust slices rather than the
complete parser, checker, planner, runtime, proof system, execution graph, SDK, or CLI.

The machine ledger in [`phase-2-evidence-audit.txt`](phase-2-evidence-audit.txt)
pins every recorded experiment to its exact source, task/prompt, contract and
semantic ID, evaluated skill (or explicit absence), Codex/model/reasoning request,
withheld oracle, result report, executable test declaration, delivery PR, and
squash merge. Git blob checks cover both inputs and evidence, while an explicit
experiment-to-delivery map makes later artifact or provenance drift fail the Rust
suite.

## Classification

| Experiment | Admissibility | Result | Evidence interpretation |
| --- | --- | --- | --- |
| `pilot-001` | historical-unreproducible | neutral-tie | Both patches replay as accepted, but the CLI default model was not recorded. Pilot 001 cannot be reproduced at the model layer and is excluded from comparative inference. |
| `pilot-002` | historical-replay | neutral-tie | Explicit mini-model; both arms were accepted, with higher BHCP resource use. The initial judge selected the wrong Rust toolchain and was symmetrically restarted. |
| `pilot-003` | historical-replay | neutral-tie | All three patches were accepted; the skill calibrated its claim but added context overhead. |
| `pilot-004` | historical-replay | narrow-positive | On one deliberately information-asymmetric task, prose missed one invariant while both BHCP arms passed 7/7. |
| `pilot-005` | historical-replay | narrow-efficiency | The optimized skill retained 7/7 and reduced input versus the prior skill, not versus every baseline and not in latency. |
| `pilot-006` | historical-replay | mixed | Prose/raw BHCP passed 10/10; one skill run passed 8/10 and a follow-up passed 10/10 at higher cost. |
| `contextual-policy-multiseed-001` | infrastructure-invalid | invalid | Five contaminated sessions were excluded; patch replay is artifact integrity only. |
| `contextual-policy-multiseed-002` | infrastructure-invalid | invalid | Five host-readable sessions cannot establish oracle-withheld inference; patches remain replayable. |
| `contextual-policy-multiseed-003` | infrastructure-excluded | excluded | All five fixed arms stopped before a model turn; no replacement was made. |
| `contextual-policy-multiseed-004` | admissible | valid-negative | Read-confined preregistration completed: 0/5 accepted, all unchanged candidates passed 4/10 oracle invariants, and all claims were conservative. |
| `in-session-evidence-forward-001` | admissible | valid-negative | The model completed but made no edit, invoked no adapter, and claimed no success; controller result was 0/1 accepted. |

“Historical replay” means the checked-in inputs, patches, and independent judges are
reproducible; it does not retroactively claim the hardened host-read boundary added
for the registered runs. Runs 001–003 of the multi-seed series retain their original
invalid or excluded classifications. No failed or unfavorable record is replaced.

## Executable evidence

- The minimal fixture contract and public-green/oracle-red boundary are pinned by
  [`coding_agent_experiment`](../tests/coding_agent_experiment.rs), with exact
  [Pilot 001](minimal-coding-agent/results/pilot-001/README.md) and
  [Pilot 002](minimal-coding-agent/results/pilot-002/README.md) patches retained.
- The ambiguous policy fixture and its seven-invariant oracle are pinned by
  [`policy_resolution_experiment`](../tests/policy_resolution_experiment.rs), with
  [Pilot 003](policy-resolution-agent/results/pilot-003/README.md),
  [Pilot 004](policy-resolution-agent/results/pilot-004/README.md), and
  [Pilot 005](policy-resolution-agent/results/pilot-005/README.md) identities.
- [`contextual_policy_experiment`](../tests/contextual_policy_experiment.rs) replays
  every Pilot 006 and multi-seed 001/002 patch, proves run 003 remained unchanged,
  and replays all five run-004 identity patches through exact Rust 1.97.1 judges.
- [`in_session_evidence_experiment`](../tests/in_session_evidence_experiment.rs)
  proves the starter rejection, exact focused acceptance, subject-byte binding, and
  preserved [forward-001](in-session-evidence-agent/results/forward-001/README.md)
  negative result. [`in_session_evidence_runner`](../tests/in_session_evidence_runner.rs)
  freezes the packaged CLI, sandbox helper, adapter, candidate, skill, and plan.
- The generic controller rejection and isolation boundary remains pinned by
  [`experiment_controller`](../tests/experiment_controller.rs) and the protocol in
  [`CONTROLLER.md`](CONTROLLER.md).

The canonical audit command is `cargo test --test phase_two_evidence_audit`; the
full repository gate executes every linked replay and fixture test.

## Demonstrated outcomes

- The repository implements a contract-bounded project registry, a capability-
  bounded adapter runner, deterministic evidence mapping, and a fail-closed coding-
  agent controller. Integration tests prove exact accepted, rejected, unresolved,
  faulted, effect-ceiling, and subject-binding behavior.
- Historical pilots demonstrate reproducible patch/oracle outcomes and several
  narrow findings: correctness ties, one information-asymmetric positive case,
  token-intake improvement, and skill-run variance.
- The two admissible registered model studies are honest negatives: 0/5 for the
  repeated contextual-policy skill run and 0/1 for model-visible registered
  evidence. They prove the controller retains unfavorable outcomes and calibrated
  non-claims; they do not prove a positive success rate.

## Explicit non-claims and residual boundary

Positive in-session acceptance remains unproven. The runtime path itself is
implemented and independently accepts the exact focused candidate, so the negative
model turn is an empirical result rather than a missing adapter capability. The
small single-fixture samples support no hypothesis test, confidence interval,
population rate, model-wide effect, or causal skill effect. The record supports
no BHCP-versus-prose advantage. Those are future research questions outside this
finite implementation milestone, not prose substitutes for a missing required
runtime behavior.

The residual empirical gaps are now explicit roadmap work. [#92](https://github.com/bhcp-dev/bhcp/issues/92)
tracks positive in-session adapter use across a representative task sample, and
[#93](https://github.com/bhcp-dev/bhcp/issues/93) tracks symmetric BHCP-versus-prose
and claim-calibration effects. Both are natively blocked by the frozen protocol,
analysis, and resource-authorization decision in [#91](https://github.com/bhcp-dev/bhcp/issues/91),
under the separate **Future research — Evidence generalization** milestone. This
keeps the completed Phase 2 implementation boundary finite and its negative records
unchanged without presenting unmeasured claims as complete.

## Delivery and consistency

| Outcome | Issue | Pull request | Squash merge |
| --- | --- | --- | --- |
| Pilot 001–003 evidence package | pre-roadmap | [#6](https://github.com/bhcp-dev/bhcp/pull/6) | `93e3cc6b892bd4373fc112e74cd52de75fe82594` |
| Pilot 004 | pre-roadmap | [#8](https://github.com/bhcp-dev/bhcp/pull/8) | `98092552efda108cd3ce02e3787ad38239e09066` |
| Pilot 005 | pre-roadmap | [#9](https://github.com/bhcp-dev/bhcp/pull/9) | `64b5d164e4083041da0bbb09f10d5840a04f35d8` |
| Pilot 006 | [#21](https://github.com/bhcp-dev/bhcp/issues/21) | [#86](https://github.com/bhcp-dev/bhcp/pull/86) | `b227ce10e6e7e20c610c4d061a8cdb4fd15fd10c` |
| Safe controller | [#25](https://github.com/bhcp-dev/bhcp/issues/25) | [#87](https://github.com/bhcp-dev/bhcp/pull/87) | `56ae59f8fa8a8584891958f101d6a902767352fa` |
| Multi-seed series | [#26](https://github.com/bhcp-dev/bhcp/issues/26) | [#88](https://github.com/bhcp-dev/bhcp/pull/88) | `44bf1a1cf61f1829f3fbf839aea4067e06cb4a6c` |
| In-session adapters and forward 001 | [#27](https://github.com/bhcp-dev/bhcp/issues/27) | [#89](https://github.com/bhcp-dev/bhcp/pull/89) | `ee7ee62649daa31b9216379b562e7f43442231da` |

[`README`](../README.md) and [`VISION`](../VISION.md) use the same maturity and
non-claim language. Normative behavior remains in [`SEMANTICS`](../SEMANTICS.md);
this retrospective changes no semantic or wire contract. Issue #28, wiki status,
and the Phase 2 milestone are post-merge metadata and are reconciled only after the
reviewed audit reaches protected `main`.
