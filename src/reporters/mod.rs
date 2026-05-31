//! Output reporters: console (miette), JSON, and SARIF 2.1.0.

pub mod console;
pub mod json;
pub mod sarif;

use std::collections::BTreeMap;

use serde::Serialize;

use crate::diagnostic::FileReport;

/// Aggregate counts across all analysed files.
#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    pub files_with_findings: usize,
    pub total_findings: usize,
    /// Findings grouped by rule code (sorted).
    pub by_code: BTreeMap<String, usize>,
}

impl Summary {
    pub fn from_reports(reports: &[FileReport]) -> Self {
        let mut by_code: BTreeMap<String, usize> = BTreeMap::new();
        let mut total = 0;
        for file in reports {
            for diag in &file.diagnostics {
                total += 1;
                *by_code.entry(diag.code.to_string()).or_insert(0) += 1;
            }
        }
        Summary {
            files_with_findings: reports.len(),
            total_findings: total,
            by_code,
        }
    }
}
