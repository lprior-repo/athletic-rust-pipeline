# `run.py` merge semantics vs. the Rust census pipeline (ADR-015 port)

Branch `coach-acquisition-rust`, worktree `/home/lewis/src/ad-law-scrape/arh-coach-acquisition`.
Read-only research: no repo file was edited. `census-prototype` is treated as read-only ground truth.

Rust side compared: `crates/census-crawl`, `crates/census-domain`, `crates/census-store`
(plus a note on `crates/census-service`, which owns the export step where artifact-level rules would live).

---

## 1. Method

1. Read `run.py:1-473` end to end plus `extract.py`, `merge_enrichment.py`, `integrate_lanes.py`,
   `fix_state_labels.py`.
2. Executed the merge functions in-process (`import run; run.merge_state(...)`) on synthetic
   multi-source inputs to observe the artifact rules directly. Commands and complete stdout in §2.
3. Located the Rust equivalents by source read/grep only — **no Rust build or test run was executed**
   (project-wide validation belongs to the main agent). Every Rust ref below is therefore a source
   citation, not an executed observation; executed Python output is marked as such.
4. Line numbers come from `grep` output (`NNN|` prefixes) or from `read` windows; where a citation is
   function-level rather than line-level it is written as `file: symbol`.

Side-effect check: `import run` did not write under `census-prototype`
(`run.cpython-314.pyc` mtime `29 Sep 07:53` == `run.py` mtime `2026-09-29 07:53:28`, i.e. not refreshed
by the probes). The only file this task wrote is this one.

---

## 2. Evidence log (executed)

### E1 — `normalize_school` key (`run.py:33-38`)
```
python3 -B -c "... cases=[...]; [print('SCHOOLKEY', repr(c), '->', repr(run.normalize_school(c))) for c in cases]"
```
```
SCHOOLKEY 'Madison High' -> 'MADISON'
SCHOOLKEY 'Abbotsford Highschool' -> 'ABBOTSFORD HIGHSCHOOL'
SCHOOLKEY 'Center Grove High School High School' -> 'CENTER GROVE'
SCHOOLKEY 'Madison Junior High' -> 'MADISON JUNIOR'
SCHOOLKEY 'School' -> 'SCHOOL'
SCHOOLKEY 'High School' -> 'HIGH SCHOOL'
SCHOOLKEY 'Glencoe-Silver Lake HS' -> 'GLENCOE SILVER LAKE HS'
SCHOOLKEY 'A B HS School' -> 'A B HS'
SCHOOLKEY 'Abbotsford Academy' -> 'ABBOTSFORD ACADEMY'
SCHOOLKEY 'St. Marys High' -> 'ST MARYS'
SCHOOLKEY 'Center Grove Sr High' -> 'CENTER GROVE'
SCHOOLKEY 'Abbotsford H S' -> 'ABBOTSFORD H S'
SCHOOLKEY 'Abbotsford HS' -> 'ABBOTSFORD HS'
SCHOOLKEY 'Abbotsford High School' -> 'ABBOTSFORD'
SCHOOLKEY 'X School School' -> 'X'
```

### E2 — `sanitize_person` (`run.py:78-89`)
```
PERSON 'Ann Greenfield (Athletic Director)' -> 'Ann Greenfield'
PERSON 'Athletic Director' -> ''
PERSON 'Principal Laurie Kolling' -> ''
PERSON 'Dean Adams' -> 'Dean Adams'
PERSON 'Dean of Students' -> ''
PERSON 'Jane Doe, Head Coach' -> 'Jane Doe'
PERSON 'Coach' -> ''
PERSON 'John  Smith' -> 'John Smith'
PERSON 'Ann Greenfield (assistant coach)' -> 'Ann Greenfield'
PERSON 'Nurse Pat' -> ''
PERSON 'Ann Greenfield (AD)' -> 'Ann Greenfield (AD)'
```
(An earlier run of the same probe additionally showed `'Dean, Jane' -> ''`, `'Coach Bob Smith' -> 'Coach Bob Smith'`,
`'John Smith Head Coach' -> 'John Smith'`, `'Head Coach' -> ''`, `'Assistant Coach' -> ''`,
`'Mary Jones (Head Coach)' -> 'Mary Jones'`, `'Director of Athletics' -> ''`, `'The Coach' -> ''`,
`'Bob Smith, Athletic Director' -> 'Bob Smith'`, `'Jose Ruiz (Assistant Coach)' -> 'Jose Ruiz'`,
`'Jane Doe (Coach)' -> 'Jane Doe'`, `'Dean' -> ''`.)

