//! `check-case-conflict` — detect filenames that would collide on a
//! case-insensitive filesystem (macOS HFS+, Windows FAT).
//!
//! Both files already tracked by git and files newly added by this commit are
//! lower-cased and compared for collisions.

use clap::Parser;
use std::collections::HashSet;

use crate::util::{added_files, cmd_output};

#[derive(Parser)]
#[command(about = "Check for filenames that would conflict on case-insensitive filesystems.")]
struct Args {
    /// Filenames pre-commit believes are changed.
    filenames: Vec<String>,
}

/// Every ancestor directory of `file` (its parent, grandparent, …).
fn parents(file: &str) -> Vec<String> {
    let mut parts: Vec<&str> = file.split('/').collect();
    parts.pop();
    let mut out = Vec::new();
    while !parts.is_empty() {
        out.push(parts.join("/"));
        parts.pop();
    }
    out
}

/// All ancestor directories for every file in `files`.
fn directories_for(files: &HashSet<String>) -> HashSet<String> {
    files.iter().flat_map(|f| parents(f)).collect()
}

pub fn run(args: &[String]) -> i32 {
    let args = Args::parse_from(
        std::iter::once("check-case-conflict".to_owned()).chain(args.iter().cloned()),
    );

    let repo_files: HashSet<String> = match cmd_output("git", &["ls-files"], Some(0)) {
        Ok(out) => out.lines().map(str::to_owned).collect(),
        Err(_) => return 1,
    };
    let added = match added_files() {
        Ok(s) => s,
        Err(_) => return 1,
    };

    let mut repo_files = repo_files;
    repo_files.extend(directories_for(&repo_files));

    let mut relevant_files: HashSet<String> = args.filenames.iter().cloned().collect();
    relevant_files.extend(added);
    relevant_files.extend(directories_for(&relevant_files));

    // `repo_files` should only hold pre-existing entries for conflict purposes.
    let existing: HashSet<String> = repo_files
        .difference(&relevant_files)
        .cloned()
        .collect();

    let lower = |s: &HashSet<String>| s.iter().map(|x| x.to_lowercase()).collect::<HashSet<_>>();

    // A new file conflicting with an existing file.
    let mut conflicts: HashSet<String> = lower(&existing).intersection(&lower(&relevant_files)).cloned().collect();

    // A new file conflicting with another new file: when a lower-cased name
    // occurs more than once among the relevant files.
    let mut seen = HashSet::new();
    for name in relevant_files.iter().map(|f| f.to_lowercase()) {
        if !seen.insert(name.clone()) {
            conflicts.insert(name);
        }
    }

    if conflicts.is_empty() {
        return 0;
    }

    let mut conflicting_files: Vec<String> = (existing.iter().chain(relevant_files.iter()))
        .filter(|f| conflicts.contains(&f.to_lowercase()))
        .cloned()
        .collect();
    conflicting_files.sort();
    conflicting_files.dedup();
    for filename in &conflicting_files {
        println!("Case-insensitivity conflict found: {filename}");
    }
    1
}

#[cfg(test)]
mod tests {
    use super::{directories_for, parents};
    use std::collections::HashSet;

    #[test]
    fn parents_walks_up() {
        assert_eq!(parents("a/b/c"), vec!["a/b", "a"]);
        assert_eq!(parents("a"), Vec::<String>::new());
    }

    #[test]
    fn directories_for_collects_ancestors() {
        let files: HashSet<String> = ["a/b/c".to_owned(), "x".to_owned()].into();
        let dirs = directories_for(&files);
        assert!(dirs.contains("a/b"));
        assert!(dirs.contains("a"));
        assert!(!dirs.contains("x"));
    }
}
