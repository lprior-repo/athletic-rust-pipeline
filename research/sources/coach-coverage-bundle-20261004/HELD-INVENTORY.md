# Held-inventory gate — mandatory before any tap (created 2026-10-04)

The owner's ported repository already contains acquisitions for many registry sources — including a
complete prior capture pass and already-joined NCES lanes. Every worker, before fetching any
TAP-REGISTRY source, MUST check these locations and cite path + bytes + sha256/date in its comment:

- `research/sources/*/samples/**` + each tree's `samples/CAPTURES.md` (the ported capture corpus)
- `var/` run directories — stores, corpora, `generations/*`, `frozen-input.json`, join reports
- `/home/lewis/src/ad-law-scrape/data/` — retained local captures
- `research/sources/<name>/SOURCE_REPORT.md` plus that source's fixtures (`crates/**/tests/fixtures/`)
- `docs/OPERATIONS.md`, `docs/VERIFICATION-EVIDENCE.md`
- Ported sibling trees under `/home/lewis/src/ad-law-scrape/` (`arh-*`, `census-prototype`, `samples`)

A source with a held capture is REUSE-ONLY: no refetch, no re-derivation, no import into scratch
stores. A source implemented by an adapter is ADAPTER-CLASS: the adapter consumes the held capture.

## Ported capture corpus (in-repo, dated 2026-09-22) — check before any fetch

`samples/CAPTURES.md` in each tree lists URL + HTTP status + wire/on-disk bytes + capture time per file.

| Tree | samples files |
|---|---|
| coach-directories-national | 555 (CAPTURES.md: 207 logged fetches across 81 hosts; 301 URL rows; robots honoured per host) |
| state-assoc-mountain | 250 |
| state-assoc-southcentral | 249 |
| state-assoc-southeast | 175 |
| state-assoc-midatlantic | 125 |
| national-aggregators | 125 |
| timing-providers-national | 106 |
| state-assoc-westcoast | 106 |
| milesplit-national | 64 |
| athleticnet | 52 |
| milesplit-cohort-enumerability | 30 |
| state-assoc-plains | 21 |
| state-assoc-greatlakes | 1 |
| **total** | **≈1,829** |

### Screening metric (host-level, 2026-10-04)

All 12 `CAPTURES.md` tables carry 256 distinct hosts; **85 of 278 registry entries (31%) name a host the
corpus has captured at least once**. Per-state screen (covered/total): CA 10/17, LA 7/10, AZ 6/10,
MA 5/10, UT 5/11, SD 4/7, IL 4/8, OH 4/9, CT/IA/MO/KS 3/8. Largest unmatched pools: NATIONAL 30,
TX 13, FL 11, WA 10, NJ/VA/PA 9 each.

Caveat: host presence ≠ target-path held (SDHSAA is held at homepage/XC but not at
`/athletics-cooperatives/`). This only screens; the path-level map decides. Raw per-entry data:
`var/tap-corpus-coverage-20261004.json`.

## Port-tree acquisition corpus (Tier B) — the port's own results

`census-prototype/` (the Python acquisition tree this repo was ported from) already holds the
program-wide acquisition. Match registry hosts here FIRST when the tap corpus lacks a row.

- **`census-prototype/raw/`** — 56,294 captured bodies across **682 distinct hosts**, named
  `<host>__<hash>` (+ `.meta.json`); host = filename prefix, registry hosts match directly.
  Top hosts: `www.milesplit.com` 25,430, `maxinfosite-api-live.dragonflyathletics.com` 11,455,
  `www.maxpreps.com` 3,840, `www.gobound.com` 2,029, `officials.myohsaa.org` 1,564,
  `www.piaa.org` 1,475, `services.arbitersports.com` 1,208, `api.ihsa.org` 817,
  `wiaa.finalforms.com` 780, `www.mshsl.org` 750, `schools.wiaawi.org` 710, `portal.tssaa.org` 460,
  `cifsshome.org` 448, `chsaanow.com` 380, `www.aiaonline.org` 344. NJ NJSIAA: 15 bodies.
