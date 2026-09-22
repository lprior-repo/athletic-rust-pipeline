# Midwest HS Track/XC Source Exploration — Shared Mission Brief

Version: 2026-09-19 (America/Chicago). Owner: Main. **Read this before starting your assignment.**

## 1. Purpose

Find external, non-Athletic.net sources that can discover, seed, verify, and continuously refresh
Midwest high-school Class-of-2027 (Grade 11) track & field and cross-country athletes, teams, meets,
results, and coach contacts — so the production pipeline stops paying Athletic.net request costs
(and Cloudflare friction) for work another source can do better or first.

Every report answers one question well: **what can this source do for the acquisition flow, and at
what cost?**

## 2. Existing evidence you MUST leverage (read-only)

| Artifact | Path | Use |
|---|---|---|
| Pipeline repo docs | `/home/lewis/src/ad-law-scrape/athletic-rust-pipeline/{README.md,SCOPE.md,HANDOFF.md,CHROMIUM_DESIGN.md}` | Current Athletic.net source contracts, IDs, known limits |
| HAR capture #1 | `/home/lewis/Downloads/www.athletic.net.har` (16.8 MB) | Raw browser XHR evidence, primary for assignments 1–5 |
| HAR capture #2 | `/home/lewis/Downloads/www.athletic.net2.har` (7.9 MB) | Second capture; cross-check |
| Repo captured-response fixtures | `/home/lewis/src/ad-law-scrape/athletic-rust-pipeline/{tests/fixtures,fixtures,src}/**` | Request/response shapes already parsed by production code |

