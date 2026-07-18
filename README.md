# BHCP

Beyond Human-Centric Programming (BHCP) is a semantic programming model in which
people declare outcomes, authority, limits, and required evidence while machines
discover acceptable executions.

This repository defines the normative v0 foundation and focused executable slices.
The Rust implementation accepts canonical clause goals and self-hosted `all`, emits
validated canonical AST and semantic IR, dispatches registered verifier bindings,
and emits deterministic evidence bundles. Canonical artifacts use deterministic
CBOR and algorithm-tagged semantic or artifact identities. It is not yet a complete
v0 parser, checker, planner, runtime, or SDK.

## Start here

- [VISION.md](VISION.md) is the short, aspirational description of the project and
  the product direction.
- [SEMANTICS.md](SEMANTICS.md) is the normative v0 language and platform contract.
  Implementations claiming v0 conformance must follow it.
- [`schemas/v0/`](schemas/v0/) is the machine-readable CDDL form of every v0
  platform artifact. Deterministic CBOR is the canonical wire representation.
- [`schemas/v0/examples/`](schemas/v0/examples/) contains CBOR diagnostic examples
  for every root document type.
- [`conformance/v0/`](conformance/v0/) is the normative scenario catalog that future
  implementations must turn into executable fixtures.

Normative terms such as **MUST**, **SHOULD**, and **MAY** are interpreted as in
[RFC 2119](https://www.rfc-editor.org/rfc/rfc2119) and
[RFC 8174](https://www.rfc-editor.org/rfc/rfc8174) when capitalized.

## Schema validation

The Rust schema harness checks the CDDL root inventory, parses and validates all 17
diagnostic fixtures, and verifies deterministic-CBOR round trips:

```sh
./scripts/validate-schemas
```

See [`schemas/v0/README.md`](schemas/v0/README.md) for the artifact inventory,
canonical encoding rules, and validation details.

## Executable slice

Install the pinned Rust toolchain, then run the CLI through Cargo:

```sh
mise install
cargo run -- parse conformance/v0/fixtures/canonical-simple.bhcp
cargo run -- lower conformance/v0/fixtures/canonical-simple.bhcp
cargo run -- inspect conformance/v0/fixtures/canonical-simple.bhcp
cargo run -- hash conformance/v0/fixtures/canonical-simple.bhcp
```

The implemented source boundary supports namespaced/versioned goals;
typed `§input` and `§output` facts; `§requires`, `§ensures`, and `§limit` Boolean
expressions; `§allows` and `§forbids` effect atoms; ranked `§prefer`; and `§verify`
bindings with optional explicit contract-label targets. Scalar literals, binding
references, parentheses, unary `!`/`-`, and the
checked Boolean, comparison, and `+` operators form the clause-expression subset.
Closed record field types, one top-level `§all` body, and equivalent explicit
`§compose using bhcp/prelude.all-reducer@0` source are also executable. Composition
children are currently zero-argument goal calls; nested compositions, project
functions, and every other reserved construct outside the slice are rejected with a
stable diagnostic rather than erased.

`bhcp.hash/sha3-512@0` is the default and only currently registered identity
algorithm, implemented in repository-owned Rust. It provides a roughly 256-bit
post-quantum preimage margin. [`bhcp-project.toml`](bhcp-project.toml) is the explicit
algorithm-agility boundary: projects may select another algorithm once the Rust
implementation registers it; unknown selections fail before parsing.

The crate uses the `cddl` 0.10.6 parser from cddl-rs to reject malformed RFC 8610
schemas and the pure-Rust RustCrypto `sha3` 0.12.0 crate for SHA3-512. The BHCP
compiler, deterministic CBOR codec, and fixture validator remain repository-owned
safe Rust; the repository contains no project-owned C, Ruby, or Node.js tooling. Run
every local acceptance check with:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo build --release
./scripts/validate-schemas
```

`cargo run --bin generate-fixtures` regenerates the checked-in AST and IR CBOR
artifacts for the canonical simple-goal and self-hosted `all` fixtures. The semantic
model defines the minimal `kernel-network`: total pure reducers return adjectival
`Pending | Concluded` reduction states over factored execution results.

[`prelude/v0/all.bhcp`](prelude/v0/all.bhcp) is parsed and checked as canonical BHCP
source. Its compile-time lowerer constructs an ID-free network shape through the
restricted metamodel, disappears, and leaves a monomorphized runtime reducer in
semantic IR. The generic Rust kernel implements only typed sealed-observation
queries, result construction, tag-to-child resolution, and derivation sealing; the
prelude source determines `all` precedence and selects its aggregation operations.
`§all` and explicit `§compose` produce byte-identical semantic IR and semantic IDs. The runtime tests
exercise pending requests, named-product satisfaction, decisive refutation,
fault/unresolved precedence, and generic re-evaluation rejection of tampering:

```sh
cargo test --test self_hosted_all
```

The retained reducer currently calls a small, fixed typed API for sealed-observation
queries and checked result construction. General S5 pattern matching and immutable
record/collection operations are the intended source-level replacement; adding the
next derived behaviors must not introduce behavior-specific Rust primitives.

The checker in this slice re-evaluates the retained reducer and verifies that every
derivation premise is sealed evidence from an observed child. Full obligation-graph
coverage remains part of the later analysis/evidence milestone; this runtime does not
claim complete v0 proof checking yet.

The registered verification foundation resolves explicit `§verify ... for "label"`
targets to structural obligation IDs, re-evaluates total contract expressions over
typed input/output values, dispatches only host-registered evidence producers, and
emits strongly validated deterministic evidence-bundle CBOR. Accepted, rejected,
unresolved, and operationally faulted verifier outcomes remain distinct. Missing
registrations produce required unresolved gaps; arbitrary project commands are not
silently executed. Evidence timestamps are injected, and the implemented boundary
accepts canonical UTC timestamps at second precision.

This library boundary does not yet build obligation or execution graphs, load
verifier commands from a project manifest, or run the coding-agent experiment's Rust
and policy adapters. Those adapters remain the next vertical slice; they will consume
the generic registry instead of becoming kernel primitives.

## Coding-agent experiments

[`experiments/minimal-coding-agent/`](experiments/minimal-coding-agent/) contains a
pinned, dependency-free Rust repository for the first controlled coding-agent
comparison. Its visible tests pass while an independent oracle rejects partial batch
commits, conflicting idempotency replays, and overflow rollback. The Markdown task
and canonical BHCP contract state the same requirements; the experiment isolates
whether BHCP makes completion claims more precise and mechanically checkable, not
whether hidden requirements can surprise an agent.

The fixture documentation defines the two-arm protocol and verifier boundary. The
generic dispatcher and evidence-bundle model are now executable; an external
controller must still register and run the experiment-specific Rust, oracle, and
change-policy adapters and provide the execution-graph reference.

The explicitly invoked repo skill at
[`.codex/skills/interpret-bhcp-contract/`](.codex/skills/interpret-bhcp-contract/)
turns lowered contracts into implementation and evidence matrices without becoming
a second semantic authority. It verifies the pinned identity, keeps implementation
state separate from verifier acceptance, and fails closed on unsupported or
unresolved contract boundaries.

[`experiments/policy-resolution-agent/`](experiments/policy-resolution-agent/)
adds a deliberately ambiguous authorization ticket. Its canonical contract pins
tenant isolation and a specificity → priority → deny → rule-ID precedence ladder
that the prose describes only as "most applicable," "conservative," and
"deterministic." This second fixture measures intent disambiguation rather than an
equal-information syntax comparison, and its checked oracle exposes five distinct
reasonable-but-policy-invalid implementations while retaining two passing control
invariants.

The trusted composition boundary is deliberately narrow. A network carries its
structural ID, output type, finite typed children, and reducer symbol—nothing else.
It carries no behavior kind, quantifier family, guard, dependency list, budget,
scheduling order, or parallelism hint. Quantifiers expand to finite children before
IR; recursive bounds belong to the recursive child call; and budget/concurrency
decisions live in execution graphs. Pending reducers name stable child tags, which the
kernel resolves through the network; reducers never allocate child or derivation IDs.
The next executable boundary is to generalize the total pure expression evaluator and
the metamodel beyond this `all` slice, then add `gate`, `any`, `none`, and `chain` as
checked-in prelude source without adding behavior kinds to Rust or semantic IR.

## Status

The executable slice is not a claim that the execution platform already exists. v0
is complete only when the parser, checker, planner, runtime, evidence machinery,
SDK, and CLI implement the complete type system, minimal kernel, proof checker, and
standard self-hosted prelude end-to-end.
