use std::io;
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

    #[error("invalid --set-key argument \"{arg}\": {reason}")]
    InvalidSetKey { arg: String, reason: String },

    #[error("Output error: {0}")]
    OutputError(#[from] io::Error),
}

impl JsonDiffError {
    pub const fn exit_code(&self) -> u8 {
        match self {
            Self::FileNotFound(_) => 2,
            Self::InvalidJson { .. } => 3,
            Self::FileRead { .. } => 4,
            Self::SetKeyMissing { .. }
            | Self::SetKeyDuplicate { .. }
            | Self::InvalidSetKey { .. } => 5,
            Self::StdinConflict | Self::OutputError(_) => 1,
        }
    }
}
