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
fixture to its expected root kind. The validation script:

- runs the pinned `cddl` 0.12.14 parser;
- converts every diagnostic fixture to CBOR and validates it against the bundle;
- confirms all root kinds are present exactly as declared by the fixture manifest;
- checks every understood `sha2-256` digest length; and
- decodes and re-encodes each instance deterministically, requiring byte equality.

Run from the repository root:

```sh
./scripts/validate-schemas
```

Schema shape validation is not a substitute for the cross-field and behavioral
rules in [`SEMANTICS.md`](../../SEMANTICS.md), including denominator positivity,
float bit widths, policy monotonicity, freshness, uniqueness, normalization, and hash
verification.
