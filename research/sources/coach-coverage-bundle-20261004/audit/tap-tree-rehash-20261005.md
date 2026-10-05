# Tap-tree digest and structural audit - 2026-10-05

## TASK

Read-only re-hash and structural audit of every delivered `probes/` and `extract/` tree present in the inventory snapshot, including `probes/sources/*`. Executor: **NjTap-2**, active `run_shell` context, repository cwd. No network requests, store opens, crate changes, bead claiming/closing, commits or shared-ledger edits.

## OWNERSHIP / DO NOT MODIFY

Only this report is written: `research/sources/coach-coverage-bundle-20261004/audit/tap-tree-rehash-20261005.md`. All inspected source trees, manifests, records and captures remain read-only. Main owns the shared ledger and repairs. This auditor previously authored `probes/nj-njsiaa`; its bytes were mechanically re-hashed here, but its authorship is **not independent**. All other trees are independent of this auditor's tap work.

## INPUT CONTRACT / METHOD

Read `SHA256SUMS*`, `manifest.json`, `sweep.json`, `derivation.json`, `execution.json`, `readback-summary.json`, the files they name, and tree reports/inventories. The existing `verify-captures.sh` was read but **not executed**: it writes temporary files, recognizes fewer schemas, and skips missing named files. No worker checksum claims were accepted without an active `sha256sum` invocation.

Inventory/hash snapshot: **2026-10-05T01:43:03.146153+00:00 to 01:43:03.535082+00:00**. Additional unclaimed-file hashing and structural attribution ran afterward. The bundle was not frozen; concurrent/future deliveries are outside this snapshot. Per-file size and mtime were compared before/after the named-file hashing; no in-command changes were reported. There is no transactional whole-tree snapshot guarantee.

Digest resolution rules: tree-relative paths, explicit repository/bundle-relative paths, and sibling-prefix paths from sweep records; explicit `..` is honored literally, not silently repaired. Flat manifests with no filename resolve only to their **existing uniquely named `<digest>.body`** artifact. Duplicate declarations of the same resolved filename+expected digest are counted once, including repeated per-row provenance citations. A different digest for the same path would be counted separately. Four initially unbound checksum occurrences were identified as corroborating duplicates, not extra files: Indiana's XLSX trace, the two SNTCCCA `capture_file` declarations, and Wellesley's excerpt verification.

`Claimed` below means unique resolved filename/expected-digest pairs in published machine-readable records, not distinct digest values or duplicate occurrences. `OK` means the actual `sha256sum` equals the published digest. Undigested/unexplained means raw/derived files with no published digest / those also not mentioned by filename in the tree's records or reports. Candidate data includes raw/, captures/, robots/, sweep/, and top-level CSV/JSON/body data, not all narrative/diagnostic scaffolding. Unclaimed candidates were also hashed, but no hash-match claim is possible without a published expected value.

Structural findings are **tree-local artifact presence**, not newly executed import/readback proof. Narrative/embedded records are distinguished from retained raw logs. Negative prose can mention a command without proving it ran; therefore the presence scan is not an acceptance verdict. Paired probe/extract trees and external ledger/store evidence are not silently credited across trees. A refused/seed-only probe can legitimately omit coach import; listed gaps identify missing tap evidence, not an instruction to bypass its access boundary.

## OUTPUT CONTRACT / OBSERVED TOTALS

- Trees: **82**.
- Published unique file/digest pairs: **394** = **380 OK + 13 MISMATCH + 1 MISSING**.
- Existing named physical files hashed: **393**. Additional unclaimed raw/derived files hashed: **108**. These are hash-operation categories, not a deduplicated global physical-file total.
- Undigested raw/derived files: **108**, of which **98** are also neither listed nor explained by filename; the other **10** are explained but lack a published digest.
- Final structural classification: **3 OK**, **57 STRUCTURAL_GAPS**, **22 REQUALIFY**.
- Declared capture dates: **70 trees** publish 2026-10-04/05 capture dates; **12** have undocumented capture dates. **Zero** publish an older capture date in the fields inspected. Historical source content dates are not treated as capture dates.
- All **13 mismatches** are named below with full expected and actual digests. No mismatch is hidden or classified as a successful match.

Count reconciliation examples: NJ publishes **13 raw capture entries**, all 13 matching, plus 3 bound derived-file digests = **16/16 total bound claims**. IA's SHA256SUMS contains **16 files**, including 4 raw responses plus derived/report/CLI artifacts = **16/16**. Indiana's current manifest publishes **3 raw capture entries** and 6 total bound claims = **6/6**. Repeated row citations are not additional files. A raw-capture count and a whole-tree checksum count are different denominators; no unsupported 17-of-16 claim is inferred.

