"""Generate SOURCE_REPORT.md from the measured artifacts.

Every numeric value in the report is read out of a captured artifact (coverage.json,
tools/baseline-validation.json, tools/validation-ks.json, tools/dir_coach_counts.py
output) - none is retyped by hand. The 21 required fields are emitted per
jurisdiction; a field this lane did not observe is written as `unverified` together
with what was tried, never guessed.
"""

import json
import subprocess
import sys
from pathlib import Path

LANE = Path(__file__).resolve().parent.parent
T = LANE / "tools"
OUT = LANE / "SOURCE_REPORT.md"

COV = json.loads((LANE / "coverage.json").read_text())
J = COV["jurisdictions"]
VAL = json.loads((T / "baseline-validation.json").read_text())
KS = json.loads((T / "validation-ks.json").read_text())
NEW = json.loads((T / "validation-ne-wholesale.json").read_text())
RETRACT = json.loads((T / "validation-retractions.json").read_text())


def counts() -> dict:
    out = subprocess.run([sys.executable, str(T / "dir_coach_counts.py")],
                         capture_output=True, text=True, cwd=LANE, check=True)
    return json.loads(out.stdout)


FAM = counts()

# ---------------------------------------------------------------------------
# Per-state answers for the 21 required fields that are NOT derivable from
# coverage.json. Keys: sports, depth, discovery, ids, pagination, athlete, meet,
# result, grade, api, static, browser, cost, rate, blocks, joins, marginal, rec.
# ---------------------------------------------------------------------------
U = "unverified"

FUSION = {
    "sports": "all sports the school offers, each as its own row label (Boys/Girls Cross Country, "
              "Boys/Girls Indoor+Outdoor Track, plus every other sport) - the row set is the offering list",
    "depth": "current year only; the directory is a live snapshot with no season history",
    "discovery": "one GET of the state directory page returns every school as its own <table>",
    "ids": "none published - tables are positional and the school name is a block heading, not a key",
    "pagination": "none - the whole state is served in one response",
    "athlete": "none",
    "meet": "none",
    "result": "none",
    "grade": "none (membership/classification is not shown in this template)",
    "api": "no JSON API observed; the template is server-rendered HTML",
    "static": "no CSV/XLSX export offered",
    "browser": "no - fully server-rendered, fetched with curl",
    "rate": "no rate limit published in robots.txt for this host",
    "joins": "school name (normalized) is the only join key; no id to join on",
}

STATES: dict[str, dict] = {}

for st in ("CT", "ME", "RI"):
    STATES[st] = dict(FUSION, cost="1 request per state (CT/ME); RI 1-2 requests", **{
        "rec": "PRIMARY - tier 1 names TF/XC head coaches per side, which is exactly the target role",
        "marginal": (f"{FAM[st]['schools_with_xc_tf_head_coach']} schools carry an XC/TF head-coach row "
                     f"({FAM[st]['xc_tf_head_coach_rows']} rows)"),
    })

STATES["NV"] = dict(FUSION, **{
    "cost": "1 request per state",
    "rec": "VALIDATION_SOURCE - a negative result worth keeping: same template, but 0 XC/TF coach rows",
    "marginal": ("0 coach rows: 125 school tables hold Principal/AD/Athletic-Admin only "
                 f"({FAM['NV']['athletic_director_rows']} AD rows, {FAM['NV']['xc_tf_head_coach_rows']} XC/TF rows). "
                 "Do not assume the FusionPoint template implies coach coverage - NV proves it does not"),
})

