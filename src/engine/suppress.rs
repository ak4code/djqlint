//! Inline suppression directives.
//!
//! Supported on the same physical line as the finding:
//!   * `# noqa`                         — suppress every rule on this line
//!   * `# noqa: djqlint-P001`           — suppress only P001 (comma separated)
//!   * `# noqa: P001, N001`             — bare codes also accepted
//!   * `# djqlint: disable`             — suppress every djqlint rule
//!   * `# djqlint: disable=P001,P002`   — suppress specific codes

use std::collections::{HashMap, HashSet};

use once_cell::sync::Lazy;
use regex::Regex;

static NOQA: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)#\s*noqa(?::\s*(?P<codes>[A-Za-z0-9_,\-\s]+))?").unwrap());

static DISABLE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)#\s*djqlint\s*:\s*disable(?:\s*=\s*(?P<codes>[A-Za-z0-9_,\-\s]+))?").unwrap()
});

/// Per-line suppression state.
#[derive(Debug, Clone)]
enum LineRule {
    /// Suppress everything on the line.
    All,
    /// Suppress only these (normalised, e.g. `P001`) codes.
    Codes(HashSet<String>),
}

/// Map of 1-based line number -> suppression directive.
#[derive(Debug, Default)]
pub struct Suppressions {
    lines: HashMap<usize, LineRule>,
}

/// Normalise `djqlint-P001` / `P001` / ` p001 ` to canonical `P001`.
fn normalise_code(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let bare = trimmed
        .strip_prefix("djqlint-")
        .or_else(|| trimmed.strip_prefix("djqlint:"))
        .unwrap_or(trimmed);
    Some(bare.trim().to_uppercase())
}

fn parse_codes(codes: &str) -> HashSet<String> {
    codes.split([',', ' ']).filter_map(normalise_code).collect()
}

impl Suppressions {
    /// Scan raw source text line by line for suppression comments.
    pub fn parse(source: &str) -> Self {
        let mut lines = HashMap::new();

        for (idx, line) in source.lines().enumerate() {
            let line_no = idx + 1;

            // `# djqlint: disable[=codes]` takes precedence for our own codes.
            if let Some(caps) = DISABLE.captures(line) {
                match caps.name("codes") {
                    Some(c) => {
                        Self::merge(
                            &mut lines,
                            line_no,
                            LineRule::Codes(parse_codes(c.as_str())),
                        );
                    }
                    None => {
                        lines.insert(line_no, LineRule::All);
                        continue;
                    }
                }
            }

            if let Some(caps) = NOQA.captures(line) {
                match caps.name("codes") {
                    Some(c) => {
                        Self::merge(
                            &mut lines,
                            line_no,
                            LineRule::Codes(parse_codes(c.as_str())),
                        );
                    }
                    None => {
                        lines.insert(line_no, LineRule::All);
                    }
                }
            }
        }

        Suppressions { lines }
    }

    fn merge(lines: &mut HashMap<usize, LineRule>, line_no: usize, rule: LineRule) {
        match lines.get_mut(&line_no) {
            Some(LineRule::All) => {} // already suppresses everything
            Some(LineRule::Codes(existing)) => {
                if let LineRule::Codes(new) = rule {
                    existing.extend(new);
                } else {
                    lines.insert(line_no, LineRule::All);
                }
            }
            None => {
                lines.insert(line_no, rule);
            }
        }
    }

    /// Should a finding for `code` on `line` (1-based) be suppressed?
    pub fn is_suppressed(&self, line: usize, code: &str) -> bool {
        match self.lines.get(&line) {
            Some(LineRule::All) => true,
            Some(LineRule::Codes(codes)) => codes.contains(&code.to_uppercase()),
            None => false,
        }
    }
}
