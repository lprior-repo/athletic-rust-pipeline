# IA coach-lane report (Class-of-2027 TF/XC census)

Fragment: `out/coach-fragments/IA.csv` — 93 rows, 25 schools, 54 distinct source URLs.
Contract check: `tools/merge_coach_fragments.py` `judge()` returns None (importable) for **all 93 rows**;
sandbox merge with only this fragment: `kept 93 rows from 1 states`, `rejected: 0`.

## 1. Counts
| Measure | Value |
|---|---|
| Work-list entries attempted | 40 (top 40 by `co2027`) |
| Distinct schools among them | 36 (2 are non-school cohort labels, 2 are duplicate school rows) |
| Schools producing >=1 row | 25 |
| Schools with a coach name | 21 |
| Schools with a coach EMAIL | 17 |
| Schools with an AD name and/or email | 12 |
| Rows total | 93 (81 coach rows + 12 director rows) |
| Rows carrying a coach email | 60 |
| Rows carrying an AD email | 12 |

## 2. Per-source tallies (rows written, by host)
| Host | rows |
|---|---|
| `kennedy.crschools.us` | 9 |
| `hawks.ankenyschools.org` | 7 |
| `jaguars.ankenyschools.org` | 6 |
| `www.johnstoncsd.org` | 5 |
| `www.linnmar.k12.ia.us` | 5 |
| `www.pleasval.org` | 5 |
| `www.pellaschools.org` | 5 |
| `city.iowacityschools.org` | 4 |
| `athletics.marshalltown.k12.ia.us` | 4 |
| `senior.dbqschools.org` | 4 |
| `www.ccaschools.org` | 4 |
| `www.dowlingcatholic.org` | 4 |
| `west.iowacityschools.org` | 4 |
| `www.bfschools.org` | 4 |
| `www.j-hawks.com` | 4 |
| `washington.crschools.us` | 4 |
| `www.ameshighathletics.org` | 3 |
| `www.northpolk.org` | 3 |
| `tigers.wdmcs.org` | 2 |
| `hempstead.dbqschools.org` | 2 |
| `sites.google.com` | 1 |
| `wolves.waukeeschools.org` | 1 |
| `www.southeastpolk.org` | 1 |
| `decorah.k12.ia.us` | 1 |
| `www.waterloo.k12.ia.us` | 1 |