### E3 — `is_vendor_fixture` (`run.py:65-75`)
```
VENDOR ('NC Test School 1', '') -> True
VENDOR ('DF Test School 1', '') -> True
VENDOR ('Test School', 'x@a.org') -> True
VENDOR ('Contest School', '') -> False          # word boundary: "contest school" is not a fixture
VENDOR ('Madison West High School', 'ad@dragonflyathletics.com') -> True
VENDOR ('Madison West High School', 'X@DRAGONFLYATHLETICS.COM') -> True
VENDOR ('Madison West High School', 'ad@school.org') -> False
VENDOR ('Test  School 2', '') -> True
```

### E4 — `normalize_school` / `clean_text` / `normalize_person` variants
```
NORM 'MADISON WEST' 'MADISON WEST HS' 'ADMIRAL FARRAGUT ACADEMY'
CLEAN 'A B C' '' 'JOHN SMITH'      # clean_text(NBSP run), clean_text(None), normalize_person('john  smith')
```

### E5 — `merge_state` on a synthetic three-source state (`run.py:255-313`)
Input (executed): `src_a` -> schools `[Madison West High School, address 100 Main St, zip 53711]`,
coaches `[John Smith/Track/HeadCoach/Boys/js@x.org, John Smith/AthleticDirector/HeadCoach/Boys,
Fixture Person @ NC Test School 1]`; `src_b` -> `[Madison West HS, address 999 Other Rd, city Madison]`,
coaches `[JOHN SMITH/Track/HeadCoach/Boys, Ann Greenfield (Athletic Director)/Track/HeadCoach/Girls]`;
`src_c` -> `[Madison West High School, zip 53711]`, coaches `[John Smith/Track/HeadCoach/Girls,
Dana Vendor (d@dragonflyathletics.com), No School @ ' ']`.
```
SROW {"address": "100 Main St", "name": "Madison West High School", "sources": ["src_a", "src_c"], "state": "WI", "zip": "53711"}
SROW {"address": "999 Other Rd", "city": "Madison", "name": "Madison West HS", "sources": ["src_b"], "state": "WI"}
CROW {"email": "js@x.org", "gender": "Boys", "person": "John Smith", "role": "HeadCoach", "school": "Madison West High School", "sport": "Track", "state": "WI"}
CROW {"gender": "", "person": "John Smith", "role": "AthleticDirector", "school": "Madison West High School", "sport": "AthleticDirector", "state": "WI"}
CROW {"gender": "Boys", "person": "JOHN SMITH", "role": "HeadCoach", "school": "Madison West HS", "sport": "Track", "state": "WI"}
CROW {"gender": "Girls", "person": "Ann Greenfield", "role": "HeadCoach", "school": "Madison West HS", "sport": "Track", "state": "WI"}
COUNTS 2 2 1 1 3 3 1
EMPTY 0 0 Alaska
```
What this proves, in Python:
* school dedup key is name-based only: `Madison West High School` and `Madison West HS` stay two rows;
* first source wins per field (`100 Main St` beat `999 Other Rd`; later `zip 53711` filled a gap);
* `sources` accumulates every source id that produced a row for the key (`src_a`,`src_c`);
* coach dedup ignores **gender** (`John Smith/Track/HeadCoach` Boys+Girls collapsed, Boys kept) and
  **casing is preserved** (`JOHN SMITH` is a different person key from `John Smith`);
* `sport == "AthleticDirector"` rows collapse to `role=AthleticDirector, gender=""` and keep
  `sport: "AthleticDirector"`;
* fixture rows dropped: `NC Test School 1` school+coach, vendor-email coach (`d@dragonflyathletics.com`),
  whitespace-only school (`' '` -> empty key);
* metrics: `with_address 2, with_city 1, with_zip 1, tf_xc_coaches 3, head_coaches 3, admins 1`;
* an empty state still returns a well-formed row (`EMPTY 0 0 Alaska`).

### E6 — `normalize_person` is dead code
`grep -n normalize_person` across `census-prototype` returns only its definition (`run.py:41-42`); no caller.

### E7 — Rust absence greps
* `ppin|private_school|osm_|openstreetmap|pss_` over `crates/` -> no production hits (only unrelated
  substring matches in fixtures/tests). No PSS/OSM/PPIN enrichment lane exists.
* `varsity|Varsity|Level::|competition_level` -> only competition-level classification for meets, the
  coach-contacts CSV role strings, and `coach_directories/collect.rs:230-232`, which states the
  deliberate choice: *"every track, cross-country and athletic-director row the school publishes is
  emitted, at every level (the census model stores no coach level, so sub-varsity rows are kept rather
  than filtered)"*.
