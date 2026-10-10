# SOURCE_REPORT — AthleticLIVE (registry families `athleticlive`, `athleticlive_results`, `athleticlive_athletes`)

Lane: `research/sources/athleticlive` · evidence fetched 2026-10-09 UTC · fixtures:
`crates/census-crawl/tests/fixtures/athleticlive/` · sibling coverage:
`research/sources/timing-providers-national/` (platform, data model, samples) and
`research/sources/national-aggregators/` (hosts, tenant inventory, field schema).

Scope: arm the three AthleticLIVE provider families for their applicable states. This report carries
the collector contract (field names, id shapes, hosts, limits, refusal paths) that a coder needs, the
live evidence that the discovery surfaces still answer, and a fixture set that replays offline.

## 0. Status and verification statement

`[V]` marks facts verified by a request issued from this session or by a cited repository line.
`[EXEC-PENDING]` marks the capture bodies and digests that this session could not write: it has no
shell (its tool set is read/grep/glob/write) and `proc://` exposes no usable job surface, so `curl`,
`sha256sum` and byte counts are unavailable to it. Those bytes are materialized by one deterministic
script, `crates/census-crawl/tests/fixtures/athleticlive/capture.sh`, whose 14 requests, expected
statuses and expected tallies are specified below; the metas the script writes carry `url`, `method`,
`status`, `bytes`, `content_digest` (SHA-256), `fetched_at`, `content_type` plus the evidence aliases
`sha256` / `fetched_at_utc`.

Requests this session actually issued (GET through the harness URL reader, spaced ≥ 1 s, no crawling):

| # | Request | Observed |
|---|---|---|
| 1 | `GET https://search.athletic.live/*_meet_list/_search?q=lsa.keyword:Wisconsin AND sd:[2025-08-01 TO 2025-12-31]&size=3` | 200; `hits.total.value` 208; `_shards` 259/259 successful; a meet doc with `xcp` event ids |
| 2 | `GET https://search.athletic.live/athlete_list/_search?q=mi:55746&size=1` | 200; `hits.total.value` 318; one decoded doc (below) |
| 3 | `GET .../athlete_list/_search?q=mi:55746 AND y:SR&size=0` | 200; 0 (no senior tokens at this meet) |
| 4 | `GET .../athlete_list/_search?q=mi:55746 AND y:11&size=0` | 200; 70 |
| 5 | `GET .../athlete_list/_search?q=mi:55746 AND y:12&size=0` | 200; 60 |
| 6 | `GET .../*_meet_list/_search?q=lsa.keyword:(Wisconsin OR Minnesota) AND sd:[2025-08-01 TO 2025-12-01]&size=50&track_total_hits=true` | 200; `hits.total.value` 553; page 1 contains meet 55746 under `_index` `athletic_timing_meet_list` |
| 7 | `GET https://s-gke-usc1-nssi3-33.firebaseio.com/meet_55746/event_summary.json?ns=trackmeet-io` | 200; 6 individual XC events, ids 2121672–2121677 |
| 8 | `GET https://live.athletic.net/meets.csv` | 200 `text/html`: the Angular shell (title `AthleticLIVE`), not a CSV |

## 1. Applicability

Both slugs are registered for the same 12 states — `crates/census-crawl/src/applicability/table/data.rs:12-26`
(`athleticlive`) and `:31-45` (`athleticlive_athletes`): **Alabama, Iowa, Illinois, Indiana, Kansas,
Michigan, Minnesota, Missouri, North Dakota, Nebraska, Ohio, Wisconsin**. The registry's own gap notes
record the two refusals this lane arms: "qualified meet-results collector lacks durable jurisdiction
artifact discovery" and "qualified athlete-index collector lacks bounded jurisdiction seed discovery".
`athleticnet` is `CENSUS_SCOPE` (`data.rs:50-51`) and is the cross-source join key here (`ani` fields).

## 2. Hosts and endpoints

From `crates/census-crawl/src/athleticlive/wire.rs:1-26` — the collector's canonical URLs, which a
capture's `url` must equal exactly (`results/run/capture.rs:25-33`):

| Role | URL |
|---|---|
| Event document | `https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/<event_id>` (`wire.rs:9-11`) |
| Event summary | `https://s-gke-usc1-nssi3-33.firebaseio.com/meet_<meet_id>/event_summary.json?ns=trackmeet-io` (`wire.rs:13-15`) |
| Standings | `.../meet_<meet_id>/liveRunStandings/<run_id>.json?ns=trackmeet-io`; `run_id` ≤ 16 bytes, digits and `-` only (`wire.rs:17-21`, `:23-26`) |

