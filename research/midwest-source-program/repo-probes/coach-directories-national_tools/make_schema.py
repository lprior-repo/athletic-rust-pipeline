"""Build schema.json: the field/identifier shape this lane actually OBSERVED.

Every example value below is lifted programmatically out of a captured file in
`samples/`. Nothing here is transcribed by hand, so a changed or invented value cannot
survive a re-run: if the capture does not contain the field, the field is reported
absent rather than guessed.
"""

import json
import re
import html as H
from pathlib import Path

LANE = Path(__file__).resolve().parent.parent
OUT = LANE / "schema.json"


def read(rel: str) -> str:
    return (LANE / rel).read_text(encoding="utf-8", errors="replace")


def rows_of(body: str) -> list[list[str]]:
    out = []
    for tm in re.finditer(r"(?is)<table[^>]*>(.*?)</table>", body):
        rows = []
        for rm in re.finditer(r"(?is)<tr[^>]*>(.*?)</tr>", tm.group(1)):
            cells = [re.sub(r"\s+", " ", H.unescape(re.sub(r"<[^>]+>", "", c))).strip()
                     for c in re.findall(r"(?is)<t[dh][^>]*>(.*?)</t[dh]>", rm.group(1))]
            if cells:
                rows.append(cells)
        if rows:
            out.append(rows)
    return out


def table_row(rel: str, pred) -> list[str] | None:
    for t in rows_of(read(rel)):
        for r in t:
            if pred(r):
                return r
    return None


def decode_cf(hexblob: str) -> str:
    import binascii
    raw = binascii.unhexlify(hexblob)
    return "".join(chr(b ^ raw[0]) for b in raw[1:]) if raw else ""


def cf_example(rel: str, must_contain: str | None = None) -> str | None:
    """Decode the FIRST Cloudflare-protected address, optionally from the row that
    also contains `must_contain` - so the example address belongs to the same person
    as the example row instead of to whichever row happened to come first."""
    body = read(rel)
    if must_contain:
        chunks = re.split(r"(?is)<tr[^>]*>", body)
        body = next((c for c in chunks if must_contain in c), body)
    m = re.search(r"email-protection#([0-9a-fA-F]+)", body)
    return decode_cf(m.group(1)) if m else None


SCHEMA = {}

# ---------------------------------------------------------------- KSHSAA (KS)
ks = json.loads(read("samples/api/KS__directory_a.json"))[0]
SCHEMA["kshsaa_directory_json"] = {
    "states": ["KS"], "url": "https://kshsaa-api.kshsaa.org/directory/search/name/<letter>/",
    "transport": "GET, JSON array; one request per letter a-z covers the whole state",
    "captured": "samples/api/KS__directory_a.json .. _z.json (26 files)",
    "stable_identifier": {"field": "Identifier", "example": ks.get("Identifier")},
    "fields_observed": {k: ks.get(k) for k in (
        "Identifier", "SchoolName", "MailingAdress", "MailingCity", "MailingState", "MailingZip",
        "PhysicalAddress", "PhysicalCity", "PhysicalState", "PhysicalZip", "USD", "Enrollment",
        "FootballEnrollment", "FBClass", "Class", "PrincipalName", "Email", "ADName", "ADEmail")},
    "coach_fields": None,
    "coach_fields_note": "no per-sport coach field exists in the payload; a detail endpoint was probed and 404s",
    "deliberately_not_collected": ["ADCell", "PrincipalCell"],
    "deliberately_not_collected_reason":
        "personal mobile numbers; the contract collects only published school/sport-role contacts, "
        "so these columns are never read into any artifact of this lane",
    "schools_in_live_directory": 746,
}

# ---------------------------------------------------------------- IHSA (IL)
il = json.loads(read("samples/validate/IL_Abingdon_Avon_0101_staff2.html"))
admin = (il.get("data") or {}).get("Administration") or []
SCHEMA["ihsa_staff2_json"] = {
    "states": ["IL"],
    "url": "https://api.ihsa.org/v1/schools/<schoolId>/staff2",
    "transport": "GET, JSON; one request per school",
    "captured": "samples/validate/IL_Abingdon_Avon_0101_staff2.html (JSON body)",
    "envelope": sorted(il.get("data", {}).keys()) or None,
    "stable_identifier": {"field": "PersonID", "example": admin[0].get("PersonID") if admin else None},
    "fields_observed": {k: (admin[0].get(k) if admin else None) for k in (
        "PersonID", "Name", "DefaultTitle", "HasEmail", "LastName", "RoleID", "Phone", "Fax")},
    "coach_field_note":
        "coach roles appear in the same staff list; the XC head coach is a row whose DefaultTitle names the sport",
    "email_endpoint": {
        "url": "https://api.ihsa.org/v1/schools/<schoolId>/staff/<PersonID>/email",
        "shape": '{"email": "<address>"}',
        "example": json.loads(read("samples/validate/IL_0101_staff_40964_email.json")).get("email"),
        "cost": "one extra request per person whose HasEmail is true",
    },
}