### Gap codes

- **M**: no `manifest.json` (some trees have a `sweep.json` substitute).
- **D**: no published machine-readable checksum records.
- **U**: raw/derived data not covered by a published digest; exact filenames below.
- **I**: no import/provider record detected in the tree's artifact/record scan.
- **R**: no readback record detected in that scan.
- **L**: import/readback narrative or embedded JSON exists, but no runs/, cli/, evidence/, readback/ equivalent raw logs are retained in this tree.
- **P**: no provenance trace found in the scanned records.
- **N**: provenance narrative only, no separate/embedded machine-readable source mapping detected.
- **F**: no REPORT.md or FINDINGS.md.

`04/05` = declared capture date 2026-10-04 or 2026-10-05; `unknown` is not proof of an older capture. **REQUALIFY** means digest/path/undocumented-data reconciliation is required before claiming this tree as self-contained tap evidence. It does not automatically require a live refetch.

## Per-tree table

| Tree | Claimed | OK | Mismatch | Missing | Undigested/unexplained | Gaps | Date | Verdict |
| --- | ---: | ---: | ---: | ---: | ---: | --- | --- | --- |
| extract/097-oh-ohsaa-seed | 3 | 3 | 0 | 0 | 1/0 | U,I,R,N | 04/05 | STRUCTURAL_GAPS |
| extract/107-mi-eem | 9 | 9 | 0 | 0 | 1/0 | U,I,R,P | 04/05 | STRUCTURAL_GAPS |
| extract/130-ia-doe | 0 | 0 | 0 | 0 | 1/1 | M,D,U,I,R,P,F | unknown | REQUALIFY |
| extract/131-ia-arcgis | 0 | 0 | 0 | 0 | 2/2 | M,D,U,I,R,P,F | unknown | REQUALIFY |
| extract/184-va-vdoe-directory | 2 | 2 | 0 | 0 | 1/0 | U,I,R,P | 04/05 | STRUCTURAL_GAPS |
| extract/185-va-visaa | 2 | 2 | 0 | 0 | 1/0 | U,I,R,P | 04/05 | STRUCTURAL_GAPS |
| extract/186-va-vhsl | 1 | 1 | 0 | 0 | 1/1 | U,I,R,P | 04/05 | REQUALIFY |
| extract/204-ok-directory | 0 | 0 | 0 | 0 | 2/2 | M,D,U,I,R,P,F | unknown | REQUALIFY |
| extract/213-la-bese-nonpublic | 2 | 2 | 0 | 0 | 1/0 | U,I,N | 04/05 | STRUCTURAL_GAPS |
| extract/223-sd-edudir | 0 | 0 | 0 | 0 | 2/2 | M,D,U,I,R,P,F | unknown | REQUALIFY |
| extract/228-sdhsca-membership | 0 | 0 | 0 | 0 | 1/1 | M,D,U,I,R,P,F | unknown | REQUALIFY |
| extract/230-pa-edna | 0 | 0 | 0 | 0 | 2/2 | M,D,U,I,R,P,F | unknown | REQUALIFY |
| extract/231-paisaa-members | 0 | 0 | 0 | 0 | 3/3 | M,D,U,I,R,P,F | unknown | REQUALIFY |
| extract/239-ks-directory | 0 | 0 | 0 | 0 | 0/0 | M,D,I,R,P,F | unknown | STRUCTURAL_GAPS |
| extract/SRC-036 | 7 | 7 | 0 | 0 | 0/0 | M,I,R,N | 04/05 | STRUCTURAL_GAPS |
| extract/SRC-054 | 6 | 4 | 2 | 0 | 0/0 | M,L,N | 04/05 | REQUALIFY |
| extract/SRC-066 | 19 | 19 | 0 | 0 | 0/0 | M,I,R,N | 04/05 | STRUCTURAL_GAPS |
| extract/SRC-068 | 11 | 9 | 2 | 0 | 0/0 | M,R,N | 04/05 | REQUALIFY |
| extract/SRC-100 | 11 | 8 | 3 | 0 | 0/0 | M,R,P | 04/05 | REQUALIFY |
| extract/SRC-106 | 3 | 3 | 0 | 0 | 0/0 | M,I,R,N | 04/05 | STRUCTURAL_GAPS |
| extract/SRC-113-in-doe | 6 | 6 | 0 | 0 | 0/0 | none | 04/05 | OK |
| extract/SRC-114 | 2 | 2 | 0 | 0 | 0/0 | M,I,R,N | 04/05 | STRUCTURAL_GAPS |
| extract/SRC-175 | 7 | 5 | 2 | 0 | 0/0 | M,L,N | 04/05 | REQUALIFY |
| extract/SRC-178 | 2 | 2 | 0 | 0 | 2/0 | M,U,I,R | 04/05 | STRUCTURAL_GAPS |
| extract/SRC-179 | 2 | 2 | 0 | 0 | 2/0 | M,U,I,R | 04/05 | STRUCTURAL_GAPS |
| extract/SRC-212 | 3 | 3 | 0 | 0 | 0/0 | M,I,R,N | 04/05 | STRUCTURAL_GAPS |
| extract/SRC-215 | 4 | 4 | 0 | 0 | 0/0 | M,I,R,N | 04/05 | STRUCTURAL_GAPS |
| extract/SRC-222 | 5 | 3 | 2 | 0 | 0/0 | M,L,N | 04/05 | REQUALIFY |
| extract/SRC-266 | 3 | 3 | 0 | 0 | 0/0 | M,L,N | 04/05 | STRUCTURAL_GAPS |
| extract/SRC-267 | 3 | 3 | 0 | 0 | 0/0 | M,L,N | 04/05 | STRUCTURAL_GAPS |
| extract/SRC-268 | 3 | 3 | 0 | 0 | 0/0 | M,L,N | 04/05 | STRUCTURAL_GAPS |
| extract/SRC-269 | 3 | 3 | 0 | 0 | 0/0 | M,L,N | 04/05 | STRUCTURAL_GAPS |
| extract/SRC-270 | 3 | 3 | 0 | 0 | 0/0 | M,L,N | 04/05 | STRUCTURAL_GAPS |
| extract/SRC-271 | 3 | 3 | 0 | 0 | 0/0 | M,L,N | 04/05 | STRUCTURAL_GAPS |
| probes/097-oh-ohsaa-seed | 3 | 3 | 0 | 0 | 0/0 | I,R,N | 04/05 | STRUCTURAL_GAPS |
| probes/107-mi-eem | 9 | 9 | 0 | 0 | 0/0 | I,R,P | 04/05 | STRUCTURAL_GAPS |
| probes/122-il-isbe | 0 | 0 | 0 | 0 | 55/55 | M,D,U,I,R,P,F | unknown | REQUALIFY |
| probes/130-ia-doe | 5 | 5 | 0 | 0 | 0/0 | I,R,P | 04/05 | STRUCTURAL_GAPS |
| probes/131-ia-arcgis | 9 | 9 | 0 | 0 | 6/6 | U,I,R,P | 04/05 | REQUALIFY |
| probes/184-va-vdoe-directory | 2 | 2 | 0 | 0 | 0/0 | I,R,P | 04/05 | STRUCTURAL_GAPS |
| probes/185-va-visaa | 2 | 2 | 0 | 0 | 0/0 | I,R,P | 04/05 | STRUCTURAL_GAPS |
| probes/186-va-vhsl | 1 | 1 | 0 | 0 | 0/0 | I,R,P | 04/05 | STRUCTURAL_GAPS |
| probes/204-ok-directory | 4 | 4 | 0 | 0 | 0/0 | I,R,P | 04/05 | STRUCTURAL_GAPS |
| probes/213-la-bese-nonpublic | 2 | 2 | 0 | 0 | 0/0 | I,N | 04/05 | STRUCTURAL_GAPS |
| probes/223-sd-edudir | 4 | 4 | 0 | 0 | 0/0 | I,R,N | 04/05 | STRUCTURAL_GAPS |
| probes/228-sdhsca-membership | 2 | 2 | 0 | 0 | 0/0 | I,R,P,F | 04/05 | STRUCTURAL_GAPS |
| probes/230-pa-edna | 17 | 17 | 0 | 0 | 17/17 | U,I,R,P | 04/05 | REQUALIFY |
| probes/231-paisaa-members | 0 | 0 | 0 | 0 | 2/2 | M,D,U,I,R,P,F | unknown | REQUALIFY |
| probes/239-ks-directory | 0 | 0 | 0 | 0 | 2/2 | M,D,U,I,R,P,F | unknown | REQUALIFY |
| probes/247-nces-ccd-selector | 5 | 5 | 0 | 0 | 0/0 | I,R,P | 04/05 | STRUCTURAL_GAPS |
| probes/ciac-dragonfly | 0 | 0 | 0 | 0 | 2/2 | M,D,U,I,R,P,F | unknown | REQUALIFY |
| probes/ciac-fpsports | 3 | 3 | 0 | 0 | 0/0 | L,N,F | 04/05 | STRUCTURAL_GAPS |
| probes/edlio_delnorte | 2 | 2 | 0 | 0 | 0/0 | I | 04/05 | STRUCTURAL_GAPS |
| probes/finalsite_starrsmill | 2 | 2 | 0 | 0 | 0/0 | I,N | 04/05 | STRUCTURAL_GAPS |
| probes/gobound | 7 | 7 | 0 | 0 | 0/0 | I,R,P | 04/05 | STRUCTURAL_GAPS |
| probes/googlesites_hopatcong | 2 | 2 | 0 | 0 | 0/0 | L,N | 04/05 | STRUCTURAL_GAPS |
| probes/home_campus | 32 | 32 | 0 | 0 | 0/0 | I,R | 04/05 | STRUCTURAL_GAPS |
| probes/ia-ihsaa | 16 | 16 | 0 | 0 | 0/0 | none | 04/05 | OK |
| probes/iatrackcoaches | 6 | 6 | 0 | 0 | 0/0 | L | 04/05 | STRUCTURAL_GAPS |
| probes/il-ihsa | 5 | 5 | 0 | 0 | 0/0 | L,N,F | 04/05 | STRUCTURAL_GAPS |
| probes/il-ihsa-conference | 2 | 2 | 0 | 0 | 0/0 | I,R,N | 04/05 | STRUCTURAL_GAPS |
| probes/il-ihsa-mobile | 5 | 5 | 0 | 0 | 0/0 | I,R,N | 04/05 | STRUCTURAL_GAPS |
| probes/il_ihsa_records | 7 | 7 | 0 | 0 | 0/0 | I | 04/05 | STRUCTURAL_GAPS |
| probes/kcctfca | 3 | 3 | 0 | 0 | 0/0 | I,N | 04/05 | STRUCTURAL_GAPS |
| probes/ks-kshsaa | 2 | 2 | 0 | 0 | 0/0 | L,N,F | 04/05 | STRUCTURAL_GAPS |
| probes/mascotmedia_rne | 2 | 2 | 0 | 0 | 0/0 | L,N | 04/05 | STRUCTURAL_GAPS |
| probes/nhstfxcca | 3 | 3 | 0 | 0 | 0/0 | L | 04/05 | STRUCTURAL_GAPS |
| probes/nj-njsiaa | 16 | 16 | 0 | 0 | 0/0 | none | 04/05 | OK |
| probes/njxctfca | 2 | 2 | 0 | 0 | 0/0 | I,N | 04/05 | STRUCTURAL_GAPS |
| probes/oh-ohsaa | 4 | 4 | 0 | 0 | 0/0 | L,N,F | 04/05 | STRUCTURAL_GAPS |
| probes/ok-ossaarankings | 6 | 6 | 0 | 0 | 1/0 | U | 04/05 | STRUCTURAL_GAPS |
| probes/pa-piaa | 4 | 4 | 0 | 0 | 0/0 | L,N,F | 04/05 | STRUCTURAL_GAPS |
| probes/schoolowned_wellesley | 3 | 3 | 0 | 0 | 0/0 | I | 04/05 | STRUCTURAL_GAPS |
| probes/sidearm_miramonte | 3 | 3 | 0 | 0 | 0/0 | I | 04/05 | STRUCTURAL_GAPS |
| probes/sources/SRC-058 | 12 | 12 | 0 | 0 | 0/0 | I,R | 04/05 | STRUCTURAL_GAPS |
| probes/sources/SRC-106 | 19 | 18 | 1 | 0 | 0/0 | M,I,R,P | 04/05 | REQUALIFY |
| probes/sources/SRC-113 | 6 | 6 | 0 | 0 | 0/0 | I,R | 04/05 | STRUCTURAL_GAPS |
| probes/sources/SRC-114 | 9 | 8 | 1 | 0 | 0/0 | M,I,R,P | 04/05 | REQUALIFY |
| probes/sources/SRC-128 | 7 | 7 | 0 | 0 | 0/0 | I,R | 04/05 | STRUCTURAL_GAPS |
| probes/sources/SRC-133 | 3 | 2 | 0 | 1 | 0/0 | L | 04/05 | REQUALIFY |
| probes/sources/SRC-198 | 2 | 2 | 0 | 0 | 0/0 | I,R,P | 04/05 | STRUCTURAL_GAPS |
| probes/sources/SRC-203 | 9 | 9 | 0 | 0 | 0/0 | I,R | 04/05 | STRUCTURAL_GAPS |

