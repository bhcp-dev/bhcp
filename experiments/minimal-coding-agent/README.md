# Minimal coding-agent experiment

This fixture is the first controlled subject for comparing an ordinary coding-agent
task with the same task governed by a BHCP contract. It is intentionally small
enough to audit completely and realistic enough that `cargo test` is not sufficient
evidence of success.

## Experiment claim

The experiment does **not** initially ask whether BHCP makes an agent write better
Rust. It asks whether BHCP makes a success claim more precise and independently
checkable when the visible repository tests are incomplete.

Both arms receive the exact requirements in [`TASK.md`](TASK.md), the same checkout,
model, tools, and budget:

- the baseline arm receives the Markdown task;
- the BHCP arm receives the same human projection plus the compiled semantic
  contract in [`contract.bhcp`](contract.bhcp).

The pinned contract identity is checked into
[`contract.semantic-id`](contract.semantic-id), so every trial can identify the
exact machine contract independently of comments or presentation labels.

The controller withholds [`oracle/`](oracle/) from the coding agent in both arms and
runs it only after the agent has stopped. This prevents either arm from optimizing
directly for hidden test examples. The oracle contains no unstated product
requirements: every invariant it checks appears in the task and contract.

## Subject condition

[`subject/`](subject/) is a dependency-free Rust ledger. Its public tests cover
successful batches, exact replay, and errors before mutation. They all pass in the
pinned buggy state. Two defects remain:

1. a failure after an earlier transfer can leave a partial batch committed; and
2. reuse of a request ID with a different payload is accepted as an exact replay.

Destination and aggregate overflow create additional partial-commit boundaries.
These are variations of the stated atomicity invariant, not separate surprise
requirements.

Verify the pinned condition from this directory:

```sh
cargo test --offline --manifest-path subject/Cargo.toml
cargo test --offline --manifest-path oracle/Cargo.toml
```

The first command must pass. The second must report five defect-revealing failures
and one independent successful-batch invariant in the unfixed fixture. The BHCP
repository test suite locks those expectations so an accidental fixture repair or
oracle weakening is visible.

## Trial protocol

For each trial, the controller should:

1. create a fresh scratch directory and copy only `subject/` into it;
2. place `TASK.md` in the agent-visible subject directory;
3. record the subject-tree digest, contract semantic ID, model, prompt, limits,
   elapsed time, tool calls, and token counts;
4. let the agent stop without revealing the oracle;
5. record the agent's claimed result and evidence;
6. copy `oracle/` beside the modified subject and run public tests, formatting,
   Clippy, the invariant oracle, and the one-file change policy; and
7. preserve the patch and verifier outputs as the trial evidence bundle.

An accepted result must satisfy every verifier. Passing public tests alone is not an
accepted result.

## Deliberate boundary

The current BHCP executable slice compiles and hashes the contract but does not yet
dispatch verifier bindings or construct an evidence bundle. Until that runtime
boundary exists, the controller performs the verifier commands. This fixture is
therefore the subject and oracle for the experiment, not a claim that the complete
experiment runner already exists.
