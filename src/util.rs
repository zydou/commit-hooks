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

/// Build a line-offset table for `src` that can be indexed with a 1-based line
/// number: `table[1]` is `0`, and `table[n]` is the byte offset at which line
/// `n` begins. Lines are terminated by `\n`.
///
/// This mirrors the "padded so we can index with line number" offset tables the
/// upstream token-based hooks build from `src.splitlines(True)`.
pub fn line_start_offsets(src: &str) -> Vec<usize> {
    let mut table = Vec::with_capacity(src.lines().count() + 2);
    table.push(0); // sentinel so the table is 1-based
    let mut offset = 0;
    for line in src.split('\n') {
        table.push(offset);
        offset += line.len() + 1;
    }
    table
}

/// Convert a byte `offset` in `src` into a 1-based `(line, column)` pair.
///
/// `column` is the byte distance from the start of the line, matching Python
/// 3.8+'s `ast`/`tokenize` `(row, col)` tuples, which report UTF-8 byte columns
/// (so a multi-byte character advances the column by more than one).
pub fn line_col(src: &str, offset: usize) -> (usize, usize) {
    let table = line_start_offsets(src);
    let mut line = table.len() - 1;
    while line > 1 && table[line] > offset {
        line -= 1;
    }
    let column = offset - table[line];
    (line, column)
}

/// Write `contents` to a fresh, uniquely-named temp file and return its path.
///
/// Test-only: because `cargo test` runs tests in parallel threads, each
/// invocation must use a distinct path to avoid clobbering a shared fixture.
#[cfg(test)]
pub fn write_temp_file(contents: &[u8]) -> std::path::PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join("commit-hooks-tests");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join(format!("f-{}-{}", std::process::id(), n));
    std::fs::write(&path, contents).expect("write temp file");
    path
}
