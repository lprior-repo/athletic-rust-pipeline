# 46. Event-token mapping: the seven bare-numeric event tokens in the delivered grade-11 workbook

Status: complete — offline file analysis; **zero network requests** were issued for this report (no HTTP
statuses exist to report; every probe below is a local read).
Observed on: 2026-09-20 (America/Chicago). Delivery workbook mtime 2026-09-18 21:27; nav-catalog dump
mtime 2026-09-19 23:14.

Scope of the assignment: close the one open item report 04 raised — the `Events` column of the retained
delivery (`/home/lewis/Downloads/Athletic-2026-US-Boys-Grade11-All-Athletes.xlsx`) contains **7 distinct
bare-numeric tokens** that "no captured artifact explains". This report maps them, with exact row counts,
exact provenance, and the residual unknowns.

---

## Bottom line

The seven tokens are **not event tokens at all**. They are **comma-fragments of distance-medley relay
event shorts**, produced by report 04's *own* tokenizer, which normalises the delivery's `; `-joined
`Events` cell with `.replace(";", ",").split(",")` (`tools/04-rankings-request-model.py:42`). The delivery
itself never contains a bare numeral: splitting the `Events` cell on `;` yields 74 distinct shorts and
**zero** numeric shorts; splitting it on `,` as well yields 79 tokens, of which exactly 14 are phantoms
(7 numerals + 6 `distmedN` prefixes + `mile`). Each numeral is the *leg distance in hundreds of metres* of
a distance-medley relay (`distmed…`) whose short code is comma-delimited by design
(`distmed12,4,8,16` = DMR 4000 m). The event identity to join on is the **whole comma-bearing short**,
never the numeral.

Reproduction is exact and closed: for all seven numerals, the measured occurrence count equals the count
predicted from the legs of the observed medley shorts, with no residual source
(`tools/scratch-46/exact-attribution.py` → `ALL NUMERALS CLOSED: True`).

---

## 1. Token map (deliverable table)

Counts are per athlete row of the delivery (142,705 rows, one row per distinct `AthleteID`).
"Occurrences" = the metric report 04's tokenizer actually produced (see §3.3 for the reconciliation with
its "~11.3k" statement). "Co-occurring event names" = the other comma-fragments present in the *same*
`Events` cell; medley parents are listed with their full comma-bearing short.

| token | athlete rows | occurrences (report-04 metric) | co-occurring event names | mapped meaning | confidence |
|---|---|---|---|---|---|
| `4` | 3,588 | 3,606 | parents: `distmed12,4,8,16` (3,230), `distmed2,2,4,8` (289), `distmed4,4,8,16` (45), `distmed10,2,4,8` (25), `distmed8,2,4,16` (13), `distmed13,4,8,mile` (4); top others: `8` 3,578, `16` 3,279, `800m` 2,421, `1600m` 2,098, `4x800m` 1,528, `3200m` 1,386, `400m` 1,367, `4x400m` 863, `200m` 658, `100m` 450 | **400 m relay leg** (leg 2 or 3 of the parent short) | **high** |
| `8` | 3,617 | 3,636 | parents: `distmed12,4,8,16` (3,230), `distmed2,2,4,8` (289), `distmed4,4,8,16` (45), `distmed4,6,8,10` (37), `distmed10,2,4,8` (25), `distmed4,8,12,12` (5), `distmed13,4,8,mile` (4), `distmed8,8,16,16` (1); top others: `4` 3,578, `16` 3,270, `800m` 2,439, `1600m` 2,107, `4x800m` 1,534, `3200m` 1,387, `400m` 1,376, `4x400m` 865, `200m` 661, `100m` 456 | **800 m relay leg** (leg 2, 3 or 4 of the parent short) | **high** |
| `16` | 3,280 | 3,290 | parents: `distmed12,4,8,16` (3,230), `distmed4,4,8,16` (45), `distmed8,2,4,16` (13), `distmed8,8,16,16` (1, contributes 2 fragments); top others: `4` 3,279, `8` 3,270, `800m` 2,337, `1600m` 2,039, `4x800m` 1,461, `3200m` 1,353, `400m` 1,268, `4x400m` 782, `200m` 537, `1mile` 361 | **1600 m relay leg** (leg 3 or 4) | **high** |
| `2` | 326 | 327 | parents: `distmed2,2,4,8` (289), `distmed10,2,4,8` (25), `distmed8,2,4,16` (13); top others: `4` 326, `8` 316, `sprintmed2248` 133, `200m` 127, `400m` 106, `100m` 103, `800m` 93, `4x400m` 85, `4x800m` 74, `1600m` 73 | **200 m relay leg** (always leg 2) | **high** (n small) |
| `6` | 37 | 37 | parents: `distmed4,6,8,10` (37); top others: `10` 37, `8` 37, `800m` 21, `400m` 15, `1600m` 13, `200m` 9, `4x400m` 9, `100m` 8, `300mh` 8 | **600 m relay leg** (leg 2 of DMR 2800 m) | **high** (n = 37) |
| `10` | 37 | 37 | parents: `distmed4,6,8,10` (37); top others: `6` 37, `8` 37, `800m` 21, `400m` 15, `1600m` 13, `200m` 9, `4x400m` 9, `100m` 8, `300mh` 8 | **1000 m relay leg** (leg 4 of DMR 2800 m) | **high** (n = 37) |
| `12` | 5 | 10 | parents: `distmed4,8,12,12` (5, contributes 2 fragments per row); top others: `8` 5, `800m` 5, `1600m` 3, `4x800m` 3, `3200m` 2, `400m` 2, `16` 1, `200m` 1, `4` 1, `discus` 1, `shot` 1, `sprintmed1124` 1 | **1200 m relay leg** (legs 3 and 4 of DMR 3600 m) | **medium-high** (n = 5 rows) |

