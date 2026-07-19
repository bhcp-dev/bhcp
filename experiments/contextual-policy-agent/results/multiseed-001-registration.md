# Registration: contextual-policy multi-seed run 001

Registered: 2026-07-19, before any included session.

## Question and scope

This run measures whether the Pilot 006 compact evaluated skill's ordered-obligation failure and token behavior recur across five additional independent stochastic sessions on the unchanged contextual-policy fixture. Codex CLI exposes no numeric model seed; `seed-01` through `seed-05` are fixed session identifiers, not claims that a hidden sampler seed is controlled. Five sessions can reveal variance and obvious recurrence, but cannot establish population rates or generality beyond this fixture, model, skill, and toolchain.

## Frozen protocol

- Sample count and order: `seed-01`, `seed-02`, `seed-03`, `seed-04`, `seed-05`; no replacement runs.
- Every session starts from the same subject, receives the same prompt, canonical BHCP contract, semantic-ID pin, and exact evaluated skill, and has no oracle path while the agent is active.
- Evaluated skill: Pilot 006 `evaluated-skill/SKILL.md`, Git blob `b1a2f5fdfb3044be679f1e947bf1a1e56957e278`; its text will not change during this run.
- Subject `src/lib.rs` blob: `3f126bfde1c0e06309686c9c3514548759d650eb`.
- Task blob: `82ae3d3545ee1f73fe6ed7180a1278e4680ab420`.
- Contract blob: `dfb58210587b15abfc0d0cbaa337a653b5d6dd29`.
- Withheld oracle test blob: `3667d107a7777a09f71a69c871802c0f4e07dde1`; ten frozen invariants.
- Model and reasoning: `gpt-5.4-mini`, medium.
- Runtime: Codex CLI `0.142.4`, Rust `1.97.1`, ephemeral Codex state, ignored user configuration, approval policy `never`, workspace-write sandbox with agent-command network disabled, fifteen-minute session limit, and only `subject/src/lib.rs` permitted to change.
- Judges, in order: Rustfmt, offline Clippy with warnings denied, all five public tests, then all ten withheld oracle invariants. Each judge receives a fresh candidate view; only the last receives the oracle.

The checked-in safe-Rust runner freezes the complete fixture, executable identities, absolute commands, arm order, limits, and tool pins through `bhcp::experiment` before launch. It reduces raw Codex JSON events to token and completed-command counts without storing model traces in Git. Candidate patches and the controller's Markdown evidence are created only after all five sessions stop.

## Outcomes and exclusions

The primary result is the count and proportion of the five registered candidates accepted by every judge. A verification failure is an included semantic failure. A controller rejection for interruption, contamination, adaptive-oracle access, or incomplete protocol is reported but excluded from the semantic acceptance denominator; it is never replaced. Every stored patch will be replayed from the pinned starter through the frozen public and oracle tests.

Secondary measures are the ten-invariant pass count and failed invariant names per included candidate; input, cached-input, output, and reasoning-token distributions; completed-command and wall-time distributions; and final-claim calibration. Because the registered skill cannot receive its bound withheld oracle during the session, `claimed_success=false` is the evidence-calibrated claim; `true` is recorded as overclaiming even if later independent judges accept the patch. For comparison only, the Pilot 006 exact evaluated-skill session failed two of ten invariants with 153,676 input tokens, 139,008 cached tokens, 5,985 output tokens, 4,009 reasoning tokens, 15 completed commands, and 96.95 seconds.

Results will report all individual observations plus ranges and medians. No hypothesis test, confidence interval, causal skill effect, model-wide rate, or BHCP-versus-prose advantage will be claimed from this small single-arm sample.
