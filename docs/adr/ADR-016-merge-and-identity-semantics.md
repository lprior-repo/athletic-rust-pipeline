# ADR-016 — Merge and identity semantics: port the hygiene, keep identity lossless, count by rule

**Status:** Accepted (2026-09-29).

[ADR-015](ADR-015-port-prototype-acquisition-to-rust.md) ports acquisition. What the prototype then
*does* with the rows lives in `run.py`, `integrate_lanes.py`, `merge_enrichment.py` and
`fix_state_labels.py`, not in the parsers. The audit
([run-py-merge-semantics.md](../../research/sources/coach-directories-national/run-py-merge-semantics.md),
27 rules S01–S27 with executed case tables E1–E7) compares every one of those rules against the Rust
census path. This ADR decides, once, what the Rust census reproduces, what it deliberately
supersedes, and what it excludes — so parallel slices cannot each answer the question differently.

## Decision

1. **Row hygiene is a shared, emission-time helper.** The prototype's per-row cleaning runs in the
   merge; the Rust equivalent must run where rows are minted, because the store cannot represent the
   distinction afterwards (S14). One shared module, `crates/census-crawl/src/row_hygiene/`, owns the
   post/role tables, the placeholder rules and the vendor-row drops; every coach-emitting adapter
   calls it. Existing per-lane cleaners that already implement one of its rules delegate to it rather
   than keeping a second copy.
2. **Identity stays lossless; the census's counting rules move to the metrics layer.** The Rust
   school and coach identities keep the more precise form. Where the prototype reached a *count* by
   collapsing rows at key time, the Rust census reaches the same count by a counting rule at the
   metrics layer. No entity or persisted shape changes for that.
3. **Census scope filters are explicit and counted.** The varsity-only and junior-high rules (S12,
   S13) live at adapter emission, not in the store, and the rows they drop appear in the adapter
   report's per-level counters. Eligibility and row hygiene precede duplicate admission: a rejected
   JV or vendor row never reserves the key of a later eligible varsity/public contact.
4. **Artifact shape is a projection.** Fields the prototype's JSON needed (`sources[]`, `state` on a
   coach, `level`, row order) are export- or report-layer mappings over the Rust entities, not new
   store fields.
5. **The enrichment lanes stay out of this port.** The prototype's cross-lane enrichment
   (`fix_state_labels.py`, `merge_enrichment.py`, the MileSplit address join) is excluded: it feeds
   the census from a private workbook-era corpus, which [ADR-013](ADR-013-fresh-national-source-census.md)
   replaced with public-source discovery, and it is not a registered Rust source. School addresses in
   the Rust census come from the directory row of the source that names the school.

## Rule-by-rule disposition

Verdicts: **REPRODUCE** (the rule becomes Rust behavior), **SUPERSEDE** (Rust keeps a different rule
and the difference is intentional), **EXCLUDE** (not brought over), **EXISTS** (Rust already has it).

