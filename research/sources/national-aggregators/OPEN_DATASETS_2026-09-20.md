# Bulk and open datasets — 2026-09-20 survey record (dated evidence)

Dated reconnaissance inherited from the deleted root doc `SOURCES_SURVEY.md` (§8 dataset census, §12
adversarial checks). Those probes ran 2026-09-20 with the provider-level `web_search` tool unavailable,
so Kaggle rows were probed through the public dataset API and via `curl` on the dataset zip endpoints.
Nothing here was re-measured after that date; treat the verdicts as dated findings about dataset
families this lane owns, not as a current plan. The current multi-state database evidence is
`SOURCE_REPORT.md` in this lane.

| Dataset / host | License | Measured size | Verdict (2026-09-20) |
| --- | --- | --- | --- |
| Kaggle `ferdinanddelgadophd/nys-section-v-100m-high-school-track-20082025` | CC0 | zip 905,779 B → CSV 8,050,287 B, **82,889 data rows verified by download** | Usable: 100 m outdoor only, anonymized (no names/schools/joinable ids); columns include `athlete_id, team_id, graduation_year, sex, meet_*, season, career_*, time_seconds, timing_type, timing_corrected` — a timing/grade normalization corpus, not an identity source |
| Kaggle `jeannicolasduval/world-athletics-all-time-rankings` | CC0 | ~489k entries, description 98.9 MB | Elite-only ranking snapshot; no HS coverage |
| Kaggle `laurenainsleyhaines/paris-2024-…` | CC0 | 1.67 MB | Olympic track & field results only |
| Kaggle `heesoo37/120-years-of-olympic-history-…` | CC0 | 41.5 MB | Medal table only, frozen at 2016 |
| GitHub `JPcheco/stonehill-tfrrs-scraper` published CSV | NO-LICENSE | 6,135,057 B `performances_normalized.csv` | Schema reference only; unlicensed, and the source named by this project's no-scraper direction means the code path is out of scope |
| HuggingFace dataset search | — | 5 saved searches | No track & field / TFRRS / XC corpus exists |
| Zenodo / Figshare / archive.org / data.gov CKAN | — | APIs reachable | Zenodo's 4.4M hits are papers, not data; Figshare returns no TF/XC sports datasets; `catalog.data.gov` CKAN API 404s; archive.org holds meet-day uploads only |
| `stats.ncaa.org` | — | — | `robots.txt` `Disallow: /`; college-only even where reachable |
| `opentrack.run` | — | — | robots response carried `cf-mitigated: challenge` (browser required); this lane's `SOURCE_REPORT.md` records it as REJECT |
| OpenSplitTime `api/v1/events` | — | 401 "You need to sign in" | Self-hostable OSS, no HS corpus; API is authenticated |

Adversarial checks recorded in the same session:

* The NYS dataset description claims **82,898** results while the shipped CSV holds **82,889** rows; the
  description is also version 3 while the Kaggle API reports version 5 (as of 2026-04-19). Verified on
  the downloaded file, not on the description text.
* Kaggle's `/api/v1/datasets/view` returns `files: []` for an unauthenticated client, so every non-NYS
  column list above comes from the uploader description and was not parsed from the data.
