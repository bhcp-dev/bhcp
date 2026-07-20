# Experiment evidence-generalization-positive-tenant-policy-seed-02

- Plan: `bhcp.hash/sha3-512@0:af4e641254d81a96baf391d473dee8adf7de5f8d96e17b0782b704db28faa5f3b4844fce8e6e62d7a8925361519276d9840b1e8abdf28682b570e23ccb749c4f`
- Fixture: `bhcp.hash/sha3-512@0:18f46b06f9997effd973b1e94da9fa9394dc2df16050b9b14d12434685500e0456e222910d98d8d1860ba2febcac563af541f9400a616a5aee16804db2a60fe3`
- Model: `gpt-5.4-mini`
- Reasoning: `medium`
- Sandbox: `workspace-write/no-network/read-confined`
- Toolchain: `codex-cli-0.142.4+rust-1.97.1`
- Run order: seed-02

| Arm | Status | Claimed | Input tokens | Commands | Elapsed ms |
| --- | --- | --- | ---: | ---: | ---: |
| seed-02 | rejected (verification-failed) | no | 157107 | 0 | 65578 |

## Arm seed-02

- Result: one or more configured judges rejected the candidate
- Total elapsed: 65578 ms
- Agent elapsed: 63942 ms
- Agent command: `/Users/sasha/.codex/worktrees/bhcp-issue-92-positive-adapter-use/target/release/bhcp_codex_experiment_driver` `/Users/sasha/.local/share/mise/installs/codex/0.142.4/codex-aarch64-apple-darwin` `/Users/sasha/.codex` `/Users/sasha/.cargo` `/Users/sasha/.rustup` `/Users/sasha/.codex/worktrees/bhcp-issue-92-positive-adapter-use/target/release/bhcp` `/Users/sasha/.rustup/toolchains/1.97.1-aarch64-apple-darwin/bin` `1.97.1` `/Users/sasha` `/Users/sasha/.codex/worktrees/bhcp-issue-92-positive-adapter-use/experiments/policy-resolution-agent/oracle/tests/invariants.rs`
- Subject before: `bhcp.hash/sha3-512@0:df53569fd85d24682e236b32fe70596bbd73fea7b84c62f8c25fd7e89800bf313623f304489c60a4f72d3c274172f0661dfc25d8a81ebf8c081f20710463bf0b`
- Subject after: `bhcp.hash/sha3-512@0:df53569fd85d24682e236b32fe70596bbd73fea7b84c62f8c25fd7e89800bf313623f304489c60a4f72d3c274172f0661dfc25d8a81ebf8c081f20710463bf0b`
- Agent executable: `bhcp.hash/sha3-512@0:ec82cf095819683495ce9727232bca350e1a030c542667f4788324b8ddd894ded6ea0eb14f554147dfecfaafd8c9372755bb25f69106d1918efc89bba57577b1`
- Agent stdout: `bhcp.hash/sha3-512@0:de74ed7119b90957489b6a27ca2ac95510d96479d07f7bb8aa726be50ff7b4ef71eac34a1de47b08b2bd3cadb4139b4b45f23889f249e273fda4641d7c903ec3`
- Agent stderr: `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
- Frozen inputs: 25
- Tokens: input 157107, cached 141184, output 3153, reasoning 2480
- Completed commands: 0
- Input `POSITIVE_USE_PROMPT.md`: `bhcp.hash/sha3-512@0:6c54bfb95b44bdb40418f1722df57680808e287a7de217b5837b9bbaa930460a7f4d2c754382ddb4cb728b531fc9933a51836b426b95344e04de2c5f579c2325`
- Input `REGISTRY_COMMAND.txt`: `bhcp.hash/sha3-512@0:16b98328d44fd680e9144bd12569f07f3a4782ea05f89a88b9c76910482b5c7c8057d87acb7a34ae05d8fe1239acac1148390bafe2ea6154ca7ecb3863a579d7`
- Input `TASK.md`: `bhcp.hash/sha3-512@0:06148d50cc40233d1de9dbad9d129da472ed231e32d44f22dbcfe93c651e90d6ac1109f1bb44de4bb61559251aa2a57e82c48e8c86bf72cabf259fb9c8586461`
- Input `bhcp-project.toml`: `bhcp.hash/sha3-512@0:7d7902db4526ee50643ac070b189b79c133174c81c58e72f08da6db390498895ed88fb675fdd11cb44111e3ba701ed73ae53d30c499e17482bf8efc1008de1ae`
- Input `candidate.cbor`: `bhcp.hash/sha3-512@0:0c9d9e135f1f12347dbdad6b724b65f298e1e33a3ef9005c9687f51dde63d975966517c323094ac475bb63a2e78c9f197b9dd728af43a9f09629ff30d193ab7c`
- Input `contract.bhcp`: `bhcp.hash/sha3-512@0:99b89678ad18c2553294822f61f85325add8f89881be5cff66027b5bd90f541d357213b53395e22818ba47e3cc4c4fa93b8d6fd81c34156a96ef209bf3dbcb9a`
- Input `contract.semantic-id`: `bhcp.hash/sha3-512@0:382fc0bb10a3075e31312b49bc849879c37634eab34f740245b0f6c3065fc4b0eafd40606a3a40386eb9f7b09c413ff5e87369c4b4124c002ea9bdc047aded21`
- Input `subject`: `directory`
- Input `subject/.agents`: `directory`
- Input `subject/.agents/skills`: `directory`
- Input `subject/.agents/skills/interpret-bhcp-contract`: `directory`
- Input `subject/.agents/skills/interpret-bhcp-contract/SKILL.md`: `bhcp.hash/sha3-512@0:8a92d3f6afcb7732464fd22b06ebc969ee540976d86b1294985a8c6349381a0cf48461a3819bdb5f40898259cddd1e4bd77e52857eb560c3e65cefa8808ac54e`
- Input `subject/.agents/skills/interpret-bhcp-contract/agents`: `directory`
- Input `subject/.agents/skills/interpret-bhcp-contract/agents/openai.yaml`: `bhcp.hash/sha3-512@0:a7edfde74b970ffbb1d527055cd8168593ef0b00a81644f4ad3a8a60559216c3564e68521758923b417d79ff12ad60142d91152a8249b397d8ffa3cf3a060647`
- Input `subject/.gitignore`: `bhcp.hash/sha3-512@0:9be9f39fca13920266e2bee5474bedf4d96abe85cd647f98406185f226130f4cbbaae2d367116c73ce50d09faab8091d8c9a1f275001fd58167f8863b66495da`
- Input `subject/.mise.toml`: `bhcp.hash/sha3-512@0:3214b7551815c603e93136d4907bc85e0a129335eb9a3ff48ef9da25f5464ae6807e3e55d308a66031577a251be42ecde76faf07142fa4863ac278bd0d734992`
- Input `subject/Cargo.lock`: `bhcp.hash/sha3-512@0:68cac31af1b9f690a2011b08d7a99e4c88aaea90a9d146ac4c7220fcb9b820292e0ecd7608cfa5bf99e8724bc8844cb66671e8a6c60a3fea7b240af7911cc523`
- Input `subject/Cargo.toml`: `bhcp.hash/sha3-512@0:ec6881ebb715e1fc0067b682ea765bba52dc65d987b27e4ff1f8ec503dc8e0083dd644d69e2daed9c6ddc28d191f14e1dc5045c10f32f663c21abe3c6f866e2e`
- Input `subject/README.md`: `bhcp.hash/sha3-512@0:7131849de9a8386a156c5a5d529c4a83328261ed618e2fff41fa008de0bf98182a75abdec934acc3f9f760c19e2cd27b02c80a8655318513dca6c6f556e8f602`
- Input `subject/src`: `directory`
- Input `subject/src/lib.rs`: `bhcp.hash/sha3-512@0:88cc56ed939073ac27ec8c828320f5a8b43579f33ed7fdb470dfa5375f62c9501dd8fd4a9e175e772adedcbd6786e7f98c2c9a33fdc17026c15a5ce82b11fd39`
- Input `subject/tests`: `directory`
- Input `subject/tests/public.rs`: `bhcp.hash/sha3-512@0:a10274c7136b09951914c394fcce5b13c2e619d3c926b83357239be8e1a1111bb4d0209083a1ea90816ada98a5d6452cd24560b18d9b1f51355a6e43864553a4`
- Input `subject/tools`: `directory`
- Input `subject/tools/evidence-generalization-adapter`: `bhcp.hash/sha3-512@0:e96af61347cdd4298b181e0d72ef10e6522a9917c0657e5e319a1471d8a62a55170c08fa3fc9f22e6b27374085f935bdd2e6c60d07e0fcb888776f9a6f2d4f15`
- Judge `format`: accepted (exit Some(0), 53 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `fmt` `--check` `--manifest-path` `subject/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
- Judge `clippy`: accepted (exit Some(0), 123 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `clippy` `--offline` `--manifest-path` `subject/Cargo.toml` `--all-targets` `--` `-D` `warnings`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:a3d593428a9a8d2f5c9313d5ab0f3afcf4deb6230bf058caf2b781469c3d70ef4be59080f0245db257e7cf423b3ffebdf5dddcaed0c14f01a9dea8bab90c0b54`
- Judge `public`: accepted (exit Some(0), 727 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `test` `--offline` `--manifest-path` `subject/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:0b68168e74cfaa974e5546d2acb6453e710bb5b7e2df7e7397abcfa1a6cbabacb9bf3bd77b03a81d6f5967bae440eb0499f56c6cdb2c8bab57dcaf38d1ac69e0`; stderr `bhcp.hash/sha3-512@0:bda68894d3db58b9e50854aad490af335b2401b0e99bccb1107fd9cbb239b1d90a0b8b792cc0f0dbc52f254101c32b941b6f19dd6f9768d6803c77eacc611c19`
- Judge `oracle`: rejected (exit Some(101), 650 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `test` `--offline` `--manifest-path` `oracle/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:545dd882edfec292ee7e49db0a0e43c97388db2e56db6071cc3608d03bb4d913195c9791d1c2214139d3458090212cfa95b682f3bf5bab6f834c979d4dabfbc9`; stderr `bhcp.hash/sha3-512@0:b1636c83b76e84d5d450394f05ac15ca08374a4bdb54935ee0a0db3933abd2687e7ead5c4fbf5fe5524aa568bafe07e48cf9577ebda17a2a0769952d47e6c6ab`
- Judge `change-policy`: accepted (exit Some(0), 7 ms); command `/Users/sasha/.codex/worktrees/bhcp-issue-92-positive-adapter-use/target/release/evidence_generalization_adapter` `judge-change-policy`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
