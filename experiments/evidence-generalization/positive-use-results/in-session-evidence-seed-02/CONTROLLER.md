# Experiment evidence-generalization-positive-in-session-evidence-seed-02

- Plan: `bhcp.hash/sha3-512@0:52ad80f350ba2ad80785ff0560b88a7f2634093571748b81c6f11cdb8695d5665e714779bbd2ee2ea7c83c79a229cf132fc934332ad50f1738aa2de345732141`
- Fixture: `bhcp.hash/sha3-512@0:3eb4f82d5f09788fe1880d0a74d2bf9a1e5a94c7802688feeef92b76c71ad4c1336ae9bfbd05ef295930206bae9cfe3a97a3a14188c438824c9fef4e360dd0bd`
- Model: `gpt-5.4-mini`
- Reasoning: `medium`
- Sandbox: `workspace-write/no-network/read-confined`
- Toolchain: `codex-cli-0.142.4+rust-1.97.1`
- Run order: seed-02

| Arm | Status | Claimed | Input tokens | Commands | Elapsed ms |
| --- | --- | --- | ---: | ---: | ---: |
| seed-02 | rejected (verification-failed) | no | 132006 | 0 | 65489 |

## Arm seed-02

- Result: one or more configured judges rejected the candidate
- Total elapsed: 65489 ms
- Agent elapsed: 63972 ms
- Agent command: `/Users/sasha/.codex/worktrees/bhcp-issue-92-positive-adapter-use/target/release/bhcp_codex_experiment_driver` `/Users/sasha/.local/share/mise/installs/codex/0.142.4/codex-aarch64-apple-darwin` `/Users/sasha/.codex` `/Users/sasha/.cargo` `/Users/sasha/.rustup` `/Users/sasha/.codex/worktrees/bhcp-issue-92-positive-adapter-use/target/release/bhcp` `/Users/sasha/.rustup/toolchains/1.97.1-aarch64-apple-darwin/bin` `1.97.1` `/Users/sasha` `/Users/sasha/.codex/worktrees/bhcp-issue-92-positive-adapter-use/experiments/in-session-evidence-agent/oracle/tests/invariants.rs`
- Subject before: `bhcp.hash/sha3-512@0:e29c178775811973c9196c860f65d87dce03467b9b42abfa9e1d724add61d06162a2a3eaa1264b73dfbd04276febb40207ae56a5ac8c14168073009c2fb944d4`
- Subject after: `bhcp.hash/sha3-512@0:e29c178775811973c9196c860f65d87dce03467b9b42abfa9e1d724add61d06162a2a3eaa1264b73dfbd04276febb40207ae56a5ac8c14168073009c2fb944d4`
- Agent executable: `bhcp.hash/sha3-512@0:ec82cf095819683495ce9727232bca350e1a030c542667f4788324b8ddd894ded6ea0eb14f554147dfecfaafd8c9372755bb25f69106d1918efc89bba57577b1`
- Agent stdout: `bhcp.hash/sha3-512@0:e8b634dc0fe5c7c2262128ed312b484fe9e5f7b8a37248c8cd197120e41c6708a3626fd58df53e5a572cf115de3994b239e8e045522139fa075a49d813179ff1`
- Agent stderr: `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
- Frozen inputs: 22
- Tokens: input 132006, cached 121984, output 2466, reasoning 1485
- Completed commands: 0
- Input `POSITIVE_USE_PROMPT.md`: `bhcp.hash/sha3-512@0:6c54bfb95b44bdb40418f1722df57680808e287a7de217b5837b9bbaa930460a7f4d2c754382ddb4cb728b531fc9933a51836b426b95344e04de2c5f579c2325`
- Input `REGISTRY_COMMAND.txt`: `bhcp.hash/sha3-512@0:8f89823d46ec326724aeff65e0b7bd295971387604978bf8d2e49f91593d3d0325181c967ecc7debaa49bc67abf08b06fbd7b06fe53d5cea4e467846fe90dc52`
- Input `TASK.md`: `bhcp.hash/sha3-512@0:09cc271029b9419c47677c14f103a88e4b80f0967abc908d370fb74808b0e1d7822fc6b6d6cd21f1f428b78afe97370727cdee21f7fa33c74fcdfcfef99a22a3`
- Input `bhcp-project.toml`: `bhcp.hash/sha3-512@0:21b92dd98500674a2d74996f29010e266d4ba3ba99d7a14c2d4815c2c6756dc0169be53a289797aa34336333fda411045fd2b52599b4ce44c1e186ae455c8acd`
- Input `candidate.cbor`: `bhcp.hash/sha3-512@0:85231ea5a4d3a02266c2d0e94faa420d1682e5cec882bc8b53451a5606a1833f012cdeb910307fa3c4b4322b765dbb1345657f254bfb3aca95ba62627c228425`
- Input `contract.bhcp`: `bhcp.hash/sha3-512@0:400f2bb1fb66645e5709263feb5f358c9fbb02fb40e4653649a07e35e4799509a656fafa2edeaee53ee875da7573219a4bad163e9fe6a9581c96de3efe7cef4e`
- Input `contract.semantic-id`: `bhcp.hash/sha3-512@0:3b94fed696cb2d3909de5745ffa2ee629796d264df5aa3837033791832606467d63c364b2228a57f91bf9f1c61312d54646e2aaf9afb5b59c36e310f48048ec2`
- Input `subject`: `directory`
- Input `subject/.agents`: `directory`
- Input `subject/.agents/skills`: `directory`
- Input `subject/.agents/skills/interpret-bhcp-contract`: `directory`
- Input `subject/.agents/skills/interpret-bhcp-contract/SKILL.md`: `bhcp.hash/sha3-512@0:8a92d3f6afcb7732464fd22b06ebc969ee540976d86b1294985a8c6349381a0cf48461a3819bdb5f40898259cddd1e4bd77e52857eb560c3e65cefa8808ac54e`
- Input `subject/.agents/skills/interpret-bhcp-contract/agents`: `directory`
- Input `subject/.agents/skills/interpret-bhcp-contract/agents/openai.yaml`: `bhcp.hash/sha3-512@0:a7edfde74b970ffbb1d527055cd8168593ef0b00a81644f4ad3a8a60559216c3564e68521758923b417d79ff12ad60142d91152a8249b397d8ffa3cf3a060647`
- Input `subject/Cargo.lock`: `bhcp.hash/sha3-512@0:77de4dcb7fb90eabe4bf3d681f9f2a631825163ff44fb953df3b86239de9bbc1009c8a665ddba27a3ea00a349f492870c90555664989dbb24e971edef6f802a1`
- Input `subject/Cargo.toml`: `bhcp.hash/sha3-512@0:b50a8b28f883706049685d287f2bd67cd939241ff21eaaed299bb7a1bcd455868efd2479a282ca4503581ebc31747caa5cee65d3caa5c4ab585d8873e757cf9e`
- Input `subject/src`: `directory`
- Input `subject/src/lib.rs`: `bhcp.hash/sha3-512@0:0a3a915d41d2a1e253bf7a8662f596e969253c89781bd9eec9d9cfdecc59635ddf5099b2b10fe38371941e3ad6f21b9cb3f7d91eca682adc5cf1f7a2f1d94487`
- Input `subject/tests`: `directory`
- Input `subject/tests/public.rs`: `bhcp.hash/sha3-512@0:ba4bea5dc920bfd5cd88d30b743b1fd127e2505af41a3488cf6e636ab2f90d72a5548679eae4f8b4185300f6b1151a44d75c813254ef20a1d4ab05a31cdd4778`
- Input `subject/tools`: `directory`
- Input `subject/tools/evidence-generalization-adapter`: `bhcp.hash/sha3-512@0:e96af61347cdd4298b181e0d72ef10e6522a9917c0657e5e319a1471d8a62a55170c08fa3fc9f22e6b27374085f935bdd2e6c60d07e0fcb888776f9a6f2d4f15`
- Judge `format`: accepted (exit Some(0), 127 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `fmt` `--check` `--manifest-path` `subject/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
- Judge `clippy`: accepted (exit Some(0), 234 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `clippy` `--offline` `--manifest-path` `subject/Cargo.toml` `--all-targets` `--` `-D` `warnings`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:f9f38ca9d89b807d722e9a01c0e50e2cb54700caef1f2031b84735d066f54cdb4c017c378687b72ea18f6bb8c689d806e5f56e6768aa46f8bdf39a460f83112e`
- Judge `public`: rejected (exit Some(101), 680 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `test` `--offline` `--manifest-path` `subject/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:18206f5315dd01f1c09ccd690e088b1bfcb5c5379dc73facd55c221adbb07b5daca2b1b56079c339da3f437919c04d661c558dba8f728ba98e0113dcf776a86c`; stderr `bhcp.hash/sha3-512@0:e02512e8c35cc17ed26d707ebcfe1ebd51ccf486c9ae92ac68f8deabc29590ec2e41a4fa7163ec23228c12d13381df7a4356a44811ef02761bb77201886e78e1`
- Judge `oracle`: rejected (exit Some(101), 401 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `test` `--offline` `--manifest-path` `oracle/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:7d2fd603274265e0d1e25a66028cbfa5473a9ddd3330e1969cbe53fbc1379b2a97775c2e581b82659cac5d91b3623d2e2dd5eab2d8d360e56d5b221fabb423bd`; stderr `bhcp.hash/sha3-512@0:e863288259b5eb59951509d47b890614bc58b45b44ab03c85b8af2504649fa016c7ddb30fa6656c534003a1b38e3a47d04dab17653f3abcbd563ccdedd5a6988`
- Judge `change-policy`: accepted (exit Some(0), 9 ms); command `/Users/sasha/.codex/worktrees/bhcp-issue-92-positive-adapter-use/target/release/evidence_generalization_adapter` `judge-change-policy`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
