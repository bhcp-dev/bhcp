# Experiment evidence-generalization-comparative-contextual-policy-seed-02-prose-control

- Plan: `bhcp.hash/sha3-512@0:a94f1d795b2639cde8ad9d168d69abe2d4a3656b02c66ef693d4bb4b84c0ced9a291504cc9715b7e05b37b64163060289e6f6b537f2a44ba17973caad78e8170`
- Fixture: `bhcp.hash/sha3-512@0:88024a336e195eb5e15b29d90cbcfcb15017f0e4199f8fc094dacb3e3b82af542a40873c6d9d7f0c3dd0bf8282749b7341f0c44f72fedaa05ceb4e258dcc9755`
- Model: `gpt-5.4-mini`
- Reasoning: `medium`
- Sandbox: `workspace-write/no-network/read-confined`
- Toolchain: `codex-cli-0.142.4+rust-1.97.1`
- Run order: prose-control

| Arm | Status | Claimed | Input tokens | Commands | Elapsed ms |
| --- | --- | --- | ---: | ---: | ---: |
| prose-control | rejected (verification-failed) | no | 233098 | 0 | 72732 |

## Arm prose-control

- Result: one or more configured judges rejected the candidate
- Total elapsed: 72732 ms
- Agent elapsed: 70417 ms
- Agent command: `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/target/release/bhcp_codex_experiment_driver` `/Users/sasha/.local/share/mise/installs/codex/0.142.4/codex-aarch64-apple-darwin` `/Users/sasha/.codex` `/Users/sasha/.cargo` `/Users/sasha/.rustup` `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/target/release/bhcp` `/Users/sasha/.rustup/toolchains/1.97.1-aarch64-apple-darwin/bin` `1.97.1` `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/experiments` `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/experiments/contextual-policy-agent/oracle/tests/invariants.rs`
- Subject before: `bhcp.hash/sha3-512@0:2c4b358732dcc5537d70c870ac0ad3464beb754bfbdfef47d3420fb27fb13815f9d688325dceb87bdcac74fe7dd17fde8831cfc78699b4952183fdb469af1a0b`
- Subject after: `bhcp.hash/sha3-512@0:2c4b358732dcc5537d70c870ac0ad3464beb754bfbdfef47d3420fb27fb13815f9d688325dceb87bdcac74fe7dd17fde8831cfc78699b4952183fdb469af1a0b`
- Agent executable: `bhcp.hash/sha3-512@0:ec82cf095819683495ce9727232bca350e1a030c542667f4788324b8ddd894ded6ea0eb14f554147dfecfaafd8c9372755bb25f69106d1918efc89bba57577b1`
- Agent stdout: `bhcp.hash/sha3-512@0:984c769e10af6351578234994a1328d5aa520d4522f78dcc66a0c8dbafc6d7dfc57ddc7b8bfa8c680a5418fe3cf8e544feb22a11d71724fdf5b8447c943131dd`
- Agent stderr: `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
- Frozen inputs: 10
- Tokens: input 233098, cached 204160, output 3229, reasoning 2627
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
- Judge `format`: accepted (exit Some(0), 177 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `fmt` `--check` `--manifest-path` `subject/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
- Judge `clippy`: accepted (exit Some(0), 413 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `clippy` `--offline` `--manifest-path` `subject/Cargo.toml` `--all-targets` `--` `-D` `warnings`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:8c98ac0bfc7d1c8a7682fe80f2380fd9285d4581c0ddee31602748c1177f32d4666bb4677f6e4c3feea1fd040d5b89ccb385d28ba89fbde2db36849acc2d36dc`
- Judge `public`: accepted (exit Some(0), 941 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `test` `--offline` `--manifest-path` `subject/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:84335e37c6c9218f9016b1eb89f3b5acfccaebaf362937d732c12314c77bc621523c1c7fd19e24eb8b000f5f48cc3479c2393221e5cb270146c6c3f4b35a2e45`; stderr `bhcp.hash/sha3-512@0:38b8c43c711d611b464f4793c18fdf7367868d0675e80a9e254832966af2606834338f683cedbbca68e23ecb86222999762d874a391486b841318cf001b82b3e`
- Judge `oracle`: rejected (exit Some(101), 739 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `test` `--offline` `--manifest-path` `oracle/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:d6b542e830640b090d511e6921a09a207886f8c32d43e7647694a5ed57156e5da8001998a081c4e673ba9d41ce08f39658e1f29dbd0068df4725d767ca1addc6`; stderr `bhcp.hash/sha3-512@0:17cdc8d2c1c8d25dedf75e94c96c74b86eb58e7650476368daac9b190da1837b4d2e5a99737ad796ad7f557f04ba003ca525b7ad21ed527c6e9eababa7f7efd1`
- Judge `change-policy`: accepted (exit Some(0), 5 ms); command `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/target/release/evidence_generalization_comparative_policy`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
