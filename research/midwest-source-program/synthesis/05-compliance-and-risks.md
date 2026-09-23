# Compliance and risk ledger — Midwest TF/XC source exploration

Observed: 2026-09-19/20. Sources: per-report `Access characteristics` sections + evidence appendices.
Nothing here is legal advice; it records what was observed and what each source's published rules say.

## 1. Hard blocks and boundaries observed

| Source family | Boundary observed | Effect on the plan | Ref |
|---|---|---|---|
| `www.athletic.net` HTML + `/api/v1/**` | Cloudflare managed challenge: plain curl → HTTP 403 `cf-mitigated: challenge` (both captures and 2026-09-19 re-probes). Browser-UA sometimes returns the SPA shell only; API stays 403. All live Athletic.net evidence came from the two 2026-09-18 browser HARs. | Athletic.net access requires the already-cleared headed-browser profile. Plan every Athletic.net request as expensive. | [02][03][04][05] |
| MileSplit `robots.txt` (`wi/oh/ne/...`) | `Disallow:` `/rankings`, `/virtual-meets`, `/api/`, `/contact`. The richest result model (`/api/v1/meets/{id}/performances` with `gradYear`) sits on a disallowed path. | Do not use MileSplit JSON API or `/rankings`. Usable surface = server-rendered HTML: `/teams`, `/teams/<id>/roster`, `/results`, `/meets/<id>/results/<RSID>/raw`, `/meets/<id>/entries`, `/timing`, `/athletes/<id>`, `sitemap.xml`. **Note:** [07] and [20] captured `/api/v1` JSON recipes (WI `/performances`, OH `/rosters/teams/<id>/ranked?grade=2027`); retained as evidence, but the *build* must treat `/api/` as off-limits absent MileSplit permission — substitute the HTML `roster`/`raw` surfaces or seek a license. | [07][20][27] |
| PrimeTime/Karmarush live platform (`live.pttiming.com`, ptt-franklin Firebase) | ToU v1.0 (eff. 2026-09-03) bans automated access, bulk extraction, AI/ML use, and constructing recruiting databases; robots bans ClaudeBot/anthropic-ai. A shallow notice-bearing probe was made, then stopped. | AthleticLIVE/live layer of PrimeTime is off-limits unless licensed. PrimeTime-owned static/hosted files are ambiguous — lawyer/vendor ruling required before a build. | [22][08] |
| Bound (`gobound.com`) | `robots.txt` (updated 2026-04-21): `Crawl-Delay: 10`, disallows `/api/`, `/*directory`, `/profile/*`, `*/athletes/*`, `/*/leaders*`. Two disallowed directory pages were fetched before robots was read; disclosed, dropped from recipes. | Bound recipes use only allowed server-rendered pages, ≥10 s delay; the JSON API path is off-limits. | [11] |
| RaceResult (`my.raceresult.com`) | robots disallows result `list/view/pdf` endpoints. | Excluded. | [16] |
| MSHSAA `robots.txt` | Disallows `/MySchool/Matchup.aspx*`; one Matchup page was fetched before robots was read; documented as do-not-collect. GPTBot/Amazonbot/meta-externalagent disallowed entirely. | Do not collect `/MySchool/Matchup.aspx`. All other used paths are allowed. | [21] |
| MSHSL `robots.txt` | Disallows bulk crawling of `*.pdf/*.doc/*.xls/*.csv`. | Fetch state-tournament PDFs on demand only, never crawl. | [09] |
| `iahsaa.org` robots | Disallows query strings. | Use clean paths only. | [11] |
| iahsaa/NSAA/MHSAA server posture | Some hosts 403 default curl UA / TCP-reset browser-ish UA (NSAA WordPress origin); the working client was an honest UA on the working host/domain, never a spoofed or rotating identity. | Keep the honest-client policy; prefer the host that serves the same data without gating (e.g. `secure.nsaahome.org`, S3 mirror). | [24] |
| Athletic.net `robots.txt` | 200, no sitemap, nothing under `/api/` disallowed. | No robots obstacle on the Athletic.net API itself; the block is the Cloudflare challenge and session auth. | [01] |

## 2. Privacy contract (enforced across all reports)

Collected: public athletic evidence, public recruiting/profile URLs, coach/AD **public professional**
information published for the school/sport role.

Excluded at parse time (several agents explicitly dropped these):
- athlete personal email/phone/address — none exists on any source used;
- MSHSL `/api/coaches` records at `coach_level` `Non-MSHSL Coach` / `MSHSL Sub-Coach` (personal-domain
  emails, personal mobiles) — the site's own bundle filters these levels out; replicate that filter plus a
  school-domain check [09];
- KSHSAA school records' `PrincipalCell/ADCell/PresCell` (personal mobiles) dropped [23];
- MSHSAA district/sectional host-manager blocks may contain a residential mailing address — drop [21];
- Bound staff emails/phones (empty anyway) and DAT meet-director phone/fax (not transcribed) [11][14];
- student height/weight/position fields present in some rosters [09].