- **`census-prototype/out/coaches.jsonl`** — 12,326,749 B, **61,083 merged coach-contact rows**;
  16,947 distinct (state, school); 36,917 with email; 25,318 with phone; 49 states. Sports:
  AthleticDirector 22,956 / Track 22,473 / CrossCountry 15,654. Roles: AD 22,956 / HeadCoach 21,883 /
  Coach 8,993 / AssistantCoach 7,251. Fields: `state, school, person, sport, role, gender, email,
  phone, code`.
- **`nj-captures.md`** — index of 10 NJ captures → raw bodies: 9 NJSIAA member-information pages
  (50 schools / 50 coaches each) + `nj.milesplit.com/teams` (538 schools).
- **`arh-piaa/.../fixtures/pa_piaa/PROVENANCE.json`** — PIAA directory fixtures copied byte-identical
  from `census-prototype/raw`, captured 2026-09-27, sha256 + bytes verified, robots note included.
- **`arh-coach-directories/.../fixtures/coach_directories/`** — 5 DragonFly fixtures (GHSA p2,
  IN staff summary, NC staff summary, NCHSAA p1, access-denied sample).

Rule: a registry source whose host appears in `census-prototype/raw/` (count ≥ 1) or whose state has
`coaches.jsonl` rows is REUSE-ONLY (class DERIVED) unless the specific target path is provably absent
there. `arh-python-port`, `arh-integration-20261001` and `arh-closeout` retain copies of these
research captures and fixtures.

## Proven held (receipts verified 2026-10-04)

| Source | Held as | Evidence |
|---|---|---|
| SRC-248 NCES CCD directory | **2025–26** vintage (supersedes the 2024–25 re-tap) | `var/school-address-join-20261004/ccd/ccd_sch_029_2526_w_0a_050626.csv`, 41,054,983 B, sha256 `d1473136285b5994b73a1a8b640757811eb81e0ae770953bcf915ee8c422386e` |
| SRC-254 NCES PSS 2023–24 public-use CSV | captured 2026-09-28 | `/home/lewis/src/ad-law-scrape/data/nces/pss/pss2324_pu.csv`, 39,521,631 B, sha256 `14a2f9e600a492940fd57646792b4b5163ea9d03b8015a7df1135066bcec3b8b` |
| SRC-257 PSS 2023–24 frame file | captured 2026-09-28 | `/home/lewis/src/ad-law-scrape/data/nces/pss/2023-24_PSS_Frame_Data.csv`, 849,644 B |
| SRC-255/256 PSS layout + codebook | **not held** | `data/nces/pss/` holds `2023-24_PSS_Data_Dictionary.pdf` (157,766 B, documents the **frame** CSV) and `2021-22_PSS_Documentation.pdf` (2,791,303 B) — neither is the 2023-24 record layout or codebook; documentation-only |
| SRC-225 SDHSAA XC | captured 2026-09-22 | `state-assoc-plains/samples/sdhsaa-cross-country-2026-09-22.html`, 509,970 B, sha256 `7fc5719b775307637087b56334122ad67cbe5167e9b6c3fcd20d5e639c3e44f6` (target `https://sdhsaa.com/activity/cross-country/`); region file 286,293 B, sha256 `5eea4018ddc54e65fa05bb9127a7fc39781741512a1be3a654e78aa7684d8f49`; robots 156 B; plus `coach-directories-national/samples/assoc-home/SD__sdhsaa.com.html`, 1,096,336 B, sha256 `76025973e8f144396cf6fadfbca7ef6d44eb03425aed60d5434509e91d5597c0`; CAPTURES rows 33–34 |
| SRC-158 AIA school-search JSON (AZ) | captured 2026-09-22 | `state-assoc-mountain/samples/`: `aia-search-limit500.json` 7,639 B sha256 `0e414eb850340481419933f9022d665ca6633458fb816570640ab35c1198f55a` (live school records), `aia-schools-search-a.json` 7,590 B, `aia-schools-search-emptystring.json` 7,639 B (the `/schools/search.json` family your bead targets) + `aia-schools.html` 59,463 B sha256 `4421348fa541557ab721e0f5e2cc11d2a28d8150a10844821b165cdf72f1a607`, `aia-sport-xc.html`, `aia-sport-tf.html`, `aia-alignments.html`, `aia-app.js` 313,857 B; plus coach-directories rows 3/70/121/249 (aiaonline.org app.js, home, `/schools`, robots). **2026-10-04 addition:** per-school `/schools/<id>` profiles were not in the corpus — the AIA lane's pre-freeze captures (281 profile bodies + 183 search bodies, robots, failure report in `coach-coverage-bundle-20261004/probes/158-aia-json/`; `extract/158-aia-json/` has no rows) are new evidence; derivation stays frozen |
| SRC-100 OATCCC (OH) | captured + extracted (bead 6ec.6.93 closed) | `coach-coverage-bundle-20261004/extract/SRC-100/REPORT.md` (robots, membership, CSV export, docs robots); corroborating corpus `state-assoc-greatlakes/samples/CAPTURES.md` rows 259–264 (OATCCC coaches/docs/robots/sitemap) |

