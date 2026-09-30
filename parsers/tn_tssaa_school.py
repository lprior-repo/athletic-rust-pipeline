"""
Tennessee School Directory (portal.tssaa.org) - single-school detail parser.

This parser reads a single school's detail page (obtained by deepening the
TSSAA directory listing with `--parser tn_tssaa_school --url-field detail_url`).
Each detail page contains staff information organized into sport-specific
cards. This parser emits only Track, CrossCountry, and AthleticDirector
coach rows from those cards.

Source: https://portal.tssaa.org/common/directory/ (list)
      https://portal.tssaa.org/common/directory/?id=<school_id> (detail)

Limitations:
- School address is not present on the detail page; only city/county/phone.
- Sport is derived from the card header (e.g., "Boys' Cross Country",
  "Boys' Track and Field") or from the `sportN` anchor preceding each card.
- Email addresses are obfuscated on the page via JS (`mail_hide`). This parser
  decodes them by reversing the domain argument, so the `email` field in
  output is the actual address, not the literal obfuscated form.
"""

import re
from bs4 import BeautifulSoup


# TSSAA sport code to sport name mapping (for sportN anchors)
SPORT_CODE_NAMES = {
    "1": "Football",
    "2": "Boys' Basketball",
    "3": "Girls' Basketball",
    "4": "Baseball",
    "5": "Girls' Softball",
    "6": "Swimming",
    "7": "Wrestling",
    "8": "Boys' Cross Country",
    "9": "Girls' Cross Country",
    "10": "Boys' Track and Field",
    "11": "Girls' Track and Field",
}


def decode_email(user_part, domain_part):
    """
    Decode TSSAA obfuscated email addresses.
    The page passes arg2=user_part, arg4=domain_part (reversed).
    Returns "user@reversed_domain".
    """
    domain = domain_part[::-1]
    return f"{user_part}@{domain}"


def parse_sport_from_header(header_text):
    """
    Extract census sport value from a card header.
    Headers like "Boys' Cross Country" or "Girls' Track and Field".
    Returns (sport, gender).
    """
    header = header_text.strip()

    # Check for Athletic Director
    if "Athletic Director" in header or header.strip().endswith("AD"):
        return "AthleticDirector", ""

    # Check for Cross Country
    if "Cross Country" in header:
        gender = ""
        if header.startswith("Boys'") or header.startswith("Boys "):
            gender = "Boys"
        elif header.startswith("Girls'") or header.startswith("Girls "):
            gender = "Girls"
        return "CrossCountry", gender

    # Check for Track
    if "Track" in header:
        gender = ""
        if header.startswith("Boys'") or header.startswith("Boys "):
            gender = "Boys"
        elif header.startswith("Girls'") or header.startswith("Girls "):
            gender = "Girls"
        return "Track", gender

    return None, ""


def parse_sport_from_code(code):
    """Map a sportN code to a census sport value."""
    sport_name = SPORT_CODE_NAMES.get(code, "")
    if not sport_name:
        return None, ""

    if "Cross Country" in sport_name:
        gender = ""
        if sport_name.startswith("Boys'"):
            gender = "Boys"
        elif sport_name.startswith("Girls'"):
            gender = "Girls"
        return "CrossCountry", gender

    if "Track" in sport_name:
        gender = ""
        if sport_name.startswith("Boys'"):
            gender = "Boys"
        elif sport_name.startswith("Girls'"):
            gender = "Girls"
        return "Track", gender

    return None, ""


def parse(body):
    """
    Parse a single TSSAA school detail page.
    Returns a dict with schools and coaches lists.
    """
    html = body.decode("utf-8", "replace")
    soup = BeautifulSoup(html, "html.parser")

    result = {"schools": [], "coaches": []}

    # Extract school name from <h2>
    h2 = soup.find("h2")
    school_name = h2.get_text().strip() if h2 else "Unknown"

    # Extract city from <p class="mt-0"> or nearby text
    city = ""
    p = soup.find("p", class_="mt-0")
    if not p:
        for elem in soup.find_all("p"):
            if school_name in elem.get_text():
                p = elem
                break
    if p:
        text = p.get_text().replace("\n", " ").strip()
        # TSSAA format: "Adamsville, TN (McNairy County)"
        match = re.search(r"([A-Za-z\s'-]+),\s*TN", text)
        if match:
            city = match.group(1).strip()

    # Extract phone
    phone = ""
    phone_label = soup.find(string=lambda s: s and "Office Phone:" in s)
    if phone_label:
        parent = phone_label.parent
        if parent:
            siblings = parent.find_next_siblings()
            for sib in siblings:
                if sib.name == "a" and sib.get("href", "").startswith("tel:"):
                    phone = sib["href"][4:].strip()
                    break
                if sib.string and re.search(r"\d{3}-\d{3}-\d{4}", str(sib.string)):
                    phone = re.search(r"(\d{3}-\d{3}-\d{4})", str(sib.string)).group(1)
                    break

    # Add school record
    result["schools"] = [{
        "name": school_name,
        "city": city,
        "state": "TN",
        "phone": phone,
    }]

    # Find all staff rows (tr with class staffPerson)
    staff_rows = soup.find_all("tr", class_="staffPerson")

    for row in staff_rows:
        # Find the card that contains this row
        card = row.find_parent("div", class_=re.compile("card"))
        if not card:
            continue

        # Find the card header for this card
        header_div = card.find("div", class_=re.compile("card-header"))
        if not header_div:
            continue

        header_text = header_div.get_text()
        sport, gender = parse_sport_from_header(header_text)

        # If not found in header, try sportN anchor before the card
        if sport is None:
            prev = card.find_previous_sibling()
            if prev and prev.name == "div" and prev.get("id", "").startswith("sport"):
                code = prev["id"][5:]
                sport, gender = parse_sport_from_code(code)

        # Only process Track, CrossCountry, and AthleticDirector
        if sport not in ("Track", "CrossCountry", "AthleticDirector"):
            continue

        # Parse the staff row
        tds = row.find_all("td")
        if len(tds) < 3:
            continue

        # Name
        name = tds[0].get_text().strip()

        # Role
        role_text = tds[1].get_text().strip()
        if role_text == "Head Coach":
            role = "HeadCoach"
        elif role_text == "Assistant Coach":
            role = "AssistantCoach"
        elif role_text in ("Athletic Director", "AD"):
            role = "AthleticDirector"
        else:
            role = role_text

        # Email (obfuscated via mail_hide JS)
        email = ""
        email_cell = tds[2]
        # Look for mail_hide script call
        mail_hide_match = re.search(
            r'mail_hide\("(\d+)",\s*"([^"]+)",\s*\d+\s*,\s*"([^"]+)"\)',
            str(email_cell)
        )
        if mail_hide_match:
            user = mail_hide_match.group(2)
            domain_reversed = mail_hide_match.group(3)
            try:
                email = decode_email(user, domain_reversed)
            except Exception:
                email = ""

        result["coaches"].append({
            "school": school_name,
            "state": "TN",
            "person": name,
            "sport": sport,
            "role": role,
            "gender": gender,
            "email": email,
        })

    return result