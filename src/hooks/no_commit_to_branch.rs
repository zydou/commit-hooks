//! `no-commit-to-branch` — protect branches from direct commits.
//!
//! Fails if HEAD currently points at one of the protected branches (by exact
//! name or regex pattern). Defaults to protecting `master` and `main`. This
//! hook runs with no filenames (`pass_filenames: false`).

use clap::Parser;
use regex::Regex;
use std::collections::HashSet;

use crate::util::{cmd_output, CalledProcessError};

#[derive(Parser)]
#[command(about = "Disallow commits to specific branches.")]
struct Args {
    /// Branch to disallow commits to; may be given multiple times.
    #[arg(short = 'b', long = "branch")]
    branch: Vec<String>,

    /// Regex pattern for branch names to disallow; may be given multiple times.
    #[arg(short = 'p', long = "pattern")]
    pattern: Vec<String>,
}

/// Whether HEAD is on a protected branch.
fn is_on_branch(protected: &HashSet<String>, patterns: &[Regex]) -> bool {
    // Outside a normal branch checkout `git symbolic-ref HEAD` fails; treat
    // that (detached HEAD) as not-on-a-protected-branch.
    let ref_name = match cmd_output("git", &["symbolic-ref", "HEAD"], Some(0)) {
        Ok(s) => s,
        Err(CalledProcessError { .. }) => return false,
    };
    let chunks: Vec<&str> = ref_name.trim().split('/').collect();
    let branch_name = chunks.iter().skip(2).copied().collect::<Vec<_>>().join("/");
    protected.contains(&branch_name) || patterns.iter().any(|re| is_match_at_start(re, &branch_name))
}

/// Equivalent of Python's `re.match`: the pattern must match starting at the
/// beginning of the string (but need not consume the whole string).
fn is_match_at_start(re: &Regex, text: &str) -> bool {
    re.find(text).is_some_and(|m| m.start() == 0)
}

pub fn run(args: &[String]) -> i32 {
    let args = Args::parse_from(
        std::iter::once("no-commit-to-branch".to_owned()).chain(args.iter().cloned()),
    );

    let protected: HashSet<String> = if args.branch.is_empty() {
        ["master".to_owned(), "main".to_owned()].into()
    } else {
        args.branch.iter().cloned().collect()
    };
    let patterns: Vec<Regex> = args
        .pattern
        .iter()
        .filter_map(|p| Regex::new(p).ok())
        .collect();

    is_on_branch(&protected, &patterns) as i32
}

#[cfg(test)]
mod tests {
    use super::is_match_at_start;
    use regex::Regex;

    #[test]
    fn pattern_matches_at_start_only() {
        let re = Regex::new("release/.*").unwrap();
        assert!(is_match_at_start(&re, "release/1.0"));
        assert!(!is_match_at_start(&re, "feature/release/x"));
    }

    #[test]
    fn pattern_does_not_need_full_match() {
        let re = Regex::new("main").unwrap();
        assert!(is_match_at_start(&re, "mainline"));
    }
}
