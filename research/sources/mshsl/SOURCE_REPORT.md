# MSHSL Source Report

Source ID: mshsl (adapter constant `SOURCE_ID`, `crates/census-crawl/src/mshsl/mod.rs:33`)
State: MN (Minnesota)
Association: Minnesota State High School League (MSHSL)
Report date: 2026-10-09

## Source URLs

- School list: `https://www.mshsl.org/schools` (`crates/census-crawl/src/mshsl/mod.rs:35`)
- School detail: `https://www.mshsl.org/schools/<slug>` (`crates/census-crawl/src/mshsl/mod.rs:36`)
- Teams JSON:API view: `https://www.mshsl.org/jsonapi/views/teams/list_school` (`crates/census-crawl/src/mshsl/mod.rs:37`)
- Coach records API: `https://www.mshsl.org/api/coaches/<nid>` (`crates/census-crawl/src/mshsl/mod.rs:38`)

## Related records in this repository

- `research/sources/state-assoc-plains/SOURCE_REPORT.md:222-256` — MN section of the plains group report (chain `/schools` -> `/schools/<slug>` -> team view -> `/api/coaches/<nid>`; store run 665 schools / 6,596 coach rows). That section cites the adapter at `crates/census-service/src/sources/mshsl/`; no such path exists in this worktree (`crates/census-service/src/sources/mshsl/**` glob returned no match). The adapter verified for this report lives at `crates/census-crawl/src/mshsl/`.
- `research/sources/coach-directories-national/SOURCE_REPORT.md:702-723` — records the school page as school-identity + AD-name discovery only; "the AD emails present in the baseline are NOT served by this page".
- `research/sources/coach-coverage-bundle-20261004/HELD-INVENTORY.md:59` and `research/sources/coach-coverage-bundle-20261004/state-work-orders/group-b.md:147` — "`www.mshsl.org` 750" responses.

## Capture retention and contact provenance (2026-10-09)

### 1. Evidence URL recorded for a coach email parsed from `/api/coaches/<nid>`

- Request URL: `crates/census-crawl/src/mshsl/collect/teams.rs:75` builds `let url = format!("{COACH_API_PREFIX}{}", node.nid);` with `COACH_API_PREFIX = "https://www.mshsl.org/api/coaches/"` (`crates/census-crawl/src/mshsl/mod.rs:38`).
- Parse: `crates/census-crawl/src/mshsl/collect/teams.rs:79-83` (`serde_json::from_str::<Value>`), row locator `:93` (`format!("{url}#row={ordinal}")`), row decode `:94` (`serde_json::from_value::<CoachRecord>`).
- Email attachment: `crates/census-crawl/src/mshsl/map.rs:202-205` runs `published_coach_email` and then `coach.set_published_email(&address)`; the filter at `crates/census-crawl/src/mshsl/map.rs:169-173` validates address shape only (its `_domains` parameter is unused on this path).
- Evidence record: `crates/census-crawl/src/mshsl/map.rs:214-217`:
  `coach.evidence.push(Evidence::parsed(SourceRef::new(SOURCE_ID, Some(team.api_url.clone())), observed_on));`
  `team.api_url` is set from the request URL (`crates/census-crawl/src/mshsl/collect/teams.rs:101-105`; struct at `crates/census-crawl/src/mshsl/teams.rs:33-37`) and `observed_on` is `capture.fetched_at` (`crates/census-crawl/src/mshsl/collect/teams.rs:106-112`).
- The same URL is the source identity URL: `crates/census-crawl/src/mshsl/map.rs:207-212` (`SourceIdentity::new(Other(COACH_NAMESPACE), "<nid>:<name>").with_url(team.api_url.clone())`).
- A second evidence entry uses the team page URL `https://www.mshsl.org<alias>` (`crates/census-crawl/src/mshsl/teams.rs:15-19`) at `crates/census-crawl/src/mshsl/map.rs:218-222`; it is a secondary entry, not the payload location.
- `SourceRef` carries only `id` plus optional `url` (`crates/census-domain/src/model/provenance.rs:5-14`); `Evidence` carries `source`, `method`, `observed_on`, and optional `note` (`crates/census-domain/src/model/provenance.rs:33-40`); `Evidence::parsed` sets `note: None` (`crates/census-domain/src/model/provenance.rs:51-58`). No sha256 field exists, and the mshsl module references no `content_digest`/`sha256`/`digest` outside its test fixture (`crates/census-crawl/src/mshsl/tests.rs:49`).

Finding: for an email parsed from the `https://www.mshsl.org/api/coaches/<nid>` payload, the URL recorded as evidence is the API payload URL itself. The school/team page URL is recorded only as a secondary evidence entry. No capture sha256 is attached.

### 2. Fetch path and retention behavior

- `crates/census-crawl/src/mshsl/collect/teams.rs:50-57` -> `run.ctx.fetcher.get(url, fetch_options)`. Then `crates/census-crawl/src/net/request.rs:12-15` (`get` -> `fetch`), `crates/census-crawl/src/net/execute.rs:135-148` (`key_for` :146, `cache_paths` :147, `read_cache` :148), `:188` (`fetch_once`), `crates/census-crawl/src/net/execute/attempt.rs:28` (`fetch_once`) and `:144` (`cache_and_record(response, plan, 200, &self.stats)`).
- `crates/census-crawl/src/net/execute/cache_writer.rs:8-38` builds `CacheMeta { status, content_digest, bytes, fetched_at, ... }` (`:27-31`) and writes it via `write_cache` for HTTP 200 (`:33-37`); `crates/census-crawl/src/net/cache.rs:36-41` names the pair `<key>.body` + `<key>.meta.json`; `:43-51` derives `key` as a sha256 prefix-16 over method + url + representation; `:63-67` computes the body sha256 as `content_digest`; `:133-141` re-verifies that digest on cache read.

