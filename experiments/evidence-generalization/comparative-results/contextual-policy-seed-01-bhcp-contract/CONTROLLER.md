# Experiment evidence-generalization-comparative-contextual-policy-seed-01-bhcp-contract

- Plan: `bhcp.hash/sha3-512@0:fe7b01476f539de5bbd5ed77f546e76128b9e29976eb2a250323580e181f01f97ef5c2a7bd384a33c2a55f1f7f8cba0c9265adcc8a9ca69d90b4f14900313616`
- Fixture: `bhcp.hash/sha3-512@0:88024a336e195eb5e15b29d90cbcfcb15017f0e4199f8fc094dacb3e3b82af542a40873c6d9d7f0c3dd0bf8282749b7341f0c44f72fedaa05ceb4e258dcc9755`
- Model: `gpt-5.4-mini`
- Reasoning: `medium`
- Sandbox: `workspace-write/no-network/read-confined`
- Toolchain: `codex-cli-0.142.4+rust-1.97.1`
- Run order: bhcp-contract

| Arm | Status | Claimed | Input tokens | Commands | Elapsed ms |
| --- | --- | --- | ---: | ---: | ---: |
| bhcp-contract | rejected (verification-failed) | no | 703767 | 0 | 146987 |

## Arm bhcp-contract

- Result: one or more configured judges rejected the candidate
- Total elapsed: 146987 ms
- Agent elapsed: 144854 ms
- Agent command: `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/target/release/bhcp_codex_experiment_driver` `/Users/sasha/.local/share/mise/installs/codex/0.142.4/codex-aarch64-apple-darwin` `/Users/sasha/.codex` `/Users/sasha/.cargo` `/Users/sasha/.rustup` `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/target/release/bhcp` `/Users/sasha/.rustup/toolchains/1.97.1-aarch64-apple-darwin/bin` `1.97.1` `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/experiments` `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/experiments/contextual-policy-agent/oracle/tests/invariants.rs`
- Subject before: `bhcp.hash/sha3-512@0:2c4b358732dcc5537d70c870ac0ad3464beb754bfbdfef47d3420fb27fb13815f9d688325dceb87bdcac74fe7dd17fde8831cfc78699b4952183fdb469af1a0b`
- Subject after: `bhcp.hash/sha3-512@0:2c4b358732dcc5537d70c870ac0ad3464beb754bfbdfef47d3420fb27fb13815f9d688325dceb87bdcac74fe7dd17fde8831cfc78699b4952183fdb469af1a0b`
- Agent executable: `bhcp.hash/sha3-512@0:ec82cf095819683495ce9727232bca350e1a030c542667f4788324b8ddd894ded6ea0eb14f554147dfecfaafd8c9372755bb25f69106d1918efc89bba57577b1`
- Agent stdout: `bhcp.hash/sha3-512@0:71d1eba28c44b88fe9e423c476ae9ef40af912baacda3a8c72873a899c6741d1e810c292a8ececd4d161c828b45145fcd101dfcdf7cddf41d7f9af11afd6a6d2`
- Agent stderr: `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
- Frozen inputs: 12
- Tokens: input 703767, cached 641280, output 6503, reasoning 5520
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
- Judge `format`: accepted (exit Some(0), 183 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `fmt` `--check` `--manifest-path` `subject/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
- Judge `clippy`: accepted (exit Some(0), 397 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `clippy` `--offline` `--manifest-path` `subject/Cargo.toml` `--all-targets` `--` `-D` `warnings`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:f06eb5c0733c4697bfa262b1c0abe7288353d8548ecb4528e39bd1f5a3e046d2ec5a7b190d7a30887629edff4c6c92f0fd8663919c01bd5e9352929207993716`
- Judge `public`: accepted (exit Some(0), 836 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `test` `--offline` `--manifest-path` `subject/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:7d397e0a5867272a5883fbaf022d8a6b7473aeb5a7d4b7654abec323d5faa864d57e63c72115f2f879fd7d69b688486b1e2263a08cf808ba7c7863078dc6995b`; stderr `bhcp.hash/sha3-512@0:60534203c1bfbf1448058136bc2f934a1ec3eb8a08326d69f7ec361a49861035c785d53d2d97a6714446a90067829f674390e0561604e47cd97029acd9b52ea9`
- Judge `oracle`: rejected (exit Some(101), 666 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `test` `--offline` `--manifest-path` `oracle/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:d4e7b5129edfd2b57534cd4f56fffd1cc17683c48f7e73576dacae70a135eac26331ee1c43703be55d0bb2ae89dc270ca040d9067fb085b2c3c4fbd48b24ad75`; stderr `bhcp.hash/sha3-512@0:1562526b4c4ce54c10edef2edf45fd94d0a70a17b140f45398a737faad49d7618d9140e838268afec154ed84c22ab2fca3d1d94ead030a869b6abca0afc5901e`
- Judge `change-policy`: accepted (exit Some(0), 5 ms); command `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/target/release/evidence_generalization_comparative_policy`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
