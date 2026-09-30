# `state_ed` source adapter (scaffold)

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
shown in `SOURCE_ADAPTER_GUIDE.md`; the scaffold starts with the fixture-driven test inside `parse.rs`.

## Fixtures

Captured documents live in `crates/census-crawl/tests/fixtures/state_ed/`; that directory's README
says what to capture and how to name it. Tests must run offline from those files.

## Commands

```bash
cargo xtask source-fixture state_ed   # what is captured for this source
cargo xtask source-test state_ed      # this adapter's tests
```

`cargo xtask` is the alias in `.cargo/config.toml`; `cargo run -p xtask -- source-test state_ed` is the
same command spelled out. Both run nextest with a `test(state_ed)` filter over the crate.

## Before this adapter lands

- [ ] real URLs and their robots status written into this README
- [ ] parsing and mapping asserted against captured fixtures
- [ ] registered in `census.rs`, or exposed as a `provider state_ed` CLI subcommand
- [ ] rows and journal entries appended through the page `AdapterContext::write_batch` hands out,
      never by writing keys directly
