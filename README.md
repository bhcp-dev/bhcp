# BHCP

Beyond Human-Centric Programming (BHCP) is a semantic programming model in which
people declare outcomes, authority, limits, and required evidence while machines
discover acceptable executions.

This repository currently defines the v0 foundation. It deliberately does not yet
contain a parser, checker, planner, runtime, CLI, or SDK.

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

The schema bundle pins the `cddl` validator and checks every CDDL file, validates at
least one example of every root document type, and verifies deterministic-CBOR
round trips:

```sh
./scripts/validate-schemas
```

See [`schemas/v0/README.md`](schemas/v0/README.md) for the artifact inventory,
canonical encoding rules, and validation details.

## Status

The semantic contract is specification work, not a claim that the execution
platform already exists. v0 is complete only when the parser, checker, planner,
runtime, evidence machinery, SDK, and CLI implement the complete type system and
all six core combinators end-to-end.
