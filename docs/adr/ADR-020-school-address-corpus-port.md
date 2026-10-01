# ADR-020 — School-directory corpus port: the address pipeline and the TSSAA reader become Rust

**Status:** Accepted (2026-09-30).

## Context

The [ADR-008 amendment](ADR-008-rust-only-tooling.md) admitted `hs-address-pipeline/` (9 Python
files, 2 041 lines) and `parsers/tn_tssaa_school.py` as research material carrying (a) the discovery
strategy for school postal addresses and (b) the Tennessee association reader the Rust adapters still
owed. That amendment is the standing order to port them, one reader per Rust adapter, and to remove
the `tree.rs` skip entries rather than widen them. This ADR records what the port found and what it
built.

**What the Python actually does, and what the captures support.** The pipeline's four readers were
checked against the artifacts that exist in the two research trees:

|Prototype reader|Prototype assumption|Capture reality|
|---|---|---|
|`nccs_crawler.py`|Fixed positional CCD columns (`UnitID,Sector,SchoolName,Address1,…`)|`ccd_sch_029_2526_w_0a_050626.csv` (inside `ccd_sch_029_2526.zip`, sha256 `b5d8dc34…`, fetch `https://nces.ed.gov/ccd/data/zip/ccd_sch_029_2526_w_0a_050626.zip`) has 102 103 lines (header plus 102 102 schools) over 65 comma-separated, header-named columns: `SCHOOL_YEAR,FIPST,STATENAME,ST,SCH_NAME,LEA_NAME,…,MSTREET1,MCITY,MSTATE,MZIP,PHONE,WEBSITE,SY_STATUS_TEXT,SCH_TYPE_TEXT,CHARTER_TEXT,GSLO,GSHI,LEVEL,…`. Positional mapping reads `STATENAME` as the school name, `SCH_NAME` as the city, `LSTREET2` as the enrollment, and admits the header row as a school. Exactly 10 lines (50 040, 53 989, 72 897, 73 882, 74 506, 95 647, 95 648, 95 653, 97 062, 100 596) contain a `"`, every one of them a doubled quote inside an unquoted field, and no line begins with `"`; the prototype's `line.startswith('"')` header skip therefore never fires.|
|`pss_crawler.py`|Header-named PSS columns (`InstitutionID,InstitutionName,Street,…`) from `pss_schools.csv.gz`|No capture of that URL exists. The real local artifacts are the PSS public-use file `data/nces/pss/pss2324_pu.csv` (22 510 rows, 359 columns, sha256 `14a2f9e6…`) and the frame file `2023-24_PSS_Frame_Data.csv` (57 265 rows of `PPIN,ISR,OOS,INACTIVE`, sha256 `760b5de8…`). The public-use file carries `PPIN`, `PINST`, `PADDRS`, `PCITY`, `PSTABB`, `PZIP`, `PZIP4`, `PPHONE`, `NUMSTUDS`, `NUMTEACH`, `LEVEL`, `TYPOLOGY`, `RELIG`, `DIOCESE`, `SIZE`, `REGION` — `PINST` **is** the school name (`MT. PILGRIM CHRISTIAN ACADEMY`), so the port reads it; only the frame file lacks a name column, which is why the prototype derived one from `PADDRS`.|
|`state_ed_crawler.py`|Guessed CSV/JSON URLs for CA/TX/NY/FL/PA; `SchoolDirectoryParser` HTMLParser stub that is never called|The NY capture (`data.nysed.gov__58a5886b064eaca830473668`, sha256 `adf44bb6…`, 786 227 bytes, no URL sidecar) is the **letter-A** page of the state index (`<title>A - Schools | NYSED Data Site</title>`). The committed fixture replay measures **2 528 numeric `instid=` occurrences, 220 distinct school ids and 220 `profile.php?instid=<id>` links**, no `<table>`, no city and no ZIP. `800000054526` is already one of those twelve-digit ids, not an additional school. A profile page (`?instid=800000038718`, `A A KINGSTON MIDDLE SCHOOL`, 60 020 bytes) is captured separately. `notes/ny-nysed.md` records that city/address must come from a profile page or the download extracts. No robots capture exists for `data.nysed.gov`.|
|`private_associations_crawler.py`|NAIS listing parse; CAPE and NASSP return `[]` with "similar parsing logic"|No capture exists for `nais.org`, `cape-ed.org` or `nassp.org`, and no robots verdict for any of them. The test strategy therefore cannot be "match the prototype's URL"; it is "read the listing shape the operator supplies".|
|`tn_tssaa_school.py`|Detail-page parse: `tr.staffPerson` rows → card header sport, `mail_hide` email decode|Captured: **456 school pages** under `portal.tssaa.org/common/directory/?id=<n>` (455 with a URL sidecar; 430 carrying at least one `staffPerson` row), e.g. `portal.tssaa.org__7e2c78a9fba681cba03b6ed2` (`id=3`, `<h2>Alcoa High School`, 79 409 bytes, sha256 `c69f3c2d…`), plus three HTML shells with no school header or table and one `robots.txt` (`User-agent: * / Allow: /common / Allow: /access/attendance / Disallow: /`). **No `?type=high` listing page was captured**: the static HTML contains no data `<tr>` and no `?id=<digits>` anchor, because the table is built at runtime from a JavaScript typeahead array of **456** entries (`{id: '1', name: 'Adamsville High School (Adamsville, TN)'}`) that every school page embeds — which is where the 456 figure comes from. Measured over the 456 pages: 14 962 `staffPerson` rows and 405 pages with a Track & Field or Cross Country card header (`notes/tn-tssaa.md` instead records the prototype's emitted rows: 2 769 coach rows, 384 schools with a Track/CrossCountry row). A page does not name its own id, so the id comes from the capture's URL sidecar.|

**Prototype defects the port must not reproduce.** Silent `except Exception` skips that hide both
parse failures and fetch failures; a deduplicator whose docstring promises PSS-id priority but never
indexes `institution_id`; `_select_best_record` returning `group[0]` from the validated subset and
discarding every other field of the group; `normalize_local`'s blanket `str.title()`; USPS/Google
keys defaulting to `"YOUR_USPS_API_KEY"`; `update_strategy._next_update_time` returning prose
(`"September 2027"`, `"TBD"`); `needs_update` comparing only month numbers, so a January update makes
July look stale but a December one makes January look fresh; `_get_changes` annotated `List[str]` but
appending dicts; `_export_report` counting `final_count` over the fetch list rather than the exported
list. The port keeps the strategy (phases, sources, exports, schedule cadence) and drops the defects.

## Decision

1. **The corpus model is a pure domain module.** `census-domain::school_directory` owns `ZipCode`,
   `StreetLine`/`CityName`/`SchoolName`, `PostalAddress`, `GradeSpan`/`Grade`, `Enrollment`,
   `Coordinates` (exact 1e-7-degree integers — no floating point in the domain), `DirectoryKey`
   (`Nces`/`Pss`/`StateRecord`/`Weak`), `SourceLabel` (with collapse priority), `SchoolDirectoryEntry`,
   `ChangeSet`/`DirectoryField`/`FieldDelta`, `Cadence`/`YearMonth`/`ScheduleLedger`/`UpdateDecision`
   and `DirectoryError`. It has no I/O, no clock and no new dependency. `DirectoryKey::Weak` carries the
   name it is keyed by, so a nameless weak entry is unrepresentable.
2. **Four readers, one home each.** `census-crawl::nces` reads the CCD school file (`ccd`) and the PSS
   public-use file (`pss`) by **required header name**, refusing a file whose header lacks the
   columns the mapping needs. `census-crawl::state_ed` reads the NYSED A-Z index and profile pages and
   the tabular CSV/JSON directory family. `census-crawl::tssaa` reads the directory typeahead payload
   (the school array every page embeds) and the per-school page, emitting school rows keyed by
   Tennessee state id and Track/CrossCountry/AthleticDirector coach rows. A school page cannot name
   its own id — the captures carry neither a canonical link nor a populated `?id=` anchor in the
   static HTML — so the page parser takes the id the URL sidecar supplies and refuses a page whose id
   is absent rather than guessing one from the body.
   `census-crawl::private_assoc` reads association membership listings. Every reader returns
   `ReadOutcome { entries, skipped }`, so a dropped row is counted, never silent.
3. **Registry and applicability placement follows the evidence.** `nces` and `state_ed` are
   **artifact** readers (`admission.origin = local-artifact`): their inputs are operator-supplied
   files, and `nces`'s CCD capture is a ZIP whose decompression the operator performs
   (`unzip -p … > ccd_sch_029_2526.csv`) rather than a new compression dependency. `nces` plans all 49
   census jurisdictions (it is the national public-school universe); `state_ed` plans New York only.
   `tssaa` is **fetched** (`portal.tssaa.org`, robots allows `/common`) and plans Tennessee. None is
   wired into a per-jurisdiction sweep, so `plan` reports the recorded refusal
   ("no run stage sweeps this source per jurisdiction") and the corpus verb is the reachable path.
   `private_assoc` gets **no descriptor**: no capture, no robots verdict, so it stays a parser-only
   module listed in `NON_ADAPTERS` (like `compiled`, `hytek`, `raceday`) and is reachable only through
   the verb with an operator-supplied file.
4. **Geocoding and postal validation are typed clients, never stubs.** `census-crawl::geocode` holds a
   Google geocoder and a USPS validator, also `NON_ADAPTERS`: they admit no census origin, they are
   reachable only from the verb, they take a `SecretKey` whose `Debug` redacts, they never put the key
   in an error string, and they return typed per-row outcomes (`GeocodeOutcome`, `ValidationOutcome`)
   instead of the prototype's fabricated `validated: false` rows and `"accuracy"` field Google does not
   return. Absent credentials refuse the phase; absent coverage leaves coordinates absent.
5. **The pipeline becomes one verb.** `census-service school-address` takes artifact paths
   (`--ccd`, `--pss`, `--state-ed-index`, `--state-ed-profile`, `--state-ed-tabular`,
   `--associations`), an output directory, and — for the diff and the update ledger — a baseline path,
   a ledger path and a run month `--now YYYY-MM`. The verb reads no process clock: the month is the
   caller's, and a request that names a baseline or a ledger without it is refused rather than served
   with an inferred date. It runs read → collapse → optional validate/geocode → diff → stage →
   verify → atomically publish a generation, and prints counts and every skipped row. A first run
   against an absent baseline reports `changes: null` and writes no `changes.json`; a consumed,
   manifest-verified baseline requires `changes.json`, including an empty diff. Baseline and update
   ledger outputs always live inside the generation; supplied paths are read inputs, not mirrors.
6. **The Python is deleted, not shadowed.** `hs-address-pipeline/` and `parsers/tn_tssaa_school.py`
   are removed, together with the spent one-off scripts under `tools/`: `chsaa_golden.py`,
   `chsaa_make_fixtures.py` and `chsaa_port.sh` were zero-byte files, and `port_chsaa.py` and
   `port_chsaa_fixtures.sh` copied captures from the prototype tree into
   `crates/census-crawl/tests/fixtures/chsaa/`, whose committed `PROVENANCE.json` is the standing
   record of that capture. `xtask/src/contract/tree.rs`'s `SKIP` list returns to
   `["target", ".git", "var"]`, so `python_free` covers `tools/` and every other directory again, and
   the ADR-008 amendment is marked resolved so the exclusion cannot be reused.
7. **Documented deviations from the prototype**, each with a fixture or a test: header-name mapping
   instead of positions; quoted fields honored; rows without a name or without any key are skipped
   into the ledger instead of being emitted; dedup unions fields by source priority instead of
   keeping `group[0]`; re-casing applies only to all-caps input; `Coordinates` is exact fixed point;
   the schedule decision is a pure function of `(cadence, last, now)`; the report's counts are over
   the exported corpus; **ZIP is text, not a number** (`ZipCode` holds a five-character digit string
   and an optional four-digit extension, so `07030` stays `07030` and a nine-digit run is split on
   parse, where the prototype parsed an integer and lost the leading zero); **phone keeps the
   artifact's spelling** (`Phone::parse` rejects control characters, collapses whitespace and
   requires 7–15 digits, otherwise preserving the source text, where the prototype re-rendered US
   numbers with its own separators; `Website` requires an `http(s)://` scheme and no whitespace);
   **the report carries no wall-clock instant** (`pipeline_report.json` identifies the run by the
   caller's `--now` month and the input paths, which is what makes
   `school_address_writes_the_same_bytes_for_the_same_input` true, where the prototype stamped
   `datetime.now()`).

