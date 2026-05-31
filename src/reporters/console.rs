//! Human-facing console reporter built on `miette`'s graphical handler.
//!
//! Produces compiler-style output: the offending line is printed with an arrow
//! pointing at the exact call, plus a `help:` remediation hint.

use std::fmt;

use miette::{
    Diagnostic as MietteDiagnostic, GraphicalReportHandler, LabeledSpan, NamedSource,
    Severity as MietteSeverity, SourceCode,
};

use crate::diagnostic::{Diagnostic, FileReport};

/// Adapter turning one of our [`Diagnostic`]s into a fully-fledged
/// `miette::Diagnostic`. Implemented by hand (rather than via derive) so the
/// rule code and severity can be set dynamically at runtime.
struct PrettyDiagnostic {
    code: &'static str,
    severity: MietteSeverity,
    message: String,
    label: String,
    help: Option<String>,
    src: NamedSource<String>,
    offset: usize,
    len: usize,
}

impl fmt::Debug for PrettyDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl fmt::Display for PrettyDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for PrettyDiagnostic {}

impl MietteDiagnostic for PrettyDiagnostic {
    fn code(&self) -> Option<Box<dyn fmt::Display + '_>> {
        Some(Box::new(format!("djqlint::{}", self.code)))
    }

    fn severity(&self) -> Option<MietteSeverity> {
        Some(self.severity)
    }

    fn help(&self) -> Option<Box<dyn fmt::Display + '_>> {
        self.help
            .as_ref()
            .map(|h| Box::new(h.clone()) as Box<dyn fmt::Display>)
    }

    fn source_code(&self) -> Option<&dyn SourceCode> {
        Some(&self.src)
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = LabeledSpan> + '_>> {
        let span = LabeledSpan::at(self.offset..self.offset + self.len, self.label.clone());
        Some(Box::new(std::iter::once(span)))
    }
}

fn to_pretty(file: &FileReport, diag: &Diagnostic) -> PrettyDiagnostic {
    PrettyDiagnostic {
        code: diag.code,
        severity: diag.severity.to_miette(),
        message: diag.message.clone(),
        label: diag.label.clone(),
        help: diag.help.clone(),
        src: NamedSource::new(file.path.to_string_lossy(), file.source.clone())
            .with_language("Python"),
        offset: diag.start_byte,
        len: diag.span_len(),
    }
}

/// Render every finding into `out`.
pub fn render(reports: &[FileReport], out: &mut String) -> fmt::Result {
    use fmt::Write;
    let handler = GraphicalReportHandler::new();
    for file in reports {
        for diag in &file.diagnostics {
            let pretty = to_pretty(file, diag);
            handler.render_report(out, &pretty)?;
            out.write_char('\n')?;
        }
    }
    Ok(())
}