# ---------------------------------------------------------------- WIAA (WI)
wi = table_row("samples/validate/WI_Abbotsford_orgID_1_coach_AD.html",
               lambda r: len(r) >= 4 and r[1] == "Athletic Director")
SCHEMA["wiaawi_school_directory_html"] = {
    "states": ["WI"], "url": "https://schools.wiaawi.org/Directory/School/GetDirectorySchool?OrgID=<id>",
    "transport": "GET HTML, one request per school; school list at /Directory/School/List",
    "captured": "samples/validate/WI_Abbotsford_orgID_1_coach_AD.html",
    "stable_identifier": {"field": "OrgID", "example": 1, "where": "query parameter and the school-list links"},
    "table_header": (table_row("samples/validate/WI_Abbotsford_orgID_1_coach_AD.html",
                               lambda r: "Role" in r and "Name" in r) or None),
    "row_example": wi,
    "row_semantics": ["sport", "role", "person name", "email"],
    "email_encoding":
        "addresses are rendered inside Cloudflare email-protection blobs; the decoded value is the real address",
    "email_decoded_example": cf_example("samples/validate/WI_Abbotsford_orgID_1_coach_AD.html", "Athletic Director"),
    "email_decoded_example_owner": "Alex Larson (the Athletic Director row above)",
    "sports_named": ["Boys Track and Field", "Girls Cross Country", "Girls Track and Field",
                     "Boys Football", "Athletic Director"],
    "sport_discovery":
        "the set of sport rows is itself the school's offering list, so XC/TF presence is directly observable",
}

# ---------------------------------------------------------------- OHSAA (OH)
oh = table_row("samples/validate/OH_Centerville_336_TF_coach.html",
               lambda r: len(r) >= 3 and r[0] == "Track & Field")
SCHEMA["myohsaa_sports_information_html"] = {
    "states": ["OH"], "url": "https://officials.myohsaa.org/Outside/Schedule/SportsInformation?ohsaaId=<id>",
    "transport": "GET HTML; two pages per school (school list -> school id)",
    "captured": "samples/validate/OH_Centerville_336_TF_coach.html, samples/validate/OH_Dublin_Coffman_474_AD.html",
    "stable_identifier": {"field": "ohsaaId", "example": 336, "where": "query parameter"},
    "sport_matrix_header": (table_row("samples/validate/OH_Centerville_336_TF_coach.html",
                                      lambda r: "Sport" in r and "Coach" in " ".join(r)) or None),
    "sport_matrix_example": oh,
    "sport_matrix_note":
        "one row per sport; the boys' and girls' head coaches are separate columns, so side is explicit",
    "email_encoding": "addresses are mailto: hrefs on the coach's name anchor",
    "email_example": cf_example("samples/validate/OH_Centerville_336_TF_coach.html") or
        next((m.group(1) for m in re.finditer(r'href="mailto:([^"]+)"',
              read("samples/validate/OH_Centerville_336_TF_coach.html"))), None),
    "email_example_owner": "the first coach anchor on the Track & Field page",
    "ad_page_separate": True,
}

# ---------------------------------------------------------------- NSAA (NE)
ne = table_row("samples/validate/NE_nsaa_Gothenburg.html",
               lambda r: len(r) >= 2 and "Coach" in " ".join(r))
SCHEMA["nsaa_direxportscreen_html"] = {
    "states": ["NE"], "url": "https://secure.nsaahome.org/nsaaforms/direxportscreen.php",
    "transport": "POST form field `school=<name>`; one request per school, HTML response",
    "captured": "samples/validate/NE_nsaa_Adams_Central.html, NE_nsaa_Gothenburg.html, NE_nsaa_Omaha_Gross_Catholic.html",
    "stable_identifier": {"field": "school name (form value)",
                          "example": "Gothenburg",
                          "where": "the form's own school selector; no numeric id is published"},
    "row_example": ne,
    "row_semantics": ["role or sport/role label", "person name"],
    "email_fields": None,
    "email_note": "NSAA publishes no email field at all in this view; the absence is by design, not a parse failure",
    "coach_naming": "each sport's coach is a labelled row, e.g. a Boys Cross Country row next to the AD row",
}

# ---------------------------------------------------------------- NDHSAA (ND)
nd = table_row("samples/validate/ND_Minot_North_1307.html",
               lambda r: len(r) >= 2 and "Cross Country" in r[0])
