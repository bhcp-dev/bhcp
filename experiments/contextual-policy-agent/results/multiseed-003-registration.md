# Registration: contextual-policy multi-seed run 003

Registered: 2026-07-19, before any run-003 session.

Experiment ID: `contextual-policy-multiseed-003`.

## Reason for a new run

Independent review found that runs 001 and 002 did not establish filesystem read
confinement. Codex's workspace sandbox constrained writes but left the original
repository, oracle, and prior artifacts readable from the host. Run 002 also used
a direct-Cargo Clippy judge that selected Rust 1.96.0 after environment clearing.
Those records are preserved, retrospectively downgraded, and not relabeled. Run
003 is a new five-session sample without replacement.

## Frozen question and sample

The question is whether the exact Pilot 006 compact evaluated skill's two
ordered-obligation failures recur across five independent stochastic sessions when
each agent is genuinely confined to the registered visible inputs. Codex exposes
no numeric model seed; `seed-01` through `seed-05` are fixed session identifiers,
run once in that order. Five observations can describe variance and obvious
recurrence on this fixture, but cannot estimate a population rate or establish
generality.

Every session uses the unchanged starter, task, canonical BHCP contract and
semantic ID, ten-invariant oracle, prompt, and Pilot 006 evaluated-skill Git blob
`b1a2f5fdfb3044be679f1e947bf1a1e56957e278`. The model is `gpt-5.4-mini`, reasoning
is medium, Codex CLI is exactly `0.142.4`, Rust is exactly `1.97.1`, the session
limit is fifteen minutes, agent-command network access is disabled, and only
`subject/src/lib.rs` may change.

## Read and executable boundary

The registered sandbox pin is
`workspace-write/no-network/read-confined`. On macOS, an outer `sandbox-exec`
profile denies reads and writes across the user's host tree and temporary trees,
then permits only the staged workspace, controller-owned target, Cargo and Rustup
caches, exact Codex executable, and exact isolated credential access required by
the Codex parent process. Codex child commands cannot read the isolated credential
file. Before each model launch, the driver proves with the same profile that the
staged prompt is readable and the original oracle file is unreadable. Unsupported
operating systems fail closed.

The runner resolves Codex, BHCP, Rustup, and the Rust 1.97.1 Cargo, rustc, rustdoc,
Rustfmt, Cargo-Clippy, and Clippy-driver paths through symlinks before freezing. It
hashes every exact executable into the plan, verifies that Rustup selects those
same 1.97.1 files, and rechecks the frozen identities before every arm. The driver
copies the frozen BHCP executable into its confined tool directory. Historical run
IDs 001 and 002 retain their original direct-Cargo judge plans; this corrected
protocol uses only ID 003.

## Outcomes and exclusions

Judges run in fixed order: Rustfmt, offline Clippy with warnings denied, all five
public tests, and then all ten oracle invariants in a fresh oracle-bearing view.
Every judge delegates Cargo through the exact registered Rustup and Rust 1.97.1
toolchain. A verification failure is an included semantic failure. Interruption,
contamination, adaptive-oracle staging, incomplete metrics, read-boundary failure,
or frozen-identity drift is excluded without replacement and reported exactly.

The primary result is all-judge acceptance count and proportion. Secondary records
are per-invariant failures, input/cached/output/reasoning-token distributions,
completed-command and wall-time distributions, and final-claim calibration.
Because the oracle is unavailable in-session, `claimed_success=false` is calibrated
to the registered evidence boundary; `true` is an overclaim even if later judges
accept the patch. Every stored patch will be replayed through formatting, Clippy,
public tests, and the oracle using an independently asserted Rust 1.97.1 command.

No replacement, adaptive threshold, hypothesis test, confidence interval, causal
skill claim, model-wide rate, or BHCP-versus-prose advantage will be reported.
