# TSSAA reader

Tennessee Secondary School Athletic Association school directory and coach contacts.

Source: `https://portal.tssaa.org/common/directory/` (robots allows `/common/`).
Publishes: 456 high schools, 456 school detail pages, ~14,962 staff rows.
Corpus: 456 `?id=<n>` pages; 430 with staff rows, 26 without.

## Entry points

| Function | Purpose |
|---|---|
| `parse_school_list(text)` | Parse 456 schools from the embedded typeahead array |
| `parse_school_page(text, school_id)` | Parse school entry + Track/XC/AD coach rows |

## Fixtures

`crates/census-crawl/tests/fixtures/tssaa/`

- `directory_id3.html` — Alcoa High School detail page with 35 staffPerson rows
- `directory_id407.html` — Redemption School of Worship with no staff rows
- `robots.txt` — `Allow: /common`, `Disallow: /`
- `SOURCE.md` — byte digests and corpus measurements

## Run

```sh
cargo xtask source-fixture tssaa
cargo xtask source-test tssaa
```

## Before this adapter lands

- [x] `parse_school_list` parses 456 entries from the fixture
- [x] `parse_school_page` parses school entry for id 3 with coach rows
- [x] `parse_school_page` parses id 407 with no coach rows
- [x] Test functions are named `tssaa_*`