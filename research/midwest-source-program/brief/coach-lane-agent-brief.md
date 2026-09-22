# Coach-lane agent brief (national Class-of-2027 TF/XC census)

You are one of ~51 state agents raising **head coach / XC coach / athletic director** coverage for the
United States Class-of-2027 high-school Track & Field / Cross Country recruiting census.

## 1. Where things are

| Thing | Path |
|---|---|
| Census repo (READ-ONLY for you — never edit, never run its builds) | `/home/lewis/src/ad-law-scrape/athletic-rust-pipeline` |
| Research workspace (yours to write) | `/home/lewis/Downloads/midwest-tfxc-source-research` |
| Your work list | `targets/<ST>.csv` — columns `school_id,name,co2027,athletes_all`, ranked by cohort size |
| Your deliverable | `out/coach-fragments/<ST>.csv` |
| Prior national research (read first) | `research/sources/coach-directories-national/SOURCE_REPORT.md`, `schema.json`, `coverage.json` |

Do not create files anywhere else. Do not modify another state's fragment.

## 2. Deliverable format

`out/coach-fragments/<ST>.csv` — exactly this header, one row per (school, sport, role):

```
school,city,state,sport,role,coach_name,public_professional_email,ad_name,ad_email,source_url,last_observed
```

The Rust importer parses these labels, so use them **verbatim**:

- `state` — the two-letter code. `last_observed` — the date you actually read the page (`2026-09-22` today).
- `sport` — one of `Boys Track and Field`, `Girls Track and Field`, `Boys Cross Country`,
  `Girls Cross Country`, `Boys Indoor Track`, `Girls Indoor Track`.
  A label with no literal `track` or `cross country` will not register as a sport.
- `role` — `Head Coach` or `Assistant Coach` for coaching rows.
- Athletic director goes in the `ad_name` / `ad_email` columns **of that school's rows**; additionally
  emit one row per school with an empty `sport` and `role=Athletic Director` when the AD is published.
- Quote any field containing a comma. Leave a field empty rather than guessing.

## 3. Source hierarchy (work in this order)

1. the state association's own directory for that sport/role, when it publishes contacts;
2. the school's **official athletics page** (usually a district domain such as `schoolname.k12.xx.us`,
   or an athletics subdomain) — this is where coach names and `mailto:` addresses actually live;
3. the official district / school **staff directory**;
4. otherwise a qualified public athletics source that cites the school (state coaches association, etc.).

Search tip: `"<school name>" track coach email`, `"<school name>" athletic director site:<district domain>`
and reading the athletics "Coaches" / "Staff" page usually resolves a school in 1–3 fetches.

## 4. Hard constraints (non-negotiable)

- Public **professional school/sport-role** contacts only: name, role, published professional email,
  source URL, observation date.
- NEVER record: home addresses, personal or mobile phone numbers, personal email addresses, athlete or
  parent contacts, birthdates. If a directory exposes them, read past them and do not ingest.
- Respect `robots.txt` per host. If a host disallows the path, skip the host and note it. Never use a
  headless browser to fetch a path a host's `robots.txt` disallows.
- No CAPTCHA solving, no auth-wall circumvention, no paid/subscription content, no data brokers.
- `source_url` must be a page **you fetched in this session** and that actually shows the name/email you
  wrote. One row per real observation. Never invent a name, an email, or a URL.

## 5. Acceptance (all of it, in your final report)

1. The fragment CSV exists with the exact header.
2. **Control check** — pick 3 of your rows, and for each prove the cited `source_url` really contains the
   string you wrote (quote the surrounding match). Then mutate one name or email (e.g. swap the domain)
   and show it does **not** appear. A check that cannot fail proves nothing.
3. Raw counts: schools attempted; schools with a coach name; with a coach email; with an AD name or email;
   per-source tallies; every skip with its reason (robots, 403, JS-only, no coach published, no page found).
4. State plainly which schools from the work list you did **not** get to, so the next wave can resume there.

## 6. Budget

Work the **top 40 schools** from your work list (highest `co2027` first). Breadth beats depth: a name with
no email is still value; a school you never opened is not. Stop at 40, report, and hand back.