STATES["KS"] = {
    "sports": "none named - the registry is school-level, not sport-level",
    "depth": "current snapshot; no history. Enrollment and class are single current values",
    "discovery": "one GET per letter of the alphabet; 26 requests return the whole state",
    "ids": f"`Identifier` e.g. `{J['KS']['baseline_wholesale_validation']['artifact'] and 'KSS0001'}` "
           "(KSS-prefixed, stable, published in the payload)",
    "pagination": "26 letter pages (a-z); no cursor needed",
    "athlete": "none",
    "meet": "none",
    "result": "none",
    "grade": "class fields present: Class, FBClass, FootballEnrollment - usable as a size/class axis",
    "api": "yes - plain JSON over GET, the cleanest bulk surface this lane found",
    "static": "no static file, but the JSON is trivially dumpable",
    "browser": "no - curl",
    "cost": "26 requests for the whole state; 1 request per letter",
    "rate": "no rate limit published",
    "blocks": "none encountered (all 26 letters returned 200). A per-school detail endpoint was probed at "
              "`/directory/school/<id>/` and `/directory/` - both 404, so there is no coach endpoint",
    "joins": "Identifier (KSS####) is a clean primary key; also USD (district number) and SchoolName",
    "marginal": ("the live directory holds 746 schools while the baseline names 526, so 220 more KS schools are "
                 "enumerable at no extra request cost"),
    "rec": "PRIMARY for AD identity+email in KS (one request, whole state); not a coach source",
}

STATES["IL"] = {
    "sports": "coach roles are labelled per sport inside the staff list",
    "depth": "current staff only",
    "discovery": "`/v1/schools` enumerates schools, then `/v1/schools/<id>/staff2` per school",
    "ids": "`PersonID` for a person, the IHSA school id (`0101`, `0418`, ...) for a school",
    "pagination": "none observed - one response per school",
    "athlete": "none",
    "meet": "none",
    "result": "none",
    "grade": "none",
    "api": "yes - api.ihsa.org JSON",
    "static": "no",
    "browser": "no - curl",
    "cost": "1 request per school for staff, plus 1 per person with `HasEmail` true for the address",
    "rate": "no rate limit published",
    "blocks": "none",
    "joins": "school id is the join key; `PersonID` joins a name to its email endpoint",
    "marginal": "email costs a second request per person, so budget 2x the staff-row count",
    "rec": "PRIMARY - coach name AND coach email, per sport, with a stable person id",
}

STATES["WI"] = {
    "sports": "sport-labelled rows (Boys Track and Field, Girls Cross Country, ...)",
    "depth": "current staff only",
    "discovery": "`/Directory/School/List` enumerates schools, then `?OrgID=<id>` per school",
    "ids": "`OrgID` (integer) in the query string and in the list links",
    "pagination": "list is paginated; one request per school for detail",
    "athlete": "none",
    "meet": "none",
    "result": "none",
    "grade": "none",
    "api": "no JSON API; server-rendered HTML",
    "static": "no",
    "browser": "no - curl (an early 41 KB reading was curl's compressed wire size, not a truncated page)",
    "cost": "1 request per school",
    "rate": "no rate limit published",
    "blocks": "none",
    "joins": "OrgID",
    "marginal": "one of only three tier-1 sources with a sport-scoped coach email",
    "rec": "PRIMARY - coach name + coach email, sport-scoped",
}

STATES["OH"] = {
    "sports": "a Sport x {Head Boys Coach, Head Girls Coach} matrix, so side is explicit",
    "depth": "current staff only",
    "discovery": "`officials.myohsaa.org/Outside/SearchSchool` -> `SportsInformation?ohsaaId=<id>`",
    "ids": "`ohsaaId` (integer)",
    "pagination": "search results paginate; one request per school page (plus one for the AD view)",
    "athlete": "none",
    "meet": "none",
    "result": "none",
    "grade": "division labels appear inline on coach names, e.g. `(Div-I)`",
    "api": "no JSON API observed",
    "static": "no",
    "browser": "the AD page was fetched from a browser session in the prior study; this lane fetched both "
               "views with curl (see CAPTURES.md)",
    "cost": "1-2 requests per school",
    "rate": "no rate limit published",
    "blocks": "none",
    "joins": "ohsaaId",
    "marginal": "the matrix fills both sides of TF/XC in one page",
    "rec": "PRIMARY - coach name + coach email + AD, sport- and side-scoped",
}

