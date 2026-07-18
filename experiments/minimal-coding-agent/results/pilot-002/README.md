# Pilot 002: pinned mini-model comparison

Date: 2026-07-18

This paired run repeats Pilot 001 with the smaller `gpt-5.4-mini` model pinned
explicitly for both arms. The arm order is reversed: BHCP ran first and the prose
baseline ran second.

## Verdict

Both arms claimed success and both passed every independent verifier. This is
another correctness tie. Reducing model capability did not make the current
fixture discriminate between prose and BHCP.

The BHCP arm inspected the pinned semantic ID and mapped the contract obligations
before editing. That traceability did not produce a better accepted patch on this
task. It used more commands, tokens, and wall time than the prose arm.

## Frozen inputs and model

Both arms started from isolated repositories with the same inputs:

- `src/lib.rs` Git blob: `3f5de256365a238bbef3bf72ea67e6dc5e9c0ffc`
- task: [`../../TASK.md`](../../TASK.md)
- public tests: 4
- withheld oracle tests: 6
- network access: prohibited by the prompt
- permitted edit: `src/lib.rs` only

The BHCP arm additionally received [`../../contract.bhcp`](../../contract.bhcp)
and this semantic ID:

```text
bhcp.hash/sha3-512@0:b7a1092d7143aaf79d5d51ee08e4c884be46df5ca87012573d197bf5f06f8b8688860b0b80b84bc95fad79c764c37e75877c3dbfb06bcbb4590aaf71dbcdd776
```

Each accepted session was launched with Codex CLI `0.142.4` and:

```text
--ephemeral --ignore-user-config --model gpt-5.4-mini
--sandbox workspace-write -c model_reasoning_effort="medium"
```

The model was therefore explicit at the request boundary rather than inferred from
a default. The structured event stream still did not echo the resolved model name.

The oracle was absent until both agents stopped. A preliminary BHCP launch received
an empty prompt because of a controller shell-variable mistake, made no changes,
and was discarded before the recorded session. It is not included in the metrics.

## Prompts

The shared prompt was:

```text
Work autonomously on the coding task in TASK.md. Meet every stated requirement; an independent oracle is withheld until you stop. Do not use the network. Edit only src/lib.rs and preserve the public API. Run all visible formatting, Clippy, and test checks before reporting success. Do not ask questions. In your final response, state whether you claim success, name every changed file, and list the checks you actually ran.
```

The BHCP arm additionally received:

```text
The canonical machine contract is contract.bhcp and its pinned identity is contract.semantic-id. Treat every targeted obligation and verifier binding in that contract as required for acceptance.
```

## Results

| Measure | Prose baseline | BHCP |
| --- | ---: | ---: |
| Claimed success | yes | yes |
| Public tests | 4/4 | 4/4 |
| Withheld invariant tests | 6/6 | 6/6 |
| Formatting | pass | pass |
| Clippy with warnings denied | pass | pass |
| One-file/dependency/API policy | pass | pass |
| Patch size | +17/-10 | +19/-12 |
| Completed shell commands | 14 | 28 |
| Failed intermediate shell commands | 4 | 5 |
| Input tokens | 127,109 | 267,385 |
| Cached input tokens | 112,000 | 246,400 |
| Output tokens | 3,732 | 6,446 |
| Reasoning output tokens | 1,364 | 2,486 |
| Approximate session wall time | 59.2 s | 106.0 s |
| Resulting `src/lib.rs` Git blob | `3fb213ad2a3ec69cdcd15999f7b46f7c25939dc4` | `bbd63f882fc2e7c6831c653a11ff6f609b4c68d3` |

The BHCP arm reported 140,276 more input tokens than the baseline, approximately
110%. Some of the extra work was stochastic recovery: its first patch application
missed the exact function signature, and both arms had to locate the Rust toolchain
outside the default `PATH`. This single pair cannot isolate contract overhead from
ordinary run variance.

The candidate changes are preserved in [`baseline.patch`](baseline.patch) and
[`bhcp.patch`](bhcp.patch).

## Independent acceptance

After both agents stopped, the controller copied the same withheld oracle into
separate judge checkouts and ran formatting, offline Clippy, all public tests, all
oracle tests, `git diff --check`, and the one-file change policy. The first judge
launch selected the host's Rust 1.96 toolchain and ran no tests; both judges were
then restarted symmetrically with `RUSTUP_TOOLCHAIN=1.97.1`.

Both candidates passed all six oracle invariants:

- destination overflow does not debit the source;
- later failure does not commit earlier transfers;
- conflicting request-ID reuse is rejected;
- a failed request ID can be retried against the original state;
- aggregate receipt overflow rolls back the whole batch; and
- successful batches conserve balance and report the checked sum.

## Comparison with Pilot 001

Across the two paired pilots, both arms are accepted in both runs. That is not four
independent task observations: the same fixture was repeated, so it remains a
protocol exercise with a strong ceiling effect.

The next useful experiment is not another repetition of this repair. It is a harder
fixture with several plausible visible-test-passing patches whose differences map
to explicit obligations. Process-backed verifier adapters should also replace the
manual controller checks so BHCP's claimed evidence advantage becomes executable.
