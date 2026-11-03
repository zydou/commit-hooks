//! Shared helpers for the two shebang-related hooks.
//!
//! `check-executables-have-shebangs` and `check-shebang-scripts-are-executable`
//! both need to read a file's git file-mode and inspect its first bytes; that
//! logic lives here so the two hooks stay consistent.

use crate::util::{cmd_output, zsplit};

/// The file-mode "executable" bits git reports in the user/owner positions.
/// A mode's last three characters are the owner/group/other permission triple;
/// an `x` bit renders as one of these odd digits.
pub const EXECUTABLE_VALUES: &[char] = &['1', '3', '5', '7'];

/// A single entry from `git ls-files --stage`.
#[derive(Debug, Clone)]
pub struct GitLsFile {
    pub mode: String,
    pub filename: String,
}

/// Whether a file begins with a `#!` shebang.
pub fn has_shebang(path: &str) -> bool {
    let Ok(bytes) = std::fs::read(path) else {
        return false;
    };
    bytes.starts_with(b"#!")
}

/// Whether a git file-mode marks the entry executable.
pub fn is_executable_mode(mode: &str) -> bool {
    mode.chars().rev().take(3).any(|c| EXECUTABLE_VALUES.contains(&c))
}

/// Run `git ls-files -z --stage -- <paths...>` and parse each staged entry.
pub fn git_ls_files(paths: &[String]) -> Result<Vec<GitLsFile>, crate::util::CalledProcessError> {
    let mut args: Vec<&str> = vec!["ls-files", "-z", "--stage", "--"];
    args.extend(paths.iter().map(String::as_str));
    let out = cmd_output("git", &args, Some(0))?;

    let mut files = Vec::new();
    for entry in zsplit(&out) {
        // Each entry is `<mode> <object> <stage>\t<filename>`.
        let Some((metadata, filename)) = entry.split_once('\t') else {
            continue;
        };
        let mode = metadata.split_whitespace().next().unwrap_or("").to_owned();
        files.push(GitLsFile { mode, filename: filename.to_owned() });
    }
    Ok(files)
}