## Consequences

- The corpus verb is the only consumer of these readers; the census graph, its handlers and its stores
  are untouched. `census-store` is never opened by this path, so the one-process rule is unaffected.
- `nces`'s CCD path documents the operator's `curl` + `unzip` step; a `.gz`/`.zip` passed straight to
  the verb is refused with that command in the message. Adding in-process decompression would add a
  dependency to a vetted graph and is a separate decision.
- PSS public-use rows carry the published `PINST` school name, plus address, phone, level,
  enrollment and PPIN identity. The frame file lacks names; the port never guesses one from an
  address. The fixture's `A2380006` is `MT. PILGRIM CHRISTIAN ACADEMY`.
- `state_ed`'s tabular CSV/JSON reader exists for the state-agency family the prototype targeted, but
  only the NY index/profile shapes are capture-evidenced; the tabular reader's required-header
  contract is documented as unverified against a live state artifact and refuses rather than guesses.
- Fresh-run evidence obligations are unchanged: this port is tooling, and no export it produces is a
  census acceptance artifact until a run cites it.

## Implementation status

Landed with this ADR: `census-domain::school_directory`; the readers `census-crawl::nces` (CCD and
PSS), `census-crawl::state_ed` (NYSED index and profile), `census-crawl::tssaa` (school array and
school page) and `census-crawl::private_assoc`; the `census-service school-address` verb; the
applicability rows and registry descriptors; the fixture captures named in each
`crates/census-crawl/tests/fixtures/<source>/SOURCE.md`; the acceptance lanes
`cargo xtask source-test <source>`.

