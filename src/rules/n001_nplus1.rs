//! N001 — N+1 query anti-pattern.
//!
//! Detects iterating a QuerySet and then, inside the loop body, reaching across a
//! relation on the loop variable (`item.author.name`, `item.tags.all()`). Without
//! `select_related`/`prefetch_related` each iteration issues an extra query.
//!
//! The detection is deliberately conservative: it requires a *two-level* attribute
//! access rooted at the loop variable (`item.<relation>.<...>`), which in practice
//! denotes traversing into a related object rather than reading a plain column.

use std::collections::HashSet;

use tree_sitter::{Node, Query};

use crate::diagnostic::{Diagnostic, Severity};
use crate::engine::queryset::{expr_is_queryset, has_eager_loading};
use crate::engine::util::{node_text, walk_preorder};
use crate::engine::FileUnit;
use crate::rules::{capture_node, for_each_match, Rule};

const CODE: &str = "N001";
const NAME: &str = "n-plus-one";

pub struct NPlusOne {
    query: Query,
    target_idx: u32,
    iter_idx: u32,
    body_idx: u32,
}

impl NPlusOne {
    pub fn new() -> anyhow::Result<Self> {
        let query = crate::engine::parser::compile_query(CODE, include_str!("queries/n001.scm"))?;
        let capture = |name: &str| {
            query
                .capture_index_for_name(name)
                .unwrap_or_else(|| panic!("n001.scm must define a @{name} capture"))
        };
        Ok(Self {
            target_idx: capture("target"),
            iter_idx: capture("iter"),
            body_idx: capture("body"),
            query,
        })
    }
}

/// If `node` is `<base>.<a>.<b>` where `<base>` is the identifier `loop_var`,
/// return the related-access sub-node `<base>.<a>` (the cause of the extra
/// query). Returns `None` otherwise.
fn related_access<'tree>(node: Node<'tree>, loop_var: &str, source: &str) -> Option<Node<'tree>> {
    if node.kind() != "attribute" {
        return None;
    }
    // outer = inner.<attr>
    let inner = node.child_by_field_name("object")?;
    if inner.kind() != "attribute" {
        return None;
    }
    // inner = base.<relation>
    let base = inner.child_by_field_name("object")?;
    if base.kind() == "identifier" && node_text(base, source) == loop_var {
        Some(inner)
    } else {
        None
    }
}

impl Rule for NPlusOne {
    fn code(&self) -> &'static str {
        CODE
    }
    fn name(&self) -> &'static str {
        NAME
    }
    fn description(&self) -> &'static str {
        "Iterating a QuerySet and accessing related objects per row causes N+1 queries."
    }

    fn check(&self, unit: &FileUnit) -> Vec<Diagnostic> {
        let mut out = Vec::new();

        for_each_match(&self.query, unit.tree.root_node(), unit.source, |m| {
            let (Some(target), Some(iter), Some(body)) = (
                capture_node(m, self.target_idx),
                capture_node(m, self.iter_idx),
                capture_node(m, self.body_idx),
            ) else {
                return;
            };

            // Only simple `for item in qs:` loops (no tuple unpacking).
            if target.kind() != "identifier" {
                return;
            }
            let loop_var = node_text(target, unit.source);
            if loop_var == "_" {
                return;
            }

            // The iterable must be a QuerySet that hasn't been eager-loaded.
            if !expr_is_queryset(iter, unit.source, unit.bindings) {
                return;
            }
            if has_eager_loading(iter, unit.source) {
                return;
            }

            let iter_text = node_text(iter, unit.source);
            let mut seen: HashSet<usize> = HashSet::new();

            walk_preorder(body, |n| {
                if let Some(access) = related_access(n, loop_var, unit.source) {
                    // Deduplicate repeated `item.author.x` / `item.author.y`.
                    if !seen.insert(access.start_byte()) {
                        return;
                    }
                    let access_text = node_text(access, unit.source);
                    out.push(Diagnostic::from_node(
                        CODE,
                        NAME,
                        Severity::Warning,
                        access,
                        format!(
                            "`{access_text}` follows a relation inside a loop, \
                             issuing one query per iteration (N+1)"
                        ),
                        "related lookup runs every iteration",
                        Some(format!(
                            "Eager-load the relation on the QuerySet, e.g. \
                             `{iter_text}.select_related(\"…\")` for ForeignKey/OneToOne \
                             or `{iter_text}.prefetch_related(\"…\")` for ManyToMany / reverse FK."
                        )),
                    ));
                }
            });
        });

        out
    }
}
