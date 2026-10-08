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
- Unrecognized or malformed published state → row skipped with `state` in the ledger.
  Absent/blank state remains unknown on a retained weak row; it is not inferred.
- Malformed optional city → retain the row without city and record a note.
- Rows are deduplicated by QID; first occurrence wins and later occurrences are noted.
- Order is preserved as published; no sorting.
- A malformed binding object is skipped independently; later valid bindings remain readable.
  Syntax/envelope failure or a resource refusal retains the already accepted prefix plus
  `ReadOutcome::unfinished`, rather than discarding prior schools.

## Admission and completeness

The reader checks an 8 MiB source-body limit before decoding, a 16 KiB raw-binding
limit before decoding owned fields, a 1,024-byte decoded-field limit before copying
strings, and 20,000 source rows/QIDs. QID storage and string copies use fallible
reservations. `ReadOutcome` independently enforces bounded entry/issue counts, issue
details and conservative aggregate retained bytes. Resource failures propagate;
they are not recast as a malformed row or a successful empty result.

A source response with `bindings: []` is accepted only with the required SPARQL
head variables and results envelope, but is not a completeness certificate.
The generated query has a `LIMIT 10000` and no exhausted pagination protocol.
Consequently the reader never calls `finish`: clean responses remain `Unknown`,
and retained skips/notes/admission stops are `Partial`.

## Fixtures

A small captured WDQS JSON result lives in `tests/fixtures/wikidata/`.
To refresh: capture a real WDQS response to `schools.json` and update `SOURCE.md`
with the endpoint, query, capture date, and fixture sha256.

## Limitations

- Only reads JSON; does not fetch from the endpoint
- No pagination support
- No rate limiting