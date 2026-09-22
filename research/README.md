# National source research — lane index and evidence contract

This tree is the input side of the Class-of-2027 census: what each source publishes, how it enumerates,
what it costs to acquire, and whether it joins the identities the census already holds. Every lane is
one source family, written by one agent, and every claim in it points at a raw capture in the same lane.

## Lane contract

Each lane lives at `research/sources/<lane>/` and owes four things:

| Path | Contents |
| --- | --- |
| `SOURCE_REPORT.md` | The report, one section per source (or per jurisdiction for association lanes). Required fields below. |
| `samples/` | Byte-exact raw captures (HTML/JSON/CSV/XLSX) plus `samples/CAPTURES.md`: one line per file — URL, HTTP status, bytes, UTC timestamp, exact command. |
| `schema.json` | The field and identifier schema actually observed, each field carrying one real example value and the capture file it came from. |
| `coverage.json` | Machine-readable coverage: what enumerates, what does not, measured counts, and the gaps. |

Required `SOURCE_REPORT.md` fields (objective §13):

```text
Source name
Geographic coverage
Sports
Historical depth
Discovery mechanism
Stable identifiers
Pagination
Athlete fields
Meet fields
Result fields
Grade/class evidence
Coach/contact fields
Public API availability
Static file availability
Browser requirement
Request cost
Published rate limits
Known blocks
Cross-source join keys
Estimated marginal coverage
Implementation recommendation
```

`Implementation recommendation` is one of: `PRIMARY`, `RESULT_SOURCE`, `DISCOVERY_SOURCE`,
`COACH_SOURCE`, `VALIDATION_SOURCE`, `CONDITIONAL`, `REJECT`.

## Evidence rules

- A number without a capture file behind it does not exist. Cite `samples/<file>` or the command that
  produced it, inline, next to the number.
- Reasoning that was not observed is marked `[INFERENCE]` where it appears.
- Never bypass authentication, a CAPTCHA, a paywall or an access control. Record the block as a finding.
- Check and record `robots.txt` before fetching a host; keep to one request per second per host.
- Public professional contact information for coaches and athletic directors only. No athlete contact
  data, no personal email, no personal phone, no home address.

## Lanes

| Lane | Jurisdictions / scope | Status |
| --- | --- | --- |
| `milesplit-national/` | MileSplit, 51 jurisdictions | in progress |
| `athleticnet/` | Athletic.net, 51 jurisdictions | in progress |
| `national-aggregators/` | DirectAthletics/TFRRS, AthleticLIVE, RunnerSpace, MaxPreps, World Athletics | in progress |
| `timing-providers-national/` | FlashResults, PrimeTime, Wayzata, FinishLynx/MeetPro, RACE RESULT, OpenTrack | in progress |
| `coach-directories-national/` | Official coach/AD directories, 51 jurisdictions | in progress |
| `state-assoc-westcoast/` | CA, OR, WA, AK, HI, NV | in progress |
| `state-assoc-mountain/` | AZ, CO, ID, MT, NM, UT, WY | in progress |
| `state-assoc-southcentral/` | AL, AR, KY, LA, MS, OK, TN, TX | in progress |
| `state-assoc-southeast/` | FL, GA, NC, SC, VA, WV | in progress |
| `state-assoc-midatlantic/` | DC, DE, MD, NJ, NY, PA, CT, MA, RI, NH, VT, ME | in progress |
| `state-assoc-plains/` | IA, KS, MN, MO, ND, NE, SD (consolidation of the Midwest phase) | in progress |
| `state-assoc-greatlakes/` | IL, IN, MI, OH, WI (consolidation of the Midwest phase) | in progress |

A lane is a *source family*, not always a single provider: an association lane covers several states, and
each provider inside it is one section of that lane's `SOURCE_REPORT.md` with its own `schema.json`
entry and captures. The per-provider section is the unit that becomes an adapter, so a provider section
must stand alone — a reader who opens only that section can decide the recommendation without reading
its siblings. (The objective's per-provider directory layout is satisfied by per-provider sections plus
captures; splitting the tree per provider would fragment the capture evidence that supports them.)

The prior Midwest phase lives outside this tree: `~/Downloads/midwest-tfxc-source-research/`
(`research/midwest/01..46*.md`, `synthesis/*.md`, `data/*.csv`, `reports/*.xlsx`). Consolidation lanes
cite it by path rather than copying it, so one copy stays authoritative.

## From lanes to adapters

A lane becomes an adapter only when the objective's ranking says it earns the engineering time:

```text
new verified Class-of-2027 athletes + new performances + new school/coach coverage + independent corroboration
-------------------------------------------------------------------------------------------------------------
                                    engineering effort + request cost
```

The census prefers one bulk meet payload over N athlete profile calls (ADR-004), so a lane that returns a
whole meet scores far above a lane that returns one athlete per request, at equal athlete yield.
