# Experiment evidence-generalization-positive-contextual-policy-seed-01

- Plan: `bhcp.hash/sha3-512@0:30a669edb228b5eed2e0e4918ec4104b681b144b2acb39c56963198f58ff2ca9b918309cc39013bff6bef0d4cdcd1f915e00252c1c8bf0d9dd5e63705c01075d`
- Fixture: `bhcp.hash/sha3-512@0:c60c67013376fc78b61c918db06510297a46f697e2449011490236b59cfa36f3f095ff602b48ad248cc4578f140621be5c9da393070cfd8441f9fd8659343f89`
- Model: `gpt-5.4-mini`
- Reasoning: `medium`
- Sandbox: `workspace-write/no-network/read-confined`
- Toolchain: `codex-cli-0.142.4+rust-1.97.1`
- Run order: seed-01

| Arm | Status | Claimed | Input tokens | Commands | Elapsed ms |
| --- | --- | --- | ---: | ---: | ---: |
| seed-01 | rejected (verification-failed) | no | 143114 | 0 | 66993 |

## Arm seed-01

- Result: one or more configured judges rejected the candidate
- Total elapsed: 66993 ms
- Agent elapsed: 65319 ms
- Agent command: `/Users/sasha/.codex/worktrees/bhcp-issue-92-positive-adapter-use/target/release/bhcp_codex_experiment_driver` `/Users/sasha/.local/share/mise/installs/codex/0.142.4/codex-aarch64-apple-darwin` `/Users/sasha/.codex` `/Users/sasha/.cargo` `/Users/sasha/.rustup` `/Users/sasha/.codex/worktrees/bhcp-issue-92-positive-adapter-use/target/release/bhcp` `/Users/sasha/.rustup/toolchains/1.97.1-aarch64-apple-darwin/bin` `1.97.1` `/Users/sasha` `/Users/sasha/.codex/worktrees/bhcp-issue-92-positive-adapter-use/experiments/contextual-policy-agent/oracle/tests/invariants.rs`
- Subject before: `bhcp.hash/sha3-512@0:5ba3d35e249169c2f2a8638e2f09a69003de4bb48618533462503a191679cacedd89fd66daca81866d565e60d5036c19bc2afbba2743f6137d5723cdab8b43cd`
- Subject after: `bhcp.hash/sha3-512@0:5ba3d35e249169c2f2a8638e2f09a69003de4bb48618533462503a191679cacedd89fd66daca81866d565e60d5036c19bc2afbba2743f6137d5723cdab8b43cd`
- Agent executable: `bhcp.hash/sha3-512@0:ec82cf095819683495ce9727232bca350e1a030c542667f4788324b8ddd894ded6ea0eb14f554147dfecfaafd8c9372755bb25f69106d1918efc89bba57577b1`
- Agent stdout: `bhcp.hash/sha3-512@0:bd7004e8581b98f943dc27fc1ee7cd6ca0cc45dd3a0f5715c696ad8038498f7fd08c65157f29b9a73cf4e67bc2cb32fd9d35e67fe029da49a1cbce9d86941e9a`
- Agent stderr: `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
- Frozen inputs: 23
- Tokens: input 143114, cached 132608, output 3207, reasoning 2387
- Completed commands: 0
- Input `POSITIVE_USE_PROMPT.md`: `bhcp.hash/sha3-512@0:6c54bfb95b44bdb40418f1722df57680808e287a7de217b5837b9bbaa930460a7f4d2c754382ddb4cb728b531fc9933a51836b426b95344e04de2c5f579c2325`
- Input `REGISTRY_COMMAND.txt`: `bhcp.hash/sha3-512@0:2bf8825854d3e6d958365a851d62fa945d6c7a4f70ac86c62aca662e93c690b71682809ba6e6753709b786521da4af2dc71a0ddcb57f2e07209532d21cb684e8`
- Input `TASK.md`: `bhcp.hash/sha3-512@0:0231ab12f5a147baa1c2c92fb262749e94acadeef0bcc665ac780048a424b326ecef294e16cef5ec45388245fcb04a14b1788826f7357e742010a21de8dbf9d1`
- Input `bhcp-project.toml`: `bhcp.hash/sha3-512@0:28a677ac51939977d661344157d78d3f68696961e262b86ec05bf8dfeecf227651dfc50cc6a8bb6f99ea0fb97584a53ad4b33fc3f620c0002b92d4ed4ef1543c`
- Input `candidate.cbor`: `bhcp.hash/sha3-512@0:a52a0ef5d76cd4949e464474ddfb2ffa1bdd302fc65fab6c2f124b7aa13d0b1cdf0c0736aaa6522090dec979bc4058349221d289b9702156894cd0c49047f84d`
- Input `contract.bhcp`: `bhcp.hash/sha3-512@0:500df232b642efabea4ef362487f676b629c5874e479f78f4e980f54445980a30e1363160b787885e181c136785d29d987f1fd34bd739f4b19b3ef4451806976`
- Input `contract.semantic-id`: `bhcp.hash/sha3-512@0:d45d4926442ca393efcf5bd5bf335fda3acfef0173f8f1bd1d6baf7f18f3087676cb074e814c17725dec089558223a622420738fa9a9fcaf9983e392e38c7b3e`
- Input `subject`: `directory`
- Input `subject/.agents`: `directory`
- Input `subject/.agents/skills`: `directory`
- Input `subject/.agents/skills/interpret-bhcp-contract`: `directory`
- Input `subject/.agents/skills/interpret-bhcp-contract/SKILL.md`: `bhcp.hash/sha3-512@0:8a92d3f6afcb7732464fd22b06ebc969ee540976d86b1294985a8c6349381a0cf48461a3819bdb5f40898259cddd1e4bd77e52857eb560c3e65cefa8808ac54e`
- Input `subject/.agents/skills/interpret-bhcp-contract/agents`: `directory`
- Input `subject/.agents/skills/interpret-bhcp-contract/agents/openai.yaml`: `bhcp.hash/sha3-512@0:a7edfde74b970ffbb1d527055cd8168593ef0b00a81644f4ad3a8a60559216c3564e68521758923b417d79ff12ad60142d91152a8249b397d8ffa3cf3a060647`
- Input `subject/Cargo.lock`: `bhcp.hash/sha3-512@0:dc2e3465ee77e06935ddbf01ddd34edf24304ef37ad1dfd3ce8c69f32c54b93f292e4e42c32fa1beca77fcf7110f8dbed034a2c2ab38d808b8452cc0f321d9e0`
- Input `subject/Cargo.toml`: `bhcp.hash/sha3-512@0:eba8081e69f405f273b2b477cfb629cb2b2649080829473d7c738f0dec8151d10a89211c6eca4a6776e604c2c0e0992f28ece454210a1e3b7612fa62ad21782b`
- Input `subject/README.md`: `bhcp.hash/sha3-512@0:a6e4fb450a76aaa0ac75cd1189c301a5eaa803d0fa2a206c19ccafa7ab19ed679391dd72dfd71f84f347035eb52f6029687a5741b7ece015da06829fb5b9a6f1`
- Input `subject/src`: `directory`
- Input `subject/src/lib.rs`: `bhcp.hash/sha3-512@0:fbd107e18aa1420486f69f71888dac5c0e641c2df8eea5a010eeeea5f928c4bfcbe764a03043575d3f1c697685d88ff2b329a75a3cd6a56d7ce2e0729df93b93`
- Input `subject/tests`: `directory`
- Input `subject/tests/public.rs`: `bhcp.hash/sha3-512@0:749d44720b6120c732f9d75e1882ee0c933c65327e939fb028b60580dcf0b8a41d7029680556b12c1405c6485f6552a3a8152d32e10ea48c845d69a6137b911d`
- Input `subject/tools`: `directory`
- Input `subject/tools/evidence-generalization-adapter`: `bhcp.hash/sha3-512@0:e96af61347cdd4298b181e0d72ef10e6522a9917c0657e5e319a1471d8a62a55170c08fa3fc9f22e6b27374085f935bdd2e6c60d07e0fcb888776f9a6f2d4f15`
- Judge `format`: accepted (exit Some(0), 55 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `fmt` `--check` `--manifest-path` `subject/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
- Judge `clippy`: accepted (exit Some(0), 125 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `clippy` `--offline` `--manifest-path` `subject/Cargo.toml` `--all-targets` `--` `-D` `warnings`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:e58bbf86f04d590ce27eecf637b520e121c4e0bb82bf0579feef9c37ef23edca705ca4a13bc3f876a7075ce84e0f8d0c24b9e2d783a86443442ef08c4e5a1080`
- Judge `public`: accepted (exit Some(0), 749 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `test` `--offline` `--manifest-path` `subject/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:cb86ed71209e241c579b93ec5cc33d2f8aa1df60ea4a30d68c658601e968ddd47c04c3fc2b7b4fa406fb806b9bd3332a3cd4a42820fdff7435a7aee883f002d1`; stderr `bhcp.hash/sha3-512@0:2de9c71cd0fe1f4d36f08f61595a0a2c2966f75e6c0dcaf031393e1f72a19651a5d1bf1b6f5552e0d4fc9838bdfd7991efedca16925de6ceaad730e830b6a657`
- Judge `oracle`: rejected (exit Some(101), 664 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `test` `--offline` `--manifest-path` `oracle/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:d586fb3c96f4a60fd33360da0d38fb9c83dc172e7d8489e44734e65e8b843f2c34315a16d9e9d980c3fbcd3e5cc4860b27bd731ab2b07bc1cee260df49630a94`; stderr `bhcp.hash/sha3-512@0:382282f3c2b85803081edca68f44e641c33f666c240dad28373c6861ca313e3945576022a31d8d254d3c523ad51463604d9e86a0bb0c1053b7b1b3dd8fafe368`
- Judge `change-policy`: accepted (exit Some(0), 7 ms); command `/Users/sasha/.codex/worktrees/bhcp-issue-92-positive-adapter-use/target/release/evidence_generalization_adapter` `judge-change-policy`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
