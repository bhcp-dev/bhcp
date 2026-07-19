# Pilot 006: ordered contextual-policy resolution

Date: 2026-07-18

This round evaluates the token-optimized BHCP interpretation skill on a harder
policy selector. The primary comparison has three arms; a skill-only follow-up
captures the revision that reached `main` while that comparison was running. The
fixture has ten independently judged semantic
boundaries and an ordered specificity lattice: resource, subject, action, priority,
denial, then smaller rule ID. The prose ticket describes these concepts without
stating that exact ladder; the canonical BHCP contract names every step.

## Verdict

Raw BHCP and prose both produced independently accepted patches. The optimized
skill did not:

- raw BHCP passed 10/10 withheld invariants and claimed success;
- prose passed 10/10 withheld invariants and claimed success; and
- BHCP plus the skill passed 8/10, correctly withheld a success claim, but replaced
  the ordered resource/subject/action lattice with a flat exact-field count.

The primary negative skill result is the important finding. The compact skill preserved
its process advantages—it used one `hash`, one `inspect`, no raw compiler artifact,
the fewest commands, the fewest input tokens, and the shortest wall time—but its
working representation did not preserve two distinct precedence obligations.

After the primary run, `main` advanced to PR #10. A skill-only follow-up with that
revision passed 10/10. PR #10 changed CLI discovery, not the semantic workflow, so
the correctness difference is evidence of run variance rather than evidence that
the later edit fixed ordered-obligation handling.

This run does not demonstrate a BHCP correctness advantage over prose: both
non-skill arms were accepted. It demonstrates that the optimized skill can improve
efficiency and evidence calibration while still exhibiting semantic variance on
interacting ordered obligations.

## Frozen inputs and controls

The admissible run froze all inputs before any included session:

- BHCP base commit: `64b5d164e4083041da0bbb09f10d5840a04f35d8`;
- subject `src/lib.rs` Git blob:
  `3f126bfde1c0e06309686c9c3514548759d650eb`;
- task Git blob: `82ae3d3545ee1f73fe6ed7180a1278e4680ab420`;
- contract Git blob: `dfb58210587b15abfc0d0cbaa337a653b5d6dd29`;
- withheld oracle Git blob:
  `3667d107a7777a09f71a69c871802c0f4e07dde1`;
- semantic ID:
  `bhcp.hash/sha3-512@0:f66185376bb6dcdc316914002ea0d7e21a289cf3c720d2490c5e0ea247df9e400d70f709ca85ea5b96e6b137b92da7c19544719cb73bca75853d2e2a357f168e`;
- five public tests and ten withheld oracle tests;
- Rust `1.97.1`, no added dependencies, no network, and only `src/lib.rs`
  permitted to change; and
- Codex CLI `0.142.4`, `gpt-5.4-mini`, medium reasoning, ephemeral state,
  ignored user configuration, and workspace-write isolation.

The run order was raw BHCP, prose, then BHCP plus skill. The first prose process was
externally interrupted before it emitted a final response or usage record; it was
discarded and recreated from the frozen inputs. The table contains only the fresh,
completed prose session.

Fixture development also included an unrecorded formatting preflight and a
diagnostic comparison before the final oracle was frozen. Those sessions are not
included in the results. This admissible run began only after the oracle blob above
was fixed; the oracle remained absent from every agent workspace until all three
included agents had stopped.

The primary evaluated skill is saved exactly under
[`evaluated-skill/`](evaluated-skill/):

- `SKILL.md` Git blob: `b1a2f5fdfb3044be679f1e947bf1a1e56957e278`;
- `agents/openai.yaml` Git blob:
  `e6611a4fb13245035f7a9ecf191d48baac49b858`.

The follow-up used `main` commit
`4a9a7788e9b7066fdf308b5d8e5ea6ef35842cee` and is saved under
[`latest-skill/`](latest-skill/):

- `SKILL.md` Git blob: `7c6b39df7e7682afa59e2d75d1f4cb13276ea2c7`;
- `agents/openai.yaml` Git blob:
  `e6611a4fb13245035f7a9ecf191d48baac49b858`.

The oracle was unchanged for the follow-up and remained absent until that agent
also stopped.

## Prompts

The shared prompt was:

```text
Work autonomously on the coding task in TASK.md. Meet every stated requirement; an independent oracle is withheld until you stop. Do not use the network. Edit only src/lib.rs and preserve the public API. Run all visible formatting, Clippy, and test checks before reporting success. Do not ask questions. In your final response, state whether you claim success, name every changed file, and list the checks you actually ran.
```

The raw-BHCP arm additionally received:

```text
The canonical machine contract is contract.bhcp and its pinned identity is contract.semantic-id. Treat every targeted obligation and verifier binding in that contract as required for acceptance.
```

The skill arm explicitly invoked `$interpret-bhcp-contract`, received the same
authority statement, and was told to report obligation evidence only when its bound
producer actually ran.

## Results

