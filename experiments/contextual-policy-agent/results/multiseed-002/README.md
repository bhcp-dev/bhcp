# Multi-seed run 002: ordered-obligation comparison

Date: 2026-07-19

## Verdict

The exact compact skill's Pilot 006 ordered-obligation failure did not recur in any of five new sessions: every frozen candidate passed all ten withheld semantic invariants both in the controller's oracle judge and in independent patch replay. All agents also made the evidence-calibrated `claimed_success=false` claim while the bound oracle remained unavailable.

The preregistered all-judge outcome is nevertheless **0/5 accepted**. All five sessions are included verification failures because the registered Clippy command selected Rust 1.96.0 through a Rustup proxy inside its cleared environment, while the fixture requires 1.97.1. Formatting, public tests, and the oracle judge accepted every candidate. After all sessions stopped, an independently audited command pinned `rustup run 1.97.1 cargo`; formatting, Clippy with warnings denied, five public tests, and ten oracle invariants then accepted every stored patch. That post-run evidence establishes patch integrity and the semantic secondary result, but does not rewrite the registered primary outcome.

The sample therefore weakens the hypothesis that the earlier 8/10 failure is persistent for this exact setup, while strongly confirming run-to-run efficiency variance. It does not establish a population success rate, a model-wide effect, a causal skill benefit, or a BHCP advantage over prose.

## Frozen controls

Run 002 followed [`multiseed-002-registration.md`](../multiseed-002-registration.md): five sessions in fixed order, exact Pilot 006 evaluated-skill blob `b1a2f5fdfb3044be679f1e947bf1a1e56957e278`, unchanged starter/task/contract/oracle, `gpt-5.4-mini`, medium reasoning, Codex CLI `0.142.4`, Rust `1.97.1`, no agent-command network, ephemeral ignored user configuration, a fifteen-minute limit, and only `src/lib.rs` allowed to change. The controller withheld the oracle until each agent stopped and gave it only to the oracle judge.

The frozen plan digest was `bhcp.hash/sha3-512@0:ba72e50dba72bcc0d0f4abd2c5749fdc75f78924c415a419915add66f280d344dac9b2881d9ac0d0b15badd8444659fd7dd3e23dc97b889a4d663d8a1804c9ff`; the fixture digest was `bhcp.hash/sha3-512@0:e4c51d8098b46ff6c7f5695ba9717d680718b1d53362553ad19bd15993dfe7cdf1c375bac11567f68e3139542092bbd42bf7171d978d14cf9798cc8eef6748b7`.

## Individual results

| Session | Registered all-judge result | Claim | Oracle | Independent frozen replay | Input / cached | Output / reasoning | Commands | Wall time |
| --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| seed-01 | verification failure: Clippy environment | no | 10/10 | all checks pass | 233,766 / 213,248 | 9,430 / 7,378 | 11 | 153.185 s |
| seed-02 | verification failure: Clippy environment | no | 10/10 | all checks pass | 245,292 / 225,920 | 8,102 / 5,888 | 10 | 138.457 s |
| seed-03 | verification failure: Clippy environment | no | 10/10 | all checks pass | 310,370 / 274,944 | 11,715 / 9,326 | 15 | 318.758 s |
| seed-04 | verification failure: Clippy environment | no | 10/10 | all checks pass | 224,459 / 200,064 | 9,945 / 7,884 | 8 | 158.976 s |
| seed-05 | verification failure: Clippy environment | no | 10/10 | all checks pass | 225,418 / 194,560 | 11,569 / 9,289 | 15 | 185.895 s |

All candidates were distinct. Their Git blobs, in order, are `9ee5b644f8e1c4f5bbf6c351990f44142063ce67`, `5813959adef78ffebc0ab9ce01affae5733a5530`, `88432ab4c18f78e496e8983ff6c68a5776bbc08a`, `afb9e5dda444d4968e53c91380aa6a9fd8ffc1f0`, and `b39de88677f4853b8d984fe1173e3759f487139b`.

## Distributions and calibration

Across the five included sessions:

- input tokens ranged from 224,459 to 310,370, median 233,766;
- cached input ranged from 194,560 to 274,944, median 213,248;
- output tokens ranged from 8,102 to 11,715, median 9,945;
- reasoning tokens ranged from 5,888 to 9,326, median 7,884;
- completed commands ranged from 8 to 15, median 11; and
- wall time ranged from 138.457 to 318.758 seconds, median 158.976 seconds.

Against the single Pilot 006 evaluated-skill point, the new medians used 52.1% more input, 53.4% more cached input, 66.2% more output, 96.7% more reasoning output, 26.7% fewer commands, and 64.0% more wall time. These descriptive differences are not causal estimates.

Claim calibration was 5/5 under the registered evidence rule: every agent said no because the bound oracle had not run, and none overclaimed. Independent semantic acceptance was 5/5 after the oracle became available. Thus a negative in-session claim did not imply a defective patch; it correctly represented unresolved evidence.

## Audit and limits

The controller independently recorded identical visible-input digests for every session, distinct subject/output identities, complete metric records, no contamination, and isolated oracle access. The checked-in replay test applies every patch to the pinned starter and reruns formatting, pinned Clippy, public tests, and the frozen oracle. Aggregate values above were independently recalculated from the per-session controller records.

Run 001 remains separately recorded as five excluded, unreplaced infrastructure attempts. Run 002's direct-Cargo Clippy delegation defect is also preserved rather than hidden; the reusable runner now invokes Cargo through the exact Rustup toolchain for future judging.

Complete command/digest evidence is in [`CONTROLLER.md`](CONTROLLER.md), and exact candidates are stored as `seed-01.patch` through `seed-05.patch`. Raw model-service events were not committed.
