//! Thin, panic-free wrapper around `tree-sitter` for Python.

use anyhow::{Context, Result};
use tree_sitter::{Language, Parser, Query, Tree};

/// The Python grammar handle. Cheap to clone (it is reference-counted in the C
/// runtime), so we materialise it on demand rather than holding global state.
pub fn python_language() -> Language {
    tree_sitter_python::LANGUAGE.into()
}

/// Parse Python source into a syntax tree.
///
/// tree-sitter is error-recovering: malformed input still yields a tree with
/// `ERROR`/`MISSING` nodes rather than failing, so this only returns `None` if
/// the grammar itself cannot be loaded — which never happens at runtime.
pub fn parse(source: &str) -> Option<Tree> {
    let mut parser = Parser::new();
    parser.set_language(&python_language()).ok()?;
    parser.parse(source, None)
}

/// Compile a tree-sitter query against the Python grammar, attaching the rule
/// code to any compilation error for easier debugging when authoring queries.
pub fn compile_query(code: &str, source: &str) -> Result<Query> {
    Query::new(&python_language(), source)
        .with_context(|| format!("failed to compile tree-sitter query for rule {code}"))
}