| Rule | Prototype behavior | Verdict | Where it lands |
|---|---|---|---|
| S01 school key | strips bare `HIGH`/`SCHOOL` anywhere, keeps `HS` | SUPERSEDE | Rust strips only trailing `hs`/`highschool`/`high school` tokens, so `Madison Junior High` stays distinct while `Madison West HS` and `Madison West High School` merge. The prototype's rule merges the first pair and splits the second. Recorded as an identity difference; the parity harness compares mapped keys. |
| S02 `normalize_person` | would upper-case person keys; never called | EXCLUDE | Dead in the prototype. Rust's case-folding is its own decision, not a port. |
| S03 `sanitize_person` | strips a trailing post (`(AD)`, `, Head Coach`, free text), drops post-only strings, non-coach leads and `Dean` posts | REPRODUCE | `row_hygiene` (name part); the E2 18-case table becomes its tests. |
| S04 vendor school drop | drops names matching `\btest\s+school\b` (42 rows measured) | REPRODUCE | `row_hygiene` (school part); the DragonFly adapter calls it. |
| S05 vendor email drop | drops coaches whose email ends `@dragonflyathletics.com` (15 rows measured) | REPRODUCE | `row_hygiene` (contact part). |
| S06 whitespace/NBSP | `[\s\u00a0]+` → single space, trimmed | REPRODUCE | `row_hygiene::clean_text`; per-lane copies delegate to it. |
| S07 school mint, first-wins | first naming source mints; later rows fill gaps only | EXISTS | `census-store::entities::canonical` (`CanonicalSchool::merge`). |
| S08 `sources[]` accumulation | every contributing source id, arrival order | SUPERSEDE | `source_identities` already records this; the artifact field is an export projection. |
| S09 empty school key | row dropped when the normalised key is empty | REPRODUCE | `row_hygiene` rejects an empty school or person name before mint. |
| S10 coach dedup key | `(school, person, sport, role)`; gender first-seen, Boys/Girls collapse | SUPERSEDE | `CoachId` keeps gender, so the entity stays lossless. The metrics layer counts distinct persons per school, sport family and role, which is the number the prototype reported. |
| S11 AD collapse | rows whose sport slot is `AthleticDirector` become one AD row per school+person | EXISTS | `CoachRole::AthleticDirector`, `Gender::Mixed`, sport `None`. The metrics layer counts `admins` by that role, not by the prototype's sport slot. |
| S12 varsity-only detail rows | keeps a detail coach only when `(level or "Varsity") == "Varsity"` | REPRODUCE | DragonFly detail/deepen emission; the level counters keep the evidence of what was dropped. |
| S13 junior-high staff pages | detail pages for junior-high staff are dropped wholesale | REPRODUCE | Same emission point: the page is skipped and counted, not fetched and discarded. |
| S14 level through merge | level filters pre-merge and is not a key component | SUPERSEDE | `CanonicalCoach` deliberately has no level; S12/S13 therefore decide at emission. |
| S15 contact first-wins | email/phone/code/source_url fill when empty | EXISTS | `CanonicalCoach::merge` with professional/personal address slots. |
| S16 sport/gender normalisation | raw strings; metrics count `Track`/`CrossCountry` | SUPERSEDE | Typed `Sport`/`Gender`; the metrics layer folds `IndoorTrack`/`OutdoorTrack` into the prototype's `Track`. |
| S17 jurisdiction label | every school and coach row carries state | SUPERSEDE | Coaches reference their school; the state is a join at projection time. |
| S18 state-label correction | relabels OSM sweep rows from `addr_state` | EXCLUDE | OSM lane not ported (decision 5). |
| S19 source precedence | declared `SOURCES` order decides contested fields | SUPERSEDE | The run plan fixes adapter order and the seal records it; there is no second, hidden priority table. Recorded as a run-plan obligation. |
| S20 ordering | deterministic given inputs; row order is not comparable | SUPERSEDE | Deterministic by construction; the parity harness compares sorted on `(school, person, sport, role)`. |
| S21 empty-state artifacts | a state with no rows writes no file but still reports a row | EXISTS | Adapter reports plus the service's coverage aggregate. |
| S22 per-source counters | status/bytes/school/coach/address/city/zip/vacancy | EXISTS | `AdapterReport`; the prototype's `vacancy`/`school_sample` counters are the metric names to re-check when the seal's report is written. |
| S23 MaxPreps row hygiene | rejects empty/`Coach`/`NULL`/digit/single-word/over-40-char names | EXCLUDE | No MaxPreps source is registered; if one is ever admitted, its hygiene goes through `row_hygiene` with these rules as tests. |
| S24 lane container join | wraps lane rows in `{url,state,school,status,title_school,name_match,coaches}` | EXCLUDE | Loader plumbing for a JSON the Rust census does not produce; the equivalent provenance is `PROVENANCE.json` plus `source_identities`. |
| S25 per-state metrics | schools, with_address, with_city, with_zip, tf_xc_coaches, head_coaches, admins | REPRODUCE | Metric definitions are pinned in the counting layer (S10/S11/S16 apply); the parity harness compares these numbers, not row counts. |
| S26 MileSplit school enrichment | address + indoor/outdoor/XC flags from `schools_enriched.jsonl`, joined by name | EXCLUDE | Decision 5. |
| S27 PSS + OSM enrichment | address/website/lat/lon/ppin from `extra/Private-schools-enriched.jsonl` | EXCLUDE | Decision 5. |

## Consequences

- Parallel slices have one contract: hygiene from `row_hygiene`, identity from the domain types,
  census-scope filters at emission with counters, counts from the metrics layer.
- The prototype's merged JSON is not a row-for-row oracle. Parity is asserted three ways: parser
  fixtures (unchanged bodies), the metrics named in S25, and the hygiene tables ported as tests.
- The port must not add `level`, `state` or `sources[]` to persisted entities to chase the old JSON;
  the export layer projects them.
- Deleting the prototype's merge code requires `row_hygiene` to exist with the S03/S04/S05/S06/S09
  rules and the S12/S13 emission filters in place (ADR-015's "ported and evidenced" test).
- Open obligation recorded, not silently deferred: the run plan must make adapter order explicit
  (S19) and the seal must record it, because contested fields resolve by first write.
