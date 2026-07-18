# Pilot 005: skill token-intake optimization

Date: 2026-07-18

This single-arm replay optimizes the merged BHCP interpretation skill against the
frozen Pilot 004 challenge. The source, refined task, contract, semantic ID, model,
prompt, visible tests, and withheld oracle are unchanged. Pilot 004's raw-BHCP and
merged-skill arms are the controls.

## Verdict

The accepted revision preserved correctness and fail-closed evidence reporting while
reducing total input tokens from 210,920 to 154,887, a 26.6% reduction. Its premium
over raw BHCP fell from 49.2% to 9.6%, below the 20% target set before the replay.

The optimized agent:

- produced a focused, independently accepted patch with 7/7 oracle invariants;
- ran one `hash` and one `inspect`, with no `parse` or `lower`;
- printed no obligation matrix and loaded no schema or contract source;
- used 12 commands with no failed intermediate command; and
- correctly refused to claim success because no registered verifier adapter ran.

Wall time did not improve: the accepted replay took about 116 seconds versus 112
seconds for the merged skill and 94 seconds for raw BHCP. This revision optimizes
token intake and process reliability, not model latency.

## Iteration findings

Two forward tests were rejected before the accepted replay:

1. A 363,555-token run opened the compiled `bhcp` executable as text after choosing
   the wrong wrapper, then added six tests as a substitute for unavailable adapters.
   The skill now resolves the CLI once, forbids executable inspection and `--help`
   probing, and forbids substitute edits.
2. A 216,883-token run avoided those failures but performed fourteen pre-edit
   discovery commands and treated visible Rust checks as registered evidence. The
   skill now batches independent reads, forbids redundant inventory and metadata
   queries, and states that a visible check is evidence only when it is the
   registered producer.

Rejected event records and working directories remained outside the repository.
They are diagnostic iterations, not accepted measurements.

## Frozen inputs and controls

- BHCP base commit: `98092552efda108cd3ce02e3787ad38239e09066`;
- subject `src/lib.rs` Git blob:
  `f84499b07f4e01f009ecb9bba4be3798a31e3f73`;
- refined task Git blob: `5b30594b6099af66291a19abfab3b62bd5db7db1`;
- semantic ID:
  `bhcp.hash/sha3-512@0:d10ead1268ed05db2bfbc018756555804360fc6aa3369ea96a71adf0750850460d0d834160c6c617da1a4347c928885b5a5a3b29b93d5ab29252dbd9e3156880`;
- Codex CLI `0.142.4`, `gpt-5.4-mini`, medium reasoning;
- ephemeral state, ignored user configuration, workspace-write isolation;
- public tests: 4;
- withheld oracle tests: 7; and
- only `src/lib.rs` permitted to change.

The exact accepted skill is saved under [`evaluated-skill/`](evaluated-skill/):

- `SKILL.md` Git blob: `b1a2f5fdfb3044be679f1e947bf1a1e56957e278`;
- `agents/openai.yaml` Git blob:
  `e6611a4fb13245035f7a9ecf191d48baac49b858`.

The resulting `src/lib.rs` Git blob is
`1135d01d46d53a2f2575c1bbae35053a404f4f5e`. Its exact change is preserved in
[`accepted.patch`](accepted.patch).

## Comparison

| Measure | Raw BHCP | Merged skill | Optimized skill |
| --- | ---: | ---: | ---: |
| Claimed success | yes | no | no |
| Independently accepted | yes | yes | yes |
| Public tests | 4/4 | 4/4 | 4/4 |
| Withheld policy invariants | 7/7 | 7/7 | 7/7 |
| Patch size | +28/-2 | +27/-2 | +30/-2 |
| Completed commands | 18 | 19 | 12 |
| Failed intermediate commands | 2 | 2 | 0 |
| Input tokens | 141,372 | 210,920 | 154,887 |
| Cached input tokens | 127,360 | 192,000 | 139,520 |
| Output tokens | 5,057 | 6,856 | 7,364 |
| Reasoning output tokens | 2,904 | 3,223 | 4,891 |
| Approximate wall time | 94 s | 112 s | 116 s |

Relative to the merged skill, cached input fell 27.3% and completed commands fell
36.8%. The skill itself is 3,563 bytes and 538 words, down from 5,573 bytes and 823
words. The remaining output-token and latency increase should be treated separately
from intake: neither was improved by this run.

## Independent acceptance

After the agent stopped, the controller copied the same withheld oracle into a fresh
judge directory and ran formatting, offline Clippy, all four public tests, all seven
oracle tests, `git diff --check`, and the one-file policy. Every check passed.

The final response did not convert visible checks into verifier evidence. It named
all three unavailable registered producers and withheld the success claim while the
independent post-session judge accepted the candidate.