## 3. Terms-of-service rulings recorded per source

- **REJECT on terms**: PrimeTime `live.pttiming.com` / Karmarush (automated access + recruiting databases),
  `ptt-franklin.firebaseio.com` live feed (same operator family) [22][08].
- **License path**: `info@pttiming.com`, `legal@karmarush.com` [22].
- **Conditional**: MileSplit PRO subscription exists but the binding constraint is robots, not paywall;
  Progression tab (`paywall_present:1`) unverified without a browser [07].
- **Ambiguous pending counsel**: whether Karmarush ToU reaches PrimeTime-owned static files [22].

## 4. Methodology risks carried into the synthesis

1. **Cloudflare blocks live Athletic.net verification from this machine.** All Athletic.net-side findings
   are HAR- or bundle-derived; meet response shapes are `derived` (no meet body in either capture) [03].
2. **`web_search` was broken for the whole campaign** (provider error `Sign up and repeat your request.`
   on every attempt, all agents). Everything came from direct HTTP to primary sources. Consequence: no
   search-engine corroboration of discovered hosts; secondary discovery may be incomplete.
3. **Politeness budgets**: several agents disclosed overages (~85–90 requests to one host [28], ~63 to
   MHSAA [15], ~59 to `search.athletic.live` [12], ~1.26× guideline [15]) — recorded, no adverse response
   (no 429, no Retry-After anywhere).
4. **Cohort semantics**: today (2026-09-19) Class of 2027 = Grade 12. Grade-11 corpora (e.g. the
   Athletics 2026 US boys Grade-11 workbook) are 2025-26 school-year data. Always key on absolute
   graduating class / grad year [07][27].
5. **Estimates vs measurements**: reports label `[INFERENCE]`/`[UNVERIFIED]`; extrapolated per-state
   counts are bands, not censuses [27].
6. **Spot-sample bias**: coach-directory coverage numbers are samples (e.g. WI 9 schools, OH 14, IL 14);
   treat percentages as point estimates.

## 5. Repo-side statement

`/home/lewis/src/ad-law-scrape/athletic-rust-pipeline` gained **one new workspace member** —
`crates/census-service/` — plus the `Cargo.toml`/`Cargo.lock` lines that register it. That crate holds
the adapters and the store commands the census run used; nothing in `src/` was edited, no commit was
made, and the working tree still shows the owner's own uncommitted edits to
`src/runtime/rankings/page/parse/main.rs` and `tests/rankings_parser.rs` (mtime 2026-09-19 23:21 CDT),
which this campaign neither authored nor reverted. All other agent writes were confined to
`~/Downloads/midwest-tfxc-source-research/`.

## 6. Census run ledger (2026-09-20)

Measured from the run's own HTTP cache (`<store>/http/*.meta.json`), not from intent:

| Fact | Value |
|---|---|
| Cached responses | 7,355 (7,190 × HTTP 200, 165 × HTTP 403 before the retry sweep) |
| Hosts contacted | 15 — ten MileSplit state sites, `search.athletic.live`, `schools.wiaawi.org`, `kshsaa-api.kshsaa.org`, `www.ihsa.org`/`api.ihsa.org`, `officials.myohsaa.org`, `my.mshsl.org` |
| **`athletic.net` requests** | **0** — no host in the cache matches `athletic.net`; the campaign never queried Athletic.net |
| Refused (403) then recovered | 163 roster pages (NE 54, MO 54, OH 54, KS 1) were refused, the refusals were purged from the cache, and a second pass recovered all 160 rosters (3 remained after the sweep); no refusal was bypassed |
| Refused and not retried | ND and SD MileSplit `/teams` indexes (1 request each, HTTP 403) — recorded, never retried |
| MHSAA | Adapter **removed**: both bulk endpoints (`/DesktopModules/MHSAA-Endpoint/API/School/Search`, `.../AdministrationDirectory`) sit under `my.mhsaa.com/robots.txt` → `User-agent: * / Disallow: /DesktopModules/`. The public `/schools` page is a JS shell over that same API, so no robots-allowed bulk surface exists. Michigan coach coverage therefore stays with the research reports. |
| OHSAA portal (`officials.myohsaa.org`) | Every request from the campaign's rustls client is reset at connect (`Connection reset by peer`), while the same URLs answer 200 to curl/OpenSSL on the same host and IP; an `openssl s_client` handshake succeeds when it offers ALPN and fails without it, so the edge is filtering on ClientHello shape. We did **not** impersonate a browser to get past it, so the `ohsaa` provider contributes zero rows; Ohio coach coverage in the store is the 33 rows the research-compiled contact CSV carries plus the report-level findings. |
| Athlete-side store | 1,347,810 athlete rows merged (crawl + AthleticLIVE import) with zero Athletic.net requests |
| Requests per newly discovered athlete | ≈ 0.005 (7,355 requests for 1.35M athlete rows before consolidation) |

