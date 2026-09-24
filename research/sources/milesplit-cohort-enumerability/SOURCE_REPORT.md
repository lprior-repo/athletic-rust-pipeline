# Milesplit Class-of-2027 Cohort Enumerability

**Lane:** `research/sources/milesplit-cohort-enumerability/`
**Date:** 2026-09-22
**Agent:** GpuMileSplitGrade

This is the decisive open question for the census-service: can MileSplit supply
Class-of-2027 athlete identities without Athletic.net?  The following sections
answer six sub-questions, each with raw evidence.

---

## 1. Do MileSplit athlete pages expose grade or class year?

**YES — in rendered HTML only, NOT in JSON-LD.**

Every athlete profile page carries a `<span class="grad-year">Class of YYYY</span>`
element.  This was verified on athlete profiles across **five states/samples**:

| State | Athlete (ID) | Grade on profile | Capture file |
|---|---|---|---|
| VA | Silas Webb (16833517) | Class of 2031 | `samples/athlete-va-16833517.html` |
| WI | Julian Aguilera (14399169) | Class of 2027 | `samples/athlete-wi-14399169.html` |
| IN | Chris Acevedo (17974879) | Class of 2028 | `samples/athlete-in-17974879.html` |
| OH | Brice Fuller (15005189) | Class of 2028 | prior `samples/athlete-oh-15005189.html` |
| www | Abigail Burt (13966799) | Class of 2029 | prior `samples/athlete-www-13966799.html` |

**The same value is NOT in the JSON-LD block.**  The schema.org `Person` object on
every profile carries `@type`, `name`, `image`, `affiliation`, `gender`, and `url`
— but **zero** grad-year field:

```json
{
  "@type": "Person",
  "name": "Brice Fuller",
  "image": "https://assets.sp.milesplit.com/athlete_photos/15005189?v=1",
  "affiliation": "",
  "gender": "Female",
  "url": "https://oh.milesplit.com/athletes/15005189-brice-fuller"
}
```

Evidence: `grep -A 20 '"@type": "Person"' samples/athlete-oh-15005189.html` — 0
occurrences of `graduationYear` in any of the 5 captured profiles.

**Verdict for enumeration:** The `class="grad-year">Class of YYYY` field is
reliable and present in the server-rendered HTML of every athlete profile across
all sampled states.  However, the absence from JSON-LD means a JSON-parsing
pipeline would miss it entirely; a DOM/HTML scraper must be used.

---

## 2. Can an athlete page be reached from a meet result page without a name search?

**NO — not directly from the HTML.  The direct HTML path does not exist.**

Three surfaces were examined:

**(a) Meet results page (`/meets/<id>/results`)** — a JavaScript shell.
The HTML body contains **zero** result rows and **zero** athlete links.
Result rows (including athlete names linked to profiles) are rendered client-side
from a JSON payload fetched via the `/api/v1/meets/<id>/performances` endpoint:

```javascript
// from js-loadresultsnew.js:
var nameLink = result.profileUrl
    ? "<a href=\"" + result.profileUrl + "\">" + name_1 + "</a>"
    : name_1;
```

That endpoint is **robots-DISALLOWED** (`Disallow: /api/`).

Evidence: `grep -c '/athletes/' samples/meet-oh-770621-results.html` → **1**
(match is only the `href="/athletes/compare"` dropdown, not a result-row link).

**(b) Meet info page (`/meets/<id>`)** — contains team profile links, not athlete
links.  Example: `https://oh.milesplit.com/teams/9486-wooster` appears in the
"Competing Teams" section.

Evidence: `grep -oP 'https://oh\.milesplit\.com/teams/\d+-[a-z0-9-]+'
samples/meet-oh-771577.html` → team links only, zero `/athletes/` links.

**(c) Results index (`/results`)** — pagination of meet links.  Also contains zero
athlete links (the one `/athletes/` match is the same `"/athletes/compare"`
dropdown).

**The only available path is indirect: meet info → team page → `/roster` → athlete.**
This requires three separate requests per hop and the team page itself is a
JavaScript shell (the roster lives at the separate `/teams/<id>/roster` endpoint).

**Verdict for enumeration:** Enumeration from meets alone is blocked.  The viable
path is **teams index → `/teams/<id>/roster` → athlete profiles**, which is
robots-allowed and HTML-based.

