#!/usr/bin/env python3
"""Emit coverage.json: per-jurisdiction tier-1 directory coverage with measured fields.

Every value is derived from the saved artefacts:
  tools/assoc-probe-home.json   - association homepage fetch (status/bytes/title)
  tools/dir-probe.json          - tier-1 directory fetch (status/bytes/effective/fields)
  tools/dir-evidence.json       - literal field counts over the saved sample
  tools/fetch-log.jsonl         - robots.txt outcome per host
Manual verdicts (coach/AD name+email) are encoded in VERDICT with the sample that
proves them; nothing is asserted without a sample reference.
"""
from __future__ import annotations

import json
from pathlib import Path

LANE = Path(__file__).resolve().parent.parent
T = LANE / "tools"

# state -> verdict for the tier-1 surface, each key citing the sample that proves it.
#   school/ad_name/ad_email/coach_name/coach_email/tf_xc  in {yes,no,partial,unknown,n/a}
VERDICT: dict[str, dict] = {
    "AL": dict(school="n/a", ad_name="n/a", ad_email="n/a", coach_name="n/a", coach_email="n/a", tf_xc="n/a",
               why="robots.txt `User-agent: * Disallow: /` -> not fetched (see samples/CAPTURES.md refusals)"),
    "AK": dict(school="yes", ad_name="no", ad_email="no", coach_name="no", coach_email="no", tf_xc="no",
               why="218-row school table: SCHOOL/PHONE/EN/RG/CL/ADDRESS/CITY/ZIP/DISTRICT; no staff or coach column (samples/dir/AK__member-schools.html)"),
    "AZ": dict(school="unknown", ad_name="unknown", ad_email="unknown", coach_name="unknown", coach_email="unknown", tf_xc="unknown",
               why="/schools is a JS shell (2 117 visible chars, 0 school rows); data endpoint not located (samples/dir/AZ__schools.html)"),
    "AR": dict(school="unknown", ad_name="no", ad_email="no", coach_name="no", coach_email="no", tf_xc="unknown",
               why="AAA's menu item 'Public Directory' resolves to go.dragonflyathletics.com, a client-rendered DragonFly app whose served HTML has no directory content (visible text 'DragonFly'); AAA's own site serves 0 <tr> (samples/dir/AR__dragonfly.html, samples/dir/AR__home.html)"),
    "CA": dict(school="unknown", ad_name="unknown", ad_email="unknown", coach_name="unknown", coach_email="unknown", tf_xc="unknown",
               why="cifstate.org homepage returned HTTP 202 with an empty body; CIF is 10 autonomous sections, no state directory observed"),
    "CO": dict(school="no", ad_name="no", ad_email="no", coach_name="no", coach_email="no", tf_xc="no",
               why="schools.chsaa.org redirects to /login?callbackUrl=/dashboard - member-only School Center (samples/dir/CO__schools2.html)"),
    "CT": dict(school="yes", ad_name="yes", ad_email="no", coach_name="yes", coach_email="no", tf_xc="yes",
               why="FusionPoint directory, 190 school tables: 182 schools carry at least one XC/indoor+outdoor-track Head Coach row (1 043 such rows), 189 Athletic Director rows; the 4th column is a tel: phone (often the school main line, sometimes empty) and there is no email column (samples/dir/CT__directory.html; counted by tools/dir_coach_counts.py)"),
    "DE": dict(school="unknown", ad_name="unknown", ad_email="unknown", coach_name="unknown", coach_email="unknown", tf_xc="unknown",
               why="diaa.org HTTP 403 'Just a moment...' Cloudflare interstitial to this client (samples/dir/DE__home.html)"),
    "DC": dict(school="unknown", ad_name="unknown", ad_email="unknown", coach_name="unknown", coach_email="unknown", tf_xc="unknown",
               why="www.dcsaa.org fetched (109 872 B) but no directory table or contact field served (samples/dir/DC__home.html)"),
    "FL": dict(school="unknown", ad_name="no", ad_email="no", coach_name="no", coach_email="no", tf_xc="unknown",
               why="/schools 404; SPA shell only (samples/dir/FL__schools.html)"),
    "GA": dict(school="yes", ad_name="no", ad_email="no", coach_name="no", coach_email="no", tf_xc="no",
               why="GHSA member school directory = 40 rows of {school, street address, city/state/zip, colors}; the 'coach' hits are the nav item 'Coaches / ADs' (samples/dir/GA__school-directory.html)"),
    "HI": dict(school="unknown", ad_name="no", ad_email="no", coach_name="no", coach_email="no", tf_xc="unknown",
               why="member-schools page is a JS shell; no contact table (samples/dir/HI__schools.html)"),
    "ID": dict(school="unknown", ad_name="no", ad_email="no", coach_name="no", coach_email="no", tf_xc="unknown",
               why="idhsaa.org loads a Cloudflare Turnstile challenge (challenges.cloudflare.com/turnstile) - no directory data served to this client (samples/dir/ID__home.html)"),
    "IL": dict(school="yes", ad_name="yes", ad_email="yes", coach_name="yes", coach_email="yes", tf_xc="yes",
               why="api.ihsa.org/v1/schools (828 rows) + /staff2 + /staff/<PersonID>/email; verified 3/3 coach emails live (samples/dir/IL__ihsa-schools.html, samples/validate/IL_0101_staff2.html)"),
    "IN": dict(school="yes", ad_name="no", ad_email="no", coach_name="no", coach_email="no", tf_xc="no",
               why="IHSAA school-directory page: membership statistics + hand-off to myIHSAA.net; no per-school contact rows (samples/dir/IN__ihsaa-school-directory.html)"),
    "IA": dict(school="yes", ad_name="unknown", ad_email="unknown", coach_name="unknown", coach_email="unknown", tf_xc="unknown",
               why="IHSAA tier-1 list is {School, Nickname, School Colors, Conference} with no contacts (samples/dir/IA__member-schools.html). IHSAA member pages link out to Bound, whose directory is robots-disallowed (`Disallow: /*directory`); a compliant GET of the IA Bound directory returned a 118-byte 403 body (samples/validate/IA_Bound_adm.html). Contacts NOT verified by this lane; earlier table claims are retracted in tools/validation-retractions.json"),
    "KS": dict(school="yes", ad_name="yes", ad_email="yes", coach_name="no", coach_email="no", tf_xc="no",
               why="kshsaa-api.kshsaa.org/directory/search/name/a/ = 526 schools in 1 request, 526/526 AD name+email re-verified live (samples/dir/KS__kshsaa-api-letter-a.html)"),
    "KY": dict(school="unknown", ad_name="unknown", ad_email="unknown", coach_name="unknown", coach_email="unknown", tf_xc="unknown",
               why="KHSAA delegates its school directory to ArbiterLive: khsaa.arbitersports.com/front/104219/Site (samples/dir/KY__arbiterlive-note.html). arbiterlive.com and *.arbitersports.com both publish `User-agent: * Disallow: /`, so the delegated directory is NOT collectible under this contract (tools/robots/khsaa.arbitersports.com.txt)"),
    "LA": dict(school="unknown", ad_name="no", ad_email="no", coach_name="no", coach_email="no", tf_xc="unknown",
               why="LHSAA /school-directory is a JS shell; no contact rows served (samples/dir/LA__school-directory.html)"),
    "ME": dict(school="yes", ad_name="yes", ad_email="no", coach_name="yes", coach_email="no", tf_xc="yes",
               why="FusionPoint directory, 147 school tables: 81 schools carry an XC/indoor+outdoor-track Head Coach row (310 such rows), 149 Athletic Director rows; no email column (samples/dir/ME__directory.html; counted by tools/dir_coach_counts.py)"),
    "MD": dict(school="yes", ad_name="no", ad_email="no", coach_name="no", coach_email="no", tf_xc="no",
               why="wp-json/mpssaa/v1/search?q= returns school records; the school page carries 0 mailto and no AD name (samples/dir/MD__school-montgomery-blair.html)"),
    "MA": dict(school="unknown", ad_name="no", ad_email="no", coach_name="no", coach_email="no", tf_xc="unknown",
               why="miaa.net/schools and /about-miaa/miaa-member-schools are JS shells (0 <tr>, 13-69 KB, no contact rows); MIAA's Quick Links point to ArbiterSports, whose hosts publish `Disallow: /` (samples/dir/MA__member-schools.html)"),
    "MI": dict(school="yes", ad_name="unknown", ad_email="unknown", coach_name="unknown", coach_email="unknown", tf_xc="unknown",
               why="my.mhsaa.com AdministrationDirectory returns AD/Principal/Superintendent with MailAddress, BUT robots.txt disallows /DesktopModules/ for every user-agent -> not fetched this wave (tools/robots/my.mhsaa.com.txt)"),
    "MN": dict(school="yes", ad_name="yes", ad_email="no", coach_name="no", coach_email="no", tf_xc="no",
               why="mshsl.org/schools/<slug>: Activities Director name + Cloudflare-obfuscated email; verified 3/3 AD names live (samples/dir/MN__schools.html)"),
    "MS": dict(school="unknown", ad_name="unknown", ad_email="unknown", coach_name="unknown", coach_email="unknown", tf_xc="unknown",
               why="misshsaa.com HTTP 403 to this client (samples/dir/MS__home.html)"),
    "MO": dict(school="n/a", ad_name="n/a", ad_email="n/a", coach_name="n/a", coach_email="n/a", tf_xc="n/a",
               why="robots.txt has a second `User-agent: *` group with `Disallow: /`; combining groups (RFC 9309) blocks the whole host -> not fetched"),
    "MT": dict(school="unknown", ad_name="no", ad_email="no", coach_name="no", coach_email="no", tf_xc="unknown",
               why="/schools/directory/ and /schools/member-schools/ are JS shells (0 <tr>); the only buried API references are WordPress REST routes and an arbiter widget (samples/dir/MT__directory.html, tools/api-hunt.json)"),
    "NE": dict(school="yes", ad_name="yes", ad_email="no", coach_name="yes", coach_email="no", tf_xc="yes",
               why="NSAA POST 'school=<name>' directory returns role->name pairs including each sport's coach; 3/3 sampled schools re-verified live (coach name + AD name), no email field exists (samples/validate/NE_nsaa_Adams_Central.html). Row volumes 1 217 coach / 311 AD are from the prior Midwest study [29], not re-measured here"),
    "NV": dict(school="yes", ad_name="yes", ad_email="no", coach_name="no", coach_email="no", tf_xc="no",
               why="FusionPoint directory: 464 rows across 125 schools = Principal/AD/Athletic-Admin only; 0 XC/track coach rows (samples/dir/NV__directory.html)"),
    "NH": dict(school="unknown", ad_name="no", ad_email="no", coach_name="no", coach_email="no", tf_xc="unknown",
               why="/about-nhiaa/schools/ serves no school rows (visible text 2 439 chars) (samples/dir/NH__schools.html)"),
    "NJ": dict(school="unknown", ad_name="no", ad_email="no", coach_name="no", coach_email="no", tf_xc="unknown",
               why="/schools is an icon/nav shell with no directory rows (samples/dir/NJ__schools.html)"),
    "NM": dict(school="yes", ad_name="no", ad_email="no", coach_name="no", coach_email="no", tf_xc="no",
               why="member-schools page lists 137 school-name occurrences; no contact column (samples/dir/NM__member-schools.html)"),
    "NY": dict(school="unknown", ad_name="no", ad_email="no", coach_name="no", coach_email="no", tf_xc="unknown",
               why="nysphsaa.org/schools is 404; homepage serves no directory rows and the section pages are JS (samples/dir/NY__home.html)"),
    "NC": dict(school="yes", ad_name="partial", ad_email="partial", coach_name="no", coach_email="no", tf_xc="no",
               why="nchsaa.org/schools/ = 452 school rows + a 'Conference Administrators' table (78 rows, 74 mailto) = conference-level admin name+email, not school AD and not coaches (samples/dir/NC__schools.html)"),
    "ND": dict(school="yes", ad_name="yes", ad_email="no", coach_name="yes", coach_email="no", tf_xc="yes",
               why="ndhsaa.com/schools/<id>/<slug> carries two blocks: role->name pairs (Superintendent/Principal/AD) AND a 'Sport/Activity Offering | Coaches' matrix - e.g. Boys' Cross Country = Troy Wendt, Boys' Track and Field = 'Logan Krueger, Jordan Sanford', Girls' Cross Country = empty at this school. 3/3 AD names re-verified live; 0 emails (samples/validate/ND_Minot_North_1307.html)"),
    "OH": dict(school="yes", ad_name="yes", ad_email="yes", coach_name="yes", coach_email="yes", tf_xc="yes",
               why="officials.myohsaa.org SportsInformation (XC/TF head coaches w/ email) + AthleticDirector (AD w/ email); verified 4/4 live (samples/validate/OH_Dublin_Coffman_474_XC_coach.html)"),
    "OK": dict(school="unknown", ad_name="partial", ad_email="no", coach_name="no", coach_email="no", tf_xc="unknown",
               why="ossaaillustrated.com/athletic-directors/ is a landing page whose only data links are Google Docs plus ossaa.arbitersports.com; that host publishes `User-agent: * Disallow: /` (samples/dir/OK__athletic-directors.html, tools/robots/ossaa.arbitersports.com.txt)"),
    "OR": dict(school="yes", ad_name="no", ad_email="no", coach_name="no", coach_email="no", tf_xc="no",
               why="osaa.org/schools/full-members lists 248 school-name occurrences with no contact column (samples/dir/OR__full-members.html)"),
    "PA": dict(school="unknown", ad_name="partial", ad_email="no", coach_name="no", coach_email="no", tf_xc="unknown",
             why="piaa.org/schools/ serves 4 'athletic director' hits but no directory rows; PIAA school pages are JS (samples/dir/PA__schools.html)"),
    "RI": dict(school="yes", ad_name="yes", ad_email="no", coach_name="yes", coach_email="no", tf_xc="yes",
               why="FusionPoint directory, 55 school tables: 49 schools carry an XC/indoor+outdoor-track Head Coach row (259 such rows), 57 Athletic Director rows, 0 mailto (samples/dir/RI__directory.html; counted by tools/dir_coach_counts.py)"),
    "SC": dict(school="unknown", ad_name="no", ad_email="no", coach_name="no", coach_email="no", tf_xc="unknown",
               why="schsl-directory is a JS shell; no contact rows served (samples/dir/SC__schsl-directory.html)"),
    "SD": dict(school="unknown", ad_name="unknown", ad_email="unknown", coach_name="unknown", coach_email="unknown", tf_xc="unknown",
               why="UNVERIFIED. sdhsaa.com's homepage (1,096,210 B, 35 gobound links) points the member directory at https://www.gobound.com/sd/associations/sdhsaa/schools - that LINK is the only SD evidence in this lane. The association index itself returned a 118-byte 403 Forbidden to this client (samples/dir/SD__bound-sd-schools.html, visible_chars=27), and every school-directory path under it is additionally robots-disallowed by `Disallow: /*directory` (tools/robots/gobound.com.txt). No SD school list was ever observed here, so school/ad/coach columns are all unknown; the table claims previously attached to SD are retracted in tools/validation-retractions.json"),
    "TN": dict(school="unknown", ad_name="no", ad_email="no", coach_name="no", coach_email="no", tf_xc="unknown",
               why="tssaa.org/directory renders as a JS shell (452 visible chars) (samples/dir/TN__directory.html)"),
    "TX": dict(school="unknown", ad_name="no", ad_email="no", coach_name="no", coach_email="no", tf_xc="unknown",
               why="UIL /schools 404; homepage serves no directory rows (samples/dir/TX__home.html)"),
    "UT": dict(school="unknown", ad_name="no", ad_email="no", coach_name="no", coach_email="no", tf_xc="unknown",
               why="uhsaa.org/school-directory-new/ serves no rows; UHSAA directs schools to MaxPreps (samples/dir/UT__school-directory.html)"),
    "VT": dict(school="unknown", ad_name="no", ad_email="no", coach_name="no", coach_email="no", tf_xc="unknown",
               why="vpaonline.org/membership/ mentions AD roles and has 1 mailto (association office), no school directory rows (samples/dir/VT__membership.html)"),
    "VA": dict(school="n/a", ad_name="n/a", ad_email="n/a", coach_name="n/a", coach_email="n/a", tf_xc="n/a",
               why="robots.txt `User-agent: * Disallow: /` -> not fetched"),
    "WA": dict(school="yes", ad_name="no", ad_email="unknown", coach_name="no", coach_email="no", tf_xc="partial",
               why="wiaa.finalforms.com/state_schools: paginated (51 pages) school records with address/city/ZIP/county/phone/website, league, classifications, sport lists and one Cloudflare-protected 'Contact Info' email per school (15 decoded on page 1) - the email's role is not labelled (samples/dir/WA__state_schools.html)"),
    "WV": dict(school="unknown", ad_name="no", ad_email="no", coach_name="no", coach_email="no", tf_xc="unknown",
               why="wvssac school-directory page serves no school rows (1 452 visible chars) (samples/dir/WV__school-directory.html)"),
    "WI": dict(school="yes", ad_name="yes", ad_email="yes", coach_name="yes", coach_email="yes", tf_xc="yes",
               why="schools.wiaawi.org GetDirectorySchool?OrgID=<n> coach table (Sport|Name|Role|Email, CF-obfuscated, decodable) + Administration table; verified 3/3 schools live (samples/validate/WI_Abbotsford_orgID_1_coach_AD.html)"),
    "WY": dict(school="unknown", ad_name="unknown", ad_email="unknown", coach_name="unknown", coach_email="unknown", tf_xc="unknown",
               why="whsaa.org homepage (5 501 B) serves no directory; no /schools path located (samples/assoc-home/WY__whsaa.org.html)"),
}