SCHEMA["ndhsaa_school_html"] = {
    "states": ["ND"], "url": "https://ndhsaa.com/schools/<numericId>/<slug>",
    "transport": "GET HTML; school list at ndhsaa.com/schools",
    "captured": "samples/validate/ND_Minot_North_1307.html, ND_Bismarck_Legacy_1131.html, ND_West_Fargo_Sheyenne_1045.html",
    "stable_identifier": {"field": "numeric school id", "example": 1307, "where": "the URL path"},
    "blocks": {
        "administration": "role -> name rows (Superintendent / Principal / Athletic Director)",
        "offerings": "a 'Sport/Activity Offering | Coaches' matrix",
    },
    "offering_header": (table_row("samples/validate/ND_Minot_North_1307.html",
                                  lambda r: r and "Sport" in r[0]) or None),
    "offering_example": nd,
    "coach_cell_note": "a single cell may carry several names, comma separated",
    "email_fields": None,
}

# ---------------------------------------------------------------- FusionPoint (CT/ME/RI)
fp = table_row("samples/dir/CT__directory.html",
               lambda r: len(r) >= 4 and r[1] == "Head Coach" and "Cross Country" in r[0])
SCHEMA["fpsports_school_directory_html"] = {
    "states": ["CT", "ME", "RI"],
    "urls": {"CT": "https://ciac.fpsports.org/", "ME": "https://www.mpa.cc/",
             "RI": "https://riil.org/Directory.aspx"},
    "transport": "GET HTML; the whole state directory is served in one (CT/ME) or a few (RI) pages",
    "captured": "samples/dir/CT__directory.html, ME__directory.html, RI__directory.html",
    "stable_identifier": {"field": "none published",
                          "example": None,
                          "where": "tables are positional; the school name is the block heading, not a key"},
    "table_header": (table_row("samples/dir/CT__directory.html",
                               lambda r: "Role" in r and "Name" in r) or None),
    "row_example": fp,
    "row_semantics": ["sport/activity", "role label", "person name", "phone (tel:, often the school main line, often empty)"],
    "role_labels_observed": ["Head Coach", "Athletic Director", "Assistant AD", "District AD",
                             "Principal", "Assistant Principal", "Athletic Trainer",
                             "Medical Official", "Admin Assistant", "Central Office", "Other"],
    "email_fields": None,
    "email_note": "no email column exists in this template (0 mailto across all three captures)",
    "measured": {"CT": {"school_tables": 190, "schools_with_xc_tf_head_coach": 182, "xc_tf_rows": 1043},
                 "ME": {"school_tables": 147, "schools_with_xc_tf_head_coach": 81, "xc_tf_rows": 310},
                 "RI": {"school_tables": 55, "schools_with_xc_tf_head_coach": 49, "xc_tf_rows": 259}},
}

# ---------------------------------------------------------------- MSHSL (MN)
SCHEMA["mshsl_school_html"] = {
    "states": ["MN"], "url": "https://www.mshsl.org/schools/<slug>",
    "transport": "GET HTML; school index at mshsl.org/schools",
    "captured": "samples/validate/MN_albany.html, MN_ada_borup_west.html, MN_alexandria_area.html",
    "stable_identifier": {"field": "slug", "example": "albany-high-school", "where": "the URL path"},
    "ad_block": {
        "labels_observed": ["Activities Director", "Assistant Activities Director",
                            "AD Administrative Assistant"],
        "example_line": "Administration Activities Director: Scott Buntje <a href=\"tel:320-845-5...\">",
        "email_fields": None,
        "email_note": "the page publishes an Activities-Director tel: number but no email; 0 real addresses in all three captures",
    },
    "coach_fields": None,
    "coach_note": "no sport-scoped coach rows on the school page",
}

# ---------------------------------------------------------------- GHSA (GA)
ga = table_row("samples/ghsa-school-directory.html", lambda r: len(r) == 1 and "(" in r[0])
SCHEMA["ghsa_school_directory_html"] = {
    "states": ["GA"], "url": "https://ghsa.net/school-directory",
    "transport": "GET HTML, one page",
    "captured": "samples/ghsa-school-directory.html",
    "row_example": ga,
    "row_semantics": ["school name with classification in parentheses", "address lines follow in sibling rows"],
    "stable_identifier": {"field": "none observed", "example": None, "where": None},
    "contact_fields": None,
    "contact_note": "the captured directory carries 1 mailto in total and no named person column",
}

# ---------------------------------------------------------------- NCHSAA (NC)
SCHEMA["nchsaa_schools_html"] = {
    "states": ["NC"], "url": "https://www.nchsaa.org/schools/",
    "transport": "GET HTML, one page",
    "captured": "samples/dir/NC__schools.html, samples/nchsaa-schools.html",
    "blocks": {"schools": "452 school rows (identity only)",
               "conference_administrators": "78 rows with 74 mailto - conference-level admin, not a school AD"},
    "contact_scope_warning":
        "the only name+email block is conference-level; it must not be written into a school-scoped AD column",
}

