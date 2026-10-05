# AIA (Arizona Interscholastic Association) Source Report

## Source Identification

| Field | Value |
|-------|-------|
| Bundle ID | SRC-155 |
| Name | AIA member-school and sport-coach directory |
| URL | https://aiaonline.org/schools |
| Category | state_athletics_coach_directory |
| Coverage | 287 member schools claimed by landing; public/private statewide |
| Priority | P1 |

## Access URLs

- School search JSON: `https://aiaonline.org/schools/search.json?q={query}`
- School profile: `https://aiaonline.org/schools/{id}` (HTML, Vue.js SSR)
- Robots: `https://aiaonline.org/robots.txt` (allows all paths)

## Capture Date

2026-10-04

## Robots Status

```
User-agent: *
Disallow:
```

All paths allowed. No restrictions observed.

## URLs Capped and Captured as Fixtures

| URL | Fixture File | Status | Byte Count | SHA256 |
|-----|--------------|--------|------------|--------|
| `https://aiaonline.org/robots.txt` | `tests/fixtures/aia/robots.txt` | 200 | 24 | `e5c4b844…` |
| `https://aiaonline.org/schools/search.json?q=chandler` | `tests/fixtures/aia/search_chandler.json` | 200 | 3,809 | `888c7992…` |
| `https://aiaonline.org/schools/100` | `tests/fixtures/aia/school_100_chandler.html` | 200 | 105,424 | `f59ff639…` |
| `https://aiaonline.org/schools/68` | `tests/fixtures/aia/school_68_seton.html` | 200 | 91,648 | `54333200…` |
| `https://aiaonline.org/schools/116` | `tests/fixtures/aia/school_116_hamilton.html` | 200 | 94,785 | `a68acf08…` |

The captures are bytes the live site returned on 2026-10-04; an earlier fixture set did not match
the live template and was replaced after the first live provider run failed every profile with
`aia parsing failed`.

## Field Inventory

### School Search JSON (`/schools/search.json?q={query}`)

Returns JSON array of school objects with these fields:

| Field | Type | Present | Notes |
|-------|------|---------|-------|
| `id` | integer | Always | School numeric ID |
| `name` | string | Always | Short name (e.g., "Chandler") |
| `full_name` | string | Always | Full name (e.g., "Chandler High School") |
| `mascot` | string | Always | Team mascot name |
| `logo` | object | Always | Contains `thumbnail` URL |
| `alignment` | object | Always | Contains `conference` (e.g., "6A") and `region` (e.g., "Premier") |
| `address` | object | Always | Contains `line1`, `city`, `state`, `zip` |

### School Profile HTML (`/schools/{id}`)

Server-side rendered Vue.js HTML. Data available in the rendered DOM:

**School Info:**
- School name (short and full)
- Mascot name and logo URL
- Division (e.g., "6A", "3A")
- Region (e.g., "Premier", "Metro Mountain")
- City, state
- School colors
- Address (street, city, state, zip)
- Phone number
- Fax number
- Website URL

**Staff Directory:**
- Superintendent, Principal, Athletic Director, etc.
- Phone numbers for administrative staff
- Note: Coach emails are NOT publicly listed; admin directory requires login

**Sport Coaches:**
- Sport name (e.g., "Cross Country - Boy's", "Track - Boy's"); labels are HTML-escaped and the track cards say `Track`, not `Track & Field`
- Head coach name and role line (`Head Coach`); the card is `<div class="md:flex px-4 py-2 border-t dark:border-slate-700">` with the label in the `md:w-1/3` half and coach blocks in `md:w-2/3`
- Note: Assistant coaches NOT listed
- Note: Only head coaches are shown per sport

### Coaches Actually Published

AIA publishes **head coach names** for each sport/school combination. Coaches are listed:

- Fall: Cross Country (boys/girls), Football, etc.
- Winter: Basketball, Wrestling, etc.
- Spring: Track & Field (boys/girls), Baseball, Softball, etc.

For XC/TF specifically, the same coach typically appears for both cross country (fall) and track (spring/winter).

## Observed Coaches (From Fixtures)

### Chandler High School (ID: 100)

| Sport | Head Coach |
|-------|------------|
| Cross Country - Boy's | Michael King |
| Cross Country - Girl's | RikkiLynn Archibeque |
| Track - Boy's | Derrick Richardson |
| Track - Girl's | Eric Richardson |

### Seton Catholic Prep (ID: 68)

| Sport | Head Coach |
|-------|------------|
| Cross Country - Boy's | Dean Ouellette |

### Hamilton High School (ID: 116)

| Sport | Head Coach |
|-------|------------|
| Cross Country - Boy's | Mike Scannell |
| Cross Country - Girl's | Mike Scannell |
| Track - Boy's | E.J. Martin |
| Track - Girl's | E.J. Martin |

## Known Limits

1. **Search API returns max 10 results per query**: `GET /schools/search.json?q=` returns up to 10 schools. An empty query (`q=`) returned only 20 schools, not the complete ~287 inventory. To enumerate all schools, multiple city queries are needed.

2. **No complete school list API**: There is no documented endpoint that returns all 287 member schools in one request. The adapter works around this by querying multiple cities.

3. **Coach emails not public**: The AIA site states "Looking for email addresses? Our admin directory is available [here]" with a link to a login-required admin directory. No public coach-email API exists.

4. **JS SPA**: The school profile pages are Vue.js SPAs, but server-side rendering provides the data in the initial HTML. No API calls needed for parsing.

5. **Assistant coaches not listed**: Only head coaches are shown on the school profile pages.

## Implementation Notes

- The adapter uses the JSON search API to discover schools, then fetches individual HTML profiles for coach data.
- City-based search queries are used to enumerate schools (e.g., "phoenix", "tucson", "chandler"); the query list is a fixed sample because no endpoint returns the full inventory.
- Sport labels use AIA's naming convention (`Cross Country - Boy's`, `Track - Girl's`) which is mapped to census domain `Sport` and `Gender` types.
- XC and TF coaches are often the same person at the same school - the adapter captures each published card as its own row.
