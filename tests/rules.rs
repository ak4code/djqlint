//! End-to-end rule tests driven through the public binary-equivalent path: we
//! reach into the engine the same way `main` does, so these exercise parsing,
//! the data-flow model, rule matching and suppression together.

use std::io::Write;

use djqlint::config::Config;
use djqlint::engine;
use djqlint::rules;

/// Analyse a Python snippet written to a temp file and return the codes found,
/// in source order.
fn codes_for(src: &str) -> Vec<String> {
    let mut file = tempfile::Builder::new()
        .suffix(".py")
        .tempfile()
        .expect("temp file");
    file.write_all(src.as_bytes()).unwrap();
    file.flush().unwrap();

    let rule_set = rules::build_rules(&Config::default()).expect("rules");
    match engine::analyze_file(file.path(), &rule_set) {
        Some(report) => report
            .diagnostics
            .iter()
            .map(|d| d.code.to_string())
            .collect(),
        None => Vec::new(),
    }
}

#[test]
fn p001_flags_len_on_queryset_variable() {
    let codes = codes_for("qs = User.objects.filter(active=True)\nn = len(qs)\n");
    assert_eq!(codes, vec!["P001"]);
}

#[test]
fn p001_flags_list_on_manager_chain() {
    let codes = codes_for("rows = list(Book.objects.all())\n");
    assert_eq!(codes, vec!["P001"]);
}

#[test]
fn p001_ignores_len_on_plain_list() {
    // Not a QuerySet -> no finding.
    let codes = codes_for("items = [1, 2, 3]\nn = len(items)\n");
    assert!(codes.is_empty(), "got {codes:?}");
}

#[test]
fn p002_flags_bare_truthiness() {
    let codes = codes_for("qs = User.objects.filter(active=True)\nif qs:\n    notify()\n");
    assert_eq!(codes, vec!["P002"]);
}

#[test]
fn p002_skips_when_queryset_reused_in_body() {
    let src = "qs = User.objects.all()\nif qs:\n    for u in qs:\n        print(u)\n";
    // qs is reused inside the body, so caching via truthiness is legitimate.
    // (The reuse loop is itself fine because there is no related-field access.)
    let codes = codes_for(src);
    assert!(!codes.contains(&"P002".to_string()), "got {codes:?}");
}

#[test]
fn p002_skips_explicit_exists() {
    let codes = codes_for("if User.objects.filter(active=True).exists():\n    pass\n");
    assert!(codes.is_empty(), "got {codes:?}");
}

#[test]
fn n001_flags_related_access_in_loop() {
    let src = "for b in Book.objects.all():\n    print(b.author.name)\n";
    assert_eq!(codes_for(src), vec!["N001"]);
}

#[test]
fn n001_skips_select_related() {
    let src = "for b in Book.objects.select_related(\"author\"):\n    print(b.author.name)\n";
    assert!(codes_for(src).is_empty());
}

#[test]
fn n001_ignores_plain_column_access() {
    // Single-level attribute is a column read, not a relation traversal.
    let src = "for b in Book.objects.all():\n    print(b.title)\n";
    assert!(codes_for(src).is_empty());
}

#[test]
fn noqa_suppresses_specific_code() {
    let codes = codes_for("n = len(User.objects.all())  # noqa: djqlint-P001\n");
    assert!(codes.is_empty(), "got {codes:?}");
}

#[test]
fn djqlint_disable_suppresses_all() {
    let codes = codes_for("n = len(User.objects.all())  # djqlint: disable\n");
    assert!(codes.is_empty(), "got {codes:?}");
}

#[test]
fn malformed_python_does_not_panic() {
    // Deliberately broken syntax: must return gracefully (possibly no findings).
    let _ = codes_for("def broken(:\n  x = User.objects.\n  if for in\n");
}
