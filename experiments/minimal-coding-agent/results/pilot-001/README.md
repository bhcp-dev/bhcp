# Pilot 001: prose task versus BHCP contract

Date: 2026-07-18

This is the first paired run of the minimal coding-agent experiment. It is a
single-pair pilot for validating the protocol, not evidence of a population-level
advantage.

## Verdict

Both arms claimed success and both were accepted by every independent verifier.
The result is a correctness tie. The two repairs are functionally equivalent and
differ only in whether the staged balance map is committed immediately before or
immediately after recording the successful request.

This run therefore does **not** demonstrate that BHCP caused a better Rust patch.
It does demonstrate that the BHCP arm carried a pinned semantic contract and
explicit verifier-to-obligation mappings through the same task. The experiment
runner does not yet execute those bindings automatically, so the controller ran
the equivalent checks for both arms.

## Frozen inputs

Both arms started from fresh, isolated repositories with the same subject source:

- `src/lib.rs` Git blob: `3f5de256365a238bbef3bf72ea67e6dc5e9c0ffc`
- task: [`../../TASK.md`](../../TASK.md)
- public tests: 4
- withheld oracle tests: 6
- network access: prohibited by the prompt
- permitted edit: `src/lib.rs` only

The baseline arm received the task. The BHCP arm received the identical task plus
[`../../contract.bhcp`](../../contract.bhcp) and its semantic ID:

```text
bhcp.hash/sha3-512@0:b7a1092d7143aaf79d5d51ee08e4c884be46df5ca87012573d197bf5f06f8b8688860b0b80b84bc95fad79c764c37e75877c3dbfb06bcbb4590aaf71dbcdd776
```

The oracle was absent from both working directories until each agent stopped.

## Agent configuration

Both arms used Codex CLI `0.142.4`, ephemeral sessions, ignored user configuration,
workspace-write isolation, and `model_reasoning_effort="medium"`. The baseline ran
first. The CLI used its default model, but the event stream did not identify that
model; consequently the exact model version is not reproducible from this bundle.

The shared baseline prompt was:

```text
Work autonomously on the coding task in TASK.md. Meet every stated requirement; an independent oracle is withheld until you stop. Do not use the network. Edit only src/lib.rs and preserve the public API. Run all visible formatting, Clippy, and test checks before reporting success. Do not ask questions. In your final response, state whether you claim success, name every changed file, and list the checks you actually ran.
```

The BHCP arm received this additional paragraph:

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
| Patch size | +20/-12 | +20/-12 |
| Completed shell commands | 8 | 10 |
| Input tokens | 185,637 | 227,172 |
| Cached input tokens | 161,280 | 180,480 |
| Output tokens | 2,771 | 2,545 |
| Reasoning output tokens | 874 | 653 |
| Approximate session wall time | 88.9 s | 80.7 s |
| Resulting `src/lib.rs` Git blob | `0cdd90ed90bf8d3da2c346b4e006b2646afebc4c` | `959f8eebf18e3489ea86ff20baaf50c3643902d9` |

The BHCP arm used 41,535 more reported input tokens (22.4%). Its lower output-token
and wall-time figures are not meaningful evidence of efficiency in a sequential
single-pair run.

The exact candidate changes are preserved in [`baseline.patch`](baseline.patch)
and [`bhcp.patch`](bhcp.patch).

## Independent acceptance

After each session stopped, the controller copied the same withheld oracle into a
separate judge checkout and ran:

1. `cargo fmt --check`;
2. `cargo clippy --offline --all-targets -- -D warnings`;
3. `cargo test --offline --all-targets` for the public crate;
4. `cargo test --offline --manifest-path oracle/Cargo.toml`;
5. `git diff --check`; and
6. a repository diff policy requiring exactly one modified file, `src/lib.rs`.

For both arms, all checks exited successfully. The oracle passed these six named
invariants:

- destination overflow does not debit the source;
- later failure does not commit earlier transfers;
- conflicting request-ID reuse is rejected;
- a failed request ID can be retried against the original state;
- aggregate receipt overflow rolls back the whole batch; and
- successful batches conserve balance and report the checked sum.

## Interpretation and next boundary

This task produced a ceiling effect: the prose already states every important
invariant precisely, the repair is small, and the hidden oracle is equally applied
to both arms. One successful pair cannot establish an objective BHCP advantage.

Before scaling the experiment, the runner should gain process-backed verifier
adapters so the BHCP arm emits a deterministic evidence bundle from the contract.
A proper comparison should then pin and record an explicit model version,
counterbalance run order, repeat each arm, and include harder fixtures where
multiple plausible patches satisfy visible tests but differ on stated obligations.
