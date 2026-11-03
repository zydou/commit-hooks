//! `file-contents-sorter` — sort the lines of the given files.
//!
//! Blank lines are removed and (optionally) duplicate lines are collapsed.
//! All line endings are normalised to `\n`. `--ignore-case` folds case for the
//! sort key; `--unique` deduplicates. `--ignore-case` and `--unique` are
//! mutually exclusive.

use clap::Parser;

#[derive(Parser)]
#[command(about = "Sort the lines in the specified files.")]
struct Args {
    /// Files to sort.
    #[arg(required = true)]
    filenames: Vec<String>,

    /// Fold lower case to upper case characters for the sort key.
    #[arg(long = "ignore-case", conflicts_with = "unique")]
    ignore_case: bool,

    /// Ensure each line is unique.
    #[arg(long, short)]
    unique: bool,
}

/// Split on `\n`, keeping the `\n` on each line.
fn split_keepends(data: &[u8]) -> Vec<&[u8]> {
    let mut lines = Vec::new();
    let mut start = 0;
    for i in 0..data.len() {
        if data[i] == b'\n' {
            lines.push(&data[start..=i]);
            start = i + 1;
        }
    }
    if start < data.len() {
        lines.push(&data[start..]);
    }
    lines
}

fn sort_file_contents(before: &[u8], ignore_case: bool, unique: bool) -> (Vec<u8>, bool) {
    // Drop blank lines and strip the line terminator, like the upstream
    // `line.rstrip(b'\n\r') for line in before if line.strip()`.
    let mut lines: Vec<&[u8]> = split_keepends(before)
        .into_iter()
        .filter(|l| !l.iter().all(|b| b.is_ascii_whitespace()))
        .map(|l| {
            let mut end = l.len();
            while end > 0 && (l[end - 1] == b'\n' || l[end - 1] == b'\r') {
                end -= 1;
            }
            &l[..end]
        })
        .collect();

    let key = |l: &[u8]| -> Vec<u8> {
        if ignore_case {
            l.to_ascii_lowercase()
        } else {
            l.to_vec()
        }
    };
    lines.sort_by(|a, b| key(a).cmp(&key(b)));

    if unique {
        lines.dedup();
    }

    let mut after = lines.join(&b'\n');
    if !after.is_empty() {
        after.push(b'\n');
    }

    let changed = after != before;
    (after, changed)
}

pub fn run(args: &[String]) -> i32 {
    let args = Args::parse_from(std::iter::once("file-contents-sorter".to_owned()).chain(args.iter().cloned()));

    let mut retv = 0;
    for filename in &args.filenames {
        let Ok(before) = std::fs::read(filename) else {
            continue;
        };
        let (after, changed) = sort_file_contents(&before, args.ignore_case, args.unique);
        if changed {
            if std::fs::write(filename, &after).is_ok() {
                println!("Sorting {filename}");
            }
            retv |= 1;
        }
    }
    retv
}

#[cfg(test)]
mod tests {
    use super::sort_file_contents;

    #[test]
    fn sorts_and_normalises_endings() {
        let (after, changed) = sort_file_contents(b"c\na\nb\r\n", false, false);
        assert_eq!(after, b"a\nb\nc\n");
        assert!(changed);
    }

    #[test]
    fn removes_blank_lines() {
        let (after, _) = sort_file_contents(b"a\n\nb\n", false, false);
        assert_eq!(after, b"a\nb\n");
    }

    #[test]
    fn unique_dedupes() {
        let (after, _) = sort_file_contents(b"a\nb\na\n", false, true);
        assert_eq!(after, b"a\nb\n");
    }

    #[test]
    fn ignore_case_folds_sort_key() {
        let (after, _) = sort_file_contents(b"B\na\n", true, false);
        assert_eq!(after, b"a\nB\n");
    }

    #[test]
    fn already_sorted_is_noop() {
        let (after, changed) = sort_file_contents(b"a\nb\n", false, false);
        assert_eq!(after, b"a\nb\n");
        assert!(!changed);
    }
}
