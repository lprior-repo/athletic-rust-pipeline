# Profile replication plan — Class of 2027 (current seniors / juniors last year)

**Date:** 2026-09-20. **Companion to:** [`SOURCES_SURVEY.md`](SOURCES_SURVEY.md) (source inventory + the ≤2 RPS operating policy). This file answers four questions: what exactly we are targeting, whether the athletic.net profile page can be replicated, how free MileSplit profiles would change the design, and how coach/GPA/AI-verification fit in.

---

## 1. Cohort definition (owner clarification)

**Target: athletes who were juniors in the 2025-26 season — i.e. current seniors, Class of 2027.** "Current seniors *or* juniors last year" resolves to the same cohort; the selector is parameterised (`gradYear == 2027`, or evidenced grade 11 during 2025-26) so any future cohort is one config change.

**Evidence rule (from `SCOPE.md`):** junior status comes from season-bound source evidence — never inferred from age or assumed graduation year.

| Evidence | Source | Status |
|---|---|---|
| `gradYear` per performance row | MileSplit performance API | **verified — 420/420 rows** |
| `classYear` / `graduatingClass` | MaxPreps career page (`careerData`) | verified present |
| Grade column at the meet (`SR/JR/SO/FR`) | MileSplit `/raw`, HY-TEK hosts | verified (MileSplit XC raw) |
| `YEAR` = graduation year | TFRRS HS results | verified |
| Grade per row | CIF-SS PDFs (`10`, `12`, …) | verified |

**Cohort seed:** MaxPreps season sitemaps (`Players-{gender}-Varsity-{Sport}-{Season}-2026-2027-{n}-Sitemap.xml`; one boys-XC file = 12,929 athlete URLs) filtered by `classYear`; meet-first collection completes the national list for athletes MaxPreps does not list.

## 2. Can we replicate the athletic.net profile page? — field-by-field

| athletic.net profile element | Replicable? | Source(s) | Notes |
|---|---|---|---|
| Athlete name | **Yes** | MileSplit, TFRRS, MaxPreps | exact strings, no normalisation loss |
| School (and school history) | **Yes** | MileSplit (`current-school`), TFRRS team pages, MaxPreps school | school changes across seasons are visible in per-meet rows |
| Class year / graduation year | **Yes** | MileSplit `gradYear`, MaxPreps `classYear`, HY-TEK grade column | the SCOPE evidence requirement |
| Event list + PRs (current) | **Yes** | MileSplit PR table (event/mark/national rank/date) + deterministic recompute from stored rows | PR also recomputed from raw rows and cross-checked |
| Every performance: event, mark, wind, round, place, heat, meet, date | **Yes** | MileSplit API (all 16 fields, verified), TFRRS athlete page, MaxPreps stat tables, HY-TEK hosts | the core of the page |
| Season-by-season history | **Yes, self-built** | our own collected rows (meet-first corpus) + MaxPreps season tabs | we own the history even where a site paywalls it |
| Progressions / charts | Deferred | same rows | owner: "I will go after progress later" |
| Rankings context | Partial | MileSplit `/rankings?grade=junior` (endpoint TBD), TFRRS lists | phase 6 |
| Coach | **Partial** | school athletics pages, MaxPreps coach fields, state directories | see §5 |
| GPA | **Partial / often absent** | recruiting profiles, academic honour lists, school pages | see §6 — no universal source exists |
| Photos, bio, social links, video | No | — | not a data field we can source; out of scope |

**Answer: yes for the athletic core** — identity, school, grade, every event, every performance with wind/round/place, and PRs all replicate from MileSplit + TFRRS + MaxPreps + the HY-TEK hosts. **Coach is partial. GPA is only available where it is published. Media/biography does not replicate.**

## 3. If free (PRO) profiles were available — what changes

