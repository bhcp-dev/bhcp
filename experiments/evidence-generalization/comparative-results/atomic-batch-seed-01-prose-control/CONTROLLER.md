# Experiment evidence-generalization-comparative-atomic-batch-seed-01-prose-control

- Plan: `bhcp.hash/sha3-512@0:bbe6195e6ad030f669b2b28fc52667af04fafaebc3586add88bbf798d7c390db11a08e0f41402ac3792384a0458169d2e6f2b9e7bc4aaf4e7666924d42186125`
- Fixture: `bhcp.hash/sha3-512@0:33cafd9dc812a164b060ddd109045b06be09742dba4fc4f9d028844e34180f76ba2977bf37d839b04fdbb81def48e9ceea59ec1588233528ee8b80225a0ba461`
- Model: `gpt-5.4-mini`
- Reasoning: `medium`
- Sandbox: `workspace-write/no-network/read-confined`
- Toolchain: `codex-cli-0.142.4+rust-1.97.1`
- Run order: prose-control

| Arm | Status | Claimed | Input tokens | Commands | Elapsed ms |
| --- | --- | --- | ---: | ---: | ---: |
| prose-control | rejected (verification-failed) | no | 184237 | 0 | 71525 |

## Arm prose-control

- Result: one or more configured judges rejected the candidate
- Total elapsed: 71525 ms
- Agent elapsed: 68996 ms
- Agent command: `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/target/release/bhcp_codex_experiment_driver` `/Users/sasha/.local/share/mise/installs/codex/0.142.4/codex-aarch64-apple-darwin` `/Users/sasha/.codex` `/Users/sasha/.cargo` `/Users/sasha/.rustup` `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/target/release/bhcp` `/Users/sasha/.rustup/toolchains/1.97.1-aarch64-apple-darwin/bin` `1.97.1` `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/experiments` `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/experiments/minimal-coding-agent/oracle/tests/invariants.rs`
- Subject before: `bhcp.hash/sha3-512@0:b455923029277d4a7d05075b74ddb195361c3628d5e23350b4c457a0a164cbfb5a71b0ede2059516e3c901e354c3db3e80ad42989e634e7077c293c819ed7a94`
- Subject after: `bhcp.hash/sha3-512@0:b455923029277d4a7d05075b74ddb195361c3628d5e23350b4c457a0a164cbfb5a71b0ede2059516e3c901e354c3db3e80ad42989e634e7077c293c819ed7a94`
- Agent executable: `bhcp.hash/sha3-512@0:ec82cf095819683495ce9727232bca350e1a030c542667f4788324b8ddd894ded6ea0eb14f554147dfecfaafd8c9372755bb25f69106d1918efc89bba57577b1`
- Agent stdout: `bhcp.hash/sha3-512@0:62d490c4f29bdae27e1a616ff51394a9c219b3226f8f3753f21c77a553e0426131d8d50fb7e13117ad264f28680d45534e116faa5c63ed32e91f41938316a132`
- Agent stderr: `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
- Frozen inputs: 12
- Tokens: input 184237, cached 147072, output 3380, reasoning 2838
- Completed commands: 0
- Input `COMPARATIVE_PROSE_PROMPT.md`: `bhcp.hash/sha3-512@0:cbcb9f808d609b6a432d3ecb1ee110df319dc010c06ec59646602e90d1d4b156c5b960824cffe5f0df732756ae7efd18f4dbcbaf1dce303cb00b9452cb368486`
- Input `PROSE_TASK.md`: `bhcp.hash/sha3-512@0:a52acfa47fc1919de91f61dd4d274ce2d57d3aff64f8ee07d6930e00a2ae524ca79aab99a9c9b422d5a831320b4f3ba58150cff41a09dd9e4f58a73df94abb3d`
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
- Judge `format`: accepted (exit Some(0), 186 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `fmt` `--check` `--manifest-path` `subject/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
- Judge `clippy`: accepted (exit Some(0), 420 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `clippy` `--offline` `--manifest-path` `subject/Cargo.toml` `--all-targets` `--` `-D` `warnings`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:077391e7f437769e623eb025f428a5ecd150d7cab610e14b26c876a4f151f6b37aef9c09ceae4f0f400ebca9a2b7890c8b329deed142bb79eec83508ed76c7ce`
- Judge `public`: accepted (exit Some(0), 886 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `test` `--offline` `--manifest-path` `subject/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:d05f0981e0753c8262f75080449f60ddd0f4476fb42eb6ae1aa7edf84156c095d4f6466c4ca8c2573dbb6d1478654a4e3d6b3ac55cb9bc680969993e07d58b91`; stderr `bhcp.hash/sha3-512@0:ecb638dc6d5099e6f2b42c9651c24edc5b244a2d583a1167e25d957869b5e44eaa1568535f641fe887723a9869bed9611961459d72f4f8ff0614fc553fbe3c2c`
- Judge `oracle`: rejected (exit Some(101), 786 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `test` `--offline` `--manifest-path` `oracle/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:8a1d313500f745cc150ed1f523d07ba26662049c18302a6aee98e1ca74f26df2a9964aa75fd2fe663e7aa9e882eb12977b6d808dd97a1ed0ba86ab5388b48e91`; stderr `bhcp.hash/sha3-512@0:177bf47588a0c982930ee699ad7aaedfb886a37c512b5bbaec2fe8db6a769d23723f6c64bef8a7c65f7e7f70cf85130d7a997cb5e2f0b0791f9b4275e39d05c8`
- Judge `change-policy`: accepted (exit Some(0), 197 ms); command `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/target/release/evidence_generalization_comparative_policy`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
