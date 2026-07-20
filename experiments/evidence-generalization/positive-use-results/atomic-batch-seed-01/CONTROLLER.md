# Experiment evidence-generalization-positive-atomic-batch-seed-01

- Plan: `bhcp.hash/sha3-512@0:ba555f0982087e824cb21ef103233131c2e77a79936950a7b6af3502fa20ee4e790d1122c92263a8bb4d638fa4eb148da7d0f4adff52802671708166fa49cb80`
- Fixture: `bhcp.hash/sha3-512@0:f7f43003dd7f40156cb8aca6fc44f6b066fe1b2ced41b1b9bb5131e041a4dc3858b9aec7c877221f4a70ce3b25632840bc208ed499aec0e5969a4f53beb3da5d`
- Model: `gpt-5.4-mini`
- Reasoning: `medium`
- Sandbox: `workspace-write/no-network/read-confined`
- Toolchain: `codex-cli-0.142.4+rust-1.97.1`
- Run order: seed-01

| Arm | Status | Claimed | Input tokens | Commands | Elapsed ms |
| --- | --- | --- | ---: | ---: | ---: |
| seed-01 | rejected (verification-failed) | no | 146063 | 0 | 69514 |

## Arm seed-01

- Result: one or more configured judges rejected the candidate
- Total elapsed: 69514 ms
- Agent elapsed: 67316 ms
- Agent command: `/Users/sasha/.codex/worktrees/bhcp-issue-92-positive-adapter-use/target/release/bhcp_codex_experiment_driver` `/Users/sasha/.local/share/mise/installs/codex/0.142.4/codex-aarch64-apple-darwin` `/Users/sasha/.codex` `/Users/sasha/.cargo` `/Users/sasha/.rustup` `/Users/sasha/.codex/worktrees/bhcp-issue-92-positive-adapter-use/target/release/bhcp` `/Users/sasha/.rustup/toolchains/1.97.1-aarch64-apple-darwin/bin` `1.97.1` `/Users/sasha` `/Users/sasha/.codex/worktrees/bhcp-issue-92-positive-adapter-use/experiments/minimal-coding-agent/oracle/tests/invariants.rs`
- Subject before: `bhcp.hash/sha3-512@0:cac1add89cec8b64803fc0a53aa449d2606124c13ec9a1208bbd62e9fb4acd63bef49e8629fcb063d3a9f7ae79fe11c94b78237aef19499acc1bfc9ea84a12c4`
- Subject after: `bhcp.hash/sha3-512@0:cac1add89cec8b64803fc0a53aa449d2606124c13ec9a1208bbd62e9fb4acd63bef49e8629fcb063d3a9f7ae79fe11c94b78237aef19499acc1bfc9ea84a12c4`
- Agent executable: `bhcp.hash/sha3-512@0:ec82cf095819683495ce9727232bca350e1a030c542667f4788324b8ddd894ded6ea0eb14f554147dfecfaafd8c9372755bb25f69106d1918efc89bba57577b1`
- Agent stdout: `bhcp.hash/sha3-512@0:5df20ac94f88692572704e6aae6457eddb98b69dbc605459e5f5b415702a54408189850d01eab5545a4f6ff9450c91ef60b0d52056b03e2403796709d355608c`
- Agent stderr: `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
- Frozen inputs: 25
- Tokens: input 146063, cached 129664, output 2916, reasoning 2417
- Completed commands: 0
- Input `POSITIVE_USE_PROMPT.md`: `bhcp.hash/sha3-512@0:6c54bfb95b44bdb40418f1722df57680808e287a7de217b5837b9bbaa930460a7f4d2c754382ddb4cb728b531fc9933a51836b426b95344e04de2c5f579c2325`
- Input `REGISTRY_COMMAND.txt`: `bhcp.hash/sha3-512@0:4f7accfc36d344cc647565a57b1a4f1844db23bcf277b4f4b2e27d0452c9cda43b082abe496a97db7c6f0069e45064e359e91b9e6768797fc4a1aba7ecc4e58e`
- Input `TASK.md`: `bhcp.hash/sha3-512@0:59af0a2126f51fac806451cf5c880ef382f97da86a285e52bca617a6d852d964ecfd98dc0e60b98ef3945c62287c4e48ad656b004e68fa5da3992b058c662ed4`
- Input `bhcp-project.toml`: `bhcp.hash/sha3-512@0:ccf207b8cea03fe20d662acc6092942fa10bd31d95d269f1f6b5b8746267037a05350f6339c62adfe6e2a1dbebb10fe9bab50feda576bc424185408689e1c63a`
- Input `candidate.cbor`: `bhcp.hash/sha3-512@0:c2c441b6c810ff8ac44025b093de33cd26b38ce8f263cad607bd513e3b610396b542d439e628a1c5d1236a808994bdac2caba45906dae8f8105066a9e379d982`
- Input `contract.bhcp`: `bhcp.hash/sha3-512@0:721b099b961c721ae9e7d390992e2113d5e3e8fe4055e5ac043218201b7a674eb1e968a7c738c75ea32acd2d27978793877460b4f437fd18ea4b1267c1e9e2a3`
- Input `contract.semantic-id`: `bhcp.hash/sha3-512@0:78c1b10dadb4b629c327335fe22567abb41a9b36404a10af527b102bcc91b689b9f17c645dcdbebb21b5e5cf74633d069eded3dd91b52ca6befec35b5ef21065`
- Input `subject`: `directory`
- Input `subject/.agents`: `directory`
- Input `subject/.agents/skills`: `directory`
- Input `subject/.agents/skills/interpret-bhcp-contract`: `directory`
- Input `subject/.agents/skills/interpret-bhcp-contract/SKILL.md`: `bhcp.hash/sha3-512@0:8a92d3f6afcb7732464fd22b06ebc969ee540976d86b1294985a8c6349381a0cf48461a3819bdb5f40898259cddd1e4bd77e52857eb560c3e65cefa8808ac54e`
- Input `subject/.agents/skills/interpret-bhcp-contract/agents`: `directory`
- Input `subject/.agents/skills/interpret-bhcp-contract/agents/openai.yaml`: `bhcp.hash/sha3-512@0:a7edfde74b970ffbb1d527055cd8168593ef0b00a81644f4ad3a8a60559216c3564e68521758923b417d79ff12ad60142d91152a8249b397d8ffa3cf3a060647`
- Input `subject/.gitignore`: `bhcp.hash/sha3-512@0:9be9f39fca13920266e2bee5474bedf4d96abe85cd647f98406185f226130f4cbbaae2d367116c73ce50d09faab8091d8c9a1f275001fd58167f8863b66495da`
- Input `subject/.mise.toml`: `bhcp.hash/sha3-512@0:3214b7551815c603e93136d4907bc85e0a129335eb9a3ff48ef9da25f5464ae6807e3e55d308a66031577a251be42ecde76faf07142fa4863ac278bd0d734992`
- Input `subject/Cargo.lock`: `bhcp.hash/sha3-512@0:2b82aa9988ed745dd53072f239e76460274ae7a66c9e68877e3ba433cdb109df7fded5630c004c55bbcfd5e5c78be8fb2887e678e19e46a0f933b350f84c50a5`
- Input `subject/Cargo.toml`: `bhcp.hash/sha3-512@0:855680b6969f1feac11f94bc59db68a93a8aad2682d71cb3ae3e9db9fb7252dbad2d6cf8cb2be252d0ec63463fa6176377e4d36c89649ace6be6cc49c40f3ba6`
- Input `subject/README.md`: `bhcp.hash/sha3-512@0:3ff20154a8a0207aac7c83487868d2ebe4e4322a0871d112af441eea09b4b9eaf2de52db6e33ca83e9689c2473576f67a661f5abb3437fc17bcaa01eed347991`
- Input `subject/src`: `directory`
- Input `subject/src/lib.rs`: `bhcp.hash/sha3-512@0:6c0ff68252f4e970c135a50014914ac5fc2d015b79a15979e30a0bddc0f162005e1965a997ec8afd43f2c42d717f39a570ab198893364527a7eb8eb98eb8563b`
- Input `subject/tests`: `directory`
- Input `subject/tests/public.rs`: `bhcp.hash/sha3-512@0:58bc6418c1cd392f6c5103c991f2a29e5c1cdba93b9bfa4bec5919e872604bb04d505b1fef0fc512e33b125383cfd22d4b95116d8920fdd0ace8a81ae5b74bff`
- Input `subject/tools`: `directory`
- Input `subject/tools/evidence-generalization-adapter`: `bhcp.hash/sha3-512@0:e96af61347cdd4298b181e0d72ef10e6522a9917c0657e5e319a1471d8a62a55170c08fa3fc9f22e6b27374085f935bdd2e6c60d07e0fcb888776f9a6f2d4f15`
- Judge `format`: accepted (exit Some(0), 69 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `fmt` `--check` `--manifest-path` `subject/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
- Judge `clippy`: accepted (exit Some(0), 177 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `clippy` `--offline` `--manifest-path` `subject/Cargo.toml` `--all-targets` `--` `-D` `warnings`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:e2b0deb9fcd46cc71cbbdcacb5d3df5c9d270940ab9666f3f82c3ebe80f3deecf19f79193c1ad472cfb1ff176247f1cb85bb743887d9f034e6e1febb74afb9e9`
- Judge `public`: accepted (exit Some(0), 884 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `test` `--offline` `--manifest-path` `subject/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:cae2ad989e614d519cd77cd723475c869fd970946aeee30dc837e1b82d549bccefb48c5fa3ff4dc4b3ec00607c07d7413f2a667bbd8cd34b4d8235a4dafeb916`; stderr `bhcp.hash/sha3-512@0:0511bbdd43e920b56ce804ed9d66fedcac670aeab8133d6cf04ae6c95455f4df47c980ed31b82412c5e0945ef6501d7936e70ea916dc0b7fe645d08db565883d`
- Judge `oracle`: rejected (exit Some(101), 768 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `test` `--offline` `--manifest-path` `oracle/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:3d103b2bd06909438eda2eb8f02b05c139209d4b583d5f6c960f0157f1ad265040680a7ac7fcd955bb0dc6aa091a4786fee1665159c93b0e07e260097894fbf0`; stderr `bhcp.hash/sha3-512@0:b7bd66a2cca05f90a8cc2db848f76aea48b90c3864dbb9266f05fa1f376a5f5f5f811c15aa483b19ec7a025f8ec6d5d35436d596a3833a1bda0a6e50f261f3ec`
- Judge `change-policy`: accepted (exit Some(0), 214 ms); command `/Users/sasha/.codex/worktrees/bhcp-issue-92-positive-adapter-use/target/release/evidence_generalization_adapter` `judge-change-policy`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