Finding: this JSON goes through the normal cache writer, and a normal run retains a body+meta sidecar pair for the coach API URL, with the capture sha256 in the meta. The observed `census-prototype/raw` store is a different (prototype) store: it holds 750 `www.mshsl.org__*` bodies with zero per-body sidecars and one host-level meta only (section 3).

### 3. Absence check in the retained corpus

Session commands over `/home/lewis/src/ad-law-scrape/census-prototype/raw` (ripgrep-backed search tool, 2026-10-09):

- pattern `@nevis308\.org|@hill-murray\.org` over `www.mshsl.org*` -> 0 matches.
- pattern `"field_email":"|"field_email": "|"coach_level":"|"coach_level": "` over `www.mshsl.org*` -> 0 matches.
- positive control `bbrinkman@district745\.org|athletictrainer@district745\.org|krysavy@district745\.org` over `www.mshsl.org*` -> matched `www.mshsl.org__dd61141fd6e642463c47a23f` (directory JSON:API body), proving email strings are discoverable by this method.
- glob `*api*coaches*` under `raw/` -> no files; glob `*api*519*` under `raw/` -> only `maxinfosite-api-live.dragonflyathletics.com__*` entries, no `www.mshsl.org` file.
- pattern `nevis308` over `www.mshsl.org*` -> one file, `www.mshsl.org__2f3ca684175c7598d1d25b50:5608`, an anchor `<a href="http://nevis308.org" ...>` on the Nevis school page; owner-run pattern `hill-murray\.org` likewise matches `www.mshsl.org__85112d9a2bba3a60447ba367:5612` as an anchor href.

Owner-run commands (raw outputs received 2026-10-09):

- `ls raw | grep -c '^www\.mshsl\.org__'` -> 750 (body files); `ls raw | grep -c '^www\.mshsl'` -> 751; `ls raw | grep -c '^www\.mshsl\.org__.*\.meta\.json$'` -> 0 (no per-body sidecars); all 750 bodies have mtime date 2026-09-27.
- `rg -l 'mbrick@nevis308\.org|sbenedetto@hill-murray\.org|treroundtree@aol\.com|sv313@yahoo\.com' raw` -> zero files across the 98,680-file corpus.
- file-name check over the corpus: 0 files match `api[._-]?coaches|api[._-]?519`.
- `rg -l 'field_email|coach_level' raw/www.mshsl.org*` -> 3 files: one JS bundle (240,040 bytes) and two JSON:API directory-listing payloads (16,480 and 54,323 bytes); no coach-record payload.
- the only corpus hits for `nevis308\.org|hill-murray\.org` are school-website anchor hrefs inside MSHSL school detail pages (examples above), not the coach emails.

`www.mshsl.meta.json` (read directly in this worktree, 2026-10-09): `{"url": "https://www.mshsl.org/schools/zumbrota-mazeppa-high-school", "status": 200, "bytes": 173620}` — no sha256, no timestamp.

Finding: the retained corpus contains no `/api/coaches/<nid>` payload bodies and none of the disputed addresses; those values have no retained body+meta capture pair.

### 4. The two artifact-only values and their provenance

`canonical-coaches.csv` (`/home/lewis/Downloads/midwest-tfxc-source-research/data/canonical-coaches.csv`, 5.7 MB, 31,489 lines; sha256 `e4b10432c08d809fa7373e8a3f8c3f7ca687c37368fdd5a781ca62f4c10ee28b`, owner-run `sha256sum`, 2026-10-09):

- line 6785: `coa_378a86dd3cec77d2,Madysen Brick,assistant_coach,cross_country,girls,sch_35bc61b2cd53832e,Nevis High School,MN,mbrick@nevis308.org,https://www.mshsl.org/api/coaches/591182,mshsl,2026-09-20`; the same address also at lines 7849 (`/591192`), 12150 (`/591191`), 19996 (`/591181`).
- line 9630: `coa_4f0209b5af649325,Schuyler,assistant_coach,outdoor_track,girls,sch_ab397156637d6020,Hill-Murray School,MN,sbenedetto@hill-murray.org,https://www.mshsl.org/api/coaches/593815,mshsl,2026-09-20`; the same address also at line 10386 (`/593814`).

Both values are traceable only to this research artifact: neither appears in the retained corpus (owner `rg -l` above), in any other file of `~/Downloads/midwest-tfxc-source-research/data` (session search for the four-value pattern matched only `canonical-coaches.csv`), or anywhere in this worktree.

The other two disputed values (`sv313@yahoo.com`, `treroundtree@aol.com`) appear in no searched source: owner `rg -n 'treroundtree|sv313'` over the whole CSV -> zero matches (exit status 1); session search over the research data directory -> zero matches; owner corpus search -> zero files; worktree-wide session search -> zero matches.

### 5. Requirement

A published coach email must be traceable to a retained body+meta capture pair (body containing the address, meta sidecar carrying the capture sha256 and fetch time). Values traceable only to a research artifact (`canonical-coaches.csv`) or only to the delivered workbook must be treated as unverifiable.
