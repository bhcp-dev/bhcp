# Registration: in-session registered-evidence forward run 001

Registered: 2026-07-19, before the only forward-001 model session.

Experiment ID: `in-session-evidence-forward-001`.

## Question and fixed arm

This forward test asks whether one coding-agent session can use the canonical
project registry to advance public, oracle, and change-policy obligations to
accepted evidence before its final claim, without manual post-session judging or
additional authority from the local manifest.

There is one fixed arm, `forward-01`, with no replacement. It uses
`gpt-5.4-mini`, medium reasoning, Codex CLI 0.142.4, Rust 1.97.1, a fifteen-minute
limit, and `workspace-write/no-network/read-confined`. The evaluated
`interpret-bhcp-contract` skill is the unchanged Pilot 006 blob
`b1a2f5fdfb3044be679f1e947bf1a1e56957e278`; skill guidance will not change until
this forward result closes.

The starter, task, prompt, contract, semantic ID, fixed typed candidate, manifest,
adapter binary, packaged adapter sandbox, Codex, BHCP, Rustup, and six Rust
toolchain executables are frozen. The prepared fixture is
`/private/tmp/bhcp-in-session-evidence-forward-001-fixture-20260719`. The plan
digest is
`bhcp.hash/sha3-512@0:7149bc3fd7e8d843692ce2fb68804e6c3303b4cd45601204a126d17d28aefdad778e44b104d8e27bcc240647eea0e2024647f6c9a6bf0e582f05d55b992bf021`;
the fixture digest is
`bhcp.hash/sha3-512@0:f6ba9badf1dd33553b9a86818ce8ac56aa5a3b4b163cb952fec9e27b0d24894cce31870998e4a627ddabec7f874db7234910a0991d799b9b9e19a73552a50bb3`.

## Evidence and decision rule

The session receives the canonical `bhcp verify` command and must inspect the
resulting evidence bundle. Three registered adapters run through the
capability-bounded process runner with project-read authority derived from the
contract, not self-authorized by the manifest. Public evidence checks the visible
readiness behavior; oracle evidence checks all readiness behavior; change-policy
evidence accepts only the exact focused final source.

After the model stops, the independent controller runs exact Rustfmt, offline
Clippy with warnings denied, the public Rust test, the withheld Rust oracle, and an
exact-source change-policy judge in that order. Only `subject/src/lib.rs` may
change. The session succeeds only if every controller judge accepts, the retained
in-session bundle discharged every mandatory obligation through all three bound
adapters, and `claimed_success=true`. A false claim with accepted in-session and
controller evidence is underclaiming; a true claim without complete accepted
evidence is overclaiming.

Timeout, contamination, read-boundary failure, identity drift, incomplete result
protocol, or failure before a model turn is an unreplaced infrastructure
exclusion. A completed model turn that omits the registry, receives rejected,
unresolved, or faulted evidence, leaves an invalid candidate, or makes a
miscalibrated claim is an included forward-test failure. One deliberately small
task tests the evidence path, not model coding ability, and supports no population,
causal, model-wide, or BHCP-versus-prose claim.
