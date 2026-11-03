//! `pretty-format-json` — enforce a consistent, "pretty" JSON formatting.
//!
//! "Pretty" means keys are sorted (unless `--no-sort-keys`), an optional set of
//! `--top-keys` is hoisted to the top, and the file is indented (2 spaces by
//! default). Non-ASCII is escaped (`ensure_ascii`) unless `--no-ensure-ascii`.
//! Without `--autofix` the hook prints a unified diff of what would change.

use clap::Parser;
use serde::Serialize;
use serde_json::{Map, Value};
use std::io::Write;

#[derive(Parser)]
#[command(about = "Checks that all your JSON files are pretty.")]
struct Args {
    /// Automatically fix files that are not pretty.
    #[arg(long)]
    autofix: bool,

    /// Number of indent spaces, or a string used as the indent delimiter.
    #[arg(long, default_value = "2")]
    indent: String,

    /// Keep non-ASCII characters instead of \\uXXXX escapes.
    #[arg(long = "no-ensure-ascii")]
    no_ensure_ascii: bool,

    /// Keep JSON nodes in their original order.
    #[arg(long = "no-sort-keys")]
    no_sort_keys: bool,

    /// Comma-separated keys to keep at the top of mappings.
    #[arg(long = "top-keys", default_value = "")]
    top_keys: String,

    /// Filenames to fix.
    filenames: Vec<String>,
}

/// Rebuild every object so `top_keys` come first (in `top_keys` order) and the
/// remaining keys are sorted (when `sort_keys`). Arrays are recursed into.
fn reorder(value: &mut Value, top_keys: &[String], sort_keys: bool) {
    match value {
        Value::Object(map) => {
            let entries: Vec<(String, Value)> =
                map.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
            let mut new_map = Map::new();
            for tk in top_keys {
                if let Some((_, v)) = entries.iter().find(|(k, _)| k == tk) {
                    let mut v = v.clone();
                    reorder(&mut v, top_keys, sort_keys);
                    new_map.insert(tk.clone(), v);
                }
            }
            let mut rest: Vec<(String, Value)> = entries
                .into_iter()
                .filter(|(k, _)| !top_keys.contains(k))
                .collect();
            if sort_keys {
                rest.sort_by(|a, b| a.0.cmp(&b.0));
            }
            for (k, mut v) in rest {
                reorder(&mut v, top_keys, sort_keys);
                new_map.insert(k, v);
            }
            *value = Value::Object(new_map);
        }
        Value::Array(arr) => {
            for v in arr.iter_mut() {
                reorder(v, top_keys, sort_keys);
            }
        }
        _ => {}
    }
}

/// Escape every non-ASCII character as `\\uXXXX` (surrogate pairs for astral
/// characters), matching Python's `json.dumps(ensure_ascii=True)`.
fn ensure_ascii(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        let cp = ch as u32;
        if cp <= 0x7F {
            out.push(ch);
        } else if cp <= 0xFFFF {
            out.push_str(&format!("\\u{cp:04x}"));
        } else {
            let v = cp - 0x1_0000;
            let hi = 0xD800 + (v >> 10);
            let lo = 0xDC00 + (v & 0x3FF);
            out.push_str(&format!("\\u{hi:04x}\\u{lo:04x}"));
        }
    }
    out
}

/// Resolve the `--indent` argument: an integer means that many spaces, any
/// other string is used verbatim as the indent delimiter.
fn resolve_indent(indent: &str) -> Vec<u8> {
    match indent.parse::<usize>() {
        Ok(n) => vec![b' '; n],
        Err(_) => indent.as_bytes().to_vec(),
    }
}

fn get_pretty_format(
    contents: &str,
    indent: &str,
    ensure_ascii_flag: bool,
    sort_keys: bool,
    top_keys: &[String],
) -> Result<String, serde_json::Error> {
    let mut value: Value = serde_json::from_str(contents)?;
    reorder(&mut value, top_keys, sort_keys);

    let indent_bytes = resolve_indent(indent);
    let formatter = serde_json::ser::PrettyFormatter::with_indent(&indent_bytes);
    let mut buf = Vec::new();
    {
        let mut ser = serde_json::ser::Serializer::with_formatter(&mut buf, formatter);
        value.serialize(&mut ser)?;
    }
    let s = String::from_utf8(buf).unwrap_or_default();
    let s = if ensure_ascii_flag { ensure_ascii(&s) } else { s };
    Ok(format!("{s}\n"))
}

