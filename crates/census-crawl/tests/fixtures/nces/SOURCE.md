# `nces` captures

Both files are byte-exact windows cut from artifacts NCES published; nothing here is reformatted,
prettified or re-serialized. Neither reader makes a request, so no robots verdict applies to these
captures.

|Fixture|Window|Digests|
|---|---|---|
|`ccd_sch_029_2526_head.csv`|`ccd_sch_029_2526_w_0a_050626.csv` inside `https://nces.ed.gov/ccd/data/zip/ccd_sch_029_2526_w_0a_050626.zip`, lines 1–1600 (header + 1,599 rows: every Alabama row, 1,557, and the first 42 Alaska rows)|fixture `e01af083baa57b60f37571f9607aa8ebe9b225712563ec9f10fd8809fab661c8`; full member `d1473136285b5994b73a1a8b640757811eb81e0ae770953bcf915ee8c422386e` (102,103 lines, 65 columns); zip `b5d8dc341ecdc85549ffa9ba5accf4c4ed4f2a7c4a78a5115bbd281ef3e23bc8` (41,054,983 bytes decompressed, zip entry dated 2026-07-01)|
|`pss2324_pu_head.csv`|`pss2324_pu.csv` (PSS 2023-24 public-use file, downloaded by the operator; local copy `/home/lewis/src/ad-law-scrape/data/nces/pss/pss2324_pu.csv`), lines 1–400 (header + 399 rows: 374 in-scope, 25 Alaska)|fixture `2df29d5c9f669b698ac8b7471297ecc0ac1c9fbc103a1b4914462a7d61fabca6`; full file `14a2f9e600a492940fd57646792b4b5163ea9d03b8015a7df1135066bcec3b8b` (22,511 lines, 359 columns)|

Captured 2026-09-28 into `/home/lewis/src/ad-law-scrape/data/nces/`; the CCD zip into
`census-prototype/data/ccd_sch_029_2526.zip`. The windows were cut with `sed -n '1,1600p'` and
`sed -n '1,400p'` respectively.

Facts the fixtures pin: the CCD rows split into exactly 65 comma-separated fields with no quoted
commas (10 lines carry a literal `"` inside an unquoted field); the PSS window contains quoted fields
whose commas are part of the value (`A1970033` name, `A0100219` street).
