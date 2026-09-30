use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

/// Evaluate, validate and export cuecue configurations.
#[derive(Parser)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Parse a file and report syntax errors.
    Parse {
        /// Print the syntax tree.
        #[arg(long)]
        tree: bool,
        file: PathBuf,
    },
}

fn main() -> ExitCode {
    match Cli::parse().command {
        None => ExitCode::SUCCESS,
        Some(Command::Parse { tree, file }) => parse(&file, tree),
    }
}

fn parse(file: &PathBuf, tree: bool) -> ExitCode {
    let src = match std::fs::read_to_string(file) {
        Ok(src) => src,
        Err(err) => {
            eprintln!("{}: {err}", file.display());
            return ExitCode::from(2);
        }
    };
    let parse = cuecue_syntax::parse(&src);
    if tree {
        print!("{:#?}", parse.syntax());
    }
    for error in &parse.errors {
        let (line, column) = line_column(&src, error.offset as usize);
        eprintln!("{}:{line}:{column}: {}", file.display(), error.message);
    }
    if parse.errors.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// 1-based line and column (in characters) of a byte offset.
fn line_column(src: &str, offset: usize) -> (usize, usize) {
    let before = &src[..offset];
    let line = before.matches('\n').count() + 1;
    let column = before.rsplit('\n').next().unwrap_or("").chars().count() + 1;
    (line, column)
}