Union: **3,627 athlete rows (2.542 % of 142,705)** carry at least one of the seven numerals. No row carries
a numeral that is *not* explained by a medley short: rows with a numeral but no `distmed`/`midmed` parent = **0**.
48 of the 3,627 rows (1.3 %) list a medley short as their *only* event.

### 1.1 The nine medley parents actually present in the delivery

All nine are relays (`r: true`) in the captured 283-event nav catalog (`tools/scratch-04/navinfo-154.json`);
flags `h:false`, `t:"T"`, `m:"M"`, `w:false` on every one. "Legs" is decoded from the short; "check" equals
the nav `name`'s distance.

| delivery short | rows | sole-event rows | nav id | nav name | legs (hundreds of m) | check |
|---|---|---|---|---|---|---|
| `distmed12,4,8,16` | 3,230 | 36 | 40 | DMR 4000m | 1200‑400‑800‑1600 | 4000 m ✓ |
| `distmed2,2,4,8` | 289 | 7 | 615 | DMR 1600m | 200‑200‑400‑800 | 1600 m ✓ |
| `distmed4,4,8,16` | 45 | 1 | 392 | DMR 3200m | 400‑400‑800‑1600 | 3200 m ✓ |
| `distmed4,6,8,10` | 37 | 1 | 559 | DMR 2800m | 400‑600‑800‑1000 | 2800 m ✓ |
| `distmed10,2,4,8` | 25 | 3 | 344 | DMR 2400m | 1000‑200‑400‑800 | 2400 m ✓ |
| `distmed8,2,4,16` | 13 | 0 | 384 | DMR 3000m | 800‑200‑400‑1600 | 3000 m ✓ |
| `distmed4,8,12,12` | 5 | 0 | 388 | DMR 3600m | 400‑800‑1200‑1200 | 3600 m ✓ |
| `distmed13,4,8,mile` | 4 | 0 | 262 | DMR 2.5 Mile | 1300‑400‑800‑mile | 4100 m ≈ 2.55 mi ✓ |
| `distmed8,8,16,16` | 1 | 0 | 386 | DMR 4800m | 800‑800‑1600‑1600 | 4800 m ✓ |

Nav medley shorts **never observed** in the delivery: `distmed10,2,6,16` (id 438, "DMR 4300m"),
`distmed12,8,16,32` (id 396, "DMR 6800m"), `distmed2,4,8,12` (id 448, "DMR 2600m"), `midmed8,4,4,8`
(id 270, "MMR 2400m"), `midmed12,8,2,4` (id 472, "MMR 2600m").

### 1.2 The structural rule that closes the mapping

