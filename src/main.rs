use clap::Parser;
use colored::Colorize;
use rayon::prelude::*;
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

mod cli;
mod diff;
mod error;
mod output;
mod parser;

use cli::args::{Args, OutputFormat};
use diff::engine::DiffEngine;
use diff::types::{ArrayCompareMode, DiffConfig};
use error::JsonDiffError;
use output::formatter::{format_as_json, format_as_summary, PrettyFormatter};
use parser::json::JsonParser;
use sonic_rs::Value;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{} {}", "error:".red().bold(), e);
            ExitCode::from(e.exit_code() as u8)
        }
    }
}

fn run() -> Result<(), JsonDiffError> {
    let args = Args::parse();

    // Determine color output
    let colors_enabled = !args.no_color && atty::is(atty::Stream::Stdout);

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
    };

    // Parse input files
    let (left, right) = load_inputs(&args)?;

    // Compute diff
    let engine = DiffEngine::new(config);
    let result = engine.diff(&left, &right);

    // Format output
    let mut stdout = io::stdout().lock();
    match args.format {
        OutputFormat::Pretty => {
            let formatter = PrettyFormatter::new(colors_enabled, args.compact);
            formatter.format(&result, &mut stdout)?;
        }
        OutputFormat::Json => {
            let json_output = format_as_json(&result);
            writeln!(stdout, "{}", json_output)?;
        }
        OutputFormat::Summary => {
            let summary = format_as_summary(&result.stats);
            writeln!(stdout, "{}", summary)?;
        }
    }

    Ok(())
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
        let paths: Vec<&Path> = vec![args.file1.as_path(), args.file2.as_path()];
        let results: Vec<Result<Value, JsonDiffError>> = paths
            .par_iter()
            .map(|p| JsonParser::parse_file(p))
            .collect();

        let left = results.into_iter().next().unwrap()?;
        let right_path = args.file2.as_path();
        let right = JsonParser::parse_file(right_path)?;

        Ok((left, right))
    }
}
