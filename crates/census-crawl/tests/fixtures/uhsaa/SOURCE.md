# UHSAA Fixture Sources

Capture date: 2026-10-04. User-Agent:
`athletic-rust-pipeline-research/0.1 (+https://github.com/lprior-repo/athletic-rust-pipeline)`.
Every file is the response body exactly as served by `uhsaa.org` (a plain `GET`, no
re-serialization); bytes and sha256 below were measured with `sha256sum` and `wc -c` on the
committed files.

## robots.txt

Fetched: https://uhsaa.org/robots.txt — 200, 1181 bytes,
sha256 `1414d91a2b81cecf2555af4fbfa52128004b9b679a9809ae61cdb38b1ebf59c8`.
The directory and profile paths the adapter reads carry no disallow. Disallowed prefixes include
`/administrator/`, `/cache/`, `/cli/`, `/components/`, `/images/`, `/includes/`, `/installation/`,
`/language/`, `/libraries/`, `/logs/`, `/media/`, `/modules/`, `/portal/`, `/plugins/`, `/site/`,
`/templates/`, `/tmp/`, `/new/`, `/mlr/`, `/machform/`, `/portal2/`, `/phpgridnew/`, `/phpGrid2/`,
`/phpGrid/`, `/realignmentcalc/`, `/realignmentRegionCreator/`, `/vendor/`, `/js/`,
`/backuphomepages/`, `/v2/`.

## Directory Page

`directory.html` — https://uhsaa.org/school-directory-new/ — 200, 484,909 bytes,
sha256 `3ed900a99bd11aa577bf15fe975a0920748780756c94ec19f04c28a214152d41`.
The school tiles are `<a href="../school-directory/?id=<Name>&Reg=<Region>&schoolID=<ID>" title="<School>"
class="portfolio-item" data-school="<Name>" data-category="<Classification>">` (both quote styles occur).
The capture carries 324 anchors over 162 distinct school ids; `<v2/teams/...>` links beside them are
robots-disallowed and unread.

## School Profile Pages

Each profile is the same page template: `<h1 class="school-name">` and a `<ul class="school-details">`
list (street address, `District:`, `Classification:`, `Region:`, colors, phone, fax) followed by a
sports table whose rows pair a sport label (`Boys Cross Country`, `Girls Track & Field`, …) with a
`mailto:` coach link.

- `profile-alta.html` — https://uhsaa.org/school-directory/?Reg=6&id=Alta&schoolID=1 — 200, 458,367
  bytes, sha256 `6cf4845fab4f61e2a13322411764547d0e344b63e7ee55b71c97008a8e7418bf`.
  XC and track: Rebecca Bennion (`Rebecca.Bennion@canyonsdistrict.org`).
- `profile-murray.html` — https://uhsaa.org/school-directory/?Reg=10&id=Murray&schoolID=73 — 200,
  456,574 bytes, sha256
  `fa37204cfce4e2db4db79d8c7742e52db1903f06bad2c519e435d55025f45d42`.
  XC: Diana Stewart (`dstewart@murrayschools.org`); track: JennaBree Tollestrup
  (`jtollestrup@murrayschools.org`).
- `profile-herriman.html` — https://uhsaa.org/school-directory/?Reg=2&id=Herriman&schoolID=43 — 200,
  456,554 bytes, sha256
  `aba002e9700ac87449f8f65cbabc192769cb1fccc5912aa3200b2371628b6367`.
  XC: Josh Pugel (`joshkpugel@gmail.com`); track: Corey Wales (`corey.wales@jordandistrict.org`).

## Field Inventory

Published per sport row: sport label, head-coach name (linked), `mailto:` address, and the MaxPreps
schedule link for sports that have one. The page also carries a staff-directory table (principal,
AD, trainers) the adapter reads past. Not published: assistant coaches, coach phone numbers, tenure.
