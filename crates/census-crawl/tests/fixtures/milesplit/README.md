# Teams-stage source-error regression excerpts

Bead: `athletic-rust-pipeline-79re`.

These five files retain complete published `<tr>...</tr>` excerpts from the offline revision-1 HTTP captures in `var/national-fresh-20261009-01/http/`. They are not new fetches and are not synthetic school records. The source-error counts and original byte locators are recorded in `var/national-fresh-20261009-01/owed-inventory.json`.

| Fixture | Historical body | Original lines / rejected bytes | Contract |
| --- | --- | --- | --- |
| `dc_team_www.html` | `d1c77de8f4e9a8d488b29631cb52458a.body` | 477–484 / 20013–20310 | The state index publishes a national `www.milesplit.com` link for school 3305; retain the published identity and location. The historical index contains thirteen links of this form. |
| `la_team_www.html` | `cc2666fc48c6bc149fab355d17b9c0e3.body` | 4775–4782 / 172150–172391 | Retain the published national link for school 29171. |
| `tx_team_id_only.html` | `ae4adab765ae7538af00d1068955fdf0.body` | 696–703 / 27727–27990 | School 73074 is published without a URL slug. An absent slug remains empty; it is not invented from the name. |
| `ma_team_foreign.html` | `39b8629c1b2c53389d2e322f7146164d.body` | 4239–4246 / 161986–162236 | School 15373 publishes `MA, ENG`. Record a foreign-directory refusal with its identity and exact location; never project it as a Massachusetts school. |
| `sd_teams_foreign.html` | `ecce46450576a032a363c95d407a94f9.body` | 1896–1911 and 2179–2186 / first two locators 72037–72331 and 72356–72622 | Schools 32427, 32422 and 32432 publish `GRN` country claims. Each remains separately owed with an explicit foreign-directory reason. |

The three accepted link forms are parser repairs, not capacity/chunking exceptions. Foreign rows remain counted in `TeamIndexRead.errors` and `unfinished`, including their byte range; an all-refused excerpt is quarantined rather than certified empty.

## Other jurisdictions in the same bead

HomeCampus CA/NJ replay uses `../home_campus/details_19.json` and `details_3374.json`. Only numeric HomeCampus detail endpoints may register the published `x-requested-with: XMLHttpRequest` and same-origin school-directory Referer. Arbiter NH replay uses `../arbiter/nh_children_p1.body`; numeric public organization children and legacy coaches endpoints may carry their source's bearer authorization. Authorization is represented in cache/capture metadata by its SHA-256 digest, not the credential. Other endpoints and the default browser representation remain restrictive. The MT historical error has the same request-policy cause at organization 4497; no MT children body is claimed as a surviving fixture.

CIAC's `../ciac/directory.html` publishes 190 named school staff tables. The prior parser retained only 184 because it required a nonempty eligible staff list before retaining the school itself. School inventory identity does not require a published coach appointment. The regression retains named schools with an empty staff list without inventing coaches. To compare the original historical representation directly, the CIAC regression accepts an explicit offline `CENSUS_CIAC_REPLAY` body path; this does not open a census store or fetch the network.

The original historical CT body `2f5be98e572c286ccee95b3da32c037c.body` also replays as 190 published / 190 retained schools. Its six schools without named sport staff are Academy of Information Technology & Engineering, ACES Chase Academy, Arts at the Capitol Theater Magnet School, Connecticut International Baccalaureate Academy, USD1 School 1, and USD1 School 2. These schools are retained as inventory evidence, not fabricated coach appointments.

The remaining obligations are not parse failures and must not be certified complete:

- AZ AIA: sixteen sampled city searches, capped at ten results per query, do not qualify an exhaustive inventory. Keep sampled records and the directory obligation, with an explicit blocked full-inventory refusal.
- KS KSHSAA: the qualified `a/` capture is only a letter-a sample. Keep parsed records and the remaining directory obligation, with an explicit full-inventory refusal recorded in a Partial outcome.
- OH OHSAA: with neither configured names nor an association-owned school frontier, the search form does not qualify an exhaustive directory. Record the refusal reason and owed search root in a Partial outcome rather than an unexplained empty outcome.
- IL IHSA: a recorded access cooldown is an explicit blocked outcome; do not bypass it, close deferred schools, or fabricate successful source access.
- UT UHSAA: published school profiles can be parsed, but the appointment season is not published. Retain school evidence and unknown-tenure contact evidence while keeping each appointment obligation explicitly blocked.

Executable regressions: `tests/teams_source_errors.rs`, `tests/teams_source_refusals.rs`, `net::execute::source_header_tests`, and the IHSA recorded-cooldown regression. Acceptance and failing-first command evidence is handed to the closing reviewer for the append-only verification ledger.
