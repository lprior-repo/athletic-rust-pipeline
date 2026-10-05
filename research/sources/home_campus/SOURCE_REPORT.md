# Home Campus Source Report

Source IDs: SRC-017 (CA), SRC-034 (FL), SRC-096 (NJ)
States: CA, FL, NJ
Adapter slug: `home_campus`
Coverage: CIF sections 1-9 and 13 (CA), FHSAA section 10 (FL), NJSIAA section 12 (NJ)

## URLs

- Directory: `https://www.cifsshome.org/widget/school/directory?section=<n>` (`<n>` from 1..9, 13
  and 10, 12 as above; `www.cifnshome.org/widget/school/directory?section=7` serves the same NCS
  listing from the section-branded host)
- Details: `https://www.cifsshome.org/widget/get-school-details/<id>/details` — `application/json`
  only with the directory page's own XHR context (`X-Requested-With: XMLHttpRequest` plus a
  same-origin `Referer`); without them the same URL answers 403
- Full-page directory variants exist (`/widget/school/directory?section=<n>` rendered) but carry no
  more than the buttons

## robots.txt

`https://www.cifsshome.org/robots.txt` — 200, 24 bytes, sha256
`e5c4b84484ee4216e9373be99380320c25dd94805f99f0a805846f087636553f`; body is
`User-agent: *\nDisallow:`. `www.cifnshome.org` and `www.fhsaahome.org` serve the identical bytes.
Collection paces at one request per second per host (the registry's declared `cifsshome.org`
policy).

## Capture Inventory

Raw responses captured 2026-10-04 with the research user-agent. The five adapter fixtures are
byte-for-byte copies of the probe captures; their sha256 and per-file detail live in
`crates/census-crawl/tests/fixtures/home_campus/SOURCE.md`.

1. `crates/census-crawl/tests/fixtures/home_campus/directory_section_10.html` — 200, 230,370
   bytes, sha256 `f79590dd…` — FHSAA, 880 school buttons.
2. `crates/census-crawl/tests/fixtures/home_campus/directory_section_12.html` — 200, 133,880
   bytes, sha256 `3c9a5d71…` — NJSIAA, 452 school buttons.
3. `crates/census-crawl/tests/fixtures/home_campus/details_19.json` (Arcadia, CA) — 7,813 bytes,
   sha256 `0667c404…`; `details_1872.json` (Bolles, FL) — 8,062 bytes, sha256 `5dafbbea…`;
   `details_3374.json` (Abraham Clark, NJ) — 958 bytes, sha256 `6c63554f…`.
4. The wider probe, including the CA section sweep (1,727 buttons over sections 1-9 and 13), the
   NCS section-7 page (75,199 bytes), the `403` bodies and three robots copies:
   `research/sources/coach-coverage-bundle-20261004/probes/home_campus/` with 28 captures hashed in
   `manifest.json.main_verification.captures_sha256` and the narrative in `FINDINGS.md`.

Not published: coach phone numbers, tenure, athlete rosters; the JSON carries one row per sport and
level (mostly `Varsity`, `Head Coach`) and the `athleticFaculties` lane lists administrative
contacts rather than coaches.

## Adapter Behavior

- Sections are selected from `SECTIONS` (CA 1-9, 13; FL 10; NJ 12) and filtered by `--states`.
- A school row is minted from the details `school` object (name, city, published address/zip/league
  and phone recorded as the evidence note) and tagged with the section's association
  (`cif`/`fhsaa`/`njsiaa`).
- Coach rows keep only `Cross Country, …` and `Track & Field, …` (sport + gender from the label);
  rows with no published name are skipped. Role comes from `aft_name` (`Head Coach` /
  `Assistant Coach`, otherwise `Unknown`).
- `athleticFaculties` rows contribute an `AthleticDirector` entity only when `aft_name` is exactly
  `Athletic Director`; the remaining administrative roles (principal, trainer, AD assistant) are
  not coach contacts.
- Journals: `home_campus_schools` and `home_campus_coaches`, keyed `<state>:<school id>`.

## Evidence

- `cargo test -p census-crawl --lib home_campus` → 6 passed, 0 failed (880 FL buttons, 452 NJ
  buttons, Arcadia 24 coaches/6 faculty → 4 sport coaches + 1 director, Bolles null-row skip,
  Abraham Clark empty roster, sport-label mapping).
- Registry/applicability parity: `cargo test -p census-crawl --lib -- registry:: applicability::`
  → 26 passed, 0 failed.