ARBITER_DELEGATED = {
    "KY": "khsaa.arbitersports.com/front/104219/Site",
    "OK": "ossaa.arbitersports.com",
    "MA": "ArbiterSports (linked from MIAA Quick Links)",
    "MT": "widget.arbitersports.com (arbiter-score-ticker)",
}

OUTCOME = {
    "OK": "TIER1_FETCHED",
    "PARTIAL": "TIER1_FETCHED_CONTACT_FIELDS_PARTIAL",
    "NONE": "TIER1_FETCHED_NO_CONTACT_FIELDS",
    "SPA": "TIER1_CLIENT_RENDERED_UNVERIFIED",
    "BLOCKED": "NOT_COLLECTED_BLOCKED",
    "ROBOTS": "NOT_COLLECTED_ROBOTS_DISALLOW",
}


HTTP_BLOCKED = {
    "DE": "HTTP 403 Cloudflare interstitial to this client",
    "MS": "HTTP 403 to this client",
    "ID": "Cloudflare Turnstile challenge; no directory data served",
    "CA": "HTTP 202 with empty body from cifstate.org",
}


def classify(v: dict) -> str:
    if v["school"] == "n/a":
        return OUTCOME["ROBOTS"]
    if v["school"] == "unknown":
        return OUTCOME["SPA"]
    if v["coach_name"] == "yes":
        return OUTCOME["PARTIAL"]
    if v["ad_name"] in ("yes", "partial"):
        return OUTCOME["PARTIAL"]
    return OUTCOME["NONE"]


