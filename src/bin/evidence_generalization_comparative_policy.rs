use std::fs;
use std::process::ExitCode;

const MAX_SOURCE_BYTES: u64 = 64 * 1024;

fn main() -> ExitCode {
    match validate() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("comparative change-policy judge rejected candidate: {message}");
            ExitCode::from(90)
        }
    }
}

fn validate() -> Result<(), String> {
    let path = std::path::Path::new("subject/src/lib.rs");
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("cannot inspect subject/src/lib.rs: {error}"))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err("subject/src/lib.rs must remain a regular file".to_owned());
    }
    if metadata.len() > MAX_SOURCE_BYTES {
        return Err(format!(
            "subject/src/lib.rs exceeds the {MAX_SOURCE_BYTES}-byte bound"
        ));
    }
    let source =
        fs::read_to_string(path).map_err(|_| "subject/src/lib.rs must remain UTF-8".to_owned())?;
    if source.split_whitespace().any(|token| token == "unsafe") {
        return Err("unsafe Rust is outside the registered study boundary".to_owned());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::MAX_SOURCE_BYTES;

    #[test]
    fn source_bound_is_frozen() {
        assert_eq!(MAX_SOURCE_BYTES, 65_536);
    }
}