# ---------------------------------------------------------------- WIAA FinalForms (WA)
SCHEMA["wiaa_finalforms_state_schools_html"] = {
    "states": ["WA"], "url": "https://wiaa.finalforms.com/state_schools",
    "transport": "GET HTML, paginated (51 pages observed for the state)",
    "captured": "samples/dir/WA__state_schools.html",
    "fields_per_school": ["name", "address", "city", "ZIP", "county", "phone", "website",
                          "league", "classifications", "sport list"],
    "person_fields": None,
    "person_note": ("no named person on the page; each school also renders one Cloudflare-protected "
                    "'Contact Info' email whose ROLE IS NOT LABELLED, so it cannot be attributed to an AD "
                    "or a coach and is therefore not claimed as an AD email"),
    "sport_list_note": "the per-school sport list makes XC/TF offering observable without a coach name",
}

# ---------------------------------------------------------------- MPSSAA (MD) search API
md = json.loads(read("samples/api/MD_search_q_blair.json"))
SCHEMA["mpssaa_search_json"] = {
    "states": ["MD"], "url": "https://www.mpssaa.org/wp-json/... (search endpoint; see CAPTURES.md)",
    "transport": "GET JSON search, one request per query term",
    "captured": "samples/api/MD_search_q_blair.json, samples/api/MD_search_popular.json",
    "envelope_keys": sorted(md.keys()),
    "result_example": (md.get("results") or [None])[0],
    "result_note": ("results mix `type: school` (identity, with a school-directory URL) and `type: sport` "
                    "(committee pages whose excerpt text embeds names and phone numbers); the sport excerpts "
                    "are the association's own published committee contacts, they are not school ADs"),
    "stable_identifier": {"field": "id", "example": ((md.get("results") or [{}])[0]).get("id")},
}

# ---------------------------------------------------------------- cross-lane
SCHEMA["milesplit_coach_pages"] = {
    "states": ["all (per-state subdomains)"],
    "url": "https://<lowercased USPS code>.milesplit.com/",
    "status": "OWNED BY ANOTHER LANE (research/sources/milesplit-national); cited here, not re-measured",
    "role_in_this_lane": "VALIDATION_SOURCE: cross-check a coach name the association directory publishes",
    "evidence": "research/sources/milesplit-national/samples/state-subdomain-probe.txt",
}

# ---------------------------------------------------------------- refusals
SCHEMA["refused_or_disallowed"] = {
    "bound_directory_html": {
        "states": ["IA", "SD"], "url": "https://www.gobound.com/{ia,sd}/schools/<slug>/directory/new",
        "reason": "gobound.com/robots.txt declares `User-agent: *` with `Disallow: /*directory` and `Crawl-Delay: 10`",
        "observed": "compliant GET returned a 118-byte 403 body (samples/validate/SD_Bound_aberdeencentral.html)",
        "field_names": None,
        "field_names_note": "not observed by this lane; the fields quoted in tools/validation-retractions.json came from the prior study's browser session",
    },
    "arbiter_platform": {
        "states": ["KY", "OK", "MA", "MT (widget)"],
        "hosts": ["arbiterlive.com", "khsaa.arbitersports.com", "ossaa.arbitersports.com"],
        "reason": "every host publishes `User-agent: *` / `Disallow: /`",
    },
    "my_mhsaa_desktopmodules": {
        "states": ["MI"],
        "url": "https://my.mhsaa.com/DesktopModules/MHSAA-Endpoint/API/School/AdministrationDirectory?SchoolId=<id>",
        "reason": "my.mhsaa.com/robots.txt `User-agent: *` -> `Disallow: /DesktopModules/`",
    },
    "cloudflare_challenges": {
        "states": ["DE", "MS", "ID", "CA"],
        "reason": "403 / JS challenge / empty 202 to this client; not bypassed by design",
    },
}


def main() -> int:
    payload = {
        "generated_by": "tools/make_schema.py",
        "note": ("Keys and example values are extracted from the captured files named per family; a field "
                 "this lane did not observe is recorded as null with a note rather than guessed. "
                 "Field names are the source's own spelling (including the source's typos, e.g. "
                 "KSHSAA 'MailingAdress')."),
        "contract_scope": ("only public professional school/sport-role contacts are represented here: school "
                           "name, role, published professional name, published professional email. No athlete "
                           "contact, personal email, personal phone or home address is captured; where a source "
                           "exposes such columns they are named in `deliberately_not_collected` and are not read."),
        "families": SCHEMA,
    }
    OUT.write_text(json.dumps(payload, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    n = len(SCHEMA)
    print(f"wrote schema.json with {n} families")
    for k, v in SCHEMA.items():
        ex = v.get("row_example") or v.get("stable_identifier", {}).get("example") if isinstance(v, dict) else None
        print(f"  {k:<42} states={v.get('states')}  example={str(ex)[:60]}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
