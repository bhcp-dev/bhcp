use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: impl AsRef<Path>) -> String {
    fs::read_to_string(path.as_ref())
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.as_ref().display()))
}

fn values(line: &str) -> BTreeMap<&str, &str> {
    line.split('|')
        .filter_map(|field| field.split_once('='))
        .collect()
}

fn count_files(path: &Path) -> usize {
    fs::read_dir(path)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .map(|entry| {
            if entry.is_dir() {
                count_files(&entry)
            } else {
                1
            }
        })
        .sum()
}

#[test]
fn comparative_results_reproduce_the_frozen_null_estimate() {
    let repository = root();
    let result_root = repository.join("experiments/evidence-generalization/comparative-results");
    assert_eq!(count_files(&result_root), 75);
    assert!(!result_root.join("STOPPED.md").exists());
    assert_eq!(
        read(result_root.join("AUTHORITY.txt")),
        "git-head=0f406371db9b47f3d966d609fab8d31ddeb759fe\nowner-attestation-merge=c327d80308dbf6010321ca05ef498493b04350e7\nauth-mode=chatgpt\nincremental-usd=0\nconcurrency=1\nprior-input-tokens=1961148\nprior-output-tokens=40755\nprior-reasoning-tokens=31916\n"
    );

    let registration =
        read(repository.join("experiments/evidence-generalization/comparative-registration.txt"));
    let ledger = read(result_root.join("RESULTS.txt"));
    assert_eq!(ledger.lines().count(), 25);
    assert_eq!(
        ledger.lines().next(),
        Some("version|bhcp-evidence-generalization-comparative-results@0")
    );

    let mut sessions = 0;
    let mut input = 0_u64;
    let mut cached = 0_u64;
    let mut output = 0_u64;
    let mut reasoning = 0_u64;
    let mut model_millis = 0_u64;
    for registered in registration
        .lines()
        .filter(|line| line.starts_with("session|"))
    {
        let fields = registered.split('|').collect::<Vec<_>>();
        assert_eq!(fields.len(), 8);
        let task = fields[1];
        let seed = fields[2];
        let position = fields[3];
        let arm = fields[4].strip_prefix("arm=").unwrap();
        let plan = fields[5];
        let fixture = fields[6];
        let directory = result_root.join(format!("{task}-{seed}-{arm}"));
        assert!(directory.is_dir());
        assert_eq!(fs::read(directory.join("candidate.patch")).unwrap(), b"");

        let session = read(directory.join("SESSION.txt"));
        for exact in [
            format!("task={task}"),
            format!("seed={seed}"),
            position.to_owned(),
            format!("arm={arm}"),
            "accepted=false".to_owned(),
            "claimed-success=false".to_owned(),
            "calibrated=true".to_owned(),
            "excluded=false".to_owned(),
            "failure-category=verification:oracle".to_owned(),
        ] {
            assert!(session.lines().any(|line| line == exact), "{exact}");
        }

        let controller = read(directory.join("CONTROLLER.md"));
        for exact in [
            format!("- Plan: `{plan}`"),
            format!("- Fixture: `{fixture}`"),
            "- Model: `gpt-5.4-mini`".to_owned(),
            "- Reasoning: `medium`".to_owned(),
            "- Sandbox: `workspace-write/no-network/read-confined`".to_owned(),
            "- Toolchain: `codex-cli-0.142.4+rust-1.97.1`".to_owned(),
            format!("- Run order: {arm}"),
            "- Completed commands: 0".to_owned(),
        ] {
            assert!(controller.lines().any(|line| line == exact), "{exact}");
        }
        assert!(controller.contains(&format!("| {arm} | rejected (verification-failed) | no |")));
        for judge in ["format", "clippy", "public", "change-policy"] {
            assert!(controller.contains(&format!("- Judge `{judge}`: accepted")));
        }
        assert!(controller.contains("- Judge `oracle`: rejected"));
        let before = controller
            .lines()
            .find_map(|line| line.strip_prefix("- Subject before: "))
            .unwrap();
        let after = controller
            .lines()
            .find_map(|line| line.strip_prefix("- Subject after: "))
            .unwrap();
        assert_eq!(before, after, "{task}/{seed}/{arm} changed the starter");
        match arm {
            "prose-control" => {
                assert!(controller.contains("- Input `PROSE_TASK.md`:"));
                assert!(!controller.contains("- Input `TASK.md`:"));
                assert!(!controller.contains("- Input `contract.bhcp`:"));
            }
            "bhcp-contract" => {
                for input in ["TASK.md", "contract.bhcp", "contract.semantic-id"] {
                    assert!(controller.contains(&format!("- Input `{input}`:")));
                }
                assert!(!controller.contains("- Input `PROSE_TASK.md`:"));
            }
            _ => panic!("unexpected arm: {arm}"),
        }

        let prefix = format!("session|{task}|{seed}|{position}|arm={arm}|");
        let record = ledger
            .lines()
            .find(|line| line.starts_with(&prefix))
            .unwrap_or_else(|| panic!("missing result for {task}/{seed}/{arm}"));
        let values = values(record);
        for (key, expected) in [
            ("accepted", "false"),
            ("claimed", "false"),
            ("calibrated", "true"),
            ("excluded", "false"),
            ("failure", "verification:oracle"),
            ("commands", "0"),
        ] {
            assert_eq!(values.get(key), Some(&expected), "{record}");
        }
        input += values["input"].parse::<u64>().unwrap();
        cached += values["cached"].parse::<u64>().unwrap();
        output += values["output"].parse::<u64>().unwrap();
        reasoning += values["reasoning"].parse::<u64>().unwrap();
        model_millis += values["model-ms"].parse::<u64>().unwrap();
        sessions += 1;
    }
    assert_eq!(sessions, 24);
    assert_eq!(
        (input, cached, output, reasoning, model_millis),
        (9_139_153, 7_947_136, 99_550, 79_708, 2_130_651)
    );
    assert!(1_961_148 + input < 12_000_000);
    assert!(40_755 + output < 500_000);
    assert!(31_916 + reasoning < 500_000);

    let report = read(result_root.join("README.md"));
    for exact in [
        "`24` session records with `0` infrastructure exclusions",
        "Arm acceptance: **prose-control 0/12; bhcp-contract 0/12**",
        "paired risk difference +0.0000",
        "12 included blocks",
        "discordants BHCP-only `0`, prose-only `0`",
        "exact McNemar `p=1.000000`",
        "Arm claim calibration: **prose-control 12/12; bhcp-contract 12/12**",
        "9139153 input, 7947136 cached input, 99550 output, 79708 reasoning tokens; 35.511 model-minutes",
        "observed spend: **USD 0**",
    ] {
        assert!(report.contains(exact), "aggregate report omitted {exact}");
    }
}
