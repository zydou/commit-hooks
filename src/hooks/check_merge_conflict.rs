//! `check-merge-conflict` — detect leftover merge-conflict markers.
//!
//! The hook only runs when git reports an in-progress merge/rebase (or when
//! `--assume-in-merge` is passed), then scans each file for conflict strings.

use clap::Parser;
use std::path::Path;

use crate::util::cmd_output;

/// Byte prefixes that indicate a conflict marker line.
const CONFLICT_PATTERNS: &[&[u8]] = &[
    b"<<<<<<< ",
    b"======= ",
    b"=======\r\n",
    b"=======\n",
    b">>>>>>> ",
];

#[derive(Parser)]
#[command(about = "Check files for merge conflict strings.")]
struct Args {
    /// Filenames to check.
    filenames: Vec<String>,

    /// Run the check even when git doesn't report an in-progress merge.
    #[arg(long = "assume-in-merge")]
    assume_in_merge: bool,
}

/// Whether git is currently in a merge or rebase.
fn is_in_merge() -> bool {
    let Ok(git_dir) = cmd_output("git", &["rev-parse", "--git-dir"], Some(0)) else {
        return false;
    };
    let git_dir = git_dir.trim_end();
    let base = Path::new(git_dir);
    let exists = |name: &str| base.join(name).exists();
    exists("MERGE_MSG")
        && (exists("MERGE_HEAD") || exists("rebase-apply") || exists("rebase-merge"))
}

pub fn run(args: &[String]) -> i32 {
    let args = Args::parse_from(
        std::iter::once("check-merge-conflict".to_owned()).chain(args.iter().cloned()),
    );

    if !is_in_merge() && !args.assume_in_merge {
        return 0;
    }

    let mut retcode = 0;
    for filename in &args.filenames {
        let Ok(bytes) = std::fs::read(filename) else {
            continue;
        };
        for (i, line) in split_lines(&bytes).iter().enumerate() {
            for pattern in CONFLICT_PATTERNS {
                if line.starts_with(pattern) {
                    let marker = String::from_utf8_lossy(pattern).trim_end().to_owned();
                    println!("{filename}:{}: Merge conflict string '{marker}' found", i + 1);
                    retcode = 1;
                }
            }
        }
    }
    retcode
}

/// Split on `\n`, keeping the `\n` on each line (binary `enumerate(f)`).
fn split_lines(data: &[u8]) -> Vec<&[u8]> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn run_with(contents: &[u8], extra: &[&str]) -> i32 {
        let path = crate::util::write_temp_file(contents);
        let mut argv = vec![path.to_string_lossy().into_owned()];
        argv.extend(extra.iter().map(|s| s.to_string()));
        run(&argv)
    }

    #[test]
    fn assume_in_merge_detects_conflict() {
        let content = b"line\n<<<<<<< HEAD\nours\n=======\ntheirs\n>>>>>>> other\n";
        assert_eq!(run_with(content, &["--assume-in-merge"]), 1);
    }

    #[test]
    fn clean_file_passes() {
        assert_eq!(run_with(b"just\ntext\n", &["--assume-in-merge"]), 0);
    }

    #[test]
    fn split_lines_keeps_newline() {
        let lines = split_lines(b"a\nb\nc");
        assert_eq!(lines, vec![&b"a\n"[..], &b"b\n"[..], &b"c"[..]]);
    }
}
