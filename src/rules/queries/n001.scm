; N001 — N+1 query detection.
;
; Capture every `for <target> in <iter>:` loop. The Rust handler verifies that
; <iter> is a QuerySet without eager loading, then scans the body for related
; attribute traversals (`item.fk.field`, `item.m2m.all()`) on the loop variable.
(for_statement
  left: (_) @target
  right: (_) @iter
  body: (block) @body) @for
