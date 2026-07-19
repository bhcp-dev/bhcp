use std::fs;
use std::io::Read;
use std::path::PathBuf;

fn main() {
    let target = PathBuf::from(std::env::var_os("CARGO_TARGET_DIR").expect("missing target"));
    assert!(target.is_dir());
    let arguments: Vec<_> = std::env::args_os().collect();
    let output_index = arguments.iter().position(|argument| argument == "-o").unwrap();
    fs::write(&arguments[output_index + 1], b"{\"claimed_success\":false}\n").unwrap();
    let mut prompt = String::new();
    std::io::stdin().read_to_string(&mut prompt).unwrap();
    assert_eq!(prompt, "frozen prompt\n");
    println!(
        "{{\"type\":\"item.completed\",\"item\":{{\"type\":\"command_execution\",\"status\":\"completed\"}}}}"
    );
    println!(
        "{{\"type\":\"turn.completed\",\"usage\":{{\"input_tokens\":10,\"cached_input_tokens\":4,\"output_tokens\":3,\"reasoning_output_tokens\":2}}}}"
    );
}
