# Experiment evidence-generalization-comparative-contextual-policy-seed-01-prose-control

- Plan: `bhcp.hash/sha3-512@0:ee9385db527d53eed5ef9a145eb08718e8dadd1744210d9a09188b0ad007ed519fec1984005e27faaca4f397614204aa40ef0b7b5d27786a651826e3536a35c5`
- Fixture: `bhcp.hash/sha3-512@0:88024a336e195eb5e15b29d90cbcfcb15017f0e4199f8fc094dacb3e3b82af542a40873c6d9d7f0c3dd0bf8282749b7341f0c44f72fedaa05ceb4e258dcc9755`
- Model: `gpt-5.4-mini`
- Reasoning: `medium`
- Sandbox: `workspace-write/no-network/read-confined`
- Toolchain: `codex-cli-0.142.4+rust-1.97.1`
- Run order: prose-control

| Arm | Status | Claimed | Input tokens | Commands | Elapsed ms |
| --- | --- | --- | ---: | ---: | ---: |
| prose-control | rejected (verification-failed) | no | 383364 | 0 | 78467 |

## Arm prose-control

- Result: one or more configured judges rejected the candidate
- Total elapsed: 78467 ms
- Agent elapsed: 76416 ms
- Agent command: `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/target/release/bhcp_codex_experiment_driver` `/Users/sasha/.local/share/mise/installs/codex/0.142.4/codex-aarch64-apple-darwin` `/Users/sasha/.codex` `/Users/sasha/.cargo` `/Users/sasha/.rustup` `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/target/release/bhcp` `/Users/sasha/.rustup/toolchains/1.97.1-aarch64-apple-darwin/bin` `1.97.1` `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/experiments` `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/experiments/contextual-policy-agent/oracle/tests/invariants.rs`
- Subject before: `bhcp.hash/sha3-512@0:2c4b358732dcc5537d70c870ac0ad3464beb754bfbdfef47d3420fb27fb13815f9d688325dceb87bdcac74fe7dd17fde8831cfc78699b4952183fdb469af1a0b`
- Subject after: `bhcp.hash/sha3-512@0:2c4b358732dcc5537d70c870ac0ad3464beb754bfbdfef47d3420fb27fb13815f9d688325dceb87bdcac74fe7dd17fde8831cfc78699b4952183fdb469af1a0b`
- Agent executable: `bhcp.hash/sha3-512@0:ec82cf095819683495ce9727232bca350e1a030c542667f4788324b8ddd894ded6ea0eb14f554147dfecfaafd8c9372755bb25f69106d1918efc89bba57577b1`
- Agent stdout: `bhcp.hash/sha3-512@0:3db748a1fbd7a2e958e735f64ed6ddbc06a5c1c020fb772c37bb536cada6088af8dc0b09e2d3a559564bfd4101f0fad71cf4a5114c1f44be16f80e8e745c222a`
- Agent stderr: `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
- Frozen inputs: 10
- Tokens: input 383364, cached 341376, output 3541, reasoning 2467
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
- Judge `format`: accepted (exit Some(0), 145 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `fmt` `--check` `--manifest-path` `subject/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
- Judge `clippy`: accepted (exit Some(0), 377 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `clippy` `--offline` `--manifest-path` `subject/Cargo.toml` `--all-targets` `--` `-D` `warnings`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:ebfa92b899147d8e7ae2d7a88dc5125ccf36f0accc8b4a81728243b89f7ac0efd88247165060bbb822bbec8ad69d5ded6f6a4648c10ecf10dd13d82a89003212`
- Judge `public`: accepted (exit Some(0), 811 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `test` `--offline` `--manifest-path` `subject/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:f11fd5cbf64e2a79f8286c914fbc09a9f40c80ae0097af40bbcfa21369c7a66930cf74fbefda610078029f01aff3b06458c775ec962e7d09673031b4156c0fc5`; stderr `bhcp.hash/sha3-512@0:460d7f07614375e1362f01d89d4d2c97b3e89036ce239f478b1564597847f78874538149ab8932f2353dde39b303b4468cc9bd6b388c9d754c159fc703b9d4db`
- Judge `oracle`: rejected (exit Some(101), 671 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `test` `--offline` `--manifest-path` `oracle/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:126a3edfd7a465df30991862cb8a3fb09c541928d0c0623b0236568f82c64e678d66a9f0729d8442522d91c54afe3acf40e405b1d0097564b27eefaa80219b18`; stderr `bhcp.hash/sha3-512@0:d4fa815b006fadd2d0f7441d24af4bca4d26489e002003372f6e91edfbfa61481af8d5d5f7eac9e6116ac8170392ece5d2d004221d1eece39aa9f43b831a05d4`
- Judge `change-policy`: accepted (exit Some(0), 5 ms); command `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/target/release/evidence_generalization_comparative_policy`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