The verb's own lane is `crates/census-service/tests/school_address_corpus.rs`, which runs it over the
two committed NCES fixture windows without a store and asserts the observed corpus (1 931 entries —
1 557 CCD, 374 PSS — over 1 931 rows with 67 counted skips), the CCD lane digest, byte-identical
outputs across runs, `changes: null` on a first baseline run and a 0/0/0 change set with a `not-due`
schedule on the next month's run, the ledger's recorded month, and the four refusals: geocoding
without `GOOGLE_MAPS_API_KEY`/`GOOGLE_API_KEY`, postal validation without `USPS_API_TOKEN`, a baseline
diff without `--now`, and an artifact of the wrong shape. The `--state-ed-*` and `--associations`
lanes carry the same read → collapse → export path and are exercised by the readers' fixture lanes.

Built after that first landing: `census-crawl::geocode` (the typed clients §4 decides) and the two
phases that reach them. `Geocoder` joins `SourceLabel` at rank 5, so a geocoded coordinate never
displaces a source-published one; the phases run read → collapse → optional geocode/validate → diff →
stage, tally their verdicts in `report.phases`, stamp a filled coordinate `geocoder`, rewrite no
published field, and refuse a phase whose credential is absent before any request is built. Evidence:
14 client tests over the vendors' documented response shapes — request URLs, bearer header, status
mapping, typed field mapping, and the key/token absent from every rendered outcome — plus the two
credential refusals in the verb lane. No live vendor call has been made from this repository, so no
committed capture exists and the phases remain unqualified for a live run. Tracked as
`athletic-rust-pipeline-9p7`.

