#![allow(
    dead_code,
    reason = "path formatting utilities reserved for future use"
)]

use crate::diff::JsonPath;

/// Format a JSON path for display
pub fn format_path(path: &JsonPath) -> String {
    path.to_string()
}
