//! Core diagnostic types shared by the engine, rules and reporters.

use std::path::PathBuf;

use serde::Serialize;
use tree_sitter::Node;

/// All findings for a single source file, plus the source text needed by the
/// console reporter to render snippets. Files with no findings are dropped by
/// the engine, so an empty `diagnostics` vector should not normally appear.
#[derive(Debug, Clone, Serialize)]
pub struct FileReport {
    pub path: PathBuf,
    #[serde(skip)]
    pub source: String,
    pub diagnostics: Vec<Diagnostic>,
}

/// Severity of a finding. Maps onto SARIF `level` and `miette::Severity`.
///
/// `Error`/`Note` are part of the public severity vocabulary that future rules
/// and config overrides can emit, even though the current built-ins only use
/// `Warning`.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Note,
}

impl Severity {
    /// SARIF 2.1.0 `level` string.
    pub fn sarif_level(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Note => "note",
        }
    }

    pub fn to_miette(self) -> miette::Severity {
        match self {
            Severity::Error => miette::Severity::Error,
            Severity::Warning => miette::Severity::Warning,
            Severity::Note => miette::Severity::Advice,
        }
    }
}

/// A single finding produced by a rule.
///
/// Byte offsets are kept for span-accurate rendering (miette / SARIF), while the
/// 1-based line/column pair is convenient for human-readable output and JSON.
#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    /// Stable rule identifier, e.g. `"P001"`.
    pub code: &'static str,
    /// Human readable rule name, e.g. `"in-memory-counting"`.
    pub rule_name: &'static str,
    pub severity: Severity,
    /// Primary message shown to the user.
    pub message: String,
    /// Short text rendered on the underline arrow.
    pub label: String,
    /// Optional remediation advice.
    pub help: Option<String>,

    // --- span ---
    pub start_byte: usize,
    pub end_byte: usize,
    /// 1-based line of the span start.
    pub line: usize,
    /// 1-based column of the span start.
    pub column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

impl Diagnostic {
    /// Build a diagnostic whose span is taken verbatim from `node`.
    pub fn from_node(
        code: &'static str,
        rule_name: &'static str,
        severity: Severity,
        node: Node,
        message: impl Into<String>,
        label: impl Into<String>,
        help: Option<String>,
    ) -> Self {
        let start = node.start_position();
        let end = node.end_position();
        Diagnostic {
            code,
            rule_name,
            severity,
            message: message.into(),
            label: label.into(),
            help,
            start_byte: node.start_byte(),
            end_byte: node.end_byte(),
            line: start.row + 1,
            column: start.column + 1,
            end_line: end.row + 1,
            end_column: end.column + 1,
        }
    }

    /// Byte length of the highlighted span (>= 1 so miette always draws an arrow).
    pub fn span_len(&self) -> usize {
        self.end_byte.saturating_sub(self.start_byte).max(1)
    }
}