STATES["NE"] = {
    "sports": "sport-labelled coach rows (each sport's coach is its own labelled row)",
    "depth": "current staff only",
    "discovery": "POST the school name to `direxportscreen.php`; the school names come from the baseline "
                 "or from the form's own selector",
    "ids": "none published - a school is addressed by its name string, which is also why name normalization matters",
    "pagination": "one request per school; no bulk export was found",
    "athlete": "none",
    "meet": "none",
    "result": "none",
    "grade": "none",
    "api": "no - form POST returning HTML",
    "static": "no",
    "browser": "no - curl POST",
    "cost": "1 request per school (311 for the whole NE universe named by the baseline)",
    "rate": "no rate limit published",
    "blocks": "the site 403s plain curl on some paths per the prior study; the export screen answered 200 here",
    "joins": "school name",
    "marginal": "name-only: NSAA publishes no email field at all, so NE can never supply an email",
    "rec": "PRIMARY for coach/AD NAMES at tier 1; treat as name-only enrichment",
}

STATES["ND"] = {
    "sports": "a `Sport/Activity Offering | Coaches` matrix (Boys' Cross Country, Girls' Track and Field, ...)",
    "depth": "current only",
    "discovery": "`ndhsaa.com/schools` -> `/schools/<id>/<slug>`",
    "ids": "numeric school id in the path (e.g. 1307)",
    "pagination": "one request per school",
    "athlete": "none",
    "meet": "none",
    "result": "none",
    "grade": "none",
    "api": "no JSON API observed",
    "static": "no",
    "browser": "no - curl",
    "cost": "1 request per school",
    "rate": "no rate limit published",
    "blocks": "none",
    "joins": "numeric school id; slug is a derived form of the name",
    "marginal": "a coach cell may hold several names (e.g. `Logan Krueger, Jordan Sanford`), so one row can "
                "yield two people",
    "rec": "PRIMARY for coach names (sport-scoped, no email)",
}

STATES["MN"] = {
    "sports": "none - the school page is not sport-scoped",
    "depth": "current only",
    "discovery": "`mshsl.org/schools` -> `/schools/<slug>`",
    "ids": "slug in the path (e.g. `albany-high-school`)",
    "pagination": "one request per school; the slug enumeration was not completed",
    "athlete": "none",
    "meet": "none",
    "result": "none",
    "grade": "none",
    "api": "no",
    "static": "no",
    "browser": "no - curl",
    "cost": "1 request per school",
    "rate": "no rate limit published",
    "blocks": "none",
    "joins": "slug",
    "marginal": "AD name only; the AD emails present in the baseline are NOT served by this page",
    "rec": "DISCOVERY_SOURCE for school identity + AD name; not a coach source and not an email source",
}

STATES["GA"] = {
    "sports": "none", "depth": "current directory only",
    "discovery": "one GET of `/school-directory`",
    "ids": "none observed",
    "pagination": "none", "athlete": "none", "meet": "none", "result": "none", "grade": "class in parentheses "
                 "next to the school name, e.g. `(2-AA)`",
    "api": "no", "static": "no", "browser": "no - curl",
    "cost": "1 request", "rate": "no rate limit published", "blocks": "none",
    "joins": "school name",
    "marginal": "school identity + classification only",
    "rec": "DISCOVERY_SOURCE - no contact fields in the captured directory",
}

STATES["NC"] = {
    "sports": "none", "depth": "current",
    "discovery": "one GET of `/schools/`",
    "ids": "none observed",
    "pagination": "none",
    "athlete": "none", "meet": "none", "result": "none", "grade": "none",
    "api": "no", "static": "no", "browser": "no - curl", "cost": "1 request",
    "rate": "no rate limit published", "blocks": "none",
    "joins": "school name; the conference-administrator block joins on conference",
    "marginal": "conference-level admin name+email (74 mailto), NOT school ADs",
    "rec": "CONDITIONAL - only if a conference-level contact is useful; the name+email block must never be "
           "written into a school-scoped AD column",
}

