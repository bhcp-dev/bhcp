# Positive registered-adapter study

The frozen twelve-session study completed with `12` included sessions and `0` infrastructure exclusions. Every frozen session remains in `RESULTS.txt`; no result was replaced.

- Positive registry use: **0/12** (two-sided 95% Clopper–Pearson `0.0000..0.2646`).
- In-session acceptance: **0/12** (two-sided 95% Clopper–Pearson `0.0000..0.2646`).
- Usage: 1961148 input, 1765248 cached input, 40755 output, 31916 reasoning tokens; 15.088 model-minutes.
- Incremental pay-as-you-go spend authority and observed spend: **USD 0**.

| Task | Seed | Positive use | Registered accepted | Independent accepted | In-session accepted | Claim | Calibrated | Excluded |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| atomic-batch | seed-01 | no | no | no | no | no | yes | no |
| atomic-batch | seed-02 | no | no | no | no | no | yes | no |
| atomic-batch | seed-03 | no | no | no | no | no | yes | no |
| tenant-policy | seed-01 | no | no | no | no | no | yes | no |
| tenant-policy | seed-02 | no | no | no | no | no | yes | no |
| tenant-policy | seed-03 | no | no | no | no | no | yes | no |
| contextual-policy | seed-01 | no | no | no | no | no | yes | no |
| contextual-policy | seed-02 | no | no | no | no | no | yes | no |
| contextual-policy | seed-03 | no | no | no | no | no | yes | no |
| in-session-evidence | seed-01 | no | no | no | no | no | yes | no |
| in-session-evidence | seed-02 | no | no | no | no | no | yes | no |
| in-session-evidence | seed-03 | no | no | no | no | no | yes | no |

## By-task estimates

| Task | Included | Positive use (95% CI) | In-session accepted (95% CI) |
| --- | ---: | --- | --- |
| atomic-batch | 3 | 0/3 (0.0000..0.7076) | 0/3 (0.0000..0.7076) |
| tenant-policy | 3 | 0/3 (0.0000..0.7076) | 0/3 (0.0000..0.7076) |
| contextual-policy | 3 | 0/3 (0.0000..0.7076) | 0/3 (0.0000..0.7076) |
| in-session-evidence | 3 | 0/3 (0.0000..0.7076) | 0/3 (0.0000..0.7076) |

## Resource distributions

Medians and interquartile ranges use Tukey hinges over included sessions with closed usage records.

| Measure | Median | IQR |
| --- | ---: | --- |
| Input tokens | 151585.0 | 136157.0..181137.0 |
| Cached-input tokens | 136896.0 | 123008.0..162560.0 |
| Output tokens | 3135.0 | 2935.0..4143.0 |
| Reasoning tokens | 2404.0 | 2320.0..3389.0 |
| Model wall milliseconds | 64645.5 | 62828.5..81258.0 |

These are descriptive estimates for the four frozen repository fixtures under one pinned model. They do not establish a population rate, model-wide effect, developer-productivity effect, or general BHCP advantage. Null, unfavorable, and incomplete outcomes remain part of the result.