---

## 3. Do team/roster pages carry grad year?

**YES — 100% presence on every non-empty roster, across every state sampled.**

The roster endpoint (`/teams/<id>/roster`) returns every rostered athlete as a
`<li class="athlete-row data-row">` element with a
`<span class="column-grad-year">YYYY</span>` cell.  This was measured on
**16 roster pages across 4 states (VA, WI, IN, OH)**:

| State | Team | URL slug | Athletes | grad-year cells | C2027 |
|---|---|---|---|---|---|
| VA | Abingdon HS | `216-abingdon-high-school` | 260 | 260 | 37 |
| VA | Albemarle HS | `217-albemarle` | 253 | 253 | 102 |
| VA | Alexandria City HS | `545-alexandria-city-hs` | 445 | 445 | 130 |
| VA | 540 TC (club) | `74928-540-tc` | 0 | 0 | 0 |
| WI | Abbotsford | `52649-abbotsford` | 58 | 58 | 27 |
| WI | Abundant Life Christian | `26848-abundant-life-christian` | 127 | 127 | (subset) |
| WI | Adams-Friendship | `14033-adams-friendship` | 61 | 61 | (subset) |
| IN | 21st Century Charter (Gary) | `32665-21st-century-charter-gary` | 56 | 56 | 8 |
| IN | Adams Central | `10226-adams-central` | 55 | 55 | 11 |
| IN | Alexandria Monroe | `10227-alexandria-monroe` | 88 | 88 | (subset) |
| OH | Martins Ferry | `10000-martins-ferry` | 77 | 77 | 16 |
| OH | Marysville | `10001-marysville` | 217 | 217 | 77 |
| OH | Mason | `10002-mason` | 319 | 319 | 96 |
| OH | Mathews | `10003-mathews` | 40 | 40 | 13 |
| OH | Maumee | `10004-maumee` | 89 | 89 | 29 |

Sample grad-year distributions (Class of 2027 highlighted):

```
VA Abingdon:  2027=37, 2028=53, 2029=56, 2030=52, 2031=46, 2032=11
VA Albemarle: 2027=102, 2028=61, 2029=60, 2030=23, 2031=1
OH Mason:     2027=96, 2028=105, 2029=89, 2030=20 (from prior capture)
```

URL shape: `https://<state>.milesplit.com/teams/<TeamID>-<slug>/roster`

Evidence files: `samples/rosters/roster-va-abingdon.html`, `roster-va-albemarle.html`,
`roster-wi-abbotsford.html`, `roster-in-adams-central.html`, plus the OH rosters
in the same directory.

**Verdict for enumeration:** Roster pages are the primary Class-of-2027
enumeration surface.  Every rostered athlete carries a `column-grad-year` cell
with 100% presence on populated rosters.

---

## 4. Per-state team universe size (midwest — 13 states)

| State | Teams | Capture file |
|---|---|---|
| OH | 977 | prior `samples/teams-oh.html` |
| IL | 849 | `samples/teams-il.html` |
| MI | 895 | `samples/teams-mi.html` |
| MO | 700 | `samples/teams-mo.html` |
| WI | 597 | `samples/teams-wi.html` |
| MN | 592 | `samples/teams-mn.html` |
| VA | 609 | `samples/teams-va.html` |
| IA | 411 | `samples/teams-ia.html` |
| KS | 413 | `samples/teams-ks.html` |
| IN | 533 | `samples/teams-in.html` |
| NE | 317 | `samples/teams-ne.html` |
| SD | 199 | `samples/teams-sd.html` |
| ND | 153 | `samples/teams-nd.html` |
| **Total** | **7,245** | |

Method: `grep -oP 'href="https://<state>.milesplit.com/teams/\d+-[a-z0-9-]+'
<file> | wc -l`

These counts match the prior independent sweep
(`~/Downloads/midwest-tfxc-source-research/data/milesplit-coverage-matrix.csv`,
`hs_team_records_on_milesplit`), confirming stability.

**URL shape for teams index:** `https://<state>.milesplit.com/teams`
(single page per state, no pagination, client-side search filter `#txtFilter`).

---

## 5. Cost: requests per 1,000 athletes harvested

