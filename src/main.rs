use clap::Parser as _;
use colored::Colorize as _;
use rayon::prelude::*;
use std::collections::HashMap;
use std::io::{self, IsTerminal as _, Write as _};
use std::process::ExitCode;

mod cli;
mod diff;
mod error;
mod output;
mod parser;

use cli::args::{Args, OutputFormat};
use diff::engine::DiffEngine;
use diff::types::{ArrayCompareMode, DiffConfig, SetKeyConfig};
use error::JsonDiffError;
use output::formatter::{format_as_json, format_as_summary, PrettyFormatter};
use parser::json::JsonParser;
use sonic_rs::Value;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{} {e}", "error:".red().bold());
            ExitCode::from(e.exit_code())
        }
    }
}

fn run() -> Result<(), JsonDiffError> {
    let args = Args::parse();

    // Determine color output
    let colors_enabled = !args.no_color && io::stdout().is_terminal();

    // Parse set-key arguments into config
    let set_keys = if args.set_keys.is_empty() {
        None
    } else {
        Some(parse_set_key_args(
            &args.set_keys,
            args.set_key_allow_missing,
            args.set_key_allow_duplicates,
        )?)
    };

    // Build diff configuration
    let config = DiffConfig {
        array_mode: if args.array_as_set {
            ArrayCompareMode::Set
        } else if args.array_as_multiset {
            ArrayCompareMode::MultiSet
        } else {
            ArrayCompareMode::Ordered
        },
        ordered_objects: args.ordered_objects,
        set_keys,
    };

    // Parse input files
    let (left, right) = load_inputs(&args)?;

    // Compute diff
    let engine = DiffEngine::new(config);
    let result = engine.diff(&left, &right)?;

    // Format output
    let mut stdout = io::stdout().lock();
    match args.format {
        OutputFormat::Pretty => {
            let formatter = PrettyFormatter::new(colors_enabled, args.compact);
            formatter.format(&result, &mut stdout)?;
        }
        OutputFormat::Json => {
            let json_output = format_as_json(&result);
            writeln!(stdout, "{json_output}")?;
        }
        OutputFormat::Summary => {
            let summary = format_as_summary(&result.stats);
            writeln!(stdout, "{summary}")?;
        }
    }

    Ok(())
}

/// Parse --set-key arguments into a `SetKeyConfig`.
///
/// Each argument is "path.key" where the last dot-separated segment is the key field
/// and everything before it is the array path.
/// No dot means root array: `"id"` -> `path="" key="id"`.
/// Same path prefixes are grouped: `"users.id" + "users.type"` -> `{"users" => ["id", "type"]}`.
fn parse_set_key_args(
    set_keys: &[String],
    allow_missing: bool,
    allow_duplicates: bool,
) -> Result<SetKeyConfig, JsonDiffError> {
    let mut paths: HashMap<String, Vec<String>> = HashMap::new();

    for arg in set_keys {
        // TODO: if we accumulate more of these expects, disable option_if_let_else in Cargo.toml
        #[expect(
            clippy::option_if_let_else,
            reason = "match reads better than map_or_else here"
        )]
        let (array_path, key_field) = match arg.rfind('.') {
            Some(pos) => (arg[..pos].to_string(), arg[pos + 1..].to_string()),
            None => (String::new(), arg.clone()),
        };
        if key_field.is_empty() {
            return Err(JsonDiffError::InvalidSetKey {
                arg: arg.clone(),
                reason: "empty key field name".to_string(),
            });
        }
        let entry = paths.entry(array_path).or_default();
        if !entry.contains(&key_field) {
            entry.push(key_field);
        }
    }

    Ok(SetKeyConfig {
        paths,
        allow_missing,
        allow_duplicates,
    })
}

fn load_inputs(args: &Args) -> Result<(Value, Value), JsonDiffError> {
    let file1_is_stdin = args.file1.to_string_lossy() == "-";
    let file2_is_stdin = args.file2.to_string_lossy() == "-";

    if file1_is_stdin && file2_is_stdin {
        return Err(JsonDiffError::StdinConflict);
    }

    if file1_is_stdin || file2_is_stdin {
        // Sequential loading when stdin is involved
        let left = if file1_is_stdin {
            JsonParser::parse_stdin()?
        } else {
            JsonParser::parse_file(&args.file1)?
        };

        let right = if file2_is_stdin {
            JsonParser::parse_stdin()?
        } else {
            JsonParser::parse_file(&args.file2)?
        };

        Ok((left, right))
    } else {
        // Parallel loading for two files
        let paths = [args.file1.as_path(), args.file2.as_path()];
        let results: Vec<Result<Value, JsonDiffError>> = paths
            .par_iter()
            .map(|p| JsonParser::parse_file(p))
            .collect();

        let mut iter = results.into_iter();
        let left = iter.next().expect("results should have a first element")?;
        let right = iter.next().expect("results should have a second element")?;

        Ok((left, right))
    }
}
