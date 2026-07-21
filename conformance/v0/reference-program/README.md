# Practical v0 reference program

This directory freezes the milestone-7 end-to-end acceptance subject. It is a
normative target. The practical front end now compiles its canonical and remapped
program sources, together with the derived extension, to equivalent governed
semantic IR. Downstream graph, planner, runtime, evidence, SDK, CLI, and end-to-end
claims remain assigned to later roadmap issues.

The source inventory deliberately crosses every practical v0 boundary:

- `program.bhcp` uses named/refined types, total functions with selection and
  exhaustive matching, a verifier-backed predicate, affine/linear ownership and
  moves, effects, ordinary helper goals for `all`/`gate`/`chain`, a bounded decreasing
  recursive walk, budgets, preferences, and exact evidence labels.
- `program.words.bhcp` is the same source structure under the checked-in
  `goal` → `intent` profile mapping. `program-contract.txt` is the reviewed typed
  projection that closes definitions, facts, calls, transfer modes, and consumption.
- `policy.bhcp` and `waiver.bhcp` require monotonic layered governance and one
  time-bounded exact weakening decision; the current policy parser/composer and
  typed waiver projection validate their already-implemented boundaries.
- `syntax.bhcp` and `profile.bhcp` select a closed custom presentation with a
  infer-strict type and two-layer policy boundary; their typed diagnostic projections
  validate against the existing syntax/profile models.
- `extension.bhcp` is invoked by the program, names a kernel-signature reducer and
  a `bhcp/meta.network-shape@0` lowerer, and must disappear completely into checked
  core IR. Its five descriptor rule references bind reviewed, hashed rule files.
- `registry.txt` and the planner/execution inputs connect every source and projection,
  fix budgets, capabilities, waiver decision time, cancellation, and output shape.
  `policy-evidence-registry.txt` explicitly binds both policy evidence demands to
  their registered verifier producers. `expected-obligations.txt` contains only
  valid obligation states; the separate
  `outcome-matrix.txt` preserves planning refusal, completed verdict, cancellation,
  stale-evidence, and operational-fault distinctions.

The source-to-IR portion now means deterministic source and artifact identities and
checked governed semantic IR. Full reference-program completion additionally means
mutually consistent governed graphs, a valid explained plan, capability-bounded
execution, and deterministic evidence through both the public Rust SDK and CLI. It
does not require full theorem proving,
unrestricted macros or grammar plugins, comprehensive temporal/reactive logic, or
universal workflow synthesis.
