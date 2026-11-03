//! `double-quote-string-fixer` — replace `"` string quotes with `'`.
//!
//! Strings are rewritten in reverse so earlier byte offsets stay valid. The
//! meat of a string containing a quote character is left alone (it would need
//! escaping), as are triple-quoted strings and anything inside an f-string.

use clap::Parser;
use ruff_python_ast::token::TokenKind;
use ruff_python_parser::{lexer::lex, Mode};

#[derive(Parser)]
#[command(about = "Fixes strings that use double quotes to use single quotes.")]
struct Args {
    /// Filenames to fix.
    filenames: Vec<String>,
}

/// Rewrite a string token's quotes from `"` to `'` when it is safe to do so.
fn handle_match(token_text: &str) -> Option<String> {
    if token_text.contains("\"\"\"") || token_text.contains("'''") {
        return None;
    }
    // `^[a-zA-Z]*"` — an optional string prefix followed by a double quote.
    let prefix_len = token_text
        .find(|c: char| !c.is_ascii_alphabetic())
        .unwrap_or(token_text.len());
    if prefix_len < token_text.len() && token_text.as_bytes()[prefix_len] == b'"' {
        let meat = &token_text[prefix_len + 1..token_text.len() - 1];
        if meat.contains('"') || meat.contains('\'') {
            None
        } else {
            let prefix = token_text[..prefix_len].replace('"', "'");
            Some(format!("{prefix}'{meat}'"))
        }
    } else {
        None
    }
}

/// Fix every eligible double-quoted string; returns the new contents (equal to
/// the input when nothing changed).
fn fix_strings(contents: &str) -> String {
    // Collect the tokens up front so we can walk them in reverse.
    let mut tokens = Vec::new();
    let mut lexer = lex(contents, Mode::Module);
    loop {
        let kind = lexer.next_token();
        let range = lexer.current_range();
        if kind.is_eof() {
            break;
        }
        tokens.push((kind, range));
    }

    let mut split: Vec<u8> = contents.as_bytes().to_vec();
    let mut fstring_depth: i32 = 0;

    // Reverse order keeps every remaining token's byte offsets valid, even when
    // a replacement changes the length.
    for (kind, range) in tokens.iter().rev() {
        match kind {
            TokenKind::FStringStart => fstring_depth += 1,
            TokenKind::FStringEnd => fstring_depth -= 1,
            TokenKind::String if fstring_depth == 0 => {
                let start = range.start().to_usize();
                let end = range.end().to_usize();
                if let Some(new_text) = handle_match(&contents[start..end]) {
                    split.splice(start..end, new_text.into_bytes());
                }
            }
            _ => {}
        }
    }

    String::from_utf8(split).unwrap_or_else(|_| contents.to_owned())
}

pub fn run(args: &[String]) -> i32 {
    let args = Args::parse_from(std::iter::once("double-quote-string-fixer".to_owned()).chain(args.iter().cloned()));

    let mut retv = 0;
    for filename in &args.filenames {
        let Ok(contents) = std::fs::read_to_string(filename) else {
            continue;
        };
        let new_contents = fix_strings(&contents);
        if new_contents != contents {
            if std::fs::write(filename, &new_contents).is_ok() {
                println!("Fixing strings in {filename}");
            }
            retv = 1;
        }
    }
    retv
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixes_double_quotes() {
        assert_eq!(fix_strings("x = \"hello\"\n"), "x = 'hello'\n");
    }

    #[test]
    fn keeps_single_quotes() {
        assert_eq!(fix_strings("y = 'keep'\n"), "y = 'keep'\n");
    }

    #[test]
    fn keeps_string_with_quote_in_meat() {
        // A double-quoted string containing a single quote is left alone.
        assert_eq!(fix_strings("z = \"it's\"\n"), "z = \"it's\"\n");
    }

    #[test]
    fn keeps_escaped_quote() {
        // Escaped quote in the meat -> left as a double-quoted string.
        assert_eq!(fix_strings("a = \"say \\\"hi\\\"\"\n"), "a = \"say \\\"hi\\\"\"\n");
    }

    #[test]
    fn leaves_fstrings_alone() {
        // The f-string stays; the plain string after it is fixed.
        assert_eq!(fix_strings("f\"{x}\"\nb = \"plain\"\n"), "f\"{x}\"\nb = 'plain'\n");
    }

    #[test]
    fn keeps_inner_fstring_quotes() {
        assert_eq!(
            fix_strings("x = f\"a{'inner'}b\"\nc = \"d\"\n"),
            "x = f\"a{'inner'}b\"\nc = 'd'\n"
        );
    }

    #[test]
    fn no_trailing_newline() {
        assert_eq!(fix_strings("x = \"no newline\""), "x = 'no newline'");
    }

    #[test]
    fn docstring_is_fixed() {
        assert_eq!(fix_strings("\"docstring\"\nx=\"y\"\n"), "'docstring'\nx='y'\n");
    }
}