### Review amendments (2026-09-30)

An adversarial review of the corpus verb, readers and domain types raised three contract gaps.
The closing rounds resolved the gaps below under Main-approved contracts. Executed evidence and
the separate owner/environment blockers remain in the append-only review and verification ledgers.

- `athletic-rust-pipeline-37k` now has checked boundaries for the whole approved per-type inventory
  below, with executed invalid-value rejection and valid-byte preservation cases. This is artifact
  boundary evidence, not a claim of exhaustive national or live-source qualification.
- `athletic-rust-pipeline-tin` is implemented by the required field-provenance cutover below.
  Absorption no longer gives every populated field the strongest source in the record's union.
- `athletic-rust-pipeline-dtl` uses immutable generations and an atomic `current` publication
  reference, under the closing-review schema below. The directory-destination regression now
  rejects the legacy/blocking path before creating staging or publishing any artifact.

The three deviations the review forced into the open (ZIP as text, phone spelling, no wall-clock
stamp in the report) are in §7's list above.

### Closing-review contract cutovers (2026-09-30)

**Numbered grades and names (bounded 37k slice).** The public enum remains `Grade`, with variant
`Numbered(NumberedGrade)`; `NumberedGrade` has a private `u8`, `new`, `TryFrom<u8>`, and a checked
Serde `try_from` boundary. Numbered school grades admit **1–12 inclusive**. This makes explicit the
range already enforced by `Grade::parse` and the owning domain's published-form tests; the NCES
capture corpus corroborates every numbered value 1–12, plus the separately tagged PreK and
Kindergarten values. It is not evidence about postsecondary grades or a reason to clamp grade 13.
`SchoolName` checks its existing 160-character, nonempty, control-character and normalized-whitespace
contract through `TryFrom<String>`. Deserialization rejects noncanonical spelling rather than
silently trimming/collapsing it. Parsing raw reader input still normalizes as before.

