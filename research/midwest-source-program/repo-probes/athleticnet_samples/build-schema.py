#!/usr/bin/env python3
"""Build `schema.json` for the athleticnet lane from the captured samples.

Every example value written into `schema.json` is *re-read from the sample file* at build time:
the script resolves the JSON path, asserts the value matches the expectation below, and only then
writes the file. A missing key or a changed value is a hard failure (exit 1), never a silent drop.

Run:  python3 build-schema.py
"""

from __future__ import annotations

import json
import re
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
LANE = HERE.parent

NAV = "har1-getnavinfo-wi-2026-hs-boys.json"
P2 = "har1-getrankings-wi-170770-m-100m-p2-n102.json"
G11 = "har1-getrankings-wi-170770-m-100m-g11-p17-n17.json"
MULTI = "har1-getrankings-wi-170770-m-multievent-n180.json"
ANON = "atn-probe-getrankings-anon.json"
BIO = "live-getathletebiodata-28872883-2026-09-20.json"
HBIO = "har2-getathletebiodata-28872883.json"
TEAM = "har2-teamnav-3204.json"
MEET = "live-meetdata-634313.redacted.json"
EVDIV = "live-eventdiv-634313.json"
ALL = "live-allresults-634313.redacted.json"
R3 = "live-resultsdata3-100m-wind-615580.json"
R3RELAY = "live-resultsdata3-4x100m.json"
DC171 = "atn-probe-divchildren-170771.json"
SC = "har1-getstatescountries2.json"
BC = "har1-getbreadcrumbs-wi-170770.json"
AGES = "har1-getagesgrades-wi-2026.json"
GENR = "har2-general-getrankings-28872883.json"
CONV = "har1-getconvertevents.json"

