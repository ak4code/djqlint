//! Stable JSON report — convenient for custom dashboards and scripting.

use serde::Serialize;

use crate::diagnostic::FileReport;

use super::Summary;

#[derive(Serialize)]
struct JsonReport<'a> {
    tool: &'static str,
    version: &'static str,
    summary: Summary,
    results: &'a [FileReport],
}

pub fn render(reports: &[FileReport]) -> serde_json::Result<String> {
    let report = JsonReport {
        tool: "djqlint",
        version: env!("CARGO_PKG_VERSION"),
        summary: Summary::from_reports(reports),
        results: reports,
    };
    serde_json::to_string_pretty(&report)
}
