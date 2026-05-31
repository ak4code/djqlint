; P002 — redundant QuerySet truthiness test.
;
; Capture the condition and the enclosing `if`. The Rust handler decides whether
; the condition is a QuerySet used purely for its truthiness (and not referenced
; again in the body, which would make caching legitimate).
(if_statement
  condition: (_) @cond) @if
