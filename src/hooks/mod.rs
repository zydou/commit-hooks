//! The hook registry.
//!
//! Each hook lives in its own module and exposes `run(args) -> i32`, mirroring
//! the upstream per-module `main(argv) -> int`. `dispatch` maps a pre-commit
//! hook id onto its implementation; `HOOKS` drives `commit-hooks --list`.

pub mod check_added_large_files;
pub mod check_case_conflict;
pub mod check_executables_have_shebangs;
pub mod check_json;
pub mod check_merge_conflict;
pub mod check_shebang_scripts_are_executable;
pub mod check_symlinks;
pub mod check_toml;
pub mod check_vcs_permalinks;
pub mod check_xml;
pub mod check_yaml;
pub mod detect_aws_credentials;
pub mod detect_private_key;
pub mod destroyed_symlinks;
pub mod end_of_file_fixer;
pub mod file_contents_sorter;
pub mod fix_byte_order_marker;
pub mod forbid_new_submodules;
pub mod mixed_line_ending;
pub mod name_tests_test;
pub mod no_commit_to_branch;
pub mod removed;
pub mod requirements_txt_fixer;
pub mod shebang_common;
pub mod sort_simple_yaml;
pub mod trailing_whitespace_fixer;

/// `(hook id, short description)` pairs, used by `commit-hooks --list`.
pub const HOOKS: &[(&str, &str)] = &[
    ("check-added-large-files", "prevents giant files from being committed"),
    ("check-case-conflict", "checks for files that would conflict in case-insensitive filesystems"),
    ("check-executables-have-shebangs", "ensures that (non-binary) executables have a shebang"),
    ("check-json", "checks json files for parseable syntax"),
    ("check-merge-conflict", "checks for files that contain merge conflict strings"),
    ("check-shebang-scripts-are-executable", "ensures that (non-binary) files with a shebang are executable"),
    ("check-symlinks", "checks for symlinks which do not point to anything"),
    ("check-toml", "checks toml files for parseable syntax"),
    ("check-vcs-permalinks", "ensures that links to vcs websites are permalinks"),
    ("check-xml", "checks xml files for parseable syntax"),
    ("check-yaml", "checks yaml files for parseable syntax"),
    ("detect-aws-credentials", "detects *your* aws credentials from the aws cli credentials file"),
    ("detect-private-key", "detects the presence of private keys"),
    ("destroyed-symlinks", "detects symlinks which are changed to regular files"),
    ("end-of-file-fixer", "ensures files end with exactly one newline"),
    ("file-contents-sorter", "sorts the lines in specified files"),
    ("fix-byte-order-marker", "removes utf-8 byte order marker"),
    ("forbid-new-submodules", "prevents addition of new git submodules"),
    ("mixed-line-ending", "replaces or checks mixed line ending"),
    ("name-tests-test", "verifies that test files are named correctly"),
    ("no-commit-to-branch", "disallow commits to specific branches"),
    ("pre-commit-hooks-removed", "informs about removed hooks"),
    ("requirements-txt-fixer", "sorts entries in requirements.txt"),
    ("sort-simple-yaml", "sorts simple yaml files which consist only of top-level keys"),
    ("trailing-whitespace", "trims trailing whitespace"),
];

/// Run the hook identified by `id` with the remaining command-line `args`.
///
/// Returns `None` when `id` does not name a known hook; the caller turns that
/// into a usage error.
pub fn dispatch(id: &str, args: &[String]) -> Option<i32> {
    match id {
        "check-added-large-files" => Some(check_added_large_files::run(args)),
        "check-case-conflict" => Some(check_case_conflict::run(args)),
        "check-executables-have-shebangs" => Some(check_executables_have_shebangs::run(args)),
        "check-json" => Some(check_json::run(args)),
        "check-merge-conflict" => Some(check_merge_conflict::run(args)),
        "check-shebang-scripts-are-executable" => Some(check_shebang_scripts_are_executable::run(args)),
        "check-symlinks" => Some(check_symlinks::run(args)),
        "check-toml" => Some(check_toml::run(args)),
        "check-vcs-permalinks" => Some(check_vcs_permalinks::run(args)),
        "check-xml" => Some(check_xml::run(args)),
        "check-yaml" => Some(check_yaml::run(args)),
        "detect-aws-credentials" => Some(detect_aws_credentials::run(args)),
        "detect-private-key" => Some(detect_private_key::run(args)),
        "destroyed-symlinks" => Some(destroyed_symlinks::run(args)),
        "end-of-file-fixer" => Some(end_of_file_fixer::run(args)),
        "file-contents-sorter" => Some(file_contents_sorter::run(args)),
        "fix-byte-order-marker" => Some(fix_byte_order_marker::run(args)),
        "forbid-new-submodules" => Some(forbid_new_submodules::run(args)),
        "mixed-line-ending" => Some(mixed_line_ending::run(args)),
        "name-tests-test" => Some(name_tests_test::run(args)),
        "no-commit-to-branch" => Some(no_commit_to_branch::run(args)),
        "pre-commit-hooks-removed" => Some(removed::run(args)),
        "requirements-txt-fixer" => Some(requirements_txt_fixer::run(args)),
        "sort-simple-yaml" => Some(sort_simple_yaml::run(args)),
        "trailing-whitespace" => Some(trailing_whitespace_fixer::run(args)),
        _ => None,
    }
}