Discovery surfaces (not in `wire.rs`, used by the fixtures and by sibling lanes):

| Surface | Shape |
|---|---|
| Meet listing | `POST https://search.athletic.live/*_meet_list/_search`; one doc per `<tenant>_meet_list` index; 259 shards `[V]` |
| Athlete index | `POST https://search.athletic.live/athlete_list/_search` (`athleticlive_athletes/mod.rs:23`) |
| Tenant inventory | `data/athleticlive-tenant-inventory.csv` (256 tenants, 153,524 meet docs) — sibling lane |
| Static/config | `livestatic.athletic.net/assets/sites/<tenant>/config.json` — sibling lane, not exercised here |

The tenant is the ES index prefix, not the doc's `e` field: meet 55746 sits in
`athletic_timing_meet_list` while its doc says `"e": "athletictiming"`, `"tna": "Duluth Timing and Events"` `[V]`.
The existing CSV fixture uses the index-prefix convention (`live_results`, `palatine`, `athleticlive`),
so a harvest must take the tenant from `_index` and strip the `_meet_list` suffix.

A note on query dialects: an `sd` **date range** and the `lsa.keyword` **terms** filter both answer
across the 259-shard wildcard `[V]`, and the state-assoc-plains lane used `match {"ls":"ND"}` +
`range {"md": …}` for the same index family. Sorting is not exercised by this lane.

## 3. Family 1 — `athleticlive` (meets)

Entry point: `athleticlive::collect` (`athleticlive.rs:46-55`) → `csv_capture::open`
(`csv_capture.rs:20-40`) → `collect_csv` (`athleticlive.rs:57-96`).

- **Input**: `--input <csv>` plus **mandatory** `--input-metadata <CacheMeta json>`
  (`cli/provider.rs:44-49` for the flag, `:82-99` for the wiring; `athleticlive.rs:27-34` for
  `Options`). Without `--input` the arm refuses by name (`athleticlive.rs:47-52`).
- **Producer meta validation**: the CSV is frozen through `capture::freeze` (`capture.rs:11-27`): path
  ≤ 4096 bytes (`:17-19`), producer meta required (`:20`), then `validate` (`:29-61`) requires
  meta field bytes ≤ 4096 (`:36-38`), `bytes` ≤ 32 MiB (`csv_capture.rs:7`; `capture.rs:39-41`),
  `method == "GET"` + `status == 200` + RFC 3339 `fetched_at` (`:42-50`), and a public absolute
  `http(s)` URL with no credentials (`:51-59`).
- **CSV columns** (`athleticlive/parse.rs:88-103` required list, `:117-155` row decode): required
  `tenant`, `athleticlive_meet_id`, `name`, `state`, `start`; optional `athleticnet_meet_id`,
  `city_state`, `end`, `has_results`. `start`/`end` are taken as their 10-char `YYYY-MM-DD` prefix
  (`parse.rs:24-30`); `state` must parse as a jurisdiction; fields over 4096 bytes are refused
  (`athleticlive.rs:151-158`); `MAX_ROWS` bounds the walk (`athleticlive.rs:91-94`, `:136-149`).
- **Selection**: `--states` keeps only matching rows (`athleticlive.rs:163-165`); `--limit` owes the
  tail (`:166-173`); years outside 2015..=2030 are refused (`parse.rs:18-22`, `athleticlive.rs:174-188`);
  rows dated after `--as-of` are owed, not imported (`athleticlive.rs:189-191`).
- **Landing**: each retained row mints/projects a canonical meet and appends it once
  (`append_row_once("athleticlive_meets_csv_projection_v1", Table::Meets, &meet)`,
  `athleticlive.rs:196-207`, append at `:201`). The meet carries
  `SourceIdentity::new(TimerMeet { provider: tenant }, athleticlive_meet_id)` (`meets.rs:42-48`) plus an
  Athletic.net identity built from `athleticnet_meet_id` (`meets.rs:49-56`). Evidence binds the CSV's
  URL and `capture sha256=<digest>` (`csv_capture.rs:43-48`), and `acquired_on` is the meta's fetch date
  (`csv_capture.rs:27-30`).
