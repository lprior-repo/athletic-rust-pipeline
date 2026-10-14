# Parser-envelope additive evidence decision

STATUS: REJECTED

Artifact integrity is separately audited. This status rejects unconditional acceptance because executed strict-size and module-seam gates remain failing; it does not erase the narrower passing results.

PASS: 107/107 selected focused tests (1 slow; 3195 skipped), all eight compiler API oracles, format-check, source lint, all-target compilation, post-probe clean check/format, comments on 3002 Rust files, and panic-extraction on 3002 files plus 3 templates. All 38 source files match commit `771072a1013017a4f8b4993751f801df16f789a2`; temporary probes and module registrations are removed.

FAIL: strict handwritten size, 14 functions above 60 logical lines; module seams, three installation-to-bootstrap edges and zero crate violations. Their source paths are outside the envelope delta and byte-unchanged across its commit, but these remain open repository gate failures. Required active-context truth-serum audit is unavailable and not run.

Every earlier compile failure, ineffective debug override, effective retry, interrupted comments attempt and expected negative compiler rejection is preserved. The original provisional remains unchanged and archived. No full-specification, all-historical-stores, release, census or production publication approval is claimed. Intentional Rust API tightening, persistence-neutral scope and variant-only context guard limitations remain binding.