## Exact mismatch evidence

Paths below are relative to the bundle root. All were recomputed with `sha256sum`; the record pointers identify the published claim. Six stale `sweep.json` entries hash the checksum document itself: updating such a self-containing digest does not establish a stable self-hash. They require an external checksum document or an explicitly excluded self-entry, not silent re-pinning. Five other mismatches are REPORT.md and two are derived CSV data. No named raw response mismatch was observed.

```text
extract/SRC-054/REPORT.md
 expected 3ad4fe8b7f7ca4553aaf104a9415a327c0428c5672c9b80d640b7b92fdc35bbd
 actual   e604900e6ef1ab55e6d112251fd15f755cf820e2ebb851dc64f2c6dd976e11fc
 record   sweep.json/files/0/sha256
extract/SRC-054/sweep.json
 expected 183295ac3ec93d31d8bc73509c506a6053ffe1fae1b2d4c42109931bed926bd7
 actual   08e40f03741688999e00eb472972d2d596f0826268b0acc57debddb2bac8907c
 record   sweep.json/files/5/sha256
extract/SRC-068/REPORT.md
 expected 34a1674e6d2e9663f56933108c7a9566eb4dc80b403a9a506f71be56fee3187c
 actual   c69aab3c20c686fcd8c533fc220a2221ba9992ee5abcb8c3645bb585b143ba35
 record   sweep.json/files/0/sha256
extract/SRC-068/sweep.json
 expected a9332ad5cb52fa5977d50f873a9022bb37a14657f97da6e86088f842a8fffd20
 actual   f801b14b429ba087203578638e584adbe6691ed806add04364eb44874d79dd6d
 record   sweep.json/files/10/sha256
extract/SRC-100/REPORT.md
 expected 84127ad606957fd353a1f0c314958bcfa3075a65b7f09ee4c91b3fb6f735bb79
 actual   6facb05ce3578e8dcd3892ed3ef552440e1e43295fd82b77d652ca19f5d60152
 record   sweep.json/files/0/sha256
extract/SRC-100/excluded_rows.csv
 expected cc6db861a2514f2610184ff711c000d224154b00b813447013afc7e76df961e5
 actual   ec23e97a43aff11452c189d5523d5a4dffbe02373b42964bdb72105fcde6a9ea
 record   sweep.json/files/9/sha256
extract/SRC-100/rows.csv
 expected 2e71e6ea58a5d9ad40b748cfb47931c22a18d177844105a0958c0e77bb0c275b
 actual   e3bd01a0fb5fb19106b53205fe9dc9ec12ff51791830a2f77bafe087ee0403d9
 record   sweep.json/files/10/sha256
extract/SRC-175/REPORT.md
 expected 427578a59d0b577e7147f76d0ee6c5a3a1f9fafeebbef1cf665d24c473f86507
 actual   a3ab9f80b6dbe53ab2d3046558072be5724b54155d6c7356bcffc24a6c973ad4
 record   sweep.json/files/0/sha256
extract/SRC-175/sweep.json
 expected 1bf6cc245bf3d5a70fc95cc191740f42a7695685f9496426f534bd045e7db5ef
 actual   f384dc2c45f280b24ff6595ee070b914ffcf215dd59c2bb214ca80a81d9a0256
 record   sweep.json/files/6/sha256
extract/SRC-222/REPORT.md
 expected d901ebfeaf7bcb84fdc6b295dd238355510f261bf68982948f6b8beb6a6e1eca
 actual   7f1c8ebe0f8b2b5d56061aea3f9c6b5abfa147d85b73413d1d835cacf5785c01
 record   sweep.json/files/0/sha256
extract/SRC-222/sweep.json
 expected dda4b6bb5612ae8774a37f3d8a9daf735d8ab71b6b20c784e1b4716b6eedf396
 actual   ee957bf1e44742963595f016fb40c4d1d44e21743e749ad622cb35e24d77b530
 record   sweep.json/files/4/sha256
probes/sources/SRC-106/sweep.json
 expected 976acef3311b4f9b91f8937edf76f6d847aed55b08b68f12417944c020dae6aa
 actual   2bd2abb9dcccc4ed1d63a43069899a56172e6a4e10345a4694ed15c299dc1b0e
 record   sweep.json/files/12/sha256
probes/sources/SRC-114/sweep.json
 expected e2bb7b2e1ed768fe545c4b122152a1ac496b705481278492cdd8ade41cbff45c
 actual   71083f7a1b5feea41cc352b6455aa40fb315d6c89ce87f06576f1463f0709dd2
 record   sweep.json/files/5/sha256
```

