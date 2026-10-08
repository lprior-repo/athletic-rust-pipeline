# `nces` reader — the national school universes

## Purpose

Reads the two national NCES files the census corpus is addressed from, and turns every in-scope row
into one `SchoolDirectoryEntry`:

|File|What it is|Rows|Shape|
|---|---|---|---|
|`ccd_sch_029_2526_w_0a_050626.csv` (inside `ccd_sch_029_2526.zip`)|Common Core of Data public-school universe, 2025-26 school year|102,102 data rows|65 unquoted, comma-separated, header-named columns|
|`pss2324_pu.csv`|Private School Survey public-use file, 2023-24 school year|22,510 data rows|359 quoted-capable, header-named columns|

Both are **artifact** readers (`admission.origin` = `local-artifact`): the operator downloads the
file and hands the extracted CSV to the `census-service school-address` verb, which reads it from
disk. The reader makes no request, so no robots verdict applies. The CCD member is a ZIP; the
operator decompresses it outside the process:

```console
curl -fL -o ccd_sch_029_2526.zip https://nces.ed.gov/ccd/data/zip/ccd_sch_029_2526_w_0a_050626.zip
unzip -p ccd_sch_029_2526.zip ccd_sch_029_2526_w_0a_050626.csv > ccd_sch_029_2526.csv
```

## Entry points

|Item|What it is|
|---|---|
|`parse_ccd(text)`|CCD rows to entries keyed `IdentifiedKey::Nces` with `SourceLabel::Ccd`|
|`parse_pss(text)`|PSS rows to entries keyed `IdentifiedKey::Pss` with `SourceLabel::Pss`|
|`SOURCE_ID`|`"nces"`|

Both return `ReadOutcome`: entries, plus a skip ledger that names the line and the field every
dropped row lost. A malformed header is not a ledger row — the file is not the shape this reader is
contracted to read, so the whole artifact is refused with `CrawlError::Invariant` naming the missing
columns.

## Mapping

Required header names (a missing one refuses the artifact): CCD `NCESSCH, SCH_NAME, MSTREET1, MCITY,
MSTATE, MZIP, MZIP4, PHONE, GSLO, GSHI, CHARTER_TEXT`; PSS `PPIN, PINST, PADDRS, PCITY, PSTABB, PZIP,
PZIP4, PPHONE, NUMSTUDS`. Every other column is optional: the reader uses it when the header carries
it and records nothing when it does not (both files carry `LZIP`/`LZIP4`, `ENROLLMENT` does not exist
in the CCD file, and `LATITUDE24`/`LONGITUDE24` exist only in the PSS file).

- **Identity.** CCD `NCESSCH` must be 12 digits (`NcesSchoolId`); PSS `PPIN` must be eight
  alphanumerics (`PssId`, upper-cased — real PPINs are `A2380006` and `02004116`).
- **School jurisdiction.** CCD uses the published `ST`, then `LSTATE`/`MSTATE` only when `ST`
  is absent; PSS uses `PSTABB`. This is stored separately as the entry's jurisdiction, never
  copied into an address component. A school jurisdiction outside `UsJurisdiction::CENSUS_SCOPE`
  drops the row with `state` named in the ledger. Closed and future schools stay in the corpus.
- **Address tuples.** CCD parses location (`LSTREET1`, `LSTREET2`, `LCITY`, `LSTATE`, `LZIP`,
  `LZIP4`) and mailing (`MSTREET1`, `MSTREET2`, `MCITY`, `MSTATE`, `MZIP`, `MZIP4`) independently.
  Any nonempty published location component selects `Physical`, even when partial or malformed;
  only an entirely absent location tuple selects `Mailing`. Selection never uses parse success or
  completeness as evidence of source absence. No mailing street, second line, city, state, ZIP
  or extension fills a physical component. Field failures retain valid components and ledger
  diagnostics. An extension without its source tuple's base ZIP remains a diagnostic, not a ZIP.
