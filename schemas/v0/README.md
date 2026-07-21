# BHCP v0 schema bundle

[`bhcp-v0.cddl`](bhcp-v0.cddl) is the normative CDDL bundle for BHCP v0. Its first
rule, `root-document`, enumerates every platform artifact:

1. canonical AST and semantic IR;
2. syntax definitions, profiles, policies, waivers, and extension descriptors;
3. obligation, capability, state, and execution graphs;
4. evidence bundles and execution results;
5. planner requests and results;
6. feature manifests; and
7. standalone content references.

The bundle follows RFC 8610. Instances use deterministic CBOR under RFC 8949 §4.2:
definite lengths, shortest integer encodings, deterministically ordered map keys,
UTF-8 NFC text, and no duplicate keys. Semantic sets are sorted arrays with unique
members. Exact and machine numeric values never depend on a host number model.

The v0 semantic kernel uses `kernel-network`, not a closed enumeration of behavioral
composition kinds. Total pure BHCP reducer functions consume sealed
`child-observation` values and emit `reduction` values. Pending reductions name stable
child tags; the runtime resolves those tags to child structural IDs through the
enclosing network. Reduction states are the
adjectives `pending | concluded`; execution states are `completed | faulted`; and
completed verdict states are `satisfied | refuted | unresolved`. Operational faults
therefore remain outside semantic verdicts.

A `kernel-network` contains only structural identity, output type, finite children,
and a reducer symbol. Quantified derived forms must expand before IR, recursion
bounds attach to recursive children, and budget/scheduling/parallel-eligibility
analysis belongs to execution graphs rather than semantic IR. Derivations carry only
an ID and sealed premise references: reducer source cannot select the ID, and the
generic checker derives it from the network plus exact reducer inputs, re-evaluates
the network's BHCP reducer, and seals accepted premises. No behavior-specific
proof-rule registry is part of the kernel.
Reducer validation requires exactly the parent input and a closed record containing
one `Option<ExecutionResult<ChildOutput>>` field per child, and requires
`Reduction<ParentOutput>` as the result.
The executable evaluator validates the complete retained expression before running
it, admits only typed literals, a finite Boolean/equality calculus, total
conditionals, parameter references, and a closed behavior-neutral primitive set,
and checks every satisfied output against `ParentOutput`. Unsupported calls remain
ordinary stable diagnostics rather than an extension or host-callback mechanism.

The checked-in `feature-manifest.diag` example declares canonical AST, semantic IR,
and all five graph-root wire documents, requires the core feature, advertises the
complete self-hosted `all`/`any`/`none`/`chain`/`gate` algebra and shared typed graph
model, and advertises deterministic obligation structural construction plus
capability graph decision construction as supported. The support notes keep
execution-time discharge, state/execution graph construction, planning, runtime
enforcement, and final evidence-graph assembly outside those two builder claims. The
goal-algebra conformance harness validates that bounded distinction alongside the
fixed 17-root inventory.

The waiver root is an authorization artifact with no standalone `semantic_id`. Its
non-empty target list pairs exact source-rule identities and optional policy scopes
with one of six closed category-specific weakening shapes. Direct or delegated issuer
authority, half-open timestamps, authorization material, justification, and the audit
reference enter artifact identity. Effective policy records retain the waiver
reference, exact targets, and injected decision time, while semantic identity remains
the normalized post-waiver `effective` member. The strong Rust model now validates
this root and the application path emits the corresponding typed applied-waiver
record deterministically.

Self-hosted lowerers use compile-time-only `meta-type` values. A lowerer receives a
typed `derived-form-shape` and returns an ID-free `network-shape`; both use
source-independent resolved children and expressions. Each derived child exposes its
resolved output type to the lowerer; runtime network children continue to resolve
their type through the referenced goal. The elaborator validates the
shape, assigns structural IDs, and rejects all meta values that survive into runtime
semantic IR.

Extension descriptors preserve the same boundary. A derived descriptor must name a
BHCP lowering function, is not must-understand after full lowering, and has no native
payload schema. A native descriptor has a required payload schema, is always
must-understand, and cannot masquerade as a derived lowering.

Syntax documents use a closed union of keyword, punctuation, and alias mappings.
Category-specific wire shapes prevent alias coordinates from degrading into arbitrary
values, while S9.1 supplies the registered canonical-coordinate, NFC, lexical,
uniqueness, prefix, alias, and core-protection checks that CDDL cannot express. The
separate closed formatting record permits only bounded indentation/line width and a
final-newline choice, none of which can alter tokens. Profile documents retain one
exact optional parent, one exact syntax, an ordered overlay list, and type mode;
S9.1 defines root-to-leaf resolution and rejects unrelated syntax, type-mode
relaxation, duplicate overlays, missing parents, and cycles before tokenization.

Policy documents use a closed typed union rather than an unrestricted `value` slot.
Each category has exactly one operation: requirements and evidence add typed demands,
prohibitions deny typed effect scopes, capabilities narrow typed effect scopes,
limits tighten exact maxima, and type modes strengthen along the v0 mode order.
Source policy documents carry `form = source`. Canonical effective documents carry
`form = effective`, separate the semantic restriction value from content-addressed
source layers and rule provenance, and keep all 17 root alternatives unchanged by
remaining under the existing `policy-document` root. The comparison, duplicate,
waiver, identity, and composition rules are normative in SEMANTICS S9.2.
For the effective form, the semantic hash projection is exactly `effective`; the
artifact projection retains every document field except `artifact_id` itself.
Content-addressed source layers and `rule_provenance` are consequently audit inputs,
not semantic inputs, while effective waivability and issuer constraints remain
semantic.

