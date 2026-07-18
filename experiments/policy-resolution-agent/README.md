# Ambiguous policy-resolution agent experiment

This fixture tests a different claim from the minimal ledger experiment. The prose
ticket deliberately uses realistic but underspecified phrases such as "most
applicable," "conservative," and "stable." The BHCP arm receives the canonical
interpretation of those phrases as explicit obligations.

This is therefore an **intent-disambiguation experiment**, not an equal-information
comparison of two surface syntaxes. A BHCP advantage here would mean that preserving
canonical intent helps a coding agent avoid reasonable but policy-invalid guesses.

## Canonical resolution semantics

A rule is eligible only when it is enabled, belongs to the requested tenant, and
each of its subject, action, and resource patterns is either `*` or an exact match.
Eligible rules are ordered by this strict precedence ladder:

1. more exact patterns across subject, action, and resource;
2. greater numeric priority;
3. `Deny` over `Allow`;
4. lexicographically smaller rule ID.

If no rule is eligible, the decision is `Deny` with no selected rule. Rule insertion
order has no semantic meaning.

## Pinned buggy condition

The dependency-free Rust subject passes all visible tests. Its selector ignores the
tenant during matching and chooses only by numeric priority, using insertion order
implicitly for ties. The withheld oracle exposes tenant leakage and every missing
precedence layer while retaining an independent passing priority invariant.

Verify the pinned condition from this directory:

```sh
cargo test --offline --manifest-path subject/Cargo.toml
cargo test --offline --manifest-path oracle/Cargo.toml
```

The subject must pass. The oracle must fail five defect-revealing tests and pass two
independent invariants: default denial and equal-specificity priority.

## Trial protocol

Use the isolation, model pinning, oracle withholding, evidence capture, and
post-session policy checks described by the minimal experiment. The baseline arm
receives [`TASK.md`](TASK.md); the BHCP arm receives the same ticket plus
[`contract.bhcp`](contract.bhcp) and [`contract.semantic-id`](contract.semantic-id).
Only `src/lib.rs` may change. The first paired run should pin both arms to
`gpt-5.4-mini` with medium reasoning, matching Pilot 002's explicit model control.

## Recorded trial

- [`results/pilot-003/`](results/pilot-003/) — prose, raw BHCP, and BHCP plus the
  interpretation skill all produced accepted patches; raw BHCP was the most
  efficient arm, while the skill arm correctly withheld an unverified success claim.
- [`results/pilot-004/`](results/pilot-004/) — a refined prose ticket produced a
  visible-test-passing but oracle-rejected priority-first patch; both BHCP arms
  preserved the canonical precedence ladder, and the merged skill again withheld
  an unverified success claim.