### Missing named file

`probes/sources/SRC-133/manifest.json/robots/0/sha256` names `../iatrackcoaches/robots/www.iatrackcoaches.org.txt`, expected `42c1b29b65ce022d1e3bedb325c961d22de02b487a6a7272b7624cbe744fd253`. Relative to that tree, the named path resolves under `probes/sources/iatrackcoaches/` and is **missing**. The similarly named robots capture under `probes/iatrackcoaches/` is separately hashed in that tree; this audit does not rewrite or silently reinterpret the broken reference.

## Exact digest-coverage gaps

All paths below are relative to the named tree. **Explained** means filename-mentioned in its report/records, not digest-covered. All remaining entries are **unexplained**. These are 108 files total, not mismatches (there is no expected digest to compare).

Explained, but undigested (10):

- `extract/097-oh-ohsaa-seed`: `rows.csv`.
- `extract/107-mi-eem`: `rows.csv`.
- `extract/184-va-vdoe-directory`: `rows.csv`.
- `extract/185-va-visaa`: `rows.csv`.
- `extract/213-la-bese-nonpublic`: `rows.csv`.
- `extract/SRC-178`: `excluded_rows.csv`, `rows.csv`.
- `extract/SRC-179`: `excluded_rows.csv`, `rows.csv`.
- `probes/ok-ossaarankings`: `rows.csv`.

