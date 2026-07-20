# Positive registered-adapter study registration

This issue-local closure of the [parent preregistration](preregistration.md) freezes
the positive-use study before its first model turn. The normative machine record is
[`positive-use-registration.txt`](positive-use-registration.txt). It identifies the
runner, adapter, prompt, canonical withheld answers, prepared fixtures, executable
surface, controller plans, and all twelve sessions. A clean Git checkout containing
these inputs is required at launch.

Each of the four registered repository fixtures runs once under each of `seed-01`,
`seed-02`, and `seed-03`, for twelve sessions total. The labels identify isolated
sessions rather than a hosted deterministic random-seed facility. Runs are
sequential (`concurrency=1`), have a 15-minute per-session timeout, share a
180-model-minute issue budget, and authorize USD 0 incremental pay-as-you-go spend.
The billing preflight rejects API keys, API-base overrides, stored API keys, and any
authentication mode other than the existing ChatGPT entitlement authorized by the
reviewed #91 merge.

The registered adapters are deliberately conservative: they accept only the exact
task-specific source already approved by each deterministic public check and
withheld oracle. Other sources are rejected rather than guessed at. The model sees
the shared task, canonical BHCP contract and semantic ID, exact checked-in
`interpret-bhcp-contract` skill, a three-producer project registry, and the
canonical registry command. It cannot read the checked-in canonical answers or
withheld oracle. The independent controller replays every retained candidate using
format, Clippy, public, oracle, and change-policy judges in that order.

A completed model turn remains in the result with no replacement whether it edits
nothing, omits the registry, produces rejected, unresolved, or faulted evidence,
fails independent replay, or miscalibrates its final claim. Only preregistered
infrastructure failures are exclusions, and exclusions are never replaced.

Positive use requires a deterministic, parseable evidence bundle bound to the
session's exact candidate and containing all three registered adapter results.
In-session acceptance additionally requires every mandatory target to be discharged
and the independent controller to accept the same candidate. Overall and per-task
counts use two-sided 95% Clopper–Pearson intervals. Claim calibration is reported
without changing acceptance. Input, cached-input, output, reasoning-token, and model
wall-time distributions use medians and interquartile ranges calculated with
Tukey hinges. All inference remains descriptive over these four fixtures and this one
pinned model.