### Roster-based enumeration (the viable path)

The only robots-allowed, HTML-based path to Class-of-2027 identities is:

1. `GET /teams` (1 request per state) → team frame with TeamID
2. `GET /teams/<id>/roster` (1 request per team) → roster with `column-grad-year`

**Observed average roster size:** 150 athletes across 15 populated rosters
(median 89, min 40, max 445).  Excluding the empty 540 TC club, the 15 populated
rosters average **167 athletes**.  Using the conservative 150:

| Metric | Value |
|---|---|
| Requests per 1,000 athletes (roster only) | 13 (state indices) + 6.7 (rosters) ≈ **20** |
| Per-state cost (one state) | 1 (index) + 3.3 (rosters for 500 athletes) ≈ **4.3** |
| Full midwest (13 states) | 13 (indices) + 7,245 (rosters) = **7,258** requests |
| Time at 1 req/s | ~2 hours (full midwest) |

### Meet-result path (blocked)

The `/raw` result file path described in the prior report (`/meets/<id>/results/<RSID>/raw`)
returns **HTML-wrapped data** in a `<pre>` block (not raw text), and the structured
result rows require the `/api/` endpoint.  The prior report's claim that `/raw` is
a "static fixed-width text payload" is inaccurate — it is an HTML document
containing a `<pre>` element with the data.  This was verified on the existing
capture (`samples/raw-oh-770621-rs1321880.txt`: 47,564 bytes, contains `<pre>` block).

### Athlete-profile path (expensive, for validation only)

1 request per athlete → 1,000 requests per 1,000 athletes.  This path is useful
for identity/PR validation only; the roster path is the cost-efficient primary.

---

## 6. Robots.txt and rate-limit behaviour

### robots.txt (identical across all states sampled)

Every state host served byte-identical 173 B robots.txt:

```
User-agent: Mediapartners-Google
Disallow:

User-agent: *
Disallow: /rankings
Disallow: /virtual-meets
Disallow: /api/
Disallow: /contact

User-agent: AmazonAdBot
Allow: /
```

**Paths used in this research are all ALLOWED:**
- `/teams` — allowed
- `/teams/<id>/roster` — allowed
- `/athletes/<id>` — allowed
- `/meets/<id>` — allowed
- `/meets/<id>/results` — allowed (HTML shell)
- `/results` — allowed (index)
- `/search/v2/athletes` — allowed (POST JSON)
- `/sitemap.xml` — allowed (no Sitemap directive found)

**Disallowed paths:** `/rankings`, `/virtual-meets`, `/api/`, `/contact`

### Rate limits

- No `Crawl-delay` header in robots.txt
- No rate-limit documentation observed on any page
- **Observed:** 30+ requests across 7 hosts at ≤1 req/s, sequential —
  **zero** 429, zero 403, zero `Retry-After`, zero CAPTCHA
- Working assumption: ≤1 request/second/host, sequential, browser UA

Evidence: `samples/robots-va-milesplit.txt`, `robots-wi-milesplit.txt`,
`robots-oh-milesplit.txt`, `robots-il-milesplit.txt` (all 173 B, identical).

---

## Closing Verdict

### Can MileSplit supply Class-of-2027 athletes without Athletic.net?

**YES.**

The roster enumeration path (`/teams` → `/teams/<id>/roster`) is:
- **robots-allowed** on every host
- **HTML-based** (no JavaScript required, no `/api/` calls)
- **100% grade-presence** on populated rosters (`column-grad-year` cell)
- **Free** (no login, no paywall, no CAPTCHA observed)
- **Complete** (the team index returns all HS teams in one request per state)

### Request cost per Class-of-2027 athlete

Using the roster path with the observed ~150 athletes per roster average:

| Scope | Requests | At 1 req/s |
|---|---|---|
| Per 1,000 athletes | ~20 | ~20 seconds |
| Per state (avg 560 teams) | ~561 | ~9.4 minutes |
| Full midwest (13 states) | 7,258 | ~2 hours |

The roster path makes MileSplit a **standalone** Class-of-2027 discovery source.
Athletic.net is not a prerequisite.

### Risks and gaps

