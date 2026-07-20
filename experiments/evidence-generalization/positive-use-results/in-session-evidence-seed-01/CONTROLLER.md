# Experiment evidence-generalization-positive-in-session-evidence-seed-01

- Plan: `bhcp.hash/sha3-512@0:80547bd4645a4d8ff1a3505ce51ced901ac8d40da60ae5ccadd7e3936b99e966c81e40b1bb17b365684df7239f37e26e9381c915d8a0abc492231a85172ec2bf`
- Fixture: `bhcp.hash/sha3-512@0:3eb4f82d5f09788fe1880d0a74d2bf9a1e5a94c7802688feeef92b76c71ad4c1336ae9bfbd05ef295930206bae9cfe3a97a3a14188c438824c9fef4e360dd0bd`
- Model: `gpt-5.4-mini`
- Reasoning: `medium`
- Sandbox: `workspace-write/no-network/read-confined`
- Toolchain: `codex-cli-0.142.4+rust-1.97.1`
- Run order: seed-01

| Arm | Status | Claimed | Input tokens | Commands | Elapsed ms |
| --- | --- | --- | ---: | ---: | ---: |
| seed-01 | rejected (verification-failed) | no | 251629 | 0 | 137082 |

## Arm seed-01

- Result: one or more configured judges rejected the candidate
- Total elapsed: 137082 ms
- Agent elapsed: 135450 ms
- Agent command: `/Users/sasha/.codex/worktrees/bhcp-issue-92-positive-adapter-use/target/release/bhcp_codex_experiment_driver` `/Users/sasha/.local/share/mise/installs/codex/0.142.4/codex-aarch64-apple-darwin` `/Users/sasha/.codex` `/Users/sasha/.cargo` `/Users/sasha/.rustup` `/Users/sasha/.codex/worktrees/bhcp-issue-92-positive-adapter-use/target/release/bhcp` `/Users/sasha/.rustup/toolchains/1.97.1-aarch64-apple-darwin/bin` `1.97.1` `/Users/sasha` `/Users/sasha/.codex/worktrees/bhcp-issue-92-positive-adapter-use/experiments/in-session-evidence-agent/oracle/tests/invariants.rs`
- Subject before: `bhcp.hash/sha3-512@0:e29c178775811973c9196c860f65d87dce03467b9b42abfa9e1d724add61d06162a2a3eaa1264b73dfbd04276febb40207ae56a5ac8c14168073009c2fb944d4`
- Subject after: `bhcp.hash/sha3-512@0:e29c178775811973c9196c860f65d87dce03467b9b42abfa9e1d724add61d06162a2a3eaa1264b73dfbd04276febb40207ae56a5ac8c14168073009c2fb944d4`
- Agent executable: `bhcp.hash/sha3-512@0:ec82cf095819683495ce9727232bca350e1a030c542667f4788324b8ddd894ded6ea0eb14f554147dfecfaafd8c9372755bb25f69106d1918efc89bba57577b1`
- Agent stdout: `bhcp.hash/sha3-512@0:5dcb9b3dad7fad204298d9b01b93ef6fa0e0696c1b579c66f14d03bae5499a4b284c9caf13a35bc54a61284ae06d527cffb96f6a28c42b14f8ae32ac8ffdda77`
- Agent stderr: `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
- Frozen inputs: 22
- Tokens: input 251629, cached 227200, output 5501, reasoning 4413
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
- Judge `format`: accepted (exit Some(0), 171 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `fmt` `--check` `--manifest-path` `subject/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
- Judge `clippy`: accepted (exit Some(0), 309 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `clippy` `--offline` `--manifest-path` `subject/Cargo.toml` `--all-targets` `--` `-D` `warnings`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:e0a7bc8d6299133967ab483af4829b7c0422f6c51ae8750bdd8e9ec78ad9b5ceab6a16cb16fffe8a5db966f61f846745e6533bfdd5245e22552728957afa2959`
- Judge `public`: rejected (exit Some(101), 684 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `test` `--offline` `--manifest-path` `subject/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:7850a1092acd50cf2e0cf989f975b98826ee41104a3fc5c24ed75a5bf50242887b1858c51947674253c5ff440c1fbf632730044c93eb56d98385a17bd1be1fea`; stderr `bhcp.hash/sha3-512@0:06991881956d65b3612d5315d17fc75f3796009330c2f21e12da1d98a644ecffc78ce663c12d1d2de8282c2d048832d7f7ebc02d53a2eead3ef10613525dfdb7`
- Judge `oracle`: rejected (exit Some(101), 397 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `test` `--offline` `--manifest-path` `oracle/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:d81a2b23431c388f197f15e0fc38d689ead1fefc5f92dc319c70601ea3f301744b7e23a72de7c5af2d0af317ee01b2f98fcaf0e17bed7b4331f54b78c3250fe1`; stderr `bhcp.hash/sha3-512@0:9f78a49d2ad6cd3431c254bc60a44fc85d372bc812e405b29edf95d486e744de30f22afb2ff711eda054f23e133ba9e7cbf2e2f2a3cbcf9679395c6b253a6385`
- Judge `change-policy`: accepted (exit Some(0), 6 ms); command `/Users/sasha/.codex/worktrees/bhcp-issue-92-positive-adapter-use/target/release/evidence_generalization_adapter` `judge-change-policy`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
