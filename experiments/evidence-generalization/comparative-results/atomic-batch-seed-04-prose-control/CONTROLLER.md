# Experiment evidence-generalization-comparative-atomic-batch-seed-04-prose-control

- Plan: `bhcp.hash/sha3-512@0:35b854a920bd34f65e750858384ddf8efe5ab5690db91ac74c4afe074fb2f237dc38cc165d02db4ac5864eebf610865d713891a2675acaa2d7775925f3b118b7`
- Fixture: `bhcp.hash/sha3-512@0:33cafd9dc812a164b060ddd109045b06be09742dba4fc4f9d028844e34180f76ba2977bf37d839b04fdbb81def48e9ceea59ec1588233528ee8b80225a0ba461`
- Model: `gpt-5.4-mini`
- Reasoning: `medium`
- Sandbox: `workspace-write/no-network/read-confined`
- Toolchain: `codex-cli-0.142.4+rust-1.97.1`
- Run order: prose-control

| Arm | Status | Claimed | Input tokens | Commands | Elapsed ms |
| --- | --- | --- | ---: | ---: | ---: |
| prose-control | rejected (verification-failed) | no | 577560 | 0 | 140880 |

## Arm prose-control

- Result: one or more configured judges rejected the candidate
- Total elapsed: 140880 ms
- Agent elapsed: 138414 ms
- Agent command: `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/target/release/bhcp_codex_experiment_driver` `/Users/sasha/.local/share/mise/installs/codex/0.142.4/codex-aarch64-apple-darwin` `/Users/sasha/.codex` `/Users/sasha/.cargo` `/Users/sasha/.rustup` `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/target/release/bhcp` `/Users/sasha/.rustup/toolchains/1.97.1-aarch64-apple-darwin/bin` `1.97.1` `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/experiments` `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/experiments/minimal-coding-agent/oracle/tests/invariants.rs`
- Subject before: `bhcp.hash/sha3-512@0:b455923029277d4a7d05075b74ddb195361c3628d5e23350b4c457a0a164cbfb5a71b0ede2059516e3c901e354c3db3e80ad42989e634e7077c293c819ed7a94`
- Subject after: `bhcp.hash/sha3-512@0:b455923029277d4a7d05075b74ddb195361c3628d5e23350b4c457a0a164cbfb5a71b0ede2059516e3c901e354c3db3e80ad42989e634e7077c293c819ed7a94`
- Agent executable: `bhcp.hash/sha3-512@0:ec82cf095819683495ce9727232bca350e1a030c542667f4788324b8ddd894ded6ea0eb14f554147dfecfaafd8c9372755bb25f69106d1918efc89bba57577b1`
- Agent stdout: `bhcp.hash/sha3-512@0:1ebdcd61eee12cd0781603ec59b07cc2f19b393063b5d7b6c6e0c8cdb7823b68ed1bd6f46dff8deebe4272343b7fa364fd4e1d090c2c66f9a0da8cc0252a8891`
- Agent stderr: `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
- Frozen inputs: 12
- Tokens: input 577560, cached 487680, output 7406, reasoning 6198
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
- Judge `format`: accepted (exit Some(0), 198 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `fmt` `--check` `--manifest-path` `subject/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
- Judge `clippy`: accepted (exit Some(0), 530 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `clippy` `--offline` `--manifest-path` `subject/Cargo.toml` `--all-targets` `--` `-D` `warnings`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:52aec3ac1789280bdbfc4892f656814f516ad1f2c453dfd99e124a503db4baafabda5397ac1c41e8dc5091f24d435370e5a96d7fb622ed99f6c0a9c637394947`
- Judge `public`: accepted (exit Some(0), 909 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `test` `--offline` `--manifest-path` `subject/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:998117c363ab64badb2331ea19c5835f08fa8f0b3867db0d585cdc6191153fd2d7307340e42c525d034f96ef856bb1fd2ba68e09ec47ee557efd1b2e72ebcca3`; stderr `bhcp.hash/sha3-512@0:64f4e657b44a7f3cfeafb6efc95c707e3be8abb8eeb7e187a9f84c2a8f1c4f5c6124219ed4942717c52e805d002b0c6ad50cc2d4245378208664d613f1f899cc`
- Judge `oracle`: rejected (exit Some(101), 772 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `test` `--offline` `--manifest-path` `oracle/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:28efdee4e02a54083f04ce28033c7a7ed78edf949b221d2ba04920813ae1edc1bb503fb8e3db969c164b1ee07409d6ef5a0ea24751db77c1d2cf8ac9fdd24cc2`; stderr `bhcp.hash/sha3-512@0:3163a3007a29ea3c2bf3ca71e3d605bc1f9e95f6b57d1588e1a454ecb06ef63fa9045c9162dc65bf752efa26f60ff2b59f610de4e97f2fc513c151477c2c721f`
- Judge `change-policy`: accepted (exit Some(0), 5 ms); command `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/target/release/evidence_generalization_comparative_policy`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