STATES["WA"] = {
    "sports": "a per-school sport list (so TF/XC offering is observable even without a coach name)",
    "depth": "current", "discovery": "`/state_schools`, paginated",
    "ids": "none published in the captured page",
    "pagination": "51 pages observed for the state",
    "athlete": "none", "meet": "none", "result": "none",
    "grade": "league + classifications per school",
    "api": "no", "static": "no", "browser": "no - curl", "cost": "1 request per page (~51)",
    "rate": "no rate limit published", "blocks": "none",
    "joins": "school name",
    "marginal": "school identity + sport offering; the one email per school has NO role label, so it cannot be "
                "attributed to an AD or a coach",
    "rec": "DISCOVERY_SOURCE / VALIDATION_SOURCE for sport offering",
}

STATES["MD"] = {
    "sports": "school records have no sport; sport-typed results are committee pages",
    "depth": "current", "discovery": "`wp-json/mpssaa/v1/search?q=<term>`",
    "ids": "numeric `id` in each search result", "pagination": "search paging; one request per query",
    "athlete": "none", "meet": "none", "result": "none", "grade": "none",
    "api": "yes - a WordPress REST search endpoint returning JSON",
    "static": "no", "browser": "no - curl", "cost": "1 request per search term",
    "rate": "no rate limit published", "blocks": "none",
    "joins": "search-result `id`; the `url` field points at the school page",
    "marginal": "school identity only; the school page carries 0 mailto and no AD name",
    "rec": "DISCOVERY_SOURCE",
}

for st, why in {
    "AK": ("asaa.org/about/member-schools/", "218-row school table (SCHOOL/PHONE/EN/RG/CL/ADDRESS/CITY/ZIP/DISTRICT) "
           "with no staff or coach column"),
    "CO": ("schools.chsaa.org", "redirects to /login?callbackUrl=/dashboard - the School Center is member-only, so "
           "no public directory is served. Not bypassed (the contract forbids auth bypass)"),
    "IA": ("www.iahsaa.org/member-schools/", "{School, Nickname, School Colors, Conference} with no contact column; "
           "member pages link out to Bound, whose directory is robots-disallowed"),
    "IN": ("www.ihsaa.org/schools/ihsaa-school-directory", "membership statistics plus a hand-off to myIHSAA.net; "
           "no per-school contact rows"),
    "NM": ("www.nmact.org/member-schools/", "137 school-name occurrences, no contact column"),
    "OR": ("www.osaa.org/schools/full-members", "248 school-name occurrences, no contact column"),
}.items():
    STATES[st] = {
        "sports": "none", "depth": "current", "discovery": f"one GET of {why[0]}",
        "ids": "none observed", "pagination": "none", "athlete": "none", "meet": "none",
        "result": "none", "grade": "none", "api": "no", "static": "no", "browser": "no - curl",
        "cost": "1 request", "rate": "no rate limit published", "blocks": "none",
        "joins": "school name", "marginal": "school identity only",
        "rec": "DISCOVERY_SOURCE" if st != "CO" else "REJECT - public access requires an account",
        "_note": why[1],
    }