Valid name JSON strings and `{"Numbered":n}` grade JSON retain their previous byte encoding under
ADR-017. Empty/overlong/control-bearing/noncanonical serialized names and numbered grades outside
1–12 are rejected as typed `DirectoryError` values at checked construction, mapped to Serde data
errors at artifact reads; no default or clamping. This verb writes JSON artifacts/baselines only,
not Fjall records, so there is no Fjall schema revision to bump.

**Per-field provenance (tin).** `SchoolDirectoryEntry` keeps the existing public value API and
adds private, required `provenance: BTreeMap<EntryField, SourceLabel>`. The private exhaustive keys
are `Name`, `Address`, `Kind`, `Grades`, `Enrollment`, `Phone`, `Website`, `Coordinates`; `Address`
is the existing aggregate postal-address field, not a new component-reconciliation policy. The
rank for every field comes solely from
`crates/census-domain/src/school_directory/label.rs::SourceLabel::rank`; there is no serialized
numeric rank, inferred ranking, or second table. A populated field's winning source travels with
its value. Equal ranks keep the existing deterministic value-order rule (greater value wins);
identical values at equal rank choose the smaller `SourceLabel` by its existing enum ordering,
so provenance and canonical artifact bytes are arrival-order independent. The record's `sources`
set is only the union of contributing sources.

The entry wire gains, for example,
`"provenance":{"Phone":{"StateEducationAgency":{"state":"Alabama"}}}`. The private self-remote Serde
derive uses the same entry type; there is no mirrored DTO. The decoder rejects provenance-free
payloads (`missing field provenance`), a missing source for a present field
(`MissingFieldProvenance`), a source for an absent field (`UnexpectedFieldProvenance`), an
unrecorded field source (`UnrecordedFieldSource`), or an empty source union
(`MissingEntrySources`). Source labels reject unknown fields, including any attempted `rank`
override. Because rank is computed from the source enum, a separately forged or inconsistent
numeric rank is not representable.

This is a clean artifact-schema cutover, not a compatibility path: old provenance-free baselines
must be regenerated from their original captured inputs and get an explicit error if supplied.
No repository school-directory schema-revision counter exists; the added required key is the
rejection discriminator. All six reader surfaces use the existing entry constructors/builders;
the collapse and baseline paths preserve winning sources through export/readback. The CSV
projection has no new columns and its fixture output is byte-identical before/after this review.

Given CCD without a phone, an athletic-association phone `9999999999`, and a state-ED phone
`1111111111`, every arrival permutation must resolve to `1111111111`, including after serializing
and decoding an intermediate merge. The former record-priority implementation retained
`9999999999` in that concrete case. Executed before/after observations and native corpus smoke
are appended to `docs/VERIFICATION-EVIDENCE.md` and `var/sol-review/state.md`.

**Completed 37k inventory (Main approval, 2026-09-30).** Strings and scaled coordinates use the
existing checked `serde(try_from)` convention with private payloads. Cross-field composites
`ZipCode`, `PostalAddress`, `GradeSpan` and `WeakKey` use the entry's self-remote Serde convention,
not mirrored DTOs. Coordinates require both checked scalar components. Persisted values are
rejected with typed `DirectoryError` mapped into Serde data errors; no normalization, clamping
or fallback occurs at decode. Valid representations below retain their prior JSON bytes.

