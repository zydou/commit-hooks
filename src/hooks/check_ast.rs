//! `check-ast` — verify that Python files parse.
//!
//! Upstream feeds each file to `ast.parse` and, on `SyntaxError`, prints the
//! interpreter name/version plus a formatted traceback. We use the `ruff`
//! parser instead, so the traceback body necessarily differs, but the contract
//! is identical: a file that does not parse is reported and fails the hook.

use clap::Parser;
use ruff_python_parser::parse_module;
use crate::util::line_col;

#[derive(Parser)]
#[command(about = "Checks that all your Python files parse.")]
struct Args {
    filenames: Vec<String>,
}

pub fn run(args: &[String]) -> i32 {
    let args = Args::parse_from(std::iter::once("check-ast".to_owned()).chain(args.iter().cloned()));

    let mut retval = 0;
    for filename in &args.filenames {
        let Ok(source) = std::fs::read_to_string(filename) else {
            // Unreadable or non-UTF-8: report it the same way a parse failure is.
            println!("{filename}: failed parsing with ruff (ruff_python_parser):");
            println!();
            println!("    could not read file as UTF-8");
            println!();
            retval = 1;
            continue;
        };
        if let Err(err) = parse_module(&source) {
            println!("{filename}: failed parsing with ruff (ruff_python_parser):");
            println!();
            let (line, col) = line_col(&source, err.location.start().to_usize());
            println!("    {line}:{col}: {}", err.error);
            println!();
            retval = 1;
        }
    }
    retval
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_ids(contents: &[u8]) -> i32 {
        let path = crate::util::write_temp_file(contents);
        let name = path.to_string_lossy().into_owned();
        run(&[name])
    }

    #[test]
    fn valid_python_passes() {
        assert_eq!(run_ids(b"import os\nx = 1\n"), 0);
    }

    #[test]
    fn syntax_error_fails() {
        assert_eq!(run_ids(b"def f(:\n"), 1);
    }
}
