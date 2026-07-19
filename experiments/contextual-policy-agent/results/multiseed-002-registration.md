# Registration: contextual-policy multi-seed run 002

Registered: 2026-07-19, after run 001 was closed and before any run-002 session.

Run 001's five sessions were all excluded without replacement because the trusted driver failed to forward the controller-owned Cargo target path. Run 002 is a new five-session experiment using the regression-tested forwarding fix; it does not relabel or replace a run-001 observation.

The research question, sample size, fixed order (`seed-01` through `seed-05`), exact Pilot 006 evaluated skill, subject, task, BHCP contract and semantic ID, withheld ten-invariant oracle, `gpt-5.4-mini` model, medium reasoning, Codex CLI `0.142.4`, Rust `1.97.1`, sandbox, time limit, one-file policy, ordered judges, outcome definitions, exclusions, claim-calibration rule, distribution summaries, historical point comparison, and limits on inference are unchanged from [`multiseed-001-registration.md`](multiseed-001-registration.md).

The sole protocol change is the trusted-driver fix: `CARGO_TARGET_DIR` is now explicitly forwarded to Codex after environment clearing. A fake-Codex integration test proves that model-launched Cargo commands receive that controller-owned path. No evaluated skill, prompt, fixture source, public test, oracle test, arm count, arm order, threshold, outcome rule, or analysis rule changed in response to run 001's candidate patches.

As in run 001, a verification failure is an included semantic failure. Interruption, contamination, adaptive-oracle access, or incomplete protocol is reported and excluded without replacement. Every stored run-002 patch will be independently replayed through the frozen public and withheld oracle tests after all sessions stop. Five sessions remain too small for a hypothesis test, confidence interval, causal skill-effect claim, population rate, or generality beyond this frozen setup.
