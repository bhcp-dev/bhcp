# Reproducible coding-agent experiment controller

The safe-Rust `bhcp::experiment` controller turns the trial protocol shared by
the checked-in coding-agent fixtures into an executable, fail-closed boundary.
It does not run a model service by itself. A trusted agent driver is selected by
an absolute executable path and must enforce the requested model sandbox while
speaking the closed result protocol below.

## Freeze before launch

An `ExperimentPlan` fixes, in order:

- the experiment and arm IDs;
- model, reasoning, `workspace-write/no-network` sandbox, and toolchain pins;
- timeout and captured-output ceilings;
- each arm's prompt, optional canonical contract inputs, executable identity,
and exact arguments;
- the global ordered judge list, exact judge executable files and arguments,
  and allowed candidate paths; and
- the complete fixture tree, including the unchanged withheld oracle.

`ExperimentPlan::freeze` deterministically returns the plan digest, fixture
digest, and run order before any process starts. Changing an input, pin, arm
order, executable, judge, or limit changes the plan digest. The controller
rechecks that frozen value before every arm and after the run.

## Isolated session lifecycle

For every arm, the controller creates a new absent workspace, copies only the
fixture's `subject/`, prompt, and declared contract files, and records byte-level
input and subject-tree identities. Symbolic links and unsupported file types are
rejected. Agent environment inheritance is cleared; the four pins and prompt
path are injected explicitly. The process runs in its own process group with a
pinned timeout and bounded stdout/stderr capture.

The oracle is absent until the agent process has stopped. An early `oracle/`, a
changed prompt or contract, a change outside the allowlist, an unexpected empty
directory, a pin mismatch, timeout, output overflow, non-zero exit, or incomplete
result rejects the session. A process group that remains active after its driver
exits is killed and rejected as incomplete. Generated Cargo `target/` output is ignored for
contamination purposes and remains outside repository artifacts.

Only after a complete session does the controller copy the exact frozen oracle
and run the same ordered judges for every arm. At least one judge must be explicitly
registered as using that oracle. Cargo is forced offline and uses
a controller-owned target directory outside the candidate subject. A judge that
changes the candidate or oracle contaminates the session even if it exits zero.

## Agent result protocol

The driver writes one bounded ASCII record to stdout:

```text
bhcp-agent-result@0
status=completed
model=<exact plan pin>
reasoning=<exact plan pin>
sandbox=workspace-write/no-network
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

The fake-agent integration tests exercise every rejection category and judge the
unmodified starters of both existing Rust fixtures symmetrically: public tests
pass, the unchanged withheld oracles reject them, and neither agent sees an oracle
before stopping.
