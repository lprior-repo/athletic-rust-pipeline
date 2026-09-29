# DragonFly school-summary parse parity — 2026-09-29

Comparison of the prototype's school-summary lane against the Rust lane over the same captured
bodies, executed on both sides.

* **Reference:** `census-prototype/parsers/dragonfly_school.py:parse`, then the extras lane's
  varsity filter (`run.py:190-194`) and `run.merge_state` (`run.py:255-313`) applied to a single
  detail record — i.e. exactly the path that produced the prototype's `coaches` rows for a school.
* **Rust:** `coach_directories::parse_summary`, `absorb_summary` and
  `coach_entities(…, EmissionScope::Census)`.
* **Inputs:** all 18 captured summaries under `crates/census-crawl/tests/fixtures/coach_directories`
  (16 probe page-1 summaries for AK/AL/GA/WY, plus the NC and IN staff summaries).

## Result

| Measure | Prototype | Rust | Difference |
|---|---|---|---|
| Captured summaries | 18 | 18 | — |
| Published coach rows | 76 | 61 | — |
| Sub-varsity rows dropped by level | 15 (`Middle School` 9, `All Teams` 5, `JV` 1) | 15 (same labels) | none |
| Rows kept | 48 | 48 after the S10 gender collapse | none |
| Fields compared | person, sport, role, gender, email, phone, code | same | none (0 divergences) |
| School name | 18 | 18 | none |
| School city | 18 | 17 identical, 1 whitespace-only | `probe/AL/summary-SVXJDF` |

The prototype's merge key excludes gender (S10), so its 48 kept rows are the Rust 61 rows
collapsed by `(person, sport, role)` first-wins. With that collapse applied the row *sets* are
identical; the comparison is on sets because row order is not artifact-comparable (S20).

The harness caught one real defect in the port, which is the reason this comparison exists:
the Rust lane originally filtered by level **before** de-duplicating, so a school that lists its
junior-varsity team before its varsity team kept the varsity row. The prototype's parser claims
the key on the first published row and filters afterwards, so the junior-varsity row is the one
that is dropped and the varsity row never replaces it. Affected fixture:
`probe/WY/summary-SS28UB.json` (boys cross country lists JV before Varsity; the prototype keeps 2
rows, the port kept 3). The port now claims the key on the raw row before hygiene and the level
filter, and `coach_directories::tests::a_live_coach_row_is_decided_by_the_first_team_that_publishes_the_key`
pins the exact row set, the `JV` count and the total.

## Retained artifacts

* `golden_summary_rows.json` — the prototype's merged rows for all 18 fixtures, produced by running
  `parsers/dragonfly_school.py:parse` + the varsity filter + `run.merge_state`, keyed by
  fixture-relative path, carrying `school` (name, city), `school_unslotted` (the fields Rust has no
  slot for), `coaches` and `dropped_counts`.
* `PROVENANCE.json` — 18 appended entries with the summary URL, prototype cache file name, sha256
  and byte count; every fixture resolved to a live prototype cache capture.
* `coach_directories::tests::the_live_summary_pages_reproduce_the_prototypes_rows` — asserts the
  golden row set, the varsity drop counts, and school name/city for all 18 fixtures (48 rows, 15
  drops). The test projects the Rust rows into the prototype's vocabulary; the projection lives in
  the test, not in production code, because it is an artifact-shape mapping, not a domain rule.

## Field mapping

| Prototype row | Rust |
|---|---|
| `person` | `CanonicalCoach::name` (both post-hygiene) |
| `sport` `Track`/`CrossCountry`/`AthleticDirector` | `Sport::IndoorTrack`/`OutdoorTrack`/`CrossCountry`, `None` for a director |
| `role` `Coach`/`HeadCoach`/`AssistantCoach`/`AthleticDirector` | `CoachRole::Unknown`/`HeadCoach`/`AssistantCoach`/`AthleticDirector` |
| `gender` `Boys`/`Girls`/`""` | `Gender::Boys`/`Girls`/`Mixed` |
| `email` ← `emails[0]` | `professional_email` first, else `personal_email` |
| `phone` ← `tel[0].num` | `CanonicalCoach::phone` |
| `code` ← `amrId` | `SourceIdentity` id whose prefix is the `amrId` |
| `level` | not stored; drives the emission-time varsity filter |
| school `name`, `city` | `CanonicalSchool::name`, `city` |
| school `address`, `zip`, `phone`, `association_id` | **no slot** — see divergence 5 and 6 |