* `Test School|dragonflyathletics.com|555-01|fixture school|sample row` over `crates/census-crawl/src`
  -> no production guard; hits are test fixtures/docs only
  (`census-crawl/tests/fixtures/coach_directories/.../nchsaa_directory_p1.json` contains `NC Test School 1`).

### E8 — Rust source observations used below
`census-domain/src/model/normalization.rs` (`normalize_name` L1, `strip_type_suffix` suffix list at
L23-38, `flip_last_first` L66), `census-domain/src/model_tests/name_tests.rs:92-106` and `:109-128`
(exact expectations quoted in §3), `census-domain/src/model/coach.rs:4-49,66-107`,
`census-domain/src/model/school.rs:31-59`, `census-domain/src/model/natural_key.rs:19`,
`census-store/src/entities/canonical.rs:9-13,27-58,83-110`,
`census-store/src/write_batch/commit.rs:105-113`, `census-store/src/keys.rs:27-30`,
`census-store/src/table.rs:120-123`, `census-crawl/src/coach_directories/{collect.rs:230-232,map.rs}`,
`census-crawl/src/coach_directories/tests.rs:240-266,276-315`,
`census-crawl/src/coach_contacts/{entities.rs,import.rs}`,
`census-crawl/src/plain_names/parse.rs:76-89,105-123`, `census-crawl/src/ciac/pages.rs:46-56`,
`census-crawl/src/milesplit.rs:1-26`, `census-crawl/src/milesplit/wire.rs:82-88`.

---

## 3. Semantics table

Status: `EXISTS` = same rule reachable in Rust; `PARTIAL` = same intent, different key/shape/scope;
`MISSING` = no Rust equivalent.

