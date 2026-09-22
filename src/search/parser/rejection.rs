//! Why a response is not a usable search page.
//!
//! The live endpoint answers a same-site scripting POST to `Search.aspx/runSearch` with its JSON
//! envelope, and only with that: an address that does not name the `search.aspx` document is
//! answered `401`, and a crawler-grade client is held by a Cloudflare managed challenge (`403`,
//! `cf-mitigated: challenge`, retained at `research/captures/g1/search-post-20260922T044528Z.body`).
//! A rejection therefore has to be named and fatal. Reading a challenged, redirected, or rowless
//! body as "this search had no results" is the failure that drops a whole query's rows from the
//! census while leaving no trace of why.
//!
//! Every class carries the detail it was raised from, so the recorded failure message names both
//! the class and the cause, and callers branch on the variant — for example
//! `error.downcast_ref::<PageRejection>()` over an `anyhow::Error`.

/// Why a response cannot be read as the live search page.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PageRejection {
    /// The body is not this endpoint's envelope: an HTML document (a managed challenge, a login or
    /// error page), a truncated body, or JSON without the `d.count`/`d.pager`/`d.results` fields.
    #[error("search page is not the endpoint JSON envelope: {detail}")]
    NotJson { detail: String },
    /// The envelope arrived, but `d.results` carries no `<tr>` result row, so it is not the result
    /// markup this parser keys on. A search that genuinely has no usable result is not this class:
    /// it carries rows the queried sport cannot use, which the parser counts in
    /// `SearchPage::skipped` and never turns into candidates.
    #[error("search page carries no result row: {detail}")]
    RowsAbsent { detail: String },
    /// The envelope is this page's but breaks a structural bound of the contract: an unusable
    /// advertised count, a broken pager offset, or markup past the page, row, or row-text bounds.
    #[error("search page violates the response contract: {detail}")]
    Malformed { detail: String },
}

impl PageRejection {
    /// The class as a stable token, without its detail: the part of the message a caller or an
    /// operator can count on.
    #[must_use]
    pub fn class(&self) -> &'static str {
        match self {
            Self::NotJson { .. } => "not_json_envelope",
            Self::RowsAbsent { .. } => "rows_absent",
            Self::Malformed { .. } => "malformed_page",
        }
    }
}
