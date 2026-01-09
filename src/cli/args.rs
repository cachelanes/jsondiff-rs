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

#[derive(clap::ValueEnum, Clone, Debug, Default, PartialEq)]
pub enum OutputFormat {
    #[default]
    Pretty,
    Json,
    Summary,
}
