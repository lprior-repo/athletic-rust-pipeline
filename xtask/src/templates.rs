
pub(crate) fn adapter_module(name: &str) -> String {
    format!(
        r#"use census_crawl::{{AdapterContext, AdapterReport, CrawlError, CrawlResult}};

pub mod map;
pub mod parse;

pub const SOURCE_ID: &str = "{name}";

#[derive(Debug, Clone, Default)]
pub struct Options {{
    pub refresh: bool,
}}

pub async fn collect(_ctx: &AdapterContext<'_>, _options: &Options) -> CrawlResult<AdapterReport> {{
    Err(CrawlError::Invariant {{
        detail: "{name} adapter not implemented".to_string(),
    }})
}}
"#
    )
}

pub(crate) fn parse_module(name: &str) -> String {
    format!(
        r#"use anyhow::{{bail, Result}};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedRow {{
    pub label: String,
}}

pub fn parse_rows(_document: &str) -> Result<Vec<ParsedRow>> {{
    bail!("{name} parsing not implemented")
}}

#[cfg(test)]
mod tests {{
    use super::{{parse_rows, ParsedRow}};
    use anyhow::{{ensure, Result}};
    use std::path::{{Path, PathBuf}};

    fn fixtures() -> Vec<PathBuf> {{
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/{name}");
        let Ok(entries) = std::fs::read_dir(&dir) else {{
            return Vec::new();
        }};
        let mut files: Vec<PathBuf> = entries
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.is_file() && path.extension().and_then(|ext| ext.to_str()) != Some("md"))
            .collect();
        files.sort();
        files
    }}

    #[test]
    fn parses_every_captured_fixture() -> Result<()> {{
        let files = fixtures();
        if files.is_empty() {{
            return Ok(());
        }}
        for file in files {{
            let document = std::fs::read_to_string(&file)?;
            let rows: Vec<ParsedRow> = parse_rows(&document)?;
            ensure!(!rows.is_empty(), "{{}} parsed to no rows", file.display());
        }}
        Ok(())
    }}
}}
"#
    )
}

pub(crate) fn map_module() -> String {
    String::new()
}

pub(crate) fn adapter_readme(name: &str) -> String {
    format!(
        r#"# `{name}` source adapter (scaffold)

## Purpose

One paragraph on the public surface this adapter collects, the authority or association it belongs
to, and the canonical entities it feeds (schools, coaches, athletes, results). Record the real URLs
and their `robots.txt` status here once the walk exists.

## Entry points

| Item | What it is |
| --- | --- |
| `mod.rs` `collect` | the adapter body, `(&AdapterContext, &Options) -> Result<AdapterReport>`, called by `census.rs` and the durable services |
| `mod.rs` `SOURCE_ID` | the evidence slug stamped into every `SourceRef` |
| `parse.rs` | pure parsing: captured text in, parsed rows out, no I/O |
| `map.rs` | parsed rows to canonical entities from `crate::model` |

When an adapter's tests outgrow `parse.rs`, its `#[cfg(test)]` module moves to the sibling `tests.rs`
shown in `docs/DECOMPOSITION.md`; the scaffold starts with the fixture-driven test inside `parse.rs`.

## Fixtures

Captured documents live in `crates/census-crawl/tests/fixtures/{name}/`; that directory's README
says what to capture and how to name it. Tests must run offline from those files.

## Commands

```bash
cargo xtask source-fixture {name}   # what is captured for this source
cargo xtask source-test {name}      # this adapter's tests
```

`cargo xtask` is the alias in `.cargo/config.toml`; `cargo run -p xtask -- source-test {name}` is the
same command spelled out. Both run nextest with a `test({name})` filter over the crate.

## Before this adapter lands

- [ ] real URLs and their robots status written into this README
- [ ] parsing and mapping asserted against captured fixtures
- [ ] registered in `census.rs`, or exposed as a `provider {name}` CLI subcommand
- [ ] rows and journal entries appended through the page `AdapterContext::write_batch` hands out,
      never by writing keys directly
"#
    )
}

pub(crate) fn fixture_readme(name: &str) -> String {
    format!(
        r#"# `{name}` fixtures

Raw captures that the `{name}` adapter's tests parse offline. Every file is bytes a real request
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
"#
    )
}

