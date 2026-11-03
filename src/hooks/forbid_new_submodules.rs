//! `forbid-new-submodules` — prevent new git submodules from being added.
//!
//! Intended as a migration helper; use `forbid-submodules` to ban them entirely.

use clap::Parser;

use crate::util::cmd_output;

#[derive(Parser)]
#[command(about = "Prevent addition of new git submodules.")]
struct Args {
    /// Filenames pre-commit believes are changed.
    filenames: Vec<String>,
}

pub fn run(args: &[String]) -> i32 {
    let args = Args::parse_from(
        std::iter::once("forbid-new-submodules".to_owned()).chain(args.iter().cloned()),
    );

    // During `pre-commit`/`pre-push` for a push, pre-commit exposes the ref
    // range; otherwise compare against the staged index.
    let diff_arg = match (
        std::env::var("PRE_COMMIT_FROM_REF"),
        std::env::var("PRE_COMMIT_TO_REF"),
    ) {
        (Ok(from), Ok(to)) => format!("{from}...{to}"),
        _ => "--staged".to_owned(),
    };

    // Build `git diff --diff-filter=A --raw <diff_arg> -- <filenames...>`.
    let mut git_args: Vec<String> = vec![
        "diff".into(),
        "--diff-filter=A".into(),
        "--raw".into(),
        diff_arg,
        "--".into(),
    ];
    git_args.extend(args.filenames.iter().cloned());
    let git_args: Vec<&str> = git_args.iter().map(String::as_str).collect();

    let added_diff = match cmd_output("git", &git_args, Some(0)) {
        Ok(out) => out,
        Err(_) => return 1,
    };

    let mut retv = 0;
    for line in added_diff.lines() {
        let Some((metadata, filename)) = line.split_once('\t') else {
            continue;
        };
        // Raw metadata looks like `:000000 160000 0000000 0000000 A`.
        let new_mode = metadata.split(' ').nth(1).unwrap_or("");
        if new_mode == "160000" {
            println!("{filename}: new submodule introduced");
            retv = 1;
        }
    }

    if retv != 0 {
        println!();
        println!("This commit introduces new submodules.");
        println!("Did you unintentionally `git add .`?");
        println!("To fix: git rm {{thesubmodule}}  # no trailing slash");
        println!("Also check .gitmodules");
    }

    retv
}
