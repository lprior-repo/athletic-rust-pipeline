# coach_contacts source report

Source slug: `coach_contacts` (researched official coach-contact dataset). No `SRC-###` id is
assigned to this slug in this worktree; the research-workspace citation key is
`coach_contact_graph` [29] (`crates/census-crawl/src/applicability/table/prose.rs:97`;
`research/sources/national-aggregators/data/source-coverage-matrix.csv:7`).

## Source identity (registry row)

`crates/census-crawl/src/registry/table/through_milesplit.rs:66-71`:

```
    SourceDescriptor {
        slug: "coach_contacts",
        provider: "Researched official coach-contact dataset",
        transport: TransportKind::Csv,
        capabilities: SCHOOL_COACH_CONTACT,
        admission: artifact(),
    },
```

`artifact()` is `fetched(ARTIFACT_ORIGIN, FETCHER_RPS)`
(`crates/census-crawl/src/registry/policy.rs:42-44`); the capability block
`SCHOOL_COACH_CONTACT` sets `school_evidence`, `coach_directory` and
`public_professional_contact` (`policy.rs:15-18`).

## URLs

This lane is artifact-fed: the registry row declares a CSV transport and an
artifact admission, so there is no single provider URL. Each sampled row carries
its own `source_url`; hosts present in the fixture are `schools.wiaawi.org`,
`kshsaa-api.kshsaa.org`, `secure.nsaahome.org`, `my.mhsaa.com`, `gobound.com`
and `api.ihsa.org`.

## Applicability (jurisdiction coverage)

`crates/census-crawl/src/applicability/table/data.rs:89-117` declares the row:

- `slug: "coach_contacts"` (data.rs:90)
- `jurisdictions:` (data.rs:91) lists 22 jurisdictions (data.rs:92-113):
  Arizona, California, Colorado, DistrictOfColumbia, Florida, Georgia, Iowa,
  Illinois, Indiana, Kansas, Michigan, Minnesota, Missouri, NorthDakota,
  Nebraska, NewJersey, Ohio, Oregon, Tennessee, Texas, Utah, Wisconsin.
- `evidence: super::prose::COACH_CONTACTS_EVIDENCE` (data.rs:115),
  `refusal: super::prose::COACH_CONTACTS_REFUSAL` (data.rs:116).

Contradiction with the task context: the assignment described the adapter as
covering "23 jurisdictions". The applicability table lists 22 (data.rs:92-113)
and data.rs contains no second `coach_contacts` row, so 22 is the repository
value.

Evidence prose, `crates/census-crawl/src/applicability/table/prose.rs:97-100`:

> `coach_contact_graph` [29], `verified (sampled; yields recorded per state)`: the 2026-09-21 research-workspace artifact held 6,215 rows for exactly the jurisdictions listed (WI 3,166, KS 556, MN 389, WI/IL 183 each, ...), each with the role, the published professional address where the provider publishes one, and the page it was observed on.

Refusal prose, `prose.rs:102-105`:

> `[coach-directories-national]` collected 22 tier-1 directories of 51: the rest are login-gated (MI `my.mhsaa.com`, MS 403 WAF), client-rendered (PA, NH, TX `/files/` robots-disallowed), or publish names with no address anywhere (ND 0 emails, SD 4 coach emails from a prior study), and a directory the adapter cannot read is not a planned source.

## Capture inventory

Fixture (raw-import lane, committed):
`crates/census-crawl/tests/fixtures/coach_contacts/coach_contacts_sample.csv`

- sha256 `27340d8a9b46c0ecae108e6b3d31d307e1edd3627ce1660001785573dfa65934`
- 2926 bytes, 16 lines = 1 header + 15 data rows
- byte-identical copy of the root sample
  `crates/census-crawl/tests/fixtures/coach_contacts_sample.csv` (same sha256,
  same 2926 bytes, `cmp` returned no output)
- header, line 1:
  `school,city,state,sport,role,coach_name,public_professional_email,ad_name,ad_email,source_url,last_observed`