Comma-fragmenting a medley short does **not** emit the first leg separately — it stays glued to the prefix
(`distmed12,4,8,16` fragments to `distmed12`, `4`, `8`, `16`). Bare numerals therefore only ever come from
legs 2–4, and the number of occurrences per numeral is exactly the sum over source variants of the number
of legs equal to that numeral. (Leg positions in `numeral-provenance-exact.tsv` are 1-based; position 1 is
glued to the prefix and never yields a bare numeral, so a source listed there as `[1, 2]` means only
position 2 produced the token.) Measured vs predicted:

| token | measured occurrences | predicted from legs 2–4 | closed? |
|---|---|---|---|
| `2` | 327 | 327 (289 + 25 + 13) | yes |
| `4` | 3,606 | 3,606 (3,230 + 289 + 45 + 25 + 13 + 4) | yes |
| `6` | 37 | 37 | yes |
| `8` | 3,636 | 3,636 (3,230 + 289 + 45 + 37 + 25 + 5 + 4 + 1) | yes |
| `10` | 37 | 37 | yes |
| `12` | 10 | 10 (5 rows × 2 occurrences) | yes |
| `16` | 3,290 | 3,290 (3,230 + 45 + 13 + 1 row × 2) | yes |

### 1.3 Independent corroboration from the same workbook (not from nav names)

Athletes carrying a medley relay are strongly enriched for the individual event at the leg distance:
`800m` appears in 67.4 % of the 3,617 rows carrying `8` vs 20.2 % of all delivery rows (3.33×);
`1600m` in 62.2 % of `16`-rows vs 17.4 % overall (3.57×); `400m` 38.1 % vs 23.5 % (1.62×);
`200m` 39.0 % vs 32.1 % (1.22×). The two rarest legs have no individual counterpart in the delivery to
compare with (`600m` leg → 0 of 37 rows, individual 600 m occurs in 305 rows overall; `1000m` leg → 0 of 37,
individual 1000 m in 84 rows overall) — consistent with rarity, not contradiction. [INFERENCE] the
enrichment is at least partly selection (medley relays attract distance runners), so it corroborates but
does not by itself prove the leg-distance reading; the nav `name` arithmetic above does that.

### 1.4 False-mapping trap (must not be used)

Bare numerals collide numerically with *unrelated* nav event ids: 2 = `200m`, 4 = `800m`, 6 = `3000m`,
8 = `4x400m`, 10 = `110mh`, 12 = `shot`, 16 = `pv` in the captured boys catalog. Joining a bare numeral to
`EventID` would silently produce seven wrong mappings. There is no case in the delivery where the numeral
identity and the nav-id identity agree on the same event.

### 1.5 Complete co-occurrence lists (every distinct comma-fragment sharing a cell)

Parent fragments are separated out; every other token that ever appears in the same `Events` cell as the
numeral is listed with its row count (self-count omitted). Machine-readable: `numeral-cooccurrence.tsv`.

- **`2`** — parents: `distmed2` 289, `distmed10` 25, `distmed8` 13, `distmed12` 10, `distmed4` 1. Others
  (34): `4` 326, `8` 316, `sprintmed2248` 133, `200m` 127, `400m` 106, `100m` 103, `800m` 93, `4x400m` 85,
  `4x800m` 74, `1600m` 73, `4x100m` 70, `sprintmed1124` 68, `4x200m` 65, `lj` 43, `3200m` 40, `16` 21,
  `110mh` 17, `hj` 17, `300mh` 12, `discus` 11, `110shuttleh` 9, `shot` 8, `4x1600m` 5, `tj` 5, `60m` 4,
  `55shuttleh` 3, `pv` 3, `1500m` 2, `1mile` 2, `400mh` 2, `2ksteeple` 1, `2miles` 1, `3000m` 1, `60mh` 1.
