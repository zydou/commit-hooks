# commit-hooks

Out-of-the-box [pre-commit](https://pre-commit.com) hooks, re-implemented in
Rust as a single statically-linked binary. This is a drop-in replacement for the
reference [`pre-commit-hooks`](https://github.com/pre-commit/pre-commit-hooks)
package: instead of needing a Python interpreter, you compile one `commit-hooks`
executable and reference it directly from your `.pre-commit-config.yaml`.

## Why

The upstream hooks are Python and require a working Python interpreter at
pre-commit time. This project compiles to a single
`x86_64-unknown-linux-musl` binary with no runtime dependencies, so it starts
instantly and can be committed/installed anywhere without a Python toolchain.

## Usage

Each hook is selected by its id, exactly as in the upstream `.pre-commit-hooks.yaml`:

```bash
commit-hooks <hook-id> [hook-args...] [filenames...]
```

pre-commit appends the matched filenames to `entry`, so a hook is wired up like:

```yaml
-   repo: https://github.com/zydou/commit-hooks
    rev: v0.1.0
    hooks:
    -   id: trailing-whitespace
        entry: commit-hooks trailing-whitespace   # entry points at the binary + id
```

List the available hooks with:

```bash
commit-hooks --list
```

## Implemented hooks

All 31 hooks from the upstream `pre-commit-hooks` package:

- `check-added-large-files` — prevent giant files from being committed
- `check-ast` — check Python files for parseable syntax
- `check-builtin-literals` — require literal syntax for builtin type calls
- `check-case-conflict` — check for files that would conflict on case-insensitive filesystems
- `check-docstring-first` — check that a module docstring appears first
- `check-executables-have-shebangs` — ensure executables have a shebang
- `check-json` — validate JSON syntax (rejects duplicate keys)
- `check-merge-conflict` — check for files that contain merge conflict strings
- `check-shebang-scripts-are-executable` — ensure shebang scripts are executable
- `check-symlinks` — check for symlinks which do not point to anything
- `check-toml` — validate TOML syntax
- `check-vcs-permalinks` — ensure links to VCS websites are permalinks
- `check-xml` — validate XML syntax
- `check-yaml` — validate YAML syntax
- `debug-statements` — check for debugger imports and `breakpoint()` calls
- `detect-aws-credentials` — detect AWS credentials from the credentials file
- `detect-private-key` — detect the presence of private keys
- `destroyed-symlinks` — detect symlinks changed to regular files
- `double-quote-string-fixer` — fix strings that use double quotes
- `end-of-file-fixer` — ensure files end with exactly one newline
- `file-contents-sorter` — sort the lines in specified files
- `fix-byte-order-marker` — remove the UTF-8 byte order marker
- `forbid-new-submodules` — prevent addition of new git submodules
- `mixed-line-ending` — replace or check mixed line endings
- `name-tests-test` — verify that test files are named correctly
- `no-commit-to-branch` — disallow commits to specific branches
- `pretty-format-json` — set a standard for formatting JSON files
- `pre-commit-hooks-removed` — inform about removed hooks
- `requirements-txt-fixer` — sort entries in requirements.txt
- `sort-simple-yaml` — sort simple top-level-key YAML files
- `trailing-whitespace` — trim trailing whitespace

## Building

```bash
# native
cargo build --release

# static musl binary
cross build --release --target x86_64-unknown-linux-musl
```

## Development

- `cargo test` runs the unit tests.
- Each hook lives in `src/hooks/<name>.rs` and exposes `run(args) -> i32`,
  mirroring the upstream per-module `main(argv) -> int`.

## License

MIT
