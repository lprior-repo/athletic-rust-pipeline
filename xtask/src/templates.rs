//! The bodies `new-source` writes: one template per generated file.
//!
//! Nothing here touches the filesystem — `scaffold` decides where the files go — and every template
//! that takes a `name` substitutes it into the module doc, `SOURCE_ID`, the README paths and the
//! test filter.

/// `mod.rs`: the adapter entry point, in the shape `census.rs` and the adapter registry call.
pub(crate) fn adapter_module(name: &str) -> String {
    format!(
        r#"//! `{name}` source adapter (scaffold).
//!
//! Fixtures: `crates/census-crawl/tests/fixtures/{name}/`.
//! This adapter's tests: `cargo xtask source-test {name}` (see `xtask/README.md`).

use census_crawl::{{AdapterContext, AdapterReport, CrawlError, CrawlResult}};

pub mod map;
pub mod parse;

/// Evidence/adapter slug stamped into every `SourceRef`.
pub const SOURCE_ID: &str = "{name}";

/// Everything one run of this adapter can be told to do.
#[derive(Debug, Clone, Default)]
pub struct Options {{
    /// Ignore cached HTTP bodies and re-fetch.
    pub refresh: bool,
}}

/// Fetch, parse, append: one `{name}` run.
///
/// Scaffold: nothing is implemented yet. Keep the signature — the adapter registry and `census.rs`
/// call adapters in this shape — hand parsing to [`parse`], canonical mapping to [`map`], and
/// append rows with `sources::append_all`, never by writing keys directly. Adapters never panic:
/// every failure returns a [`CrawlError`] variant, which is what the durable retry policy reads.
pub async fn collect(_ctx: &AdapterContext<'_>, _options: &Options) -> CrawlResult<AdapterReport> {{
    Err(CrawlError::Invariant {{
        detail: "{name} adapter not implemented".to_string(),
    }})
}}
"#
    )
}

/// `parse.rs`: pure parsing plus the fixture-driven test the adapter's fixtures will drive.
pub(crate) fn parse_module(name: &str) -> String {
    format!(
        r#"//! Pure parsing: no I/O, no store access.
//!
//! Everything here takes captured text and returns parsed rows, so the module is fixture-testable
//! and reusable by the durable services.

use anyhow::{{bail, Result}};

/// One row exactly as the source publishes it, before canonical mapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedRow {{
    /// Row label, untouched.
    pub label: String,
}}

/// Parse one captured `{name}` document into rows.
///
/// Scaffold: replace this with the real parser and keep the function free of I/O.
pub fn parse_rows(_document: &str) -> Result<Vec<ParsedRow>> {{
    bail!("{name} parsing not implemented")
}}

#[cfg(test)]
mod tests {{
    use super::{{parse_rows, ParsedRow}};
    use anyhow::{{ensure, Result}};
    use std::path::{{Path, PathBuf}};

    /// Every captured document, so a new fixture is covered the moment it lands.
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
            // Nothing captured yet: this test starts biting with the first fixture file.
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

/// `map.rs`: where parsed rows become canonical entities.
pub(crate) fn map_module() -> String {
    r#"//! Canonical mapping: parsed rows -> canonical entities.
//!
//! Nothing here reads the network, the store or a file: it takes [`super::parse`] output and returns
//! the canonical types from `crate::model`, which keeps it unit-testable and reusable by the durable
//! services.
"#
    .to_string()
}

/// The adapter module's README: purpose, entry points, fixtures, and the commands that drive it.
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

- [ ] real URLs and their robots status written into the module doc
- [ ] parsing and mapping asserted against captured fixtures
- [ ] registered in `census.rs`, or exposed as a `provider {name}` CLI subcommand
- [ ] rows appended with `sources::append_all`, never by writing keys directly
"#
    )
}

/// The fixture directory's README: what to capture, in what form, under what names.
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

#[cfg(test)]
mod tests {
    use super::{adapter_module, parse_module};

    /// The generated adapter presents the shape the registry and `census.rs` call adapters in.
    ///
    /// The template once emitted `anyhow::Result<AdapterReport>`, so a scaffolded source compiled
    /// only after its author rewrote the signature the scaffold exists to settle. The assertion is
    /// on the rendered text because xtask cannot compile a generated adapter (its `crate::`
    /// imports need the crawl crate); the text is therefore the contract of record.
    #[test]
    fn the_adapter_template_declares_the_canonical_collect_shape() {
        let module = adapter_module("example_source");
        assert!(module.contains("pub async fn collect("));
        assert!(module.contains("-> CrawlResult<AdapterReport>"));
        assert!(module.contains("AdapterContext, AdapterReport, CrawlError, CrawlResult"));
        assert!(module.contains("CrawlError::Invariant"));
        assert!(!module.contains("anyhow::"));
    }

    /// Parsing is pure and testable: the generated parse module carries no store, no fetcher and no
    /// `async`, which is what lets its fixture test run without network.
    #[test]
    fn the_parse_template_stays_pure() {
        let module = parse_module("example_source");
        assert!(!module.contains("async fn"));
        assert!(!module.contains("Store"));
        assert!(!module.contains("Fetcher"));
    }

    /// The name substitution reaches every template that takes a name.
    #[test]
    fn every_named_template_carries_the_source_name() {
        for body in [
            adapter_module("example_source"),
            parse_module("example_source"),
        ] {
            assert!(body.contains("example_source"));
        }
    }
}
