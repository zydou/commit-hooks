//! `check-docstring-first` — ensure a module docstring is the first statement.
//!
//! A "docstring" is any string token starting at column 0. The hook fails when
//! there is more than one such string, or when one appears after real code.

use clap::Parser;
use ruff_python_ast::token::TokenKind;
use ruff_python_parser::{lexer::lex, Mode};
use crate::util::line_col;

#[derive(Parser)]
#[command(about = "Checks that a module has a single docstring at the top of the file.")]
struct Args {
    filenames: Vec<String>,
}

/// Tokens that never count as "code" when looking for the first statement.
fn is_non_code(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Comment | TokenKind::Newline | TokenKind::NonLogicalNewline | TokenKind::EndOfFile
    )
}

/// Returns nonzero if the source has a docstring that is not at the very top.
fn check_docstring_first(source: &str, filename: &str) -> i32 {
    let mut found_docstring_line = None;
    let mut found_code_line = None;

    let mut lexer = lex(source, Mode::Module);
    loop {
        let kind = lexer.next_token();
        let range = lexer.current_range();
        if kind.is_eof() {
            break;
        }
        let (sline, scol) = line_col(source, range.start().to_usize());

        // A string at column 0 looks like a module docstring.
        if kind == TokenKind::String && scol == 0 {
            if let Some(first) = found_docstring_line {
                println!(
                    "{filename}:{sline}: Multiple module docstrings (first docstring on line {first})."
                );
                return 1;
            } else if let Some(code) = found_code_line {
                println!(
                    "{filename}:{sline}: Module docstring appears after code (code seen on line {code})."
                );
                return 1;
            } else {
                found_docstring_line = Some(sline);
            }
        } else if !is_non_code(kind) && found_code_line.is_none() {
            found_code_line = Some(sline);
        }
    }

    0
}

pub fn run(args: &[String]) -> i32 {
    let args = Args::parse_from(std::iter::once("check-docstring-first".to_owned()).chain(args.iter().cloned()));

    let mut retv = 0;
    for filename in &args.filenames {
        let Ok(source) = std::fs::read_to_string(filename) else {
            continue;
        };
        retv |= check_docstring_first(&source, filename);
    }
    retv
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn top_docstring_passes() {
        assert_eq!(check_docstring_first("\"\"\"doc\"\"\"\nx = 1\n", "f.py"), 0);
    }

    #[test]
    fn no_docstring_passes() {
        assert_eq!(check_docstring_first("x = 1\n", "f.py"), 0);
    }

    #[test]
    fn docstring_after_code_fails() {
        assert_eq!(check_docstring_first("x = 1\n\"\"\"doc\"\"\"\n", "f.py"), 1);
    }

    #[test]
    fn multiple_docstrings_fail() {
        assert_eq!(check_docstring_first("\"\"\"a\"\"\"\n\"\"\"b\"\"\"\n", "f.py"), 1);
    }

    #[test]
    fn comments_and_blank_lines_ignored() {
        let src = "# comment\n\n\"\"\"doc\"\"\"\nx = 1\n";
        assert_eq!(check_docstring_first(src, "f.py"), 0);
    }

    #[test]
    fn indented_string_is_not_a_docstring() {
        // A string inside an expression is not at column 0.
        let src = "x = (\n    \"not a docstring\"\n)\n";
        assert_eq!(check_docstring_first(src, "f.py"), 0);
    }
}
