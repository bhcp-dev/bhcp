# Contextual policy-resolution agent experiment

This fixture extends the tenant-policy benchmark with an ordered specificity
lattice. The prose ticket uses realistic terms such as "closest contextual fit"
and "conservative" without disclosing the normative ordering. The canonical BHCP
contract makes that ordering explicit: resource specificity, subject specificity,
action specificity, priority, denial, then rule ID.

The dependency-free Rust subject counts exact fields instead of respecting their
ordered policy meaning, ignores tenant ownership, and relies on insertion order for
complete ties. It passes every visible test. The withheld oracle exercises ten
independent semantic boundaries.

This remains an intent-disambiguation experiment. It is deliberately
information-asymmetric: the BHCP arms receive canonical intent that prose alone
does not contain.

## Trial protocol

Each arm starts in an isolated copy of `subject/`. Only `src/lib.rs` may change.
The prose arm receives `TASK.md`; the raw-BHCP arm additionally receives
`contract.bhcp`, `contract.semantic-id`, and a compiled `bhcp` command; the skill
arm also receives the pinned `interpret-bhcp-contract` skill.

Run the visible checks during the session. Copy the withheld oracle only after the
agent stops, then run its ten invariants plus formatting, Clippy, the public tests,
`git diff --check`, and the one-file/dependency policy.

The implemented safe-Rust [experiment controller](../CONTROLLER.md) freezes and
enforces this arm order, input identity, oracle-withholding point, bounded result
protocol, and symmetric offline judge sequence for subsequent runs.

## Recorded trial

- [`results/pilot-006/`](results/pilot-006/) — raw BHCP and prose passed all ten
  invariants; the primary optimized-skill run used the fewest tokens but failed two
  ordered-specificity invariants, while a latest-main follow-up passed 10/10 with
  substantially higher token intake.
- Multi-seed runs 001 and 002 are retained as invalid infrastructure records after
  independent review found that host reads were not confined. Run 003 established
  the read boundary but was excluded before model launch because temporary
  metadata traversal was too narrow. Run 004 is
  [preregistered](results/multiseed-004-registration.md) after a real Codex smoke
  request completed through the corrected boundary.
