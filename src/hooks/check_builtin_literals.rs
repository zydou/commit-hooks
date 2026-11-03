//! `check-builtin-literals` — require literal syntax for builtin type calls.
//!
//! `list()`, `dict()`, `str()`, etc. should be written as `[]`, `{}`, `''`.
//! A `dict()` call with keyword arguments (`dict(a=1)`) is allowed by default
//! since it cannot be written as a plain literal.

use clap::Parser;
use ruff_python_ast::visitor::Visitor;
use ruff_python_ast::{Expr, Stmt};
use ruff_python_parser::parse_module;
use ruff_text_size::Ranged;
use std::collections::HashSet;
use crate::util::line_col;

#[derive(Parser)]
#[command(about = "Checks for builtin types that can be written as literals.")]
struct Args {
    /// Comma-separated builtin names to ignore.
    #[arg(long, default_value = "", value_delimiter = ',')]
    ignore: Vec<String>,

    /// Allow `dict(**kwargs)` calls (this is already the default; accepted for
    /// compatibility, it has no effect beyond cancelling `--no-allow-dict-kwargs`).
    #[arg(long = "allow-dict-kwargs")]
    _allow_dict_kwargs: bool,

    /// Disallow `dict(**kwargs)` calls.
    #[arg(long = "no-allow-dict-kwargs")]
    no_allow_dict_kwargs: bool,

    filenames: Vec<String>,
}

/// The literal each builtin call should be replaced with.
const BUILTIN_TYPES: &[(&str, &str)] = &[
    ("complex", "0j"),
    ("dict", "{}"),
    ("float", "0.0"),
    ("int", "0"),
    ("list", "[]"),
    ("str", "''"),
    ("tuple", "()"),
];

/// One flagged builtin-type call.
struct Call {
    name: String,
    line: usize,
    column: usize,
}

struct VisitorImpl<'a> {
    source: &'a str,
    builtin_type_calls: Vec<Call>,
    allow_dict_kwargs: bool,
    disallowed: HashSet<&'a str>,
}

impl Visitor<'_> for VisitorImpl<'_> {
    fn visit_expr(&mut self, expr: &Expr) {
        if let Expr::Call(call) = expr {
            // Ignore attribute calls (`foo.list()`); only bare names count.
            if let Expr::Name(func) = call.func.as_ref() {
                let name = func.id.as_str();
                if self.disallowed.contains(name)
                    // `dict(a=1)` is only allowed when kwargs are permitted.
                    && (name != "dict" || !self.allow_dict_kwargs || call.arguments.keywords.is_empty())
                    && call.arguments.args.is_empty()
                {
                    let (line, column) = line_col(self.source, func.range().start().to_usize());
                    self.builtin_type_calls.push(Call {
                        name: name.to_owned(),
                        line,
                        column,
                    });
                }
            }
        }
        ruff_python_ast::visitor::walk_expr(self, expr);
    }

    fn visit_stmt(&mut self, stmt: &Stmt) {
        ruff_python_ast::visitor::walk_stmt(self, stmt);
    }
}

fn check_file(filename: &str, ignore: &HashSet<String>, allow_dict_kwargs: bool) -> (Vec<Call>, bool) {
    let Ok(source) = std::fs::read_to_string(filename) else {
        return (Vec::new(), false);
    };
    let Ok(parsed) = parse_module(&source) else {
        return (Vec::new(), false);
    };

    // `ignore` is matched against the builtin name; only names in BUILTIN_TYPES
    // are ever flagged, so the disallowed set is the builtin names minus ignore.
    let disallowed: HashSet<&str> = BUILTIN_TYPES
        .iter()
        .map(|(n, _)| *n)
        .filter(|n| !ignore.contains(*n))
        .collect();

    let mut visitor = VisitorImpl {
        source: &source,
        builtin_type_calls: Vec::new(),
        allow_dict_kwargs,
        disallowed,
    };
    visitor.visit_body(&parsed.syntax().body);
    (visitor.builtin_type_calls, true)
}

pub fn run(args: &[String]) -> i32 {
    let args = Args::parse_from(std::iter::once("check-builtin-literals".to_owned()).chain(args.iter().cloned()));

    let ignore: HashSet<String> = args.ignore.into_iter().collect();
    // `dict(**kwargs)` is allowed unless explicitly disabled; `--allow-dict-kwargs`
    // just re-asserts that default, matching upstream's `set_defaults` behaviour.
    let allow_dict_kwargs = !args.no_allow_dict_kwargs;

    let mut rc = 0;
    for filename in &args.filenames {
        let (calls, _) = check_file(filename, &ignore, allow_dict_kwargs);
        if !calls.is_empty() {
            rc = 1;
        }
        for call in &calls {
            let literal = BUILTIN_TYPES
                .iter()
                .find(|(n, _)| *n == call.name)
                .map(|(_, l)| *l)
                .unwrap_or("");
            println!("{filename}:{0}:{1}: replace {2}() with {literal}", call.line, call.column, call.name);
        }
    }
    rc
}

#[cfg(test)]
mod tests {
    use super::*;

    fn findings(contents: &str, ignore: &[&str], allow_dict_kwargs: bool) -> Vec<String> {
        let path = crate::util::write_temp_file(contents.as_bytes());
        let name = path.to_string_lossy().into_owned();
        let ignore_set: HashSet<String> = ignore.iter().map(|s| s.to_string()).collect();
        let (calls, _) = check_file(&name, &ignore_set, allow_dict_kwargs);
        calls
            .iter()
            .map(|c| format!("{}:{}:{}", c.name, c.line, c.column))
            .collect()
    }

    #[test]
    fn flags_simple_builtin_calls() {
        let f = findings("a = list()\nb = dict()\nc = str()\n", &[], false);
        assert_eq!(f, vec!["list:1:4", "dict:2:4", "str:3:4"]);
    }

    #[test]
    fn allows_kwargs_dict_by_default() {
        // dict(x=1) has kwargs -> allowed; dict() has none -> flagged.
        let f = findings("dict(x=1)\ndict()\n", &[], true);
        assert_eq!(f, vec!["dict:2:0"]);
    }

    #[test]
    fn no_allow_dict_kwargs_flags_kwargs() {
        let f = findings("dict(x=1)\n", &[], false);
        assert_eq!(f, vec!["dict:1:0"]);
    }

    #[test]
    fn ignore_skips_builtin() {
        let f = findings("list()\ndict()\n", &["list"], false);
        assert_eq!(f, vec!["dict:2:0"]);
    }

    #[test]
    fn attribute_and_arg_calls_ignored() {
        assert!(findings("foo.list()\nlist([1])\n", &[], false).is_empty());
    }

    #[test]
    fn column_is_byte_based() {
        // 'é' is two bytes, so 'list' starts at byte 10.
        let f = findings("é = 'x'; list()\n", &[], false);
        assert_eq!(f, vec!["list:1:10"]);
    }
}
