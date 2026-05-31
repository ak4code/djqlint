; P001 — in-memory counting / materialisation.
;
; Match any call whose callee is a bare builtin identifier (`len`, `list`, ...).
; The Rust handler narrows this to len()/list() and inspects the single argument
; to decide whether it is a QuerySet. Capturing broadly here keeps the
; S-expression resilient to Python formatting (newlines, trailing commas, etc.).
(call
  function: (identifier) @builtin
  arguments: (argument_list) @args) @call
