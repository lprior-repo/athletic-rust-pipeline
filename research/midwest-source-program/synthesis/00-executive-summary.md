# Executive summary — Midwest TF/XC source exploration (12 states, Class of 2027)

**Question.** Find every viable way to discover Midwest Class-of-2027 track & field / cross-country
athletes, the associated meets, results and coach contacts *without* multi-day Athletic.net scraping —
and quantify how much Athletic.net work each alternative removes.

**Method.** 30 parallel research agents (29 source assignments + a Pareto consolidator), ~2,000 recorded
HTTP requests against ~120 primary hosts, everything evidence-grade with per-report URL/status/timestamp
appendices. Athletic.net itself was studied from two browser HAR captures (2026-09-18) plus shipped
Angular bundles because live access from this machine is Cloudflare-403. Nothing outside this folder was
modified except one new workspace member in the pipeline repo (`05-compliance-and-risks.md` §5).

**Deliverables.** `research/midwest/01..46-*.md` (30 assigned reports + 16 follow-up probes), `data/*.csv`
(data products incl. a coach-contact graph, a 22,527-row DAT team index, 93,619 Athletic.net athlete
seeds and the canonical census exports), `synthesis/*` (this set, incl. **`10-measured-census.md`**),
`tools/*` (runbooks + analysis scripts + raw capture evidence), `brief/*` (mission + consolidator
contracts).

**Then we built it.** `crates/midwest-census` runs the qualified adapters end to end: 707,933 canonical
athletes, 183,875 of them Class of 2027, 93,619 Athletic.net athlete ids and 9,593 Athletic.net meet ids
obtained **without a single Athletic.net request**. Numbers and gaps: `10-measured-census.md`.

---

## Headline findings

1. **Athletic.net is not the only enumerator — and in five states its IDs are handed out for free.**
   IHSA (IL) returns `athleticNetId` per state-finalist and per team via a plain public API; OHSAA (OH)
   publishes literal Athletic.net URLs (team `22671`, meets `656920`, live `74520`); MHSAA (MI) publishes
   **89 Athletic.net MeetIDs per season** from 2 requests; WIAA tournament result files *are* Athletic.net
   exports; SDHSAA publishes its whole Athletic.net division/team/meet inventory. [13][19][15][06][26]

2. **AthleticLIVE is Athletic.net's own white-label** — and it is the cheapest compliant read path into
   that ecosystem: an unauthenticated Azure blob (`ind_res_list/_doc/<eventId>`) plus Elasticsearch and
   (ND) Firebase RTDB endpoints return **every result row with grade and the Athletic.net athlete id
   (`ani`), no browser, no Cloudflare**. Measured: MSHSL 2025 state XC = 6 requests → 959 rows → 242
   Co2027 (100 % with `ani`); Iowa state T&F = 1,265 grade-11 athletes; ND state series 185 entries in one
   234 KB response. It is *not independent corroboration* — treat it as an unblocked Athletic.net reader.
   [10][12][25]

3. **MileSplit can enumerate Class of 2027 directly, robots-allowed.** One request per team returns the
   full roster with absolute graduating year (`column-grad-year`), gender and season flags; filtering is
   client-side. 6,633 teams across the 12 states (6,645 requests for a full pass). Sampled overlap says
   **34.1 % (CI 28.9–39.8) of sampled outdoor Co2027 boys are absent from the Athletic.net Grade-11
   corpus** — i.e. MileSplit reaches roughly a third beyond Athletic.net rank enumeration. Its JSON API
   (`/api/v1/...`) is robots-disallowed, so production must use the HTML roster/results surfaces. [27][07]

4. **The full 12-state Athletic.net discovery sweep already costs just 2,728 requests** (state-scoped,
   single-event, grade-filtered, page-until-empty) versus 40,695 profile requests — the repo should keep
   Athletic.net for the tracked core and *progress/PR* (`GetAthleteBioData`, 1 request/athlete), move
   result acquisition to whole-meet pulls (3 XHR/meet), and **retain the `MeetID` that every rankings row
   already carries** (zero extra requests; the delivered workbook currently drops it). [03][04][05]