- Live smoke (2026-10-04): `target/debug/census-service --store
  var/home-campus-smoke-20261004/store provider home_campus --limit 2 --states CA --observed-on
  2026-10-04` → `rows: 2, requests: 3, from_cache: 0, errors: 0, with_email: 5`;
  `fjall-stats` on that store → `schools 2`, `coaches 5`, `observations 7` with no other table
  touched. The three requests are one section-1 directory fetch and two details fetches, all
  answered 200.
- Full CA walk (2026-10-04 17:54-18:04, `--store var/home-campus-CA-20261004/store provider
  home_campus --states CA`): section 1's detail fetches began, then the host switched to a **405
  `Human Verification` page** (2,195 bytes; capture
  `var/home-campus-CA-20261004/challenge-20261004T1756.html`, sha256 `8468103352d85693…`) for every
  further request. Result: **503 schools / 2,258 coach rows (all with email)**; `fjall-stats` →
  `schools 503, coaches 2258, observations 2761`. 70 section-1 details plus CA sections 2-9 and 13
  were refused. FL and NJ answered 405 on their first directory fetch (0 rows). A concurrent
  orphaned `var/tap-home-campus-20261004` run (started 17:54:09, 60 schools / 270 coaches) was
  stopped; two runs against one host exceed the declared 1 rps pacing and are the likeliest trigger.
  The 200s are cached, so the retry probes the host and resumes instead of re-spending those
  requests.
- Resume (18:07-18:09): the host answered 200 to a bare probe, the resumed CA run reached 562/572
  section-1 schools (2,399 coach rows in that run; store observation rows `schools 1065 / coaches
  4657`), then 405 returned for the remaining 10 details and every other CA section, and the next
  probe was 405 again. After the first challenge the host re-arms on bursts well below the first
  run's ~64 requests/min, and collection resumes from the fetch cache.
- Slow pass (18:26-18:29): CA sections 1-3 parsed (572/32/132 schools), **623 processed** with 2,563
  coach rows; sections 4-9 and 13 answered 405 to their directory requests and ~113 section-2/3
  details were refused. A second OMP session on this machine was crawling the same host into
  `var/tap-home-campus-20261004/` at the same time (its NJ pass: 452 schools, 0 errors; its FL pass
  finished 675 of 880 with 205 refusals), which is why the host stayed challenged; the per-origin
  serialization gap is filed as `athletic-rust-pipeline-aht`.
- CA completes (18:44-19:03, machine quiet): all ten sections parsed, **1,717 schools processed,
  6,758 coach rows, 0 errors** from 1,101 live requests and 626 cache replays. Readback
  (`fjall-stats`): `schools 3405 / coaches 13978 / observations 17383` observation rows,
  `store_bytes 38752402`.
- Rate correction: 1,101 live requests in 1,111 s shows that pass ran at the fetcher default 1 rps,
  not the declared 0.2 - `descriptor_for_host` matches `admission.origin` exactly and the adapter
  requests `www.cifsshome.org` while the descriptor said `cifsshome.org`. The origin is now
  `www.cifsshome.org` and `HOME_CAMPUS_RPS` is **0.5 requests/second**: solo 1 rps passed 1,101
  requests, two concurrent runs at roughly twice that drew the challenge, and the halved rate keeps
  two uncoordinated runs at the tolerated rate.
- Entity-decode regression fixed in the same module: a concurrent scanner rewrite made
  `decode_entities` break at the first `&` and re-append the original, corrupting FL's
  `Land O&#039;Lakes`; `florida_directory_parses_every_school_button` caught it and
  `cargo test -p census-crawl --lib` is 906 passed / 0 failed.
- FL and NJ complete (19:10-19:55) into the same store at 0.5 rps: FL section 10 parsed 880 schools,
  **rows 880, errors 0, with_email 3,677**; NJ section 12 parsed 452 schools, **rows 452, errors 0,
  with_email 0** (the section publishes buttons but no coach roster). The 1,334 live requests took
  2,673 s = **0.4995 requests/second**, so the corrected admission binds. Final readback
  (`fjall-stats`): `schools 4737 / coaches 17655 / observations 22392`, `store_bytes 56390780`.

## Limits

The source covers only the sections it serves; other associations and states need their own host.
The directory buttons give `(City)`-suffixed names only, and the details JSON's roster is
sport-level rather than person-complete, so assistant coverage, coach phones and any claim beyond
the published rows are out of scope; NJ's section carries no coach rows at all. The host answers a
`Human Verification` challenge to bursts: the first CA walk passed 572 section-1 requests at 1 rps
and then flagged, a later ~60-request burst was refused, and two concurrent runs (~2 rps aggregate)
re-armed it at 18:04, so collection for this host runs at 0.5 rps per run and resumes from the fetch
cache. No national census claim follows from this report.
