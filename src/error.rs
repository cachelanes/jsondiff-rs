use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum JsonDiffError {
    #[error("Failed to read file '{path}': {message}")]
    FileRead { path: PathBuf, message: String },

    #[error("Invalid JSON in '{path}': {message}")]
    InvalidJson { path: PathBuf, message: String },

    #[error("File not found: '{0}'")]
    FileNotFound(PathBuf),

    #[error("Cannot read from stdin for both files")]
    StdinConflict,

    #[error("object at {path} missing required key field \"{field}\"\nhint: use --set-key-allow-missing to fall back to set comparison")]
    SetKeyMissing { path: String, field: String },

    #[error("duplicate key [{key}] found at {path1} and {path2}\nhint: use --set-key-allow-duplicates to use first occurrence")]
    SetKeyDuplicate {
        key: String,
        path1: String,
        path2: String,
    },

    #[error("Output error: {0}")]
    OutputError(#[from] std::io::Error),
}

impl JsonDiffError {
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::FileNotFound(_) => 2,
            Self::InvalidJson { .. } => 3,
            Self::FileRead { .. } => 4,
            Self::SetKeyMissing { .. } | Self::SetKeyDuplicate { .. } => 5,
            _ => 1,
        }
    }
}