5. **The coach-contact graph is buildable in 10 of 12 states.** Best: OH (myOHSAA: AD + head coaches with
   professional email, 3 requests/school), IL (IHSA staff API), WI (WIAA school DB), KS (1 request → 526
   schools with AD emails). MO and IN are verified dead ends (no public contact source). Athlete-weighted:
   **54.5 % have an identified TF/XC coach; 35.4 % a coach email; 68.1 % an AD email** — with a
   conservative 0-rate wherever only names exist. [29][06][13][19][23]

6. **Grade evidence independent of Athletic.net exists for ~3,993 state-meet rows** (IL 1,196, IA 1,265,
   OH 561, WI 382, MN 242, ND 185, SD 106+, KS 89, MO ~300–350) = ~9.8 % of the 40,695-athlete corpus
   corroborated, all from association/timer artifacts that carry a literal grade column. [13][12][19][07][10][25][26][23][21]

7. **Union estimate after reconciliation: ≈ 62,000 ± 5,000 Co2027 boys in the 12 states**, versus 40,695
   (12-state Athletic.net sweep) today — the delta is the MileSplit-only complement. It is a model, not a
   census: a 100-athlete manual audit is the cheapest way to firm it up (ranked action #3). [30 Q2]

8. **Compliance boundaries are real and were mapped, not crossed.** Michigan/Ohio/Indiana/Wisconsin
   association sources are plain HTML/JSON; MileSplit `/api/` + `/rankings` are robots-disallowed;
   PrimeTime's Karmarush live platform explicitly forbids automated collection *and recruiting databases*
   (license path recorded); Bound requires 10 s crawl-delay; Athletic.net requires the headed session.
   No CAPTCHA/auth barrier was bypassed anywhere. `web_search` was broken all campaign (provider error), so
   everything is direct-from-primary HTTP. See `05-compliance-and-risks.md`.

## What to build (first five, ranked by verified coverage ÷ effort)

Full table in `data/adapter-ranking.csv` + `03-adapter-ranking.md`.

1. **MileSplit team-roster adapter (12 states)** — breadth winner; Co2027 grad-year evidence, robots-allowed.
2. **IHSA API (IL)** — highest payload per request in the study (828 schools / 1 request; AN ids + coach emails).
3. **MSHSL JSON API (MN)** — grade + coach name/email + AN MeetID in one family.
4. **KSHSAA directory API (KS)** — 1 request = 526 schools with AD emails (cheapest coach layer found).
5. **AthleticLIVE bridge (MN/IA/ND + 258 tenants)** — AN athlete ids + grades without a browser.

Then: TFRRS Indiana (1,861 Co2027 in 2 requests) · OHSAA portal (coach emails) · WIAA school DB ·
NSAA directory + S3 · Bound IA/SD + MHSAA admin API + NDHSAA long tail.

## Caveats you must carry into any build decision

- **No free state-level Class-of-2027 census exists outside Athletic.net.** Per-state MileSplit totals are
  3-team-sample bands, not measurements. [27]
- **Grade semantics are route- and season-dependent** (at-meet `JR` on a 2025-26 sheet = Co2027; current
  `SR` on a 2026-27 roster = Co2027; literal "Class of YYYY" elsewhere). Mixing them is an off-by-one-class
  error. Today Class of 2027 = Grade 12. [27][18][30 §Contradictions]
- **The Athletic.net meet channel is still half-qualified**: no meet response body exists in any capture
  (3-XHR whole-meet shape is derived from client code), and no meet-calendar endpoint was identified. Two
  browser captures close this (`06-open-questions.md` A1/A2). [03]
- **AthleticLIVE/HAR-derived identities need one confirmation pass** (`ani` → profile for 3 athletes; one
  XC rankings call, since all five Athletic.net reports are TF-outdoor-scoped). [30 §next actions]
