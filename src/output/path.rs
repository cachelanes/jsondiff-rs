#![allow(dead_code)]

use crate::diff::JsonPath;

/// Format a JSON path for display
pub fn format_path(path: &JsonPath) -> String {
    path.to_string()
}