Neither listed nor explained (98):

- `extract/130-ia-doe`: `rows.csv`.
- `extract/131-ia-arcgis`: `rows.csv`, `schools_full.json`.
- `extract/186-va-vhsl`: `qualified.json`.
- `extract/204-ok-directory`: `rows.csv`, `schools_full.json`.
- `extract/223-sd-edudir`: `qualified.json`, `rows.csv`.
- `extract/228-sdhsca-membership`: `qualified.json`.
- `extract/230-pa-edna`: `rows.csv`, `schools_full.json`.
- `extract/231-paisaa-members`: `captures/3698405d7db8fb8c010c0e7f7ab0eef8ba975e5d887c30fda9325aa88c0cdeae.body`, `captures/c42e591d652b30b14a74f88e8e65d989079a4de831d24fd3c62969126bf26aba.body`, `rows.csv`.
- `probes/131-ia-arcgis`: `count0.json`, `count1.json`, `layer0.json`, `layer1.json`, `robots.body`, `service.json`.
- `probes/230-pa-edna`: `child-01.body`, `child-02.body`, `child-03.body`, `child-04.body`, `child-05.body`, `child-06.body`, `child-07.body`, `child-08.body`, `child-09.body`, `page.body`, `pnp-export-2.body`, `pnp-export.body`, `pnp-form-2.body`, `pnp-form.body`, `public-export.body`, `public-form.body`, `robots.body`.
- `probes/231-paisaa-members`: `captures/3698405d7db8fb8c010c0e7f7ab0eef8ba975e5d887c30fda9325aa88c0cdeae.body`, `captures/c42e591d652b30b14a74f88e8e65d989079a4de831d24fd3c62969126bf26aba.body`.
- `probes/239-ks-directory`: `captures/2510c48aa6246882e33fa7d500410799263ae4626e2cca59a7fc47d98b97b9a0.body`, `captures/dc1d54dab6ec8c00f70137927504e4f222c8395f10760b6beecfcfa94e08249f.body`.
- `probes/ciac-dragonfly`: `directory-p1.json`, `directory-p2.json`.
- `probes/122-il-isbe` (55 filenames):

