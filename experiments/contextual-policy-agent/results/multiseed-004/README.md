# Multi-seed run 004: registered negative result

Date: 2026-07-19

Run 004 produced **0/5 accepted** candidates under the preregistered protocol.
Every model turn completed inside the read-confined boundary, returned
`claimed_success=false`, and left `src/lib.rs` unchanged. Formatting, Clippy with
warnings denied, and all five public tests accepted the unchanged starter; the
withheld oracle accepted 4/10 invariants and rejected the candidate. These are five
included semantic failures, not infrastructure exclusions, and none is replaced.
Equivalently, each candidate passed 4/10 oracle invariants.

## Frozen controls

The controller used the registered five-arm order, `gpt-5.4-mini` with medium
reasoning, Codex CLI 0.142.4, exact Rust 1.97.1 executables, the Pilot 006 evaluated
skill, fixed prompt and fixture identities, a fifteen-minute bound, and the
`workspace-write/no-network/read-confined` sandbox. Immediately before each model
launch, the operating-system profile successfully read the staged prompt and
rejected a read of the original oracle. The oracle was copied into a fresh judge
view only after the model turn stopped.

The frozen plan digest was
`bhcp.hash/sha3-512@0:4793c47dc2c92336369eb6573d1db010e0735f0a57f583d1c7ed41685eeb0190d060a2c1ac5c37306dd99198dadc9ecb1ce9b50957ff8722430e4b7323187ad5`;
the fixture digest was
`bhcp.hash/sha3-512@0:5f1c2fc32a57d9518f2bddf30ad24cf046969e57b2707eca97f538509f94778db9df1cc8dc02a378f0d8a5646a2af82df9599324e7486e67fd74907963298db8`.

## Individual results

| Session | Registered result | Claim | Oracle | Input / cached | Output / reasoning | Commands | Wall time |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| seed-01 | verification failure: oracle | no | 4/10 | 144,552 / 130,048 | 2,717 / 2,007 | 0 | 64.787 s |
| seed-02 | verification failure: oracle | no | 4/10 | 120,129 / 105,728 | 3,076 / 2,593 | 0 | 64.181 s |
| seed-03 | verification failure: oracle | no | 4/10 | 232,083 / 208,256 | 3,977 / 3,053 | 0 | 141.792 s |
| seed-04 | verification failure: oracle | no | 4/10 | 162,039 / 146,304 | 3,763 / 3,146 | 0 | 72.750 s |
| seed-05 | verification failure: oracle | no | 4/10 | 171,224 / 161,024 | 2,948 / 2,037 | 0 | 66.688 s |

The event stream recorded zero completed command-execution items in every turn.
The controller also recorded identical before/after semantic hashes for all five
subjects. The five empty patch files are therefore the exact identity-transform
candidates, not missing artifacts.

## Failures, distributions, and calibration

All five unchanged candidates failed the same six independent oracle invariants:

- tenant locality;
- resource specificity over other exact fields;
- subject specificity at equal resource scope;
- denial as the equal-policy tie breaker;
- the smaller rule ID as the final tie breaker; and
- insertion-order independence.

Across the five included sessions, input tokens ranged from 120,129 to 232,083,
median 162,039; cached input ranged from 105,728 to 208,256, median 146,304;
output ranged from 2,717 to 3,977, median 3,076; reasoning output ranged from 2,007
to 3,146, median 2,593; completed commands were always zero; and wall time ranged
from 64.181 to 141.792 seconds, median 66.688 seconds.

Against the single Pilot 006 evaluated-skill point, the run-004 medians used 5.4%
more input, 5.2% more cached input, 48.6% less output, 35.3% less reasoning output,
100% fewer completed commands, and 31.2% less wall time. These are descriptive
differences, not estimates of a skill or model effect.

Claim calibration was exact for this sample: 5/5 `claimed_success=false` records
matched 5/5 registered failures. This establishes only that the bounded claims
were appropriately conservative in these sessions.

## Replay and limits

The checked-in validator confirms all five patches are empty, all five controller
records preserve the starter identity, and the starter has its pinned Git blob.
The exact Rust 1.97.1 replay then reruns formatting, Clippy, public tests, and the
frozen oracle against that identity transform, reproducing the 4/10 result and the
same six failures.

Five observations on one fixture, model, skill, and toolchain do not support a
hypothesis test, confidence interval, population success rate, model-wide claim,
causal skill claim, or BHCP-versus-prose advantage. Runs 001 and 002 remain invalid
for oracle-withheld inference; run 003 remains five unreplaced infrastructure
exclusions. Complete per-arm inputs, executable identities, metrics, commands,
hashes, and judge records are in [`CONTROLLER.md`](CONTROLLER.md).
