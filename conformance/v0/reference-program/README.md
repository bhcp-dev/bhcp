# Practical v0 reference program

This directory freezes the milestone-7 end-to-end acceptance subject. It is a
normative target, not a claim that the current partial front end can execute it.
Until issues #100–#132 close, the current compiler must reject `program.bhcp` at
the first unsupported definition instead of emitting a partial AST or IR.

The source inventory deliberately crosses every practical v0 boundary:

- `program.bhcp` uses named/refined types, a total function, a verifier-backed
  predicate, owned and borrowed values, effects, nested `all`/`chain`/`gate`
  goals, a finite recursive walk, budgets, preferences, and exact evidence labels.
- `policy.bhcp` and `waiver.bhcp` require monotonic layered governance and one
  scoped, time-bounded weakening decision.
- `syntax.bhcp` and `profile.bhcp` select a closed custom presentation with a
  strict type and repository-policy boundary.
- `extension.bhcp` is derived and must disappear completely into checked core IR.
- the planner and execution inputs fix budgets, capabilities, cancellation, and
  output shape; `expected-obligations.txt` fixes per-obligation result categories.

Passing the reference program eventually means deterministic source and artifact
identities, checked semantic IR, mutually consistent governed graphs, a valid
explained plan, capability-bounded execution, and deterministic evidence through
both the public Rust SDK and CLI. It does not require full theorem proving,
unrestricted macros or grammar plugins, comprehensive temporal/reactive logic, or
universal workflow synthesis.
