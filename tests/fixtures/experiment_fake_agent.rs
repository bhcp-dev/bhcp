use std::process::ExitCode;

#[cfg(unix)]
#[allow(clippy::zombie_processes)]
fn spawn_background_process_for_controller_test() {
    std::process::Command::new("/bin/sleep")
        .arg("5")
        .spawn()
        .unwrap();
}

fn emit(model: &str) {
    println!("bhcp-agent-result@0");
    println!("status=completed");
    println!("model={model}");
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
}

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
            emit(&std::env::var("BHCP_EXPERIMENT_MODEL").unwrap());
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
        "empty-contamination" => {
            std::fs::create_dir("unexpected").unwrap();
            ExitCode::SUCCESS
        }
        "pin-mismatch" => {
            emit("different-model@9");
            ExitCode::SUCCESS
        }
        "overflow" => {
            println!("{}", "x".repeat(4_096));
            ExitCode::SUCCESS
        }
        "timeout" => {
            std::thread::sleep(std::time::Duration::from_millis(500));
            emit(&std::env::var("BHCP_EXPERIMENT_MODEL").unwrap());
            ExitCode::SUCCESS
        }
        #[cfg(unix)]
        "background" => {
            spawn_background_process_for_controller_test();
            emit(&std::env::var("BHCP_EXPERIMENT_MODEL").unwrap());
            ExitCode::SUCCESS
        }
        "judge-mutate" => {
            std::fs::write("subject/src/lib.rs", "// judge changed candidate\n").unwrap();
            ExitCode::SUCCESS
        }
        "judge-success" => ExitCode::SUCCESS,
        _ => ExitCode::from(64),
    }
}