Join that already consumed the NCES lanes: `var/school-address-join-20261004/` (122,692 entries per
`docs/OPERATIONS.md`; report at `serve/out/school-address-join/report.json`; TAP-REGISTRY entries
SRC-248/254/257 carry `held_in_repo` receipts). Adjacent generation exists with an association lane:
`var/school-address-join-20261004/corpus-assoc/generations/c42d7d54f2c8cee6/` (lanes nces-ccd,
nces-pss, `association:tssaa`; no `out/`).

## Registry classification outcome (278-row map)

`HELD-CLASSIFY.json` (134,756 B; recovered from the classifier's session transcript after two
self-clobbers; verified 278/278 rows with SRC-001..SRC-278 unique and **0 dangling evidence paths**
after Main's 3-path fix; `HELD-CLASSIFY.md` (24,382 B, 184 lines) is the readable summary and
`audit/held-classify-index.txt` is the validation index). Classes: **HELD 73,
HELD+ADAPTER 30, DERIVED 158, ADAPTER+DERIVED 12, ADAPTER 1, ABSENT 4** — i.e. **274 of 278 sources
already covered** by held bodies, adapters or derived state rows (`census-prototype/out/coaches.jsonl`).
Every TAP-REGISTRY entry now carries its `held_class`.

The only genuine gaps, all NATIONAL and verified absent across every location checked:
**SRC-262 NIAAA**, **SRC-263 NASO**, **SRC-264 NHSACA**, **SRC-276 SchoolDigger**. DERIVED means the
state's rows exist even where the exact source body is not held (e.g. SRC-221 LA: LA rows exist;
`louisianaschools.com` itself was never captured).

## Classification outcomes (this pass)

**Redundant — held elsewhere; audit-only, never corpus input:**
- SRC-247/248 re-tap (2024–25 ZIP) and its 100,384-row import in `var/tap-248-nces-ccd/import-store`.
- SRC-254 PSS re-fetch: halted before any bytes were fetched.

