# Experiment evidence-generalization-comparative-atomic-batch-seed-02-bhcp-contract

- Plan: `bhcp.hash/sha3-512@0:8aa8e1674872e897d7aafe579c4f69b2fe1a6dcb40af5069871a947c9ff418f04616f1db123ae16e5500068906c5f49eb2f6da38263cbe79439e658692705e28`
- Fixture: `bhcp.hash/sha3-512@0:33cafd9dc812a164b060ddd109045b06be09742dba4fc4f9d028844e34180f76ba2977bf37d839b04fdbb81def48e9ceea59ec1588233528ee8b80225a0ba461`
- Model: `gpt-5.4-mini`
- Reasoning: `medium`
- Sandbox: `workspace-write/no-network/read-confined`
- Toolchain: `codex-cli-0.142.4+rust-1.97.1`
- Run order: bhcp-contract

| Arm | Status | Claimed | Input tokens | Commands | Elapsed ms |
| --- | --- | --- | ---: | ---: | ---: |
| bhcp-contract | rejected (verification-failed) | no | 382880 | 0 | 94974 |

## Arm bhcp-contract

- Result: one or more configured judges rejected the candidate
- Total elapsed: 94974 ms
- Agent elapsed: 92624 ms
- Agent command: `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/target/release/bhcp_codex_experiment_driver` `/Users/sasha/.local/share/mise/installs/codex/0.142.4/codex-aarch64-apple-darwin` `/Users/sasha/.codex` `/Users/sasha/.cargo` `/Users/sasha/.rustup` `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/target/release/bhcp` `/Users/sasha/.rustup/toolchains/1.97.1-aarch64-apple-darwin/bin` `1.97.1` `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/experiments` `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/experiments/minimal-coding-agent/oracle/tests/invariants.rs`
- Subject before: `bhcp.hash/sha3-512@0:b455923029277d4a7d05075b74ddb195361c3628d5e23350b4c457a0a164cbfb5a71b0ede2059516e3c901e354c3db3e80ad42989e634e7077c293c819ed7a94`
- Subject after: `bhcp.hash/sha3-512@0:b455923029277d4a7d05075b74ddb195361c3628d5e23350b4c457a0a164cbfb5a71b0ede2059516e3c901e354c3db3e80ad42989e634e7077c293c819ed7a94`
- Agent executable: `bhcp.hash/sha3-512@0:ec82cf095819683495ce9727232bca350e1a030c542667f4788324b8ddd894ded6ea0eb14f554147dfecfaafd8c9372755bb25f69106d1918efc89bba57577b1`
- Agent stdout: `bhcp.hash/sha3-512@0:cbd3ff40524252d4969eeae0968fa273b5833026c3917af49c142cd9ddcc415662d17d06879ef1aef9572d28bcc7973522da49985592b94972a653f5e5c4a969`
- Agent stderr: `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
- Frozen inputs: 14
- Tokens: input 382880, cached 336384, output 4618, reasoning 3749
- Completed commands: 0
- Input `COMPARATIVE_BHCP_PROMPT.md`: `bhcp.hash/sha3-512@0:9bdcc4d2b2120e48f06a24a887f4403faa49c1af6b2f4e5f0e432b392662455b56f980ea413b0feac0ec64456b354affdf34ef3802388bcc10fd947246887231`
- Input `TASK.md`: `bhcp.hash/sha3-512@0:59af0a2126f51fac806451cf5c880ef382f97da86a285e52bca617a6d852d964ecfd98dc0e60b98ef3945c62287c4e48ad656b004e68fa5da3992b058c662ed4`
- Input `contract.bhcp`: `bhcp.hash/sha3-512@0:721b099b961c721ae9e7d390992e2113d5e3e8fe4055e5ac043218201b7a674eb1e968a7c738c75ea32acd2d27978793877460b4f437fd18ea4b1267c1e9e2a3`
- Input `contract.semantic-id`: `bhcp.hash/sha3-512@0:78c1b10dadb4b629c327335fe22567abb41a9b36404a10af527b102bcc91b689b9f17c645dcdbebb21b5e5cf74633d069eded3dd91b52ca6befec35b5ef21065`
- Input `subject`: `directory`
- Input `subject/.gitignore`: `bhcp.hash/sha3-512@0:9be9f39fca13920266e2bee5474bedf4d96abe85cd647f98406185f226130f4cbbaae2d367116c73ce50d09faab8091d8c9a1f275001fd58167f8863b66495da`
- Input `subject/.mise.toml`: `bhcp.hash/sha3-512@0:3214b7551815c603e93136d4907bc85e0a129335eb9a3ff48ef9da25f5464ae6807e3e55d308a66031577a251be42ecde76faf07142fa4863ac278bd0d734992`
- Input `subject/Cargo.lock`: `bhcp.hash/sha3-512@0:2b82aa9988ed745dd53072f239e76460274ae7a66c9e68877e3ba433cdb109df7fded5630c004c55bbcfd5e5c78be8fb2887e678e19e46a0f933b350f84c50a5`
- Input `subject/Cargo.toml`: `bhcp.hash/sha3-512@0:855680b6969f1feac11f94bc59db68a93a8aad2682d71cb3ae3e9db9fb7252dbad2d6cf8cb2be252d0ec63463fa6176377e4d36c89649ace6be6cc49c40f3ba6`
- Input `subject/README.md`: `bhcp.hash/sha3-512@0:3ff20154a8a0207aac7c83487868d2ebe4e4322a0871d112af441eea09b4b9eaf2de52db6e33ca83e9689c2473576f67a661f5abb3437fc17bcaa01eed347991`
- Input `subject/src`: `directory`
- Input `subject/src/lib.rs`: `bhcp.hash/sha3-512@0:6c0ff68252f4e970c135a50014914ac5fc2d015b79a15979e30a0bddc0f162005e1965a997ec8afd43f2c42d717f39a570ab198893364527a7eb8eb98eb8563b`
- Input `subject/tests`: `directory`
- Input `subject/tests/public.rs`: `bhcp.hash/sha3-512@0:58bc6418c1cd392f6c5103c991f2a29e5c1cdba93b9bfa4bec5919e872604bb04d505b1fef0fc512e33b125383cfd22d4b95116d8920fdd0ace8a81ae5b74bff`
- Judge `format`: accepted (exit Some(0), 185 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `fmt` `--check` `--manifest-path` `subject/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
- Judge `clippy`: accepted (exit Some(0), 439 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `clippy` `--offline` `--manifest-path` `subject/Cargo.toml` `--all-targets` `--` `-D` `warnings`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:b7f1950945e62dc594d67cb8a583d9b7334d645e1f9ab2c05ea9118b421d13e74a0695ecf227aec305da13072db068e7252c7bfe936168878aa5c19eb46ec016`
- Judge `public`: accepted (exit Some(0), 899 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `test` `--offline` `--manifest-path` `subject/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:30169aca2b7cb0ad5ebda3226948abeac04b4c4192d30e16e6b1296482ef119b31cb57ce73ef203e3a8a2497903483b763b08758bd4951cc0faef3d7e2481a2c`; stderr `bhcp.hash/sha3-512@0:992978c5b91e11e4678ec73378bcbe845fcb1b8fe142277fbdddf80e6735e8c8ed39036f32940bb130ed840ca6ac0e8739d2b840284abfe8602770913086d992`
- Judge `oracle`: rejected (exit Some(101), 765 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `test` `--offline` `--manifest-path` `oracle/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:4464da8b5f128f38b5792d6104df92d226169e9f32e45da40ed5bac67cac4cdd6ae489953d7fd567a681b707da3eda714296cae1fd1d7dd09f6999c3d3d9ef42`; stderr `bhcp.hash/sha3-512@0:03d50783b706d9f0a0e1c065583c683b99a5911afd186c6d353838ee78f7c179bdf05b188ed1c2b7e39a25fa08cba7993a1ecf4137b6a00e3bf30ed1ab989108`
- Judge `change-policy`: accepted (exit Some(0), 5 ms); command `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/target/release/evidence_generalization_comparative_policy`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
