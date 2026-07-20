# Practical v0 reference program

This directory freezes the milestone-7 end-to-end acceptance subject. It is a
normative target, not a claim that the current partial front end can execute it.
Until issues #100–#132 close, the current compiler must reject `program.bhcp` at
the first unsupported definition instead of emitting a partial AST or IR.

The source inventory deliberately crosses every practical v0 boundary:

- `program.bhcp` uses named/refined types, a total function, a verifier-backed
  predicate, affine/linear ownership and moves, effects, nested `all`/`chain`/`gate`
  goals, a finite recursive walk, budgets, preferences, and exact evidence labels.
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

Passing the reference program eventually means deterministic source and artifact
identities, checked semantic IR, mutually consistent governed graphs, a valid
explained plan, capability-bounded execution, and deterministic evidence through
both the public Rust SDK and CLI. It does not require full theorem proving,
unrestricted macros or grammar plugins, comprehensive temporal/reactive logic, or
universal workflow synthesis.
