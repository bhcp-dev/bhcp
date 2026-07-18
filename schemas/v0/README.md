# BHCP v0 schema bundle

[`bhcp-v0.cddl`](bhcp-v0.cddl) is the normative CDDL bundle for BHCP v0. Its first
rule, `root-document`, enumerates every platform artifact:

1. canonical AST and semantic IR;
2. syntax definitions, profiles, policies, waivers, and extension descriptors;
3. obligation, capability, state, and execution graphs;
4. evidence bundles and runtime outcomes;
5. planner requests and results;
6. feature manifests; and
7. standalone content references.

The bundle follows RFC 8610. Instances use deterministic CBOR under RFC 8949 §4.2:
definite lengths, shortest integer encodings, deterministically ordered map keys,
UTF-8 NFC text, and no duplicate keys. Semantic sets are sorted arrays with unique
members. Exact and machine numeric values never depend on a host number model.

The files in [`examples/`](examples/) use CBOR diagnostic notation and contain at
least one instance of every root alternative. `examples/manifest.txt` binds each
fixture to its expected root kind. The Rust validation harness:

- parses the normative bundle with cddl-rs 0.10.6 and rejects malformed CDDL;
- checks the CDDL root inventory and the declarative-goal rule;
- parses every diagnostic fixture and validates its v0 root contract;
- confirms all root kinds are present exactly as declared by the fixture manifest;
- checks every understood `bhcp.hash/sha3-512@0` digest length;
- decodes and re-encodes each instance deterministically, requiring byte equality;
  and
- validates the checked-in compiler-emitted canonical AST and semantic IR CBOR
  artifacts under [`conformance/v0/fixtures/`](../../conformance/v0/fixtures/).

The cddl-rs CBOR validator is not used for instances yet: version 0.10.6 misvalidates
repeated references to controlled aliases used by this schema, including
`[* feature-id]` where `feature-id` ultimately carries `.regexp`. The normative
schema is not weakened to accommodate that behavior. Implemented compiler artifacts
are checked by strongly typed Rust models, while the root fixture suite checks the
stable document inventory, required root fields, digest rules, and canonical wire
behavior. Enabling full CDDL-driven instance validation after that upstream
compatibility boundary is resolved is the next schema-tooling step.

Run from the repository root:

```sh
./scripts/validate-schemas
```

Schema shape validation is not a substitute for the cross-field and behavioral
rules in [`SEMANTICS.md`](../../SEMANTICS.md), including denominator positivity,
float bit widths, policy monotonicity, freshness, uniqueness, normalization, and hash
verification.
