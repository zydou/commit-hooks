//! The hook registry.
//!
//! Each hook lives in its own module and exposes `run(args) -> i32`, mirroring
//! the upstream per-module `main(argv) -> int`. `dispatch` maps a pre-commit
//! hook id onto its implementation; `HOOKS` drives `commit-hooks --list`.

pub mod check_json;
pub mod detect_private_key;
pub mod end_of_file_fixer;
pub mod fix_byte_order_marker;
pub mod trailing_whitespace_fixer;

/// `(hook id, short description)` pairs, used by `commit-hooks --list`.
pub const HOOKS: &[(&str, &str)] = &[
    ("check-json", "checks json files for parseable syntax"),
    ("detect-private-key", "detects the presence of private keys"),
    ("end-of-file-fixer", "ensures files end with exactly one newline"),
    ("fix-byte-order-marker", "removes utf-8 byte order marker"),
    ("trailing-whitespace", "trims trailing whitespace"),
];

/// Run the hook identified by `id` with the remaining command-line `args`.
///
/// Returns `None` when `id` does not name a known hook; the caller turns that
/// into a usage error.
pub fn dispatch(id: &str, args: &[String]) -> Option<i32> {
    match id {
        "check-json" => Some(check_json::run(args)),
        "detect-private-key" => Some(detect_private_key::run(args)),
        "end-of-file-fixer" => Some(end_of_file_fixer::run(args)),
        "fix-byte-order-marker" => Some(fix_byte_order_marker::run(args)),
        "trailing-whitespace" => Some(trailing_whitespace_fixer::run(args)),
        _ => None,
    }
}