Commands: `sha256sum`, `wc -c` and `wc -l` were executed in this worktree on
2026-10-09 (main-agent shell; raw output:
`27340d8a9b46c0ecae108e6b3d31d307e1edd3627ce1660001785573dfa65934`, `2926`,
`16`; the fixture copy's `sha256sum` equals the root hash and `cmp` was clean).
The row count and header were independently re-derived here by reading the file:
16 newline-terminated lines, 11 header columns, 15 data rows.

The root sample is unmodified and stays in place:
`crates/census-crawl/src/coach_contacts/tests.rs:10` includes it by relative path
and `crates/census-service/tests/parity_national.rs:33` resolves
`coach_contacts_sample.csv` under the fixtures dir.

The full artifact is not in this repository; this fixture is a 15-row documented
sample (scope in Known limits).

## Field inventory

- 11 columns (header above), matching `CoachContactRow`
  (`crates/census-crawl/src/coach_contacts/wire.rs:9-30`): school, city
  (default), state, sport (default), role (default), coach_name (default),
  public_professional_email (default), ad_name (default), ad_email (default),
  source_url (default), last_observed (default).
- Raw lane expects 11 columns: `RAW_CONTACT_HEADER_COUNT: usize = 11`
  (`crates/census-crawl/src/coach_contacts/artifact.rs:21`). Verified lane
  expects 12: `VERIFIED_CONTACT_HEADER_COUNT: usize = 12` (`artifact.rs:23`);
  the 12th column is `CONTACT_PROOF_COLUMN`, re-exported with `CONTACT_COLUMNS`
  at `artifact.rs:27`.
- The same 11-column list is recorded for the baseline product at
  `research/sources/national-aggregators/data/source-coverage-matrix.csv:7`
  ("2,298 rows / 874 schools / 10 states in data/coach-contacts.csv (school,
  city, state, sport, role, coach_name, public_professional_email, ad_name,
  ad_email, source_url, last_observed)").

## Artifact-lane read/write contract

- Raw read: `read_raw_contacts(path)`
  (`crates/census-crawl/src/coach_contacts/artifact/read.rs:15`) opens a
  standalone CSV and validates the 11-column header (read.rs:16-17). Caps:
  `MAX_CSV_BYTES` 128 MiB (`artifact/io.rs:4`), `MAX_RECORD_BYTES` 1 MiB
  (`io.rs:6`), `MAX_ROWS` 200,000 (`io.rs:8`).
- Verified read: `read_verified_contacts(directory)` (read.rs:39) requires the
  trio `manifest.json` + `contacts.csv` + `contacts.csv.evidence.jsonl` (names at
  `io.rs:9-11`; directory joins in read.rs:42-46) and checks the csv and jsonl
  sha256 digests against the manifest (read.rs:54-63).
- Write: `stage_verified_contacts`
  (`crates/census-crawl/src/coach_contacts/artifact/write.rs:13`);
  `MANIFEST_FORMAT_VERSION: u32 = 1` (`artifact.rs:25`).
- This fixture is the raw-import lane only: it commits the 11-column CSV and no
  artifact trio. No `contacts.csv`, `contacts.csv.evidence.jsonl` or
  `manifest.json` exists under `crates/census-crawl/tests/fixtures/coach_contacts/`,
  so the digest-checked verified read path cannot be exercised from this fixture.

Import lane (raw): `import_csv(store, path, default_observed_on)` at
`crates/census-crawl/src/coach_contacts/import.rs:16`; CLI form
`census-service --store <root> import-coaches <csv> --observed-on <date>`
(`research/sources/coach-coverage-bundle-20261004/TAP-PLAN.md:26-27`).

## Tests that consume this fixture

- `crates/census-crawl/src/coach_contacts/tests.rs` - in-module import tests;
  the module embeds the root sample at tests.rs:10
  (`include_str!("../../tests/fixtures/coach_contacts_sample.csv")`). Test
  functions: tests.rs:20 `parses_sport_and_gender_labels`, tests.rs:37
  `non_coaching_roles_are_not_imported`, tests.rs:57
  `coach_rows_become_canonical_entities_with_evidence`, tests.rs:99
  `import_preserves_published_people_and_excludes_non_coaching_roles`.
- `crates/census-service/tests/parity_national.rs:29` - golden test
  `coach_contacts_csv_preserves_published_people_and_deduplicates_replay`,
  reading the sample at parity_national.rs:33.
- Artifact-lane tests: `crates/census-crawl/src/coach_contacts/artifact/tests.rs`,
  `.../artifact/tests_edge.rs`, `.../artifact/tests/{helpers,limits,edge_cases}.rs`
  (present in the tree; not read for this report).

## Underlying research artifact - what the repository documents

Documented: the 2026-09-21 research-workspace artifact, 6,215 rows, per-state
yields WI 3,166 / KS 556 / MN 389 / ... (prose.rs:97-100, quoted above).

Files found and read in this worktree that describe the lane:

- `research/sources/coach-coverage-bundle-20261004/TAP-PLAN.md:23-27` -
  derivation contract: rows in the `CoachContactRow` CSV schema, citing this
  fixture path, then `import-coaches`; TAP-PLAN.md:40 routes 51 `extract_rows`
  sources to the `coach_contacts` import.
- `research/sources/coach-coverage-bundle-20261004/TAP-REGISTRY.json` - 20 source
  entries carry `"mech": "coach_contacts_csv"` (lines 28, 108, 228, 428, 508,
  628, 1228, 1268, 1428, 1868, 1908, 2108, 2628, 2708, 2788, 3152, 3232, 3792,
  3986, 4226).
- `research/sources/coach-coverage-bundle-20261004/state-work-orders/group-a.md`,
  `group-b.md`, `group-c.md`.
- Bundle root also holds `COACH-SCHOOL-RESOURCES-COMPLETE.json`,
  `HELD-CLASSIFY.{json,md,recovered.json}`, `HELD-INVENTORY.md`,
  `STATE-COVERAGE-MATRIX.md`, `ALL-STATE-COACH-AND-SCHOOL-RESOURCES.md`,
  `verify-captures.sh`, `audit/{reconciliation-20261005.md,tap-tree-rehash-20261005.md,held-classify-index.txt}`.
  The bundle contains no coach-contacts CSV.

The artifact's own sha256 or byte count is absent from this repository: a
worktree-wide search for `coach_contact_graph|6,215` matched only prose.rs:97-98
and unrelated matrix rows, and the bundle tree publishes no digest for a
coach-contacts CSV. The closest recorded sibling file is the MSHSL research
artifact `~/Downloads/midwest-tfxc-source-research/data/canonical-coaches.csv`
(5.7 MB, 31,489 lines, sha256
`e4b10432c08d809fa7373e8a3f8c3f7ca687c37368fdd5a781ca62f4c10ee28b`, owner-run
`sha256sum` on 2026-10-09, `research/sources/mshsl/SOURCE_REPORT.md:68`) - a
different file with its own hash, not this artifact. A 2,298-row
`data/coach-contacts.csv` is referenced for the baseline lane
(`research/sources/state-assoc-plains/SOURCE_REPORT.md:29`; the re-derivation
record `data/coach-contacts.csv (re-derived 2026-09-21 by this lane)` is at
`research/sources/state-assoc-greatlakes/coverage.json:236`), also without a
repository-recorded hash.

## Verified data (15 rows in the fixture)

Six schools in six states (WI, KS, NE, MI, SD, IL); `last_observed` is
`2026-09-19` on every row; the NE/SD/MI rows carry no city:

- Abbotsford (WI, `schools.wiaawi.org/.../orgID=1`): 3 `Head Coach` rows with
  emails - Boys Track and Field, Girls Cross Country, Girls Track and Field
  (JACOB KNAPMILLER, Dillon Novak, JACOB KNAPMILLER) - plus 1 Athletic Director
  row (Alex Larson).
- Abilene HS (KS, `kshsaa-api.kshsaa.org`): 1 Athletic Director row (Derek
  Berns).
- Adams Central (NE, `secure.nsaahome.org`): 3 `Head Coach` rows with no emails
  - Cross-Country (Boys), Cross-Country (Girls), Track & Field (Boys).
- East Kentwood HS (MI, `my.mhsaa.com/DesktopModules/MHSAA-Endpoint/API/School/AdministrationDirectory?SchoolId=4351`):
  Superintendent, Principal and Athletic Trainer rows - non-coaching roles the
  importer filters (tests.rs:37).
- aberdeencentral (SD, `gobound.com/sd/schools/aberdeencentral/directory/new`):
  Activities Director and Athletic Director rows, no emails.
- Abingdon-Avon High School (IL, `api.ihsa.org/v1/schools/0101/staff2`): 2
  `Head Coach` rows with emails - Boys Cross Country, Boys Track & Field.

## Qualification status

- Registered adapter: CSV transport, capability `SCHOOL_COACH_CONTACT`,
  admission `artifact()` (through_milesplit.rs:66-71; policy.rs:15-18, 42-44).
- Applicability: 22 jurisdictions (data.rs:89-117). Evidence prose asserts 6,215
  verified sampled rows across "exactly the jurisdictions listed"
  (prose.rs:97-100); refusal prose records 22 of 51 tier-1 directories
  collected and names the blockers (prose.rs:102-105).

## Known limits

1. This fixture is a 15-row documented sample, not the 6,215-row artifact; it
   cannot stand in for the artifact's jurisdiction coverage.
2. Sample states (6) are a subset of the applicable 22; there are no MN/OH/TX/MO
   and no other rows here.
3. No artifact trio is committed, so the digest-checked verified read/write path
   is not exercisable from this fixture.
4. Artifact hash absent: no sha256 or byte count for the 2026-09-21 artifact is
   recorded in the repository (searched as described above). Any claim about the
   artifact's exact bytes is unverifiable from this worktree.
5. Dates: `last_observed` is 2026-09-19 on every row while the artifact is
   documented as 2026-09-21 (prose.rs:98); the sample carries no tenure
   guarantee.
6. Non-coaching rows: the MI block (Superintendent/Principal/Athletic Trainer)
   is filtered by role at import (tests.rs:37), so imported rows are fewer than
   15.
7. Michigan conflict: the sample's MI rows come from
   `my.mhsaa.com/DesktopModules/...`, while the refusal prose lists MI as
   login-gated for collection (prose.rs:102-103: "the rest are login-gated (MI
   `my.mhsaa.com`, MS 403 WAF)"). The rows document the researched dataset; they
   are not evidence that the MI lane is collectible under the current policy.
8. The evidence prose lists "WI 3,166, KS 556, MN 389, WI/IL 183 each"
   (prose.rs:98); WI appears twice in that sentence as written. Quoted verbatim,
   not resolved here.