- **`4`** — parents: `distmed12` 3,230, `distmed2` 289, `distmed4` 48, `distmed10` 25, `distmed8` 13,
  `distmed13` 4. Others (54): `8` 3,578, `16` 3,279, `800m` 2,421, `1600m` 2,098, `4x800m` 1,528, `3200m`
  1,386, `400m` 1,367, `4x400m` 863, `200m` 658, `100m` 450, `1mile` 362, `4x1600m` 339, `2` 326,
  `sprintmed2248` 325, `1500m` 283, `lj` 212, `4x200m` 205, `4x100m` 203, `3000m` 190, `sprintmed1124` 149,
  `300mh` 147, `hj` 136, `2miles` 114, `110mh` 104, `pv` 82, `tj` 73, `shot` 66, `discus` 62, `javelin` 53,
  `400mh` 52, `2ksteeple` 51, `600m` 49, `4xmile` 38, `110shuttleh` 24, `3ksteeple` 22, `1000m` 19, `300m` 19,
  `swedish1234` 11, `60m` 8, `mile` 4, `10` 3, `55shuttleh` 3, `6` 3, `decathlon` 3, `100shuttleh` 2,
  `120yshuttleh` 2, `60mh` 2, `sprintmed6248` 2, `100mh` 1, `12` 1, `500m` 1, `70shuttleh` 1,
  `sprintmed4224` 1, `sprintmed880` 1.
- **`6`** — parents: `distmed4` 37, `distmed12` 2. Others (28): `10` 37, `8` 37, `800m` 21, `400m` 15,
  `1600m` 13, `200m` 9, `4x400m` 9, `100m` 8, `300mh` 8, `4x800m` 6, `3200m` 5, `4x200m` 5, `110mh` 4,
  `1500m` 4, `16` 3, `1mile` 3, `4` 3, `discus` 3, `javelin` 3, `shot` 3, `4x100m` 2, `hj` 2, `lj` 2, `pv` 2,
  `400mh` 1, `sprintmed1124` 1, `sprintmed2248` 1, `tj` 1.
- **`8`** — parents: `distmed12` 3,230, `distmed2` 289, `distmed4` 86, `distmed10` 25, `distmed13` 4,
  `distmed8` 4. Others (54): `4` 3,578, `16` 3,270, `800m` 2,439, `1600m` 2,107, `4x800m` 1,534, `3200m`
  1,387, `400m` 1,376, `4x400m` 865, `200m` 661, `100m` 456, `1mile` 365, `4x1600m` 338, `sprintmed2248` 325,
  `2` 316, `1500m` 287, `lj` 212, `4x200m` 206, `4x100m` 202, `3000m` 190, `300mh` 153, `sprintmed1124` 149,
  `hj` 137, `2miles` 114, `110mh` 107, `pv` 83, `tj` 74, `shot` 70, `discus` 66, `javelin` 56, `400mh` 53,
  `2ksteeple` 51, `600m` 49, `4xmile` 38, `10` 37, `6` 37, `110shuttleh` 22, `3ksteeple` 22, `1000m` 19,
  `300m` 19, `swedish1234` 11, `60m` 8, `12` 5, `mile` 4, `55shuttleh` 3, `decathlon` 3, `100shuttleh` 2,
  `120yshuttleh` 2, `60mh` 2, `sprintmed6248` 2, `100mh` 1, `500m` 1, `70shuttleh` 1, `sprintmed4224` 1,
  `sprintmed880` 1.
- **`10`** — parents: `distmed4` 37, `distmed12` 2. Others (28): `6` 37, `8` 37, `800m` 21, `400m` 15,
  `1600m` 13, `200m` 9, `4x400m` 9, `100m` 8, `300mh` 8, `4x800m` 6, `3200m` 5, `4x200m` 5, `110mh` 4,
  `1500m` 4, `16` 3, `1mile` 3, `4` 3, `discus` 3, `javelin` 3, `shot` 3, `4x100m` 2, `hj` 2, `lj` 2, `pv` 2,
  `400mh` 1, `sprintmed1124` 1, `sprintmed2248` 1, `tj` 1.
- **`12`** — parents: `distmed4` 5, `distmed12` 1. Others (12): `8` 5, `800m` 5, `1600m` 3, `4x800m` 3,
  `3200m` 2, `400m` 2, `16` 1, `200m` 1, `4` 1, `discus` 1, `shot` 1, `sprintmed1124` 1.