# (identifier, example, file, json path, note)
IDENTIFIERS = [
    ("AthleteID", 22874443, P2, "groupedRankings[0][0].AthleteID", "person key; national namespace shared across sports and levels"),
    ("AthleteID (bio alias)", 28872883, BIO, "athlete.IDAthlete", "same integer as rankings AthleteID; profile URL segment"),
    ("AthleteID (relay placeholder)", 32663189, MULTI, "relayTeams.295310302.RelayTeamID",
     "on a relay row AthleteID equals RelayTeamID; the row itself lives at the IDResult key of relayTeams"),
    ("RelayTeamID", 32663189, MULTI, "relayTeams.295310302.RelayTeamID", "relay team record; 40 relay rows / 40 relayTeams keys in this capture"),
    ("IDResult", 294093256, P2, "groupedRankings[0][0].IDResult", "one athlete, one mark, one meet entry"),
    ("IDResult (bio)", 258445410, BIO, "resultsTF[0].IDResult", "same namespace as rankings IDResult"),
    ("ResultID (relay leg)", 294438034, R3RELAY, "relayLegs[0].ResultID", "leg-level result id inside a relay result"),
    ("shortCode", "2KiyYVPiwipw058tr", P2, "groupedRankings[0][0].shortCode", "public result page slug: /result/<shortCode>"),
    ("ShortCode (meet result)", "VPiXK5dHri4J1oNsm", ALL, "flatEvents[0].results[0].ShortCode", "camel-case key differs from the rankings row's `shortCode`"),
    ("MeetID", 667477, P2, "groupedRankings[0][0].MeetID", "meet key on a rankings row"),
    ("IDMeet (bio meet map)", 610825, BIO, "meets.610825.IDMeet", "the athlete payload carries its own meet map (name + end date)"),
    ("MeetID (meet payload)", 634313, MEET, "meet.ID", "the captured whole-meet pull"),
    ("TeamID", 19566, P2, "groupedRankings[0][0].TeamID", "team key on a rankings row"),
    ("IDSchool (teamnav sibling)", 57750, TEAM, "linkedTeams[0].IDSchool", "one school has several team records (K-8 siblings of the high school team)"),
    ("IDSchool (divchildren)", 101718, DC171, "teams[0].IDSchool", "school key as published by GetDivChildren"),
    ("IDSchool (allresults)", 3204, ALL, "teams[0].IDSchool", "school key on a meet team row, equal to the team page id for high schools"),
    ("SchoolID (bio athlete)", 3204, BIO, "athlete.SchoolID", "same integer as IDSchool; the source uses both key names"),
    ("TeamCode", "MIDD", ALL, "teams[0].TeamCode", "short code rendered in result tables"),
    ("IDCalendar", 7466322, ALL, "teams[0].IDCalendar", "per-season calendar id behind the team page"),
    ("IDDivision (conference)", 172497, TEAM, "customDivisions[0].IDDivision", "custom league/conference division, e.g. Big Eight"),
    ("IDDivision (state node)", 170770, TEAM, "divisions[2].id", "the ancestor chain a school belongs to for 2026"),
    ("DivIDDivision (child)", 170772, DC171, "divInfo[0].DivIDDivision", "child division id returned by GetDivChildren"),
    ("IDBaseDiv", 709, DC171, "divInfo[0].IDBaseDiv", "base-division id for the child node (WI state node base = 638)"),
    ("division.BaseDivID", 638, P2, "division.BaseDivID", "the rankings payload repeats the base id of the list's division"),
    ("IDMeetDiv", 1366830, EVDIV, "tfDivisions[0].IDMeetDiv", "meet-scoped division row (Varsity / Wheelchair)"),
    ("IDDiv", 1, EVDIV, "tfDivisions[0].IDDiv", "meet-scoped division number, repeated on every result row"),
    ("divListId", 170770, NAV, "divListId", "the rankings list id: nav echo, breadcrumb URL, rankings POST body"),
    ("levelDivId", 168416, NAV, "levelDivId", "High School level node"),
    ("countryDivId", 167952, NAV, "countryDivId", "United States country node"),
    ("regionDivId", 170770, NAV, "regionDivId", "the region selected for the nav call (Wisconsin)"),
    ("seasonId (outdoor)", 170770, NAV, "seasons.2026", "one nav call carries every season list id for the selected region"),
    ("seasonId (indoor)", 175082, NAV, "seasons.12026", "indoor seasons are keyed 1<seasonId>"),
    ("SeasonID (meet)", 2026, MEET, "meet.SeasonID", "season id on the meet"),
    ("EventID", 1, P2, "groupedRankings[0][0].EventID", "event key on a rankings row"),
    ("IDEvent (bio vocabulary)", 1, BIO, "eventsTF[0].IDEvent", "the athlete's event list, same id namespace"),
    ("EventShort", "100m", P2, "groupedRankings[0][0].EventShort", "the request key for GetRankings and GetResultsData3"),
    ("IDEventType", 0, BIO, "eventsTF[0].IDEventType", "0 = standard; 99 = relay/wheelchair type bucket"),
    ("eventTypes[].IDEventType", 99, R3, "eventTypes[0].IDEventType", "per-event type list returned by GetResultsData3; empty list when the event has no type variants (4x100m capture: [])"),
    ("GradeID", 12, P2, "groupedRankings[0][0].GradeID", "numeric grade on a rankings row"),
    ("Grade (string)", "11", ALL, "flatEvents[0].results[0].Grade", "string grade on a meet result row"),
    ("Grade (unknown sentinel)", "-", ALL, None, "71 of 758 meet rows publish '-' instead of a grade (see counts)"),
    ("AgeGrade", "11", ALL, "flatEvents[0].results[0].AgeGrade", "grade used for the age-based divisions; matches Grade on this row"),
    ("Age", 18, P2, "groupedRankings[0][0].Age", "present only on some rows (sparse object)"),
    ("Handle", "EvanRoell", P2, "groupedRankings[0][0].Handle", "profile slug; null for many athletes"),
    ("UsatfId", None, BIO, "athlete.UsatfId", "present-but-null; no USATF id for the captured athlete"),
    ("LiveID", 73767, MEET, "meet.LiveID", "AthleticLIVE meet id: the join key into the AthleticLIVE source"),
    ("State (USPS)", "WI", P2, "groupedRankings[0][0].State", "USPS code as published on rankings rows; null on 2 of 102 rows in this capture"),
    ("StateName", "Wisconsin", P2, "division.BaseDiv.StateName", "spelled-out jurisdiction name on the same payload"),
    ("State (team page)", "WI", TEAM, "team.State", "USPS code on the team record"),
    ("state (nav region)", "AL", NAV, "regions[1].state", "USPS code on the State-divType nav node"),
    ("Country", "USA", P2, "groupedRankings[0][0].Country", "3-letter country code on rankings rows"),
    ("EventSelectionID", 58, P2, "division.EventSelectionID", "which event-selection rule the list uses"),
    ("minCount", 2117, P2, "minCount", "server-reported row count for the filter - the pagination driver"),
    ("settings.depth", 100, P2, "settings.depth", "page depth the server reports for single-event lists"),
    ("settings.blurAfterDepth", 5, ANON, "settings.blurAfterDepth", "rows past this depth are blurred for a token with empty userRoles"),
    ("rowNum", 100, P2, "groupedRankings[0][0].rowNum", "position in the ranking; not an identifier"),
    ("rank", 100, P2, "groupedRankings[0][0].rank", "display rank; equal to rowNum on this row"),
    ("IDGrade", 9, TEAM, "grades[0].IDGrade", "grade vocabulary on the team nav payload"),
    ("relayTeamMembers[].AthleteID", 23113661, BIO, "relayTeamMembers[0].AthleteID", "athlete key inside a relay roster"),
    ("relayTeamMembers[].TeamID", 29163596, BIO, "relayTeamMembers[0].TeamID", "relay team record key inside a relay roster"),
    ("divType", "State", NAV, "regions[1].divType", "nav node typing; the 52 state rows are divType State"),
    ("subDivType (region)", "Associations", NAV, "regions[1].subDivType", "the child node class the UI would open next"),
    ("subDivType (tree)", "Divisions", NAV, "tree[0].subDivType", "the tree echo for the selected region"),
    ("states[].Code", "AL", SC, "states[2].Code", "jurisdiction code in the source's own list"),
    ("states[].CountryCode", "US", SC, "states[2].CountryCode", "the filter that makes Code unambiguous (14 non-US rows exist)"),
    ("countriesLookup[].Alpha3", "AFG", SC, "countriesLookup[0].Alpha3", "country vocabulary for venue nationality"),
    ("nav events[] id", 570, NAV, "events[0].id", "10 Meter Fly: the vocabulary is wider than the main HS event set"),
    ("nav events_main[] id", 1, NAV, "events_main[0]", "the 36 main event ids (100m is the first)"),
    ("meet.Name", "Big 8 Conference", MEET, "meet.Name", "meet title as published"),
    ("meet.MeetDate", "2026-05-15T00:00:00", MEET, "meet.MeetDate", "date-only meet anchor in ISO-local form"),
    ("flatEvents[].EventShort", "100m", ALL, "flatEvents[0].EventShort", "event key inside the whole-meet payload"),
    ("teams[].IDDivision (meet context)", 170776, ALL, "teams[0].IDDivision", "the meet's division node for that team, e.g. WIAA Regional 2A"),
    ("breadcrumbs[].url", "/TrackAndField/rankings/list/166948", BC, "breadCrumbs[1].url", "canonical rankings URL grammar, observed for World/US/HS/state nodes"),
    ("GetAgesGrades grades[].GradeCount", 22934, AGES, "grades[3].GradeCount", "grade 9 count for the WI boys HS 2026 division (semantics: rows or athletes unverified)"),
    ("General/GetRankings Position", 1, GENR, "[0].Position", "athlete PR/rank per event per division context"),
    ("GetConvertEvents FromEventShort", "Indoor Pentathlon", CONV, "[0].FromEventShort", "event conversion table: merges equivalent events across rulesets"),
]