The files in [`examples/`](examples/) use CBOR diagnostic notation and contain at
least one instance of every root alternative. `examples/manifest.txt` binds each
fixture to its expected root kind; its `layered-policy.diag` entry exercises all six
closed policy category/operation shapes. The Rust validation harness:

- parses the normative bundle with cddl-rs 0.10.6 and rejects malformed CDDL;
- checks the CDDL root inventory, the minimal kernel-network shape, and the disjoint
  derived/native extension rules;
- checks that policy categories use closed category/operation/value shapes and uses
  a finite executable model to exercise antisymmetry and associative composition;
- parses every diagnostic fixture and validates its v0 root contract;
- confirms all root kinds are present exactly as declared by the fixture manifest;
- checks every understood `bhcp.hash/sha3-512@0` digest length;
- decodes and re-encodes each instance deterministically, requiring byte equality;
  and
- validates the checked-in compiler-emitted canonical AST and semantic IR CBOR
  artifacts for the simple, self-hosted `all`, `any`, `none`, `chain`, and `gate`
  programs under
  [`conformance/v0/fixtures/`](../../conformance/v0/fixtures/).

The Rust implementation also has a strongly typed `evidence-bundle-document` model
for the registered verifier slice. It validates claim/item/gap references,
per-obligation status justification, evidence classes, verifier and trust symbols,
content references, deterministic artifact identity, and canonical timestamp tags
before emission. Claims, producing items, and required gaps optionally retain the
exact network or child `execution_instance`; the proof boundary requires it and
rejects cross-instance claim/item/gap substitution. Process-produced items retain
the captured executable as
`verifier_artifact` and the normalized local adapter declaration as the optional
provenance `source`; the local manifest remains deployment configuration rather than
a new root document. The process-integration suite validates exact structural target
mapping, deterministic registration ordering and timestamps, all outcome categories,
and malformed output through this same CDDL root. Applicable policy evidence demands
use the optional `policy_obligations` mapping to retain the structural target,
effective-rule index, accepted classes, minimum, and sorted source-layer provenance;
their claims, items, gaps, and status remain in the ordinary evidence-bundle fields.
This does not claim the
still-deferred general obligation/execution graph builders or full CDDL instance
evaluation.

Source and effective `policy-document` values also cross a strongly typed Rust
boundary. It rejects unknown fields and invalid category/operation/value pairings,
validates every layer, scope, exact limit, evidence demand, issuer set, source and
rule identity, checks deterministic ordering, and verifies effective semantic and
artifact IDs before accepting external deterministic CBOR. Effective rule provenance
uses `rule_provenance`; the distinct generic document-header `provenance` map remains
available without a wire-key collision.

The manifest-driven fixtures under
[`conformance/v0/policy/`](../../conformance/v0/policy/) extend that root example into
the four-layer no-waiver slice. Each generated effective document is validated as a
`policy` root, decoded and re-encoded deterministically, and reconstructed through the
strongly typed policy boundary before its pinned semantic and artifact identities are
accepted. The compiled baseline program is likewise validated as a `semantic-ir`
root with the exact normalized policy decision retained.

Canonical `§policy` source is lowered into this source-document model before it is
accepted. The implemented slice covers layer and `§extends`, every closed typed rule,
scope and parameter meta-values, waivability and issuers, and retains definition and
rule spans in the canonical AST. Diagnostic-only labels, comments, and formatting do
not enter the policy document. Waiver/profile shorthand and expression-valued policy
clauses are explicitly deferred and rejected rather than accepted as opaque values.

The policy composer now emits canonical `effective-policy-document` values from
validated source documents. Source layers and policy references are sorted,
content-addressed, and retained separately from effective semantic meaning; rule
provenance is indexed after deterministic category ordering. Empty scope maps
canonicalize to an omitted universe, capability scopes intersect, exact limits take
their minimum, type mode strengthens, and exact duplicate governance combines by
waivability conjunction and issuer intersection. Missing/cyclic/cross-layer
inheritance, duplicate sources, forbidden weakening, and overlapping incompatible
limit units fail before an effective artifact is emitted. The Rust boundary reports
`BHCP8101`–`BHCP8107` for distinct weakening/conflict categories and `BHCP8110` for
source-topology failures. Conflict messages retain the attempted source rule and
earlier authority, while malformed typed policy values remain `BHCP8001`.

Policy-aware semantic IR optionally retains an `effective_policy` reference with
both semantic and artifact identities. Each governed goal carries a normalized
`policy_decision` whose category arrays index the applicable effective requirements,
evidence demands, prohibitions, capabilities, and limits, plus the enforced type
mode. A contract `limit` may carry a semantic dimension symbol for exact ceiling
comparison. These are optional for legacy ungoverned IR and add no new root type.

The `capability-graph` root now has an executable deterministic builder. Typed nodes
separate effect requests, structural resources, authored or propagated grants,
effective-policy grants and denials, applied-waiver audit records, and final
decisions. Policy detail retains the positional effective-rule audit link plus exact
layer/policy/rule provenance; source clause links and waiver presentation are also
artifact-only. Structural graph meaning uses normalized effect, resource, goal, and
effective restriction coordinates, so policy decomposition or unrelated positional
changes do not rename the corresponding authority node. Unsafe, foreign, and
unsupported extension-effect decisions carry an explicit gap, and exact validation
rebuilds the graph from semantic IR and the retained effective-policy envelope.

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
cargo test --test schema_fixtures
cargo test --test policy_conformance
```

Schema shape validation is not a substitute for the cross-field and behavioral
rules in [`SEMANTICS.md`](../../SEMANTICS.md), including denominator positivity,
float bit widths, policy monotonicity, freshness, uniqueness, normalization, and hash
verification.
