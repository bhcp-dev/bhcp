# Experiment in-session-evidence-forward-001

- Plan: `bhcp.hash/sha3-512@0:7149bc3fd7e8d843692ce2fb68804e6c3303b4cd45601204a126d17d28aefdad778e44b104d8e27bcc240647eea0e2024647f6c9a6bf0e582f05d55b992bf021`
- Fixture: `bhcp.hash/sha3-512@0:f6ba9badf1dd33553b9a86818ce8ac56aa5a3b4b163cb952fec9e27b0d24894cce31870998e4a627ddabec7f874db7234910a0991d799b9b9e19a73552a50bb3`
- Model: `gpt-5.4-mini`
- Reasoning: `medium`
- Sandbox: `workspace-write/no-network/read-confined`
- Toolchain: `codex-cli-0.142.4+rust-1.97.1`
- Run order: forward-01

| Arm | Status | Claimed | Input tokens | Commands | Elapsed ms |
| --- | --- | --- | ---: | ---: | ---: |
| forward-01 | rejected (verification-failed) | no | 189447 | 0 | 90298 |

## Arm forward-01

- Result: one or more configured judges rejected the candidate
- Total elapsed: 90298 ms
- Agent elapsed: 88841 ms
- Agent command: `/Users/sasha/.codex/worktrees/bhcp-issue-27-in-session-adapters/target/release/bhcp_codex_experiment_driver` `/Users/sasha/.local/share/mise/installs/codex/0.142.4/codex-aarch64-apple-darwin` `/Users/sasha/.codex` `/Users/sasha/.cargo` `/Users/sasha/.rustup` `/Users/sasha/.codex/worktrees/bhcp-issue-27-in-session-adapters/target/release/bhcp` `/Users/sasha/.rustup/toolchains/1.97.1-aarch64-apple-darwin/bin` `1.97.1` `/Users/sasha` `/Users/sasha/.codex/worktrees/bhcp-issue-27-in-session-adapters/experiments/in-session-evidence-agent/oracle/tests/invariants.rs`
- Subject before: `bhcp.hash/sha3-512@0:0f211948e3702c7ea84b2114f642fd86a4bf9373bc604d4c77d1f6756a6056d65637cd5a872e868de72ef7093a96dea81bbbb95c3c78b9316ed2748f3039e7ac`
- Subject after: `bhcp.hash/sha3-512@0:0f211948e3702c7ea84b2114f642fd86a4bf9373bc604d4c77d1f6756a6056d65637cd5a872e868de72ef7093a96dea81bbbb95c3c78b9316ed2748f3039e7ac`
- Agent executable: `bhcp.hash/sha3-512@0:e05587da324f0a3ebe413387ea9320e044a921f569d05c6bf4b72d9f73537957dc629ba81f12d64ef0c58f878fa04e36375433e4934d1e3447e928a031270f51`
- Agent stdout: `bhcp.hash/sha3-512@0:911a584ee72cd10fb1d17630760f5672b3293b0b50cacd0c69f5e69f5cf7c141b29926f6929938b26fed8758dfa77880df4f51d6779223db6e73babf62cca37a`
- Agent stderr: `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
- Frozen inputs: 21
- Tokens: input 189447, cached 171136, output 4192, reasoning 3447
- Completed commands: 0
- Input `FORWARD_PROMPT.md`: `bhcp.hash/sha3-512@0:1515709aa73eddb2497b32beb203896330d5c72e5fa7af21570d5976b868c401548ba76cf70da0cf488807f551d1ad161c7e686a716bc16348fa7b40891795e0`
- Input `TASK.md`: `bhcp.hash/sha3-512@0:09cc271029b9419c47677c14f103a88e4b80f0967abc908d370fb74808b0e1d7822fc6b6d6cd21f1f428b78afe97370727cdee21f7fa33c74fcdfcfef99a22a3`
- Input `bhcp-project.toml`: `bhcp.hash/sha3-512@0:86a08778d7929a75308708a93b77ec229e48477900f499a62ff44a2ba4b80388103dbd79976ba7785c30e0ad85989e71ce764c7396280cbec779f50c6063c975`
- Input `candidate.cbor`: `bhcp.hash/sha3-512@0:85231ea5a4d3a02266c2d0e94faa420d1682e5cec882bc8b53451a5606a1833f012cdeb910307fa3c4b4322b765dbb1345657f254bfb3aca95ba62627c228425`
- Input `contract.bhcp`: `bhcp.hash/sha3-512@0:400f2bb1fb66645e5709263feb5f358c9fbb02fb40e4653649a07e35e4799509a656fafa2edeaee53ee875da7573219a4bad163e9fe6a9581c96de3efe7cef4e`
- Input `contract.semantic-id`: `bhcp.hash/sha3-512@0:3b94fed696cb2d3909de5745ffa2ee629796d264df5aa3837033791832606467d63c364b2228a57f91bf9f1c61312d54646e2aaf9afb5b59c36e310f48048ec2`
- Input `subject`: `directory`
- Input `subject/.agents`: `directory`
- Input `subject/.agents/skills`: `directory`
- Input `subject/.agents/skills/interpret-bhcp-contract`: `directory`
- Input `subject/.agents/skills/interpret-bhcp-contract/SKILL.md`: `bhcp.hash/sha3-512@0:7243d49c076938427acbd80952096d0328229f3a3d08816decb5a2ab0c006adc91b146f0c3dfd17fb9de61a10122d265e90ed45783f5edff9aa86498d40c78cb`
- Input `subject/.agents/skills/interpret-bhcp-contract/agents`: `directory`
- Input `subject/.agents/skills/interpret-bhcp-contract/agents/openai.yaml`: `bhcp.hash/sha3-512@0:a7edfde74b970ffbb1d527055cd8168593ef0b00a81644f4ad3a8a60559216c3564e68521758923b417d79ff12ad60142d91152a8249b397d8ffa3cf3a060647`
- Input `subject/Cargo.lock`: `bhcp.hash/sha3-512@0:77de4dcb7fb90eabe4bf3d681f9f2a631825163ff44fb953df3b86239de9bbc1009c8a665ddba27a3ea00a349f492870c90555664989dbb24e971edef6f802a1`
- Input `subject/Cargo.toml`: `bhcp.hash/sha3-512@0:b50a8b28f883706049685d287f2bd67cd939241ff21eaaed299bb7a1bcd455868efd2479a282ca4503581ebc31747caa5cee65d3caa5c4ab585d8873e757cf9e`
- Input `subject/src`: `directory`
- Input `subject/src/lib.rs`: `bhcp.hash/sha3-512@0:0a3a915d41d2a1e253bf7a8662f596e969253c89781bd9eec9d9cfdecc59635ddf5099b2b10fe38371941e3ad6f21b9cb3f7d91eca682adc5cf1f7a2f1d94487`
- Input `subject/tests`: `directory`
- Input `subject/tests/public.rs`: `bhcp.hash/sha3-512@0:ba4bea5dc920bfd5cd88d30b743b1fd127e2505af41a3488cf6e636ab2f90d72a5548679eae4f8b4185300f6b1151a44d75c813254ef20a1d4ab05a31cdd4778`
- Input `subject/tools`: `directory`
- Input `subject/tools/in-session-evidence-adapter`: `bhcp.hash/sha3-512@0:f6c0a2bfefc5551b5812b54e8b29f27d768607d73e2441fbb544873c8df994a3bc80cb84b49eed69a7dbb332925d31170889707771a697ed691e4b4523fe4ffd`
- Judge `format`: accepted (exit Some(0), 54 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `fmt` `--check` `--manifest-path` `subject/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
- Judge `clippy`: accepted (exit Some(0), 102 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `clippy` `--offline` `--manifest-path` `subject/Cargo.toml` `--all-targets` `--` `-D` `warnings`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:454f1b9c9c81dc695ba2089735a8e814a0f5da6057109491bd4f064a96a0c7e18eff46c8b78718bca77963b9f64916457be80321411b8cf8a3ac47b27d694d07`
- Judge `public`: rejected (exit Some(101), 639 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `test` `--offline` `--manifest-path` `subject/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:f4ff36d0f0c45b09fc3e2cc8819babdb50ff258a131c5ac3bb785f9275500fb55adca769c6b4ccddfb4b4dd5f18312ee553b4983e0e0ebd5df62dc9a6e41dcdc`; stderr `bhcp.hash/sha3-512@0:d6ba3968ea3bcb5e96c446ab0340924881ad763d400c0585037f6a2aa104793f4fbf7e84154443a5746e43c078244434b702763d5e010160d271caec465a666b`
- Judge `oracle`: rejected (exit Some(101), 395 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `test` `--offline` `--manifest-path` `oracle/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:b8632d251fb495bf8e9900ec919177efdd36c12e77d2160ecd2bb8f503e2faeaa479b92daad2e654409264cb305e239a91b44289571f03ea92bb1257905e093e`; stderr `bhcp.hash/sha3-512@0:dd866443a4213648101c41a13071b5700cf918b7e73fbd126f475e0fcd2926c710221dd2d1a169ef90d0b32faedfa58b4993036ff0c6f1fb3fa977bb6b68ce2b`
- Judge `change-policy`: rejected (exit Some(90), 202 ms); command `/Users/sasha/.codex/worktrees/bhcp-issue-27-in-session-adapters/target/release/bhcp-in-session-evidence-adapter` `judge-change-policy`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