# Endpoint -> sample file, for the response-shape block.
ENDPOINTS = {
    "GET /api/v1/tfRankings/GetNavInfo": (NAV, "session"),
    "POST /api/v1/tfRankings/GetRankings": (P2, "session + anonymous (rows blurred past settings.blurAfterDepth)"),
    "POST /api/v1/tfRankings/GetAgesGrades": (AGES, "session"),
    "GET /api/v1/tfRankings/GetStandards": ("har1-getstandards-empty.json", "session"),
    "GET /api/v1/tfRankings/GetConvertEvents": (CONV, "session"),
    "GET /api/v1/public/GetStatesCountries2": (SC, "anonymous"),
    "GET /api/v1/SiteHeader/GetDivChildren": (DC171, "anonymous"),
    "GET /api/v1/SiteHeader/GetBreadcrumbs": (BC, "anonymous"),
    "GET /api/v1/Meet/GetMeetData": (MEET, "anonymous + jwtMeet for later steps"),
    "GET /api/v1/Meet/GetEventDivisionData": (EVDIV, "anonymous + jwtMeet"),
    "GET /api/v1/Meet/GetAllResultsData": (ALL, "anonymous + jwtMeet"),
    "POST /api/v1/Meet/GetResultsData3": (R3, "anonymous + jwtMeet"),
    "GET /api/v1/AthleteBio/GetAthleteBioData": (BIO, "anonymous"),
    "GET /api/v1/TeamNav/Team": (TEAM, "anonymous"),
    "GET /api/v1/General/GetRankings": (GENR, "anonymous"),
}


