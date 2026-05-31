//! P002 — Redundant QuerySet truthiness check.
//!
//! `if queryset:` evaluates and caches the *entire* result set just to find out
//! whether any rows exist. When the QuerySet is not used again inside the block,
//! `if queryset.exists():` is dramatically cheaper (`SELECT 1 ... LIMIT 1`).

use tree_sitter::{Node, Query};

use crate::diagnostic::{Diagnostic, Severity};
use crate::engine::queryset::expr_is_queryset;
use crate::engine::util::{node_text, walk_preorder};
use crate::engine::FileUnit;
use crate::rules::{capture_node, for_each_match, Rule};

const CODE: &str = "P002";
const NAME: &str = "redundant-truthiness";

pub struct RedundantTruthiness {
    query: Query,
    cond_idx: u32,
    if_idx: u32,
}

impl RedundantTruthiness {
    pub fn new() -> anyhow::Result<Self> {
        let query = crate::engine::parser::compile_query(CODE, include_str!("queries/p002.scm"))?;
        let cond_idx = query
            .capture_index_for_name("cond")
            .expect("p002.scm must define a @cond capture");
        let if_idx = query
            .capture_index_for_name("if")
            .expect("p002.scm must define an @if capture");
        Ok(Self {
            query,
            cond_idx,
            if_idx,
        })
    }
}

/// Is `name` referenced anywhere inside `block`?
fn identifier_used_in(block: Node, name: &str, source: &str) -> bool {
    let mut used = false;
    walk_preorder(block, |n| {
        if n.kind() == "identifier" && node_text(n, source) == name {
            used = true;
        }
    });
    used
}

impl Rule for RedundantTruthiness {
    fn code(&self) -> &'static str {
        CODE
    }
    fn name(&self) -> &'static str {
        NAME
    }
    fn description(&self) -> &'static str {
        "Testing a QuerySet for truthiness loads every row; use .exists() instead."
    }

    fn check(&self, unit: &FileUnit) -> Vec<Diagnostic> {
        let mut out = Vec::new();

        for_each_match(&self.query, unit.tree.root_node(), unit.source, |m| {
            let (Some(cond), Some(if_node)) =
                (capture_node(m, self.cond_idx), capture_node(m, self.if_idx))
            else {
                return;
            };

            // The condition itself must be a bare QuerySet expression. Anything
            // already terminating in `.exists()`/`.count()` (or a boolean op,
            // comparison, `not`, ...) is not a QuerySet and is skipped.
            if !expr_is_queryset(cond, unit.source, unit.bindings) {
                return;
            }

            // If the condition is a simple variable that is *reused* in the body
            // the QuerySet cache is actually being exploited — don't flag it.
            if cond.kind() == "identifier" {
                let name = node_text(cond, unit.source);
                if let Some(body) = if_node.child_by_field_name("consequence") {
                    if identifier_used_in(body, name, unit.source) {
                        return;
                    }
                }
            }

            let qs_text = node_text(cond, unit.source);
            out.push(Diagnostic::from_node(
                CODE,
                NAME,
                Severity::Warning,
                cond,
                format!("`if {qs_text}:` evaluates the full QuerySet just to test for rows"),
                "truthiness test loads every row",
                Some(format!(
                    "Use `if {qs_text}.exists():` — it issues a cheap `SELECT 1 … LIMIT 1` \
                     instead of fetching and caching the whole result set."
                )),
            ));
        });

        out
    }
}