- **What closes the gap**: this arm has no network discovery of its own. The durable discovery surface
  is the ES meet listing captured here; a projection of it into the CSV column shape is
  `harvest-wi-mn-2025.csv` `[EXEC-PENDING]`.

**Constraint to resolve in code (not in evidence)**: a listing response is a POST body, while
`freeze` admits only `method == "GET"` captures (`capture.rs:42-50`). So a CSV projected from the ES
listing cannot carry an honest producer meta, and `live.athletic.net/meets.csv` publishes no CSV at all
`[V]`. Arming this family durably therefore needs either a GET-reachable listing surface or an arm that
ingests the listing directly (the fixture set supports both: raw POST response, its projection, and the
exact request body).

## 4. Family 2 — `athleticlive_results` (manifest replay)

Entry point: `collect_manifest` (`results/manifest.rs:118-152`) with `--input <manifest.json>`
(`ManifestOptions` at `manifest.rs:40-46`). It issues **no** request: every byte comes from operator
captures (refusal message at `manifest.rs:122-124`).

Manifest shape (`manifest.rs:12-38`: `ManifestFile` `:12-15`, `CaptureEntry` `:17-32`,
`StandingsEntry` `:34-38`; decoded by `entry_options` `:68-116`):

```json
{"meets": [{
  "athleticlive_meet_id": 55746,
  "tenant": "athletic_timing",
  "name": "North Shore Challenge",
  "state": "Minnesota",
  "date": "2025-08-28",
  "summary": "crates/census-crawl/tests/fixtures/athleticlive/event-summary-meet-55746.json",
  "documents": ["crates/census-crawl/tests/fixtures/athleticlive/ind-res-list-2121672.json"],
  "standings": [{"run_id": "1-1", "path": "<path>"}],
  "captures": {"<the exact path string>": {"url": "…", "method": "GET", "status": 200,
    "content_digest": "<64 lowercase hex>", "bytes": 1234, "fetched_at": "2026-10-10T00:00:00Z",
    "content_type": "application/json"}}
}]}
```

`summary`, `documents`, `standings` and `captures` are all `#[serde(default)]` (`manifest.rs:23-31`),
so a minimal entry (meet id, tenant, name, state, date + summary/documents/captures) parses.

Rules, each with its refusal path:

| Rule | Source |
|---|---|
| `captures` is keyed by the **path string** used in `summary`/`documents`/`standings[].path` | `inputs.rs:36-57` (lookup at `:52`), `manifest.rs:110-113` |
| A capture with no meta is rejected ("capture has no physical producer metadata") | `run.rs:109-118` |
| `capture::freeze` validation: GET, 200, RFC 3339 `fetched_at`, public http(s) URL, ≤ 1 MiB per document/summary/standings capture | `capture.rs:29-61`, `run/capture.rs:4` |
| `url` must equal the canonical URL for the role | `run/capture.rs:25-33`; `run.rs:93-97` (summary), `:167` (document), `:223` (standings) |
| Document body must carry `_source.i == <event_id>` and `_source.mi == <meet_id>` | `run.rs:157-171` (meet check `:168-170`) |
| A summary-listed event with no document is **owed**, counted in `events_unfetched` | `inputs.rs:82-98`; counter `map.rs:43`, folded at `results.rs:260-262` |
| Retained receipts are scanned: a capture that belonged to another meet/provider is refused | `receipts.rs:6-37`; receipt shape `:39-90` |
| Limits: manifest ≤ 8 MiB (`admission.rs:4`), ≤ 8192 meets/records (`:5`, `:25-42`), path ≤ 4096, `tenant` ≤ 128 bytes `[A-Za-z0-9_-]`, name ≤ 4096, date ≤ 64 | `admission.rs:45-66` |
| `--states` filters entries by declared state; identical owners merge; conflicting owners/metas are refused | `selection.rs:27-31`, `:38-82`, `:84-103`; withheld tail `:105-126` |
| State strings parse as jurisdictions; unknown ones refuse by name | `manifest.rs:73-79`; `integration3.rs:136-152` |