/// Split into lines keeping their terminators (`\n`, `\r\n`, or `\r`), like
/// Python's `str.splitlines(keepends=True)`.
fn splitlines_keepends(s: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let bytes = s.as_bytes();
    let mut start = 0;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\n' => {
                lines.push(&s[start..=i]);
                i += 1;
                start = i;
            }
            b'\r' => {
                // \r\n counts as one terminator.
                if i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
                    lines.push(&s[start..=i + 1]);
                    i += 2;
                } else {
                    lines.push(&s[start..=i]);
                    i += 1;
                }
                start = i;
            }
            _ => i += 1,
        }
    }
    if start < s.len() {
        lines.push(&s[start..]);
    }
    lines
}

/// Unified diff of `source` vs `target` (both keep their line endings), with
/// three lines of context, in the style of Python's `difflib.unified_diff`.
fn unified_diff(source: &str, target: &str, file: &str) -> String {
    let a = splitlines_keepends(source);
    let b = splitlines_keepends(target);
    let n = 3;

    let ops = diff_opcodes(&a, &b);
    let groups = grouped_opcodes(&ops, n);

    let mut out = String::new();
    if groups.is_empty() {
        return out;
    }
    out.push_str(&format!("--- {file}\n"));
    out.push_str(&format!("+++ {file}\n"));
    for group in &groups {
        let a_start = group.first().unwrap().i1;
        let a_end = group.last().unwrap().i2;
        let b_start = group.first().unwrap().j1;
        let b_end = group.last().unwrap().j2;
        out.push_str(&format!(
            "@@ -{} +{} @@\n",
            format_range(a_start, a_end),
            format_range(b_start, b_end),
        ));
        for op in group {
            match op.tag {
                Tag::Equal => {
                    for line in &a[op.i1..op.i2] {
                        out.push(' ');
                        out.push_str(line);
                    }
                }
                Tag::Delete => {
                    for line in &a[op.i1..op.i2] {
                        out.push('-');
                        out.push_str(line);
                    }
                }
                Tag::Insert => {}
            }
            if matches!(op.tag, Tag::Insert) {
                for line in &b[op.j1..op.j2] {
                    out.push('+');
                    out.push_str(line);
                }
            }
        }
    }
    out
}

