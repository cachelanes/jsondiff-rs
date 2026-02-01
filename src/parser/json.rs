use crate::error::JsonDiffError;
use memmap2::Mmap;
use sonic_rs::Value;
use std::io::Read as _;
use std::path::Path;
use std::{fs, io};

const MMAP_THRESHOLD: u64 = 10 * 1024 * 1024; // 10MB

pub struct JsonParser;

impl JsonParser {
    /// Parse JSON from a file path, using memory mapping for large files
    pub fn parse_file(path: &Path) -> Result<Value, JsonDiffError> {
        if !path.exists() {
            return Err(JsonDiffError::FileNotFound(path.to_path_buf()));
        }

        let metadata = fs::metadata(path).map_err(|e| JsonDiffError::FileRead {
            path: path.to_path_buf(),
            message: e.to_string(),
        })?;

        if metadata.len() > MMAP_THRESHOLD {
            Self::parse_file_mmap(path)
        } else {
            Self::parse_file_direct(path)
        }
    }

    /// Parse JSON from stdin
    pub fn parse_stdin() -> Result<Value, JsonDiffError> {
        let mut buffer = Vec::new();
        io::stdin()
            .read_to_end(&mut buffer)
            .map_err(|e| JsonDiffError::FileRead {
                path: "<stdin>".into(),
                message: e.to_string(),
            })?;

        sonic_rs::from_slice(&buffer).map_err(|e| JsonDiffError::InvalidJson {
            path: "<stdin>".into(),
            message: e.to_string(),
        })
    }

    fn parse_file_direct(path: &Path) -> Result<Value, JsonDiffError> {
        let content = fs::read(path).map_err(|e| JsonDiffError::FileRead {
            path: path.to_path_buf(),
            message: e.to_string(),
        })?;

        sonic_rs::from_slice(&content).map_err(|e| JsonDiffError::InvalidJson {
            path: path.to_path_buf(),
            message: e.to_string(),
        })
    }

    #[expect(
        unsafe_code,
        reason = "mmap requires unsafe; file handle is kept alive"
    )]
    fn parse_file_mmap(path: &Path) -> Result<Value, JsonDiffError> {
        let file = fs::File::open(path).map_err(|e| JsonDiffError::FileRead {
            path: path.to_path_buf(),
            message: e.to_string(),
        })?;

        // SAFETY: The file handle is kept alive for the duration of the mmap usage.
        // The mmap is read-only and consumed immediately by the JSON parser.
        let mmap = unsafe { Mmap::map(&file) }.map_err(|e| JsonDiffError::FileRead {
            path: path.to_path_buf(),
            message: format!("Memory mapping failed: {e}"),
        })?;

        sonic_rs::from_slice(&mmap).map_err(|e| JsonDiffError::InvalidJson {
            path: path.to_path_buf(),
            message: e.to_string(),
        })
    }
}
