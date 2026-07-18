use std::fs;
use std::path::{Path, PathBuf};

use crate::diagnostic::{Diagnostic, Result};
use crate::hash::HashAlgorithm;

pub const FILE_NAME: &str = "bhcp-project.toml";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub struct ProjectManifest {
    pub identity_algorithm: HashAlgorithm,
}

impl ProjectManifest {
    pub fn parse(source: &str, source_name: &str) -> Result<Self> {
        let mut algorithm = None;
        for (index, raw_line) in source.lines().enumerate() {
            let line = raw_line.split('#').next().unwrap().trim();
            if line.is_empty() {
                continue;
            }
            let Some((key, raw_value)) = line.split_once('=') else {
                return Err(Diagnostic::new(
                    "BHCP6002",
                    "manifest entry must use key = \"value\"",
                    source_name,
                    index + 1,
                    1,
                ));
            };
            if key.trim() != "identity_algorithm" {
                return Err(Diagnostic::new(
                    "BHCP6002",
                    format!("unknown project-manifest key {:?}", key.trim()),
                    source_name,
                    index + 1,
                    1,
                ));
            }
            if algorithm.is_some() {
                return Err(Diagnostic::new(
                    "BHCP6002",
                    "duplicate identity_algorithm",
                    source_name,
                    index + 1,
                    1,
                ));
            }
            let value = raw_value.trim();
            if value.len() < 2 || !value.starts_with('"') || !value.ends_with('"') {
                return Err(Diagnostic::new(
                    "BHCP6002",
                    "identity_algorithm must be a quoted algorithm ID",
                    source_name,
                    index + 1,
                    1,
                ));
            }
            algorithm = Some(HashAlgorithm::from_id(&value[1..value.len() - 1]).map_err(
                |error| Diagnostic::new(error.code, error.message, source_name, index + 1, 1),
            )?);
        }
        Ok(Self {
            identity_algorithm: algorithm.unwrap_or_default(),
        })
    }

    pub fn discover(source_path: &Path) -> Result<Self> {
        let mut directory = source_path.parent();
        while let Some(candidate_directory) = directory {
            let candidate = candidate_directory.join(FILE_NAME);
            if candidate.is_file() {
                let source = fs::read_to_string(&candidate).map_err(|error| {
                    Diagnostic::new(
                        "BHCP6002",
                        error.to_string(),
                        candidate.display().to_string(),
                        1,
                        1,
                    )
                })?;
                return Self::parse(&source, &candidate.display().to_string());
            }
            directory = candidate_directory.parent();
        }
        Ok(Self::default())
    }
}

pub fn manifest_path(root: &Path) -> PathBuf {
    root.join(FILE_NAME)
}