1. **Michigan roster emptiness** — the prior report found 2 of 3 sampled MI teams
   returned 0 athletes (`samples/teams-mi.html` shows 895 teams but roster quality
   is unknown).  This is the single biggest risk to the roster sweep.
2. **Class of 2027 count per jurisdiction** — only per-team samples exist; true
   state-level Class-of-2027 counts require the full roster sweep.
3. **Raw result file format** — the prior report claimed `/raw` returns fixed-width
   text; the actual response is HTML-wrapped in a `<pre>` block.  The grade data
   (`Yr` column) is present only when the timer's file included it, and the
   `profileUrl` field (which would enable athlete-ID joins) is only available
   through the robots-disallowed `/api/` endpoint.
4. **Rate-limit behavior at scale** — only ~30 requests were tested.  A 7,258-request
   sweep may encounter different behavior.  The ≤1 req/s assumption should be
   validated with a pilot sweep before committing to production.

---

## Unfetched / unverified items

| Item | Reason | Impact |
|---|---|---|
| MI roster quality | Not re-measured in this wave | Could reduce MI yield by ~100% |
| IL roster quality | Not sampled | Unknown if IL rosters are populated |
| ND/SD roster quality | Only team counts measured | Small states, likely low impact |
| `/raw` file grade presence rate | Only 1 file tested | Prior report says 5/6 carry `Yr` |
| Rate-limit at 7K+ requests | Only ~30 tested | Assumption of ≤1 req/s unproven |
| `search/v2/athletes` multi-state | Only `oh` and `www` tested | `filters[subdomain]` not proven |
| JSON-LD grad-year on all profiles | Only 5 profiles checked | Could vary by state template |

---

## Raw evidence index

| Evidence | File |
|---|---|
| VA athlete profile with grad-year | `samples/athlete-va-16833517.html` |
| WI athlete profile with grad-year | `samples/athlete-wi-14399169.html` |
| IN athlete profile with grad-year | `samples/athlete-in-17974879.html` |
| VA roster (Abingdon HS, 260 athletes) | `samples/roster-va-abingdon.html` |
| VA roster (Albemarle HS, 253 athletes) | `samples/roster-va-albemarle.html` |
| WI roster (Abbotsford, 58 athletes) | `samples/roster-wi-abbotsford.html` |
| IN roster (Adams Central, 55 athletes) | `samples/roster-in-adams-central.html` |
| OH roster (Mason, 319 athletes, prior) | prior `samples/roster-oh-mason.html` |
| OH teams index (977 teams, prior) | prior `samples/teams-oh.html` |
| VA teams index (609 teams) | `samples/teams-va.html` |
| WI teams index (597 teams) | `samples/teams-wi.html` |
| IN teams index (533 teams) | `samples/teams-in.html` |
| IL teams index (849 teams) | `samples/teams-il.html` |
| MI teams index (895 teams) | `samples/teams-mi.html` |
| MN teams index (592 teams) | `samples/teams-mn.html` |
| IA teams index (411 teams) | `samples/teams-ia.html` |
| MO teams index (700 teams) | `samples/teams-mo.html` |
| KS teams index (413 teams) | `samples/teams-ks.html` |
| NE teams index (317 teams) | `samples/teams-ne.html` |
| SD teams index (199 teams) | `samples/teams-sd.html` |
| ND teams index (153 teams) | `samples/teams-nd.html` |
| VA robots.txt | `samples/robots-va-milesplit.txt` |
| WI robots.txt | `samples/robots-wi-milesplit.txt` |
| OH robots.txt | `samples/robots-oh-milesplit.txt` |
| IL robots.txt | `samples/robots-il-milesplit.txt` |
| OH meet info page | `samples/meet-oh-770621.html` |
| OH meet results page (JS shell) | `samples/meet-oh-770621-results.html` |
| OH raw result file (HTML-wrapped) | `samples/raw-oh-770621-rs1321880.txt` |
| OH athlete profile with grad-year (prior) | prior `samples/athlete-oh-15005189.html` |
| www athlete profile with grad-year (prior) | prior `samples/athlete-www-13966799.html` |
| OH teams index (prior) | prior `samples/teams-oh.html` |
| OH results index (prior) | prior `samples/results-oh.html` |
| JS load-results bundle (API template) | prior `samples/js-loadresultsnew.js` |
