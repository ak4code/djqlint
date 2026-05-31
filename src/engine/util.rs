//! Low-level AST helpers used across the engine and rules.

use tree_sitter::Node;

/// Return the source slice covered by `node`.
#[inline]
pub fn node_text<'a>(node: Node, source: &'a str) -> &'a str {
    // `byte_range` is always within bounds for nodes produced from `source`.
    source.get(node.byte_range()).unwrap_or("")
}

/// Depth-first pre-order traversal that invokes `f` for every descendant
/// (including `root`). Uses an explicit `TreeCursor` so we never recurse on the
/// Rust stack — important for deeply nested generated Python.
///
/// The explicit `'tree` lifetime lets callers store the yielded nodes (they are
/// borrowed from the tree, not from the transient cursor).
pub fn walk_preorder<'tree, F: FnMut(Node<'tree>)>(root: Node<'tree>, mut f: F) {
    let mut cursor = root.walk();
    'descend: loop {
        f(cursor.node());
        if cursor.goto_first_child() {
            continue;
        }
        // No children: advance to the next sibling, climbing back up as needed.
        loop {
            if cursor.goto_next_sibling() {
                continue 'descend;
            }
            if !cursor.goto_parent() {
                return;
            }
        }
    }
}

/// Named children of `node`, skipping comments.
pub fn significant_named_children<'tree>(node: Node<'tree>) -> Vec<Node<'tree>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .filter(|c| c.kind() != "comment")
        .collect()
}