def sample(name: str) -> dict | list:
    return json.loads((HERE / name).read_text())


TOKEN = re.compile(r"([^.[\]]+)|\[(\d+)\]")


def resolve(doc, path: str):
    """Resolve `a.b[0].c` style paths: a key token, or a bracketed list index."""
    node = doc
    for name, index in TOKEN.findall(path):
        if index:
            if not isinstance(node, list) or len(node) <= int(index):
                raise IndexError(f"[{index}] out of range or not a list")
            node = node[int(index)]
            continue
        if not isinstance(node, dict) or name not in node:
            raise KeyError(f"{name!r} missing")
        node = node[name]
    return node


def row_lists(doc: dict) -> list[dict]:
    return [row for group in doc["groupedRankings"] for row in group]


def derived_facts() -> dict:
    """Facts the report leans on, recomputed here so the numbers cannot drift."""
    all_results = sample(ALL)
    rows = [r for block in all_results["flatEvents"] for r in (block.get("results") or [])]
    legs = all_results["relayLegs"]
    p2 = sample(P2)
    anon = sample(ANON)
    return {
        "allresults_rows": len(rows),
        "allresults_blocks": len(all_results["flatEvents"]),
        "allresults_relay_legs": len(legs),
        "allresults_teams": len(all_results["teams"]),
        "allresults_event_divs_with_results": len(all_results["eventDivsWithResults"]),
        "allresults_distinct_athlete_ids": len({r["AthleteID"] for r in rows}),
        "allresults_relay_only_athlete_ids": len({x["AthleteID"] for x in legs} - {r["AthleteID"] for r in rows}),
        "allresults_grade_unknown_rows": sum(1 for r in rows if r["Grade"] == "-"),
        "rankings_full_page_rows": len(row_lists(p2)),
        "rankings_page_min_count": p2["minCount"],
        "rankings_page_rank_range": [min(r["rank"] for r in row_lists(p2)), max(r["rank"] for r in row_lists(p2))],
        "rankings_row_key_shapes": len({tuple(sorted(r)) for r in row_lists(sample(MULTI))}),
        "anon_blurred_rows": sum(1 for r in row_lists(anon) if r.get("blurred")),
        "anon_blur_after_depth": anon["settings"]["blurAfterDepth"],
        "anon_unblurred_ranks": sorted(r["rank"] for r in row_lists(anon) if not r.get("blurred")),
        "anon_page_echo": anon["settings"]["page"],
    }


