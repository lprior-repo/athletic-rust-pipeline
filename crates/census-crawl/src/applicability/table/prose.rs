pub(super) const ATHLETICLIVE_EVIDENCE: &str = "`athleticlive + timers` rows for the twelve target states, `verified` in eleven of them: MN 6 blob \
                   requests -> 959 rows (242/242 Juniors at 2025 state XC) [10], ND 4 state XC races plus RTDB standings \
                   carrying `y` [25], MO 10 HTML files -> 3,393 rows [22], WI PrimeTime API 319 meets 2025 [08], IA \
                   per-event JSON + `search.athletic.live` ES [12]. AL (`xt.anet.live` via Xpress Timing) from §1b. The \
                   adapter reads harvest/capture artifacts and contacts no host (`admission.origin` = `local-artifact`).";

pub(super) const ATHLETICLIVE_REFUSAL: &str = "Hawaii is the one other jurisdiction with a §1b capture (`live.athletic.net/meets/58854`) and it is \
                  outside the census scope, so a run does not plan it; SD is `verified (absence)`: regular-season \
                  results are not published by any source, so its meets reach the graph through Athletic.net alone [26]. \
                  Every remaining jurisdiction has no AthleticLIVE harvest or capture in the corpus, so there is nothing \
                  for the adapter to read.";

pub(super) const NCES_EVIDENCE: &str = "The two national universes the census addresses are built from: the CCD school file \
                   `ccd_sch_029_2526_w_0a_050626.csv` (102,102 rows over 65 header-named columns inside \
                   `ccd_sch_029_2526.zip`, sha256 `b5d8dc34…`; every row carries `SCH_NAME`, a 12-digit `NCESSCH`, a \
                   mailing or location street, a five-digit ZIP and a two-letter state) and the PSS public-use file \
                   `pss2324_pu.csv` (22,510 rows over 359 columns, sha256 `14a2f9e6…`; every row carries `PPIN`, \
                   `PINST`, `PADDRS`, `PCITY`, `PSTABB`, `PZIP` and `LATITUDE24`/`LONGITUDE24`). Both readers key on \
                   header names and refuse a file that lacks them. The reader is an artifact reader \
                   (`admission.origin` = `local-artifact`): the operator supplies the extracted CSV, and the CCD ZIP is \
                   decompressed outside the process.";

pub(super) const NCES_REFUSAL: &str = "Alaska and Hawaii are outside the census scope, and the territories the files carry (PR, GU, VI, AS, \
                  MP) are not census jurisdictions, so those rows are skipped with their line and field recorded. An \
                  in-scope row without a usable name or a 12-digit NCES id is skipped the same way. Nothing else is \
                  excluded: a closed or future school stays in the corpus as the file publishes it.";
