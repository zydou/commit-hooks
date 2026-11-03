//! `check-executables-have-shebangs` — executable text files must have a shebang.

use clap::Parser;
use std::io::Write;

use super::shebang_common::{git_ls_files, has_shebang, is_executable_mode};

#[derive(Parser)]
#[command(about = "Check that executable text files have a shebang.")]
struct Args {
    /// Filenames to check.
    filenames: Vec<String>,
}

fn message(path: &str) {
    let stderr = std::io::stderr();
    let mut h = stderr.lock();
    let _ = writeln!(
        h,
        "{path}: marked executable but has no (or invalid) shebang!\n  \
         If it isn't supposed to be executable, try: `chmod -x {path}`\n  \
         If on Windows, you may also need to: `git add --chmod=-x {path}`\n  \
         If it is supposed to be executable, double-check its shebang."
    );
}

/// Fallback used when the filesystem doesn't track the executable bit
/// (`core.fileMode=false`): inspect the mode recorded in the git index.
fn check_git_filemode(paths: &[String]) -> i32 {
    let mut seen = false;
    let Ok(files) = git_ls_files(paths) else {
        return 1;
    };
    for f in &files {
        if is_executable_mode(&f.mode) && !has_shebang(&f.filename) {
            message(&f.filename);
            seen = true;
        }
    }
    seen as i32
}

pub fn run(args: &[String]) -> i32 {
    let args = Args::parse_from(
        std::iter::once("check-executables-have-shebangs".to_owned()).chain(args.iter().cloned()),
    );

    // When the FS doesn't track the executable bit we must consult git's index.
    let fs_tracks_exec_bit = crate::util::cmd_output(
        "git",
        &["config", "core.fileMode"],
        None,
    )
    .map(|s| s.trim().to_owned())
    .unwrap_or_default();
    if fs_tracks_exec_bit == "false" {
        return check_git_filemode(&args.filenames);
    }

    let mut retv = 0;
    for path in &args.filenames {
        if !has_shebang(path) {
            message(path);
            retv = 1;
        }
    }
    retv
}
