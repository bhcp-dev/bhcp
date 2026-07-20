# Comparative representation study registration

This machine-closed registration implements the comparative arm of the parent
[evidence-generalization preregistration](preregistration.md) for issue #93. It
freezes 12 paired blocks and 24 sessions before any comparative model turn.

Each of the three repository fixtures runs four fresh seed-labelled blocks. The
registered order alternates within tasks and gives each arm the first position in
six blocks. `prose-control` receives only its equal-information prose treatment;
`bhcp-contract` receives the shared task, canonical contract, and semantic ID.
Neither arm receives a BHCP skill, project registry, verifier adapter, evidence
bundle, or withheld oracle. Both arms may change only `subject/src/lib.rs` and face
the identical format, Clippy, public-test, withheld-oracle, and safe-Rust
change-policy judges.

The exact source artifacts, prepared-fixture digests, controller plans, run order,
executable identities, model/toolchain pins, timeout, and resource boundaries are
closed in [`comparative-registration.txt`](comparative-registration.txt). Execution
requires ChatGPT entitlement authentication, rejects API-key and API-base settings,
uses sequential concurrency, authorizes at most 24 sessions and 360 model-minutes,
and authorizes USD 0 incremental pay-as-you-go spend.

The primary estimate is the paired acceptance risk difference (BHCP minus prose)
over blocks with two non-excluded arms, with discordant counts and a two-sided exact
McNemar value. Exact claim calibration uses the same paired analysis. Token,
completed-command, and wall-time distributions are secondary medians and Tukey-hinge
interquartile ranges by arm. Infrastructure exclusions remove their pair, remain in
the ledger, stop later launches, and are never replaced.

`alpha=descriptive-only`. Every neutral, unfavorable, failed, incomplete, or
excluded observation must remain visible. This small single-model repository study
cannot establish a population effect, causal language effect, model-wide effect,
developer-productivity effect, or general BHCP advantage.
