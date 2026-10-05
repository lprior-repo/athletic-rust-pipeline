# `home_campus` fixtures

Raw captures that the `home_campus` adapter's tests parse offline. Every file is bytes a real
request returned: nothing here is hand-written, prettified or re-serialized, so a parser that
passes here has been tested against the live surface.

## What to capture

- One file per distinct response shape the adapter reads (directory section, details JSON, edge
  rows).
- The edge the parser is most likely to get wrong: an empty roster, a null column, a name with an
  HTML entity, a school with no coaches.
- The smallest capture that still proves the claim it sits next to in a test.

## Form

- Bytes exactly as served (`.html`, `.json`); no reformatting.
- `<surface>_<which>.<ext>`, lowercase, underscores: `directory_section_10.html`,
  `details_19.json`.
- Keep captures anonymized the same way the source publishes them; never add data the source does
  not serve.

## Provenance

`SOURCE.md` next to the captures records the request path, capture date, robots status and the
sha256 of every file. `.md` files are documentation, not test input, and the fixture test skips
them.
