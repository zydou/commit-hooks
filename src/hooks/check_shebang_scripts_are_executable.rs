//! `check-shebang-scripts-are-executable` — text files with a shebang must be
//! marked executable.

use clap::Parser;
use std::io::Write;

use super::shebang_common::{git_ls_files, has_shebang, is_executable_mode};

#[derive(Parser)]
#[command(about = "Check that text files with a shebang are executable.")]
struct Args {
    /// Filenames to check.
    filenames: Vec<String>,
}

fn message(path: &str) {
    let stderr = std::io::stderr();
    let mut h = stderr.lock();
    let _ = writeln!(
        h,
        "{path}: has a shebang but is not marked executable!\n  \
         If it is supposed to be executable, try: `chmod +x {path}`\n  \
         If on Windows, you may also need to: `git add --chmod=+x {path}`\n  \
         If it is not supposed to be executable, double-check its shebang is wanted."
    );
}

/// This hook always inspects the mode recorded in the git index (rather than
/// the filesystem bit) so the check also works on platforms like win32 where
/// non-executability is the common source of problems.
fn check_git_filemode(paths: &[String]) -> i32 {
    let mut seen = false;
    let Ok(files) = git_ls_files(paths) else {
        return 1;
    };
    for f in &files {
        if !is_executable_mode(&f.mode) && has_shebang(&f.filename) {
            message(&f.filename);
            seen = true;
        }
    }
    seen as i32
}

pub fn run(args: &[String]) -> i32 {
    let args = Args::parse_from(
        std::iter::once("check-shebang-scripts-are-executable".to_owned())
            .chain(args.iter().cloned()),
    );
    check_git_filemode(&args.filenames)
}