1. **The athlete profile becomes the primary endpoint.** The masked rows are the same DOM with values omitted; with an unlocked session the same selectors fill in [INFERENCE — the mask pattern is verified, the filled state is not]. One request per athlete per season yields full per-performance history + PRs.
2. **Collection flips from meet-seeded to athlete-seeded.** Meet-first collection (≈3 requests/meet) becomes *cross-validation* instead of the primary path; the athlete id list becomes the work queue.
3. **Cost:** ~400k juniors ≈ 400k requests → ~2.3 days serialised on one host at 2 RPS, **hours when spread across the 58 state hosts** (each capped at 2 RPS, ≤8 hosts concurrently) [INFERENCE on cohort size].
4. **Identity gets simpler:** `athleteId` + `profileUrl` + school + gradYear from one document; the probabilistic name+school joins are only needed for regional HY-TEK hosts.
5. **New engineering:** authenticated fetch (cookie/token replay into the collector), one session reused (never parallel logins), and **two parser paths** — masked and unmasked — chosen by detection, with the masked path still recording row presence (count, event, season) as evidence.
6. **What does not change:** the 2 RPS ceiling, the raw-bytes cache, per-field provenance, and the deterministic-gate requirement for AI output.

## 4. Cloudflare — what actually blocks us, and the contingency ladder

**Survey fact:** across ~170 requests to 25+ hosts, **no target host presented a challenge.** The only challenges observed were `opentrack.run` (`cf-mitigated: challenge` on robots.txt) and `www.thepowerof10.info` (`wafrule: 5`). The hosts we actually need — MileSplit, TFRRS, MaxPreps, DirectAthletics, the HY-TEK hosts — answer plain requests.

So CF handling is a **contingency**, not a dependency. The ladder, in order:

1. **Prefer non-CF equivalents** where a host misbehaves — baumspage, yentiming, elitefeats, runnercard, finishtimingresults are all plain nginx/IIS/Apache.
2. **Use the existing headed-Chromium transport.** A real browser with a real profile passes managed challenges the way a human does — that is the same transport the pipeline already runs for the current target.
3. **Stay boring.** 2 RPS, single in-flight request, ETag/`If-None-Match` reuse, session cookies reused. Aggressive crawl shapes are what *trigger* new challenges; slow pacing is the cheapest evasion.
4. **Escalating host ⇒ drop it.** If a host starts issuing persistent challenges, mark `challenge_persistent` in `HostPolicy` and stop. The engineering cost of fighting a challenge-storm exceeds the value of any single regional host.
5. **Not doing:** third-party CAPTCHA-solving services, TLS/fingerprint-spoofing tooling, proxy rotation. A real browser plus slow pacing covers everything observed in this survey; anything stronger changes the risk profile for no data gain.

## 5. Coach contact

**What exists (verified in page source):**
- MileSplit team pages carry contact fields — sampled page exposes an `"Email":""` property in its embedded JSON (empty for that team) plus social handles.
- MaxPreps school pages expose a coach subsystem (`coach.aspx`, `coachAssociationId/Name/Abbreviation`, `coachNote`, "Coach Tools") — names/emails are not on the school landing page.
- Coach **names** are widely published: school/district athletics pages, MaxPreps staff lists, state association directories, and meet entry pages (where the coach is the listed contact).
- Coach **emails** are rarer: often obfuscated (`coach [at] school.org`), form-gated, or absent.

**Plan:** coach enrichment is a **second pass keyed by school/team**, not by athlete — school → district athletics site/staff directory → role + name + email. Store provenance per field; local model extracts names/roles from unstructured pages; deterministic regex validates email/phone shapes and rejects guesses. Coverage target: name+role for most schools, direct email for a minority, with per-school coverage reported rather than silently assumed.

## 6. GPA

**Honest position: there is no universal public source.** GPA is published in exactly three places:

1. **Recruiting profiles** (NCSA, FieldLevel, SportsRecruits, Hudl) — the most consistent text source; self-reported by the athlete.
2. **Academic honour lists** — state-association "academic all-state" teams, district honour rolls — usually as lists or PDFs, often as a GPA band rather than a number.
3. **School/team pages** — sporadic.

