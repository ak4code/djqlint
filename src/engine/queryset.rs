//! Minimal, conservative data-flow model for Django QuerySets.
//!
//! This is intentionally *heuristic* — a full type inference engine for dynamic
//! Python is out of scope. Instead we recognise the syntactic shapes that, in
//! real Django code, almost always denote a `QuerySet`/`Manager`:
//!
//!   * `Model.objects` / `Model.objects.filter(...)` (a manager-rooted chain),
//!   * a call to a known QuerySet-returning method on something that is itself a
//!     QuerySet (`qs.filter(...)`, `qs.select_related("a")`, ...),
//!   * a local variable previously bound to one of the above.
//!
//! The variable tracking gives us the "basic data-flow" the spec asks for: if
//! `qs = User.objects.filter(...)` appears anywhere in the module, later uses of
//! the bare name `qs` are treated as a QuerySet.

use std::collections::HashSet;

use tree_sitter::Node;

use super::util::node_text;

/// QuerySet-returning methods. A chain that *ends* in one of these, rooted in a
/// manager or another QuerySet, is itself a QuerySet.
pub const QS_METHODS: &[&str] = &[
    "all",
    "filter",
    "exclude",
    "annotate",
    "alias",
    "select_related",
    "prefetch_related",
    "order_by",
    "reverse",
    "distinct",
    "values",
    "values_list",
    "only",
    "defer",
    "none",
    "union",
    "intersection",
    "difference",
    "extra",
    "using",
    "select_for_update",
    "get_queryset",
];

/// Set of local variable names known to hold a QuerySet within a module.
#[derive(Debug, Default, Clone)]
pub struct Bindings {
    queryset_vars: HashSet<String>,
}

impl Bindings {
    pub fn is_queryset_var(&self, name: &str) -> bool {
        self.queryset_vars.contains(name)
    }

    fn insert(&mut self, name: &str) -> bool {
        self.queryset_vars.insert(name.to_string())
    }
}

/// Does the expression rooted at `node` evaluate to a QuerySet/Manager?
pub fn expr_is_queryset(node: Node, source: &str, bindings: &Bindings) -> bool {
    match node.kind() {
        "identifier" => bindings.is_queryset_var(node_text(node, source)),

        // Strip wrappers.
        "parenthesized_expression" => node
            .named_child(0)
            .map(|c| expr_is_queryset(c, source, bindings))
            .unwrap_or(false),

        // `something(...)` — a QuerySet iff the callee is a QuerySet method chain.
        "call" => node
            .child_by_field_name("function")
            .map(|f| expr_is_queryset(f, source, bindings))
            .unwrap_or(false),

        // `obj.attr`
        "attribute" => {
            let attr = match node.child_by_field_name("attribute") {
                Some(a) => node_text(a, source),
                None => return false,
            };
            let object = match node.child_by_field_name("object") {
                Some(o) => o,
                None => return false,
            };

            // `Model.objects` is the canonical manager entry point.
            if attr == "objects" {
                return true;
            }
            // `<queryset>.filter(...)` etc. keeps the QuerySet-ness.
            if QS_METHODS.contains(&attr) {
                return expr_is_queryset(object, source, bindings);
            }
            false
        }

        // `qs[a:b]` slicing returns a QuerySet (when not indexing a single item).
        "subscript" => node
            .child_by_field_name("value")
            .map(|v| expr_is_queryset(v, source, bindings))
            .unwrap_or(false),

        _ => false,
    }
}

/// Walk the whole tree collecting `name = <queryset-expr>` bindings. Runs to a
/// fixpoint so transitive bindings (`qs2 = qs.filter(...)`) are also captured.
pub fn collect_bindings(root: Node, source: &str) -> Bindings {
    // Gather candidate `(name, value_node)` assignments once.
    let mut assignments: Vec<(String, Node)> = Vec::new();
    super::util::walk_preorder(root, |n| {
        if n.kind() != "assignment" {
            return;
        }
        let (Some(left), Some(right)) = (
            n.child_by_field_name("left"),
            n.child_by_field_name("right"),
        ) else {
            return;
        };
        if left.kind() == "identifier" {
            assignments.push((node_text(left, source).to_string(), right));
        }
    });

    let mut bindings = Bindings::default();
    // Fixpoint: keep re-evaluating until no new variable is discovered. The
    // number of iterations is bounded by the number of assignments, but in
    // practice converges in 1–2 passes.
    loop {
        let mut changed = false;
        for (name, value) in &assignments {
            if bindings.is_queryset_var(name) {
                continue;
            }
            if expr_is_queryset(*value, source, &bindings) {
                changed |= bindings.insert(name);
            }
        }
        if !changed {
            break;
        }
    }
    bindings
}

/// Returns true if a chain's text contains an eager-loading hint
/// (`select_related` / `prefetch_related`), used to suppress N+1 reports.
pub fn has_eager_loading(node: Node, source: &str) -> bool {
    let text = node_text(node, source);
    text.contains("select_related") || text.contains("prefetch_related")
}
