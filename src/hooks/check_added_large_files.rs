//! `check-added-large-files` — prevent giant files from being committed.
//!
//! Files larger than `--maxkb` (default 500 KB) fail the hook. By default only
//! files git has staged for addition are considered, and git-lfs-tracked files
//! are skipped.

use clap::Parser;
use std::collections::HashSet;
use std::io::Write;
use std::process::{Command, Stdio};

use crate::util::{added_files, zsplit};

#[derive(Parser)]
#[command(about = "Prevent giant files from being committed.")]
struct Args {
    /// Filenames pre-commit believes are changed.
    filenames: Vec<String>,

    /// Enforce all listed files are checked, not just staged additions.
    #[arg(long = "enforce-all")]
    enforce_all: bool,

    /// Maximum allowable KB for added files.
    #[arg(long = "maxkb", default_value_t = 500)]
    maxkb: i64,
}

/// Remove files tracked by git-lfs from `filenames`.
fn filter_lfs_files(filenames: &mut HashSet<String>) -> std::io::Result<()> {
    if filenames.is_empty() {
        return Ok(());
    }
    let input = filenames.iter().cloned().collect::<Vec<_>>().join("\0");
    let mut child = Command::new("git")
        .args(["check-attr", "filter", "-z", "--stdin"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    child.stdin.as_mut().unwrap().write_all(input.as_bytes())?;
    let output = child.wait_with_output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parts = zsplit(&stdout);
    for chunk in parts.chunks(3) {
        if chunk.len() == 3 && chunk[2] == "lfs" {
            filenames.remove(&chunk[0]);
        }
    }
    Ok(())
}

pub fn run(args: &[String]) -> i32 {
    let args = Args::parse_from(
        std::iter::once("check-added-large-files".to_owned()).chain(args.iter().cloned()),
    );

    let mut filenames_filtered: HashSet<String> = args.filenames.iter().cloned().collect();
    if filter_lfs_files(&mut filenames_filtered).is_err() {
        // If git-lfs introspection fails, fall through with the unfiltered set.
    }

    if !args.enforce_all {
        let Ok(added) = added_files() else {
            return 1;
        };
        filenames_filtered.retain(|f| added.contains(f));
    }

    let mut retv = 0;
    for filename in filenames_filtered {
        let Ok(meta) = std::fs::metadata(&filename) else {
            continue;
        };
        let kb = meta.len().div_ceil(1024) as i64;
        if kb > args.maxkb {
            println!("{filename} ({kb} KB) exceeds {} KB.", args.maxkb);
            retv = 1;
        }
    }
    retv
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn large_file_is_flagged() {
        let dir = std::env::temp_dir().join("large-file-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("big");
        std::fs::write(&path, vec![0u8; 2000]).unwrap();
        let code = run(&[
            "--enforce-all".to_owned(),
            "--maxkb".to_owned(),
            "1".to_owned(),
            path.to_string_lossy().into_owned(),
        ]);
        assert_eq!(code, 1);
    }

    #[test]
    fn small_file_passes() {
        let dir = std::env::temp_dir().join("large-file-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("small");
        std::fs::write(&path, vec![0u8; 10]).unwrap();
        let code = run(&[
            "--enforce-all".to_owned(),
            "--maxkb".to_owned(),
            "1".to_owned(),
            path.to_string_lossy().into_owned(),
        ]);
        assert_eq!(code, 0);
    }
}
