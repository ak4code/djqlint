//! Fast, `.gitignore`-aware filesystem traversal.

use std::path::{Path, PathBuf};

use ignore::WalkBuilder;

/// Collect every `*.py` file under the given roots.
///
/// Honours `.gitignore`/`.ignore` by default (toggle with `respect_gitignore`)
/// and skips any path containing one of the user-provided exclude fragments.
pub fn collect_python_files(
    roots: &[PathBuf],
    excludes: &[String],
    respect_gitignore: bool,
) -> Vec<PathBuf> {
    if roots.is_empty() {
        return Vec::new();
    }

    let mut builder = WalkBuilder::new(&roots[0]);
    for root in &roots[1..] {
        builder.add(root);
    }
    builder
        .hidden(false)
        .git_ignore(respect_gitignore)
        .git_exclude(respect_gitignore)
        .git_global(respect_gitignore)
        .parents(respect_gitignore)
        .standard_filters(respect_gitignore);

    let mut files = Vec::new();
    for result in builder.build() {
        let entry = match result {
            Ok(e) => e,
            Err(err) => {
                tracing::warn!(%err, "skipping unreadable path");
                continue;
            }
        };

        let path = entry.path();
        if !is_python_file(path) {
            continue;
        }
        if is_excluded(path, excludes) {
            tracing::debug!(path = %path.display(), "excluded by config");
            continue;
        }
        files.push(path.to_path_buf());
    }

    files.sort();
    files.dedup();
    files
}

fn is_python_file(path: &Path) -> bool {
    path.is_file()
        && path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("py") || e.eq_ignore_ascii_case("pyi"))
            .unwrap_or(false)
}

fn is_excluded(path: &Path, excludes: &[String]) -> bool {
    if excludes.is_empty() {
        return false;
    }
    let as_str = path.to_string_lossy();
    excludes
        .iter()
        .any(|frag| !frag.is_empty() && as_str.contains(frag.as_str()))
}
