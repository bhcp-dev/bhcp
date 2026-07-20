# Experiment evidence-generalization-comparative-tenant-policy-seed-02-prose-control

- Plan: `bhcp.hash/sha3-512@0:da60c88fb506ab8873f7510b6de7c0d593d07a0b94bebca56a40288949a8865c192d551712e425821346ccb4e0d042c2cd45c9704f25ccfd8bd8aca8428a2003`
- Fixture: `bhcp.hash/sha3-512@0:b387f7a170f43608d7aedec9c0b5046c54b691ca08cd0a00a5a422c7ea1f63a89e5240ebb1e55dd28456c148554a024b30deca1a45065fa914f7e7763774cc3e`
- Model: `gpt-5.4-mini`
- Reasoning: `medium`
- Sandbox: `workspace-write/no-network/read-confined`
- Toolchain: `codex-cli-0.142.4+rust-1.97.1`
- Run order: prose-control

| Arm | Status | Claimed | Input tokens | Commands | Elapsed ms |
| --- | --- | --- | ---: | ---: | ---: |
| prose-control | rejected (verification-failed) | no | 394090 | 0 | 83192 |

## Arm prose-control

- Result: one or more configured judges rejected the candidate
- Total elapsed: 83192 ms
- Agent elapsed: 81511 ms
- Agent command: `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/target/release/bhcp_codex_experiment_driver` `/Users/sasha/.local/share/mise/installs/codex/0.142.4/codex-aarch64-apple-darwin` `/Users/sasha/.codex` `/Users/sasha/.cargo` `/Users/sasha/.rustup` `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/target/release/bhcp` `/Users/sasha/.rustup/toolchains/1.97.1-aarch64-apple-darwin/bin` `1.97.1` `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/experiments` `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/experiments/policy-resolution-agent/oracle/tests/invariants.rs`
- Subject before: `bhcp.hash/sha3-512@0:48d8ee26a9412bf5d15fe0ecf44bd21fe5d6060de6ac18250eaa7064a8901d456c088616e899b6106ea806e90d773d0e2d7db825973181b5dbead867c9cd9c84`
- Subject after: `bhcp.hash/sha3-512@0:48d8ee26a9412bf5d15fe0ecf44bd21fe5d6060de6ac18250eaa7064a8901d456c088616e899b6106ea806e90d773d0e2d7db825973181b5dbead867c9cd9c84`
- Agent executable: `bhcp.hash/sha3-512@0:ec82cf095819683495ce9727232bca350e1a030c542667f4788324b8ddd894ded6ea0eb14f554147dfecfaafd8c9372755bb25f69106d1918efc89bba57577b1`
- Agent stdout: `bhcp.hash/sha3-512@0:a41951b9f6d16a89d5d87c34ef73803641556ddfe721360aba7cf680b0dfdb7899445af6646976cb6a3b6875e94e25bc91298def0c5b61b934396134b9719c08`
- Agent stderr: `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
- Frozen inputs: 12
- Tokens: input 394090, cached 350208, output 3955, reasoning 3124
- Completed commands: 0
- Input `COMPARATIVE_PROSE_PROMPT.md`: `bhcp.hash/sha3-512@0:cbcb9f808d609b6a432d3ecb1ee110df319dc010c06ec59646602e90d1d4b156c5b960824cffe5f0df732756ae7efd18f4dbcbaf1dce303cb00b9452cb368486`
- Input `PROSE_TASK.md`: `bhcp.hash/sha3-512@0:dbbb91df43fe7ed9ba93b5cc2e99367ceed7b878409fec85bc7666faca65e34f2db854bf0adb4707f62fdb62925067b3adb34f0c37b885dd7c88f4fb3bfeda68`
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
- Judge `format`: accepted (exit Some(0), 61 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `fmt` `--check` `--manifest-path` `subject/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
- Judge `clippy`: accepted (exit Some(0), 186 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `clippy` `--offline` `--manifest-path` `subject/Cargo.toml` `--all-targets` `--` `-D` `warnings`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:662f8ef6e48fa4525ce13345ba720a08e3a5e2612dbdb604d3932acab451bf3ab37da1e84ae13456dcb6683fb692c9d64e98b9abc06514f067a2caf376b192da`
- Judge `public`: accepted (exit Some(0), 734 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `test` `--offline` `--manifest-path` `subject/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:5fc880d880472d8be4c2159f56dd9d10ddd21efd88e8f6c31958df27a0f1cc9f0b50c6d61af7aa4129ac35d3de8b6daac604841b0200fdd92a4e25703218ddbd`; stderr `bhcp.hash/sha3-512@0:e12a0c4edde193b30caec56a3c9b93181efa85b7b5ae05fde8a641aee5ee4e7cfa3fe22661007639613f8cc064b87d86d84b9f5973243da68630c8b691bf329d`
- Judge `oracle`: rejected (exit Some(101), 657 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `test` `--offline` `--manifest-path` `oracle/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:e59b034cb8e035aaef3ee13b859cd4d7777ee3f85e25c0ac7fb05ea5b90a25544d4a38549912f2b82284be365b689bd08e0b2825c218de81301535d02912130d`; stderr `bhcp.hash/sha3-512@0:d80b9244839f691d0f5bdbf1e65cd68e9e52ced7b37498c0368bcf14e3bacb3096042f22faea111efd9825f0fea8ea1fb799b947a95f6967a4d177fa4dfe3d44`
- Judge `change-policy`: accepted (exit Some(0), 3 ms); command `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/target/release/evidence_generalization_comparative_policy`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