```text
candidate-01.body
candidate-02.body
candidate-03.body
candidate-04.body
candidate-05.body
candidate-06.body
candidate-07.body
candidate-08.body
candidate-09.body
candidate-10.body
candidate-11.body
candidate-12.body
candidate-13.body
candidate-14.body
candidate-15.body
candidate-16.body
candidate-17.body
candidate-18.body
candidate-19.body
candidate-20.body
candidate-21.body
candidate-22.body
candidate-23.body
candidate-24.body
candidate-25.body
candidate-results.json
captures/0b34e897ef3af1a969d3dd050685cd91436bcd437a6f7c90b8098eb467c19fba.body
captures/0ffa85f9d329f8f93b0c67d6a71decd7458f4166fedd24cefcb27a2aca014f64.body
captures/12c2492d6ee05749728c52d7390ea5e831942659f2c361541daac9895191743f.body
captures/19d2fcc89866e1b0983161689f758b88088d533193dea2938e27accbb21aba44.body
captures/2f544714e0a7b92192c69616b1a199c0a6ad75f04ad63ffa623fc778dec8d840.body
captures/30e21a571632d1d1c3330007d0b761404fc9278f981fdc4fccb4cbf284f17b00.body
captures/36777ef1d7837ec1238299900b5c15f4120d0060b4558062d1f83ad852bec7e4.body
captures/4628a2246dc0e4edc6cd50a049ac6843623352786e4c448c025b74cd70ef0487.body
captures/4e71ab7ec809ab6418e65b7ca9577e18c581f6178d602eb7c535ce8b7130e1ec.body
captures/50631b29771bc6ad15aa38271e26c9ceee4e41ac56e8d8396ffb536c101ed1e0.body
captures/6a5cb84f39c16bca2d4a212c9ffb978e0f0ea0003fe5c5c547cc9e8cb4f2314e.body
captures/715cdc9cb1c45b03eab134f04806f9cd7c5e611484c46fc12b4a8916b66d7635.body
captures/75d14e57a40f164504c6a9a63d1a376708851cbe32b5e5331699bf52cf7fd6a7.body
captures/852ed53c8668524e90f6fc84a713b9e07eac519a7599fa8d2fc8d0877ec8eab8.body
captures/8eceaaaf735d4ae78101628614cf06de0c1af11d8e333064e66cf71ff9d5dac5.body
captures/914f1fdf21eef102e570fa58d585dc5ffb79e28c0b966316cad5393d205cc5e3.body
captures/a094e4f44fcb1a7e31ab8142783fcf09a510da1c84a9341bb09c7def5853a731.body
captures/a6bf6c5143331e437db18ab20a451c1153710763fe5baa508784b43a158e689d.body
captures/aacaae5e96b6136d82349855737aaa2926ad504407646c86ecc25429301156da.body
captures/abbc418b6d31a52dd6974c8e227d49ee5a8671046e8e4570c3895287dcefa359.body
captures/af400336084a407c03c3de57edc4c808a115fa068d4b6cf0b5e82b74434b06cc.body
captures/c0b43429e2820e43110934915351fad1f32fad78f905b792b20bb4278c07006d.body
captures/d245281d2e31c41c17fffe2c80a45a78359cbabe94ef569dc17c1f0bb2401d8f.body
captures/e6fc67f74e1966723a097e3bf0c42446d85ae8d94a60487884062b0a79834eed.body
captures/e7a1dd58428e78e1e7c5b809b6d7d077e9461c839af974551d96829cc7be238c.body
captures/ed3a96184fb66bffa0db1fa9f1457a92750d3216626089055eec3b50c842010e.body
captures/f4958d2a4b2805e8d371a03c9508d24e19c2d59b0e3afbcc162f041fd226ff79.body
page.body
robots.body
```