**NEW-TAP-NO-PRIOR — valid new evidence (corpus check found no prior row):**
- SRC-227 SDHSAA cooperatives (bead 6ec.6.209): page 484,091 B sha256 `2d051fa915c992a21a3390277227fa63c59e1c48c31d9412927a4b9ef18a23ae`; Handbook PDF 176,305 B sha256 `0ae176abceb047f1e7486ecc5c90d8f21062f6180ca71b4dcdb63c648d57fbb1`; blank Agreement PDF 113,248 B sha256 `55f9e8a1747c96fcd71d70f4129e8d80e860a77975f39fc0ea832343d68d7098`; no rows (blank forms/policy only).
- SRC-228 SDHSCA membership (bead 6ec.6.210): robots 112 B sha256 `c42e591d652b30b14a74f88e8e65d989079a4de831d24fd3c62969126bf26aba`; page 39,859 B sha256 `2b8117b2ed80f422103d424e73ecef62807d2db2b2d9a0366f8a751a9f4d384f`; membership PDF 195,254 B sha256 `e20d31aaa7c4a8f9745d84e71a7e038d0fb6c812cebc6312c8be0aeba5bf9fde`; Class A/B 100,238 B sha256 `51c1c9006c4ec6d65b894616e75cfcdaf0bcf1f71b89138c2eb5d75d102f9a2c`; Class AA 72,924 B sha256 `c120f55309b9966df6e21abdb3cf591c958ed313e210f9af22d49506503ee72a`; 286 coaches imported to `var/tap-228-sdhsca/import-store`.
- SRC-221 LA Louisiana School Finder (bead 6ec.6.205): no `louisianaschools.com` capture in any tier; LA coach rows exist as DERIVED via `coaches.jsonl`, but this specific seed source remains a genuine body gap (map class: DERIVED).
- SRC-108 MITCA: no prior corpus cover (filename + CAPTURES sweeps found nothing); today's pre-freeze fetch stands as new evidence, and the final map classes it HELD on its own artifacts (`probes/108-mitca/REPORT.md` 2.2 KB, `extract/108-mitca/rows.csv` 4.6 KB).

## Per-state work orders (2026-10-04)

`state-work-orders/group-a.md` (TX CA FL NY IL MI WA AZ VA NJ CO IN KY LA NE), `state-work-orders/group-b.md` (OH GA NC WI MN TN PA MO OK IA MD MA CT SC AL AR MS), `state-work-orders/group-c.md` (ND SD MT ID WY UT NV NM OR KS WV DE DC RI NH VT ME) — one section per state: matrix row, held-evidence paths with counts, fill route, true-missing sites. Built by three read-only evidence workers; zero fetches.

Two structural findings: (1) **24 states have no TAP-REGISTRY entry at all** (ND MT ID WY NM WV DE DC RI NH ME; GA NC WI MN TN MD SC AL AR MS; CO KY NE) — the 278-source board under-covered exactly the states with the largest gaps; the work orders now cover them. (2) In every state the coach-**email** lane is school-site dependent: held association bodies are AD/name-heavy or address-only (DragonFly); the school universe (`var/school-address-join-20261004/corpus/generations/57edd0b2c3c88b25/school_directory.csv`, 122,692 entries; street/city/state/zip/phone 100%, lat/long) has a `website` column that is **0% populated** — school-site URL discovery is itself part of the email lane.

Per-state backlog of schools still lacking coach contact: `var/missing-coach-schools-20261004/` (`<ST>.csv` with school/kind/city/zip/phone; `SUMMARY.md`). Conservative normalized-name match: 11,398 of 122,675 schools matched; 111,277 missing (91%).

Live-blocked hosts needing the documented qualification path rather than a bypass: MO (MSHSAA robots-refused; wayback listing held), AL (AHSAA robots-refused), MS (misshsaa WAF), DE (`diaa.org` Cloudflare).

## School-site email lane — recon verdict (2026-10-04)

`var/school-site-lane-20261004/RECON.md`: **no held school-site bodies** for RI/ME/DE/ND/NM (raw/ holds association/platform hosts only; DC is the sole exception with 20 school hosts/86 bodies, all with empty coach lists). Per-state `out/extra/<ST>-school-sites.jsonl` lanes hold 5–12 rows (RI file absent). The prototype's own attempt (`site_seeds.py` → `school_sites.py` Playwright lane) was a dead end: 4,542 guessed domains fetched 298 / 0 rows; search lane provider-blocked; MileSplit coach HTML absent; the applicability matrix marks the lane "provider-specific qualification required" with no accepted source appointment. Repo machinery: one school-website fetcher exists (`sidearm_staff`, CA/gomats.org only) with an offline replay path (`cargo xtask replay <slug>`); no link-following; prototype raw keys (24-hex) are not directly replayable into the repo cache (sha256 keys) without a converter. **Conclusion:** coach emails require new qualified per-host-family adapters plus live fetching — they cannot come from held evidence. The stated requirement bar (coach + school + address + state) is served by the association/held-body lanes; emails are a reachability upgrade.
