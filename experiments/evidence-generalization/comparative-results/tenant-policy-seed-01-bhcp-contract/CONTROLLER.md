# Experiment evidence-generalization-comparative-tenant-policy-seed-01-bhcp-contract

- Plan: `bhcp.hash/sha3-512@0:392a66d373479bbcdb8945c90ddc45e2bb6fee1e5f5f477a9ead1360309e04e3d8a390f74bc9a45ff33bc5a64c831933caecf89ba2181fbd4a4494b470e49141`
- Fixture: `bhcp.hash/sha3-512@0:b387f7a170f43608d7aedec9c0b5046c54b691ca08cd0a00a5a422c7ea1f63a89e5240ebb1e55dd28456c148554a024b30deca1a45065fa914f7e7763774cc3e`
- Model: `gpt-5.4-mini`
- Reasoning: `medium`
- Sandbox: `workspace-write/no-network/read-confined`
- Toolchain: `codex-cli-0.142.4+rust-1.97.1`
- Run order: bhcp-contract

| Arm | Status | Claimed | Input tokens | Commands | Elapsed ms |
| --- | --- | --- | ---: | ---: | ---: |
| bhcp-contract | rejected (verification-failed) | no | 418952 | 0 | 81303 |

## Arm bhcp-contract

- Result: one or more configured judges rejected the candidate
- Total elapsed: 81303 ms
- Agent elapsed: 79231 ms
- Agent command: `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/target/release/bhcp_codex_experiment_driver` `/Users/sasha/.local/share/mise/installs/codex/0.142.4/codex-aarch64-apple-darwin` `/Users/sasha/.codex` `/Users/sasha/.cargo` `/Users/sasha/.rustup` `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/target/release/bhcp` `/Users/sasha/.rustup/toolchains/1.97.1-aarch64-apple-darwin/bin` `1.97.1` `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/experiments` `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/experiments/policy-resolution-agent/oracle/tests/invariants.rs`
- Subject before: `bhcp.hash/sha3-512@0:48d8ee26a9412bf5d15fe0ecf44bd21fe5d6060de6ac18250eaa7064a8901d456c088616e899b6106ea806e90d773d0e2d7db825973181b5dbead867c9cd9c84`
- Subject after: `bhcp.hash/sha3-512@0:48d8ee26a9412bf5d15fe0ecf44bd21fe5d6060de6ac18250eaa7064a8901d456c088616e899b6106ea806e90d773d0e2d7db825973181b5dbead867c9cd9c84`
- Agent executable: `bhcp.hash/sha3-512@0:ec82cf095819683495ce9727232bca350e1a030c542667f4788324b8ddd894ded6ea0eb14f554147dfecfaafd8c9372755bb25f69106d1918efc89bba57577b1`
- Agent stdout: `bhcp.hash/sha3-512@0:8378b9186cd26939efa293e48f1e9f6e99102a9e483288295e049aebf5b85f7fd8e58e3ccbd05e887be520cca67bd89ccb328db135ea8cfb1879e5db53f0cea5`
- Agent stderr: `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
- Frozen inputs: 14
- Tokens: input 418952, cached 358400, output 3378, reasoning 2519
- Completed commands: 0
- Input `COMPARATIVE_BHCP_PROMPT.md`: `bhcp.hash/sha3-512@0:9bdcc4d2b2120e48f06a24a887f4403faa49c1af6b2f4e5f0e432b392662455b56f980ea413b0feac0ec64456b354affdf34ef3802388bcc10fd947246887231`
- Input `TASK.md`: `bhcp.hash/sha3-512@0:06148d50cc40233d1de9dbad9d129da472ed231e32d44f22dbcfe93c651e90d6ac1109f1bb44de4bb61559251aa2a57e82c48e8c86bf72cabf259fb9c8586461`
- Input `contract.bhcp`: `bhcp.hash/sha3-512@0:99b89678ad18c2553294822f61f85325add8f89881be5cff66027b5bd90f541d357213b53395e22818ba47e3cc4c4fa93b8d6fd81c34156a96ef209bf3dbcb9a`
- Input `contract.semantic-id`: `bhcp.hash/sha3-512@0:382fc0bb10a3075e31312b49bc849879c37634eab34f740245b0f6c3065fc4b0eafd40606a3a40386eb9f7b09c413ff5e87369c4b4124c002ea9bdc047aded21`
- Input `subject`: `directory`
- Input `subject/.gitignore`: `bhcp.hash/sha3-512@0:9be9f39fca13920266e2bee5474bedf4d96abe85cd647f98406185f226130f4cbbaae2d367116c73ce50d09faab8091d8c9a1f275001fd58167f8863b66495da`
- Input `subject/.mise.toml`: `bhcp.hash/sha3-512@0:3214b7551815c603e93136d4907bc85e0a129335eb9a3ff48ef9da25f5464ae6807e3e55d308a66031577a251be42ecde76faf07142fa4863ac278bd0d734992`
- Input `subject/Cargo.lock`: `bhcp.hash/sha3-512@0:68cac31af1b9f690a2011b08d7a99e4c88aaea90a9d146ac4c7220fcb9b820292e0ecd7608cfa5bf99e8724bc8844cb66671e8a6c60a3fea7b240af7911cc523`
- Input `subject/Cargo.toml`: `bhcp.hash/sha3-512@0:ec6881ebb715e1fc0067b682ea765bba52dc65d987b27e4ff1f8ec503dc8e0083dd644d69e2daed9c6ddc28d191f14e1dc5045c10f32f663c21abe3c6f866e2e`
- Input `subject/README.md`: `bhcp.hash/sha3-512@0:7131849de9a8386a156c5a5d529c4a83328261ed618e2fff41fa008de0bf98182a75abdec934acc3f9f760c19e2cd27b02c80a8655318513dca6c6f556e8f602`
- Input `subject/src`: `directory`
- Input `subject/src/lib.rs`: `bhcp.hash/sha3-512@0:88cc56ed939073ac27ec8c828320f5a8b43579f33ed7fdb470dfa5375f62c9501dd8fd4a9e175e772adedcbd6786e7f98c2c9a33fdc17026c15a5ce82b11fd39`
- Input `subject/tests`: `directory`
- Input `subject/tests/public.rs`: `bhcp.hash/sha3-512@0:a10274c7136b09951914c394fcce5b13c2e619d3c926b83357239be8e1a1111bb4d0209083a1ea90816ada98a5d6452cd24560b18d9b1f51355a6e43864553a4`
- Judge `format`: accepted (exit Some(0), 195 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `fmt` `--check` `--manifest-path` `subject/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
- Judge `clippy`: accepted (exit Some(0), 390 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `clippy` `--offline` `--manifest-path` `subject/Cargo.toml` `--all-targets` `--` `-D` `warnings`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:f24ef73ec7f608dd18dec48ede7d0d774fab15d377657006e79c8f14a0e85ebd3903a5864dbf598ba8412625db41d906c2f554cf0aa60bb1bfed70c7addf8305`
- Judge `public`: accepted (exit Some(0), 780 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `test` `--offline` `--manifest-path` `subject/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:56701e237496771fe06d22f8e7aa372c89db1340792471b8401223603127ff56f3aa2248d16532f0fb42f69a4cfcec2e803f8f476f0c4ba87ff78adcaa177775`; stderr `bhcp.hash/sha3-512@0:55c7ae3ec6c3e722c3d123e0062f38cfda3008ea56cfbe91def3cf5b4e8efefa1010a760ca707bf3b0f809ba92df48e414fb42d4241056c189781b709b9dbf87`
- Judge `oracle`: rejected (exit Some(101), 659 ms); command `/Users/sasha/.cargo/bin/rustup` `run` `1.97.1` `cargo` `test` `--offline` `--manifest-path` `oracle/Cargo.toml`; stdout `bhcp.hash/sha3-512@0:914f731c3348499b912e32bfc57592c1fd3d73fc0e146812818832a15b9852161e1017c55ea9f11a9592ab3d92087ca9f0e7ace5918efccdd1c564318c5a4448`; stderr `bhcp.hash/sha3-512@0:b14d5cc05fb67cc464ed0ac631fee48371a3b612d9cb1af5179cd79f6108a9f1184e4fd1c6674de126d011636f8105f945df56fd48703bc1122f51962a564131`
- Judge `change-policy`: accepted (exit Some(0), 5 ms); command `/Users/sasha/.codex/worktrees/bhcp-issue-93-comparative-study/target/release/evidence_generalization_comparative_policy`; stdout `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`; stderr `bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26`
