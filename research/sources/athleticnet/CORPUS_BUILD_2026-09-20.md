# Athletic.net endpoint findings — 2026-09-20 corpus build (dated evidence)

Dated record inherited from the deleted `tools/athletic-net-pr-population/README.md`. That directory's
Python tooling was removed by the repository's no-Python directive, and the workbook mission it served
(`Athletic-2026-US-Boys-Grade11-All-Athletes.xlsx`, 142,705 rows, and its PR-populated derivative) is a
retained external artifact, **not** this census's population seed (ADR-013). The workbook and its CSVs
are not in this repository.

What remains useful here is the endpoint/field semantics that corpus build measured. This lane's
`SOURCE_REPORT.md` (2026-09-21/22 captures in `samples/`) is the lane contract; where it already states a
fact (ids, tiering, blurring, request costs) the report wins, and nothing below was re-measured for it.

## Rankings endpoint semantics

Request shape actually used (anonymous, browser-grade headers required — a default `curl` UA got the 403
interstitial):

```
POST https://www.athletic.net/api/v1/tfRankings/GetRankings
{"reportType":"div","mode":"list","divListId":168416,"indoor":null,
 "eventShort":"100m","gender":"m","qParams":{"grades":[11],"page":1},
 "qualifyingListKey":"","version":2,"debug":""}
```

* `divListId: 168416` = national boys; `gender: "m"`; `grades: [11]` = Class of 2027.
* Response carries `groupedRankings` (list of lists), `relayTeams`, `minCount`, `eventShort`, `gender`;
  101 rows per page, page until empty.
* Row fields observed: `AthleteID, AthleteName, GradeID, TeamID, TeamName, State, Event, EventShort,
  display, SortIntRaw, PersonalBest, PersonalEvent, SeasonID, ResultDate, MeetID, IDResult`.
  `display` is the mark; within one event the minimum `SortIntRaw` is the best mark.
* `SortIntRaw` is an event-specific fixed-point: field events return a metric `display` (`"20.51m"`) with
  the imperial form in `SortIntRaw` (`20'4.5"`). Never compare `SortIntRaw` across events.
* Invalid marks (`ND`, `NH`, `FOUL`, `DNS`, `DNF`, `X`) carry a sentinel (`SortIntRaw = 20000001`) and no
  digits in `Result`; filter on "no digit in `Result`" rather than on the sentinel value alone.
* Relays (`4x100m`, `distmed*`, `sprintmed*`, `swedish1234`, `*shuttleh*`, `decathlon`/`heptathlon`/
  `pentathlon`) return **0 rows** for `grades: [11]` — relay rows carry the placeholder `GradeID = 99`.
  Query relays with `grades: []` instead. (The lane report independently establishes `GradeID == 99` as
  the relay/masking placeholder.)

## Athlete bio endpoint PR semantics

`GET /api/v1/athlete/<AthleteID>/GetAthleteBioData` returns per-event PR objects shaped
`{Event: {Result, SortIntRaw, SeasonID, ...}}`. All-time PR = minimum `SortIntRaw` across seasons;
a season PR = minimum restricted to that `SeasonID` (2026 indoor/outdoor keys `12026`/`2026`, per the
lane report's season-key rule).

## Politeness measured during that build (self-imposed, not a published limit)

The corpus build's fetcher targeted 0.85 s/request (~1.2 req/s) and reported 429s starting near
~2.1 req/s; 1.25 req/s sustained did not trip them. The operative rules are the lane report's
≤1 request/second/host research discipline and the runtime's registry admission
(`crates/census-crawl/src/registry/policy.rs`), not this measurement.
