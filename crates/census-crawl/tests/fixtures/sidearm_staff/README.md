# `sidearm_staff` fixtures

Raw captures that the `sidearm_staff` adapter's tests parse offline. Every file is bytes a real
request returned: nothing here is hand-written, prettified or re-serialized, so a parser that
passes here has been tested against the live surface.

## What to capture

- The staff directory body the adapter reads, exactly as served.
- The edge the parser is most likely to get wrong: a member with blank email halves, a row whose
  category differs from its own sport, or a director title that is not the exact published form.
- The smallest capture that still proves the claim it sits next to in a test.

## Form

- Bytes exactly as served (`.html`); no reformatting.
- `<host>__<surface>__<which>.<ext>`, lowercase, underscores:
  `gomats.org__staff-directory__full.html`.
- Keep captures anonymized the same way the source publishes them; never add data the source does
  not serve.

## Provenance

`SOURCE.md` next to the captures records the request path, capture date, robots status and the
sha256 of every file. `.md` files are documentation, not test input, and the fixture test skips
them.
