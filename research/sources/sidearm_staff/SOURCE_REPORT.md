# SIDEARM Staff Directory Source Report

Source token: SRC-266, gomats.org / Miramonte High School
Adapter slug: `sidearm_staff`
Configured coverage: one host, one school, CA

## Endpoint and provenance

`GET https://gomats.org/staff-directory` returns the server-rendered public directory anonymously. The verified 2026-10-04 probe retained HTTP 200 HTML, 305,587 bytes, SHA-256 `8b392547b16ad80dde77bd7941df182804d1c59ee6151308a8da40fdab99b4b8`. Main staged the byte-exact fixture at `crates/census-crawl/tests/fixtures/sidearm_staff/gomats.org__staff-directory__full.html`; its metadata and robots details are in the adjacent `SOURCE.md`.

The probe's findings are under `research/sources/coach-coverage-bundle-20261004/probes/sidearm_miramonte/`. The prior SRC-266 extraction demonstrated two XC/TF coach rows joined to the school's Athletic Director; it is precedent, not verification of this adapter.

## Robots and admission

`https://gomats.org/robots.txt`: HTTP 200, 6,001 bytes, SHA-256 `454315be725af18052b2009cd7aa02628dbbf5d6445a4818036b110a2ce8b5da`. Its wildcard group permits `/staff-directory` and declares `Crawl-delay: 30`.

The registry uses the existing `fetched("gomats.org", CRAWL_DELAY_THIRTY_RPS)` admission, with `CRAWL_DELAY_THIRTY_RPS = 1.0 / 30.0`, maximum one in-flight request. The existing fetcher takes the maximum of its configured, robots and registry-declared delays; the adapter performs one normal `fetcher.get` for the directory. No JavaScript, profile pages, API endpoints or blocked assets are fetched.

The original probe reported an approximately 15.5-second saved-file robots-to-page gap and explicitly did not claim compliance. The new admission declares thirty-second pacing; execution evidence must come from Main's smoke, not this historical capture.

## Parser and mapping

- A real `article` whose class list includes `sidearm-staff` is required. Missing platform markup, missing school name and oversized bodies or tables return the existing `CrawlError::Schema`; regex construction failures use `CrawlError::RegexInit`.
- School comes from `window.client_title`, falling back to page `h1`; markup, HTML entities and whitespace are normalized.
- Only complete `tr.sidearm-staff-member` rows are considered. The parser binds `col-fullname`, `col-staff_custom_1`, `col-staff_custom_2`, `col-staff_title` and `col-staff_email` via header tokens rather than cell position. Name comes from the name-cell link text.
- Category headings are keyed by `data-category-id` and provide sport context only for an empty sport cell. Category text never replaces an explicit sport.
- Each email is assembled from that row's email-cell `firstHalf` and `secondHalf` quoted literals, trimming each half and joining with `@`. Missing or blank halves skip that member. Neither scripts nor global half-pairing are used.
- The parser retains all email-bearing sports; canonical mapping follows `home_campus`'s XC/TF scope and sport classifier. It maps coach titles to HeadCoach, AssistantCoach or Unknown; non-coach staff are excluded. The exact title `Athletic Director` maps to a separate school-linked director with no sport and unknown gender.
- Source member/category IDs, level, sport and role remain in each contact's evidence note. Canonical IDs retain the existing domain's school/person/sport/gender/role identity; level is evidence, not a new domain identity field.
- `SourceNamespace::SchoolDirectory { provider: "sidearm_staff", state: CA }` describes the vendor directory without inventing an association affiliation. School source identity is the configured host; contact source identity also includes its member ID. No city or address is inferred.
- `--states` restricts the single evidenced CA host, `--limit 0` performs no fetch, and any positive limit processes at most the one configured school. Optional school-name filtering applies to the published school name. Empty observed-on uses the adapter context date.
- Schools, school observations, contacts and journals (`sidearm_staff_schools`, `sidearm_staff_coaches`) commit through the existing write-batch path.

Expected fixture-derived canonical contacts:

| Name | Sport / role | Published email |
| --- | --- | --- |
| Brian Henderson | Cross Country / Head Coach | bhenderson@auhsdschools.org |
| Robert Kennedy | Track & Field / Head Coach | caljumper259@gmail.com |
| Sean Hennessy | School Athletic Director | shennessy@auhsdschools.org |

Gmail remains a published personal mailbox according to the existing canonical email classifier; it still counts toward `with_email`.

## Verification handoff

This implementation worker has no command-execution tool. No cargo command, live fetch, provider run or store readback was executed here. Main owns the following acceptance commands after integration:

```text
cargo test -p census-crawl --lib sidearm_staff
cargo test -p census-crawl --lib -- registry:: applicability::
cargo build -p census-service
target/debug/census-service --store var/sidearm-adapter-smoke/store provider sidearm_staff --states CA --limit 1 --observed-on 2026-10-04
target/debug/census-service --store var/sidearm-adapter-smoke/store fjall-stats
```

The module regressions cover fixture XC/TF/director mapping, exact trimmed emails, retained Flag Football level, missing and blank halves, cross-row half isolation, category fallback precedence, exact director title, non-SIDEARM and missing-school errors, and the thirty-second/CA-only registry policy. Expected fixture mapping is one school and three canonical contacts with published email; no live report or fjall counts are claimed until Main runs them. No full walk is part of this slice.

### Main acceptance (2026-10-04)

```
cargo test -p census-crawl --lib sidearm_staff               -> 8 passed, 0 failed
cargo test -p census-crawl --lib -- registry:: applicability:: -> 26 passed, 0 failed
cargo fmt --all; cargo build -p census-service               -> clean
cargo xtask replay sidearm_staff
  -> gomats.org__staff-directory__full.html  staff_directory name="Miramonte High School" members=65 published_emails=65
census-service --store var/sidearm-adapter-smoke/store provider sidearm_staff --states CA --limit 1 --observed-on 2026-10-04
  -> {"rows": 1, "requests": 1, "from_cache": 0, "errors": 0, "with_email": 3, "unit": "schools",
      "notes": ["processed Miramonte High School (3 coach_rows, 3 with email); verified host only: gomats.org"]}   (34.5 s wall)
fjall-stats + consolidate on that store -> schools 1, coaches 3, observations 5;
  Brian Henderson (cross_country/head_coach), Robert Kennedy (outdoor_track/head_coach),
  Sean Hennessy (athletic_director); athletics_website https://gomats.org
```

The live body is not byte-identical to the fixture (the store archived it as `04dbcee4…`) and both parse. The acceptance run is also recorded in `docs/VERIFICATION-EVIDENCE.md`.

## Limits

Only gomats.org is registered. SIDEARM-family reusability is grounded in one retained example, not an invented national host list or a tested multi-school campaign. CA is contextual from the page's published MaxPreps link and is not a verified postal address; city/address remain absent. Other custom-column numbering, email mechanisms, escaped JavaScript literals, authenticated directories, roster pages, tenure and phones are unverified.
