# Synthetic five-state NCES parser fixtures

These tiny CSV inputs are **synthetic test fixtures**, not NCES captures, a source census,
membership evidence, or a qualified school denominator. Names, identifiers, addresses, and
contact values are fabricated. They exist only to exercise the existing `parse_ccd` and
`parse_pss` header and row contracts with one public and one private school row in each of
Michigan, Minnesota, Iowa, Illinois, and Colorado.

The fixtures use only the required columns documented by `crates/census-crawl/src/nces/parse.rs`:

- `ccd_synthetic.csv`: `NCESSCH`, `SCH_NAME`, `MSTREET1`, `MCITY`, `MSTATE`, `MZIP`, `MZIP4`,
  `PHONE`, `GSLO`, `GSHI`, and `CHARTER_TEXT`.
- `pss_synthetic.csv`: `PPIN`, `PINST`, `PADDRS`, `PCITY`, `PSTABB`, `PZIP`, `PZIP4`, `PPHONE`,
  and `NUMSTUDS`.

No fixture row claims school membership, sport assignment, competition, or current-year source
coverage. A future integration test may load these files to verify parsing and jurisdiction
preservation; it must retain the synthetic labeling and must not present their rows as census
coverage.
