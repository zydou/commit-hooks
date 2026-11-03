//! `pre-commit-hooks-removed` — informative stub for removed hooks.
//!
//! pre-commit's `.pre-commit-hooks.yaml` points a few retired ids here with the
//! replacement hook id and URL baked into the entry arguments.

use std::process::ExitCode;

/// Exit with the "this hook was removed" message, mirroring the upstream
/// `SystemExit(str)` (which prints to stderr and exits non-zero).
pub fn run(args: &[String]) -> i32 {
    // entry passes exactly: <old-id> <new-id> <url>
    let old = args.first().map(String::as_str).unwrap_or("");
    let new = args.get(1).map(String::as_str).unwrap_or("");
    let url = args.get(2).map(String::as_str).unwrap_or("");
    eprintln!("`{old}` has been removed -- use `{new}` from {url}");
    // `SystemExit(str)` exits with status 1.
    let _ = ExitCode::from(1);
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_message_and_fails() {
        // We only assert the return code here; the message goes to stderr.
        assert_eq!(run(&["old".to_owned(), "new".to_owned(), "http://x".to_owned()]), 1);
    }
}
