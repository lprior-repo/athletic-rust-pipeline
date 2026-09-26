# Restate upstream docs mirror

Local mirror of the Restate documentation site for offline reference while
working on the census Restate endpoint (`crates/census-service`).

- **Source:** `https://docs.restate.dev/llms.txt` (every linked `*.md` page)
- **Fetch date:** 2026-09-26
- **Pages:** 218 markdown files, unchanged from upstream at fetch time
- **Tool:** `ctd` 0.7.1 (`centralized-docs`) for the searchable index only;
  the files here are the raw upstream bytes, not `ctd` output
- **URL list:** `SOURCES.txt` (one URL per file, same order as fetched)

## Layout

- `*.md` — one file per upstream docs page. Filenames are the page URL with
  `https://docs.restate.dev/` stripped and `/` replaced by `__`
  (e.g. `develop__rust__services.md`).
- `SOURCES.txt` — the 218 source URLs this mirror was fetched from.

The curated, pinned-to-this-repo notes live alongside, not in here:

- `../operations-current.md` — operational surface, fetched 2026-09-22
- `../sdk-current.md` — SDK surface for the pinned `restate-sdk = "=0.12.0"`

Upstream docs track current Restate; the curated notes track what this repo
pins and runs. When they disagree, the curated notes and `Cargo.toml` win.

## Searchable index (local only, not committed)

The full `ctd` index (24k chunks, ~1 GB with `INDEX.json`) lives in the
gitignored `local/restate-docs/` and is rebuilt from the raw files above:

```bash
ctd index ./research/restate/upstream-docs \
  --output ./local/restate-docs \
  --project-name restate \
  --project-desc "Restate docs (docs.restate.dev) local mirror" \
  --llms-txt --with-agents

ctd search "<query>" --index-dir ./local/restate-docs --limit 5
```

To refresh the mirror, re-fetch each URL in `SOURCES.txt` (or re-read the
current upstream `llms.txt`) and re-run the index command.