def main() -> int:
    probe = {r["state"]: r for r in json.loads((T / "dir-probe.json").read_text())}
    home = json.loads((T / "assoc-probe-home.json").read_text())
    ev = {r["state"]: r for r in json.loads((T / "dir-evidence.json").read_text())}
    val = json.loads((T / "baseline-validation.json").read_text()) if (T / "baseline-validation.json").exists() else []
    ks = json.loads((T / "validation-ks.json").read_text()) if (T / "validation-ks.json").exists() else []
    new = json.loads((T / "validation-ne-wholesale.json").read_text()) if (T / "validation-ne-wholesale.json").exists() else []

    assoc = {}
    for r in home:
        assoc.setdefault(r["state"], r)

    j = {}
    for st, v in sorted(VERDICT.items()):
        p = probe.get(st, {})
        e = ev.get(st, {})
        a = assoc.get(st, {})
        j[st] = {
            "association": {
                "name": a.get("expected"),
                "homepage": a.get("url"),
                "http": a.get("status"),
                "bytes": a.get("bytes"),
                "title": a.get("title"),
                "robots_allow": a.get("robots_allow"),
                "robots_rule": a.get("robots_rule"),
            },
            "http_blocked_reason": HTTP_BLOCKED.get(st),
            "tier1_response_is_stub": bool(p.get("http") not in (200,) or (p.get("bytes") or 0) < 4000),
            "tier1": {
                "url": p.get("url"),
                "http": p.get("status"),
                "bytes": p.get("bytes"),
                "sample": p.get("sample"),
                "outcome": classify(v),
                "visible_chars": e.get("visible_chars"),
                "measured": {k: e.get(k) for k in
                             ("school_word", "street", "city_state_zip", "phone", "ad_title",
                              "coach_word", "sport_label", "mailto", "cf_email")} if e else None,
            },
            "fields": {k: v[k] for k in ("school", "ad_name", "ad_email", "coach_name", "coach_email", "tf_xc")},
            "evidence": v["why"],
            "delegated_directory_platform": ARBITER_DELEGATED.get(st),
            "baseline_rows_verified": [x for x in val if x.get("state") == st and x.get("agree") is True],
            "baseline_wholesale_validation": ({
                "artifact": "tools/validation-ks.json",
                "command": "python3 tools/validate_ks.py",
                "baseline_rows": len(ks),
                "schools_matched": sum(1 for x in ks if x.get("matched")),
                "agree_ad_name": sum(1 for x in ks if x.get("agree_ad_name")),
                "agree_ad_email": sum(1 for x in ks if x.get("agree_ad_email")),
                "note": ("every KS baseline row was checked against the live KSHSAA JSON directory "
                         "(26 letter pages, saved under samples/api/KS__directory_*.json); the live "
                         "directory holds 746 schools, so the baseline's 526 is a subset of the enumerable universe"),
            } if st == "KS" and ks else ({
                "artifact": "tools/validation-ne-wholesale.json",
                "command": "python3 tools/validate_ne.py",
                "baseline_rows": len(new),
                "distinct_schools": len({x["school"] for x in new}),
                "school_pages_over_2kb": len({x["school"] for x in new if x["page_bytes"] > 2000}),
                "coach_rows": sum(1 for x in new if x["baseline_coach"]),
                "agree_coach": sum(1 for x in new if x["agree_coach"]),
                "ad_rows": sum(1 for x in new if x["baseline_ad"]),
                "agree_ad": sum(1 for x in new if x["agree_ad"]),
                "note": ("every NE baseline row was checked against its school's live NSAA export page "
                         "(311 POSTs, one per distinct school, saved under samples/validate/ne/); NE "
                         "publishes no email field, so only the coach and AD NAMES are comparable"),
            } if st == "NE" and new else None)),
            "baseline_rows_refused": [x for x in val if x.get("state") == st and x.get("status") == "refused"],
        }
    schema = {
        "generated_utc": __import__("time").strftime("%Y-%m-%dT%H:%M:%SZ", __import__("time").gmtime()),
        "jurisdictions": j,
        "counts": {
            "jurisdictions": len(j),
            "tier1_collected": sum(1 for x in j.values() if x["tier1"]["outcome"].startswith("TIER1_FETCHED")),
            "robots_disallow": sum(1 for x in j.values() if x["tier1"]["outcome"] == OUTCOME["ROBOTS"]),
            "blocked": sum(1 for x in j.values() if x["tier1"]["outcome"] == OUTCOME["BLOCKED"]),
                    "unverified_client_rendered": sum(1 for x in j.values() if x["tier1"]["outcome"] == OUTCOME["SPA"]),
            "http_blocked_to_this_client": sum(1 for x in j.values() if x.get("http_blocked_reason")),
            "coach_name_available_tier1": sum(1 for x in j.values() if x["fields"]["coach_name"] == "yes"),
            "coach_email_available_tier1": sum(1 for x in j.values() if x["fields"]["coach_email"] == "yes"),
            "ad_name_available_tier1": sum(1 for x in j.values() if x["fields"]["ad_name"] in ("yes", "partial")),
            "ad_email_available_tier1": sum(1 for x in j.values() if x["fields"]["ad_email"] in ("yes", "partial")),
            "identity_only": sum(1 for x in j.values() if x["tier1"]["outcome"] == OUTCOME["NONE"]),
        },
    }
    (LANE / "coverage.json").write_text(json.dumps(schema, indent=1, sort_keys=True), encoding="utf-8")
    print(json.dumps(schema["counts"], indent=1))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
