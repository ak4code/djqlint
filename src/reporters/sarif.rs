//! SARIF 2.1.0 reporter.
//!
//! Static Analysis Results Interchange Format — the lingua franca consumed by
//! GitHub Advanced Security ("Code scanning alerts"), SonarQube, Azure DevOps and
//! most IDEs. Emitting valid SARIF is what makes DjQlint a drop-in CI citizen:
//!
//! ```yaml
//! - run: djqlint --format sarif -o djqlint.sarif .
//! - uses: github/codeql-action/upload-sarif@v3
//!   with: { sarif_file: djqlint.sarif }
//! ```

use serde_json::{json, Value};

use crate::diagnostic::FileReport;
use crate::rules;

const SARIF_VERSION: &str = "2.1.0";
const SARIF_SCHEMA: &str =
    "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json";

/// Build the `tool.driver.rules` array from the static rule registry.
fn rule_descriptors() -> Vec<Value> {
    rules::all_rule_metadata()
        .into_iter()
        .map(|(code, name, description)| {
            json!({
                "id": code,
                "name": name,
                "shortDescription": { "text": description },
                "fullDescription": { "text": description },
                "defaultConfiguration": { "level": "warning" },
                "helpUri": format!("https://github.com/ak4code/djqlint#{}", code.to_lowercase()),
            })
        })
        .collect()
}

/// SARIF `region` uses 1-based lines and columns; column end is exclusive.
fn result_value(file: &FileReport) -> Vec<Value> {
    let uri = file.path.to_string_lossy().replace('\\', "/");
    file.diagnostics
        .iter()
        .map(|d| {
            json!({
                "ruleId": d.code,
                "level": d.severity.sarif_level(),
                "message": { "text": d.message },
                "locations": [{
                    "physicalLocation": {
                        "artifactLocation": { "uri": uri },
                        "region": {
                            "startLine": d.line,
                            "startColumn": d.column,
                            "endLine": d.end_line,
                            "endColumn": d.end_column,
                        }
                    }
                }]
            })
        })
        .collect()
}

pub fn render(reports: &[FileReport]) -> serde_json::Result<String> {
    let results: Vec<Value> = reports.iter().flat_map(result_value).collect();

    let sarif = json!({
        "version": SARIF_VERSION,
        "$schema": SARIF_SCHEMA,
        "runs": [{
            "tool": {
                "driver": {
                    "name": "DjQlint",
                    "informationUri": "https://github.com/ak4code/djqlint",
                    "version": env!("CARGO_PKG_VERSION"),
                    "rules": rule_descriptors(),
                }
            },
            "results": results,
        }]
    });

    serde_json::to_string_pretty(&sarif)
}
