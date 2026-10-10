# AthleticLIVE fixtures

Offline evidence for the three AthleticLIVE provider families (`athleticlive`,
`athleticlive_results`, `athleticlive_athletes`; slugs at
`crates/census-service/src/cli/provider.rs:129-134`). Every file here is either a
raw response body captured from the live hosts or a deterministic projection of
one, with a per-capture meta beside it.

## Materialize

From the repository root (relative `captures` paths are opened against the CWD):

    bash crates/census-crawl/tests/fixtures/athleticlive/capture.sh

14 requests (15 if the POST listing is refused and the GET fallback runs),
sequential, >=2 s apart, no crawling, no retries. The script refuses to
overwrite an existing capture set unless passed `--overwrite`. It writes the
bodies, the metas, `captures.log` (JSONL), `PROVENANCE.json`, the derived harvest
CSV and the derived replay manifest, then prints the request tally and a
`sha256sum` line per file.

`robots.txt` is captured for provenance only: this pipeline no longer gates on
robots admission (owner directive 2026-10-10). The engineered rate limits
(sequential requests, >=2 s spacing, bounded page sizes) stay in force.

## Inventory

| File | Kind | Consumer |
|---|---|---|
| `capture.sh` | script | operator; materializes everything below |
| `meet-list-wi-mn-2025.request.json` | request body | `POST https://search.athletic.live/*_meet_list/_search` |
| `meet-list-wi-mn-2025.json` | response | meet discovery: Wisconsin + Minnesota, `sd` in `[2025-08-01, 2025-12-01)` |
| `meet-doc-55746.request.json` | request body | `POST .../*_meet_list/_search`, `term {"i": 55746}` |
| `meet-doc-55746.json` | response | the fixture meet: tenant, name, state, date, Athletic.net meet id |
| `event-summary-meet-55746.json` | response | `athleticlive_results` manifest input (`summary`) |
| `ind-res-list-<event>.json` | response | `athleticlive_results` manifest input (`documents`) |
| `athlete-list-meet-55746.request.json` | request body | the collector's own `batch_query` (`athleticlive_athletes/mod.rs:34-45`) |
| `athlete-list-meet-55746.json` | response | athlete seed discovery for the fixture meet |
| `harvest-wi-mn-2025.csv` | derived | published-meet CSV shape parsed by `athleticlive::parse_meets_csv` |
| `manifest-meet-55746.json` | derived | `athleticlive_results --input` |
| `robots-*.txt` | response | provenance only |
| `<file>.meta.json` | derived | per capture: url, method, status, bytes, `content_digest`, `fetched_at`, `content_type`, plus the evidence aliases `sha256` / `fetched_at_utc` / `time_seconds` |
| `captures.log` | derived | JSONL ledger: one row per request plus `derived:true` rows |
| `PROVENANCE.json` | derived | the ledger plus the generator command |

`meets-sample.csv` predates this set and is not touched by `capture.sh`.

## Contract 1 — `athleticlive_results` manifest

`collect_manifest` reads `--input <manifest.json>` and issues no request
(`crates/census-crawl/src/athleticlive/results/manifest.rs:118-152`, refusal at
`:122-124`). Shape (`manifest.rs:12-38`, decoded by `entry_options` `:68-116`):

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
  "captures": {"<the same path string>": {"url": "...", "method": "GET", "status": 200,
    "content_digest": "<64 hex>", "bytes": 1234, "fetched_at": "2026-10-10T00:00:00Z",
    "content_type": "application/json"}}
}]}
```

`summary`, `documents`, `standings` and `captures` are `#[serde(default)]`
(`manifest.rs:23-31`), so an entry may omit `standings`.

Rules the fixtures satisfy and a new capture must keep:

- `captures` is keyed by the **path string** used in `summary` / `documents` /
  `standings[].path`, not by URL (`results/run/inputs.rs:36-57`, lookup at
  `:52`; `manifest.rs:110-113`).
- A capture is only read when its meta satisfies `capture::freeze`'s validation:
  `method == "GET"`, `status == 200`, `bytes` <= 1 MiB for documents/summary,
  `fetched_at` an RFC 3339 instant, `url` a public absolute `http(s)` URL
  (`athleticlive/capture.rs:29-61`; `results/run/capture.rs:4`).
- A capture with no producer meta is rejected
  (`results/run.rs:109-118`).
- `url` must equal the canonical wire URL for the role, or the read is refused:
  summary `https://s-gke-usc1-nssi3-33.firebaseio.com/meet_<id>/event_summary.json?ns=trackmeet-io`,
  document `https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/<event_id>`,
  standings `.../meet_<id>/liveRunStandings/<run_id>.json?ns=trackmeet-io`
  (`athleticlive/wire.rs:9-26`; `results/run.rs:93-97,167,223`; `results/run/capture.rs:25-33`).
