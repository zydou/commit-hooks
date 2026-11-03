//! commit-hooks — out-of-the-box pre-commit hooks as a single Rust binary.
//!
//! The upstream [`pre-commit-hooks`](https://github.com/pre-commit/pre-commit-hooks)
//! project ships one Python entry point per hook. This binary replaces that
//! package with a single statically-linked executable. It is invoked by id:
//!
//! ```text
//! commit-hooks <hook-id> [hook-args...] [filenames...]
//! ```
//!
//! pre-commit appends the matched filenames to `entry`, so a
//! `.pre-commit-config.yaml` hook simply sets `entry: commit-hooks <hook-id>`.
//! Each hook module exposes `run(args) -> i32` and returns the process exit
//! code, exactly like the upstream `main(argv) -> int` functions.

mod hooks;
mod util;

use std::process::ExitCode;

fn main() -> ExitCode {
    // Collect args after argv[0]. The first remaining token selects the hook.
    let args: Vec<String> = std::env::args().skip(1).collect();

    let Some((id, rest)) = args.split_first() else {
        eprintln!("usage: commit-hooks <hook-id> [args...] [filenames...]");
        eprintln!("       commit-hooks --list              # list available hooks");
        return ExitCode::from(2);
    };

    if id == "--list" || id == "list" || id == "--help" {
        println!("available hooks:");
        for (hook_id, desc) in hooks::HOOKS {
            println!("  {hook_id:<40} {desc}");
        }
        return ExitCode::from(0);
    }

    let code = match hooks::dispatch(id, rest) {
        Some(code) => code,
        None => {
            eprintln!("commit-hooks: unknown hook {id:?}");
            eprintln!("run `commit-hooks --list` for available hooks");
            return ExitCode::from(2);
        }
    };

    ExitCode::from(code.clamp(0, 255) as u8)
}