Landing: one canonical meet per owner with `TimerMeet { provider: tenant }` and the meet id as the
identity (`run.rs:256-270`), `observed_on` = the capture's `fetched_at`, rows keyed
`athleticlive:<event_id>:row<N>:…` (`absorb.rs:26-28`; `map_rows/performance.rs:25-30`), and the receipt
key is `<meet_id>:<role>:<projection digest>` (`receipts.rs:92-134`, key at `:131`). The projection
phases are `athleticlive_results_capture_v1` / `…_effect_v2` / `athleticlive_projection_v4` and the
parser id is `athleticlive_results_v4` (`results.rs:16-20`); `SOURCE_ID` is `athleticlive_results`
(`map.rs:11`). The route is idempotent: a repeated import appends no second copy (`integration3.rs:45-133`).

The in-repo wiring pattern for tests is `results/tests/captures.rs:44-86`: read the body, derive the
canonical URL from the body's own ids, and synthesize `CacheMeta` with `content_digest(body)` (`:70-86`).
The new fixtures make that unnecessary — each `<file>.meta.json` is a superset of `CacheMeta`
(`net/cache.rs:14-33`; serde ignores the extra evidence aliases), so it can be deserialized directly or
inlined into `captures`.

## 5. Family 3 — `athleticlive_athletes` (bounded jurisdiction seed discovery)

Entry point: `collect` (`athleticlive_athletes/mod.rs:47-76`) with
`Options { limit, refresh, observed_on, states, school_names }` (`mod.rs:26-32`; the provider arm is
`cli/provider.rs:133-134`).

Discovery does **not** touch the network first: `discover` (`discover.rs:7-45`) walks merged
`CanonicalMeet` records in the store and keeps those carrying a `SourceNamespace::TimerMeet { provider }`
identity (`targets.rs:49-66`), filtered by `--states`, with meet years 2015..=2030
(`targets.rs:22-30`), sorting by (state, date, meet id) (`targets.rs:67-72`) and owing when nothing
qualifies. The seed chain is therefore: meets must already be in the store with a tenant identity —
which family 1 mints (`meets.rs:42-48`) or a manifest replay mints (`run.rs:256-270`).

Each target then costs one ES page: `ENDPOINT = https://search.athletic.live/athlete_list/_search`
(`mod.rs:23`), body exactly `batch_query` (`mod.rs:34-45`): `size` = 2000, `from` paged,
`track_total_hits`, `terms mi` (meet ids), `terms y ["11","12","JR","SR","Jr","Sr"]`, `_source` limited
to `["i","n","y","g","mi","ani","t"]`, window capped at 10 000 rows (`mod.rs:20-21`). `--limit` owes
each target it does not reach, with locator `<endpoint>#meet=<id>&from=0` (`mod.rs:56-58`).

So the gap this lane closes is the **seed**, not the page: any store-meet with a tenant identity can be
seeded, and the absence of meets for a state is what leaves the family refusing. The captured meet
proves the page: 318 athlete docs for meet 55746 `[V]`, of which 70 are grade `11` and 60 are grade `12`,
so the graded page is ~130 docs — non-empty for a middle/high-school XC meet.

## 6. Worked example — meet 55746, decoded

Meet doc (index `athletic_timing_meet_list`, `_id` 55746) `[V]`:

```json
{"i": 55746, "ani": 259201, "n": "North Shore Challenge", "ls": "Grand Marais, MN",
 "lsa": "Minnesota", "sd": "2025-08-28T04:00:00Z", "sdy": "2025-08-28", "ed": null,
 "e": "athletictiming", "tna": "Duluth Timing and Events", "tz": "America/Chicago",
 "us": "http://live.tf/lqitbv", "mvs": ["middleSchool", "highSchool"],
 "xcp": [{"i": 2121675, "n": "Boys 2600 Run Junior High", "enu": 1},
         {"i": 2121672, "n": "Girls 2600 Run Junior High", "enu": 2},
         {"i": 2121674, "n": "Boys 4000 Run JV", "enu": 3},
         {"i": 2121676, "n": "Girls 4000 Run JV", "enu": 4},
         {"i": 2121673, "n": "Boys 5000 Run Varsity", "enu": 5},
         {"i": 2121677, "n": "Girls 5000 Run Varsity", "enu": 6}]}
```

Decoded into each contract (columns in the order of the existing fixture,
`meets-sample.csv`: `tenant,athleticlive_meet_id,athleticnet_meet_id,name,city_state,state,start,end,has_results`):

- **Meet CSV row**:
  `athletic_timing,55746,259201,North Shore Challenge,"Grand Marais, MN",Minnesota,2025-08-28,,`
  — `end` empty (`ed` null), `has_results` empty (no verified ES field maps to it). The sibling fixture
  writes the full `sd` instant in `start`; both forms parse, `parse.rs:24-30` keeps the date.
