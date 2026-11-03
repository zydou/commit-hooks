//! `check-vcs-permalinks` — ensure links to GitHub point at permalinks.
//!
//! A GitHub `blob/` URL is a permalink when the path segment after `blob/` is a
//! full commit SHA (hex, 4–64 chars) rather than a branch/tag name.

use clap::Parser;
use regex::bytes::Regex;
use std::io::Write;

#[derive(Parser)]
#[command(about = "Ensure that links to vcs websites are permalinks.")]
struct Args {
    /// Filenames to check.
    filenames: Vec<String>,

    /// Add a GitHub domain to check (repeatable). Defaults to github.com.
    #[arg(long = "additional-github-domain", default_value = "github.com")]
    additional_github_domains: Vec<String>,
}

fn get_pattern(domain: &str) -> Regex {
    // Capture the path segment right after `blob/`. The upstream uses a
    // negative lookahead to skip permalinks, but the `regex` crate does not
    // support look-around, so we capture the segment and classify it in code.
    let re = format!(r"https://{domain}/[^/ ]+/[^/ ]+/blob/([^/. ]+)/[^# ]+#L\d+");
    Regex::new(&re).expect("valid pattern")
}

/// Whether a `blob/<segment>` refers to a full commit SHA (a permalink) rather
/// than a branch or tag name.
fn is_permalink_sha(segment: &[u8]) -> bool {
    let len = segment.len();
    (4..=64).contains(&len) && segment.iter().all(|b| b.is_ascii_hexdigit())
}

fn check_filename(filename: &str, patterns: &[Regex]) -> i32 {
    let Ok(bytes) = std::fs::read(filename) else {
        return 0;
    };
    let mut retv = 0;
    for (i, line) in bytes.split(|&b| b == b'\n').enumerate() {
        for pattern in patterns {
            if let Some(caps) = pattern.captures(line) {
                let segment = caps.get(1).map(|m| m.as_bytes()).unwrap_or(b"");
                if is_permalink_sha(segment) {
                    continue;
                }
                print!("{filename}:{}:", i + 1);
                // The upstream writes the raw (newline-terminated) line.
                let mut out = line.to_vec();
                out.push(b'\n');
                std::io::stdout().lock().write_all(&out).ok();
                retv = 1;
            }
        }
    }
    retv
}

pub fn run(args: &[String]) -> i32 {
    let args = Args::parse_from(std::iter::once("check-vcs-permalinks".to_owned()).chain(args.iter().cloned()));

    let patterns: Vec<Regex> = args.additional_github_domains.iter().map(|d| get_pattern(d)).collect();

    let mut retv = 0;
    for filename in &args.filenames {
        retv |= check_filename(filename, &patterns);
    }

    if retv != 0 {
        println!();
        println!("Non-permanent github link detected.");
        println!("On any page on github press [y] to load a permalink.");
    }
    retv
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_with(contents: &[u8]) -> i32 {
        let path = crate::util::write_temp_file(contents);
        run(&[path.to_string_lossy().into_owned()])
    }

    #[test]
    fn branch_link_is_flagged() {
        let content = b"see https://github.com/org/repo/blob/main/file.py#L12\n";
        assert_eq!(run_with(content), 1);
    }

    #[test]
    fn sha_permalink_passes() {
        let content = b"see https://github.com/org/repo/blob/abcd1234/file.py#L12\n";
        assert_eq!(run_with(content), 0);
    }
}
