use super::color::ColorScheme;
use crate::diff::{DiffOp, DiffResult, DiffStats};
use colored::Colorize as _;
use sonic_rs::{JsonContainerTrait as _, JsonValueTrait as _, Value};
use std::io::{self, Write};

pub struct PrettyFormatter {
    color_scheme: ColorScheme,
    colors_enabled: bool,
    compact: bool,
}

impl PrettyFormatter {
    pub fn new(colors_enabled: bool, compact: bool) -> Self {
        Self {
            color_scheme: ColorScheme::default(),
            colors_enabled,
            compact,
        }
    }

    pub fn format<W: Write>(&self, result: &DiffResult, out: &mut W) -> io::Result<()> {
        if result.operations.is_empty() {
            if !self.compact {
                writeln!(out, "No differences found.")?;
            }
            return Ok(());
        }

        for op in &result.operations {
            self.format_operation(op, out)?;
        }

        if !self.compact {
            self.write_summary(&result.stats, out)?;
        }

        Ok(())
    }

    fn format_operation<W: Write>(&self, op: &DiffOp, out: &mut W) -> io::Result<()> {
        match op {
            DiffOp::Added { path, value } => {
                let path_str = path.to_string();
                writeln!(out, "{}", self.colorize(&path_str, self.color_scheme.path))?;
                let formatted = self.format_value(value, 2);
                for line in formatted.lines() {
                    let prefix = self.colorize("+ ", self.color_scheme.added);
                    let content = self.colorize(line, self.color_scheme.added);
                    writeln!(out, "{prefix}{content}")?;
                }
                writeln!(out)?;
            }
            DiffOp::Removed { path, value } => {
                let path_str = path.to_string();
                writeln!(out, "{}", self.colorize(&path_str, self.color_scheme.path))?;
                let formatted = self.format_value(value, 2);
                for line in formatted.lines() {
                    let prefix = self.colorize("- ", self.color_scheme.removed);
                    let content = self.colorize(line, self.color_scheme.removed);
                    writeln!(out, "{prefix}{content}")?;
                }
                writeln!(out)?;
            }
            DiffOp::Modified {
                path,
                old_value,
                new_value,
            } => {
                let path_str = path.to_string();
                writeln!(out, "{}", self.colorize(&path_str, self.color_scheme.path))?;

                let old_formatted = self.format_value(old_value, 2);
                for line in old_formatted.lines() {
                    let prefix = self.colorize("- ", self.color_scheme.modified_old);
                    let content = self.colorize(line, self.color_scheme.modified_old);
                    writeln!(out, "{prefix}{content}")?;
                }

                let new_formatted = self.format_value(new_value, 2);
                for line in new_formatted.lines() {
                    let prefix = self.colorize("+ ", self.color_scheme.modified_new);
                    let content = self.colorize(line, self.color_scheme.modified_new);
                    writeln!(out, "{prefix}{content}")?;
                }
                writeln!(out)?;
            }
        }
        Ok(())
    }

    fn format_value(&self, value: &Value, indent: usize) -> String {
        self.format_value_inner(value, indent, 0)
    }

    fn format_value_inner(
        &self,
        value: &Value,
        base_indent: usize,
        current_depth: usize,
    ) -> String {
        let indent_str = " ".repeat(base_indent + current_depth * 2);

        if value.is_null() {
            return "null".to_string();
        }
        if let Some(b) = value.as_bool() {
            return b.to_string();
        }
        if value.is_number() {
            return format!("{value}");
        }
        if let Some(s) = value.as_str() {
            return format!("\"{s}\"");
        }
        if let Some(arr) = value.as_array() {
            if arr.is_empty() {
                return "[]".to_string();
            }
            if arr.len() <= 3 && is_simple_array(arr) {
                // Inline short simple arrays
                let items: Vec<String> = arr
                    .iter()
                    .map(|v| self.format_value_inner(v, 0, 0))
                    .collect();
                return format!("[{}]", items.join(", "));
            }
            let child_indent = " ".repeat(base_indent + (current_depth + 1) * 2);
            let items: Vec<String> = arr
                .iter()
                .map(|v| {
                    format!(
                        "{child_indent}{}",
                        self.format_value_inner(v, base_indent, current_depth + 1)
                    )
                })
                .collect();
            return format!("[\n{}\n{indent_str}]", items.join(",\n"));
        }
        if let Some(obj) = value.as_object() {
            if obj.is_empty() {
                return "{}".to_string();
            }
            let child_indent = " ".repeat(base_indent + (current_depth + 1) * 2);
            let items: Vec<String> = obj
                .iter()
                .map(|(k, v)| {
                    format!(
                        "{child_indent}\"{k}\": {}",
                        self.format_value_inner(v, base_indent, current_depth + 1)
                    )
                })
                .collect();
            return format!("{{\n{}\n{indent_str}}}", items.join(",\n"));
        }

        // Fallback
        format!("{value}")
    }

    fn write_summary<W: Write>(&self, stats: &DiffStats, out: &mut W) -> io::Result<()> {
        let separator = self.colorize(
            "----------------------------------------",
            self.color_scheme.separator,
        );
        writeln!(out, "{separator}")?;

        let added = self.colorize(&stats.added.to_string(), self.color_scheme.added);
        let removed = self.colorize(&stats.removed.to_string(), self.color_scheme.removed);
        let modified = self.colorize(&stats.modified.to_string(), colored::Color::Yellow);

        writeln!(out, "{added} added, {removed} removed, {modified} modified")?;
        Ok(())
    }

    fn colorize(&self, text: &str, color: colored::Color) -> String {
        if self.colors_enabled {
            text.color(color).to_string()
        } else {
            text.to_string()
        }
    }
}

fn is_simple_array(arr: &sonic_rs::Array) -> bool {
    arr.iter()
        .all(|v| v.is_null() || v.is_boolean() || v.is_number() || v.is_str())
}

/// Format diff result as JSON
pub fn format_as_json(result: &DiffResult) -> String {
    use sonic_rs::json;

    let ops: Vec<sonic_rs::Value> = result
        .operations
        .iter()
        .map(|op| match op {
            DiffOp::Added { path, value } => {
                json!({
                    "op": "add",
                    "path": path.to_string(),
                    "value": value.clone()
                })
            }
            DiffOp::Removed { path, value } => {
                json!({
                    "op": "remove",
                    "path": path.to_string(),
                    "value": value.clone()
                })
            }
            DiffOp::Modified {
                path,
                old_value,
                new_value,
            } => {
                json!({
                    "op": "replace",
                    "path": path.to_string(),
                    "oldValue": old_value.clone(),
                    "newValue": new_value.clone()
                })
            }
        })
        .collect();

    let output = json!({
        "operations": ops,
        "stats": {
            "added": result.stats.added,
            "removed": result.stats.removed,
            "modified": result.stats.modified
        }
    });

    sonic_rs::to_string_pretty(&output).unwrap_or_else(|_| "{}".to_string())
}

/// Format diff result as a summary
pub fn format_as_summary(stats: &DiffStats) -> String {
    format!(
        "{} added, {} removed, {} modified",
        stats.added, stats.removed, stats.modified
    )
}