- The document body must carry `_source.i == <event_id>` and
  `_source.mi == <meet_id>` (`results/run.rs:157-171`).
- Manifest limits: 8 MiB, 8192 meets, 8192 records/entry, path <= 4096 bytes,
  `tenant` non-empty, <=128 bytes, `[A-Za-z0-9_-]` only, name <= 4096, date <= 64
  (`results/manifest/admission.rs:4-5,25-42,45-66`).
- Retained receipts are scanned for an owner conflict before a capture is
  projected: a capture that previously belonged to another meet or provider is
  refused (`results/run/receipts.rs:6-37`).
- An event the summary lists but `documents` omits is owed, not ignored
  (`results/run/inputs.rs:82-98`); `manifest-meet-55746.json` ships every listed
  individual event, so a replay owes nothing.

`<file>.meta.json` is a **superset** of `CacheMeta`
(`crates/census-crawl/src/net/cache.rs:14-33`): unknown fields are ignored by
serde, so a test can either deserialize the file straight into `CacheMeta` or
inline it into a manifest's `captures`. `results/tests/captures.rs:44-86`
synthesizes metas the same way and is the pattern to copy.

## Contract 2 — `athleticlive` (meets) CSV

`athleticlive` requires `--input <csv>` **and** `--input-metadata <meta.json>`
(`cli/provider.rs:44-49`; `athleticlive.rs:46-55`). Columns
(`athleticlive/parse.rs:88-103` required, `:117-155` decode): required `tenant`,
`athleticlive_meet_id`, `name`, `state`, `start`; optional
`athleticnet_meet_id`, `city_state`, `end`, `has_results`. `start`/`end` are read
as their 10-char `YYYY-MM-DD` prefix (`parse.rs:24-30`); `state` must parse as a
jurisdiction; `--states` filters rows (`athleticlive.rs:163-165`); years outside
2015..=2030 are refused (`parse.rs:18-22`, `athleticlive.rs:174-188`); rows whose
date is after `--as-of` are owed, not imported (`athleticlive.rs:189-191`).

`harvest-wi-mn-2025.csv` is the derived row shape: `tenant` is the ES index
prefix (`<tenant>_meet_list`), `athleticlive_meet_id` is the doc's `i`,
`athleticnet_meet_id` is `ani` when numeric, `name` is `n`, `city_state` is `ls`,
`state` is `lsa`, `start` is `sdy` (falling back to `sd[0:10]`). `has_results` is
left empty: no verified ES field maps to it.

Note the validation asymmetry: `freeze` accepts only `method == "GET"`
(`athleticlive/capture.rs:42-50`), and the listing response above is a **POST**
body, so a CSV projected from it cannot carry an honest producer meta today
(`csv_capture.rs:20-40`). `live.athletic.net/meets.csv` is not a publisher
either: it answers with the Angular shell (`text/html`, title `AthleticLIVE`),
verified 2026-10-09. The meet family therefore needs either a GET-reachable
listing surface or an arm change; the captured listing is the discovery evidence
for both.

## Contract 3 — `athleticlive_athletes` seed discovery

`discover` seeds from the store, not the network: merged `CanonicalMeet` records
that carry a `SourceNamespace::TimerMeet { provider }` identity
(`athleticlive_athletes/discover.rs:7-45`, `targets.rs:49-66`), filtered by
`--states`, dates 2015..=2030 (`targets.rs:22-30`), sorted by (state, date, meet
id) (`targets.rs:67-72`). It then issues one `POST athlete_list/_search` per meet
with `batch_query` (`athleticlive_athletes/mod.rs:34-45`): page size 2000,
`terms mi`, `terms y ["11","12","JR","SR","Jr","Sr"]`, `_source` limited to
`["i","n","y","g","mi","ani","t"]`, window capped at 10 000 rows
(`mod.rs:20-23`); `--limit` owes the tail
(`mod.rs:56-58`).

So a bounded jurisdiction run is: meets first (`athleticlive` CSV, which mints
the `TimerMeet` identity at `athleticlive/meets.rs:42-48`), then athletes over
the same store. This fixture set covers one meet end to end: the meet doc, the
summary, all six event documents and the first athlete page.

## Provenance rules

- Bodies are byte-frozen: `stdout` of the capture script prints the ledger and a
  `sha256sum` per file. `fetch` timestamps are the only run-dependent values.
- Live responses churn (`ua`, counts, page order), so a re-capture is a new
  evidence set, not a byte-identical replay; the metas pin what was read when.
- `search.athletic.live` serves no robots policy (403 ES error body) and the
  blob/RTDB roots publish none; the captured `robots-*.txt` files record that as
  bytes, and admission no longer depends on them.
