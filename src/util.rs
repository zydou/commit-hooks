//! Shared helpers used by the individual hooks.
//!
//! This mirrors the small `pre_commit_hooks/util.py` module from the upstream
//! project: a couple of `git` helpers plus a NUL-splitting utility. Keeping it
//! tiny avoids speculative abstractions; each hook stays self-contained.

use std::collections::HashSet;
use std::process::Command;

/// Error returned when a subprocess exits with an unexpected status.
#[derive(Debug)]
pub struct CalledProcessError {
    pub cmd: String,
    pub status: Option<i32>,
}

impl std::fmt::Display for CalledProcessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "command failed: {} (status {:?})", self.cmd, self.status)
    }
}

impl std::error::Error for CalledProcessError {}

/// Run a command, returning its decoded standard output.
///
/// Mirrors `util.cmd_output`: both stdout and stderr are captured, and when
/// `retcode` is `Some(expected)` a differing exit status is an error.
pub fn cmd_output(cmd: &str, args: &[&str], retcode: Option<i32>) -> Result<String, CalledProcessError> {
    let output = Command::new(cmd).args(args).output().map_err(|e| CalledProcessError {
        cmd: format!("{cmd} {args:?}: {e}"),
        status: None,
    })?;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    if let Some(expected) = retcode {
        if output.status.code() != Some(expected) {
            return Err(CalledProcessError { cmd: format!("{cmd} {args:?}"), status: output.status.code() });
        }
    }
    Ok(stdout)
}

/// The set of paths that git has staged for addition.
pub fn added_files() -> Result<HashSet<String>, CalledProcessError> {
    let out = cmd_output(
        "git",
        &["diff", "--staged", "--name-only", "--diff-filter=A"],
        Some(0),
    )?;
    Ok(out.lines().map(str::to_owned).collect())
}

/// Split a NUL-separated string, dropping surrounding NULs; empty yields `[]`.
pub fn zsplit(s: &str) -> Vec<String> {
    let s = s.trim_matches('\0');
    if s.is_empty() { Vec::new() } else { s.split('\0').map(str::to_owned).collect() }
}
