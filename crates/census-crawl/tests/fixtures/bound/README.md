# `bound` fixtures

Staff pages that the `bound` adapter's parser tests and `xtask replay bound` parse offline. Every
file marked as a capture is bytes a real request returned, taken byte-exact from the prototype crawl
cache; nothing was prettified or re-serialized, so a parser that passes here has been tested against
the live surface.

## What to capture

- One staff page per association the adapter serves: Iowa (IGHSAU) and South Dakota (SDHSAA).
- The edges the parser is most likely to get wrong: a page whose staff table has no rows, a title
  that names a mascot rather than the school, and any role text that is not `Head Coach` or
  `Assistant Coach`.
- The smallest capture that still proves the claim it sits next to in a test.

## Form

- Bytes exactly as served (`.html`); no reformatting.
- Keep captures anonymized the same way the source publishes them; never add data the source does
  not serve.

## Provenance

`SOURCE.md` next to the captures records each capture's URL, byte count, SHA-256 and the prototype
cache key it came from. `.md` files are documentation, not replay input, and the replay harness
skips them.
