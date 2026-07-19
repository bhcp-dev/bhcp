# Phase 4 presentation-layer completion audit

Status: Phase 4 presentation layer is complete through the safe v0 profile boundary.
BHCP v0 is not complete: the repository still implements focused Rust slices rather
than the full parser, checker, planner, runtime, proof system, or execution graph.

This report maps every acceptance claim in issues
[#41](https://github.com/bhcp-dev/bhcp/issues/41) through
[#49](https://github.com/bhcp-dev/bhcp/issues/49) to executable evidence. Normative
behavior remains in [SEMANTICS S9.1](../../SEMANTICS.md), wire shapes remain in the
[v0 CDDL bundle](../../schemas/v0/bhcp-v0.cddl), and this report records evidence and
maturity without creating another semantic contract.

## Acceptance evidence

| Issue | Acceptance claim | Primary executable evidence |
| --- | --- | --- |
| #41 | Mapping categories have exact uniqueness and conflict rules. | [`syntax_resolution_vectors_pin_safe_overrides_and_every_conflict_class`](../../tests/profile_contract.rs) |
| #41 | Exact parents, aliases, core protection, and overlay order are unambiguous. | [`profile_resolution_vectors_pin_parent_overlay_and_type_mode_order`](../../tests/profile_contract.rs) |
| #41 | Presentation changes cannot alter canonical meaning. | [`semantics_and_wire_contract_name_the_closed_decision_boundaries`](../../tests/profile_contract.rs) |
| #42 | Omission, BOM, ASCII spacing, and LF select exactly one profile. | [`omission_explicit_canonical_and_bom_select_exactly_one_profile`](../../tests/profile_preamble.rs) |
| #42 | Duplicate, misplaced, non-ASCII, malformed, and aliased preambles reject stably. | [`malformed_truncated_aliased_and_non_ascii_preambles_fail_stably`](../../tests/profile_preamble.rs) |
| #42 | Profile selection occurs before profile-controlled lexing. | [`custom_profile_is_selected_without_aliasing_but_fails_closed_before_lexing`](../../tests/profile_preamble.rs) |
| #43 | Every syntax mapping and profile field round-trips deterministically. | [`every_mapping_category_and_profile_field_round_trip_deterministically`](../../tests/profile_models.rs) |
| #43 | Unknown fields, invalid parents, duplicates, order, formatting, and modes diagnose stably. | [`malformed_profile_fixtures_have_stable_model_diagnostics`](../../tests/profile_models.rs) and the [malformed corpus](../../tests/fixtures/profile_models/invalid) |
| #43 | Root artifacts and open feature negotiation remain compatible. | [`root_validation_uses_typed_models_without_rejecting_negotiated_features`](../../tests/profile_models.rs) |
| #44 | All mapping categories lower completely with original spans. | [`every_mapping_category_lowers_to_the_canonical_token_stream_once`](../../tests/profile_lowering.rs) and the [paired sources](fixtures/profile-lowering-words.bhcp) |
| #44 | Ambiguity, cycles, token capture, and core override reject before parsing. | [`invalid_effective_maps_fail_before_accepting_any_program_token`](../../tests/profile_lowering.rs) |
| #44 | Canonical and custom source preserve semantic identity. | [`custom_and_canonical_source_compile_to_the_same_semantic_identity`](../../tests/profile_lowering.rs) |
| #45 | Missing/cyclic parents, unrelated syntax, weaker modes, conflicts, and overlay weakening reject. | [`missing_cycles_unrelated_syntax_weaker_modes_and_duplicate_overlays_fail_stably`](../../tests/profile_resolution.rs) |
| #45 | Inherited mappings and overlays normalize root to leaf independent of registration order. | [`syntax_profile_and_overlay_chains_resolve_root_to_leaf_deterministically`](../../tests/profile_resolution.rs) |
| #45 | Profile selection preserves core meaning and monotonic policy. | [`resolved_profile_compilation_preserves_meaning_and_applies_overlays_before_elaboration`](../../tests/profile_resolution.rs) |
| #46 | Canonical formatting is deterministic, idempotent, and semantic-invariant. | [`canonical_formatting_is_deterministic_idempotent_and_semantic`](../../tests/profile_formatting.rs) |
| #46 | Custom Unicode/comments/layout reparse to equivalent AST meaning and IR. | [`inherited_custom_formatting_preserves_comments_unicode_and_round_trips`](../../tests/profile_formatting.rs) |
| #46 | Invalid or missing formatting registrations fail instead of being ignored. | [`invalid_or_unregistered_formatting_fails_before_output`](../../tests/profile_formatting.rs) |
| #47 | Ambiguous/recursive aliases, capture, and reserved rebinding reject with structured context. | [`executable_profile_attacks_name_profile_mapping_rule_and_stable_span`](../../tests/profile_adversarial.rs) |
| #47 | Parser code, unrestricted macros, and semantic overrides remain outside the closed model. | [`parser_macro_and_semantic_override_artifacts_fail_the_closed_model`](../../tests/profile_adversarial.rs) and its [fixture corpus](../../tests/fixtures/profile_adversarial) |
| #47 | Failure returns no partial formatted source or artifact. | [`cli_failure_is_atomic_for_an_invalid_effective_profile`](../../tests/profile_adversarial.rs) |
| #48 | Two substantially different checked-in profiles parse and format one governed goal. | [`substantially_different_checked_in_layouts_preserve_governed_semantic_identity`](../../tests/profile_layout_conformance.rs) and the [layout corpus](profile-layout) |
| #48 | Semantic IDs match while retained profile and artifact identities differ. | [`substantially_different_checked_in_layouts_preserve_governed_semantic_identity`](../../tests/profile_layout_conformance.rs) |
| #48 | Overlay, comment, label, formatting, and diagnostic boundaries are pinned. | [`formatting_comments_labels_policy_and_diagnostics_pin_the_identity_boundary`](../../tests/profile_layout_conformance.rs) |
| #49 | Every Phase 4 acceptance claim names checked-in executable evidence. | [`every_phase_four_acceptance_claim_names_executable_evidence`](../../tests/profile_phase_audit.rs) and the [machine manifest](profile-phase-audit.txt) |
| #49 | Arbitrary grammars, parser plugins, and unrestricted macros remain explicit non-goals. | [`maturity_and_closed_profile_non_goals_remain_consistent`](../../tests/profile_phase_audit.rs) |
| #49 | Repository maturity statements and local evidence links agree. | [`phase_four_report_local_links_resolve`](../../tests/profile_phase_audit.rs) |

## Reviewed delivery ledger

| Issue | Pull request | Squash merge |
| --- | --- | --- |
| #41 | #73 | `31fe6421da73c2f56a06d471afe4010bd9c782e5` |
| #42 | #74 | `46f1773a898fe714eaff9fac40ce3006d2d43db3` |
| #43 | #75 | `bf96d72cb731a660718f8281065c1c6c28764882` |
| #44 | #76 | `7b7475aae72529d83f86ff276ae9770222bbc6a5` |
| #45 | #77 | `5caa7c750ebfe32f58bdc3e7ab257b6c0d583627` |
| #46 | #78 | `2440a83f00f58fa2a93c4f0bbc71473a7fafa6a4` |
| #47 | #79 | `e76e50ce7720ca8c09d89edacd5fde3e3cffef51` |
| #48 | #80 | `7084f6e6a1a1a687cc1ba746f8ac10e194301000` |
| #49 | #81 | `7ab426e6f5dabb2d11bd6245ac1da3337124ddb4` |

The ledger is complete through the audit's own reviewed squash merge. Wiki, issue,
and milestone status are post-merge metadata reconciled under the same protected
`main`-branch gate rather than inputs to an artifact's immutable delivery identity.

## Consistency result

- [README](../../README.md) and [VISION](../../VISION.md) describe the implemented
  Phase 4 slice while continuing to state that BHCP is not a complete v0 system.
- [SEMANTICS](../../SEMANTICS.md) remains normative; no audit-only wording changes
  its closed one-token mapping, identity, inheritance, overlay, or formatting rules.
- [Conformance guidance](README.md) indexes SYN-01 through SYN-08 and points to the
  executable profile contract, scanner, models, lowering, resolution, formatter,
  adversarial, cross-layout, and phase-audit harnesses.
- [AGENTS.md](../../AGENTS.md) and the
  [project-loop profile](../../.codex/project-profile.md) retain the same authority,
  review, merge, and post-merge consistency contract.
- Wiki [Purpose](https://github.com/bhcp-dev/bhcp/wiki/Purpose),
  [Architecture](https://github.com/bhcp-dev/bhcp/wiki/Architecture-and-Approaches),
  [Development](https://github.com/bhcp-dev/bhcp/wiki/Development-and-Operations),
  [Decisions](https://github.com/bhcp-dev/bhcp/wiki/Decisions-and-Interesting-Bits),
  and [Roadmap](https://github.com/bhcp-dev/bhcp/wiki/Roadmap-and-Agentic-Workflow)
  publish the completed #49 result and the remaining claimable roadmap queue.
- Pull requests [#73](https://github.com/bhcp-dev/bhcp/pull/73) through
  [#81](https://github.com/bhcp-dev/bhcp/pull/81) are the reviewed squash merges for
  the complete implementation/evidence chain. Issue and
  [Phase 4 milestone](https://github.com/bhcp-dev/bhcp/milestone/4) closure are
  post-merge metadata reconciled only after protected `main` is green.

## Explicit non-goals and residual boundary

Arbitrary grammars, executable parser callbacks, parser plugins, unrestricted macros,
semantic override payloads, implicit parents, profile fallback search, and per-goal
profile switching remain outside v0. Phase 4 proves a bounded presentation layer; it
does not claim general syntax extensibility, a complete BHCP v0 implementation, or
that presentation profiles may bypass policy or change canonical meaning.