- **Manifest entry**: `athleticlive_meet_id` 55746, `tenant` `athletic_timing`,
  `state` `Minnesota`, `date` `2025-08-28`, `name` `North Shore Challenge`
  (state parses at `manifest.rs:73-79`; date within 2015..=2030 and ≤ `--as-of`).
- **Summary** (`event-summary-meet-55746.json` `[V]`): six `Individual` XC events, keys
  `Individual-2121672 … Individual-2121677`, each with `i`, `ec`, `rui` (`1-1` … `6-1`), `sk`
  (2600/4000/5000), `eo` (Junior High / JV / Varsity), `gl`, `resd` done timestamps and `xc: true`.
  Relays are excluded from the frontier by the parser; here there are none.
- **Documents**: `ind-res-list-<id>.json` for each listed id, i.e. the blob body with
  `_source.i == <event_id>` and `_source.mi == 55746` `[EXEC-PENDING]`. A decoded row of the same
  document family (Iowa fixture, 136 rows) is asserted in `results/tests/mod.rs:180-196`: place 1,
  mark `18:20.7`, `Mark::TimeSeconds(ExactSeconds::parse("1100.7"))`, 3 splits, event `2_150_205` of
  meet 58504. Row-level keys (`m`, `im`, `y`, `a.ani`, `t.ani`) are decoded key-by-key in
  `research/sources/timing-providers-national/` §3 and `schema.json` (`national-aggregators`).
- **Athlete page** (`athlete-list-meet-55746.json` `[EXEC-PENDING]`): the `batch_query` response for
  meet 55746; one decoded doc from the same index `[V]`:

```json
{"i": 40782829, "n": "Hunter Bement", "fn": "Hunter", "l": "Bement", "y": "8", "g": "Male",
 "mi": 55746, "ani": 29518071,
 "t": {"i": 1295814, "f": "Mesabi East", "n": "Mesabi East", "mi": 55746, "ani": 12641,
       "lg": "https://lh3.googleusercontent.com/…=s80-c", "xc": 1, "hbl": true, "ani": 12641}}
```

  The mapper reads the athlete's own `ani` as its Athletic.net id, and `t.ani` as the **team's**
  Athletic.net id (`athletes/parse.rs:23-36`, `:69-72`; `map.rs:215-243` binds the team identities), so
  the join keys the athlete family consumes are `i` (AthleticLIVE), `ani` (Athletic.net athlete) and
  `t.i` / `t.ani` (team, both namespaces).

## 7. Fixture provenance

Directory: `crates/census-crawl/tests/fixtures/athleticlive/`. Every body has a sibling
`<name>.meta.json` (url, method, status, bytes, `content_digest`, `fetched_at`, `content_type`,
`sha256`, `fetched_at_utc`, `time_seconds`); `captures.log` is the JSONL ledger and `PROVENANCE.json`
the aggregate.

| File | Method | URL | Expected |
|---|---|---|---|
| `robots-search-athletic-live.txt` | GET | `https://search.athletic.live/robots.txt` | 403 ES error body; no policy published |
| `robots-www-athletic-live.txt` | GET | `https://www.athletic.live/robots.txt` | no policy; status recorded |
| `robots-blob-athletic-live.txt` | GET | `https://athleticlive.blob.core.windows.net/robots.txt` | no policy; status recorded |
| `robots-rtdb-firebaseio.txt` | GET | `https://s-gke-usc1-nssi3-33.firebaseio.com/robots.txt` | no policy; status recorded |
| `meet-list-wi-mn-2025.json` | POST | `https://search.athletic.live/*_meet_list/_search` (body `meet-list-wi-mn-2025.request.json`) | 200; `hits.total.value` 553 on 2026-10-09 `[V]`; ≤ 50 docs (`size` 50) |
| `meet-doc-55746.json` | POST | same pattern (body `meet-doc-55746.request.json`, `size` 2) | 200; first hit is meet 55746 (asserted) |
| `event-summary-meet-55746.json` | GET | `…/meet_55746/event_summary.json?ns=trackmeet-io` | 200; 6 individual events `[V]`; `type == object` asserted |
| `ind-res-list-<event>.json` ×6 | GET | `…/$web/ind_res_list/_doc/<event_id>` | 200; `_source.i`/`_source.mi` checked per file |
| `athlete-list-meet-55746.json` | POST | `https://search.athletic.live/athlete_list/_search` (body `athlete-list-meet-55746.request.json`) | 200; `hits.total.value` ≈ 130 (70 G11 + 60 G12 `[V]`); > 0 asserted |
| `harvest-wi-mn-2025.csv` | derived | projection of `meet-list-wi-mn-2025.json` | one row per listing doc, `meets-sample.csv` columns; `start` = `sdy` (fallback `sd[0:10]`), `tenant` from `_index` minus `_meet_list` |
| `manifest-meet-55746.json` | derived | manifest for the fixture meet | summary + all six documents + their metas; no `standings` key; replay owes nothing |

