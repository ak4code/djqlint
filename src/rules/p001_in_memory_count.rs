//! P001 — In-memory counting / materialisation.
//!
//! Flags `len(queryset)` and `list(queryset)` where the QuerySet is only being
//! used to count rows. Both pull every row into Python memory; `.count()` (or
//! `.exists()`) pushes the work to the database.

use tree_sitter::Query;

use crate::diagnostic::{Diagnostic, Severity};
use crate::engine::queryset::expr_is_queryset;
use crate::engine::util::{node_text, significant_named_children};
use crate::engine::FileUnit;
use crate::rules::{capture_node, for_each_match, Rule};

const CODE: &str = "P001";
const NAME: &str = "in-memory-counting";

/// Builtins that fully evaluate their iterable argument.
const MATERIALISING_BUILTINS: &[&str] = &["len", "list"];

pub struct InMemoryCount {
    query: Query,
    call_idx: u32,
}

impl InMemoryCount {
    pub fn new() -> anyhow::Result<Self> {
        let query = crate::engine::parser::compile_query(CODE, include_str!("queries/p001.scm"))?;
        let call_idx = query
            .capture_index_for_name("call")
            .expect("p001.scm must define a @call capture");
        Ok(Self { query, call_idx })
    }
}

impl Rule for InMemoryCount {
    fn code(&self) -> &'static str {
        CODE
    }
    fn name(&self) -> &'static str {
        NAME
    }
    fn description(&self) -> &'static str {
        "Counting a QuerySet with len()/list() evaluates every row in Python; use .count() or .exists()."
    }

    fn check(&self, unit: &FileUnit) -> Vec<Diagnostic> {
        let mut out = Vec::new();

        for_each_match(&self.query, unit.tree.root_node(), unit.source, |m| {
            let Some(call) = capture_node(m, self.call_idx) else {
                return;
            };

            // Identify the builtin being called.
            let Some(func) = call.child_by_field_name("function") else {
                return;
            };
            let builtin = node_text(func, unit.source);
            if !MATERIALISING_BUILTINS.contains(&builtin) {
                return;
            }

            // Must be exactly one positional argument and it must be a QuerySet.
            let Some(args) = call.child_by_field_name("arguments") else {
                return;
            };
            let arguments = significant_named_children(args);
            if arguments.len() != 1 {
                return;
            }
            let arg = arguments[0];
            if !expr_is_queryset(arg, unit.source, unit.bindings) {
                return;
            }

            let qs_text = node_text(arg, unit.source);
            let (message, help) = match builtin {
                "len" => (
                    format!("`len({qs_text})` loads the whole QuerySet into memory to count rows"),
                    format!(
                        "Use `{qs_text}.count()` so the database performs the count, \
                             or `{qs_text}.exists()` if you only need to know whether rows exist."
                    ),
                ),
                _ => (
                    format!("`list({qs_text})` materialises the entire QuerySet in memory"),
                    format!(
                        "If you only need the row count use `{qs_text}.count()`; \
                             otherwise iterate the QuerySet lazily instead of building a list."
                    ),
                ),
            };

            out.push(Diagnostic::from_node(
                CODE,
                NAME,
                Severity::Warning,
                call,
                message,
                "evaluates the entire QuerySet here",
                Some(help),
            ));
        });

        out
    }
}
