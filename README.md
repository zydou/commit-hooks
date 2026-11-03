# commit-hooks

Out-of-the-box [pre-commit](https://pre-commit.com) hooks, re-implemented in
Rust as a single self-contained binary. This is a drop-in replacement for the
reference [`pre-commit-hooks`](https://github.com/pre-commit/pre-commit-hooks)
package: instead of needing a Python interpreter, you drop in one `commit-hooks`
executable and reference it directly from your `.pre-commit-config.yaml`.

## Why

The upstream hooks are Python and require a working Python interpreter at
pre-commit time. This project compiles to a single self-contained `commit-hooks`
binary with no runtime dependencies, so it starts instantly and can be
committed/installed anywhere without a Python toolchain. Releases ship
pre-built binaries for Linux (gnu and musl) and macOS, on both x86_64 and
arm64 — see [Building](#building).

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

All 30 hooks from the upstream `pre-commit-hooks` package:

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

Plus this extra hook, not present in upstream:

- `python-shebang-coding` — add a shebang and a `coding` declaration to Python files that lack one (`--shebang` sets the shebang, default `#!/usr/bin/env python`)

## Building

CI builds a release binary for every supported target and attaches it to a
GitHub Release. Every push publishes to the rolling `nightly` release; pushing a
`v*` tag releases to that tag. Grab the archive for your target:

| Target | Platform |
| --- | --- |
| `x86_64-unknown-linux-gnu` | Linux, x86_64, glibc |
| `x86_64-unknown-linux-musl` | Linux, x86_64, statically linked |
| `aarch64-unknown-linux-gnu` | Linux, arm64, glibc |
| `aarch64-unknown-linux-musl` | Linux, arm64, statically linked |
| `x86_64-apple-darwin` | macOS, Intel |
| `aarch64-apple-darwin` | macOS, Apple silicon |

Each archive is named `commit-hooks-<target>.tar.xz` and contains the
`commit-hooks` binary. Linux targets are cross-compiled with
[cross-rs](https://github.com/cross-rs/cross) from a fixed toolchain image;
the macOS targets build natively on matching runners.

To build from source instead:

```bash
# native
cargo build --release

# a specific target (Linux targets cross-compile via cross-rs)
cross build --release --target x86_64-unknown-linux-musl
```

## Development

- `cargo test` runs the unit tests.
- Each hook lives in `src/hooks/<name>.rs` and exposes `run(args) -> i32`,
  mirroring the upstream per-module `main(argv) -> int`.

## License

MIT
