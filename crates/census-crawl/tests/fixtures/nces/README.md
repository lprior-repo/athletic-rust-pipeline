# `nces` fixtures

Raw captures that the `nces` adapter's tests parse offline. Every file is bytes a real request
returned: nothing here is hand-written, prettified or re-serialized, so a parser that passes here
has been tested against the live surface.

## What to capture

- One file per distinct response shape the adapter reads (index page, detail page, one row type).
- The edge the parser is most likely to get wrong: an empty listing, a missing column, a malformed
  row, a page mid-migration.
- The smallest capture that still proves the claim it sits next to in a test.

## Form

- Bytes exactly as served (`.html`, `.htm`, `.json`, `.csv`, `.txt`); no reformatting.
- `<surface>_<which>.<ext>`, lowercase, underscores: `directory_letter_a.html`,
  `school_org1_abbotsford.html`.
- Keep captures anonymized the same way the source publishes them; never add data the source does
  not serve.

## Provenance

Add a `SOURCE.md` next to the captures when a file needs more than its name to be read later: the
request path, the capture date, and the robots status. `.md` files are documentation, not test
input, and the fixture test skips them.
