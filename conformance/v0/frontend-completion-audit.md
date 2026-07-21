# Practical v0 front-end completion audit

This audit closes the completion-manifest source-to-governed-IR boundary. It does
not claim completion of graph construction, planning, execution, evidence assembly,
the SDK, the CLI, or certification.

## Executable evidence

`completion-manifest.txt` contains one `frontend` record for every S4-S9 scenario
owned by the practical front end. Each record names an exact Rust test function and
an exact checked-in artifact. The completion-contract validator rejects an omitted,
duplicate, unknown, unsafe, missing, or stale test/artifact target.

The canonical reference program and its substantially remapped `goal` → `intent`
form now compile, together with the derived extension, to byte-identical semantic IR
and the same semantic identity. Their canonical AST identities remain different
because selected profile, source bytes, and spans are artifact provenance rather
than program meaning. Both AST and IR validate before this comparison is accepted.

## Contradictions resolved

- `§resource` and `§state` declarations now enter typed goal input while retaining
  their authored fact kinds, so ownership, policy, and effect-resource provenance
  remain inspectable.
- Source `Result<T,E>` reaches the IR's canonical `Err | Ok` variant without losing
  pure-call type compatibility.
- Pure source selection and exhaustive matching resolve nominal and handled record
  types, including the reference program's result obligations.
- Gate conditions may call retained total-pure definitions, and scalar derived
  extension inputs remain valid typed kernel edges.
- Child effects on resource/state facts project back to the exact parent binding.
- The reference walk uses a checked decreasing recursive gate; the runtime-only
  collection quantifier was removed because S8 requires finite expansion before IR.
- Nested gate and chain presentation was factored into ordinary helper goals so the
  retained kernel remains flat. Goal outputs were reconciled with the kernel rule
  that composition edges carry each child's complete typed output.

The reviewed program contract pins the reconciled definitions, facts, calls,
transfer modes, clauses, recursion evidence, and SHA3-512 source identities.

## Contradiction scan coverage

The audit compared the normative S4-S9 requirements in `SEMANTICS.md` with the v0
CDDL roots, checked-in conformance catalog and prior phase reports, feature and
completion manifests, parser/checker/lowering code and comments, exact test
inventory, README, VISION, live milestone-7 issues, and wiki revision
`85cb604ef6b6579dbad17e2cb897eeaed9b68a3c`. The scan corrected the stale README
statement left by #105 that still described source-defined function/predicate
elaboration as future work. No CDDL shape or prior phase result required rewriting:
they already distinguish accepted semantic IR from graph, planning, runtime, and
evidence roots. Live issues and the wiki still describe #112 as in progress until its
reviewed head merges; their downstream claims remain correctly blocked and are part
of the required post-merge consistency update.

## Bounded maturity statement

The practical completion ledger and frozen reference subject have no unsupported
source-to-IR row. The full S7 grammar still parses additional future execution forms
such as generic goals, executable cases, standalone call statements, and unexpanded
finite or nested composition. Those forms retain stable pre-IR diagnostics until a
roadmap issue explicitly adds their execution semantics; they are not silently
counted as completed by this audit. Runtime-only quantification remains a normative
rejection rather than an implementation gap.

Post-IR work remains visible in the completion manifest: state-graph completion,
graph audit, planning, execution, evidence, SDK/CLI, end-to-end conformance, security,
and final completion certification.