**Plan:** GPA is a **nullable field with a source enum** (`recruiting_profile | academic_list | school_page`) and is never inferred, never derived from test scores, never defaulted. Extraction is deterministic where the page is structured (recruiting profile selectors; `pdftotext` + row parse for honour lists); the local model only adjudicates ambiguous rows, and every GPA value stores its source URL.

## 7. Local AI verification lane

**Principle: AI proposes, deterministic gates decide.** No model output enters the record without a validator that can reject it.

| Priority | Job | AI role | Deterministic gate |
|---|---|---|---|
| 1 | Identity resolution across sources (same name, different schools; `Jr.`/nicknames) | propose candidate matches | school + grad year + team + event-set overlap must agree |
| 2 | Mark sanity | review the rejects | per-event range tables (e.g. 100m 9.5–30 s, 5K XC 13–45 min), unit/format checks |
| 3 | Cross-source conflicts (MileSplit vs MaxPreps vs TFRRS) | adjudicate the conflict | meet+athlete+event key must match exactly; wind sign and round labels reconciled by rule first |
| 4 | Unstructured clean-up (CIF-SS PDFs, HY-TEK oddities, coach/GPA pages) | extract from messy text | regex/shape validation, schema conformance |
| 5 | PR computation | — | **deterministic only**: group by event, pick the valid minimum with timing/wind flags; the model never computes a PR |

Runs on the existing local lane (3090/5090 Qwen endpoints), batched, content-hash cached, with every proposal and verdict written to the audit trail.

## 8. Determinism rules (current PRs now, progressions later)

- Retain raw bytes + content hash for every fetched document.
- One explicit parser per source format (MileSplit JSON, TFRRS tables, HY-TEK fixed-width, ASPX tables, PDF text) — never fuzzy-parse a mark when an exact format exists.
- Per-field provenance: source URL, selector/format, fetch timestamp, parser version.
- Preserve conflicts and unknowns explicitly (the repo's existing rule) rather than overwriting.
- PRs are computed from stored rows; the source's own PR table is stored as a cross-check, and disagreements are surfaced, not silently resolved.
- Phase 1 delivers **current-season PRs + the current-cohort roster**; season/career progressions are phase 2 and the schema keeps every row needed for them.

## 9. Feasibility at 2 RPS

- **Fan-out:** 58 MileSplit state hosts, paced independently at ≤2 RPS each, ≤8 hosts concurrently — the national run is host-parallel, never host-fast.
- **Meet-first:** ≈3 requests/meet × ~20k–40k meets/season ≈ 60k–120k requests → under an hour per state host, hours overall [INFERENCE on meet counts].
- **Athlete-first (with profiles):** ≈400k juniors ≈ 400k requests → hours across hosts; ~2.3 days if serialised on one host [INFERENCE on cohort size].
- **Storage:** 420 rows/meet × 20k–40k meets ≈ 8–17M performance rows/season; tens of GB once raw documents are retained.

## 10. Phased plan

| Phase | Deliverable | Depends on |
|---|---|---|
| P0 | Shared 2 RPS scheduler + `HostPolicy` (token bucket, ETag cache, backoff, cooldown, budget dry-run) | — |
| P1 | Cohort seed: MaxPreps sitemaps → `classYear` filter → candidate junior list | P0 |
| P2 | HS results + grade evidence: MileSplit meet hub → meet pages → performance API | P0, P1 |
| P3 | History/cross-check: TFRRS athlete pages (1 request/career) | P0 |
| P4 | Enrichment: coach pass (school-keyed), GPA pass (recruiting/honour lists) | P2 |
| P5 | Verification lane: AI proposals + deterministic gates + conflict ledger | P2, P3 |
| P6 | Progressions, rankings, live layer (owner's later call) | P2, P3 |

**Immediate next build:** P0 + a MileSplit performance-API adapter, proving one state-season end-to-end with the budget dry-run printed before any fetch.
