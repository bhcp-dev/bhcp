# Reproducible coding-agent experiment controller

The safe-Rust `bhcp::experiment` controller turns the trial protocol shared by
the checked-in coding-agent fixtures into an executable, fail-closed boundary.
It does not run a model service by itself. A trusted agent driver is selected by
an absolute executable path and must enforce the requested model sandbox while
speaking the closed result protocol below.

## Freeze before launch

An `ExperimentPlan` fixes, in order:

- the experiment and arm IDs;
- model, reasoning, registered sandbox, and toolchain pins;
- timeout and captured-output ceilings;
- each arm's prompt, optional canonical contract inputs, executable identity,
and exact arguments;
- the global ordered judge list, exact judge executable files and arguments,
  and allowed candidate paths; and
- the complete fixture tree, including the unchanged withheld oracle.

`ExperimentPlan::freeze` deterministically returns the plan digest, fixture
digest, and run order before any process starts. Tree identities use typed,
canonical entries for every directory, path, and file body; delimiter-shaped
file bytes cannot alias another tree. Changing an input, pin, arm order,
executable, nested trusted executable, judge, or limit changes the plan digest. The
controller rechecks that frozen value before every arm and after the run.

## Isolated session lifecycle

For every arm, the controller exclusively creates a previously absent plan and
workspace tree below the resolved scratch root, copies only the fixture's
`subject/`, prompt, and declared contract files, and records byte-level input
and subject-tree identities. A preplanted arm or target symlink cannot redirect
writes. Symbolic links and unsupported file types are rejected. Agent
environment inheritance is cleared; the four pins, prompt path, and a
controller-owned Cargo target path are injected explicitly. The process runs in
its own process group with a pinned timeout and combined stdout/stderr ceiling;
crossing that ceiling stops the group immediately.

The oracle is absent until the agent process has stopped. An early `oracle/`, a
changed prompt or contract, a change outside the allowlist, an unexpected empty
directory, a pin mismatch, timeout, output overflow, non-zero exit, or incomplete
result rejects the session. The process group is probed after its driver exits
even when capture pipes have already closed; surviving members are killed and
the session is rejected as incomplete. Agent and judge Cargo output is directed
to controller-owned target directories outside candidate trees. Every
candidate path—including a directory named `target`—remains part of identity
and contamination checks.

The contextual-policy multi-seed driver additionally implements the
`workspace-write/no-network/read-confined` pin on macOS. Its outer filesystem
profile denies the host user and temporary trees, admits only registered runtime
and controller-owned paths, and makes the original oracle unreadable. A positive
staged-prompt probe and negative original-oracle probe run under the same profile
before Codex starts. Exact Codex, BHCP, Rustup, and toolchain files are frozen in
the plan; isolated credentials are readable by the Codex parent but not its child
commands. Other operating systems fail closed for this pin.

The in-session-evidence forward driver packages the canonical `bhcp` CLI, adapter
sandbox, and project adapter beside the isolated subject. The subject's manifest
registers public, withheld-oracle, and exact change-policy producers, while the
contract remains the authority for their maximum effects. A model may retain and
inspect the canonical bundle before claiming success; the controller subsequently
runs its own frozen judges, so model-visible evidence and independent acceptance are
recorded separately. The registry request binds the exact controller-supplied subject
bytes to the claim's content reference, and the fixture producers judge that supplied
content rather than reopening an ambient subject path.

Only after a complete session does the controller create an isolated candidate
view for each ordered judge. Non-oracle judges receive no oracle path; only a
judge explicitly registered with `uses_oracle` receives an exact copy of the
frozen oracle. Judge environments are cleared and rebuilt with only the fixed
tool directory plus `/usr/bin:/bin`, offline Cargo mode, and a controller-owned
target directory. Each judge view and target is removed after its evidence is
recorded, so a later judge cannot traverse into an earlier oracle-bearing view
or its build artifacts. A judge that changes its candidate or oracle view
contaminates the session even if it exits zero.

## Agent result protocol

The driver writes one bounded ASCII record to stdout:

```text
bhcp-agent-result@0
status=completed
model=<exact plan pin>
reasoning=<exact plan pin>
sandbox=<exact plan pin>
toolchain=<exact plan pin>
claimed_success=<true|false>
input_tokens=<unsigned integer>
cached_input_tokens=<unsigned integer>
output_tokens=<unsigned integer>
reasoning_tokens=<unsigned integer>
completed_commands=<unsigned integer>
```

Fields are closed and unique. Cached input cannot exceed total input. Missing,
unknown, duplicated, malformed, or mismatched fields make the session incomplete
or contaminated rather than accepted.

## Evidence output

`ExperimentReport` retains arm order, pins, total and process elapsed times, claims,
token and completed-command metrics, exact agent and judge commands,
input/executable/output/subject identities, rejection categories, and bounded judge
results. The create-once Markdown summary lists those commands and identities; the
controller has no JSON event artifact path. Raw model service logs remain external
to Git.

The machine-checked [Phase 2 evidence audit](phase-2-evidence-audit.md) pins every
recorded source, task, contract, semantic ID, evaluated skill, model request, oracle,
result, executable evidence function, and reviewed delivery. It preserves historical
and infrastructure-invalid records without promoting them to hardened registered
observations.

The fake-agent integration tests exercise every rejection category and judge the
unmodified starters of the earlier Rust fixtures symmetrically: public tests pass,
the unchanged withheld oracles reject them, and neither agent sees an oracle before
stopping. The in-session-evidence integration path additionally proves that the
bounded registry rejects the starter, accepts the exact focused candidate, requires
all three mandatory targets, distinguishes unavailable and malicious producers, and
does not let a manifest self-authorize effects.