Known facts from repo evidence (verify against HARs; reconcile, don't assume):

- Athletic.net division list ids: `168416` = 2026 outdoor, `173005` = 2026 indoor; indoor `SeasonID = 12026`.
- Endpoints observed in production: `GetNavInfo`, `GetRankings`; meet/result endpoints named in the
  assignments (`GetMeetData`, `GetAllResultsData`, `GetResultsData3`) are to be qualified from the HARs.
- The retained consolidated boys Grade-11 delivery contains 142,705 unique athletes, from 95 events,
  4,256 receipts, 340,238 individual results, 57,629 relay-member results, 15,724 unresolved roster results.
- **Live network observation (2026-09-19):** `https://www.athletic.net/` returns HTTP 403 (Cloudflare
  "Attention Required!") to non-browser clients from this machine. `https://wi.milesplit.com/` returned 200.
- **Do NOT attempt to bypass Cloudflare, log in, replay cookies, spoof fingerprints, or drive the
  repository's Chromium pipeline.** The HARs + docs + public third-party sources are your evidence for
  Athletic.net-side questions. Where live access is needed and blocked, record the block as the finding.

`~/Downloads` also contains prior pipeline outputs (XLSX/CSV/HAR). Treat all of it as read-only input.

## 3. Geography and population

12 states: Wisconsin, Minnesota, Iowa, Illinois, Michigan, Indiana, Ohio, Missouri, Kansas, Nebraska,
North Dakota, South Dakota.

Target: **source-verified Class of 2027 / Grade 11 athletes**, boys and girls, in boys/girls track & field
(indoor where applicable, outdoor) and boys/girls cross-country.

## 4. Hard rules

1. **No code changes anywhere.** The only paths you may write are inside
   `/home/lewis/Downloads/midwest-tfxc-source-research/`:
   - your report: `research/midwest/<NN>-<slug>.md` (yours exclusively),
   - helper/parse scripts + scratch: `tools/`,
   - CSV data outputs (only if your assignment asks): `data/`.
   Never modify anything in the repo or elsewhere. Read-only everywhere else.
2. **No access-control bypass.** No CAPTCHA solving, no auth/paywall circumvention, no cookie
   replay, no fingerprint spoofing, no proxy rotation. If a source is authenticated or
   subscription-restricted, classify it as such and document what is visible without it.
3. **Privacy contract.** Do not collect athlete personal email, personal phone, or home address.
   For coaches/administrators, retain only public professional contact information published for
   their school/sport role. If a directory exposes extra personal data, do not ingest it. Never copy
   cookies/tokens/PII from HAR request headers into reports.
4. **Politeness.** Treat every host as rate-limited: aim for ≤ ~50 requests per host, sequential,
   prefer documented indexes/sitemaps/static downloads over crawling. Honor `Retry-After`. Record
   published limits and any observed 429/`Retry-After` behavior.
5. **Evidence-first.** Every factual claim carries the URL + exact navigation/query used + the date
   observed. Unverified reasoning is marked `[INFERENCE]`; third-party claims `[UNVERIFIED]`.
   Never invent IDs, counts, endpoints, or URLs. A failed fetch is a finding: record status + classify.
6. **Quantify.** Counts, page sizes, percentages where possible. "Enumerate" means "give the exact
   request pattern and pagination", not "the site has pages".

## 5. Report contract

Path: `/home/lewis/Downloads/midwest-tfxc-source-research/research/midwest/<NN>-<slug>.md`

Header (first lines):

```md
# <NN>. <Assignment title>

Status: complete | partial — <exactly what is missing and why>
Observed on: 2026-09-19
```

Then exactly these sections (omit none; write "Not applicable / not observable" where true):

### Source
Provider/site name and URLs.

### Coverage
States, sports, school levels, seasons and historical depth.

### Enumeration
Can we enumerate: schools; teams; meets; athletes; Class of 2027 athletes; results?
Document exact navigation/query structure.

### Stable identifiers
Find and document: athlete IDs; school/team IDs; meet IDs; result IDs; event IDs; season IDs.
Do not assume names are identifiers.

### Athletic.net leverage
Determine whether this source directly exposes: Athletic.net meet links; team links; athlete links;
result links — or whether its school/name/grade/meet context can deterministically seed an
Athletic.net lookup. Estimate how many Athletic.net source requests could be avoided.

### Athlete evidence
Availability of: name; graduating class/grade; school; city/state; gender/category; TF/XC distinction;
indoor/outdoor; performances; PRs; progression; meets; athlete profile URL.

### Recruiting information
Whether the source exposes: head track coach; head XC coach; assistant coaches; athletic director;
public professional email; school athletics website; team website.
Do not collect athlete personal contact information.

### Result evidence
Availability of: ResultID; AthleteID; MeetID; EventID; mark; normalized mark inputs; timing method;
wind; implement/hurdle specification; heat/round; place; date; school represented; relay membership.

### Incremental use
How a weekly collector identifies only: new meets; changed meets; new results; affected athletes.
Avoid designs requiring a full historical re-fetch each week.

### Access characteristics
Classify as: documented API; public structured JSON; static CSV/XML/XLSX; normal HTML; browser
application; downloadable PDF; authenticated; subscription-restricted; unavailable.
Record published limits and observed `Retry-After` behavior where applicable.

### Recommendation
Choose: PRIMARY | ATHLETIC.NET-SEED | RESULT-SOURCE | COACH-DIRECTORY | VALIDATION |
DISCOVERY-ONLY | REJECT. Explain the expected marginal coverage.

Finally:

### Evidence appendix
Table of every URL queried: `URL | method | HTTP status | what it proved | timestamp`.

## 6. Reporting back

Return a short summary (≤ 20 lines) to Main: report path, source(s), recommendation, 3–6 top findings,
Athletic.net leverage, access class, biggest gaps. Full detail lives in the file — do not duplicate it.

## 7. Coordination

- You own your report file exclusively. Never write another agent's report.
- Peer reports land in `research/midwest/` as agents finish; you may read them read-only (especially
  agents 27–30 and the state agents).
- Agent 30 consolidates all 29 reports; Main writes the final synthesis into `synthesis/`.
- The whole point is **condensation into an actionable acquisition plan** — bury nothing important,
  but keep prose dense.