## 3. Skips and failures, with reason
| School | Reason |
|---|---|
| `Unattached`, `Live Healthy Iowa Kids Track` | Not schools — cohort labels for unattached/club athletes. No institution to contact. |
| `Waukee NW`, `Valley, Wdm` | Duplicate work-list rows for Waukee Northwest and Valley High School (coverage counted once, under the canonical school). |
| Waukee Northwest | District publishes Activities Department staff only (`northwest.waukeeschools.org/fans/contact/` = principals; `wolves.waukeeschools.org/fans/contact/` = AD). No XC/TF coach page published. Team pages are Bound-hosted (robots `Disallow: /*directory`). |
| Cedar Rapids Prairie | `crprairie.org/athletics/` renders with no coach content; `/staff-directory/` carried no athletics staff; no coach page reachable. |
| Cedar Rapids Jefferson | `jefferson.crschools.us/student-life/athletics/cross-country` and `/track-and-field` contain **zero** occurrences of "head coach" and no `mailto:` — the sport pages are empty shells. |
| Indianola High School | Only 7th/8th-grade coaches are published (Ben Metzger XC; Zack Kaczmarek, Ashley Woosley, Michelle Lester track). No varsity coach or AD published, so nothing that answers a recruiting contact. |
| Bettendorf, Solon, Clinton, Western Dubuque | Apptegy/Thrillshare JS shells: raw HTTP returns a 3037-byte app shell for every path; `/o/<org>/page/athletics` and `/o/<org>/page/coaches-directory` render empty or 404. |
| Davenport Central | `davenportschools.org/about/departments/athletics` publishes no coach contacts; the school's own team pages are Bound-hosted (disallowed). |
| North Scott | Only a district equity-coordinator address is published; no coach/AD directory found. |
| Norwalk High School | No coach or AD email anywhere on `norwalk.k12.ia.us`; `/athletics/` is 404. |
| Decorah, Southeast Polk, Waterloo West, Waukee | AD/activities contact only — no coach names published on any reachable page (SE Polk's 8-page activities handbook PDF carries policy, not a coach roster). |
| Pella, Dubuque Senior, Dubuque Hempstead, Bondurant-Farrar XC, Cedar Rapids Washington XC, Iowa City West GXC/GTF, Ankeny boys T&F | Coach names published **without** a professional address; email cells intentionally left empty rather than guessed. |
| Cedar Falls | Only the men's XC Google Site names a coach (Brett Egan); the girls T&F coach list is published as an image; no district AD page found. |

## 4. Work-list schools I did NOT reach (resume here next wave)
`Waukee Northwest` (2, 123), `Cedar Rapids Prairie` (12, 81), `Cedar Rapids Jefferson` (18, 70),
`Indianola High School` (20, 68), `Bettendorf` (22, 67), `Davenport Central` (23, 67),
`North Scott` (24, 67), `Clinton` (28, 64), `Western Dubuque` (30, 63), `Solon` (34, 61),
`Norwalk High School` (39, 55).
Also unfinished: full coach rosters for `Waukee`/`Southeast Polk`/`Waterloo West`/`Decorah` (AD only) and
emails for the no-email rows above.

## 5. Evidence classes (what a verifier will see)
1. **Literal** (49 rows / 22 URLs): name and email appear verbatim in the raw HTML (e.g. Johnston, Urbandale, Pleasant Valley, Linn-Mar, Marshalltown-rendered aside).
2. **Cloudflare-decoded** (Ankeny High, Ankeny Centennial, Pella AD): emails ship as `/cdn-cgi/l/email-protection#<hex>`; the literal string is absent from raw HTML and only appears after decoding. Verified with the decoder, not by eyeballing.
3. **Render-only** (Marshalltown, 4 rows): `athletics.marshalltown.k12.ia.us/o/athletics/page/coaches-directory` is a JS app; raw HTTP returns a 3037-byte shell. The rows were read from the rendered DOM (headless Chromium) and cross-checked against the page's 43 `mailto:` links.

## 6. Falsification (required check, run on the written CSV)
| Row | String present in cited `source_url` | Mutant rejected |
|---|---|---|
| Johnston / Boys Cross Country / Matt Jaschen | `Head Varsity Coach – **Matt Jaschen**` and `href="mailto:matt.jaschen@johnston.k12.ia.us"` | `matt.jaschen@johnston.k12.ia.gov` absent; `Matt Jaschenn`, `Mateusz Jaschen` absent |
| Urbandale / Boys Track and Field / Joel Jacobs | `<span class="name">Joel Jacobs</span>` … `mailto:jacobsj1@urbandaleschools.com` | `jacobsj1@urbandaleschools.org` absent; `Joel Jacobsen` absent |
| Pleasant Valley / Boys Cross Country / Erik Belby | `<strong>EriK Belby</strong><br /> Head Coach<br /> mailto:belbyerik@pleasval.org` | `belbyerik@pleasval.com` absent; `Erik Belbyy` absent |

## 7. Publication-fidelity notes (no invented data)
- `EriK Belby` (XC card) and `Erik Belby` (track card) are both as published on the Pleasant Valley page.
- Clear Creek-Amana publishes `Ben Robison` for boys track/XC and `Ben Robinson` for girls track with the same address; both spellings were kept as published.
- Cedar Rapids Kennedy lists `Co-Head Coaches: Jacob Ciabatti and BJ White`. Ciabatti keeps `Co-Head Coach`; White's role is recorded as `Co-Head Coach - Girls Track Coach` (the page's own link title) so the importer's `(school, sport, role)` dedupe does not silently drop her.
- Ankeny's boys T&F head coach Jordan Mullen publishes only a `gmail.com` address, which the importer rejects as personal — name kept, email deliberately empty.
- `athletics.marshalltown.k12.ia.us`, `www.pleasval.org` (broken TLS chain, read with `-k`), and `sites.google.com/cfschools.org/...` robots were each checked before fetching.
- Cloudflare/Akamai-independent counts above are from the file as written, not from memory.