fn format_range(start: usize, end: usize) -> String {
    let length = end.saturating_sub(start);
    if length == 1 {
        format!("{}", start + 1)
    } else {
        if length == 0 {
            format!("{},0", start)
        } else {
            format!("{},{}", start + 1, length)
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Tag {
    Equal,
    Delete,
    Insert,
}

#[derive(Clone, Copy, Debug)]
struct Op {
    tag: Tag,
    i1: usize,
    i2: usize,
    j1: usize,
    j2: usize,
}

/// Longest-common-subsequence diff producing per-line opcodes.
fn diff_opcodes(a: &[&str], b: &[&str]) -> Vec<Op> {
    let n = a.len();
    let m = b.len();
    let mut dp = vec![vec![0u32; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            dp[i][j] = if a[i] == b[j] {
                dp[i + 1][j + 1] + 1
            } else {
                dp[i + 1][j].max(dp[i][j + 1])
            };
        }
    }
    let mut ops = Vec::new();
    let (mut i, mut j) = (0usize, 0usize);
    while i < n && j < m {
        if a[i] == b[j] {
            ops.push(Op { tag: Tag::Equal, i1: i, i2: i + 1, j1: j, j2: j + 1 });
            i += 1;
            j += 1;
        } else if dp[i + 1][j] >= dp[i][j + 1] {
            ops.push(Op { tag: Tag::Delete, i1: i, i2: i + 1, j1: j, j2: j });
            i += 1;
        } else {
            ops.push(Op { tag: Tag::Insert, i1: i, i2: i, j1: j, j2: j + 1 });
            j += 1;
        }
    }
    while i < n {
        ops.push(Op { tag: Tag::Delete, i1: i, i2: i + 1, j1: j, j2: j });
        i += 1;
    }
    while j < m {
        ops.push(Op { tag: Tag::Insert, i1: i, i2: i, j1: j, j2: j + 1 });
        j += 1;
    }
    ops
}

/// Group opcodes into hunks with `n` lines of context, merging nearby changes.
fn grouped_opcodes(ops: &[Op], n: usize) -> Vec<Vec<Op>> {
    if ops.iter().all(|op| op.tag == Tag::Equal) {
        return Vec::new();
    }
    // Trim long equal runs to n at each edge, with a None separator between.
    let mut codes: Vec<Option<Op>> = Vec::new();
    for op in ops {
        if op.tag == Tag::Equal && op.i2 - op.i1 > 2 * n {
            codes.push(Some(Op { tag: Tag::Equal, i1: op.i1, i2: op.i1 + n, j1: op.j1, j2: op.j1 + n }));
            codes.push(None);
            codes.push(Some(Op { tag: Tag::Equal, i1: op.i2 - n, i2: op.i2, j1: op.j2 - n, j2: op.j2 }));
        } else {
            codes.push(Some(*op));
        }
    }

    let mut groups: Vec<Vec<Op>> = Vec::new();
    let mut group: Vec<Op> = Vec::new();
    for code in codes {
        match code {
            None => {
                if !group.is_empty() {
                    groups.push(std::mem::take(&mut group));
                }
            }
            Some(op) => group.push(op),
        }
    }
    if !group.is_empty() {
        groups.push(group);
    }

    // Drop groups that contain no actual change (pure leading/trailing context).
    groups
        .into_iter()
        .filter(|g| g.iter().any(|op| op.tag != Tag::Equal))
        .collect()
}

pub fn run(args: &[String]) -> i32 {
    let args = Args::parse_from(std::iter::once("pretty-format-json".to_owned()).chain(args.iter().cloned()));

    let top_keys: Vec<String> = if args.top_keys.is_empty() {
        Vec::new()
    } else {
        args.top_keys.split(',').map(str::to_owned).collect()
    };
    let sort_keys = !args.no_sort_keys;
    let ensure_ascii_flag = !args.no_ensure_ascii;

    let mut status = 0;
    for json_file in &args.filenames {
        let Ok(contents) = std::fs::read_to_string(json_file) else {
            continue;
        };
        match get_pretty_format(&contents, &args.indent, ensure_ascii_flag, sort_keys, &top_keys) {
            Err(_) => {
                println!("Input File {json_file} is not a valid JSON, consider using check-json");
                status = 1;
            }
            Ok(pretty_contents) => {
                if contents != pretty_contents {
                    if args.autofix {
                        println!("Fixing file {json_file}");
                        let _ = std::fs::write(json_file, &pretty_contents);
                    } else {
                        let diff = unified_diff(&contents, &pretty_contents, json_file);
                        std::io::stdout().write_all(diff.as_bytes()).ok();
                    }
                    status = 1;
                }
            }
        }
    }
    status
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorts_keys_and_indents() {
        let pretty = get_pretty_format(r#"{"b":1,"a":2}"#, "2", true, true, &[]).unwrap();
        assert_eq!(pretty, "{\n  \"a\": 2,\n  \"b\": 1\n}\n");
    }

    #[test]
    fn top_keys_hoisted() {
        let pretty = get_pretty_format(r#"{"b":1,"a":2,"keep":3}"#, "2", true, true, &["keep".to_owned()]).unwrap();
        assert_eq!(pretty, "{\n  \"keep\": 3,\n  \"a\": 2,\n  \"b\": 1\n}\n");
    }

    #[test]
    fn ensure_ascii_escapes_nonascii() {
        let pretty = get_pretty_format(r#"{"k":"é"}"#, "2", true, true, &[]).unwrap();
        assert_eq!(pretty, "{\n  \"k\": \"\\u00e9\"\n}\n");
    }

    #[test]
    fn custom_indent_tab() {
        let pretty = get_pretty_format(r#"{"a":1}"#, "\t", true, true, &[]).unwrap();
        assert_eq!(pretty, "{\n\t\"a\": 1\n}\n");
    }

    #[test]
    fn diff_shows_change() {
        let diff = unified_diff("{\n  \"a\": 1\n}\n", "{\n  \"a\": 2\n}\n", "f.json");
        assert!(diff.starts_with("--- f.json\n+++ f.json\n"));
        assert!(diff.contains("-  \"a\": 1"));
        assert!(diff.contains("+  \"a\": 2"));
    }
}
