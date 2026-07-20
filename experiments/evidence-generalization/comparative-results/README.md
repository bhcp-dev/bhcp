# Paired BHCP-contract versus prose study

The frozen comparative study retained `24` session records with `0` infrastructure exclusions. No arm was replaced. Paired estimates use only blocks with two completed, non-excluded arms.

- Arm acceptance: **prose-control 0/12; bhcp-contract 0/12**.
- Independent all-judge acceptance: **paired risk difference +0.0000** (BHCP minus prose; 12 included blocks; discordants BHCP-only `0`, prose-only `0`; two-sided exact McNemar `p=1.000000`).
- Arm claim calibration: **prose-control 12/12; bhcp-contract 12/12**.
- Exact claim calibration: **paired risk difference +0.0000** (12 included blocks; discordants BHCP-only `0`, prose-only `0`; two-sided exact McNemar `p=1.000000`).
- Usage: 9139153 input, 7947136 cached input, 99550 output, 79708 reasoning tokens; 35.511 model-minutes.
- Incremental pay-as-you-go spend authority and observed spend: **USD 0**.

| Task | Seed | Position | Arm | Accepted | Claim | Calibrated | Excluded | Failure category |
| --- | --- | ---: | --- | --- | --- | --- | --- | --- |
| atomic-batch | seed-01 | 1 | prose-control | no | no | yes | no | verification:oracle |
| atomic-batch | seed-01 | 2 | bhcp-contract | no | no | yes | no | verification:oracle |
| atomic-batch | seed-02 | 1 | bhcp-contract | no | no | yes | no | verification:oracle |
| atomic-batch | seed-02 | 2 | prose-control | no | no | yes | no | verification:oracle |
| atomic-batch | seed-03 | 1 | prose-control | no | no | yes | no | verification:oracle |
| atomic-batch | seed-03 | 2 | bhcp-contract | no | no | yes | no | verification:oracle |
| atomic-batch | seed-04 | 1 | bhcp-contract | no | no | yes | no | verification:oracle |
| atomic-batch | seed-04 | 2 | prose-control | no | no | yes | no | verification:oracle |
| tenant-policy | seed-01 | 1 | bhcp-contract | no | no | yes | no | verification:oracle |
| tenant-policy | seed-01 | 2 | prose-control | no | no | yes | no | verification:oracle |
| tenant-policy | seed-02 | 1 | prose-control | no | no | yes | no | verification:oracle |
| tenant-policy | seed-02 | 2 | bhcp-contract | no | no | yes | no | verification:oracle |
| tenant-policy | seed-03 | 1 | bhcp-contract | no | no | yes | no | verification:oracle |
| tenant-policy | seed-03 | 2 | prose-control | no | no | yes | no | verification:oracle |
| tenant-policy | seed-04 | 1 | prose-control | no | no | yes | no | verification:oracle |
| tenant-policy | seed-04 | 2 | bhcp-contract | no | no | yes | no | verification:oracle |
| contextual-policy | seed-01 | 1 | prose-control | no | no | yes | no | verification:oracle |
| contextual-policy | seed-01 | 2 | bhcp-contract | no | no | yes | no | verification:oracle |
| contextual-policy | seed-02 | 1 | bhcp-contract | no | no | yes | no | verification:oracle |
| contextual-policy | seed-02 | 2 | prose-control | no | no | yes | no | verification:oracle |
| contextual-policy | seed-03 | 1 | prose-control | no | no | yes | no | verification:oracle |
| contextual-policy | seed-03 | 2 | bhcp-contract | no | no | yes | no | verification:oracle |
| contextual-policy | seed-04 | 1 | bhcp-contract | no | no | yes | no | verification:oracle |
| contextual-policy | seed-04 | 2 | prose-control | no | no | yes | no | verification:oracle |

## Task-level paired estimates

| Task | Included blocks | Acceptance risk difference | Acceptance discordants (BHCP/prose) | Exact McNemar p | Calibration risk difference | Calibration discordants (BHCP/prose) | Exact McNemar p |
| --- | ---: | ---: | --- | ---: | ---: | --- | ---: |
| atomic-batch | 4 | +0.0000 | 0/0 | 1.000000 | +0.0000 | 0/0 | 1.000000 |
| tenant-policy | 4 | +0.0000 | 0/0 | 1.000000 | +0.0000 | 0/0 | 1.000000 |
| contextual-policy | 4 | +0.0000 | 0/0 | 1.000000 | +0.0000 | 0/0 | 1.000000 |

## Resource distributions by arm

Medians and interquartile ranges use Tukey hinges over non-excluded sessions with closed usage records.

| Arm | Measure | Median | IQR |
| --- | --- | ---: | --- |
| prose-control | Input tokens | 356652.0 | 208667.5..384075.5 |
| prose-control | Cached-input tokens | 302208.0 | 176704.0..337408.0 |
| prose-control | Output tokens | 3463.5 | 3281.0..4149.0 |
| prose-control | Reasoning tokens | 2759.5 | 2531.5..3263.0 |
| prose-control | Completed commands | 0.0 | 0.0..0.0 |
| prose-control | Model wall milliseconds | 76978.5 | 69610.5..97500.0 |
| bhcp-contract | Input tokens | 411111.5 | 362227.5..490097.0 |
| bhcp-contract | Cached-input tokens | 355904.0 | 317952.0..435456.0 |
| bhcp-contract | Output tokens | 4300.5 | 3706.0..4677.0 |
| bhcp-contract | Reasoning tokens | 3446.0 | 2898.0..3772.0 |
| bhcp-contract | Completed commands | 0.0 | 0.0..0.0 |
| bhcp-contract | Model wall milliseconds | 92783.0 | 77761.0..104195.5 |

## Execution audit

All 24 controller records report zero completed commands, identical before/after
subject digests, and an empty candidate patch. In every arm the frozen starter
passed format, Clippy, public tests, and the change-policy judge, then failed only
the withheld oracle. Every model therefore made the calibrated negative claim.
The zero paired difference is a retained failure of both representations to induce
a repair in this setup; it is not evidence that the representations are equivalent.

These are descriptive paired estimates for three frozen repository fixtures under one pinned model. `alpha=descriptive-only`: the exact McNemar values are uncertainty evidence, not a confirmatory threshold. The study cannot establish a population effect, causal language effect, model-wide effect, developer-productivity effect, or general BHCP advantage. Null, unfavorable, incomplete, and excluded outcomes remain visible.
