use std::process::ExitCode;

fn main() -> ExitCode {
    let mode = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "complete".to_owned());
    match mode.as_str() {
        "complete" => {
            if std::path::Path::new("oracle").exists() || !std::path::Path::new("subject").is_dir()
            {
                return ExitCode::from(71);
            }
            println!("bhcp-agent-result@0");
            println!("status=completed");
            println!("model={}", std::env::var("BHCP_EXPERIMENT_MODEL").unwrap());
            println!(
                "reasoning={}",
                std::env::var("BHCP_EXPERIMENT_REASONING").unwrap()
            );
            println!(
                "sandbox={}",
                std::env::var("BHCP_EXPERIMENT_SANDBOX").unwrap()
            );
            println!(
                "toolchain={}",
                std::env::var("BHCP_EXPERIMENT_TOOLCHAIN").unwrap()
            );
            println!("claimed_success=false");
            println!("input_tokens=10");
            println!("cached_input_tokens=4");
            println!("output_tokens=3");
            println!("reasoning_tokens=2");
            println!("completed_commands=1");
            ExitCode::SUCCESS
        }
        "interrupted" => ExitCode::from(70),
        "incomplete" => {
            println!("bhcp-agent-result@0");
            println!("status=completed");
            ExitCode::SUCCESS
        }
        "adaptive" => {
            std::fs::create_dir("oracle").unwrap();
            ExitCode::SUCCESS
        }
        "contaminated" => {
            std::fs::write("TASK.md", "changed after intake\n").unwrap();
            ExitCode::SUCCESS
        }
        _ => ExitCode::from(64),
    }
}