## Divergences recorded

1. **Gender participates in Rust coach identity (S10).** Deliberate supersede: the prototype's
   merge key drops gender and keeps the first-seen value. Rust keeps one row per gender; the
   metrics projection must collapse as the prototype did (48 rows). Measured: 13 extra Rust rows
   over these captures, all gender-split pairs.
2. **`Sport::IndoorTrack`/`OutdoorTrack` vs `"Track"` (S16).** Rust keeps the venue; the prototype
   collapses the family at parse time. The parity projection maps both to `Track`.
3. **`CoachRole::Unknown` vs the literal `"Coach"`.** Raw `role` in the source is usually absent;
   the prototype defaults the *string* to `Coach` while Rust records "no role stated". The metrics
   layer must render `Unknown` as the prototype's `Coach` if artifact strings are compared.
4. **`code` is a composite in Rust.** `SourceIdentity::id` is `amrId:sport-family:role:gender`, so
   two rows of one person (e.g. cross country and track) stay distinguishable; the prototype stores
   the bare `amrId`, which repeats across those rows. The `amrId` is recoverable as the prefix
   (0 prefix mismatches over 48 rows).
5. **School address, ZIP and phone are parsed but not stored.** `SummaryAddress::address1`/`zip` and
   `SummaryTel::num` reach the lane and are asserted by the golden test, but `CanonicalSchool` has
   no slot for them at this revision; the prototype's school row carries all three (plus
   `with_address`/`with_zip` in its per-state summary, S25). Blocked on the model change that owns
   those fields.
6. **`association_id` (platform org id, `payload.id`) is not stored on the school.** Rust keys the
   school's `association_school` identity on the directory row's short code (S05 lane); the
   summary's org id only builds detail URLs. The prototype's school row carries both.
7. **City whitespace.** `probe/AL/summary-SVXJDF` publishes `"Rainbow City "`; the prototype keeps the
   trailing space in the artifact (it cleans `name`/`person`/contacts but not other school fields,
   S06), Rust trims. Recorded, not reconciled: trimming is the intended behaviour on the Rust side.

### Found by the black-hat parity review (2026-09-29)

8. **Name parts are trimmed before the claim key.** `map::person_name` trims `firstName`/`lastName` and
   drops empties, where `dragonfly_school.py:101-102` joins the parts untrimmed, so a padded name would key
   differently (and change Probe-scope counts). No captured summary pads a name; recorded, not reconciled.
9. **Probe scope keeps a nameless pass-1 row** *(correction)*: `dragonfly_probe.py:56-59` counts every row
   `parse` returns, including one whose first and last name are both empty, while `push_row` dropped it in
   both scopes. The drop is now census-only and pinned by
   `tests::a_nameless_team_member_counts_for_the_probe_and_is_dropped_from_the_census`.
10. **One genderless prefix** *(correction)*: the prototype breaks after the first `Unified `/`Mixed `
    strip, so `"Unified Mixed Track, Outdoor"` has no sport there; Rust stripped both. Fixed with the break
    and pinned in `tests::the_possessive_and_genderless_labels_all_map`.
11. **Counter order** *(correction)*: the prototype's level filter runs at merge before name hygiene, so a
    sub-varsity row with a post-only name or a vendor address is a *level* drop, while Rust counted it as
    `dropped_person`/`dropped_vendor`. The level scope now runs first; the goldens are unchanged because no
    capture holds such a row.
12. **Absent `totalPages`/`totalResults` render `0`, not `null`** (probe payload). `DirectoryPage` defaults
    both to `0` and `probe_one` always writes them, where `dragonfly_probe.py:33-36` uses `payload.get()`,
    i.e. `None` → JSON `null` / printed `None`. The live API always sends both keys; the evidence recorded
    only the present-but-null variant (Rust's `Option` cannot distinguish absent from null).

## What is not established

Parsing and row mapping only — no prototype `out/` artifact was reproduced, and fetching, caching,
pacing, the store write and the metrics projection were not exercised. The comparison covers 18
captured summaries (48 kept rows); the national lane covers all 15 registered associations, so the
golden is a sample, not a census-scale equivalence proof.
