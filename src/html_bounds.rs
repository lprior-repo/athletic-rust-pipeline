use anyhow::{bail, Context, Result};
use lol_html::{HtmlRewriter, MemorySettings, Settings};

const MAX_HTML_BYTES: usize = 32 * 1024 * 1024;
const PARSER_MEMORY_BYTES: usize = 8 * 1024 * 1024;
const CHUNK_BYTES: usize = 4096;

/// Run streaming HTML extraction with bounded input and parser-owned memory.
/// Captured evidence is bounded separately by each domain adapter.
pub(crate) fn rewrite_bounded<'handlers, 'selectors>(
    source: &str,
    mut settings: Settings<'handlers, 'selectors>,
) -> Result<()> {
    if source.len() > MAX_HTML_BYTES {
        bail!("HTML exceeds streaming parser byte bound");
    }
    settings.memory_settings = MemorySettings {
        preallocated_parsing_buffer_size: CHUNK_BYTES,
        max_allowed_memory_usage: PARSER_MEMORY_BYTES,
    };
    settings.strict = true;
    settings.adjust_charset_on_meta_tag = false;
    // Extraction callbacks retain evidence; rewritten HTML is deliberately not materialized.
    let mut rewriter = HtmlRewriter::new(settings, |_: &[u8]| {});
    for chunk in source.as_bytes().chunks(CHUNK_BYTES) {
        rewriter
            .write(chunk)
            .context("streaming HTML extraction failed")?;
    }
    rewriter
        .end()
        .context("streaming HTML extraction finalization failed")?;
    Ok(())
}
