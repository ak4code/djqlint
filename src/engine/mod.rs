//! The analysis engine: parsing, the per-file context passed to rules, and the
//! multi-threaded driver that fans work out across all discovered files.

pub mod parser;
pub mod queryset;
pub mod suppress;
pub mod util;
pub mod walker;

use std::fs;
use std::path::{Path, PathBuf};

use rayon::prelude::*;
use tree_sitter::Tree;

use crate::diagnostic::FileReport;
use crate::rules::Rule;
use queryset::Bindings;
use suppress::Suppressions;

/// Everything a rule needs to analyse one file. Borrowed, so passing it to each
/// rule is free and rules never own the (potentially large) source buffer.
pub struct FileUnit<'a> {
    /// Path of the file under analysis. Exposed for path-aware rules; not used
    /// by the current built-ins.
    #[allow(dead_code)]
    pub path: &'a Path,
    pub source: &'a str,
    pub tree: &'a Tree,
    pub bindings: &'a Bindings,
}

/// Analyse every file in parallel and return one [`FileReport`] per file that
/// produced at least one (non-suppressed) finding.
pub fn analyze_files(files: &[PathBuf], rules: &[Box<dyn Rule>]) -> Vec<FileReport> {
    files
        .par_iter()
        .filter_map(|path| analyze_file(path, rules))
        .collect()
}

/// Analyse a single file. Returns `None` when the file cannot be read as UTF-8
/// or contains no findings. Never panics on malformed Python.
pub fn analyze_file(path: &Path, rules: &[Box<dyn Rule>]) -> Option<FileReport> {
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(err) => {
            tracing::debug!(path = %path.display(), %err, "skipping unreadable/non-utf8 file");
            return None;
        }
    };

    let tree = parser::parse(&source)?;
    let bindings = queryset::collect_bindings(tree.root_node(), &source);
    let suppressions = Suppressions::parse(&source);

    let unit = FileUnit {
        path,
        source: &source,
        tree: &tree,
        bindings: &bindings,
    };

    let mut diagnostics = Vec::new();
    for rule in rules {
        for diag in rule.check(&unit) {
            if suppressions.is_suppressed(diag.line, diag.code) {
                continue;
            }
            diagnostics.push(diag);
        }
    }

    if diagnostics.is_empty() {
        return None;
    }

    diagnostics.sort_by_key(|d| (d.line, d.column, d.code));
    Some(FileReport {
        path: path.to_path_buf(),
        source,
        diagnostics,
    })
}
