# Indiana coach-fragment report (IN) — 2026-09-22

Deliverable: `out/coach-fragments/IN.csv` (130 rows, exact brief header).
Work list: top 40 of `targets/IN.csv` by `co2027`. Indiana had zero coaches imported before this lane.

## 1. Counts (measured from the delivered file)

| Measure | Value |
|---|---|
| Rows written | 130 (102 coach rows, 28 Athletic Director rows) |
| Top-40 work-list schools attempted | 40 (all opened) |
| Schools with ≥1 coach **name** | 27 of 40 |
| Schools with ≥1 coach **email** | 25 of 40 |
| Schools with an AD **name** | 28 of 40 |
| Schools with an AD **email** | 26 of 40 |
| Distinct coach names / coach emails | 77 / 69 |
| Distinct AD emails | 26 |
| Coach rows with a published coach email | 94 of 102 (8 name-only) |
| Sport mix | Boys XC 26, Girls XC 26, Boys TF 25, Girls TF 25 |

Per-source tallies (rows → host): `sites.eventlink.com` 52, `bdhs.wayne.k12.in.us` 5, `www.bhsnathletics.com` 5,
`brownsburgbulldogs.com` 5, `www.carmelgreyhounds.com` 5, `columbusnorthathletics.org` 5, `fisherstigersathletics.com` 5,
`fcflashes.org` 5, `hseathletics.com` 5, `www.harrisonathletics.com` 5, `noblesvillemillers.com` 5,
`nchsathletics.com` 5, `pikehsathletics.com` 5, `www.warren.k12.in.us` 5, `static.eventlink.com` 4 (PDF),
`athletics.roncalli.org` 4, `penn.phmschools.org` 2, `www.bhsspanthers.com` 1, `crownpointathletics.com` 1,
`ehsathletics.com` 1. → 43 distinct source URLs.

Source hierarchy actually used: 24 schools resolved through their official athletics portal
(Eventlink-powered `sites.eventlink.com/s/<slug>` or a school-hosted `/AthleticDepartment`), 4 through
district/school program pages (Ben Davis per-team pages, Penn, Warren Central staff directory, Crown Point's
published CPHS Coaches Directory PDF), 1 through per-sport coaches pages (Roncalli).

## 2. Control check (3 named rows, quoted matches, plus mutation)