- **`16`** — parents: `distmed12` 3,230, `distmed4` 48, `distmed8` 14, `distmed2` 6, `distmed10` 3. Others
  (51): `4` 3,279, `8` 3,270, `800m` 2,337, `1600m` 2,039, `4x800m` 1,461, `3200m` 1,353, `400m` 1,268,
  `4x400m` 782, `200m` 537, `1mile` 361, `100m` 350, `4x1600m` 336, `1500m` 282, `sprintmed2248` 195,
  `3000m` 189, `lj` 173, `4x200m` 142, `300mh` 137, `4x100m` 136, `hj` 121, `2miles` 113, `110mh` 88,
  `sprintmed1124` 84, `pv` 80, `tj` 69, `shot` 58, `javelin` 53, `discus` 51, `2ksteeple` 50, `400mh` 50,
  `600m` 49, `4xmile` 38, `3ksteeple` 22, `2` 21, `1000m` 19, `300m` 19, `110shuttleh` 17, `swedish1234` 11,
  `60m` 4, `10` 3, `6` 3, `decathlon` 3, `100shuttleh` 2, `120yshuttleh` 2, `sprintmed6248` 2, `100mh` 1,
  `12` 1, `500m` 1, `60mh` 1, `70shuttleh` 1, `sprintmed4224` 1.

Note: parents in these lists are *fragments* (`distmed12`), which is why a parent can appear for a numeral
it did not produce (§1.2 co-present-only parents; e.g. `distmed12` beside `6` in 2 rows where the `6` came
from `distmed4,6,8,10`).

---

## 2. What remains unknown

- **Report 04's "~11.3k athlete-rows" is unreconciled and (as stated) wrong for any convention tested.**
  The affected population is **3,627 athlete rows**; the sum of per-token *occurrences* is 10,943 and the
  sum of per-token *distinct-row* counts is 10,890. The 10,943 figure is exactly what report 04's own
  aggregate artifact records (`tools/scratch-04/all-athletes-agg.json` → `ev_total`:
  `4:3606, 8:3636, 16:3290, 2:327, 6:37, 10:37, 12:10`), so the likely origin is that the "~11.3k" sentence
  describes token occurrences, not athlete rows. No source in this campaign can confirm which number the
  author intended; treated here as an unreproducible statement, not a data fact.
- **The producer of the delivered `Events` column was not located.** The read-only repo contains no code
  that emits `Events`, `AllAthletes`, or a `"; "`-join of event tokens (`grep` over `src/` for `"Events"`,
  `AllAthletes`, `join("; ` returned nothing), so the exporter is outside the repo and was not found in
  `/home/lewis/Downloads`. Consequence: it is unverified whether any *other* export build could emit a
  genuinely bare numeral, or a comma-separated event list instead of `; `-separated. In this delivery it
  does not.
