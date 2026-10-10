# state_ed (New York State Education Department school directory) — Source Report

**Date:** 2026-10-10 (read-only verification; captures not re-fetched)
**Source ID:** `state_ed`
**Authority:** New York State Education Department (NYSED), `data.nysed.gov`
**Disposition:** artifact_or_extract → captured fixtures, offline file reader

This report satisfies the strict path rule `research/sources/<slug>/SOURCE_REPORT.md` for the
`state_ed` slug. It publishes the two captures already committed under
`crates/census-crawl/tests/fixtures/state_ed/` plus the reader contract they exercise.
It is evidence + spec only; no production code was changed for this report.

## 1. Source identity

- Provider (registry): New York State Education Department school directory
  (`data.nysed.gov`) — `crates/census-crawl/src/registry/table/directories.rs:15-19`.
- Transport: `Html`; capabilities: `SCHOOL_ADDRESS`; admission: `artifact()` (local-artifact
  file reader, makes no network request).
- Applicability: New York only — `crates/census-crawl/src/applicability/table/data.rs:227-230`
  (`jurisdictions: &[UsJurisdiction::NewYork]`), evidence
  `crates/census-crawl/src/applicability/table/prose.rs:235-238` (`STATE_ED_EVIDENCE`),
  refusal `prose.rs:235-238` (`STATE_ED_REFUSAL`).
- Reader owner files: `crates/census-crawl/src/state_ed/{mod,fields,parse,tabular}.rs` and the
  scaffold `crates/census-crawl/src/state_ed/README.md`. Fixture provenance:
  `crates/census-crawl/tests/fixtures/state_ed/SOURCE.md`; fixture conventions:
  `crates/census-crawl/tests/fixtures/state_ed/README.md`.

## 2. Captures

Byte-exact responses served by `data.nysed.gov`, copied with `cp` from
`census-prototype/raw/data.nysed.gov__<sha1-of-url>`. Nothing was reformatted or re-serialized
(`crates/census-crawl/tests/fixtures/state_ed/SOURCE.md:3-5`).

| Fixture | Corpus file | URL | Status | Bytes | sha256 |
|---|---|---|---|---|---|
| `index_letter_a.html` | `data.nysed.gov__58a5886b064eaca830473668` | `https://data.nysed.gov/analysis/` — the letter-A school index, inferred from the page's own title and links (no sidecar recorded the request path) | 200 | 786,227 | `adf44bb69c0c8751459c698162fe7526950a6e4feb1778a07d4f3c6c9aa37f27` |
| `profile_kingston.html` | `data.nysed.gov__5facfe61d423d8f0828d0bed` | `https://data.nysed.gov/profile.php?instid=800000038718` | 200 | 60,020 | `6d720faec71b6dbf30eddb8045262e78daf62931d805a2a501f9eefa12442830` |

What the captures pin (`SOURCE.md:12-25`):

- `index_letter_a.html` — `<title>A - Schools | NYSED Data Site</title>`. Letter-A slice of the
  state school index, not the whole index: **2,530** literal `instid=` occurrences, of which
  **2,528** carry a numeric value, resolving to **220 distinct school ids** (all twelve digits)
  and **220 distinct `profile.php?instid=<id>` links** (220 link occurrences). The two
  non-numeric markers are JavaScript's `instid=some_value` example and an `instid=` string
  concatenation; they are not schools. (A prior 221-school/link claim double-counted
  `800000054526`.) School names + institution ids come from the index; postal address components
  require the separately captured profile.
- `profile_kingston.html` — `<title>A A KINGSTON MIDDLE SCHOOL | NYSED Data Site</title>`, with
  the page's own `AT A GLANCE 2024-25` section. Served from
  `profile.php?instid=800000038718`: a profile is addressed by the 12-digit NYSED institution id
  the index supplies.

## 3. Reader contract (which parse entry points consume these captures)

| Capture | Parser | Input shape | Identity yielded |
|---|---|---|---|
| `index_letter_a.html` | `state_ed::parse_index` (`parse.rs:56-68`, walked `parse.rs:70-107`, row `parse.rs:109-140`; re-export `mod.rs:7`) | NYSED index HTML matching `INDEX_ROW` (`fields.rs:1-2`) | One `SchoolDirectoryEntry::identified(IdentifiedKey::StateRecord { state: NY, id }, SourceLabel::StateEducationAgency { state: NY }, Some(name))` per accepted row (`parse.rs:138-139`); duplicate-id skip (`parse.rs:126-132`) |
| `profile_kingston.html` | `state_ed::parse_profile` (`parse.rs:142-163`; identity `parse.rs:177-210`; details `parse/details.rs`; re-export `mod.rs:7`) | NYSED profile HTML matching `PROFILE_TITLE` (`fields.rs:4`) and `INSTITUTION_ID` (`fields.rs:6`), plus `PHONE`, `WEBSITE`, `MAPS_QUERY`, `TOTAL_STUDENTS` (`fields.rs:8-14`) | Same `IdentifiedKey::StateRecord { state: NY, id }` identity + name (`parse.rs:208-209`), enriched with phone/website/address (percent-decoded `maps/embed…q=`) /enrollment from the details pass |

