//! Rule engine: the [`Rule`] trait, shared query helpers, and the registry that
//! instantiates the built-in rules (honouring the `disabled` config list).

mod n001_nplus1;
mod p001_in_memory_count;
mod p002_redundant_truthiness;

use anyhow::Result;
use streaming_iterator::StreamingIterator;
use tree_sitter::{Node, Query, QueryCursor, QueryMatch};

use crate::config::Config;
use crate::diagnostic::Diagnostic;
use crate::engine::FileUnit;

/// A single lint rule. Rules are constructed once (compiling their tree-sitter
/// query up front) and then shared, immutably, across all worker threads — hence
/// the `Send + Sync` bound.
pub trait Rule: Send + Sync {
    /// Stable identifier, e.g. `"P001"`.
    fn code(&self) -> &'static str;
    /// Short machine-friendly name, e.g. `"in-memory-counting"`.
    fn name(&self) -> &'static str;
    /// One-line human description (used in SARIF rule metadata).
    fn description(&self) -> &'static str;
    /// Analyse one file and return any findings.
    fn check(&self, unit: &FileUnit) -> Vec<Diagnostic>;
}

/// Build the active rule set, dropping anything listed in `config.rules.disabled`.
pub fn build_rules(config: &Config) -> Result<Vec<Box<dyn Rule>>> {
    let all: Vec<Box<dyn Rule>> = vec![
        Box::new(n001_nplus1::NPlusOne::new()?),
        Box::new(p001_in_memory_count::InMemoryCount::new()?),
        Box::new(p002_redundant_truthiness::RedundantTruthiness::new()?),
    ];

    let disabled: Vec<String> = config
        .rules
        .disabled
        .iter()
        .map(|c| c.to_uppercase())
        .collect();

    Ok(all
        .into_iter()
        .filter(|r| !disabled.iter().any(|d| d == r.code()))
        .collect())
}

/// Return the metadata of every built-in rule (for `--list-rules` / SARIF).
pub fn all_rule_metadata() -> Vec<(&'static str, &'static str, &'static str)> {
    build_rules(&Config::default())
        .map(|rules| {
            rules
                .iter()
                .map(|r| (r.code(), r.name(), r.description()))
                .collect()
        })
        .unwrap_or_default()
}

// --- shared query helpers -------------------------------------------------

/// Run `query` over `root` and invoke `f` for every match. Wraps the
/// `StreamingIterator` API introduced in tree-sitter 0.24.
pub(crate) fn for_each_match<F>(query: &Query, root: Node, source: &str, mut f: F)
where
    F: FnMut(&QueryMatch<'_, '_>),
{
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(query, root, source.as_bytes());
    while let Some(m) = matches.next() {
        f(m);
    }
}

/// Fetch the node bound to capture `index` within a match, if present.
pub(crate) fn capture_node<'tree>(m: &QueryMatch<'_, 'tree>, index: u32) -> Option<Node<'tree>> {
    m.captures.iter().find(|c| c.index == index).map(|c| c.node)
}
