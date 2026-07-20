# Experiment evidence-generalization-comparative-contextual-policy-seed-04-bhcp-contract

- Plan: `bhcp.hash/sha3-512@0:aba416fa3b2164e227ebd5cfe1f64d7d71b2ea65abdfda10f8756966b7c70188c5ce8ddca93c019abb65e7eeca4967ae820dd75eca27f5c0a63aa992eea75461`
- Fixture: `bhcp.hash/sha3-512@0:88024a336e195eb5e15b29d90cbcfcb15017f0e4199f8fc094dacb3e3b82af542a40873c6d9d7f0c3dd0bf8282749b7341f0c44f72fedaa05ceb4e258dcc9755`
- Model: `gpt-5.4-mini`
- Reasoning: `medium`
- Sandbox: `workspace-write/no-network/read-confined`
- Toolchain: `codex-cli-0.142.4+rust-1.97.1`
- Run order: bhcp-contract

| Arm | Status | Claimed | Input tokens | Commands | Elapsed ms |
| --- | --- | --- | ---: | ---: | ---: |
| bhcp-contract | rejected (verification-failed) | no | 307356 | 0 | 74258 |

## Arm bhcp-contract

- Result: one or more configured judges rejected the candidate
- Total elapsed: 74258 ms
- Agent elapsed: 72029 ms
- Agent command: `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/target/release/bhcp_codex_experiment_driver` `/Users/sasha/.local/share/mise/installs/codex/0.142.4/codex-aarch64-apple-darwin` `/Users/sasha/.codex` `/Users/sasha/.cargo` `/Users/sasha/.rustup` `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/target/release/bhcp` `/Users/sasha/.rustup/toolchains/1.97.1-aarch64-apple-darwin/bin` `1.97.1` `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/experiments` `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/experiments/contextual-policy-agent/oracle/tests/invariants.rs`
- Subject before: `bhcp.hash/sha3-512@0:2c4b358732dcc5537d70c870ac0ad3464beb754bfbdfef47d3420fb27fb13815f9d688325dceb87bdcac74fe7dd17fde8831cfc78699b4952183fdb469af1a0b`
- Subject after: `bhcp.hash/sha3-512@0:2c4b358732dcc5537d70c870ac0ad3464beb754bfbdfef47d3420fb27fb13815f9d688325dceb87bdcac74fe7dd17fde8831cfc78699b4952183fdb469af1a0b`
- Agent executable: `bhcp.hash/sha3-512@0:ec82cf095819683495ce9727232bca350e1a030c542667f4788324b8ddd894ded6ea0eb14f554147dfecfaafd8c9372755bb25f69106d1918efc89bba57577b1`
- Agent stdout: `bhcp.hash/sha3-512@0:b9528e880d596dd12ce20c406506a1fdb6789d1c831161e39697113edc7984c50c85bf7f04bcd4c468ce8676966b148e2142a833958b290325aaeeba332caf9f`
- Agent stderr: `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
- Frozen inputs: 12
- Tokens: input 307356, cached 262016, output 3643, reasoning 2923
- Completed commands: 0
- Input `COMPARATIVE_BHCP_PROMPT.md`: `bhcp.hash/sha3-512@0:9bdcc4d2b2120e48f06a24a887f4403faa49c1af6b2f4e5f0e432b392662455b56f980ea413b0feac0ec64456b354affdf34ef3802388bcc10fd947246887231`
- Input `TASK.md`: `bhcp.hash/sha3-512@0:0231ab12f5a147baa1c2c92fb262749e94acadeef0bcc665ac780048a424b326ecef294e16cef5ec45388245fcb04a14b1788826f7357e742010a21de8dbf9d1`
- Input `contract.bhcp`: `bhcp.hash/sha3-512@0:500df232b642efabea4ef362487f676b629c5874e479f78f4e980f54445980a30e1363160b787885e181c136785d29d987f1fd34bd739f4b19b3ef4451806976`
- Input `contract.semantic-id`: `bhcp.hash/sha3-512@0:d45d4926442ca393efcf5bd5bf335fda3acfef0173f8f1bd1d6baf7f18f3087676cb074e814c17725dec089558223a622420738fa9a9fcaf9983e392e38c7b3e`
- Input `subject`: `directory`
- Input `subject/Cargo.lock`: `bhcp.hash/sha3-512@0:dc2e3465ee77e06935ddbf01ddd34edf24304ef37ad1dfd3ce8c69f32c54b93f292e4e42c32fa1beca77fcf7110f8dbed034a2c2ab38d808b8452cc0f321d9e0`
- Input `subject/Cargo.toml`: `bhcp.hash/sha3-512@0:eba8081e69f405f273b2b477cfb629cb2b2649080829473d7c738f0dec8151d10a89211c6eca4a6776e604c2c0e0992f28ece454210a1e3b7612fa62ad21782b`
- Input `subject/README.md`: `bhcp.hash/sha3-512@0:a6e4fb450a76aaa0ac75cd1189c301a5eaa803d0fa2a206c19ccafa7ab19ed679391dd72dfd71f84f347035eb52f6029687a5741b7ece015da06829fb5b9a6f1`
- Input `subject/src`: `directory`
- Input `subject/src/lib.rs`: `bhcp.hash/sha3-512@0:fbd107e18aa1420486f69f71888dac5c0e641c2df8eea5a010eeeea5f928c4bfcbe764a03043575d3f1c697685d88ff2b329a75a3cd6a56d7ce2e0729df93b93`
- Input `subject/tests`: `directory`
- Input `subject/tests/public.rs`: `bhcp.hash/sha3-512@0:749d44720b6120c732f9d75e1882ee0c933c65327e939fb028b60580dcf0b8a41d7029680556b12c1405c6485f6552a3a8152d32e10ea48c845d69a6137b911d`
- Judge `format`: accepted (exit Some(0), 205 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `fmt` `--check` `--manifest-path` `subject/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
- Judge `clippy`: accepted (exit Some(0), 493 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `clippy` `--offline` `--manifest-path` `subject/Cargo.toml` `--all-targets` `--` `-D` `warnings`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:1a60951ee550e065977db2b64ad28eba5e7c33681c2959724e3439866dcde4b1b4c3838a6d326e23f04f976b41bb3a69311a15943419597fd69997ca6fffb1b0`
- Judge `public`: accepted (exit Some(0), 810 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `test` `--offline` `--manifest-path` `subject/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:ecbd89175fbbadf25c283887a8257724f7b2a6d6d47b1f71a3cc3d1306167aa0766ff6dbd5558b57a842260af60c6035c70bdcc843a3690beb0308e662890044`; stderr `bhcp.hash/sha3-512@0:7878cd5fae69351f4b2fedfcc3ffebf4e3f2b0769f94dbf65dc1adf7c9e03030cc347fda30a1020e1ccab9896a8306daf0b83ec503ef9f0bb750825bdbe93017`
- Judge `oracle`: rejected (exit Some(101), 672 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `test` `--offline` `--manifest-path` `oracle/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:aaf2c9c83cda5e5f1e59d9b881450b64bdfa9cb3d004f877212d09a40b9617f32b6ea17ff4588c2722ec772acbf2dcb9e6a2bb50383d968ad3ff510c73374414`; stderr `bhcp.hash/sha3-512@0:fdb62141edba9e25c1b63197d4220f660d38e5b42e86231465503a438e4d82498d029467408c4fdfee1fb55dbb7ba46163c42040c49da3e4f71bb1fc6cccfbf8`
- Judge `change-policy`: accepted (exit Some(0), 3 ms); command `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/target/release/evidence_generalization_comparative_policy`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
