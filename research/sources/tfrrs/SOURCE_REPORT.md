# TFRRS High School Performance Lists

State-instance TFRRS sites publish cumulative performance lists by event, school division, and league for each high school track and field season.

## Hosts

State-specific hosts follow `https://{state}.tfrrs.org/`:
- `indiana.tfrrs.org` — Indiana (verified)
- `florida.tfrrs.org` — Florida (descriptor registered; index not yet captured)
- `nh.tfrrs.org` — New Hampshire (descriptor registered; index not yet captured)

State label is parsed from the host subdomain; the reader rejects any other host.

## Index Pages

Each state instance publishes two index pages enumerating the available performance lists:

- `https://{state}.tfrrs.org/indoor_lists.html` — indoor season lists
- `https://{state}.tfrrs.org/outdoor_lists.html` — outdoor season lists

Each index page contains links to `/lists/{id}/{slug}` pages. The links may include optional query filters: `?year=`, `?gender=`, `?standard_event_hnd=`.

**Live capture provenance:**
- `indoor-lists.body` — `https://indiana.tfrrs.org/indoor_lists.html` (sha256 recorded by Main)
- `outdoor-lists.body` — `https://indiana.tfrrs.org/outdoor_lists.html` (sha256 recorded by Main)

## List Pages

List pages enumerate performances by event. Two URL shapes exist:

1. **Fixture shape (existing):** `https://{state}.tfrrs.org/lists/{id}/{slug}/{year}/{i|o}` — includes explicit year and season tokens.
2. **Live shape (extended):** `https://{state}.tfrrs.org/lists/{id}/{slug}` — no year/season segment; uses optional query filters instead.

The route classifier accepts both shapes. Query filters (`?year=`, `?gender=`, `?standard_event_hnd=`) are parsed but not required for list admission.

**Live capture provenance:**
- `list-5489.body` — `https://indiana.tfrrs.org/lists/5489/HSR_All_School_Performance_List` (sha256 recorded by Main)

## Collection Strategy

The results arm discovers the frontier rather than consuming a preseeded meet list:

1. For each applicable jurisdiction (FL, IN, NH), fetch the indoor and outdoor index pages.
2. Parse `/lists/{id}/{slug}` links from the index pages.
3. Walk each discovered list URL, applying the existing TFRRS list page reader.
4. Completion requires every discovered list URL to be processed. Discovered-but-unfinished lists remain owed on reopen.
5. Discovery state is journaled keyed by index URL so subsequent runs resume without re-discovering.

## What the Stage Persists

The arm reuses the existing TFRRS collector, parse, and map layers. It persists:
- Canonical performances, schools, teams, athletes, and events
- Source observations for every raw row absorbed
- Journal entries for every processed list URL (body digested)
- Discovery state for index pages

The arm reports results as a single `ResultsSourceRows` entry under slug `tfrrs`, including `meets`, `rows`, `disposition`, `errors`, `withheld`, `notes`, `unfinished` list URLs, and `unresolved` counters.

## Verification Pending

- `florida.tfrrs.org/indoor_lists.html` and `outdoor_lists.html` have not been captured; their shape is assumed to match Indiana.
- `nh.tfrrs.org/indoor_lists.html` and `outdoor_lists.html` have not been captured; their shape is assumed to match Indiana.

These URLs are published by the same platform family and use the same template structure. The route classifier handles any `/lists/{id}/{slug}` URL regardless of jurisdiction, so a different index layout would still be walked correctly once the URLs are discovered.