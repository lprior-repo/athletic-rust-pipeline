# PIAA member directory

Adapter for the Pennsylvania Interscholastic Athletic Association, ported from the prototype's
`parsers/pa_piaa.py` (letter pages) and `parsers/pa_piaa_details.py` (school contacts).

## Endpoints

- `GET https://www.piaa.org/schools/directory/list.aspx?alpha=<L>` renders one
  `<dl id="<PIAA id>" class="schoolBlock">` per member school: `<dt>` carries the name, the first
  non-empty `<dd>` the single printed address line `4773 NATIONAL PIKE, MARKLEYSBURG, PA  15459`,
  and the link bar the school's own details URL. The host answers the https URL with a 301 to the
  same path over plain http, so a supervised run carries operator authorization for
  `www.piaa.org` (`--authorized-host www.piaa.org`); the destination guard refuses the hop
  otherwise.
- The linked letters are A..W plus Y. `X` and `Z` print no link, and the site answers `alpha=Z`
  with the A group, so the adapter walks the 24 linked letters only.
- `GET https://www.piaa.org/schools/directory/details.aspx?ID=<id>` prints the school's vCard
  contacts under `School Contacts`, each with a `<div class="title">` post, the name spans and a
  `mailto:` address. The page title names the school, which the adapter checks against the
  directory row before accepting the page.

## Mapping

- School: `name`, `city`, `UsJurisdiction::Pennsylvania`, `association: "PIAA"`, identity
  `("pa_piaa", <PIAA id>)` carrying the school's own details URL, plus evidence naming the letter
  page it was read from.
- Contact: PIAA publishes **no coaches**. Its posts are Superintendent, Principal and Athletic
  Director, and only the athletic-director posts (Athletic Director, Assistant Athletic Director,
  Activities Director) are emitted, as `CoachRole::AthleticDirector` with no sport and
  `Gender::Mixed`, and the published address when the vCard carries one. One row per person per
  school.
- The printed address line splits into street (the text before the last comma of the leading
  group), city, state and ZIP. Only the city has a field on `CanonicalSchool`, so the street, the
  state text and the ZIP are parsed and pinned by the prototype golden but not stored.

## Crawl shape

`collect` walks the letters, stores each member school and reads each school's details page.
`Options::letters` narrows the letters, `Options::details_names` the schools whose details page is
read, and `Options::limit` caps the letter pages. Every school is journalled as
`PA:school:<id>` and every letter, once its schools are written, as `PA:list:<L>`, so a re-run
neither refetches a letter nor rereads a details page. Requests run at 1/s, one in flight, with
robots respected: `robots.txt` allows `/schools/…` to `User-agent: *`, and
`/officials/directory/` is disallowed, which is why PIAA's officials never reach this adapter.

## Not stored

The details page also states the PIAA district, school district, school type, last enrollment date
and enrollment figures. The census school record carries no field for them, and
`research/sources/state-assoc-midatlantic/SOURCE_REPORT.md` rates PIAA a validation source for
schools rather than an athlete or grade source; grades for Pennsylvania come from the MileSplit
lane.
