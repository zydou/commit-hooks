//! `mixed-line-ending` — detect and optionally fix mixed line endings.
//!
//! Counts CRLF/LF/CR line terminators per file. With `--fix=auto` (the
//! default) the most common ending is applied to every line; `--fix=lf`,
//! `--fix=crlf`, `--fix=cr` force a specific ending; `--fix=no` only reports.

use clap::Parser;
use std::collections::HashMap;

#[derive(Parser)]
#[command(about = "Replace or check mixed line endings.")]
struct Args {
    /// Line ending to apply: auto, no, crlf, lf, or cr.
    #[arg(short = 'f', long = "fix", default_value = "auto")]
    fix: String,

    /// Filenames to fix.
    filenames: Vec<String>,
}

const CR: &[u8] = b"\r";
const CRLF: &[u8] = b"\r\n";
const LF: &[u8] = b"\n";
/// Prefer LF to CRLF to CR, but detect CRLF before LF.
const ALL_ENDINGS: [&[u8]; 3] = [CR, CRLF, LF];

fn fix_to_ending(fix: &str) -> Option<&'static [u8]> {
    match fix {
        "cr" => Some(CR),
        "crlf" => Some(CRLF),
        "lf" => Some(LF),
        _ => None,
    }
}

/// Split keeping the line terminator (binary `splitlines(keepends=True)`).
fn splitlines_keepends(data: &[u8]) -> Vec<&[u8]> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i < data.len() {
        if data[i] == b'\n' {
            lines.push(&data[start..=i]);
            start = i + 1;
        } else if data[i] == b'\r' {
            // A lone CR (not followed by LF) is its own terminator.
            if i + 1 >= data.len() || data[i + 1] != b'\n' {
                lines.push(&data[start..=i]);
                start = i + 1;
            }
        }
        i += 1;
    }
    if start < data.len() {
        lines.push(&data[start..]);
    }
    lines
}

fn apply_ending(filename: &str, contents: &[u8], ending: &[u8]) -> std::io::Result<()> {
    let mut new = Vec::with_capacity(contents.len());
    for line in splitlines_keepends(contents) {
        let stripped = match line {
            _ if line.ends_with(b"\r\n") => &line[..line.len() - 2],
            _ if line.ends_with(b"\n") || line.ends_with(b"\r") => &line[..line.len() - 1],
            _ => line,
        };
        new.extend_from_slice(stripped);
        new.extend_from_slice(ending);
    }
    std::fs::write(filename, &new)
}

fn fix_filename(filename: &str, fix: &str) -> i32 {
    let Ok(contents) = std::fs::read(filename) else {
        return 0;
    };

    let mut counts: HashMap<&[u8], usize> = HashMap::new();
    for line in splitlines_keepends(&contents) {
        for ending in ALL_ENDINGS {
            if line.ends_with(ending) {
                *counts.entry(ending).or_insert(0) += 1;
                break;
            }
        }
    }

    let mixed = counts.values().filter(|c| **c > 0).count() > 1;

    if fix == "no" || (fix == "auto" && !mixed) {
        return mixed as i32;
    }

    if fix == "auto" {
        // Ordering is important (lf > crlf > cr); `>=` prefers the later one.
        let mut max_ending = LF;
        let mut max_lines = 0usize;
        for ending_type in ALL_ENDINGS {
            let c = counts.get(ending_type).copied().unwrap_or(0);
            if c >= max_lines {
                max_ending = ending_type;
                max_lines = c;
            }
        }
        let _ = apply_ending(filename, &contents, max_ending);
        1
    } else {
        let target = fix_to_ending(fix).expect("validated by clap");
        let mut other = 0usize;
        for (ending, count) in counts.iter() {
            if *ending != target {
                other += *count;
            }
        }
        if other > 0 {
            let _ = apply_ending(filename, &contents, target);
        }
        (other > 0) as i32
    }
}

pub fn run(args: &[String]) -> i32 {
    let args = Args::parse_from(std::iter::once("mixed-line-ending".to_owned()).chain(args.iter().cloned()));

    let mut retv = 0;
    for filename in &args.filenames {
        if fix_filename(filename, &args.fix) != 0 {
            if args.fix == "no" {
                println!("{filename}: mixed line endings");
            } else {
                println!("{filename}: fixed mixed line endings");
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
    fn splitlines_handles_all_endings() {
        assert_eq!(
            splitlines_keepends(b"a\r\nb\nc\rd"),
            vec![&b"a\r\n"[..], &b"b\n"[..], &b"c\r"[..], &b"d"[..]]
        );
    }

    #[test]
    fn detects_mixed() {
        let path = crate::util::write_temp_file(b"a\r\nb\nc\n");
        assert_eq!(fix_filename(path.to_str().unwrap(), "no"), 1);
    }

    #[test]
    fn auto_fixes_to_dominant_ending() {
        let path = crate::util::write_temp_file(b"a\r\nb\nc\n");
        assert_eq!(fix_filename(path.to_str().unwrap(), "auto"), 1);
        let after = std::fs::read(&path).unwrap();
        assert_eq!(after, b"a\nb\nc\n");
    }

    #[test]
    fn consistent_lf_is_not_mixed() {
        let path = crate::util::write_temp_file(b"a\nb\nc\n");
        assert_eq!(fix_filename(path.to_str().unwrap(), "no"), 0);
    }
}
