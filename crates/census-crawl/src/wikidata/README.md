# Wikidata Website Seed Reader

Reads WDQS SPARQL JSON results and produces `SchoolDirectoryEntry` weak entries
labelled `SourceLabel::Wikidata` with parsed `Website` values.

## Entry Points

| Function | Description |
|----------|-------------|
| `parse_sparql(json)` | Parse WDQS SPARQL JSON result into `ReadOutcome` |
| `sparql_query()` | Build the deterministic SPARQL query string |
| `SOURCE_ID` | `"wikidata"` |
| `SPARQL_ENDPOINT` | `"https://query.wikidata.org/sparql"` |

## Mapping Rules

- Required fields: `item` (QID), `itemLabel`, `website` (parsed via `Website::parse`)
- Missing/malformed required field → skip row with ledger entry naming the field
- Optional fields: `state` (mapped via `census_state`), `city` (parsed via `CityName::parse`)
- Unrecognized or absent state → row skipped with `state` in ledger
- Rows are deduplicated by QID; first occurrence wins
- Order is preserved as published; no sorting

## Fixtures

A small captured WDQS JSON result lives in `tests/fixtures/wikidata/`.
To refresh: capture a real WDQS response to `schools.json` and update `SOURCE.md`
with the endpoint, query, capture date, and fixture sha256.

## Limitations

- Only reads JSON; does not fetch from the endpoint
- No pagination support
- No rate limiting