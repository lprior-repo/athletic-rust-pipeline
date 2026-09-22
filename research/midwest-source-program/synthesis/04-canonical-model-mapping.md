# Canonical model mapping — school → coach → athlete → meet → performance

How the discovered sources bind into the plan's canonical entities, which id joins each, and what stays
unjoined. Derived from reports 01–29 + `research/midwest/30-cross-source-pareto.md` §Canonical model mapping.

## Identity systems (the join substrate)

| Id | Owner | Scope | Notes |
|---|---|---|---|
| `AthleteID` | Athletic.net `[05]` | global | profile URL composable; present in every rankings row; `ani`/`athleticNetId` from AthleticLIVE/IHSA resolve to it |
| `TeamID` = `IDSchool` | Athletic.net `[01]` | global, NOT state-scoped | one id serves boys+girls; `Level:4` = HS; `TeamID 0` = unattached sentinel |
| `MeetID` | Athletic.net `[03]` | global | carried on every rankings row; seeds published by MHSAA/NSAA/WIAA/SD/OH |
| `IDResult` | Athletic.net `[05]` | per result | the dedupe key; relay rows' `AthleteID` is a `RelayTeamID` |
| `EventID`/`shortCode` | Athletic.net `[05]` | per event | scope tuple with `divListId, BaseDivID, gender, seasonKey, grades` |
| Association school ids | WI `orgID`, IL 4-digit `SchoolID`, OH `OhsaaSchoolId`, MI numeric, KS `Id`, ND `<id>`, NE/IA slugs `[29]` | per state | bridge schools without Athletic.net; DAT canonical rows (5,933) carry **empty** AN columns `[28]` |
| `student_id`/athlete ids | MSHSL 32-hex, TFRRS numeric, MileSplit network-global, Bound, DA `AthleteID` `[09][18][27][11][14]` | per platform | no crosswalk to Athletic.net except via name+school+grad year |
| Grade fields | route-dependent `[18][30]` | — | at-meet grade (Hy-Tek `Yr`/`Year`), current grade (roster `FR..SR`), or literal "Class of YYYY" — do not mix |

## Entity mapping

| Entity | Fields | Primary sources | Join key | Unjoined / caveat |
|---|---|---|---|---|
| **CanonicalSchool** | name, city, state, enrollment, class/division, co-op, website | associations `[06][09][13][15][17][19][21][23][24][25][26]`, DAT registry `[28]`, MileSplit teams `[27]` | association school id; Athletic.net `TeamID` where OH/MI/NE/SD/WI seeds apply | co-ops are first-class (MI separate program ids, ND 885-row sheet, SD 53 unmatched records); DAT has no city to disambiguate |
| **CanonicalCoach** | name, role, sport, gender side, professional email | `[29]` winners: WIAA, IHSA, OHSAA, MSHSL, KSHSAA, NSAA, NDHSAA, MHSAA, Bound IA/SD | school + sport + role (no person id published) | MO/IN absent; emails opt-in in IA/SD; names-only in MN(sport)/MI/NE/ND; privacy filters required (`05-compliance-and-risks.md` §2) |
| **CanonicalAthlete** | name, school, grad year, gender, `AthleteID`, profile URL | ATN `[04][05]`; AthleticLIVE `ani` `[10][12][25]`; IHSA `athleticNetId` `[13]`; MileSplit roster grad year `[27]` | `AthleteID` where present; else normalized name+school+state+grad year | MileSplit-only complement (34.1 %) has no AN id; DAT/MileSplit athlete ids are separate namespaces |
| **CanonicalMeet** | name, date, venue, class/division, source meet id | association seeds `[15][24][06][26][25]`, AthleticLIVE `[10][12]`, MileSplit `[27]`, DAT index `[28]` | AN `MeetID` where exposed; else name+date+state | no AN meet-calendar enumerator `[03]`; MileSplit meet ids are not date-monotonic `[27]` |
| **CanonicalPerformance** | event, mark, wind, round/heat, FAT, place, result id, date | ATN `[03][04]`; AL blobs `[10][12][25]`; association PDFs `[13][19][21][23][26]`; MileSplit HTML `raw` pages `[27]` | AN `IDResult` or (`AthleteID`,`MeetID`,`EventID`) | relay legs: AN `relayTeamMembers`/`Members[].IDAthlete`; AL relays carry leg names + grade `[12][25]`; MileSplit `/api` disallowed for production |

## Athletic.net-request minimization flow (validated against reality)

```
official school/state indexes        ~60 req/season   [06][09][13][17][19][21][23][24][25][26]
  -> candidate schools               (+24 DAT leagues [28])
  -> MileSplit rosters / DAT / timers 6,645 + timer files  [27][18][08][12][25]
  -> known AN ids where handed over:
       OH literal URLs, IL athleticNetId, MI 89 MeetIDs/season, SD inventory,
       AL `ani`, WIAA tournament exports, NSAA 28 MeetIDs          ~20 req  [19][13][15][26][10][06][24]
  -> targeted AN: whole-meet 3 XHR for meets nothing else covers; Bio 1 req for tracked-core progression [03][02]
  -> canonical reconciliation on AthleteID/IDResult, else name+school+grad year
```

Contradictions with the original plan, recorded: DAT is a discovery source **only in Indiana** `[28]`;
MileSplit carries **no** Athletic.net links `[27][18]`; several associations carry no ids at all
(IN/MO) `[17][21]`; coach data is absent in MO/IN `[29]`.
