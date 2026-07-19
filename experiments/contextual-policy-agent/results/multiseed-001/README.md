# Multi-seed run 001: infrastructure exclusion

Date: 2026-07-19

All five preregistered sessions completed and produced distinct one-file candidates, but the fail-closed controller rejected every session as contaminated before parsing its closed metrics record or running its judges. The trusted Codex driver cleared its child environment and failed to forward the controller-owned `CARGO_TARGET_DIR`. Consequently, every model's visible Cargo checks created `subject/target/`, an undeclared workspace path. No session is part of the registered semantic acceptance denominator, no token distribution is available, and none was replaced.

This is an experiment-infrastructure result, not evidence that the candidates or skill failed. The defect was reproduced by an exact fake-Codex regression test before the one-line forwarding fix. A separately preregistered run 002 uses the fixed driver.

For artifact integrity only, all five stored patches were independently replayed from the pinned starter after the controller stopped. Every patch passed all five public tests and all ten unchanged oracle invariants. Those post hoc replays do not convert the controller-rejected attempts into included run-001 observations.

| Attempt | Controller | Registered semantic status | Independent post hoc replay | Candidate blob |
| --- | --- | --- | --- | --- |
| seed-01 | rejected: contaminated | excluded, not replaced | 10/10 oracle invariants | `0b384ef12340c64d6f460bd54e6a4d0f181bd780` |
| seed-02 | rejected: contaminated | excluded, not replaced | 10/10 oracle invariants | `5ae26038f167a730941c65b5e6eb05a9b4d4d09c` |
| seed-03 | rejected: contaminated | excluded, not replaced | 10/10 oracle invariants | `395c841a8299d28e3d856a502503c06b63fc36c9` |
| seed-04 | rejected: contaminated | excluded, not replaced | 10/10 oracle invariants | `9b2eaf3e3f36e605558181bbb931c08b4e770045` |
| seed-05 | rejected: contaminated | excluded, not replaced | 10/10 oracle invariants | `33784bf818a9cbef18906f9d70fc7926c8d9f148` |

The complete frozen identities, elapsed times, rejection categories, and output digests are in [`CONTROLLER.md`](CONTROLLER.md). Raw Codex event streams were reduced in memory and were not retained.
