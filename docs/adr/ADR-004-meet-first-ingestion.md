# ADR-004: Meet-first result ingestion; athlete enrichment is second

## Status

Accepted as policy, implemented where a source allows it: every timer/vendor adapter parses a whole
result file, and the AthleticLIVE athlete index is queried per batch of meets. The national
per-jurisdiction fan-out of `ARCHITECTURE.md` §6 is **planned, not in place** (target `census-crawl`).

## Context

The census needs performances for tens of thousands of athletes. Two shapes produce them: one
whole-meet payload yields hundreds to thousands of rows in a single request, while an athlete-first
walk needs one request per athlete for tens of rows each.

Athlete-first is the shape the previous pipeline drifted into and the largest avoidable source of
requests: 500 profiles for performances one meet payload already carries multiplies traffic by two to
three orders of magnitude. It also produces worse data, because a profile view is a *projection* of
results the timing provider published first and can lag or omit entries. Profile fetching is the most
gated path besides: the Athletic.net adapter cannot discover ids (its search endpoint is disallowed by
robots) and reads them from an operator registry (`athleticnet/mod.rs:6-10,155-165`), at **two** calls
per athlete — `sport=tf` and `sport=xc` are not redundant and neither is optional (`:26-29`).

## Decision

**Acquire whole meets first; enrich athletes second.**

* A discovered meet is a work item whose acquisition yields that meet's complete published result
  payload (rounds, heats, relays, field events).
* Meet identity is minted once per platform identity and merges every tenant that published the same
  meet (`sources/athleticlive/meets.rs:1-2,11-12`); targets are de-duplicated by canonical id before
  batching — explicitly because "duplicate ids would multiply requests"
  (`sources/athleticlive_athletes/targets.rs:41-45`).
* Athlete profiles are fetched only for what meet data cannot supply: PR/progression history beyond
  the discovered meets, source profile URL/seed ids, grade evidence, school history across transfers,
  the XC↔TF linkage, and identity resolution.
* Cost is measured per run, not asserted: adapters report `requests`/`from_cache`
  (`crates/census-service/src/sources/mod.rs:110-116`) and the orchestration report carries
  `requests`, `cache_hits`, `errors` (`crates/census-service/src/census/mod.rs:73-83`).
* A meet payload's result rows are the strongest evidence of a performance; a source-reported PR is
  retained separately and never silently replaces the calculated PR (`ARCHITECTURE.md:86-87`).

## Consequences

* Meet discovery becomes the critical path: quality depends on enumerating sanctioned meets per
  jurisdiction and season. Today's implemented walk is jurisdiction → team index → rosters → entities
  (`census/mod.rs:1-10,25-30`) — roster-first for MileSplit, not the planned meet-first fan-out.
* Bulk shape in code: one Elasticsearch query covers up to 40 meets and 2,000 athlete rows, bounded by
  the platform's 10,000 `from + size` window (`sources/athleticlive_athletes/mod.rs:56-63,78-82`),
  consumed oldest-meet-first so a capped run covers settled meets (`:125-130`).
* Adapters must parse whole-meet documents rather than one athlete at a time; `ParsedMeet` carries
  meet, events, per-row grade/school/mark, relays, and skips counted-but-never-guessed
  (`sources/result_file.rs:1-6,12-61`).
* Enrichment stays request-gated: because the Athletic.net adapter needs a registry and two calls per
  athlete, athlete-first acquisition cannot quietly re-enter through it.

## Alternatives considered

* **Athlete-first acquisition.** Rejected: orders of magnitude more requests for data a meet payload
  already contains, on the most rate-limited hosts.
* **Enrich every profile "to be safe".** Rejected: the profile projects the same results, adding cost
  without adding the primary evidence (`athleticnet/mod.rs:6-10`).
* **Trust a profile's reported PR over meet results.** Rejected: reported and calculated PRs are
  separate evidence with disagreements surfaced, not resolved (`ARCHITECTURE.md:86-88`).

## Evidence — what exists, what is planned

Read directly: `sources/result_file.rs:1-6,12-61`; `sources/athleticlive/meets.rs:1-12`;
`sources/athleticlive_athletes/mod.rs:1-63,125-131` and `.../targets.rs:25-45`;
`sources/athleticnet/mod.rs:6-10,26-29,155-165`; `sources/mod.rs:110-116`;
`census/mod.rs:1-10,25-30,56-83`; `ARCHITECTURE.md:86-88`.

Planned, explicitly not implemented: `ARCHITECTURE.md:94-101` — "Target workspace split (not yet in
place)"; per-jurisdiction, meet-first acquisition is the *scaling direction*, and the crate that will
hold it is `census-crawl` (`ARCHITECTURE.md:94-95`). Today's home is `crates/census-service/src/census/`.

The objective's metric "verified useful records / physical request" does **not** exist in the tree;
only raw request accounting does (`sources/mod.rs:110-116`; `census/mod.rs:73-83`). No benchmark here
compares meet-first against athlete-first on a live source: the cost argument is structural.
