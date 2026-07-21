# BHCP v0 threat model

Status: active security boundary for the implemented repository slices and the
normative v0 contract. This document records abuse cases; `SEMANTICS.md` remains the
normative authority.

## Protected assets and trust boundaries

BHCP protects declared intent, effective policy restrictions, authority, artifact
identity, and acceptance evidence. Canonical source and deterministic artifacts may
cross untrusted storage or tooling. Profile normalization, policy composition,
waiver validation, planning, adapter execution, and evidence acceptance are separate
boundaries and must fail closed without returning partial later-stage artifacts.

The host supplies registered profiles, policy documents, authorization evidence,
verifier adapters, and an injected decision time. Those inputs are not trusted merely
because they came from the host: their exact typed shapes, identities, scope, and
authority must validate before they affect meaning.

## Waiver threats

- **scope amplification:** an attacker requests universe scope, a superset goal,
  resource, or operation set, or a broader `to` value than the exact authorized
  change. Validation requires the application scope to be a subset of both the
  source restriction and the typed change; wildcards and inferred changes reject.
- **rule or change substitution:** an attacker reuses a valid signature for another
  policy/rule pair, category, value, or weakening. Targets bind the exact
  `(source-policy-symbol, rule-id)`, category-specific `from`/`to` or removed value,
  and canonical scope into the signed artifact identity.
- **delegation-chain confusion:** an attacker splices chains, changes their root or
  final issuer, repeats a principal, or supplies a disconnected link. Chains must
  begin in every affected rule's authorized-issuer intersection, connect exactly,
  end at the document issuer, remain acyclic, and carry authorization evidence for
  every hop.
- **clock rollback:** an attacker reuses an earlier successful validation after
  expiry or lets different targets observe different times. The caller injects one
  normalized decision time per atomic application; no ambient clock or cached
  validity result is authoritative. The interval is half-open, so expiry equality
  rejects.
- **audit-reference substitution:** an attacker swaps the justification, audit
  record, signature, or delegation evidence while preserving the requested policy
  change. All such metadata enters waiver artifact identity and must validate before
  application; the effective artifact retains the waiver reference and decision
  time.
- **non-waivable downgrade:** an attacker presents stronger external authority for a
  rule whose governance is non-waivable or whose contributing issuer intersection is
  empty. Non-waivable status is final; delegation cannot create waiver authority.
- **partial acceptance:** an attacker mixes valid and invalid targets so a validator
  silently applies the valid subset. Validation and policy application are atomic;
  one invalid target produces no effective artifact.
- **source-path confusion:** an attacker reorders inline governance, mixes it with a
  separately supplied effective policy, leaves symbolic authorization or audit
  references unresolved, or relies on an ambient clock. Inline policy order is
  normalized, inline/external policy mixtures reject, materialized references are
  required for application, and the caller injects one decision time before IR.

## Other implemented boundaries

- Presentation profiles are closed typed data, not parser callbacks. Ambiguous or
  recursive aliases, core capture, executable macros, semantic overrides, and
  unregistered profiles reject before artifacts escape.
- Policy layers may only restrict. Widening, loosening, removal, allow-over-deny,
  type weakening, incompatible units, and invalid topology reject a whole layer.
- Process verifiers run only through exact project-relative registrations and a
  capability-bounded sandbox with no shell, `PATH`, ambient network, or inherited
  descriptor authority. Missing enforcement fails before launch.
- Reducer conclusions are re-evaluated by the generic checker against sealed child
  observations and typed outputs. Behavior-specific proof tags are not trusted.
- Obligation proofs reject graph/dependency deletion, target substitution, forged or
  colliding derivation tokens, cross-wired item/claim edges, unsealed payload bytes,
  unretained producers, and mismatched candidate or semantic-IR identities. Contract
  expression evidence is tied to the exact retained goal and always re-evaluated
  against its sealed input/output context; observed child results must agree with
  the aggregate statuses of their structural prerequisites; policy evidence must
  satisfy the retained class and distinct-producer minimum.
- A refutation is not an operational fault, and an unresolved result is not accepted
  under a substituted reason. Faulted and unresolved conclusions must bind the exact
  required evidence gap while reducer trace contents remain opaque to semantic
  choice. Accepted counter-evidence remains a valid generic premise when the retained
  reducer uses it to establish a parent conclusion such as `none`.
- Ownership transfer is checked before semantic IR. An attacker cannot disguise an
  owned copy as `value`, move a shared or borrowed handle, grant write through a read
  handle, reuse a possibly moved binding after a branch join, or schedule an
  exclusive access beside another candidate. Diagnostics bind the resource,
  lifetime, branches, and both source locations.
- Persistent state cannot capture a borrow. A shared capture requires an exact
  goal/binding/lifetime approval supplied to the ownership boundary; naming a
  lifetime `persistent` does not create authority. Invalid ownership is atomic and
  produces no later artifact.
- Effects cannot disappear at a goal-call boundary. Canonical possible-effect rows
  propagate child atoms, substitute direct resource bindings, accumulate denies, and
  reject explicit parent-ceiling excess before semantic IR is emitted. Effective
  capability/prohibition scopes match nominal resource types and literal operations;
  unsafe or foreign atoms keep an unresolved evidence gap. Limits reject indirect,
  negative, or inexact maxima, and a same-priority preference group cannot mix
  incompatible objective types.
- Planning cannot mint capability. The capability graph is rebuilt from a validated
  semantic-IR byte/identity envelope and its exact retained effective policy. Every
  possible effect receives one structural request and final allow decision with
  authored or propagated authority and, when governed, a matching policy grant.
  Applicable denials, source rules, waiver artifacts, resource coordinates, and
  unsafe/foreign gaps remain visible. Deleted, substituted, out-of-scope, or
  fabricated decisions fail exact graph validation before planning.

## Residual and deferred risks

The repository is not a complete v0 parser, planner, runtime, proof system, evidence
graph, or execution graph. The obligation-graph proof-checker slice is implemented,
but it is not a substitute for the deferred executor and evidence-graph assembly.
Effect rows are conservative declarations, not proof that
an implementation cannot perform undeclared host effects; runtime capability
enforcement remains required. State-graph construction, signature policy and key
distribution, revocation services, durable clock attestation, and full CDDL instance
interpretation are deferred implementation work. A
deployment must not treat a normative artifact shape or decision vector as proof
that those runtime services already exist.
