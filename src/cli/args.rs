use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "jsondiff",
    version,
    about = "Lightning-fast JSON diff tool with beautiful output",
    long_about = "A blazingly fast JSON comparison tool that produces \
                  human-readable, colorful diffs. Perfect for debugging \
                  API responses, configuration changes, and data pipelines."
)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "CLI args naturally have many boolean flags"
)]
pub struct Args {
    /// First JSON file to compare (use "-" for stdin)
    #[arg(value_name = "FILE1")]
    pub file1: PathBuf,

    /// Second JSON file to compare
    #[arg(value_name = "FILE2")]
    pub file2: PathBuf,

    /// Treat arrays as unordered sets (ignore order, no duplicates)
    #[arg(short = 's', long = "array-set")]
    pub array_as_set: bool,

    /// Treat arrays as unordered multisets (ignore order, count duplicates)
    #[arg(short = 'm', long = "array-multiset")]
    pub array_as_multiset: bool,

    /// Compare objects as ordered (key order matters)
    #[arg(short = 'o', long = "ordered-objects")]
    pub ordered_objects: bool,

    /// Match array objects by a key field instead of position.
    /// Format: "path.key" (e.g. "users.id", "data.items.sku").
    /// No dot means root array (e.g. "id").
    /// Multiple --set-key with same path prefix creates composite keys.
    #[arg(long = "set-key", value_name = "PATH.KEY")]
    pub set_keys: Vec<String>,

    /// Allow elements missing the set-key field (falls back to set comparison)
    #[arg(long = "set-key-allow-missing", default_value = "true", action = clap::ArgAction::Set, value_parser = clap::value_parser!(bool))]
    pub set_key_allow_missing: bool,

    /// Allow duplicate set-key values (uses first occurrence)
    #[arg(long = "set-key-allow-duplicates", default_value = "true", action = clap::ArgAction::Set, value_parser = clap::value_parser!(bool))]
    pub set_key_allow_duplicates: bool,

    /// Output format
    #[arg(short = 'f', long = "format", value_enum, default_value = "pretty")]
    pub format: OutputFormat,

    /// Disable colored output
    #[arg(long = "no-color")]
    pub no_color: bool,

    /// Show only paths that differ (compact mode)
    #[arg(short = 'c', long = "compact")]
    pub compact: bool,
}

#[derive(clap::ValueEnum, Clone, Debug, Default, PartialEq, Eq)]
pub enum OutputFormat {
    #[default]
    Pretty,
    Json,
    Summary,
}
