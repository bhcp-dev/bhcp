# BHCP

Beyond Human-Centric Programming (BHCP) is a semantic programming model in which
people declare outcomes, authority, limits, and required evidence while machines
discover acceptable executions.

This repository defines the normative v0 foundation and a focused first executable
slice. The dependency-free Rust implementation accepts a clause-only canonical
goal, emits a validated canonical AST and semantic IR, encodes both as deterministic
CBOR, and computes algorithm-tagged semantic and artifact identities. It is not yet
a complete v0 parser, checker, planner, runtime, or SDK.

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

The implemented source boundary supports namespaced/versioned clause-only goals;
typed `§input` and `§output` facts; `§requires`, `§ensures`, and `§limit` Boolean
expressions; `§allows` and `§forbids` effect atoms; ranked `§prefer`; and `§verify`
bindings. Scalar literals, binding references, parentheses, unary `!`/`-`, and the
checked Boolean, comparison, and `+` operators form the expression subset. Every
other reserved construct is rejected with a stable diagnostic rather than erased.

`bhcp.hash/sha3-512@0` is the default and only currently registered identity
algorithm, implemented in repository-owned Rust. It provides a roughly 256-bit
post-quantum preimage margin. [`bhcp-project.toml`](bhcp-project.toml) is the explicit
algorithm-agility boundary: projects may select another algorithm once the Rust
implementation registers it; unknown selections fail before parsing.

The crate has no third-party dependencies and does not invoke C, Ruby, Node.js, or a
package registry. Run every local acceptance check with:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo build --release
./scripts/validate-schemas
```

`cargo run --bin generate-fixtures` regenerates the checked-in AST and IR CBOR
artifacts for the canonical simple-goal fixture. The next implementation boundary is
type and predicate definitions plus goal calls and the first explicit composition
node; it does not include a planner or execution runtime.

## Status

The executable slice is not a claim that the execution platform already exists. v0
is complete only when the parser, checker, planner, runtime, evidence machinery,
SDK, and CLI implement the complete type system and all six core combinators
end-to-end.