The script asserts each expected status, that the fixture meet's doc carries `i == 55746`, that the
summary is an object with ≥ 1 individual event, that every fetched document carries the matching
`_source.i`/`_source.mi`, and that the athlete page is non-empty. It refuses to overwrite an earlier
capture set unless passed `--overwrite`. Because live responses churn (`ua`, counts, page order),
a re-capture is a new evidence set; the metas pin what was read when.

## 8. robots.txt

Captured as bytes and digest in the fixture set above, together with the status that each host
actually serves. `search.athletic.live` answers the path with an Elasticsearch `security_exception`
body (403) and neither the blob store nor the RTDB publishes a policy `[V]`; the sibling lane recorded
the same for `www.athletic.live`
(`research/sources/national-aggregators/samples/athleticlive/al-athleticlive_blob_core_windows_net-robots.txt`,
`al-liveathleticnet-robots.txt`, `al-search-robots.txt`, `samples/CAPTURES.md`). Per the owner directive
of 2026-10-10 this pipeline no longer gates admission on robots; the engineered rate limits stay:
sequential requests, ≥ 2 s spacing in the capture script, bounded page sizes (`size` ≤ 50 for listings,
the collector's own 2000-row athlete page), no retries, no crawl of the SPA.

## 9. Open gaps

1. **`athleticlive` has no live discovery path of its own** — its CSV arrives from outside, and the
   only publisher-shaped URL (`live.athletic.net/meets.csv`) serves the SPA shell `[V]`. The ES
   listing captured here is the discovery surface to wire; `capture.rs:42-50` (GET-only validation)
   is the code constraint that decides how.
2. **`has_results` mapping unverified** — no ES field is confirmed to mean "has results"; the derived
   CSV leaves the column empty rather than guessing (`sur` is 1 on a cancelled meet `[V]`).
3. **Standings channel not captured** — `standings_url` accepts `rui` tokens like `1-1` and the
   manifest can list `standings` entries, but no standings body is in this fixture set; a meet whose
   `liveRunStandings` is still published would settle it.
4. **Grade-token vocabulary** — meet 55746 carries numeric grades (7…12) and no `SR`/`JR` tokens, so
   the collector's six-token filter is required; other tenants use `SR`/`JR` (sibling lane). Coverage
   of `y` across tenants remains an open question for the athlete family.
5. **`--states` on the meets arm filters, it does not refuse** — an out-of-registry state in the CSV is
   silently dropped (`athleticlive.rs:163-165`), unlike the manifest route which parses and refuses
   (`manifest.rs:73-79`).
6. **Fixture bytes pending one executor run** `[EXEC-PENDING]` — see §0; everything else in this
   report is either a request observed here or a repository citation.

## 10. Cross-references

- `research/sources/timing-providers-national/SOURCE_REPORT.md` §3 and `ATHLETICLIVE_DATA_MODEL.md` —
  platform census (256 tenants / 153,774 meet docs), row-key decode, request-cost model, earlier
  ES/blob/RTDB samples under `samples/` (including `robots-search.athletic.live.txt`).
- `research/sources/national-aggregators/schema.json` (the `athleticlive` block, from `:67`) — hosts,
  identifier shapes, meet/event/result field inventories; `data/athleticlive-tenant-inventory.csv`.
- `research/sources/state-assoc-plains/SOURCE_REPORT.md` — an ND tenant (`heros`) listing capture using
  the `ls`/`md` dialect, plus the RTDB standings surface for a state XC meet.
- `research/sources/athleticnet/SOURCE_REPORT.md` §21.1 — the 12-state `athleticlive-meet-harvest` join
  (`data/athleticnet-meet-seeds.csv`: 9,844 rows, 9,747 carrying both an AN meet id and AthleticLIVE ids).