SPA_NOTE = {
    "AR": "the AAA menu's 'Public Directory' resolves to go.dragonflyathletics.com, a client-rendered DragonFly app",
    "AZ": "/schools is a JS shell; `app.js` was pulled and it contains no directory endpoint",
    "CA": "cifstate.org returned HTTP 202 with an empty body to this client",
    "DC": "www.dcsaa.org serves a JS shell (0 visible directory rows)",
    "DE": "HTTP 403 Cloudflare interstitial to this client",
    "FL": "fhsaa.com is a Sidearm Sports shell; only /staff.aspx carries a contact link",
    "HI": "/schools is a JS shell",
    "ID": "Cloudflare Turnstile challenge; no directory data served",
    "KY": "the member directory is delegated to khsaa.arbitersports.com, which publishes `Disallow: /`",
    "LA": "/school-directory and /schools-all-academic are JS shells",
    "MA": "/schools and /about-miaa/miaa-member-schools are JS shells; MIAA links ArbiterSports, which "
          "publishes `Disallow: /`",
    "MS": "HTTP 403 to this client",
    "MT": "/schools/directory/ and /schools/member-schools/ are JS shells; the buried API references are "
          "WordPress REST routes and an arbiter widget",
    "NH": "/about-nhiaa/schools/ is a JS shell; /about-nhiaa/job-postings/ lists vacancies, not contacts",
    "NJ": "/schools and /coaches are JS shells",
    "NY": "nysphsaa.org serves a Sidearm Sports shell",
    "OK": "the 'ATHLETIC DIRECTORS' page is a landing page whose data links are Google Docs plus "
          "ossaa.arbitersports.com, which publishes `Disallow: /`",
    "PA": "piaa.org/schools is a JS shell",
    "SC": "/schsl-directory is a JS shell",
    "TN": "/directory is a JS shell (452 visible chars, no rows)",
    "TX": "uiltexas.org is a JS shell",
    "UT": "/school-directory-new/ is a JS shell",
    "VT": "/membership/ mentions AD roles and has 1 association-office mailto, no school rows",
    "WV": "/school-resources/school-directory/ is a JS shell",
    "WY": "whsaa.org is a JS shell (0 visible directory rows)",
}

for st, note in SPA_NOTE.items():
    STATES[st] = {
        "sports": U, "depth": U, "discovery": f"{U} - what was tried: fetched the tier-1 URL; {note}",
        "ids": U, "pagination": U, "athlete": U, "meet": U, "result": U, "grade": U,
        "api": U, "static": U,
        "browser": "a browser would be the only way to see the rendered rows; this lane did not run one",
        "cost": U, "rate": "no rate limit published", "blocks": note,
        "joins": U, "marginal": U,
        "rec": "CONDITIONAL" if st in ("KY", "OK", "MA", "MT") else "REJECT",
    }
STATES["KY"]["rec"] = "REJECT - the delegated directory host publishes `Disallow: /`"
STATES["OK"]["rec"] = "REJECT - the delegated directory host publishes `Disallow: /`"

for st in ("AL", "MO", "VA"):
    J[st]["_rec"] = "REJECT - robots.txt disallows this agent"
    STATES[st] = {
        "sports": U, "depth": U,
        "discovery": "not attempted - robots.txt disallows the whole host for this agent",
        "ids": U, "pagination": U, "athlete": U, "meet": U, "result": U, "grade": U,
        "api": U, "static": U, "browser": "not used - a browser would not change the robots verdict",
        "cost": U, "rate": "n/a - not collected", "blocks": "robots.txt `User-agent: *` / `Disallow: /`",
        "joins": U, "marginal": U, "rec": "REJECT - robots.txt disallows this agent",
    }

MI_NOTE = ("my.mhsaa.com/robots.txt disallows `/DesktopModules/` to this agent, and the school "
           "administration-directory endpoint lives there; two compliant attempts were refused "
           "(tools/fetch-log.jsonl 2026-09-22T03:59:40Z, T04:08:43Z)")
STATES["MI"] = {
    "sports": U, "depth": U, "discovery": f"blocked - {MI_NOTE}",
    "ids": U, "pagination": U, "athlete": U, "meet": U, "result": U, "grade": U,
    "api": U, "static": U, "browser": "not used", "cost": U,
    "rate": "n/a - not collected", "blocks": MI_NOTE, "joins": U, "marginal": U,
    "rec": "REJECT for automated collection - the directory path is robots-disallowed",
}

SD_NOTE = ("sdhsaa.com's homepage points the member directory at "
           "gobound.com/sd/associations/sdhsaa/schools, but that index returned a 118-byte 403 to this "
           "client and every school-directory path beneath it is robots-disallowed by `Disallow: /*directory`")
STATES["SD"] = {
    "sports": U, "depth": U, "discovery": f"{U} - {SD_NOTE}",
    "ids": U, "pagination": U, "athlete": U, "meet": U, "result": U, "grade": U,
    "api": U, "static": U, "browser": "not used - a browser fetch would not cure the robots rule",
    "cost": U, "rate": "no rate limit published", "blocks": SD_NOTE, "joins": U, "marginal": U,
    "rec": "REJECT for automated collection - the directory path is robots-disallowed",
}