Enforcement around both paths:

- Body ≤ 8 MiB (`parse.rs:165-175`); both captures (786,227 and 60,020 bytes) are inside budget.
- Index stops at 20,000 rows (`parse.rs:95-104`); zero-match index is
  `DirectoryError::Representation` ("NYSED index has no published school rows", `parse.rs:77-87`).
- Title-less or id-less page is refused as a directory artifact ("not a school profile",
  `parse.rs:182-192`).
- `state_ed::parse_tabular` (`tabular.rs:156-163`, `TABULAR_REQUIRED = ["NAME","CITY","STATE"]`,
  `tabular.rs:9`, re-export `mod.rs:8`) is **CSV-only and yields `WeakKey` entries
  (`tabular.rs:39-47`)**; these two HTML captures do not exercise it.

Replay boundary (per `xtask/README.md:308-312,322-329`): `replay` consumes committed fixture
bytes offline through published parse paths and establishes those captures only — it does not
fetch, open a store, or consult the live source clock. `source-fixture state_ed` lists the
fixture directory; `source-test state_ed` selects test function names containing `state_ed`
(underscore spelling).

## 4. Applicability claim

- `STATE_ED_EVIDENCE` (`prose.rs:239-242`): New York's state-agency directory is the one
  captured — the A-Z anchor index (786,227 bytes, sha256 `adf44bb6…`, 2,528
  `profile.php?instid=` link occurrences) plus profile `instid=800000038718` (60,020 bytes)
  carrying the address the index lacks; reader is an artifact reader
  (`admission.origin = local-artifact`).
- `STATE_ED_REFUSAL` (`prose.rs:244-248`): only New York has a state-agency capture; the
  prototype's CA/TX/NY/FL/PA URL guesses returned nothing and no other state artifact exists in
  the corpus. The tabular index page is parsed with the same reader as the association
  directories; only per-profile pages are state-agency specific. NJ (prototype's sole live URL)
  returned nothing, so NJ is not planned. The reader accepts any operator-supplied file.

## 5. Limits

- Read-only verification on 2026-10-06 (bead) / 2026-10-10 (this report): byte counts and
  sha256 values above were checked against the committed fixtures with `stat`/`sha256sum` only;
  the captures themselves were **not re-fetched** from `data.nysed.gov`.
- No robots verdict for `data.nysed.gov` was captured in the corpus. The reader is a file reader
  and makes no request; a fetching lane would need its own admission decision, recorded in the
  registry descriptor (`SOURCE.md:26-28`; scaffold checklist
  `crates/census-crawl/src/state_ed/README.md:36-42` still open on real URLs + robots status).
- The index capture's request path (`https://data.nysed.gov/analysis/`) is inferred from the
  page's own title and links — no sidecar recorded it (`SOURCE.md:9`). The applicability prose
  cites the anchor as `data.nysed.gov/lists.php?type=school`; that alias vs. the inferred
  `/analysis/` path is unresolved (see Open questions).
- Capture dates were not recorded by a sidecar. Related NYSED HTML survey evidence (portal root
  liveness, `?instid=` shape, 2025 enrollment cycle) lives in the hyphen-slug report
  `research/sources/state-ed/SOURCE_REPORT.md:48-51` (2026-10-09), which is a **different slug**
  and must not be confused with this `state_ed` fixture report.
- Fixture `.md` files are documentation, not test input; the fixture test skips them
  (`tests/fixtures/state_ed/README.md:24-26`).

## 6. Acceptance criteria met

- [x] Both captures inventoried with fixture path, corpus file, URL, HTTP status, byte count,
      and full sha256 (all verified with `stat`/`sha256sum` on 2026-10-10)
- [x] Reader contract bound to exact entry points, regexes, identity type, and enforcement lines
- [x] Applicability claim and refusal pinned to registry + applicability table lines
- [x] Limits stated: no re-fetch, no robots verdict, inferred index URL, undated captures
- [ ] Import/readback and `source-test state_ed` execution deferred to Main's focused checks

## Open questions

1. Index URL alias: `SOURCE.md:9` records the inferred path as
   `https://data.nysed.gov/analysis/` while `STATE_ED_EVIDENCE` cites
   `data.nysed.gov/lists.php?type=school`. Same page under two paths, or two different index
   shapes? Confirm before freezing the canonical URL in the scaffold README.
2. Occurrence vs. distinct counts: prose cites "2,528 `profile.php?instid=` links" (occurrences)
   while `SOURCE.md:14-20` pins 220 distinct ids / 220 distinct links. Keep both phrasings
   (2,528 numeric occurrences → 220 distinct schools) to avoid re-introducing the double-count.