## Capture-date classification

Undocumented capture dates (12): `extract/130-ia-doe`, `extract/131-ia-arcgis`, `extract/204-ok-directory`, `extract/223-sd-edudir`, `extract/228-sdhsca-membership`, `extract/230-pa-edna`, `extract/231-paisaa-members`, `extract/239-ks-directory`, `probes/122-il-isbe`, `probes/231-paisaa-members`, `probes/239-ks-directory`, `probes/ciac-dragonfly`. The other 70 declare 2026-10-04/05. Filesystem timestamps are not substituted for capture timestamps.

## Execution evidence / exact commands

Successful `run_shell` invocations used Python for inventory/record parsing and **`sha256sum` for every byte digest**. No network-capable program was invoked during this audit. Hashing used the exact subprocess invocation `subprocess.check_output(["sha256sum", str(p)], text=True)` for each resolved file, with size/mtime checks around it. The complete Python inventory/audit/refinement/render command inputs remain in this active NjTap-2 tool transcript; full stdout is retained at these actual tool output paths (not fabricated):

| Invocation | Exit | Raw output |
| --- | ---: | --- |
| Inventory of all probes/extract trees | 0 | `/tmp/omp-shell-1791164401022-7mbipx.log` |
| Recursive digest schema discovery | 0 | `/tmp/omp-shell-1791164429793-soteuw.log` |
| Compact manifest schema inventory | 0 | `/tmp/omp-shell-1791164455800-17w7ny.log` |
| Full named-file sha256 audit | 0 | `/tmp/omp-shell-1791164583131-6jsjot.log` |
| Anomaly summary | 0 | `/tmp/omp-shell-1791164592706-4gm6k5.log` |
| Exact mismatch/path extraction | 0 | `/tmp/omp-shell-1791164602822-w5wtvy.log` |
| Additional 108 unclaimed-file hashes / trace attribution | 0 | `/tmp/omp-shell-1791164682157-yc7bbb.log` |
| Per-tree table / full mismatch / undigested rendering | 0 | `/tmp/omp-shell-1791164744828-93ex4f.log` |
| Raw-capture vs total checksum count reconciliation | 0 | `/tmp/omp-shell-1791164771760-wixl6b.log` |

