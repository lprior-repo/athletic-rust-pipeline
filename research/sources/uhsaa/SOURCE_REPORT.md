# UHSAA Source Report

Source ID: SRC-164
State: UT (Utah)
Association: Utah High School Activities Association (UHSAA)

## URLs

Directory: https://uhsaa.org/school-directory-new/
School profile template: https://uhsaa.org/school-directory/?id=<Name>&Reg=<Region>&schoolID=<ID>
Evidence URL verified: https://uhsaa.org/school-directory/?Reg=6&id=Alta&schoolID=1

## robots.txt

Fetched: https://uhsaa.org/robots.txt
Status: 200 OK
Date: 2026-10-04
Directory and profile pages are allowed. No relevant disallows.

## Capture Inventory

Raw responses captured 2026-10-04 with the research user-agent (bytes exactly as served):

1. `research/sources/uhsaa/robots.txt` - 200, 1181 bytes, sha256 `1414d91a…` - directory and profile
   paths allowed; `/v2/`, `/js/`, `/media/` and the Joomla admin/library prefixes are disallowed.
2. `crates/census-crawl/tests/fixtures/uhsaa/directory.html` - https://uhsaa.org/school-directory-new/
   200, 484,909 bytes, sha256 `3ed900a9…` - 324 school anchors over **162 distinct schools**
   (`school-directory/?id=<Name>&Reg=<Region>&schoolID=<ID>`), tiles carrying name and classification.
3. `crates/census-crawl/tests/fixtures/uhsaa/profile-alta.html` - `?Reg=6&id=Alta&schoolID=1`, 200,
   458,367 bytes, sha256 `6cf4845f…` - Rebecca Bennion (XC and track).
4. `crates/census-crawl/tests/fixtures/uhsaa/profile-murray.html` - `?Reg=10&id=Murray&schoolID=73`,
   200, 456,574 bytes, sha256 `fa37204c…` - Diana Stewart (XC), JennaBree Tollestrup (track).
5. `crates/census-crawl/tests/fixtures/uhsaa/profile-herriman.html` - `?Reg=2&id=Herriman&schoolID=43`,
   200, 456,554 bytes, sha256 `aba002e9…` - Josh Pugel (XC), Corey Wales (track).

Each profile is ~456 KB of template plus the `.school-header` block (`h1.school-name`, `ul.school-details`
with street address, district, classification, region, phone/fax) and the sports table whose rows pair
a sport label with a `mailto:` coach link.

## Field Inventory

Published by UHSAA on school profiles:

School info:
- Name (h1 heading)
- Address (from ul list item)
- District (from ul list item)
- Classification (from ul list item)
- Region (from ul list item)
- Colors (from ul list item)
- Phone (from ul list item)
- Fax (from ul list item)

Coach info (per sport row in table):
- Sport label (e.g., "Boys Cross Country", "Girls Track & Field")
- Coach name (hyperlinked text)
- Coach email (in mailto: href attribute)
- Schedule link (MaxPreps URL, present for some sports)

Staff directory (separate table):
- Role (Principal, AD, Athletic Trainer, etc.)
- Name (hyperlinked with email)

## Verified Coach Data (from captures)

Alta Hawks (schoolID=1):
- Boys Cross Country: Rebecca Bennion (Rebecca.Bennion@canyonsdistrict.org)
- Girls Cross Country: Rebecca Bennion (Rebecca.Bennion@canyonsdistrict.org)
- Boys Track & Field: Rebecca Bennion (Rebecca.Bennion@canyonsdistrict.org)
- Girls Track & Field: Rebecca Bennion (Rebecca.Bennion@canyonsdistrict.org)

Murray Spartans (schoolID=73):
- Boys Cross Country: Diana Stewart (dstewart@murrayschools.org)
- Girls Cross Country: Diana Stewart (dstewart@murrayschools.org)
- Boys Track & Field: JennaBree Tollestrup (jtollestrup@murrayschools.org)
- Girls Track & Field: JennaBree Tollestrup (jtollestrup@murrayschools.org)

Herriman Mustangs (schoolID=43):
- Boys Cross Country: Josh Pugel (joshkpugel@gmail.com)
- Girls Cross Country: Josh Pugel (joshkpugel@gmail.com)
- Boys Track & Field: Corey Wales (corey.wales@jordandistrict.org)
- Girls Track & Field: Corey Wales (corey.wales@jordandistrict.org)

## Known Limits

1. Empty coach cells exist (e.g., Murray Boys Lacrosse shows "()" or "<>") - these are treated as absent.
2. Some schools may not have coach data published for all sports.
3. Only head coaches are listed per sport - no assistant coaches.
4. Email addresses are in mailto: href, not displayed as text.
5. No phone numbers for coaches (only school-level phone).
6. Directory page carries 162 schools on one page; no pagination observed.
7. UHSAA covers public, charter, and private member schools statewide.