| Measure | Prose | Raw BHCP | BHCP + skill |
| --- | ---: | ---: | ---: |
| Claimed success | yes | yes | no |
| Independently accepted | **yes** | **yes** | **no** |
| Public tests | 5/5 | 5/5 | 5/5 |
| Withheld policy invariants | 10/10 | 10/10 | 8/10 |
| Formatting and Clippy | pass | pass | pass |
| One-file/dependency/API policy | pass | pass | pass |
| Patch size | +18/-11 | +26/-11 | +16/-2 |
| Completed commands | 22 | 22 | 15 |
| Failed intermediate commands | 2 | 5 | 3 |
| Input tokens | 272,982 | 157,489 | 153,676 |
| Cached input tokens | 250,112 | 142,080 | 139,008 |
| Output tokens | 9,980 | 5,625 | 5,985 |
| Reasoning output tokens | 6,501 | 2,910 | 4,009 |
| Wall time | 163.56 s | 102.29 s | 96.95 s |
| Resulting `src/lib.rs` Git blob | `a5beb69383f4870da9db24d7e5ea222ee6b08938` | `b016d1e687ce3fa85f0ded03779f51edab67e576` | `8536cc2fda4ba0792b9993f3c0288f49cf23e74a` |

Relative to raw BHCP, the skill used 2.4% fewer input tokens, 31.8% fewer commands,
and 5.2% less wall time. Relative to prose, it used 43.7% fewer input tokens and
40.7% less wall time. These efficiency gains did not compensate for rejection by
the semantic oracle.

Exact candidate changes are preserved in [`prose.patch`](prose.patch),
[`raw-bhcp.patch`](raw-bhcp.patch), and [`skill.patch`](skill.patch).

## Latest-main skill follow-up

| Measure | PR #9 skill | Latest-main skill |
| --- | ---: | ---: |
| Claimed success | no | no |
| Independently accepted | **no** | **yes** |
| Withheld policy invariants | 8/10 | 10/10 |
| Patch size | +16/-2 | +26/-11 |
| Completed commands | 15 | 22 |
| Failed intermediate commands | 3 | 6 |
| Input tokens | 153,676 | 262,181 |
| Cached input tokens | 139,008 | 240,384 |
| Output tokens | 5,985 | 7,910 |
| Reasoning output tokens | 4,009 | 4,412 |
| Wall time | 96.95 s | 131.77 s |
| Resulting `src/lib.rs` Git blob | `8536cc2fda4ba0792b9993f3c0288f49cf23e74a` | `de7fd05ca696476a510354f3f37dba0585c13b4c` |

The current-skill candidate is preserved in
[`current-skill.patch`](current-skill.patch). It passed all visible and withheld
checks and retained the correct fail-closed final claim.

The later skill used 70.6% more input tokens, 46.7% more commands, and 35.9% more
wall time than the primary skill arm. Because this is one follow-up session and the
semantic instructions were unchanged, neither its correctness improvement nor its
efficiency regression should be attributed solely to PR #10.

## Independent acceptance

After all included sessions stopped, the controller copied the already-frozen
oracle into fresh judge directories. Every candidate passed formatting, offline
Clippy with warnings denied, all five public tests, `git diff --check`, and the
one-file policy.

The raw-BHCP and prose candidates both implemented exact tenant ownership and the
complete ordered comparator. They passed all ten invariants.

The skill candidate implemented tenant isolation, default denial, action
specificity, priority, denial, smaller-ID tie-breaking, insertion independence,
and disabled-rule exclusion. It failed:

- `resource_specificity_dominates_other_exact_fields`; and
- `subject_specificity_breaks_equal_resource_scope`.

Its `specificity_score` counted exact subject, action, and resource fields, allowing
two lower-precedence exact fields or a higher priority to displace a rule that
should win at an earlier specificity tier.

The latest-main skill follow-up implemented the complete ordered comparator and
passed all ten invariants against the same frozen oracle.

## Process findings

The skill arm ran exactly one `bhcp hash` and one `bhcp inspect`, emitted no raw AST
or IR, used no schema intake, changed only `src/lib.rs`, and correctly refused to
claim acceptance without the registered contextual-policy oracle. The raw-BHCP arm
did not invoke a compiler view during its session and claimed success without bound
oracle evidence, despite producing an accepted patch.

The primary skill made three avoidable failed Cargo attempts before using `mise`.
The latest-main follow-up made six failed attempts while locating and invoking the
Rust toolchain. PR #10 deliberately simplified the `bhcp` runtime contract, but it
also removed the adjacent Cargo/mise fallback sentence. The trace is consistent
with that removal increasing toolchain discovery, though a single stochastic run
cannot establish causality.

The next skill boundary should remain compact: when multiple obligations establish
an ordered precedence ladder, retain each structural ID as a distinct comparison
tier and check that the implementation has one ordered decision step per tier.
That guardrail should be evaluated across repeated seeds on this frozen fixture,
without restoring a printed obligation matrix or front-loading schemas. Repeated
runs are now necessary: one semantically unchanged skill workflow produced both an
8/10 and a 10/10 candidate.