|Type|Current JSON wire|Required invariant at decode|
|---|---|---|
|`AssociationLabel`|string|nonempty, at most 48 characters, existing control and whitespace rules|
|`MatchForm`|string|equals the lowercase alphanumeric/space form produced by `MatchForm::of`; empty is allowed only where the containing type allows it|
|`StreetLine`|string|nonempty, at most 120 characters, existing control/whitespace/presentation rules|
|`CityName`|string|nonempty, at most 64 characters, existing control/whitespace/presentation rules|
|`NcesSchoolId`|string|exactly 12 ASCII digits, no retained whitespace|
|`PssId`|string|exactly 8 ASCII alphanumeric characters, uppercase canonical spelling|
|`StateRecordId`|string|nonempty, at most 64 characters, no whitespace or control characters|
|`Phone`|string|present value is nonempty, at most 32 characters, 7–15 ASCII digits, existing control/whitespace rules; missing phone is the enclosing Option, not an empty string|
|`Website`|string|present value is nonempty, at most 200 characters, HTTP(S) prefix, no whitespace/control characters; missing is the enclosing Option|
|`Latitude`|integer in 1e-7 degrees|inclusive -900000000..900000000, not merely any i32|
|`Longitude`|integer in 1e-7 degrees|inclusive -1800000000..1800000000, not merely any i32|
|`ZipCode`|object `{"code":"12345","plus4":"6789"}` (plus4 optional)|code exactly 5 ASCII digits; present plus4 exactly 4 ASCII digits|
|`PostalAddress`|object with optional line1/line2/city/state/zip|at least one valid populated component; the empty object must not become a present address|
|`Coordinates`|object `{"latitude":scaled,"longitude":scaled}`|both required components satisfy the latitude/longitude bounds|
|`GradeSpan`|object `{"low":Grade,"high":Grade}`|both ends rankable (PreK/Kindergarten/1–12), low rank <= high rank; Ungraded/AdultEducation cannot be span endpoints|
|`WeakKey`|object `{"name":MatchForm,"city":MatchForm,"state":UsJurisdiction}` (city/state optional)|nonempty canonical matching name; a present city is nonempty canonical matching form|

`Enrollment` is already a JSON `u32` with no narrower constructor invariant. `Month` (integer
1–12) and `YearMonth` (string `YYYY-MM`, year 1900–2200) already check their own deserialization.
`SchoolKind`, `SourceLabel`, `DirectoryKey`, and `IdentifiedKey` keep their existing tagged enum
shapes and depend on their validated children. `Baseline` (`{"entries":[...]}`), `ChangeSet`
(added/removed entries and modifications), and `ScheduleLedger` (`{"last":{source:"YYYY-MM"}}`)
inherit the entry/identifier/calendar obligations rather than creating a different domain model.

The ranges are the existing reader/domain policies, not new limits inferred from a small capture:

|Range/policy|Authoritative source and corpus corroboration|
|---|---|
|SchoolName 160; AssociationLabel 48 characters|`school_directory/name.rs::{NAME_LIMIT,LABEL_LIMIT}` and `text.rs::bounded`; existing normalization policy, not a claimed government length limit|
|StreetLine 120; CityName 64 characters|`school_directory/address.rs::{STREET_LIMIT,CITY_LIMIT}` and `text.rs::presentation_case`; NCES/NYSED fixture addresses corroborate the presentation spelling|
|NCES 12 digits; PSS 8 ASCII alphanumerics|`school_directory/ids.rs::{NCES_ID_DIGITS,PSS_ID_LENGTH}`; published `NCESSCH` and `PPIN` fields in `crates/census-crawl/tests/fixtures/nces/SOURCE.md` and reader fixture windows (including leading-zero NCES and `A2380006`)|
|StateRecordId 64 characters, no whitespace/control|`school_directory/ids.rs::STATE_ID_LIMIT` and its existing parser policy; NYSED 12-digit institution ids and TSSAA numeric ids are reader corroboration, not grounds to require one numeric length for all states|
|Phone 32 characters, 7–15 digits; Website 200 characters, HTTP(S)|`school_directory/contact.rs::{PHONE_LIMIT,PHONE_DIGITS_MIN,PHONE_DIGITS_MAX,WEBSITE_LIMIT}` and existing parsers; spelling retained in NCES/NYSED fixtures. These are application policies, not invented source-wide bounds|
|Latitude ±90°, longitude ±180°, scaled by 10^7|`school_directory/coordinates.rs::{LATITUDE_MAX,LONGITUDE_MAX,SCALE}`; geographic coordinate bounds already enforced by decimal parsing, now also integer deserialization|
|ZIP 5 digits plus optional 4|`school_directory/address.rs::{ZipCode::of,is_digits}`; published NCES `MZIP/MZIP4`, PSS `PZIP/PZIP4`, and existing leading-zero/ZIP+4 reader cases|
|GradeSpan rankable PreK/K/1–12, low<=high|`school_directory/school.rs::{Grade::parse,Grade::rank,GradeSpan::new}` and existing published-form tests; every numbered grade was observed in the NCES capture endpoint census recorded in round 1|
|MatchForm canonical alphanumeric lowercase/spaces; WeakKey nonempty name/present city|`school_directory/name.rs::MatchForm::of` and `key.rs::WeakKey::of`; no arbitrary character-length limit added|
|PostalAddress at least one component; Coordinates both components|existing `PostalAddress::of`/`is_empty` and `Coordinates::parse`; populated-invalid and present-empty persisted composites now reject|