Exact executed inventory command:

```bash
python3 -c 'import pathlib,json; b=pathlib.Path("research/sources/coach-coverage-bundle-20261004"); roots=sorted([p for lane in ("probes","extract") for p in (b/lane).iterdir() if p.is_dir() and p.name!="sources"]+list((b/"probes/sources").iterdir())); print(json.dumps([{ "tree":str(p.relative_to(b)), "files":[str(f.relative_to(p)) for f in p.rglob("*") if f.is_file()]} for p in roots],indent=2))'
```

Exact executed compact summary command:

```bash
python3 -c 'import json; x=json.load(open("/tmp/omp-shell-1791164583131-6jsjot.log")); print(json.dumps(x["totals"],indent=2)); [print(t["tree"],"CLAIM",t["claimed_unique_file_digest_pairs"],"OK",t["ok"],"BAD",len(t["mismatch"]),"MISS",len(t["missing"]),"UNBOUND",t["unbound"],"UNCOVERED",t["uncovered"],"GAPS",t["gaps"]) for t in x["trees"]]'
```

That initial summary included 4 unbound duplicate occurrences and 26 preliminary REQUALIFY classifications. Source inspection resolved those duplicates and recognized embedded source mappings; the final observed totals are the 3/57/22 classification reported above. The underlying 380 matches / 13 mismatches / 1 missing file did not change.

A multiline discovery command was rejected by the tool with the exact error **`command must be ascii`**. It was retried as a single-line `python3 -c 'exec("...\\n...")'` invocation and succeeded. No source files were written to work around that tool failure.

## ACCEPTANCE / HANDOFF

Audit deliverable is complete: the per-tree table covers all 82 inventoried trees; every mismatch has its exact filename, record pointer and both digests; the missing reference is explicit; all uncovered raw/derived filenames are listed. **This is not an all-green tap bundle.**

Prioritize `extract/SRC-100`: both `rows.csv` and `excluded_rows.csv` differ from their published digests. Their derivation/import relationship needs reconciliation before reusing that checksum evidence. The five changed report files and six self-referential checksum documents also require named owner reconciliation. The broken SRC-133 robots path is a reference repair, not proof robots is absent on the source host.

Trees requiring evidence re-qualification (22): `extract/130-ia-doe`, `extract/131-ia-arcgis`, `extract/186-va-vhsl`, `extract/204-ok-directory`, `extract/223-sd-edudir`, `extract/228-sdhsca-membership`, `extract/230-pa-edna`, `extract/231-paisaa-members`, `extract/SRC-054`, `extract/SRC-068`, `extract/SRC-100`, `extract/SRC-175`, `extract/SRC-222`, `probes/122-il-isbe`, `probes/131-ia-arcgis`, `probes/230-pa-edna`, `probes/231-paisaa-members`, `probes/239-ks-directory`, `probes/ciac-dragonfly`, `probes/sources/SRC-106`, `probes/sources/SRC-114`, `probes/sources/SRC-133`.

**Read-only limits:** checksum parity cannot prove raw HTTP fidelity, robots/pacing compliance, anonymous access, accurate field extraction, current tenure, executed historical CLI commands, or live store readback. This assignment deliberately did not re-fetch, run imports/providers, open any store, or modify manifests. Some structural gaps may be fulfilled in paired trees or Main's shared evidence ledger; that linkage is not contained in the individual tree and must be made explicit by its owner. Captures/records arriving or changing after the snapshot require a new audit of those changed files. Main owns all repairs and ledger integration.