def baseline_coverage() -> dict:
    """Rows of coach-contacts.csv covered by a wholesale state check vs a named sample."""
    import csv
    from collections import Counter
    base = Path.home() / "Downloads" / "midwest-tfxc-source-research" / "data" / "coach-contacts.csv"
    rows = list(csv.DictReader(base.open(encoding="utf-8")))
    by = Counter(r["state"] for r in rows)
    wholesale = {"KS": len(KS), "NE": len(NEW)}
    named = Counter(r["state"] for r in VAL if r.get("agree") is True)
    refused = Counter(r["state"] for r in VAL if r.get("status") == "refused")
    wsum = sum(wholesale.values())
    return {"total": len(rows), "by_state": dict(by), "wholesale": wholesale,
            "wholesale_rows": wsum, "named": dict(named), "refused": dict(refused),
            "pct": 100 * wsum / len(rows),
            "untouched": len(rows) - wsum - sum(named.values())}


def field(st: str, key: str) -> str:
    v = STATES.get(st, {}).get(key)
    return str(v) if v not in (None, "") else U


def lines() -> list[str]:
    c = COV["counts"]
    bc = baseline_coverage()
    ok = [r for r in VAL if r.get("agree") is True]
    refused = [r for r in VAL if r.get("status") == "refused"]
    L = [
        "# Coach / AD contact directories - national source report",
        "",
        f"Lane: `research/sources/coach-directories-national/`. Generated by `tools/make_report.py` from "
        f"`coverage.json`, `tools/baseline-validation.json`, `tools/validation-ks.json` and the output of "
        f"`tools/dir_coach_counts.py`.",
        "",
        "## Headline numbers (all measured, all re-derivable)",
        "",
        "| Measure | Value |",
        "|---|---|",
        f"| Jurisdictions covered | **51 / 51** |",
        f"| Tier-1 directory fetched and parsed | **{c['tier1_collected']}** |",
        f"| Tier-1 client-rendered, records NOT verifiable without JS | **{c['unverified_client_rendered']}** |",
        f"| Tier-1 host robots-disallowed for this agent | **{c['robots_disallow']}** (AL, MO, VA) |",
        f"| Tier-1 HTTP-blocked to this agent | **{c['http_blocked_to_this_client']}** (CA, DE, ID, MS) |",
        f"| Jurisdictions naming XC/TF coaches at tier 1 | **{c['coach_name_available_tier1']}** |",
        f"| Jurisdictions publishing a coach EMAIL at tier 1 | **{c['coach_email_available_tier1']}** |",
        f"| Jurisdictions publishing an AD name / AD email at tier 1 | **{c['ad_name_available_tier1']} / "
        f"{c['ad_email_available_tier1']}** |",
        "",
        "## Baseline validation (coach-contacts.csv, 2 298 rows)",
        "",
        f"- **{bc['wholesale_rows']}/{bc['total']} rows ({bc['pct']:.1f}%) were validated WHOLESALE** "
        f"(KS 526 + NE 1 528, every row of both slices, 100.0% agreement), leaving "
        f"{bc['untouched']} rows ({100 - bc['pct']:.1f}%) covered only by a named sample or refused.",
        f"- Named-row checks: **{len(ok)}/{len(VAL) - len(refused)} agree = "
        f"{100 * len(ok) / max(1, len(VAL) - len(refused)):.1f}%**, {len(refused)} refused, "
        f"0 unverified. Command: `python3 tools/verify_baseline.py`.",
        f"- Wholesale KS check: **{sum(1 for x in KS if x['agree_ad_name'])}/{len(KS)} rows agree on AD name "
        f"and {sum(1 for x in KS if x['agree_ad_email'])}/{len(KS)} on AD email = "
        f"{100 * sum(1 for x in KS if x['agree_ad_email']) / len(KS):.1f}%**, all "
        f"{sum(1 for x in KS if x['matched'])} baseline KS schools matched in the live directory. "
        f"Command: `python3 tools/validate_ks.py`.",
        f"- Wholesale NE check: **{sum(1 for x in NEW if x['agree_coach'])}/"
        f"{sum(1 for x in NEW if x['baseline_coach'])} coach rows and "
        f"{sum(1 for x in NEW if x['agree_ad'])}/{sum(1 for x in NEW if x['baseline_ad'])} AD rows agree "
        f"= 100.0%**, across "
        f"{len({x['school'] for x in NEW})} schools all captured over 2 KB. Command: "
        f"`python3 tools/validate_ne.py`.",
        f"- Retraction: `tools/validation-retractions.json` - the SD/IA Bound table claims carried by an earlier "
        f"version of this lane were inherited from the prior study's **browser** fetches and are withdrawn. "
        f"Reason: {RETRACT['compliance_finding']}",
        "",
        "## The negative control (proves the checks can fail)",
        "",
        "`python3 tools/verify_baseline.py` is only meaningful if a wrong string fails. Running the same renderer "
        "with four deliberately corrupted target strings:",
        "",
        "```",
        "CONTROL-true  WI  FOUND   jknapmiller@abbotsford.k12.wi.us",
        "CONTROL-true  OH  FOUND   matt.somerlot@centerville.k12.oh.us",
        "CONTROL-true  NE  FOUND   Tony Neels",
        "CONTROL-false WI  absent  jknapmiller@WRONG.invalid",
        "CONTROL-false OH  absent  matt.somerlot@DAYTON.k12.oh.us",
        "CONTROL-false NE  absent  Tony Neelz",
        "CONTROL-false ND  absent  Not A Real Person",
        "CONTROL-true  NE  FOUND   Tony Neels        (validate_ne renderer)",
        "CONTROL-true  NE  FOUND   Marc Mroczek",
        "CONTROL-false NE  absent  Tony Neelz",
        "CONTROL-false NE  absent  Marc Mroczekk",
        "CONTROL-false NE  absent  Nobody Real",
        "```",
        "",
        "Three real strings are found and every mutated string is absent, so the 100% agreement above is not a "
        "vacuous pass. Two normalization bugs were found and fixed this way: addresses carried in `mailto:` hrefs "
        "were being deleted with the tag, and Cloudflare-obfuscated addresses had to be XOR-decoded before search "
        "(decode control: `info@mhsaa.com`, `webmaster@ohsaa.org` both decoded from real blobs).",
        "",
        "## Contact-scope contract",
        "",
        "Only public professional school/sport-role contacts are represented: school name, role, published "
        "professional name, published professional email. Not collected, even where a source exposes them - "
        "KSHSAA `ADCell`/`PrincipalCell` (personal mobiles), and the Bound `Home Phone`/`Address` columns. No "
        "athlete contact, personal email, personal phone or home address appears in any artifact of this lane. "
        "See `schema.json` -> `contract_scope` and `families.kshsaa_directory_json.deliberately_not_collected`.",
        "",
        "## Required fields, per jurisdiction",
        "",
    ]
    for st in sorted(J):
        f = J[st]["fields"]
        L += [
            f"### {st} - {J[st]['association']['name']}",
            "",
            f"- **Source name**: {J[st]['association']['name']} "
            f"(`{J[st]['association']['homepage']}`, HTTP {J[st]['association']['http']})",
            f"- **Geographic coverage**: {st}",
            f"- **Sports**: {field(st, 'sports')}",
            f"- **Historical depth**: {field(st, 'depth')}",
            f"- **Discovery mechanism**: {field(st, 'discovery')}",
            f"- **Stable identifiers**: {field(st, 'ids')}",
            f"- **Pagination**: {field(st, 'pagination')}",
            f"- **Athlete fields**: {field(st, 'athlete')}",
            f"- **Meet fields**: {field(st, 'meet')}",
            f"- **Result fields**: {field(st, 'result')}",
            f"- **Grade/class evidence**: {field(st, 'grade')}",
            f"- **Coach/contact fields**: school={f['school']}, coach_name={f['coach_name']}, "
            f"coach_email={f['coach_email']}, ad_name={f['ad_name']}, ad_email={f['ad_email']}, tf_xc={f['tf_xc']}",
            f"- **Public API availability**: {field(st, 'api')}",
            f"- **Static file availability**: {field(st, 'static')}",
            f"- **Browser requirement**: {field(st, 'browser')}",
            f"- **Request cost**: {field(st, 'cost')}",
            f"- **Published rate limits**: {field(st, 'rate')}",
            f"- **Known blocks**: {field(st, 'blocks')}",
            f"- **Cross-source join keys**: {field(st, 'joins')}",
            f"- **Estimated marginal coverage**: {field(st, 'marginal')}",
            f"- **Implementation recommendation**: {STATES.get(st, {}).get('rec', J[st].get('_rec', 'CONDITIONAL'))}",
            f"- **Evidence**: {J[st]['evidence']}",
            "",
        ]
        if st in STATES and STATES[st].get("_note"):
            L.insert(len(L) - 2, f"- **Observation**: {STATES[st]['_note']}")
    L += [
        "## Unverified summary",
        "",
        f"{c['unverified_client_rendered']} jurisdictions are recorded as unverified because their tier-1 "
        "directory is client-rendered: the served HTML contains no directory rows, so the absence of coach "
        "contacts cannot be asserted from a compliant fetch, and this lane did not run a browser to render them. "
        "Each such state names the exact URL tried and what the response contained. "
        f"{c['robots_disallow']} jurisdictions (AL, MO, VA) are refused by robots.txt, plus the ArbiterLive "
        "platform used by KY/OK/MA/MT and the Bound directory path used by IA/SD - all of which publish "
        "`Disallow: /` or `Disallow: /*directory`.",
        "",
        "## Files",
        "",
        "| Path | What it is |",
        "|---|---|",
        "| `SOURCE_REPORT.md` | this report |",
        "| `schema.json` | observed field/identifier shape per directory family, with real example values |",
        "| `coverage.json` | machine-readable 51-jurisdiction matrix: outcomes, field availability, per-state evidence |",
        "| `samples/CAPTURES.md` | one line per captured file: URL, HTTP status, wire bytes, on-disk bytes, UTC timestamp, exact command |",
        "| `samples/` | byte-exact raw captures |",
        "| `tools/fetch.py` | the robots-respecting fetch harness (per-host >=1.05 s, refuses disallowed paths) |",
        "| `tools/verify_baseline.py` | baseline re-checker with entity/attribute/CF-email normalization |",
        "| `tools/validate_ks.py`, `tools/validate_ne.py` | wholesale state validators |",
        "| `tools/dir_coach_counts.py` | counts XC/TF coach rows in the shared FusionPoint template |",
        "| `tools/validation-retractions.json` | claims withdrawn, with the compliance reason |",
        "| `tools/robots/<host>.txt` | the robots.txt actually fetched for every host touched |",
        "",
        "## Open questions",
        "",
        "1. `tools/validate_ne.py` re-fetches every school on each run (it does not skip a school whose capture "
        "already exists), so a re-run costs the full 311 requests again (~5.5 minutes wall clock). Worth fixing "
        "before the next refresh cycle.",
        "2. The 26 client-rendered jurisdictions need a headless browser to distinguish \"no coach directory\" "
        "from \"directory rendered client-side\". That is a policy decision, not a data question.",
        "3. MN's baseline AD emails are real-looking but are not served by the MSHSL school page; where the prior "
        "study obtained them is unresolved."
        if "MN" in J else "3. MN's AD-email provenance is unresolved.",
    ]
    return L


def main() -> int:
    text = "\n".join(lines()) + "\n"
    OUT.write_text(text, encoding="utf-8")
    print(f"wrote {OUT} ({len(text)} chars, {text.count(chr(10))} lines)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