Command: `/tmp/verify_in_fragment.py` (re-fetches every cited URL, checks each row's strings, then mutates).

```
### Avon High School <- https://sites.eventlink.com/s/avon-high-school/AthleticDepartment
  name  "Zach Toothman" @1760: ...eading Coach Kristen Dunbar kdunbar0915@hotmail.com Head Cross Country Coach (Boys/Girls) Zach Toothman zjtoothman@avon-schools.org Head Footba...
  email "zjtoothman@avon-schools.org" @1774: ...r0915@hotmail.com Head Cross Country Coach (Boys/Girls) Zach Toothman zjtoothman@avon-schools.org Head Football Coach Rob Gibso...
  MUTATION name  "Zach Toothmaq" present=False
  MUTATION email "zjtoothman@avon-schools.org.zz" present=False
### Noblesville High School <- https://noblesvillemillers.com/AthleticDepartment
  name  "Aaron Becker" @12881: ...in.us Boys Cross Country Bill Kenley ('89) bill_kenley@nobl.k12.in.us Girls Cross Country Aaron Becker ('08) aaron_becker@nobl.k12.in.us Dance...
  email "aaron_becker@nobl.k12.in.us" @12900: ...89) bill_kenley@nobl.k12.in.us Girls Cross Country Aaron Becker ('08) aaron_becker@nobl.k12.in.us Dance Team Bailey Mann bailey...
  MUTATION name  "Aaron Beckeq" present=False
  MUTATION email "aaron_becker@nobl.k12.in.us.zz" present=False
### Crown Point <- https://static.eventlink.com/public/8588a4b9-06ac-4bcb-9ab8-bde5b636df13/eab7f259-df86-4926-9b14-5b4e9c89582a/Untitled%20document.pdf
  name  "Seth Aydt" @157: ...ps.k12.in.us Basketball Clint Swan CPHS Extension 11285 cswan@cps.k12.in.us Cross Country Seth Aydt saydt@cps.k12.in.us Football Craig Buze...
  email "saydt@cps.k12.in.us" @167: ...Swan CPHS Extension 11285 cswan@cps.k12.in.us Cross Country Seth Aydt saydt@cps.k12.in.us Football Craig Buzea CPHS Ext...
  MUTATION name  "Seth Aydq" present=False
  MUTATION email "saydt@cps.k12.in.us.zz" present=False
```

Full-file verdict from the same tool (exit 0):

```
== stats ==
  rows=130 name_hit=130 email_hit=94 adjacent(<=400ch)=94 no_email_cited=36
  failures=0
== AD attribution ==
  schools with an AD row: 28; AD name+email proven at that URL: 28
  schools carrying ad_name on coach rows: 28; without an AD row (unbacked): none
  controls passed: 3/3
```

Every one of the 130 rows has its own cited URL re-fetched and its name (and, where present, its coach
email) located in that page's text. AD fields are proven separately against each school's AD-row URL
(the importer mints a school-wide AD from a coaching row's `ad_name`/`ad_email`, so coach rows carry the
school's AD; the AD row is the row whose URL proves it).

Sandbox importability proof (the real merge tool, own temp fragments dir so no sibling output is touched):

```
kept 130 rows from 1 states -> /tmp/in_sandbox/merged.csv
  with any email: 123  coach email: 94  AD rows: 28
  rejected: 0  unusable fragments: 0
```

## 3. Skips, with reasons (top-40 list)

| School (rank) | Reason |
|---|---|
| Hamilton Southeastern High Sch (14) | Duplicate census entity of *Hamilton Southeastern* (rank 3); rows written once under the canonical name, not duplicated under the alias. |
| Zionsville (17), Zionsville Community High Scho (24) | Duplicate census entities of *Zionsville Community* (rank 4); same handling. |
| Centerville (22) | HTTP 403 to this client on every path tried; no other official athletics page found. |
| Kirkwood (25) | No Indiana school by this name resolves (name collision); no official page found. |
| Franklin Community High School (27) | No club/school athletics domain found (Eventlink slug sweep + site probes all 404). |
| Greenfield Central High School (28) | Same: no official athletics page located. |
| Lawrence North High School (35) | Same: no official athletics page located. |
| Decatur Central High School (34) | Official athletics page found, but publishes no XC/TF coach and no AD. |
| Lawrence Central High School (38) | Same as the other unlocated pages: no official athletics page located. |
| Indiana Jr All Stars (36) | Club program, not a school; excluded by contract (school/sport-role contacts only). |
| Center Grove High School (26) | Athletics portal reachable; AD published (row emitted), no XC/TF coach published. |
| Elkhart High School (37) | Athletics portal reachable; AD published (row emitted), no XC/TF coach published. |
| Penn High School (11) | Sport index publishes **Girls** XC only (Michael Clements + email); boys XC / boys TF / girls TF coaches not published. AD page gives name only (no email). |
| Bloomington South High School (13) | Boys XC page lists two "Head Coach" entries (Jill Rensink, Larry Williams); only the first is emitted as head coach, girls XC/TF pages publish no COACHES block. AD row emitted. |
| North Central (Indianapolis) (23) | Girls TF coach Mike Vinson is published only with a yahoo.com address; the name is kept, the personal address is **not** ingested (brief §4). Same rule dropped free-mail addresses for Chesterton boys XC (gmail) and Harrison boys XC (unpublished). |
| Roncalli High School (32) | Athletic-department site publishes no AD; the four per-sport coaches pages give names without emails. |

Free-mail addresses seen on cited pages and deliberately excluded (never written to the file), each verified
by re-fetch: `bconfident@yahoo.com` — North Central girls TF head coach (name kept, address dropped);
`tommoeller27@gmail.com` — Chesterton **Boys Cross Country** head coach Tom Moeller (name kept, address dropped);
`KMStrackcoach@gmail.com` — Harrison *assistant* unified track coach Jacob Daubenmier (assistant + unified,
not a varsity head post). North Central's page additionally carries eight free-mail addresses, all for
soccer/rugby/volleyball/cheer staff outside this lane. All 123 emails delivered in the file are on
school/district or school-hosted domains; the file contains zero addresses on the brief's personal-mail list
(`gmail.com`, `yahoo.com`, `hotmail.com`, `aol.com`, `icloud.com`, `outlook.com`, `live.com`, `msn.com`,
`comcast.net`, `sbcglobal.net`, `att.net`, `verizon.net`), verified by scanning both email columns.

## 4. Corrections made during verification (self-audit trail)

The first verification pass rejected rows, and the fixes are in the delivered file:
- Bloomington South boys XC: my first URL used the label `Cross Country (B. V/JV)` (slash) → 404. Real
  label is `Cross Country (B. V-JV)` (hyphen), taken from the Teams page.
- Crown Point ×4: first URLs guessed team UUID+label pairs → 404. Real team pages are on
  `crownpointathletics.com` (not `sites.eventlink.com`) and their track pages list only an assistant coach;
  the four head coaches + emails come from the school-published *CPHS Coaches Directory* PDF, which now
  serves as the cited source and was re-extracted with `pdftotext -layout` for the check above.
- Columbus North publishes the label "Head Boys/Girls Cross County and Boys/Girls Track Coach" (source
  typo for "Country"); the four rows are the faithful per-sport/-gender split of that one published label.
- A first version attached AD fields to coach rows whose cited URL did not carry them; that was corrected
  so each AD string is proven by that school's AD-row URL.

## 5. Schools from the work list NOT reached

The top-40 request was completed in full (every one of the 40 was opened and dispositioned above).
The remaining **885 schools** of `targets/IN.csv` (ranks 41–925) were **not** worked — the budget stopped at
40. The next wave should resume at rank 41, and should start with the highest-cohort unresolved names
(ranks 22, 27, 28, 35, 38 — 403/no-page cases — plus rank 34's zero-yield athletics page).

## 6. Reproduce

- Build: `python3 /tmp/build_in_fragment.py` → writes `out/coach-fragments/IN.csv`.
- Verify: `python3 /tmp/verify_in_fragment.py` (exit 0 = all rows proven; prints quoted context + mutation controls).
- Import check: `python3 tools/merge_coach_fragments.py --fragments /tmp/in_sandbox/fragments --out /tmp/in_sandbox/merged.csv --report /tmp/in_sandbox/report.md`.