| # | Semantic | Python ref | Behavior | Rust status | Rust ref | Divergence |
|---|---|---|---|---|---|---|
| S01 | School dedup key | `run.py:30,33-38` | Uppercase, strip word-boundary `HIGH SCHOOL|SENIOR HIGH|SR HIGH|HIGH|SCHOOL` anywhere, non-alnum -> space, fall back to unstripped key when empty | PARTIAL | `normalization.rs:1` `normalize_name`; suffix strip `normalization.rs:~23-38`; id mint `school.rs:31-38` | Different both ways. Python strips bare `HIGH`/`SCHOOL` anywhere (`'Madison High'->'MADISON'`, `'Madison Junior High'->'MADISON JUNIOR'`); Rust strips only a trailing multi-token suffix and never bare `high`, but **does** strip trailing `hs`/`highschool`: `'Abbotsford Highschool'->'abbotsford'`, `'Abbotsford HS'->'abbotsford'` (`name_tests.rs:92-106`) vs Python `'ABBOTSFORD HIGHSCHOOL'`, `'ABBOTSFORD HS'`. Rust then drops all non-alphanumerics in `mint` (`school.rs:33-37`), so `'Madison West High School'` and `'Madison West HS'` collide to one id, while Python keeps two rows (E1, E5). Agreeing cases: `'Center Grove High School High School'`, `'Abbotsford High School'`, `'X School School'` (`name_tests.rs:109-128`). |
| S02 | Person key casing | `run.py:41-42` (dead) | `normalize_person` would uppercase; **it is never called** (E6) | MISSING (dead in Python) | `coach.rs:43` uses `normalize_name(name)` inside the coach id | Rust folds case/diacritics in coach identity; Python keeps raw case, so `'JOHN SMITH'` and `'John Smith'` are two coaches in Python (E5) and one id in Rust. Port must NOT resurrect `normalize_person`; it must reproduce Python's case-preserving key if artifact parity is the goal. |
| S03 | Person sanitization | `run.py:45-52,53-55,60,78-89` | Strip trailing role suffix (`ROLE_SUFFIX`), trim ` ,;:-`, drop post-only (`ROLE_ONLY`, incl. `dean`), drop non-coach leads (`NON_COACH_LEAD`), drop `DEAN_POST` | MISSING (global) / PARTIAL (per adapter) | `plain_names/parse.rs:105-123` `strip_honorific` (leading only), `plain_names/parse.rs:76-89` `OFFICE_ROLE_TOKENS`, `ciac/pages.rs:46-56` `is_placeholder_name`, `coach_contacts/parse.rs` role parsing, `ihsa` `parse_role` -> None for office roles (`ihsa/tests.rs:171-175`) | No Rust code strips a *trailing* glued post (`split_once('(')` has no matches in the crawl crate) and none implements the non-coach-lead/dean rules. Exact Python target behavior is pinned by E2 (18 cases). |
| S04 | Vendor fixture school drop | `run.py:61,65-75` | Drop any school row whose name matches `\btest\s+school\b` | MISSING | no guard in `census-crawl/src` (E7) | Python measured 42 such school rows (docstring `run.py:68-72`); the Rust DragonFly adapter emits every directory row (`coach_directories/collect.rs:230-232`, measured scope `applicability/table.rs:99-103`). |
| S05 | Vendor email drop | `run.py:62,65-75` | Drop a **coach** row whose email ends `@dragonflyathletics.com` (school row survives) | MISSING | none (E7) | Python measured 15 such coach rows, incl. two *real* schools carrying a vendor contact (`run.py:68-72`). |
| S06 | Whitespace/NBSP cleaning | `run.py:92-96` | `[\s\u00a0]+` -> single space, strip; applied to name/school/person/contact fields | PARTIAL | `plain_names/parse.rs:60-72` `clean_text` (decodes `&nbsp;`, collapses whitespace, trims) | Rust cleans inside adapters; no global pass over the merged artifact (Python applies it at mint time: `run.py:264,286,296`). |
| S07 | School row mint + field precedence | `run.py:263-270` | First source to name a key mints `{name, state, sources:[]}`; every later non-empty field fills only if absent; `name`/`state` never overwritten | PARTIAL | `canonical.rs:27-58` `CanonicalSchool::merge` (fill-if-empty for city/association/classification/enrollment/websites, `union_vec` for aliases/identities/evidence, longest-prefix name upgrade) | Same "first non-empty wins" intent, different winner order (S20) and Rust additionally upgrades `name` to the longer prefix-compatible spelling (`canonical.rs:50-52`) which Python never does. |
| S08 | `sources` accumulation | `run.py:266` | Artifact school row lists every contributing `source_id`, in arrival order | MISSING (as a field) | `canonical.rs:56` unions `source_identities`; `canonical.rs:99` unions coach identities | Same information, different shape; artifact parity needs a `source_identities -> sources[]` projection (source-id mapping table needed). |
| S09 | Empty school key dropped | `run.py:261` | `if not key ... continue` | MISSING | `school.rs:40-59` `CanonicalSchool::new` accepts any string; `mint` hashes the compressed normalized name (`school.rs:31-38`) | A whitespace/charset-only school name yields a Python drop but a Rust entity keyed on `""` (E5 shows Python dropping `' '`). |
| S10 | Coach dedup key | `run.py:282` | `(school_key, person, sport, role)` — gender and level excluded | PARTIAL | `coach.rs:35-46` `(school, normalize_name(name), sport_key, gender.stable_key(), role.stable_key())` | Rust key contains **gender**; Python does not. Executed Python proof: Boys+Girls `John Smith/Track/HeadCoach` collapsed to one row with `gender: "Boys"` (E5). Rust produces two canonical coaches for the same input. |
| S11 | AD collapse | `run.py:277-281` | Rows whose **`sport`** == `"AthleticDirector"` are rewritten to `role="AthleticDirector", gender="", level=""` and keyed to one AD row per school+person | EXISTS (equivalent), divergent encoding | `coach_contacts/entities.rs` `ad_entity` (sport `None`, `Gender::Mixed`, `CoachRole::AthleticDirector`); `coach.rs:91-107` `CoachRole::AthleticDirector`; `coach_directories/map.rs` `is_director`; `ohsaa/map.rs:120-135` | Python's trigger is the *sport slot*, not the role: a source printing `sport="Track", role="AthleticDirector"` is NOT collapsed in Python, while Rust always emits an AD row with no sport and (test `coach_directories/tests.rs:276-315`) additionally keeps the person's coaching row (`Grace Hopper|CrossCountry|Boys|Unknown` + `Grace Hopper|none|Mixed|AthleticDirector`). Artifact encodings differ: Python `sport:"AthleticDirector"`, `gender:""` vs Rust `None`/`Mixed`. |
| S12 | Varsity-only detail filter | `run.py:190-194` (comparison at `:193`) | Extras-lane coaches are kept only when `(level or "Varsity") == "Varsity"` | MISSING (deliberate) | `coach_directories/collect.rs:230-232`; test `coach_directories/tests.rs:240-266` emits a `teamLevel: "Junior High"` coach | Highest-blast-radius divergence: the DragonFly lane publishes levels for all 15 registered associations (26,566 coach rows measured, `applicability/table.rs:101-103`), and Rust keeps every level. |
| S13 | Junior-high staff page drop | `run.py:154-160` (docstring) | Detail pages for junior-high staff are dropped: "the census is varsity high-school track and cross country" | MISSING (deliberate) | same as S12; `mshsl` filters by its own coach levels (`mshsl/collect/run.rs:150-151`) | Rust keeps sub-varsity rows where Python dropped whole pages. |
| S14 | `level` carried through merge | `run.py:281` (cleared for AD), `:193` | `level` participates pre-merge (filter) and is cleared on AD rows; not part of the key | MISSING | `census-domain/src/model/coach.rs:4-25` — `CanonicalCoach` has no `level` field | Filtering after the fact is impossible unless the level is persisted (or filtered at adapter emission time). |
| S15 | Contact field first-wins | `run.py:294-296` | `email`,`phone`,`code`,`source_url` fill only when empty, cleaned | EXISTS (fill-if-empty) | `canonical.rs:83-110` coach merge (`set_published_email`, phone fill-if-empty, unions); `coach.rs:70-80` `set_published_email` (professional slot first, then personal) | Rust classifies addresses into professional/personal slots and keeps the first of each kind (`census-store/src/entities/canonical_tests/coach_tests.rs`) where Python keeps one `email` string. Winner order differs (S20). |
| S16 | Sport/gender normalization | `run.py:282,301-310`; per-source maps in the lanes (`integrate_lanes.py:34-41`) | Raw sport strings (`"Track"`,`"CrossCountry"`,`"AthleticDirector"`), gender strings or `""`; metrics count `sport in ("Track","CrossCountry")` | PARTIAL | `coach_directories/map.rs` `team_sport`/`sport_family`; `ciac/pages.rs:3-30`; typed `Sport`/`Gender` enums; family collapse indoor+outdoor -> `"Track"` for dedup only | Rust stores `IndoorTrack`/`OutdoorTrack`/`CrossCountry` (`Sport::stable_key`), Python stores `"Track"`. Artifact comparison needs an explicit family map, otherwise the `tf_xc_coaches` count cannot match. |
| S17 | Jurisdiction labeling | `run.py:264,286`, backfill `run.py:195-197` | Every school **and coach** row carries `state`; detail coaches take `join.state`/hub state | PARTIAL | state lives on `CanonicalSchool` (`school.rs:11`, id via `natural_key.rs`) and reaches coaches only through `SchoolId` (`coach.rs:7`) | Rust has no coach `state` field; artifact rows need the school join. Semantics are otherwise equivalent. |
| S18 | State-label correction | `fix_state_labels.py:26-28` `addr_state`, `:34-44` `fix_records`, `:50-85` | OSM sweep rows are relabelled to `addr_state`; `pass2_*`/`multi_*_sites` rewritten; `school_sites/*.json` relabelled by website | MISSING / N/A | no OSM lane in the Rust tree (E7) | No Rust analog because the Rust census has no OSM sweep. Must be declared out of scope or ported with the lane. |
| S19 | Source precedence | `run.py:336-343,352-354` (selection preserves `SOURCES` order) + `setdefault` | First source in `SOURCES` declaration order wins every contested field; extras appended after stage-one (`run.py:355`), enrichment last (`run.py:373`) | PARTIAL | sequence assigned at write time (`write_batch/commit.rs:105-113`, `keys.rs:27-30`); merge in `canonical.rs:27-58,83-110` | Rust winner order is adapter write order, not a declared priority; two adapters observing the same coach produce the winner by whoever wrote first. |
| S20 | Ordering / determinism | `run.py:396-400` (states sorted, dict insertion order for rows), `:414-430` (write order) | Deterministic given the same inputs: sources in `SOURCES` order, extras in `sorted(EXTRA.glob)`, enriched last | PARTIAL | prefix scan in id order (`keys.rs:27-30`), export in table order (`census-service`) | Not row-comparable; artifact diff must sort. |
| S21 | Empty-state artifacts | `run.py:402-407` | A state with zero rows after merge is skipped (no file, no summary row) | EXISTS (equivalent) | per-adapter reports (`coach_directories/collect.rs:230-232`) + coverage (`census-service/src/census/aggregate.rs`) | Different reporting units; a state can be "present" in Rust with an empty table. Verify against Python's `state-summary.json` only for states that produced rows. |
| S22 | Per-source accept counters | `run.py:99-152` | status/bytes/counts per source; failures recorded as data, rows dropped | EXISTS | `AdapterReport` (`census-crawl`), registry `census-crawl/src/registry.rs` | Metric names/sets differ (`vacancies`, `school_sample`, `evidence` have no Rust counterpart). |
| S23 | MaxPreps lane row hygiene | `integrate_lanes.py:27-32` | Reject person == ""/`Coach`-only/`NULL`/contains digit/`<2` words/`>40` chars | MISSING (global) / PARTIAL (adapter) | `ciac/pages.rs:46-56` placeholder list; milesplit roster rejections (`milesplit/roster.rs:23-36`, `milesplit/parse/roster.rs:67-101`) | Rust has no equivalent digit/word-count/length guard for coach names. |
| S24 | Lane container join shape | `integrate_lanes.py:33-56` | Rows grouped by `(state, school, source_url)` and wrapped in `{url,state,school,status:"ok",title_school,name_match,coaches}` | PARTIAL | Rust adapters emit entities directly (`coach_directories/collect.rs`) plus `PROVENANCE.json` fixtures | Python-only plumbing (a loader-shape adapter); only matters if artifacts are compared field-by-field. |
| S25 | Metrics for the summary | `run.py:297-313,454-459` | `schools, with_address, with_city, with_zip, tf_xc_coaches, head_coaches, admins` per state | PARTIAL | `census-service/src/census/aggregate.rs`, `.../seal/*` | Definitions differ (`admins` = Python `sport == "AthleticDirector"`; Rust counts `CoachRole::AthleticDirector`). |
| S26 | MileSplit school enrichment | `run.py:226-252` (`load_enriched`, joins by name) | Adds street address + per-school indoor/outdoor/XC flags from `out/schools_enriched.jsonl` | MISSING | `milesplit.rs:1-26` exposes meet/roster/team-index/result-set only; `milesplit/wire.rs:82-88` `TeamRef { id, slug, url, name, city_state }` has no address/website | Rust's MileSplit lane yields name + `city_state` only; no address enrichment. |
| S27 | PSS + OSM enrichment merge | `merge_enrichment.py:25-31` (a third `canon` variant: trailing `H S|HIGH SCH|SENIOR HIGH|SENIOR HIGH SCH`, `MID|MIDDLE|MIDDLE SCH`, `JR|JUNIOR|JUNIOR HIGH|JUNIOR HIGH SCH`), `:34-90` | Fill-if-empty `address`(+`address_source="PSS"`), `website`(+`website_source="OSM"`), `lat`, `lon`, `ppin` from `extra/Private-schools-enriched.jsonl` keyed by `(state, canon(name))`; backs up to `.jsonl.bak`; rewrites `schools.jsonl` | MISSING | E7 (no `ppin`/`pss`/`osm` in `crates/`) | Entirely absent in Rust; also note Python ships **three** different school-key functions (`run.py:33`, `merge_enrichment.py:25`, plus the lane's own matching), so parity work must decide which one is canonical. |

---

## 4. Gaps the port must close (ordered by risk)

### G1 — Varsity-only filter for detail/level-carrying lanes (S12, S13, S14) — RISK: HIGH
Python drops every extras-lane coach whose `level` is not `Varsity` (`run.py:193`); Rust keeps them by
explicit design (`coach_directories/collect.rs:230-232`) and the model cannot filter later (no `level`).
Measured blast radius: 26,566 coach rows across 15 associations (`applicability/table.rs:101-103`).
* Proposed home: persist `level: Option<CoachLevel>` on `CanonicalCoach`
  (`census-domain/src/model/coach.rs:4-25`) at emission in
  `census-crawl/src/coach_directories/map.rs` (`coach_entities`), then apply the Varsity filter in the
  artifact view (§4.11) — or, cheapest first step, filter in
  `coach_directories/collect.rs` before `batch.append_many`.
* Acceptance check: fixture `coach_directories/tests.rs:240-266` (staff `teamLevel: "Junior High"`)
  yields **no** coach row when the Varsity filter is enabled, and the two Varsity members of the same
  fixture yield exactly the two rows Python's extras lane would keep.

### G2 — Vendor-fixture rows (S04, S05) — RISK: HIGH
No Rust guard exists for `\btest\s+school\b` school names or `@dragonflyathletics.com` coach emails;
`tests/fixtures/coach_directories/probe/*/directory-1.json` are captured *live* pages, and
`census-crawl/tests/fixtures/coach_directories/.../nchsaa_directory_p1.json` shows `NC Test School 1`
in the live NC payload. Python dropped 42 school + 15 coach rows (docstring `run.py:68-72`).
* Proposed home: artifact view (§4.11) `is_vendor_fixture(school, email)` ported verbatim from
  `run.py:65-75`, applied to exported schools and coaches (school drop keyed on name only;
  coach drop keyed on school name **or** email domain, keeping the school row).
* Acceptance check: table-driven test over the eight executed E3 cases (exact booleans), plus a
  fixture test asserting the exported rows from `probe/*/directory-1.json` contain no name matching
  `\btest\s+school\b` and no `dragonflyathletics.com` address.

### G3 — `sanitize_person` (S03) — RISK: HIGH
* Proposed home: artifact view; port `ROLE_SUFFIX`/`ROLE_ONLY`/`NON_COACH_LEAD`/`DEAN_POST`
  (`run.py:45-60`) + the `strip(' ,;:-')` trim (`run.py:86-89`), applied to the exported coach name
  (and to the identity key used for artifact-level dedup).
* Acceptance check: the 18 executed E2 cases as a table test with byte-exact expected strings —
  including the negative cases `'Ann Greenfield (AD)' -> 'Ann Greenfield (AD)'` (abbreviation NOT
  stripped) and `'Dean Adams' -> 'Dean Adams'` but `'Dean' -> ''`.

### G4 — School identity differs in both directions (S01) — RISK: HIGH
Rust's `normalize_name` + `mint` collapses `HS`/`Highschool`/`Sr High` into the base name while Python
keeps them; Python strips bare `HIGH`/`SCHOOL` anywhere (`'Madison High' -> 'MADISON'`,
`'Madison Junior High' -> 'MADISON JUNIOR'`) while Rust does not. Net effect: school row counts differ
in both directions between the two implementations (E1 vs `name_tests.rs:92-106`).
* Proposed home: do **not** change `normalize_name` (it is the store identity for every entity);
  add an artifact-level `school_key()` in the artifact view reproducing `run.py:33-38` exactly, and
  key exported school aggregation on it.
* Acceptance check: table test over the 15 executed E1 cases with byte-exact Python outputs
  (`'ABBOTSFORD HIGHSCHOOL'`, `'GLENCOE SILVER LAKE HS'`, `'MADISON JUNIOR'`, `'A B HS'`, `'X'`, ...).

### G5 — Coach identity includes gender in Rust (S10) — RISK: HIGH
Executed proof of the Python rule: E5 collapses Boys+Girls to one row. Rust's `CoachId` contains
`gender.stable_key()` (`coach.rs:35-46`), so the same input yields two canonical coaches.
* Proposed home: artifact view — key exported coach rows on
  `(school_key, person, sport family, role)` and keep first-seen gender, mirroring `run.py:282`.
* Acceptance check: fixture with one person listed Boys and Girls for one sport/role exports exactly
  one coach row whose gender is the first source's value.

### G6 — Person casing (S02) — RISK: MEDIUM
Python keys on case-preserved names, Rust folds them into one id
(`'JOHN SMITH'` vs `'John Smith'`, E5). Decide explicitly: artifact view must not fold case if artifact
parity is required; the store may keep its folded identity.
* Acceptance check: two coaches differing only in case export as two artifact rows (Python behavior),
  while the store still reports one canonical coach (documented divergence).

### G7 — Contested-field winner order (S19, S15) — RISK: MEDIUM
Python's priority is the `SOURCES` declaration order (`run.py:336-354`); Rust's is adapter write order
(`write_batch/commit.rs:105-113`).
* Proposed home: artifact view takes an explicit source-priority list (the Rust analog of `SOURCES`)
  and folds per-field with first-non-empty-wins; the store keeps its own merge untouched.
* Acceptance check: two adapters observe different emails for one coach; the exported row carries the
  earlier source's email and the artifact `sources` list contains both ids.

### G8 — Enrichment lanes (S26, S27) — RISK: MEDIUM
PSS address/PPIN, OSM website/coords and MileSplit street address do not exist in Rust (E7);
`TeamRef` carries only `city_state` (`milesplit/wire.rs:82-88`).
* Proposed home: either a `census-service` enrichment pass keyed by the artifact `school_key()` with
  `*_source` labels (`run.py:226-252`, `merge_enrichment.py:34-90`), or an explicit scope decision.
* Acceptance check: artifact school rows for inputs present in the PSS/OSM/MileSplit files carry
  `address`/`website`/`lat`/`lon`/`ppin` with the matching `*_source` values, and fill-if-empty
  semantics (an existing address is never overwritten).

### G9 — Artifact shape/field mapping (S08, S11, S16, S17, S22, S25) — RISK: MEDIUM
Exported Rust rows lack Python's `sources`, coach `state`, `code`, `source_url`, `level`,
`address_source`, `website_source`, `ppin`, and use enum stable keys for sport/gender.
* Proposed home: artifact view emits the Python field set (explicit serde struct), with a documented
  `Sport::stable_key -> python sport string` map (indoor+outdoor -> `"Track"`, XC -> `"CrossCountry"`,
  AD -> `"AthleticDirector"`).
* Acceptance check: golden artifact comparison on one state: exported JSONL sorted by
  `(school, person, sport, role)` equals the Python rows for the same fixture inputs, field by field
  except the explicitly documented divergences (§3).

### G10 — Empty-name school/coach rows (S09) — RISK: LOW-MEDIUM
`CanonicalSchool::new`/`mint` (`school.rs:31-59`) accept empty names and mint an id from `""`; Python
drops the row (`run.py:261`, E5 `' '` case).
* Proposed home: artifact view drops rows whose `school_key`/person is empty (same check as Python).
* Acceptance check: input with a whitespace-only school name produces no exported school and no
  exported coach.

### G11 — Lane-level name hygiene (S23) — RISK: LOW-MEDIUM
`integrate_lanes.py:27-32` rejects `"Coach"`-only, `"NULL"`, digit-bearing, single-word or >40-char
names. Rust has no such guard for coach names.
* Proposed home: artifact view guard (or the specific lane's mapper).
* Acceptance check: table test with the rejected shapes from `integrate_lanes.py:27-32` yields no rows.

### G12 — Do not port dead code (S02) — RISK: LOW (guardrail)
`normalize_person` (`run.py:41-42`) has no caller (E6). Porting it would change identity behavior that
Python never applied; document it as dead and exclude it from the parity checklist.

---

## 5. Open questions

1. **Is artifact-level parity actually required by ADR-015**, or is the Rust store the new source of
   truth with documented divergences? Every gap above is stated as "make the artifact match"; if the
   answer is "no", G1/G4/G5/G6/G7/G9 should be recorded as accepted differences instead of work.
2. **Where should the global rules live?** The Rust design pushes filtering into adapters (office
   tokens, placeholders, roster rejections). Python applies the rules *after* folding all lanes. An
   artifact view keeps the store faithful (evidence + retained conflicts) but adds a second
   convention; adapter-time filtering matches the existing convention but cannot implement S12
   without persisting the level.
3. **Coach `level`**: `coach_directories/collect.rs:231` states the model deliberately stores no
   coach level. Adding it is a domain-model change (plus store rows). Acceptable?
4. **Vendor-email rule vs. real schools**: `run.py:68-72` says two *real* schools carry a vendor
   contact address. Python keeps those schools and drops only the coach row. Confirm Rust should do
   the same rather than dropping the school.
5. **Sport string mapping**: Python counts `sport in ("Track","CrossCountry")` (`run.py:301-309`).
   Rust has `IndoorTrack`/`OutdoorTrack`/`CrossCountry`. Is `IndoorTrack|OutdoorTrack -> "Track"` the
   intended artifact mapping (matching `sport_family` in `coach_directories/map.rs`)?
6. **Which school-key function is canonical** for parity: `run.py:33` (post-merge dedup),
   `merge_enrichment.py:25` (enrichment join), or the lane-specific matchers? They disagree
   (E1 vs `merge_enrichment.py:25-31`).
7. **Byte-exact artifacts**: Python's per-state file concatenates school rows then coach rows with a
   single trailing newline (`run.py:414-418`) and writes `report.json`/`state-summary.json` with
   `indent=2`. Is byte-exactness in scope, or is row-set equality enough?
8. **Empty states**: Python omits a state with zero merged rows (`run.py:402-407`). Should the Rust
   export omit them too, or keep a coverage row (current behavior)?

---

## 6. Limits of this report

* No Rust build/test was executed, so every Rust status is a **source-derived** claim. The strongest
  Rust evidence is quoted test expectations (`name_tests.rs:92-128`, `coach_directories/tests.rs:240-315`).
* Python findings are executed (E1–E6) but on synthetic inputs, except `normalize_school`/
  `sanitize_person`/`is_vendor_fixture` tables which are pure-function probes of real lane values.
* Row-count impact estimates (42 school rows, 15 coach rows, 26,566 DragonFly coach rows) are quoted
  from the Python docstring (`run.py:68-72`) and the Rust applicability note
  (`applicability/table.rs:99-103`); neither was re-measured.
* `census-service` export code was not audited line-by-line; it is named only as the proposed home
  for artifact-level rules.
