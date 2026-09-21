# ADR-004: Meet-first result ingestion

## Status

Accepted. Implemented where a source allows it: bulk result adapters under
`crates/midwest-census/src/sources/**` (meet/result payloads) and the meet-oriented evidence
model (`CanonicalMeet`, `CanonicalEvent`, `Performance` in
`crates/census-domain/src/model.rs` since the Phase 3 crate split).

## Context

The census needs performances for tens of thousands of athletes. Two acquisition shapes are
possible:

```text
meet-first            athlete-first
----------            -------------
1 meet payload   ->   N athlete profiles
  hundreds to          tens of rows each,
  thousands of rows    one request per athlete
```

Athlete-first acquisition is the shape the previous pipeline drifted into, and it is the single
largest avoidable source of requests: fetching 500 athlete profiles to obtain performances that
one meet payload already contains multiplies traffic by two to three orders of magnitude. It
also produces worse data, because a profile view is a *projection* of results the timing
provider published first, and the projection can lag or omit entries.

## Decision

**Acquire whole meets first; enrich athletes second.**

* A discovered meet is a work item whose acquisition yields complete result payloads
  (finals, heats, rounds, relays, field events) for that meet.
* Athlete profiles are fetched only for what the meet data cannot supply:

  ```text
  PR/progression history beyond the discovered meets
  source profile URL
  grade/graduation evidence
  school history across transfers
  sports not covered by the meet corpus (XC vs TF linkage)
  identity resolution
  ```

* Cost is compared explicitly before scheduling: prefer one bulk response over N profile
  requests whenever both can produce the same performances, and record the choice in coverage
  metrics (`verified useful records / physical request`, §45).
* A meet payload's result rows are the *strongest* evidence of a performance; a profile's
  reported PR is retained as separate, source-reported evidence and never overwrites the
  calculated PR (§23).

## Consequences

* Meet discovery becomes the critical path: the national census quality depends on enumerating
  sanctioned meets per jurisdiction and season, and on knowing each source's meet identifiers.
* Athlete discovery happens *through* results (and rosters), which means athlete identity work
  starts from school + season + event participation — a much better-conditioned problem than
  name search.
* Adapters must parse whole-meet documents (HTML tables, timing-provider payloads, CSV/XLSX
  exports) robustly, including heat/round structure, rather than scraping one athlete at a time.
* The performance tables in Fjall are append-shaped and large by design; partitioning and prefix
  scans (ADR-001, §31) exist to serve exactly this shape.
