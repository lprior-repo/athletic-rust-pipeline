# CHSAA member directory

Adapter for the Colorado High School Activities Association, ported from the prototype's
`parsers/co_chsaanow.py` (directory) and `parsers/co_school.py` (school pages).

## Endpoints

- Directory: `GET https://chsaanow.com/schools/` renders one embedded JSON array of the 378 active
  member schools with `schoolCode`, `slug`, `name`, `officialName`, `city`, `memberType`,
  `schoolType`, `setting`, `districtName`, `streetAddress`, `zipCode`, `phone`, `href`, `logoUrl`
  and `mapUrl`. The page escapes the array inside a Next.js payload, so the parser searches for the
  escaped marker `[{\"schoolCode\"` and unescapes before decoding.
- School page: `GET https://chsaanow.com/schools/<slug>/` carries the school's activities, each with
  its coach `positions[]` and their `title`. The page's title element names the school, which every
  row inherits. Activities are located by the `{"activityName":` marker and each one is consumed to
  the end offset serde reports for it, so two markers with no bytes between them yield two
  activities; no fixed-length advance is ever applied to a marker.
  `chsaa::tests::adjacent_short_activity_objects_are_both_retained` pins that boundary.

## Mapping

- School: `name`, `city`, `UsJurisdiction::Colorado`, `association: "CHSAA"`, identity
  `(source_id "chsaa", schoolCode)` and the fetched page URL. One observation per school, kind
  `Directory`, carrying `memberType`, `schoolType`, `memberStatus: "Active"` and the site URL.
- Coach: activities whose name contains `track` map to `Sport::OutdoorTrack`, `cross country` to
  `Sport::CrossCountry`; gender comes from the `Boys`/`Girls` prefix (`Gender::Mixed` otherwise);
  `Head Coach`/`Head` and `Assistant Coach`/`Assistant` map to `CoachRole::Head`/`Assistant`;
  `Athletic Director` and every other title or activity is dropped. Rows repeated on the page are
  deduplicated on `(person, activity, title)`.
- CHSAA publishes no coach address, so the mapper never sets one; that is also why the adapter's
  capabilities are `SCHOOL_COACH_NAMES` rather than `SCHOOL_COACH_CONTACT`.

## Registered scope

Slug `chsaa`, host `chsaanow.com`, `TransportKind::Html`, 1 request/s, one in flight, robots
respected. `robots.txt` allows `/` for `User-agent: *` and disallows only
`/history/champions/individual/totals/repeat/`, `/history/champions/individual/totals/repeat/*` and
`/preview/`; the one `Crawl-delay: 10` applies to PetalBot, not to the wildcard agent.

## Not stored

The directory's `streetAddress`, `zipCode`, `phone`, `districtName`, `memberType`, `schoolType` and
`setting` are parsed and pinned field-for-field against the prototype's golden, but `CanonicalSchool`
and `SourceSchoolObservation` carry no field for them, so they do not enter the graph.
