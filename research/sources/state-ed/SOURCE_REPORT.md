# state-ed source report — reader contract, state survey, fetcher spec

Scope: make state education agency (SEA) directory input **repo-regenerable** and bindable into the
`school-address` generation / `school-address-join` pipeline (bead `athletic-rust-pipeline-7lx.5`).
Evidence + spec only; no production code was edited. Written 2026-10-09 by agent
`NcesStateEdEvidence`.

Reader owner files (read-only for this report): `crates/census-crawl/src/state_ed/{mod,fields,parse,tabular}.rs`
and the scaffold README `crates/census-crawl/src/state_ed/README.md`.

---

## 1. Reader contract (what the pipeline actually consumes)

Three entry points, selected by CLI flags on `census-service school-address`
(`crates/census-service/src/school_address/mod.rs:42-58`):

| Flag | Parser | Input shape | Identity produced |
|---|---|---|---|
| `--state-ed-index <page>` (repeatable) | `state_ed::parse_index` | NYSED-style index HTML | `IdentifiedKey::StateRecord { state: NY, id }` per row (`parse.rs:70-140`) |
| `--state-ed-profile <page>` (repeatable) | `state_ed::parse_profile` | NYSED profile HTML | `IdentifiedKey::StateRecord { state: NY, id }` (`parse.rs:142-163`) + phone/website/address/enrollment from `parse/details.rs` |
| `--state-ed-tabular <file>` (repeatable) | `state_ed::parse_tabular` | **CSV text only** | `WeakKey` — no cross-run identity (`entry.rs:78-96`) |

### 1.1 NYSED HTML path (index + profile)

Regexes the capture must satisfy (`crates/census-crawl/src/state_ed/fields.rs:1-7`):

```text
INDEX_ROW      <div class="title"><a href="profile\.php\?instid=([^"]*)">([^<]*)</a></div>
PROFILE_TITLE  <title>([^|]+) \| NYSED Data Site</title>
INSTITUTION_ID <strong>INSTITUTION ID: </strong>([^<\r\n]*)
PHONE          <strong>PHONE: </strong><a href="tel:\+1([0-9]{10})"
WEBSITE        <strong>WEBSITE: </strong><a href="([^"]+)"
MAPS_QUERY     maps/embed[^"]*?q=([^"&]+)
TOTAL_STUDENTS id="total_students"[^>]*>(?:\s*<span[^>]*>)?\s*([0-9]+)
```

