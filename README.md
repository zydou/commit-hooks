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

- `check-json` — validate JSON syntax (rejects duplicate keys)
- `detect-private-key` — detect the presence of private keys
- `end-of-file-fixer` — ensure files end with exactly one newline
- `fix-byte-order-marker` — remove the UTF-8 byte order marker
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