- **Relay membership is not derivable from this column.** A medley short proves the athlete has a medley
  ranking row; it does not say which leg the athlete ran, nor the other three members (report 04 §risk 2
  covers the repo's roster-join rule and its 15,724 unresolved roster results).
- **Leg order is inferred from convention, not from a source row.** `distmed12,4,8,16` is read as
  1200‑400‑800‑1600 (the classic US DMR) because the nav name's total and the leg sum agree; the source
  never states the leg order as an ordered roster. Marked [INFERENCE] where order matters.
- **One nav catalog name is internally inconsistent** (not present in this delivery): `distmed10,2,6,16`
  is named "DMR 4300m" while its legs sum to 3400 m. This does not affect any delivered row, but it means
  nav `name` strings are not a safe source of truth on their own — the leg-sum check must be applied.
- **Live Athletic.net confirmation is out of scope by rule** (no requests to `*.athletic.net` hosts were
  permitted for this assignment) and was not attempted. The mapping rests on the captured nav catalog
  (HAR1[154] dump) plus the delivery's own internal evidence.
- **Season/kind of the delivery is not self-describing**: the workbook has no season or indoor/outdoor
  column; "2026 outdoor, national boys, grade 11" is taken from the campaign's established account of this
  artifact (report 04: national `divListId` 168416) and from the filename, not from a row in the file.

---

## 3. Mechanism (how the phantom tokens arise and how to avoid them)

### 3.1 Correct tokenization (delivery semantics)

The `Events` cell is a `; `-joined list of event shorts, e.g.
`100m; 200m; 400m; 4x200m; 4x100m; lj` and `800m; 4x800m; distmed4,8,12,12; 3200m; 1600m`.
Splitting on `;` only:
- 142,705 data rows, all with a non-empty `Events` cell;
- **397,863** event-token occurrences;
- **74** distinct event shorts;
- **0** shorts that are entirely numeric.

### 3.2 Report 04's tokenizer and the 14 phantom tokens

`tools/04-rankings-request-model.py:42` does `(r[5] or "").replace(";", ",").split(",")`. Re-running that
exactly on the same column yields **79** distinct tokens — the number report 04 published — because the
nine observed medley shorts shatter into 14 distinct phantom tokens. The 14 tokens that exist only under
this tokenizer:

| phantom token | rows | origin |
|---|---|---|
| `4`, `8`, `16`, `2`, `6`, `10`, `12` | 3,588 / 3,617 / 3,280 / 326 / 37 / 37 / 5 | medley legs (this report's subject) |
| `distmed12`, `distmed2`, `distmed4`, `distmed8`, `distmed10`, `distmed13` | 3,230 / 289 / 86 / 14 / 25 / 4 | medley prefix + first leg, glued by construction |
| `mile` | 4 | the word leg of `distmed13,4,8,mile` (imperial twin of the `16` leg) |

So 74 real shorts − 9 medley shorts + 14 phantom tokens = 79. No other comma-bearing short exists in the
delivery: the safe rule is "`sprintmed…`/`swedish…` are comma-free; only `distmed…` (and nav's `midmed…`,
absent here) carry commas."

### 3.3 Reconciliation of report 04's "~11.3k"

| convention | value |
|---|---|
| union of athlete rows carrying ≥1 numeral | **3,627** |
| sum over the 7 numerals of distinct-row counts | 10,890 |
| sum over the 7 numerals of occurrence counts (report-04 tool) | **10,943** (= report 04's own `ev_total` for these keys) |
| sum of `Result Count` over the 3,627 rows | 53,989 |

### 3.4 Why the production pipeline is not at risk from this artifact

The repo's read-only source shows event families are matched **by prefix on the full short**
(`src/runtime/rankings/catalog.rs:217-218`: `"sprint medley"` → `short.starts_with("sprintmed")`,
`"distance medley"` → `short.starts_with("distmed")`; declared at `src/runtime/rankings/types.rs:185-186`),
and the variant key is the whole short. No comma-splitting of event strings exists in `src/`
(`grep` for `split(',')` and `replace(";", ",")` over `src/` returned no matches). The phantom tokens are
therefore confined to the research tool that report 04 used, and to any *downstream consumer of the
workbook* that comma-splits the `Events` cell — which is the real, non-hypothetical exposure this report
closes.

---

## 4. Brief sections

### Source

Provider for this analysis is not a website: the two local artifacts are
(a) the retained delivery workbook `/home/lewis/Downloads/Athletic-2026-US-Boys-Grade11-All-Athletes.xlsx`
(10,147,760 bytes, one sheet `All Athletes`, 142,705 data rows, columns
`AthleteID | Names | GradeID | Teams | States | Events | Result Count | Other Observed Grades | Identity Match Status`),
and (b) the captured Athletic.net nav catalog dump
`tools/scratch-04/navinfo-154.json` (75,213 bytes, from HAR1[154] `GetNavInfo`, 283 events).
No HTTP request was made for this assignment.

### Coverage

142,705 athlete rows, one per distinct `AthleteID`, all `GradeID = 11` (the ranking filter is server-side);
60 distinct state tokens, 41,150 rows (28.8 %) touching at least one of the 12 campaign states; 74 distinct
event shorts covering track, field, relays, multi-events and both metric and imperial variants; medley
relays come from 9 distinct distance-medley variants. The workbook is a *rankings* projection (best mark per
athlete per event), so an athlete appears only for events with a ranked mark. Indoor/outdoor is not a column.

### Enumeration

Enumerating the tokens is a single local pass; no requests and no pagination:

```sh
python3 tools/scratch-46/parse-events.py        # semicolon-only tokenization: 74 shorts, 0 numerals
python3 tools/scratch-46/numeral-provenance.py  # report-04 tokenization: 79 tokens, 14 phantom
python3 tools/scratch-46/exact-attribution.py   # provenance + closure proof per numeral
```

Rule for consumers: split the `Events` cell on `"; "` (or on `;`), **never on `,`**; use the resulting
whole short as the join key (`distmed12,4,8,16`, not `distmed12` and not `4`). To enumerate medley
variants, filter shorts matching `^dist(med)`/`^midmed` and read their legs from the short itself.

### Stable identifiers

The stable event identifier in this artifact is the **event short string**. For medley relays that
identifier contains commas by design and maps 1:1 to a nav event id (40, 615, 392, 559, 344, 384, 388, 262,
386 for the nine observed; 438, 396, 448, 270, 472 for the five nav variants not observed). The seven
numerals are **not** identifiers of anything; they are leg distances and they collide numerically with
unrelated nav ids (§1.4). Athlete identity is `AthleteID` (142,705 distinct, 1:1 with rows).

### Athletic.net leverage

Zero source requests are needed for this mapping, so it changes no coverage estimate. Its leverage is
*preservation*: it prevents a consumer from spending Athletic.net requests to "resolve" seven event
families that do not exist (they are the same nine medley variants already delivered), and it prevents a
mis-join that would attach medley legs to `200m`/`800m`/`3000m`/`4x400m`/`110mh`/`shot`/`pv` nav ids.

### Athlete evidence

For the 3,627 affected rows the `Events` cell proves only *membership in an event short*; there is no mark,
no leg, no relay roster, no date, no meet, no PR — nothing beyond the event list and the row-level
`Result Count`. 48 of those rows list nothing but a medley short, i.e. their entire recorded event set is a
relay (an athlete whose individual marks were not ranked/captured).

### Recruiting information

Not applicable / not observable. The workbook carries no coach, athletic-director, contact, school-website,
or school-address data.

### Result evidence

Not applicable to this artifact beyond row counting: the workbook has no `ResultID`, mark, wind, heat,
round, place, timing method, or date column. The only result-adjacent field is `Result Count` (per athlete);
summed over the 3,627 medley rows it is 53,989 and does **not** reconcile report 04's "~11.3k".

### Incremental use

A weekly refresh needs no re-derivation of this mapping: the artifact is a point-in-time export. The
increment rule for a consumer is content-hash the workbook and re-run the single pass; if the token set
changes, the only mutations possible are (a) a new medley short appears → decode its legs from the short and
confirm the name/leg-sum check against the nav catalog, or (b) a nav variant is retired. Both are detectable
by comparing the 74-short set, not by re-walking events.

### Access characteristics

Classify the delivery as **static XLSX (offline)**; the nav catalog as **public structured JSON captured
from a browser session** (HAR1[154]). No documented API, no authentication, and no rate limit applies to
this assignment because no host was contacted. Parsing needs no third-party dependency on this host
(`openpyxl`/`pyarrow` are absent; `zipfile` + `xml.etree` handle the 58 MB `sheet1.xml` in ~2 s streaming).

### Recommendation

**VALIDATION.** Not a discovery source and not a seed: this is a consumer-correctness artifact for the
retained delivery. Expected marginal coverage: none in athletes or meets; it removes a whole class of
downstream join errors (7 fabricated event families, 3,627 athlete rows) and defines the exact token
contract a workbook consumer must implement (`;`-split, comma-bearing medley shorts kept whole).

---

### Evidence appendix

All entries are local, read-only operations; **no network request was issued**, so the HTTP-status column
records "n/a (offline)" and "method" is the exact local command. Timestamps are America/Chicago
(UTC−05:00).

| URL (local path) | method | HTTP status | what it proved | timestamp |
|---|---|---|---|---|
| `/home/lewis/Downloads/Athletic-2026-US-Boys-Grade11-All-Athletes.xlsx` (unzip listing, workbook.xml) | `unzip -l` / `unzip -p … xl/workbook.xml` | n/a (offline) | 10,147,760-byte workbook, single sheet `All Athletes`, no `sharedStrings.xml` (inline strings) | 2026-09-20T09:05-05:00 |
| same, header + rows 2–4 (cols 1–9) | `python3` stdlib `zipfile`+`ElementTree` (scratch-46 first peek) | n/a (offline) | Columns `AthleteID\|Names\|GradeID\|Teams\|States\|Events\|Result Count\|Other Observed Grades\|Identity Match Status`; `Events` is a `; `-joined short list | 2026-09-20T09:05-05:00 |
| same, whole column `Events` | `tools/scratch-46/parse-events.py` | n/a (offline) | 142,705 data rows; 74 distinct shorts under `;`-split; **0** numeric shorts; `distmed…` shorts carry commas | 2026-09-20T09:06-05:00 |
| same | `tools/scratch-46/numeral-provenance.py` | n/a (offline) | Report-04 tokenizer reproduces **79** distinct tokens; exactly **7** bare numerals `{2,4,6,8,10,12,16}`; 0 numeral-rows without a medley parent; numerals collide with unrelated nav ids | 2026-09-20T09:06-05:00 |
| same | `tools/scratch-46/reconcile.py` | n/a (offline) | Distinct numeral rows 3,588/3,617/3,280/326/37/37/5; occurrences 3,606/3,636/3,290/327/37/37/10; union 3,627 rows; occurrence sum 10,943; complete co-occurrence matrix | 2026-09-20T09:07-05:00 |
| same | `tools/scratch-46/parent-stats.py` | n/a (offline) | 142,705 rows / 397,863 token occurrences; 9 medley parents with rows 3,230/289/45/37/25/13/5/4/1; 48 sole-event rows; 5 nav medley variants never observed | 2026-09-20T09:07-05:00 |
| same | `tools/scratch-46/exact-attribution.py` | n/a (offline) | Glued-first-leg rule; predicted-without-residual occurrence counts equal measured for all 7 numerals → `ALL NUMERALS CLOSED: True`; per-token co-present-only parents listed | 2026-09-20T09:09-05:00 |
| same, `GradeID`/`States` columns | `python3` stdlib scan (scratch-46) | n/a (offline) | All 142,705 rows `GradeID 11`; 142,705 distinct `AthleteID`; 60 state tokens; 41,150 rows touch the 12 campaign states; `Other Observed Grades` non-empty on 499 rows | 2026-09-20T09:10-05:00 |
| `tools/scratch-04/navinfo-154.json` (captured `GetNavInfo`, HAR1[154]) | `python3` JSON walk | n/a (offline) | 283 nav events; **0** numeric shorts; 12 `distmed…` + 2 `midmed…` entries with `id`, `name`, `short`, `r:true`; name-vs-leg-sum check passes 13/14 (exception `distmed10,2,6,16` "DMR 4300m" vs 3400 m) | 2026-09-20T09:06-05:00 |
| `tools/04-rankings-request-model.py` (report 04's model tool) | `grep -n "split\\|replace"` | n/a (offline) | Line 42: `(r[5] or "").replace(";", ",").split(",")` — the tokenizer that fabricates the 7 numerals and the other 7 phantom tokens | 2026-09-20T09:05-05:00 |
| `tools/scratch-04/all-athletes-agg.json` (report 04's own aggregate) | `python3` JSON read | n/a (offline) | 79 `ev_total` keys; numeral keys `4:3606, 8:3636, 16:3290, 2:327, 6:37, 10:37, 12:10` — identical to this report's occurrence counts, summing to 10,943 (not ~11.3k) | 2026-09-20T09:08-05:00 |
| `/home/lewis/src/ad-law-scrape/athletic-rust-pipeline/src/runtime/rankings/catalog.rs` | `read` + `grep` (read-only) | n/a (offline) | Families matched by `short.starts_with("sprintmed"/"distmed")` (lines 217–218); no comma-splitting of event strings anywhere in `src/` (`split(',')` and `replace(";", ",")` → no matches) | 2026-09-20T09:09-05:00 |
| `/home/lewis/src/ad-law-scrape/athletic-rust-pipeline/{README,HANDOFF,SCOPE}.md` | `grep` (read-only) | n/a (offline) | The 142,705-athlete `AllAthletes` delivery is a rankings artifact separate from identity matching; no repo code emits its `Events` column | 2026-09-20T09:08-05:00 |
| raw captures (copies of script outputs) | `research/midwest/evidence/gaps/46/{token-counts,report04-token-counts,numeral-summary,numeral-cooccurrence,numeral-provenance-exact,numeral-reconciliation}.tsv` | n/a (offline) | Machine-readable backing for every count in §1 and §3 | 2026-09-20T09:09-05:00 |
