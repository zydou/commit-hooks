//! `sort-simple-yaml` — sort a "simple" YAML file alphabetically, keeping
//! blocks of comments and definitions together.
//!
//! Only the top layer is sorted; the file is assumed to look like a header
//! comment block followed by `key: value` blocks separated by blank lines.

use clap::Parser;

#[derive(Parser)]
#[command(about = "Sort simple YAML files which consist only of top-level keys.")]
struct Args {
    /// Filenames to fix.
    filenames: Vec<String>,
}

const QUOTES: &[char] = &['\'', '"'];

/// Sort the (newline-free) lines, keeping blocks together.
fn sort(lines: &[String]) -> Vec<String> {
    let mut lines = lines.to_vec();
    let mut new_lines = parse_block(&mut lines, true);

    let mut blocks = parse_blocks(&mut lines);
    blocks.sort_by_key(|b| first_key(b));
    for block in blocks {
        if !new_lines.is_empty() {
            new_lines.push(String::new());
        }
        new_lines.extend(block);
    }
    new_lines
}

/// Pop a single block off the front of `lines`.
///
/// For a header block we stop at the first non-comment (or blank) line;
/// otherwise we stop at the first blank line.
fn parse_block(lines: &mut Vec<String>, header: bool) -> Vec<String> {
    let mut block = Vec::new();
    while let Some(first) = lines.first() {
        if first.is_empty() {
            break;
        }
        if header && !first.starts_with('#') {
            break;
        }
        block.push(lines.remove(0));
    }
    block
}

/// Pop every block off `lines`, skipping blank separators.
fn parse_blocks(lines: &mut Vec<String>) -> Vec<Vec<String>> {
    let mut blocks = Vec::new();
    while !lines.is_empty() {
        if lines[0].is_empty() {
            lines.remove(0);
        } else {
            blocks.push(parse_block(lines, false));
        }
    }
    blocks
}

/// The sort key of a block: its first non-comment key, with any leading quote
/// stripped.
fn first_key(lines: &[String]) -> String {
    for line in lines {
        if line.starts_with('#') {
            continue;
        }
        if QUOTES.iter().any(|q| line.starts_with(*q)) {
            return line[1..].to_owned();
        }
        return line.clone();
    }
    String::new()
}

pub fn run(args: &[String]) -> i32 {
    let args = Args::parse_from(std::iter::once("sort-simple-yaml".to_owned()).chain(args.iter().cloned()));

    let mut retval = 0;
    for filename in &args.filenames {
        let Ok(bytes) = std::fs::read(filename) else {
            continue;
        };
        let Ok(text) = std::str::from_utf8(&bytes) else {
            continue;
        };
        // `line.rstrip()` on each line, dropping the trailing newline.
        let lines: Vec<String> = text
            .split('\n')
            .map(|l| l.trim_end_matches(['\r', '\n', ' ', '\t']).to_owned())
            .collect();
        // A trailing newline produces a final empty element; upstream's
        // `readlines()` yields the same and its rstrip keeps it as ''.
        let new_lines = sort(&lines);
        if lines != new_lines {
            println!("Fixing file `{filename}`");
            let out = new_lines.join("\n") + "\n";
            let _ = std::fs::write(filename, out);
            retval = 1;
        }
    }
    retval
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorts_blocks_keeping_header() {
        let input = vec![
            "# header".to_owned(),
            String::new(),
            "zebra: 1".to_owned(),
            String::new(),
            "apple: 2".to_owned(),
        ];
        let out = sort(&input);
        assert_eq!(
            out,
            vec![
                "# header".to_owned(),
                String::new(),
                "apple: 2".to_owned(),
                String::new(),
                "zebra: 1".to_owned(),
            ]
        );
    }

    #[test]
    fn first_key_strips_quote() {
        // Upstream returns everything after the leading quote character.
        assert_eq!(first_key(&["'foo': true".to_owned()]), "foo': true");
        assert_eq!(first_key(&["foo: true".to_owned()]), "foo: true");
    }
}