Limits and refusals: body ≤8 MiB (`parse.rs:165-175`); index stops at 20,000 rows (`parse.rs:95-104`);
an index with zero matches is `DirectoryError::Representation` ("NYSED index has no published school
rows", `parse.rs:80-92`); an id-less or title-less page is refused as "not a school profile"
(`parse.rs:177-192`). The address comes from the percent-decoded `maps/embed…q=` value split into
street, city, state, zip (`parse/details.rs:49-117`). This is the only path that yields a
**state-ed-owned identity**; the join's authority mapping is
`(StateEducationAgency{..}, StateRecord{..}) → "state-ed"` (`join/support.rs:90-110`), and the lane
token is `state-ed` (`crates/census-domain/src/school_directory/ledger.rs:24-31`,
`census-service/src/school_address/join/lanes.rs:8-10`).

Live entry point verified today: `https://data.nysed.gov/` returns `text/html`, title
`NYSED Data Site`, and links using the same `?instid=` shape (`/archive.php?instid=800000081568`);
enrollment pages are on the 2025 cycle. Content type/HTML only — no CSV bulk directory exists on
this host. [live 2026-10-09]

### 1.2 Tabular path (CSV only) — exact requirements

* Engine: comma-delimited, RFC 4180 quoting, strict UTF-8, 512 columns / 1 MiB per record
  (`crates/census-crawl/src/directory/artifact/decoded.rs:4-5`); header names trimmed, BOM-stripped,
  uppercased (`artifact.rs:18-40`).
* **Required literal headers** `TABULAR_REQUIRED = ["NAME","CITY","STATE"]`
  (`state_ed/tabular.rs:9`, re-exported `state_ed/mod.rs:8`; gate at `artifact.rs:46-54`). The alias
  lists below only affect *value* selection — a file without the literal `NAME` column is refused
  with `the state education agency directory file has no NAME column`, which aborts the lane
  (`read.rs:138-142`).
* Value columns: name ← `NAME|SCHOOL_NAME|SCHOOL`; city ← `CITY|TOWN` (city is a **required value**:
  a missing/empty city is a field failure, `tabular.rs:33-38`); state ← `STATE|STATE_CODE` (must be
  a census-scope jurisdiction or the row is skipped, `tabular.rs:58-79`); street ←
  `STREET|ADDRESS|ADDRESS1`; zip ← `ZIP|ZIP_CODE|POSTAL_CODE` — **a `ZIP4` column is never read**
  (`tabular.rs:89-97` sets `plus4: ""`; fold plus-4 into ZIP beforehand); phone ←
  `PHONE|PHONE_NUMBER`; website ← `WEBSITE|URL`; enrollment ← `ENROLLMENT|NUMSTUDS`
  (`tabular.rs:102-141`, aliases at `:114`, `:122`, `:133`).
* Budget: `ReadBudget::Page` — ≤8 MiB text, ≤20,000 data rows, ≤65,536 source rows **per lane**
  (`crates/census-crawl/src/directory/budget.rs:9-28`). Repeat `--state-ed-tabular` per shard to
  exceed this; shard files are separate lanes.
* Semantics: rows become `SchoolDirectoryEntry::weak(name, city, Some(state), SEA{state})`
  (`tabular.rs:39-47`), i.e. `DirectoryKey::Weak` (`census-domain/src/school_directory/entry.rs:78-96`).
  Weak entries are **absorbed into a strong (CCD/PSS) entry** only when their weak key (name, city,
  state) matches exactly one strong entry (`census-domain/src/school_directory/collapse.rs:66-112`);
  they can enrich address/phone/website/enrollment but can never own a state-ed claim, because the
  join only binds claims to `IdentifiedKey`s (`join/support.rs:90-110`, `link.rs:28-65`) and read.rs
  only records identified keys in `captured` (`read.rs:158-164`). Any operator expecting
  `--state-ed-tabular` to create owned state ids is wrong; use the index/profile path for that.

---

## 2. State survey — do states publish a bulk directory in the shape the reader consumes?

Survey of 8 representative states with existing probe evidence (capture dates 2026-10-04/05, all
recorded URL + HTTP status + bytes + `sha256` in
`research/sources/coach-coverage-bundle-20261004/probes/<id>/{manifest.json,rows-meta.json}`).
"Reader OK?" = consumable by `--state-ed-tabular` **as downloaded**.

| State | Canonical URL (durable page → file) | Format | Recorded capture (bytes / sha256) | Reader OK? |
|---|---|---|---|---|
| WA | `https://eds.ospi.k12.wa.us/DirectoryEDS.aspx` | HTML table (319 LEA rows, 299 contacts) | 571,249 B / `0d2c3d3fe7a7b27f773746ff63f2f22c010192de694d77a999b430be9cefcf2e` (SRC-143 `rows-meta.json`) | No — HTML only |
| NV | `https://doe.nv.gov/school-and-district-information` → `https://webapp-strapi-paas-prod-nde-001.azurewebsites.net/uploads/school_directory_9b69a05740.xlsx` | XLSX, sheet `NDE Public School List SY26-27`, 782 rows, Name/City/State columns | landing 239,943 B / `9323cc914aef32edb1d113829fc0959a46addde2fd556646c5c9d35fb116888b`; xlsx 302,305 B / `4bd8907ff64225a70ed9e75b164d91769ca37cc83fd7ece272f18de68eb9adf2` (SRC-176 `manifest.json`) | No — convert XLSX→CSV; upload file name is hash-suffixed and rotates (resolve from the landing page) |
| IN | `https://www.in.gov/doe/it/data-center-and-reports/` → `https://www.in.gov/doe/files/2025-2026-school-directory-2026-03-23.xlsx` | XLSX, sheets `SCHL` (1,962) / `NPSCHL` (420) / `CORP` (460); CITY/STATE/ZIP/PHONE/HOMEPAGE present | 447,924 B / `f750228cd81eecf99ce92b5aa7a3e014a7e4e2de68e2229163fdb6086e4221b0` (SRC-113 `rows-meta.json`); URL re-verified today (200, xlsx; CORP sheet spot-read) | No — convert XLSX→CSV; the file name carries a date stamp (rotates) |
| IL | `https://www.isbe.net/Pages/Data-Analysis-Directories.aspx` → `https://www.isbe.net/Documents/2025-26-Directory-Ed-Entities.xlsx` (robots disallows `/_layouts/`, so use the `/Documents/` path) | XLSX, 7 entity sheets; FacilityName/City present, state fixed IL | 1,454,854 B / `75d14e57a40f164504c6a9a63d1a376708851cbe32b5e5331699bf52cf7fd6a7` (SRC-122 `rows-meta.json`, two identical URLs) | No — convert XLSX→CSV and inject the literal `STATE` column |
| OH | `https://oeds.education.ohio.gov/DataExtract/GetRequestOrgExtract` (advertised from `https://oeds.education.ohio.gov/dataextract`) | **CSV** 28,536 rows, header on physical line 2; city embedded in `ORG MAILING ADDRESS` (`"<street>, <City>, Ohio, <ZIP>"`); columns `ORGANIZATION NAME`, `ORGANIZATION TYPE`, e-mail columns | 8,471,875 B / `360a3738fd4908994b63875995c6b302e68dd15959ae92b115f3891d9299c2fc` (SRC-101 `rows-meta.json`; a parameterless capture also returned HTTP 500 at 14,695 B) | **No, twice**: (a) 8,471,875 B > 8 MiB Page budget → `Capacity`; (b) no literal `NAME`/`CITY` headers → `the state education agency directory file has no NAME column`. Requires a derived, sharded CSV |
| NY | `https://data.nysed.gov/` (index/profile pages, `?instid=`) — the reader's native shape; institutional directory also at `https://www.oms.nysed.gov/sedref/home.html` and `https://www.p12.nysed.gov/irs/schoolDirectory/` | HTML | SEDREF home 20,829 B / `2eedfed3ef3ed75e40628fab0f293d6d353bc116abf797a318bc69c2569402f5`; p12 school directory 15,419 B / `74475582fd4f9585ebc02ee6becd619e6eeb8bac7f4b3bea774664b9c77d2e77` (SRC-049 `manifest.json`); portal query app only 302s (`…/sedrefpublic/SED.sed_inst_qry_vw$.startup`) | Yes, via `--state-ed-index` + `--state-ed-profile` captures; no bulk CSV exists |
| KS | `https://uapps.ksde.gov/directory_rpts/default.aspx` (hub in the source guide: `https://ksde.gov/data-and-reporting/directories`) | ASP.NET form; default view has 0 data rows; a POSTed "excel" report returned 4,852 B `text/html`, not a workbook | index 259,943 B / `6ac7f323ff56bfc640bf14acb111808e738fc3f335d43727ec87c66f58748fe8` (SRC-239 `manifest.json`) | No — interactive form; needs a posted report and its own capture contract |
| PA | `https://www.edna.pa.gov/Screens/Extracts/wfExtracts.aspx` (+ `wfExtractEntitiesAdmin.aspx`, `wfExtractRelatedEntities.aspx`) | ASP.NET extract portal; robots 404 (default-permit under RFC 9309) | extracts landing 10,405 B / `21f9293bcd017a089cb015c957bb9de06421cec7ac9255b2acf7166ebd7d70dd`; entities page 47,434 B / `ebd8cae4ef26ccf72ce3b308d55a8d59288cee871a3d3ea3655c0694fef17f3d` (SRC-230 manifest) | No — posted extract; no static bulk file |

**Finding.** None of the eight publishes a static UTF-8 CSV whose header is the literal
`NAME,CITY,STATE` trio, so `--state-ed-tabular` cannot consume any of them *as downloaded*. Three
(NV, IN, IL) publish XLSX with all three values present (convertible); OH publishes real CSV but with
a different schema and over budget; WA/NY/KS/PA are HTML (NY is consumable through the
index/profile path, the others are form- or page-driven). The general rule the fetcher must assume:
**the SEA bulk path is an operator conversion whose provenance is the converted file**, and the
original (XLSX/HTML) capture must be recorded alongside it.

HTML-only findings to file as such: WA `eds.ospi.k12.wa.us` (served table; RCW 42.56.070(8)
restriction notice on the page), NY `data.nysed.gov` (HTML profiles; reader-native), KS
`uapps.ksde.gov` (form), PA `www.edna.pa.gov` (form). Supplemental machine-readable variants exist
(e.g. WA OSPI ArcGIS layer captured by SRC-144; IA ArcGIS `IowaSchoolBldgs` FeatureServer in the
source guide) but they are JSON, not the tabular reader's shape.

---

## 3. Fetcher spec for `state-ed` lanes

### 3.1 Path A — NYSED HTML (owned state-ed identities)

```bash
UA='census-service/0.1 (independent HS track & field research collector; polite; contact: repo owner)'
D=research/sources/state-ed/downloads/nysed
mkdir -p "$D"
sleep 1.1; curl -fsSL --max-time 120 -A "$UA" -o "$D/index.html"       'https://data.nysed.gov/…index route…'
sleep 1.1; curl -fsSL --max-time 120 -A "$UA" -o "$D/profile-<instid>.html" 'https://data.nysed.gov/profile.php?instid=<instid>'
sha256sum "$D"/*.html > "$D/SHA256SUMS"
```

* The index route must contain `profile.php?instid=` links matching `fields::INDEX_ROW`; one capture
  per page, repeat `--state-ed-index` per page (20,000-row cap each).
* Profiles are one file per school, captured with the id visible in the URL; they must carry both
  `<title>… | NYSED Data Site</title>` and `INSTITUTION ID:` or they are refused.
* Verify before wiring: `grep -c 'profile\.php?instid=' index.html` > 0;
  `grep -c 'NYSED Data Site' profile-*.html` ≥ 1; file sizes ≤8 MiB.
* Wire: `--state-ed-index <page>… --state-ed-profile <page>…`, then join with
  `--evidence-url state-ed@<capture path>=https://data.nysed.gov/… --evidence-date state-ed@<capture path>=YYYY-MM-DD`
  per capture (or one `--evidence-url state-ed=… --evidence-date state-ed=…` when the generation holds
  a single state-ed capture — the selector form is required when it holds several, see
  `join/lanes.rs:104-157`; the value must be http/https and the date `YYYY-MM-DD` or RFC3339, or the
  join errors before reading anything).

### 3.2 Path B — SEA tabular (enrichment only)

1. Download the current workbook/page for the state (URLs in §2); keep it under
   `research/sources/state-ed/downloads/<state>/` with a `SHA256SUMS`.
2. Convert to CSV **outside** the pipeline, e.g. `libreoffice --headless --convert-to csv --outdir
   <dir> <file.xlsx>` (or `ssconvert`/`in2csv` if present — the converter must be named in the
   provenance note, because the reader never sees the XLSX). Select the school-level sheet only
   (e.g. IN `SCHL`/`NPSCHL`, IL entity sheets; never the corporation/district sheets).
3. Normalise the header to the literal trio and fold values into the alias columns:
   `NAME` (school name), `CITY`, `STATE` (two-letter, census scope); optional `ADDRESS`, `ZIP`
   (plus-4 folded in), `PHONE`, `WEBSITE`, `ENROLLMENT`. Drop everything else. Fix the state when the
   source sheet lacks it (IL → `IL`, IN → `IN`; never guess a different state).
4. Shard so each file is ≤8 MiB and ≤20,000 data rows (IN ~2,800 rows and IL ~7,500 rows fit; a
   TX/CA-scale directory will not). One `--state-ed-tabular` flag per shard.
5. Verify each shard cheaply before wiring: `head -1 f.csv | tr ',' '\n' | grep -x -E 'NAME|CITY|STATE'`
   returns all three; `wc -l` ≤ 20,001; `stat -c %s` ≤ 8,388,608; every row's STATE cell is a
   two-letter census jurisdiction.
6. Wire: `--state-ed-tabular <shard>`… and give the lane the **original** URL as its provenance:
   `--evidence-url state-ed=https://… --evidence-date state-ed=YYYY-MM-DD` (the recorded capture
   sha256 in the lane is the converted CSV, so the evidence note will read
   `state-ed lane <csv path> sha256=<csv digest> generation <digest>` — record the conversion command
   and the source workbook digest in the conversion log next to the shards).

### 3.3 How the run fails loudly when inputs are absent or wrong

| Situation | Behaviour (exact strings) |
|---|---|
| No artifact flags at all | `no input artifact was named: pass at least one of --ccd, --pss, --state-ed-index, --state-ed-profile, --state-ed-tabular, --associations or --association-directory` (`read.rs:62-67`) |
| XLSX/PDF/ZIP handed to `--state-ed-tabular` | read-to-string fails (`stream did not contain valid UTF-8`) before any parsing (`read.rs:200-207`) |
| Header lacks `NAME`/`CITY`/`STATE` | `the state education agency directory file has no NAME column` → lane aborts (`artifact.rs:46-54`, `read.rs:138-142`) |
| File >8 MiB / >20,000 rows | decoder byte cap at `artifact.rs:113`; source-row cap `DirectoryError::Capacity { resource: "directory CSV source rows", … }` (`artifact.rs:160-165`) → lane aborts (`budget.rs:9-28`) |
| Truncated capture | `directory capture <path> stopped at row <n>: …; no corpus is published` or `has no completed parser frontier` (`read.rs:152-156`) |
| Lane without URL/date at join time | `school-address lane state-ed@<path> requires its published URL and actual acquisition date` (`join/generation.rs:39-41`), else `provider state-ed needs a capture URL and an observation date; pass --evidence-url state-ed=URL --evidence-date state-ed=YYYY-MM-DD, or name one capture as state-ed@<path>=…` (`join/link.rs:61-71`) → counted as `evidence_missing`, never fabricated |
| Selector names a capture the generation lacks | `no capture in this generation matches "state-ed@<path>" …` (`lanes.rs:148-155`) |

### 3.4 Expectations to encode

* **state-ed tabular lanes cannot own identities** (§1.2). If owned state records are required, the
  NYSED index/profile path is the only admitted shape today; anything else needs a reader change,
  which must be bead-scoped rather than smuggled in as "just a CSV".
* Served robots status (for the record only — policy ignored per owner directive, spacing retained):
  NV/NDE 200 policy (SRC-176), IL `www.isbe.net` disallows `/_layouts/`, KS and PA robots 404
  (default-permit). Keep ≥1s spacing and the identifying UA in every capture regardless.

---

## 4. Limits of this report

* Hashes/digests in §2 are quoted from the named probe manifests (`rows-meta.json`/`manifest.json`,
  captured 2026-10-04/05); only the IN URL and `data.nysed.gov` were re-fetched today, and those two
  fetches prove reachability and content type, not a digest match.
* The survey covers the eight states with existing probe evidence; it is not an exhaustive 50-state
  scan. Any state added later follows the same verdict procedure: fetch the page once, then classify
  as HTML / XLSX / CSV-with-mismatch / form, and convert only where the trio can be produced.
* Conversion commands in §3.2 are unexecuted here (no shell); they are the spec for the coder, not a
  verified run. Nothing was downloaded under `research/sources/state-ed/downloads/` in this session.
* `crates/census-crawl/src/state_ed/README.md` is still the scaffold text (no real URLs/robots status
  recorded); that file is outside this report's ownership and was not edited.
