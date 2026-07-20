# Experiment evidence-generalization-comparative-contextual-policy-seed-04-prose-control

- Plan: `bhcp.hash/sha3-512@0:0849d9a55e04490687f7ecc69a846c0b96ae431779c1e76ea33bbb84383d3e8e88b986df1fc1d4dda02dacf38e7eb043c58108273f8ca82891d124808dcf6ef5`
- Fixture: `bhcp.hash/sha3-512@0:88024a336e195eb5e15b29d90cbcfcb15017f0e4199f8fc094dacb3e3b82af542a40873c6d9d7f0c3dd0bf8282749b7341f0c44f72fedaa05ceb4e258dcc9755`
- Model: `gpt-5.4-mini`
- Reasoning: `medium`
- Sandbox: `workspace-write/no-network/read-confined`
- Toolchain: `codex-cli-0.142.4+rust-1.97.1`
- Run order: prose-control

| Arm | Status | Claimed | Input tokens | Commands | Elapsed ms |
| --- | --- | --- | ---: | ---: | ---: |
| prose-control | rejected (verification-failed) | no | 174276 | 0 | 62100 |

## Arm prose-control

- Result: one or more configured judges rejected the candidate
- Total elapsed: 62100 ms
- Agent elapsed: 59917 ms
- Agent command: `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/target/release/bhcp_codex_experiment_driver` `/Users/sasha/.local/share/mise/installs/codex/0.142.4/codex-aarch64-apple-darwin` `/Users/sasha/.codex` `/Users/sasha/.cargo` `/Users/sasha/.rustup` `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/target/release/bhcp` `/Users/sasha/.rustup/toolchains/1.97.1-aarch64-apple-darwin/bin` `1.97.1` `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/experiments` `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/experiments/contextual-policy-agent/oracle/tests/invariants.rs`
- Subject before: `bhcp.hash/sha3-512@0:2c4b358732dcc5537d70c870ac0ad3464beb754bfbdfef47d3420fb27fb13815f9d688325dceb87bdcac74fe7dd17fde8831cfc78699b4952183fdb469af1a0b`
- Subject after: `bhcp.hash/sha3-512@0:2c4b358732dcc5537d70c870ac0ad3464beb754bfbdfef47d3420fb27fb13815f9d688325dceb87bdcac74fe7dd17fde8831cfc78699b4952183fdb469af1a0b`
- Agent executable: `bhcp.hash/sha3-512@0:ec82cf095819683495ce9727232bca350e1a030c542667f4788324b8ddd894ded6ea0eb14f554147dfecfaafd8c9372755bb25f69106d1918efc89bba57577b1`
- Agent stdout: `bhcp.hash/sha3-512@0:11e753d62d7d570b3f1ee662f5662a23e77000d13394ff47c88df3f1774ccaf926d1fbf3f2710d17eb212718232605dde363aa01e1eaea906f74c7b6e107a222`
- Agent stderr: `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
- Frozen inputs: 10
- Tokens: input 174276, cached 144640, output 3097, reasoning 2478
- Completed commands: 0
- Input `COMPARATIVE_PROSE_PROMPT.md`: `bhcp.hash/sha3-512@0:cbcb9f808d609b6a432d3ecb1ee110df319dc010c06ec59646602e90d1d4b156c5b960824cffe5f0df732756ae7efd18f4dbcbaf1dce303cb00b9452cb368486`
- Input `PROSE_TASK.md`: `bhcp.hash/sha3-512@0:a19f43d0cf7d3f35484038126e0bc453c0f577c9a6020dfdf9d22a25276a1b948dd9201445e40f049ff5588b25fc42408e4ec9d7d968e4012037735f54e29996`
- Input `subject`: `directory`
- Input `subject/Cargo.lock`: `bhcp.hash/sha3-512@0:dc2e3465ee77e06935ddbf01ddd34edf24304ef37ad1dfd3ce8c69f32c54b93f292e4e42c32fa1beca77fcf7110f8dbed034a2c2ab38d808b8452cc0f321d9e0`
- Input `subject/Cargo.toml`: `bhcp.hash/sha3-512@0:eba8081e69f405f273b2b477cfb629cb2b2649080829473d7c738f0dec8151d10a89211c6eca4a6776e604c2c0e0992f28ece454210a1e3b7612fa62ad21782b`
- Input `subject/README.md`: `bhcp.hash/sha3-512@0:a6e4fb450a76aaa0ac75cd1189c301a5eaa803d0fa2a206c19ccafa7ab19ed679391dd72dfd71f84f347035eb52f6029687a5741b7ece015da06829fb5b9a6f1`
- Input `subject/src`: `directory`
- Input `subject/src/lib.rs`: `bhcp.hash/sha3-512@0:fbd107e18aa1420486f69f71888dac5c0e641c2df8eea5a010eeeea5f928c4bfcbe764a03043575d3f1c697685d88ff2b329a75a3cd6a56d7ce2e0729df93b93`
- Input `subject/tests`: `directory`
- Input `subject/tests/public.rs`: `bhcp.hash/sha3-512@0:749d44720b6120c732f9d75e1882ee0c933c65327e939fb028b60580dcf0b8a41d7029680556b12c1405c6485f6552a3a8152d32e10ea48c845d69a6137b911d`
- Judge `format`: accepted (exit Some(0), 183 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `fmt` `--check` `--manifest-path` `subject/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
- Judge `clippy`: accepted (exit Some(0), 379 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `clippy` `--offline` `--manifest-path` `subject/Cargo.toml` `--all-targets` `--` `-D` `warnings`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:44eb478f8ab61f30dd3160f27bf15fb9e892d933731068f320d2bc7d395d926fcc1a0d456a39cfd0512c5c5eaae7e44f1859ca8030d4e802fbce268bc07106d0`
- Judge `public`: accepted (exit Some(0), 905 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `test` `--offline` `--manifest-path` `subject/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:128df1d2c062942b0fce80e2b58810ee54df894747bf098d43ca24ab17815b9b9cb297bd3225402c9949a343a250b4ad758e53fe8c281f4c77ecaa285e2eefe6`; stderr `bhcp.hash/sha3-512@0:197a78c3b6535f794ecb3fb306e1c9ec893e961f2248aaaec882ef4553585cbdbfa77f02965018ef84506b0a4ccd4067d3339d78152dba3d363c4aa38aec36e2`
- Judge `oracle`: rejected (exit Some(101), 668 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `test` `--offline` `--manifest-path` `oracle/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:966e1db043248f0a02be804b72836083117b114535786982e102694a16f47e7d82939f02c05b2bbc0f0c984030b2a71d537e27ab09cddbbaccdfb1775c991e7e`; stderr `bhcp.hash/sha3-512@0:dbabeaaab84d22b4bf5d07d5d9dcc9c21abb3aba19fe8c7ab34707c4137133e4c9b7e9a385060e1875a8ccc539ce7ea984eac57807eef45cad56d35a377087fd`
- Judge `change-policy`: accepted (exit Some(0), 3 ms); command `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/target/release/evidence_generalization_comparative_policy`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