- **Unselected claims.** The current entry carries one selected tuple. A published mailing tuple
  not selected is disclosed in the note ledger with all six raw components and its source row;
  the original artifact capture retains it. It is not a second canonical postal claim.
- **PSS address semantics.** `PADDRS`/`PCITY`/`PSTABB`/`PZIP`/`PZIP4` form the published mailing
  address, tagged `Mailing`, including PO boxes. No coordinates or school identity imply that it
  is a physical campus. Historical serialized addresses without a kind remain `Unknown`.
- **Kind.** CCD rows are `Public { charter: false }` unless `CHARTER_TEXT` is `Yes`; PSS rows are
  `Private { affiliation: None }` (NCES publishes no affiliation there).
- **Grades.** `GSLO`/`GSHI` map through `Grade::parse`/`GradeSpan::new`: `M` (missing) and `N` (not
  applicable) mean the file publishes no span, a value that is not a grade is a ledger note.
- **Enrollment, phone, coordinates.** `ENROLLMENT` (when present), `PHONE`/`PPHONE` (7–15 digits,
  formatted forms like `(256)878-2341` accepted), `LATITUDE24`/`LONGITUDE24` (exact fixed point).
- **Quoting.** Fields are read through the `csv` crate, so a quoted field keeps its comma
  (`"COTTAGE HILL CHRISTIAN ACADEMY, LOWER"` and `"130 N ST E., STE C"` are real captures).

## Fixtures

`crates/census-crawl/tests/fixtures/nces/` holds byte-exact windows of both files; `SOURCE.md`
records the URL, the window, and the digest of the window and of the full artifact. The
fixture-driven tests live in `crates/census-service/tests/nces_directory_properties.rs`:

`crates/census-service/tests/cen12_address_tuple.rs` uses synthetic, source-schema adversarial
fixtures under `crates/census-crawl/tests/fixtures/cen12/`, not captured national evidence. Its
parser → corpus generation → durable join → workbook readback scenarios assert complete and
partial physical tuples, absent physical tuples, conflicting mailing cities/states/ZIPs, PO boxes,
invalid ZIP diagnostics, and exact kind publication. Workbook/CSV projections append an explicit
address-kind field; they never infer historical kind. These scenarios require execution after
the coordinated repair wave; their presence alone is not verification.

```bash
env -u CI tools/moon-local run pipeline:xtask -- source-test nces
```

The nextest filter matches **test function names**, not file names, so every acceptance test for a
source carries the source slug in its function name (`nces_ccd_window_reads_the_same_way_twice`,
matching `mshsl_fixtures_match_golden` in the older census-adjacent lane). A test in
`nces_directory_properties.rs` whose function omits `nces` is silently skipped by the lane.

The windows measure 1,557 CCD entries with 42 Alaska skips, and 374 PSS entries with 25 Alaska skips.

## Limitations

- The CCD reader does not decompress; a `.zip` handed to the verb is refused with the `unzip`
  command above in the message. Adding in-process decompression would add a dependency to a vetted
  graph and is a separate decision.
- The PSS file is a *public-use* file: it carries `PINST` (the school name), street address, phone,
  enrollment and coordinates, and no affiliation beyond the file's own columns; the census treats it
  as address and level evidence, never as an authoritative name.
- Neither reader enforces a row limit: the verb bounds the artifact size it hands over.

The parser borrows source cells while constructing owned validated tuples and moves its selected
tuple into the entry. Corpus CSV rendering borrows published text through a fixed record array;
formatted identifiers, ZIPs, numeric values and labels still require encoded text, and the CSV
writer owns the output buffer. The existing borrowed-input `DirectoryIndex` owns copies of selected
addresses, and the join constructs an owned durable claim. Those copies are not free and have not
been measured in this repair wave. No latency, throughput or allocation improvement is certified;
post-wave acceptance must measure the representative corpus/index/join/publication path.
