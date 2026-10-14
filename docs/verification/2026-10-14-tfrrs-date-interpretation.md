# TFRRS strict date interpretation

`tfrrs/parse/date.rs` consumes a complete English day label and validates the Gregorian calendar.
The dedicated `parse/date_cell.rs` reads the whole bounded date-cell fragment rather than truncating
at the first nested div. It refuses duplicate declarations, malformed/unsupported date markup,
ambiguous ranges and trailing text. Inline text does not gain invented separators. Other column
readers are unchanged. Interpretation advances to `tfrrs_projection_v5`; physical `tfrrs_pages_v2`,
old receipts and retained captures keep their original bytes and meanings.

Limits: 65,536 input bytes checked before tokenizer creation; 8,192 tokens with exact EOF;
64 open supported date-markup elements; 128 decoded-label bytes. These are local parser limits,
not upstream page or whole-process RSS certification. Ordinary enclosing end tags outside the
cell are tolerated because the adapter passes row fragments, not full documents.

Changed tests map: `parse/date_cell_tests.rs` has eleven direct reader cases; `tests/date_provenance.rs`
checks actual-list ambiguity and unchanged non-date fields; `run_receipts_date_tests.rs` and its
fixture helper check four collector/refusal/reopen cases and exact historical physical witnesses.
Original fixture inventory is 28 rows, five relays, three gradeless individuals: 20 admitted on
valid dates, 16 after ambiguous-date rejection. Synthetic v4 receipt setup is not an old-binary run.
Previously accepted historical facts remain stored; current publication eligibility/correction
work is a separate obligation.

- PASSED: detached forward/reverse applicability, candidate byte comparison, whitespace review
  (one inherited trailing blank line removed during integration), and independent changed-surface
  static review. Five parser defects found across review passes were corrected in the candidate.
- NOT RUN: Rust formatter, compilation, all tests, parser fuzz/property execution, resource
  measurements and release checks. Cloud memory admission blocks execution. No runtime acceptance.
- PENDING: exact commands below and re-review of any resulting changed surface.

```sh
env -u CI MOON_RUST_BUILD_JOBS=1 tools/moon-local run pipeline:fmt
env -u CI MOON_RUST_BUILD_JOBS=1 tools/moon-local run pipeline:check -- --jobs 1
env -u CI MOON_RUST_BUILD_JOBS=1 tools/moon-local run pipeline:tests-scoped -- --build-jobs 1 -p census-crawl --lib -E 'test(tfrrs::)' --no-capture --no-tests fail
env -u CI MOON_RUST_BUILD_JOBS=1 tools/moon-local run pipeline:check pipeline:lint-src
env -u CI MOON_RUST_BUILD_JOBS=1 tools/moon-local run pipeline:gate -- --release
```

No live source request, provider activation, database migration or production-store change occurred.
Beads skill and functional Rust skill were located; `bd` executable is unavailable. The black-hat
response template is missing, so no claim is made that that template ran.