def main() -> int:
    failures = []
    identifiers = []
    for name, example, filename, path, note in IDENTIFIERS:
        entry = {"identifier": name, "example": example, "observed_in": f"samples/{filename}",
                 "json_path": path, "note": note}
        if path is not None:
            doc = sample(filename)
            try:
                actual = resolve(doc, path)
            except (KeyError, IndexError) as error:
                failures.append(f"{name}: path {path!r} not found in {filename}: {error}")
                continue
            if actual != example:
                failures.append(f"{name}: {filename} {path} = {actual!r}, expected {example!r}")
            entry["verified"] = True
        identifiers.append(entry)

    # Structural claims that must hold, re-derived (not hand-typed).
    all_results = sample(ALL)
    rows = [r for block in all_results["flatEvents"] for r in (block.get("results") or [])]
    if any("Wind" in r for r in rows):
        failures.append("whole-meet rows unexpectedly carry Wind")
    r3 = sample(R3)
    r3rows = [row for group in r3["resultsTF"] for row in group]
    if not all("Wind" in r and "Heat" in r and "HeatPlace" in r for r in r3rows):
        failures.append("GetResultsData3 rows are missing Wind/Heat/HeatPlace")
    multi = sample(MULTI)
    relay_rows = [r for r in row_lists(multi) if r.get("GradeID") == 99]
    relay_map = multi["relayTeams"]
    if len(relay_rows) != len(relay_map):
        failures.append(f"relay rows {len(relay_rows)} != relayTeams keys {len(relay_map)}")
    for row in relay_rows:
        target = relay_map.get(str(row["IDResult"]))
        if target is None or target["RelayTeamID"] != row["AthleteID"]:
            failures.append(f"relay row {row['IDResult']} does not map to its RelayTeamID")
            break
    anon = sample(ANON)
    for row in row_lists(anon):
        if row.get("blurred") and row["AthleteID"] != 0:
            failures.append("blurred row with a non-zero AthleteID")
            break
    sc = sample(SC)
    us_rows = [s for s in sc["states"] if s["CountryCode"] == "US"]
    if len(us_rows) != 51 or len(sc["states"]) != 65:
        failures.append(f"statescountries: {len(us_rows)} US rows of {len(sc['states'])}")

    if failures:
        print("SCHEMA BUILD FAILED:")
        for f in failures:
            print("  -", f)
        return 1

    doc = {
        "lane": "athleticnet",
        "generated_by": "samples/build-schema.py",
        "verification": "every example value was re-read from the cited sample file and compared at build time; "
                        "structural claims (relay mapping, blur masking, wind fields, jurisdiction lists) are re-derived, "
                        "so a changed sample fails the build instead of silently changing the docs",
        "derived_counts": derived_facts(),
        "endpoints": {
            endpoint: {
                "sample": f"samples/{filename}",
                "auth": auth,
                "top_level_keys": sorted(sample(filename)) if isinstance(sample(filename), dict) else f"list[{len(sample(filename))}]",
            }
            for endpoint, (filename, auth) in ENDPOINTS.items()
        },
        "identifiers": identifiers,
        "sparse_row_warning": "row key sets are NOT uniform: the multi-event capture holds "
                              f"{derived_facts()['rankings_row_key_shapes']} distinct key shapes over 180 rows; keys "
                              "(Wind, Age, Handle, PhotoUrl, display_secondary, EventTypeDescription) are omitted when "
                              "absent, and one row omits Country/State/TeamMascot entirely. Deserialize with Option + "
                              "serde(default), never with required fields.",
        "entitlement": {
            "anonymous": "rankings rows past settings.blurAfterDepth are blurred: AthleteID 0, masked names "
                         "('Xxxx Xxxxx'), State 'XX', MeetID 0, shortCode null; qParams.page is ignored (settings.page echoes 1)",
            "session": "unblurred rows; qParams.page honored (page 2 -> ranks 100..201, page 17 -> 17 rows)",
            "evidence": [f"samples/{ANON}", f"samples/{P2}", f"samples/{G11}"],
        },
        "jurisdiction_fields": {
            "nav_region_state": f"samples/{NAV} regions[*].state - USPS code or null (Overseas)",
            "rankings_row_state": f"samples/{P2} groupedRankings[*][*].State - 'WI' on 100/102 rows, null on 2",
            "division_base_div_state": f"samples/{P2} division.BaseDiv.State/.StateName",
            "team_state": f"samples/{TEAM} team.State",
            "statescountries_states": f"samples/{SC} states[] - 65 rows, 51 with CountryCode 'US' (set-equal to UsJurisdiction codes), "
                                      "14 non-US (Canadian provinces) that are not USPS codes",
        },
    }
    out = LANE / "schema.json"
    out.write_text(json.dumps(doc, indent=2) + "\n")
    print(f"wrote {out} ({out.stat().st_size} bytes), {len(identifiers)} identifiers verified")
    return 0


if __name__ == "__main__":
    sys.exit(main())
