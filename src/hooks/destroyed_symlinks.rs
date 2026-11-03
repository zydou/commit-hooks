//! `destroyed-symlinks` — detect symlinks changed into regular files whose
//! content is the path they used to point at.
//!
//! This typically happens on Windows when a repo with symlinks is cloned by a
//! user without permission to create symlinks.

use clap::Parser;

use crate::util::{cmd_output, zsplit};

const ORDINARY_CHANGED_ENTRIES_MARKER: &str = "1";
const PERMS_LINK: &str = "120000";
const PERMS_NONEXIST: &str = "000000";

#[derive(Parser)]
#[command(about = "Detect destroyed symlinks.")]
struct Args {
    /// Filenames to check.
    filenames: Vec<String>,
}

/// Returns the paths that were symlinks in HEAD but are now regular files with
/// the link target as their content.
fn find_destroyed_symlinks(files: &[String]) -> Result<Vec<String>, crate::util::CalledProcessError> {
    let mut destroyed = Vec::new();
    if files.is_empty() {
        return Ok(destroyed);
    }

    let mut args: Vec<&str> = vec!["status", "--porcelain=v2", "-z", "--"];
    args.extend(files.iter().map(String::as_str));
    let out = cmd_output("git", &args, Some(0))?;

    for entry in zsplit(&out) {
        let parts: Vec<&str> = entry.split(' ').collect();
        if parts.first().copied() != Some(ORDINARY_CHANGED_ENTRIES_MARKER) || parts.len() < 8 {
            continue;
        }
        // <state> <submodule> <mode_HEAD> <mode_index> <mode_wt> <hash_HEAD> <hash_index> <path...>
        let mode_head = parts[2];
        let mode_index = parts[3];
        let hash_head = parts[5];
        let hash_index = parts[6];
        let path = parts[7..].join(" ");

        if mode_head == PERMS_LINK && mode_index != PERMS_LINK && mode_index != PERMS_NONEXIST {
            if hash_head == hash_index {
                destroyed.push(path);
            } else {
                // New content may have been altered by a fixer (e.g. trailing
                // whitespace); only treat it as destroyed if the size grew by
                // at most 2 bytes and the trimmed contents match.
                let size = |hash: &str| -> i64 {
                    cmd_output("git", &["cat-file", "-s", hash], Some(0))
                        .ok()
                        .and_then(|s| s.trim().parse().ok())
                        .unwrap_or(0)
                };
                let size_index = size(hash_index);
                let size_head = size(hash_head);
                if size_index <= size_head + 2 {
                    let content = |hash: &str| -> Vec<u8> {
                        cmd_output("git", &["cat-file", "-p", hash], Some(0))
                            .map(|s| s.trim_end().to_owned().into_bytes())
                            .unwrap_or_default()
                    };
                    if content(hash_head) == content(hash_index) {
                        destroyed.push(path);
                    }
                }
            }
        }
    }
    Ok(destroyed)
}

pub fn run(args: &[String]) -> i32 {
    let args = Args::parse_from(std::iter::once("destroyed-symlinks".to_owned()).chain(args.iter().cloned()));

    let destroyed = match find_destroyed_symlinks(&args.filenames) {
        Ok(d) => d,
        Err(_) => return 1,
    };

    if destroyed.is_empty() {
        return 0;
    }

    println!("Destroyed symlinks:");
    for link in &destroyed {
        println!("- {link}");
    }
    println!("You should unstage affected files:");
    println!("\tgit reset HEAD -- {}", destroyed.join(" "));
    println!(
        "And retry commit. As a long term solution you may try to explicitly \
         tell git that your environment does not support symlinks:"
    );
    println!("\tgit config core.symlinks false");
    1
}
