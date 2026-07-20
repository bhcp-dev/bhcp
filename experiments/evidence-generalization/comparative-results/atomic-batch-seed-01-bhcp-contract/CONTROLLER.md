# Experiment evidence-generalization-comparative-atomic-batch-seed-01-bhcp-contract

- Plan: `bhcp.hash/sha3-512@0:ad253a2a4ea5899e7706ab7962bcb8a42306495a7ea05dccf171e38af4dfbf305f02f4324272efe0b5217492cd158e99fff07d923b915dae37156018e5f68e5f`
- Fixture: `bhcp.hash/sha3-512@0:33cafd9dc812a164b060ddd109045b06be09742dba4fc4f9d028844e34180f76ba2977bf37d839b04fdbb81def48e9ceea59ec1588233528ee8b80225a0ba461`
- Model: `gpt-5.4-mini`
- Reasoning: `medium`
- Sandbox: `workspace-write/no-network/read-confined`
- Toolchain: `codex-cli-0.142.4+rust-1.97.1`
- Run order: bhcp-contract

| Arm | Status | Claimed | Input tokens | Commands | Elapsed ms |
| --- | --- | --- | ---: | ---: | ---: |
| bhcp-contract | rejected (verification-failed) | no | 277810 | 0 | 68760 |

## Arm bhcp-contract

- Result: one or more configured judges rejected the candidate
- Total elapsed: 68760 ms
- Agent elapsed: 66316 ms
- Agent command: `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/target/release/bhcp_codex_experiment_driver` `/Users/sasha/.local/share/mise/installs/codex/0.142.4/codex-aarch64-apple-darwin` `/Users/sasha/.codex` `/Users/sasha/.cargo` `/Users/sasha/.rustup` `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/target/release/bhcp` `/Users/sasha/.rustup/toolchains/1.97.1-aarch64-apple-darwin/bin` `1.97.1` `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/experiments` `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/experiments/minimal-coding-agent/oracle/tests/invariants.rs`
- Subject before: `bhcp.hash/sha3-512@0:b455923029277d4a7d05075b74ddb195361c3628d5e23350b4c457a0a164cbfb5a71b0ede2059516e3c901e354c3db3e80ad42989e634e7077c293c819ed7a94`
- Subject after: `bhcp.hash/sha3-512@0:b455923029277d4a7d05075b74ddb195361c3628d5e23350b4c457a0a164cbfb5a71b0ede2059516e3c901e354c3db3e80ad42989e634e7077c293c819ed7a94`
- Agent executable: `bhcp.hash/sha3-512@0:ec82cf095819683495ce9727232bca350e1a030c542667f4788324b8ddd894ded6ea0eb14f554147dfecfaafd8c9372755bb25f69106d1918efc89bba57577b1`
- Agent stdout: `bhcp.hash/sha3-512@0:b766fc389013e3b874f8f3cd1dbe8900b6a265782bd802d37b7237ed08cc9c5324a919d2b405f0d430c0d92137faf47b3893bcb22e2d6212388124b49e641448`
- Agent stderr: `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
- Frozen inputs: 14
- Tokens: input 277810, cached 238336, output 3508, reasoning 2665
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
- Judge `format`: accepted (exit Some(0), 196 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `fmt` `--check` `--manifest-path` `subject/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
- Judge `clippy`: accepted (exit Some(0), 488 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `clippy` `--offline` `--manifest-path` `subject/Cargo.toml` `--all-targets` `--` `-D` `warnings`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:ef0a5817311dae3a3f9affe75952fa490846063305ad3b6f219d506d501983ac27d0b0ececabeb92e8e8edf63b3b4805d161c4c62dc44d6d488f86a8b1560328`
- Judge `public`: accepted (exit Some(0), 907 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `test` `--offline` `--manifest-path` `subject/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:d05f0981e0753c8262f75080449f60ddd0f4476fb42eb6ae1aa7edf84156c095d4f6466c4ca8c2573dbb6d1478654a4e3d6b3ac55cb9bc680969993e07d58b91`; stderr `bhcp.hash/sha3-512@0:7cc31f1f41bb98a7497e4c09cb5e68453672e54b97b9ab201f63a6c0ae0e539cf677e904d160ab8368664ec7fe7b2bee5e7bdefaad5438e544a062cc5d3bffa1`
- Judge `oracle`: rejected (exit Some(101), 792 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `test` `--offline` `--manifest-path` `oracle/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:1b1a2d3d5e45d05359023366776a4698eaa918ce4fde874bb33a9269fc7b8ce18ccba0f64a206f4ca5061b1679b476686ce8fa9f5b23ff02143d649535533263`; stderr `bhcp.hash/sha3-512@0:cfb9334e81131ce8fc7cde7a86e4a50317abc30760fee25f47b0b5c7b113ec0b98a12d068f9f7c112697623384ee18c124c3720b18b770fd7312021a84961311`
- Judge `change-policy`: accepted (exit Some(0), 5 ms); command `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/target/release/evidence_generalization_comparative_policy`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
