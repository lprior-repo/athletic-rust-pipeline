# Wayzata capture notes (2026-10-11)

## What changed on the live source

`https://www.wayzataresults.com/links/{slug}` (the only meet link shape on the public
schedule pages) is a 302 to `https://results.wayzatatiming.com/meets/{id}`. That page is an
**AthleticLIVE SPA** (JS chunks under `livestatic.athletic.net`, tenant config
`assets/sites/wayzata/config.json`, `machineName: wayzata`). The DOM served is a shell; event
and result content is fetched from the AthleticLIVE data plane, not from server-rendered HTML
tables. A live HTML results table capture does not exist because the source no longer serves one.

Fixture probe: meet 76408 (from `/links/v70kmd`) renders "There are no events in this meet" —
it is a future meet (schedule is forward-dated). Completed meets do carry data.

## Data plane (verified by capture, same protocol as `fixtures/athleticlive/`)

1. Tenant resolution + meet doc: `POST https://search.athletic.live/wayzata_meet_list/_search`
   (`{"query":{"term":{"i":<athleticlive meet id>}}}`). Hit `_index == wayzata_meet_list`;
   `_source.i` = AthleticLIVE meet id; `_source.ani` = Athletic.net meet id; `sd`/`ed` start/end.
   The 2026-10-11 SPA URL id 79158 resolves here (Packer Invitational, MN, 2026-10-13, ani 283807).
2. Event summary: `GET https://s-gke-usc1-nssi3-33.firebaseio.com/meet_{id}/event_summary.json?ns=trackmeet-io`
3. Event (results) document: `GET https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/{event_id}`
   carrying `_source.i` (event), `_source.mi` (meet), `_source.r` (result rows), `irds`, `itds`, `spd`.

## Fixture set (this directory)

| File | Supports |
|---|---|
| `xc_2026_schedule.full.html`, `track_2026_schedule.full.html` | complete schedule structure incl. all `a[href^="/links/"]` rows and dates (2026-10-11 capture; 110 XC meet links) |
| `meet-page-76408-postload.html` | rendered SPA shell evidence (redirect + no server-side results) |
| `wayzata-config.json` | tenant identity (elasticSearchIndex wayzata, esReportIndex wayzata_reports) |
| `meet-doc-79065.json` | completed-meet doc in tenant wayzata (Hopkins Harvest Hoedown, 2026-10-09) |
| `event-summary-meet-79065.json` | three individual events of meet 79065 |
| `ind-res-list-2930238.json`, `ind-res-list-2930235.json` | real per-event result documents (`_source.r` rows) for parsing/normalization work |

Rate policy: sequential requests, >=2 s apart, descriptive UA
`athletic-rust-pipeline-fixture-capture/1.0`, no crawling. `captures.log` + `*.meta.json` carry
url/status/bytes/sha256/fetched_at per file.

Pre-existing trimmed schedules (`xc_2026_schedule.html`, `track_2026_schedule.html`, 2026-09-20)
are retained unchanged.