Raw reader normalization remains separate from persisted validation. Two executed Unicode
counterexamples exposed raw case expansions that were not fixed points of their own policies:
`MatchForm::of("İ")` emitted `i` plus a combining dot, and presentation-casing `"ß"` emitted `"SS"`,
which decoded as noncanonical. Lowercase expansion now retains only alphanumeric characters,
and uppercase word-initial expansion title-cases its continuation (`"Ss"`), so parser-produced
values round-trip through the checked boundary. ASCII corpus/CSV bytes are unchanged. A present
punctuation-only WeakKey city now returns `EmptyField { field: "matching city" }`, rather than
silently discarding the supplied city. All reader callers already propagate the constructor's
fallible result into their counted row outcomes.


**Transactional generation publication (dtl, Main approval, 2026-09-30).**

The only authoritative output is `<out>/current`, a symlink to
`generations/<generation_digest first 16 hexadecimal characters>/`. A generation always contains
`school_directory.json`, `school_directory.csv`, `baseline.json`, `update_ledger.json`,
`manifest.json`, and `pipeline_report.json`. It additionally contains `changes.json` exactly when
a prior baseline was actually consumed (not merely an absent path requested). The report retains
`changes: null` and the file remains absent on the first baseline run. All flat legacy output
layouts are rejected explicitly, never migrated in place or used as a fallback.

The schema-revision-1 manifest is canonical JSON under ADR-017:
`{schema_revision, run:{run_id, created_at, inputs}, artifacts:[{name, sha256, bytes}], generation_digest}`.
`inputs` carries the existing lane reports (paths, input hashes, counts, rejected locators), plus
the consumed baseline and ledger SHA-256 or `null` for an absent input. `created_at` is the caller's
`--now` month, or `null`, never a wall clock. `run_id` is the domain canonical digest of the tuple
`[created_at, inputs]`. Artifacts are lexicographically sorted and list exactly the four required
data files plus `changes.json` when `inputs.baseline` is non-null. The generation digest is SHA-256
of canonical `{schema_revision, run, artifacts}`: only its own `generation_digest` field is omitted.
Neither manifest nor report is self-hashed. The report's `manifest_digest` must equal this digest;
its run month, input lane metadata and presence of a change report must agree with the manifest.

Each artifact is written to the unpublished `<out>/.staging.<pid>/` sibling, fsynced, and the
staging directory is fsynced. The entire staged bundle is independently verified before the first
rename. A prepared `<out>/.current.<pid>.tmp` symlink ensures pointer staging is possible first.
Rename staging to the immutable generation directory, fsync `generations/` and `<out>/`, then
perform exactly one rename of the prepared pointer onto `current` and fsync `<out>/`. That pointer
rename is the sole commit point. Existing equal generations are verified and reused, never
overwritten; a digest-prefix collision is refused. Before commit, all failures leave the previous
generation intact. Incomplete staging is removed on handled failure; a complete unreferenced
generation is retained and named in the diagnostic. Abrupt interruption can retain `.staging.<pid>`
or `.current.<pid>.tmp`; operators inspect these named siblings and `current`, never infer publication
from an orphan. No automatic deletion of complete orphan generations occurs.

Consumers resolve `current` once, require a real generation immediately inside the root's
`generations/`, rederive the manifest/run digests, verify every artifact's hash and byte length,
require the exact declared file set, and verify the report binding before exposing cached bytes.
Missing files, corruption, outside pointers and unmanifested external baselines are typed
`GenerationError` outcomes. External `--baseline`/`--ledger` inputs must be verified generation
artifacts; no external mirror writes occur. Use `<out>/current/baseline.json` and
`<out>/current/update_ledger.json` to feed the next run. The verb opens no Fjall store; revision 1
is an artifact manifest schema, not a Fjall schema bump.

Acceptance interrupts the real publisher in subprocesses before the directory rename, between
that rename and the pointer swap, and after the pointer swap. The first two retain the complete
old current generation; the last exposes the complete new generation. Every data artifact differs
between the test generations, so a mixed set cannot satisfy the byte assertions.

