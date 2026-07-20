# Forward run 001: registered negative result

Date: 2026-07-19

Forward 001 produced **0/1 accepted**. The fixed model turn completed inside the
read-confined boundary but made no edit, recorded no completed command execution,
did not invoke the canonical registry, and returned `claimed_success=false`.
Because the turn completed, this is an included forward-test failure under the
registration, not an infrastructure exclusion, and it is not replaced.

## Frozen outcome

The plan digest was
`bhcp.hash/sha3-512@0:7149bc3fd7e8d843692ce2fb68804e6c3303b4cd45601204a126d17d28aefdad778e44b104d8e27bcc240647eea0e2024647f6c9a6bf0e582f05d55b992bf021`;
the fixture digest was
`bhcp.hash/sha3-512@0:f6ba9badf1dd33553b9a86818ce8ac56aa5a3b4b163cb952fec9e27b0d24894cce31870998e4a627ddabec7f874db7234910a0991d799b9b9e19a73552a50bb3`.
The arm used the unchanged Pilot 006 evaluated skill, `gpt-5.4-mini` with medium
reasoning, Codex CLI 0.142.4, exact Rust 1.97.1 executables, and the registered
`workspace-write/no-network/read-confined` boundary.

| Arm | Result | Claim | In-session registry | Independent judges | Input / cached | Output / reasoning | Commands | Wall time |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: |
| forward-01 | verification failure | no | not invoked | format + Clippy pass; public + oracle + change reject | 189,447 / 171,136 | 4,192 / 3,447 | 0 | 90.298 s |

The controller recorded identical before/after subject identities and the exact
candidate is therefore the empty `forward-01.patch`. The unchanged starter failed
the public Rust test, withheld Rust oracle, and exact-source change-policy judge.
`claimed_success=false` was calibrated to that evidence boundary.

## What this does and does not establish

The prepared session did expose all three project registrations, the canonical
`bhcp verify` CLI, a fixed typed candidate, the packaged capability sandbox, and an
exact prompt command. However, the agent produced no in-session evidence bundle,
so this arm does not demonstrate successful accepted-evidence agreement. It also
did not invent or overclaim evidence. The independently checked integration path
proves that the same bounded registrations reject the starter, accept the exact
focused candidate, discharge every mandatory obligation, distinguish rejected,
unavailable, and malicious output, and cannot self-authorize effects beyond the
contract.

The evaluated skill remained unchanged until this result closed. Its separately
maintained latest version now documents the canonical registry workflow and exit
distinctions; forward 001 is not adaptively rerun. One deliberately simple task and
one negative turn support no population, causal, model-wide, or
BHCP-versus-prose claim. Complete frozen inputs, executable identities, metrics,
and judge records are in [`CONTROLLER.md`](CONTROLLER.md); raw model-service events
are not committed.
