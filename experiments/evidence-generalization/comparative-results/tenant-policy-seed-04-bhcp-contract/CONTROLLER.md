# Experiment evidence-generalization-comparative-tenant-policy-seed-04-bhcp-contract

- Plan: `bhcp.hash/sha3-512@0:70d63caaff8efd659743431e0c70c2171a9cdf63e151f62432baa75362651eef7c178b6289a83223fbbaa681eddc83ac96c9f1ffd848f7af4eaf5d7117e17b99`
- Fixture: `bhcp.hash/sha3-512@0:b387f7a170f43608d7aedec9c0b5046c54b691ca08cd0a00a5a422c7ea1f63a89e5240ebb1e55dd28456c148554a024b30deca1a45065fa914f7e7763774cc3e`
- Model: `gpt-5.4-mini`
- Reasoning: `medium`
- Sandbox: `workspace-write/no-network/read-confined`
- Toolchain: `codex-cli-0.142.4+rust-1.97.1`
- Run order: bhcp-contract

| Arm | Status | Claimed | Input tokens | Commands | Elapsed ms |
| --- | --- | --- | ---: | ---: | ---: |
| bhcp-contract | rejected (verification-failed) | no | 429638 | 0 | 95014 |

## Arm bhcp-contract

- Result: one or more configured judges rejected the candidate
- Total elapsed: 95014 ms
- Agent elapsed: 92942 ms
- Agent command: `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/target/release/bhcp_codex_experiment_driver` `/Users/sasha/.local/share/mise/installs/codex/0.142.4/codex-aarch64-apple-darwin` `/Users/sasha/.codex` `/Users/sasha/.cargo` `/Users/sasha/.rustup` `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/target/release/bhcp` `/Users/sasha/.rustup/toolchains/1.97.1-aarch64-apple-darwin/bin` `1.97.1` `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/experiments` `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/experiments/policy-resolution-agent/oracle/tests/invariants.rs`
- Subject before: `bhcp.hash/sha3-512@0:48d8ee26a9412bf5d15fe0ecf44bd21fe5d6060de6ac18250eaa7064a8901d456c088616e899b6106ea806e90d773d0e2d7db825973181b5dbead867c9cd9c84`
- Subject after: `bhcp.hash/sha3-512@0:48d8ee26a9412bf5d15fe0ecf44bd21fe5d6060de6ac18250eaa7064a8901d456c088616e899b6106ea806e90d773d0e2d7db825973181b5dbead867c9cd9c84`
- Agent executable: `bhcp.hash/sha3-512@0:ec82cf095819683495ce9727232bca350e1a030c542667f4788324b8ddd894ded6ea0eb14f554147dfecfaafd8c9372755bb25f69106d1918efc89bba57577b1`
- Agent stdout: `bhcp.hash/sha3-512@0:e4fabb13d0730b81cb5292e4ec5f7161ddfa840bacd09c5dfe59d3948e87f77c32d3565b017b745ad5f6b99d6ff0044e5efea7c16c0199de5558b5b38c977149`
- Agent stderr: `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
- Frozen inputs: 14
- Tokens: input 429638, cached 386432, output 4549, reasoning 3699
- Completed commands: 0
- Input `COMPARATIVE_BHCP_PROMPT.md`: `bhcp.hash/sha3-512@0:9bdcc4d2b2120e48f06a24a887f4403faa49c1af6b2f4e5f0e432b392662455b56f980ea413b0feac0ec64456b354affdf34ef3802388bcc10fd947246887231`
- Input `TASK.md`: `bhcp.hash/sha3-512@0:06148d50cc40233d1de9dbad9d129da472ed231e32d44f22dbcfe93c651e90d6ac1109f1bb44de4bb61559251aa2a57e82c48e8c86bf72cabf259fb9c8586461`
- Input `contract.bhcp`: `bhcp.hash/sha3-512@0:99b89678ad18c2553294822f61f85325add8f89881be5cff66027b5bd90f541d357213b53395e22818ba47e3cc4c4fa93b8d6fd81c34156a96ef209bf3dbcb9a`
- Input `contract.semantic-id`: `bhcp.hash/sha3-512@0:382fc0bb10a3075e31312b49bc849879c37634eab34f740245b0f6c3065fc4b0eafd40606a3a40386eb9f7b09c413ff5e87369c4b4124c002ea9bdc047aded21`
- Input `subject`: `directory`
- Input `subject/.gitignore`: `bhcp.hash/sha3-512@0:9be9f39fca13920266e2bee5474bedf4d96abe85cd647f98406185f226130f4cbbaae2d367116c73ce50d09faab8091d8c9a1f275001fd58167f8863b66495da`
- Input `subject/.mise.toml`: `bhcp.hash/sha3-512@0:3214b7551815c603e93136d4907bc85e0a129335eb9a3ff48ef9da25f5464ae6807e3e55d308a66031577a251be42ecde76faf07142fa4863ac278bd0d734992`
- Input `subject/Cargo.lock`: `bhcp.hash/sha3-512@0:68cac31af1b9f690a2011b08d7a99e4c88aaea90a9d146ac4c7220fcb9b820292e0ecd7608cfa5bf99e8724bc8844cb66671e8a6c60a3fea7b240af7911cc523`
- Input `subject/Cargo.toml`: `bhcp.hash/sha3-512@0:ec6881ebb715e1fc0067b682ea765bba52dc65d987b27e4ff1f8ec503dc8e0083dd644d69e2daed9c6ddc28d191f14e1dc5045c10f32f663c21abe3c6f866e2e`
- Input `subject/README.md`: `bhcp.hash/sha3-512@0:7131849de9a8386a156c5a5d529c4a83328261ed618e2fff41fa008de0bf98182a75abdec934acc3f9f760c19e2cd27b02c80a8655318513dca6c6f556e8f602`
- Input `subject/src`: `directory`
- Input `subject/src/lib.rs`: `bhcp.hash/sha3-512@0:88cc56ed939073ac27ec8c828320f5a8b43579f33ed7fdb470dfa5375f62c9501dd8fd4a9e175e772adedcbd6786e7f98c2c9a33fdc17026c15a5ce82b11fd39`
- Input `subject/tests`: `directory`
- Input `subject/tests/public.rs`: `bhcp.hash/sha3-512@0:a10274c7136b09951914c394fcce5b13c2e619d3c926b83357239be8e1a1111bb4d0209083a1ea90816ada98a5d6452cd24560b18d9b1f51355a6e43864553a4`
- Judge `format`: accepted (exit Some(0), 185 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `fmt` `--check` `--manifest-path` `subject/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
- Judge `clippy`: accepted (exit Some(0), 386 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `clippy` `--offline` `--manifest-path` `subject/Cargo.toml` `--all-targets` `--` `-D` `warnings`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:6a8be2e013c2bdfc9cc889e71c7273964f2e8f536d6bc8165c2350f6c03f4115814c0f3e4a69bdc3a0889cc95594b2326f7c34a133705a6cd2a2830fb1caa60e`
- Judge `public`: accepted (exit Some(0), 801 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `test` `--offline` `--manifest-path` `subject/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:f47bcc586e2d3e60edf9849f9d7a36fccaabd7efd1e0ab7a786e31c8f3f6e763d5bc8ceb679985e4280f6b4ad5d68f445d38e9776317fcc1131586fd12368e42`; stderr `bhcp.hash/sha3-512@0:8eeca2773321774dd7bedd08123aa276d7a0a63b38b77d424ecdcea3105b7683843abe1584ac7423727edbdaa521ea0c3f3a61e6a8386436ceee13877686f4d7`
- Judge `oracle`: rejected (exit Some(101), 647 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `test` `--offline` `--manifest-path` `oracle/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:8ed41069d30cf4a420beafa53151c76f72168d07bdea0711226389d7c54df2e13f45b7307dd5e4db8a9a0c626a4043271a13747be2974837ed13ce91ebc2a40d`; stderr `bhcp.hash/sha3-512@0:fd1933805fd58e70d22f9193f974d9b726024ce8ed7ebf05df7d02b162b5741fff677197e1a35147d145a8a86dd6f33cfcbceb6f77ab195ac0511441f2f1f473`
- Judge `change-policy`: accepted (exit Some(0), 5 ms); command `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/target/release/evidence_generalization_comparative_policy`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
