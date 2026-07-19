# Multi-seed run 003: read-boundary infrastructure exclusion

Date: 2026-07-19

Run 003 is **five unreplaced infrastructure exclusions**, not a semantic sample.
Every fixed arm exited before a model turn, with no metrics, claim, edit, or judge.
The operating-system profile denied the original oracle as intended, but it also
denied metadata traversal on temporary ancestor directories. Codex canonicalizes
its isolated `CODEX_HOME` during startup, so the driver exited before submitting a
model request.

| Arm | Registered result | Elapsed | Candidate |
| --- | --- | ---: | --- |
| seed-01 | excluded: interrupted | 568 ms | unchanged |
| seed-02 | excluded: interrupted | 93 ms | unchanged |
| seed-03 | excluded: interrupted | 90 ms | unchanged |
| seed-04 | excluded: interrupted | 90 ms | unchanged |
| seed-05 | excluded: interrupted | 87 ms | unchanged |

The frozen plan was
`bhcp.hash/sha3-512@0:e465ed3ec75aee86343f5cca78d2c8ab26b14a8b665bed2ce9b1679b52cfb2c5fdfb9c204c62ca090e88f1bbc034fc8038b116d3696d2e0cb94032e6a0efac68`;
the fixture was
`bhcp.hash/sha3-512@0:0a91fe1769dc5f1a0e86042a60a388e43a2bccb90503298643f62ee6bedab536fb5527cfc5686e23f5926a12f490f1d9c68efffd06c88dc14982673ac8759d61`.
The complete per-arm frozen inputs, commands, identities, and rejection records are
in [`CONTROLLER.md`](CONTROLLER.md).

The run exposed a second reporting defect: an unchanged candidate makes `git diff`
exit zero, which the patch writer treated as an error after it had written the
controller report. A red-to-green unit test now requires an empty patch artifact
for that case. The five empty patch files preserve the exact unchanged outcomes.

After the registered run closed, a fake-Codex regression required isolated-home
canonicalization. The profile was changed to allow only metadata reads on temporary
ancestors while retaining data-read denial outside the staged/runtime paths. A real
Codex smoke request then completed through that profile with no tools or edits.
That correction is separately registered as run 004; run 003 is not rerun or
relabeled.
