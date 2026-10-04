# ADR-021 — The school-address join is one tested core, one durable stage

**Status:** Accepted (2026-10-04).

## Context

[ADR-020](ADR-020-school-address-corpus-port.md) ported the school-directory pipeline to Rust and
left the census store untouched on purpose: `census-service school-address` reads operator-supplied
directory artifacts into one verified generation (the real captures produce 122 692 entries: 100 307
CCD plus 22 385 PSS) but writes no postal claim. Canonical schools therefore had no postal addresses,
the workbook's postal block stayed empty, and nothing in the national workflow consumed the corpus.

The join those requirements imply is small and deterministic: canonical schools are matched against
the generation's collapsed entries, and a match becomes an owned claim on the school. The hazards
are elsewhere — inventing capture evidence, linking across a state boundary, choosing one of two
equally plausible schools, duplicating claims on retry, and running the join outside the durable
workflow that owns the run. Each of those has one correct answer and one home.

## Decision

1. **One pure core owns the join.** `census_service::school_address` exposes
   `process(store, index, lanes, mode)` plus the composition `join_generation(store, generation_dir,
   out_dir, overrides, mode)`. `process` reads the store snapshot, matches with the domain's
   `DirectoryIndex`/`LinkDecision`, and appends. It has no clock, no network and no hidden state: the
   generation supplies each lane's path, digest and generation digest, and the caller supplies the
   observation date. `Mode::DryRun` computes and reports without appending.
2. **Evidence is required, never fabricated.** A claim is written only when its lane carries a real
   capture URL and an ISO observation date (`Overrides`, validated: providers are exactly
   `nces-ccd`/`nces-pss`, URLs are http(s), dates are `YYYY-MM-DD` or RFC 3339). An otherwise
   linkable school whose lane lacks either is reported as `evidence_missing` and stays unlinked. The
   claim's `capture_sha256` is the lane capture digest and its evidence note records
   `provider lane <path> sha256=<digest> generation <manifest digest>`; the digest chain is therefore
   byte-equal with the corpus manifest and with the workbook's postal cells.
3. **One durable service is the apply path in a run.** `SchoolAddressJoin` is a Restate workflow
   (apply-once invocation, three attempts, pause on exhaustion, 90/180/30-day retention) whose
   handler is the only in-process caller. The national workflow invokes it under the run identity's
   key (`<identity>:school-address-join`) after the jurisdiction census and consolidation, before
   workbook publication; the request carries the generation path and the url/date overrides, and the
   stage stays visible in the workflow journal instead of becoming an unlogged side effect.
4. **The offline lane is explicit.** `census-service school-address-join --store <root>
   --generation <root> [--apply] [--evidence-url S=URL] [--evidence-date S=DATE] [--out DIR]` opens a
   stopped owner directly, mirrors the service's semantics, and writes `report.json` and
   `outcomes.jsonl` under `<store>/out/school-address-join` (or `--out`). Without `--apply` it is a
   dry run. It never borrows the corpus verb's assumption that no store is open.
5. **Ambiguity and impossibility are reported, not resolved by guessing.** Two candidates that tie on
   the matcher's preference produce `review` with both candidates recorded; a name that matches only
   in another state never links; a school without a state is `missing_state`; an unmatched school is
   `no_match`. Every outcome is one JSONL row carrying the school, the rule or reason, and the
   candidate labels.
6. **Claims are ordinary owned observations.** Each claim is a `SchoolPostalAddress` owned by
   `school_directory:<provider>:<state>` with the provider's school id, so it appears in the
   workbook's existing postal block (street, second line, city, state, ZIP, owner namespace/id,
   source, URL, observed date, capture SHA) and is validated by the existing workbook verifier
   against the store. No new publication path and no new workbook column family were added.
7. **The matched entry's website is attached once and never overwritten.** When the CCD entry that
   matched carries a parsed website and the school publishes none, the join sets
   `CanonicalSchool.school_website` and pushes the same lane evidence note the postal claim carries
   (`<provider> lane <path> sha256=<digest> generation <digest>`), counting it as `websites`. A
   school that already publishes a website is left untouched, and a `WEBSITE` cell that is not an
   http(s) URL is a corpus note (`website is not a supported value`), never a guessed URL. The
   workbook's existing `School site` column renders the field, so publication gained no column
   family and no second path.

## Consequences

- ADR-020's boundary is unchanged for the corpus verb — it still opens no store. The join is the
  component that opens one, and it obeys the one-owner rule: the CLI requires `--store` and a stopped
  owner, the service owns the `--data-dir` store. A second opener still fails on the lock, which
  remains correct.
- Replay is idempotent by construction: a claim already present for the same school, lane digest and
  evidence is reported as `already_linked` and nothing is appended, and a website the school already
  publishes is not rewritten. A second apply over the postal generation after a full service restart
  reported `linked=0, already_linked=158`; the later apply over the website-carrying generation
  reported `linked=0, already_linked=158, websites=93`, and a dry run at the final revision reported
  `websites=0` — the "already attached" signature — with the same `review=1, no_match=48`.
- A private or association-only school can legitimately stay unmatched: the public directory corpus
  carries public and private-school-universe entries, not association rosters. `no_match` is a
  measurement, not a defect; the review lane and the corpus's own notes are the repair path.
- The stage is bounded by the corpus, not the store: matching is a single pass over canonical schools
  against a built index, 207 schools in the dated smoke, and the 122 692-entry index is built once
  per invocation.

## Implementation status

Landed with this ADR: the core and CLI lane, the `SchoolAddressJoin` workflow and its wire contract,
the national stage call, the CCD `WEBSITE` mapping in the NCES reader with the corpus's `website`
column, and the workbook postal block and `School site` column consumed by the existing verifier.
Evidence: `cargo test -p census-service --lib --bins` (222 + 71 passed; nine of the lib tests are the
join core's own lane plus outcome, evidence, idempotency, dry-run, ambiguity, cross-state and
override cases), `-p census-domain` (270, including the matcher's link/refusal lane and the postal
types), `-p census-crawl` (873, including the CCD/PSS reader windows and the mapped website), `-p
census-report` (195), and the integration targets `nces_directory_properties` (8),
`school_address_corpus` (8), `school_address_publication` (3) and `workbook_shape` (1). `cargo
clippy -p census-domain -p census-service -p census-crawl -p census-report --all-targets` is clean
and `cargo fmt --check` passes for the touched crates. The dated native smoke in
[VERIFICATION-EVIDENCE.md](../VERIFICATION-EVIDENCE.md) carries real captures → two corpus
generations → `SchoolAddressJoin` via ingress → two workbooks read back → verifier OK, followed by
restart re-applies and a final-revision dry run.
