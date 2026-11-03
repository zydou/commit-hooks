//! `debug-statements` — detect debugger imports and `breakpoint()` calls.
//!
//! Flags `import <debugger>` / `from <debugger> import ...` for the well-known
//! debuggers, plus any `breakpoint()` call, reporting each as
//! `file:line:col: <name> imported|called`.

use clap::Parser;
use ruff_python_ast::visitor::Visitor;
use ruff_python_ast::{Expr, Stmt};
use ruff_python_parser::parse_module;
use ruff_text_size::Ranged;
use std::collections::HashSet;
use crate::util::line_col;

#[derive(Parser)]
#[command(about = "Checks for debugger imports and py37+ breakpoint() calls.")]
struct Args {
    filenames: Vec<String>,
}

/// Module names that always indicate a debugger.
const DEBUG_STATEMENTS: &[&str] = &[
    "bpdb",
    "ipdb",
    "pdb",
    "pdbr",
    "pudb",
    "pydevd_pycharm",
    "q",
    "rdb",
    "rpdb",
    "wdb",
];

/// One detected debug statement.
struct Debug {
    line: usize,
    col: usize,
    name: String,
    reason: &'static str,
}

struct DebugStatementParser<'a> {
    source: &'a str,
    breakpoints: Vec<Debug>,
    debug_statements: HashSet<&'a str>,
}

impl Visitor<'_> for DebugStatementParser<'_> {
    fn visit_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            // `import pdb` / `import os, pdb` — flag each alias in the set.
            Stmt::Import(import) => {
                for alias in &import.names {
                    let name = alias.name.id.as_str();
                    if self.debug_statements.contains(name) {
                        let (line, col) = line_col(self.source, stmt.range().start().to_usize());
                        self.breakpoints.push(Debug {
                            line,
                            col,
                            name: name.to_owned(),
                            reason: "imported",
                        });
                    }
                }
            }
            // `from pdb import set_trace` — flag the module.
            Stmt::ImportFrom(import) => {
                if let Some(module) = &import.module {
                    let name = module.id.as_str();
                    if self.debug_statements.contains(name) {
                        let (line, col) = line_col(self.source, stmt.range().start().to_usize());
                        self.breakpoints.push(Debug {
                            line,
                            col,
                            name: name.to_owned(),
                            reason: "imported",
                        });
                    }
                }
            }
            _ => {}
        }
        ruff_python_ast::visitor::walk_stmt(self, stmt);
    }

    fn visit_expr(&mut self, expr: &Expr) {
        // `breakpoint()` (Python 3.7+).
        if let Expr::Call(call) = expr {
            if let Expr::Name(name) = call.func.as_ref() {
                if name.id.as_str() == "breakpoint" {
                    let (line, col) = line_col(self.source, call.range().start().to_usize());
                    self.breakpoints.push(Debug {
                        line,
                        col,
                        name: name.id.as_str().to_owned(),
                        reason: "called",
                    });
                }
            }
        }
        ruff_python_ast::visitor::walk_expr(self, expr);
    }
}

fn check_file(filename: &str) -> i32 {
    let Ok(source) = std::fs::read_to_string(filename) else {
        return 0;
    };
    let parsed = match parse_module(&source) {
        Ok(parsed) => parsed,
        Err(_) => {
            println!("{filename} - Could not parse ast");
            println!();
            return 1;
        }
    };

    let mut visitor = DebugStatementParser {
        source: &source,
        breakpoints: Vec::new(),
        debug_statements: DEBUG_STATEMENTS.iter().copied().collect(),
    };
    visitor.visit_body(&parsed.syntax().body);

    for bp in &visitor.breakpoints {
        println!("{filename}:{0}:{1}: {2} {3}", bp.line, bp.col, bp.name, bp.reason);
    }
    i32::from(!visitor.breakpoints.is_empty())
}

pub fn run(args: &[String]) -> i32 {
    let args = Args::parse_from(std::iter::once("debug-statements".to_owned()).chain(args.iter().cloned()));

    let mut retv = 0;
    for filename in &args.filenames {
        retv |= check_file(filename);
    }
    retv
}

#[cfg(test)]
mod tests {
    use super::*;

    fn findings(contents: &str) -> Vec<String> {
        let path = crate::util::write_temp_file(contents.as_bytes());
        let name = path.to_string_lossy().into_owned();
        // Drive the visitor directly to capture the formatted findings.
        let parsed = parse_module(contents).unwrap();
        let mut visitor = DebugStatementParser {
            source: contents,
            breakpoints: Vec::new(),
            debug_statements: DEBUG_STATEMENTS.iter().copied().collect(),
        };
        visitor.visit_body(&parsed.syntax().body);
        visitor
            .breakpoints
            .iter()
            .map(|bp| format!("{name}:{0}:{1}: {2} {3}", bp.line, bp.col, bp.name, bp.reason))
            .collect()
    }

    #[test]
    fn detects_pdb_import() {
        let f = findings("import pdb\n");
        assert_eq!(f.len(), 1);
        assert!(f[0].ends_with(": pdb imported"));
    }

    #[test]
    fn detects_breakpoint_call() {
        let f = findings("x = 1\nbreakpoint()\n");
        assert_eq!(f.len(), 1);
        assert!(f[0].ends_with(": breakpoint called"));
        // breakpoint() is on line 2, column 0.
        assert!(f[0].contains(":2:0:"));
    }

    #[test]
    fn detects_import_from() {
        let f = findings("from pdb import set_trace\n");
        assert_eq!(f.len(), 1);
        assert!(f[0].ends_with(": pdb imported"));
    }

    #[test]
    fn ignores_non_debug_imports() {
        assert!(findings("import os, sys\nfrom a import b\n").is_empty());
    }

    #[test]
    fn column_is_byte_based() {
        // Python's col_offset counts UTF-8 bytes, so the multi-byte 'é' shifts the column.
        let f = findings("é = 'x'; breakpoint()\n");
        assert_eq!(f.len(), 1);
        // 'breakpoint' starts at byte index 10 on line 1.
        assert!(f[0].contains(":1:10:"));
    }
}
