//! Streaming `lol_html` capture for search result rows, athlete links, and
//! pager offsets across unbounded result markup.

mod bounds;
mod capture;
mod link;
mod row;

use super::{SearchCandidate, SearchIssue, SearchQuery};
use crate::domain::{
    evidence::{EvidenceRef, Sport},
    identity::{AthleteId, EvidenceDigest, ProfileUrl},
};
use crate::html_bounds::rewrite_bounded;
use anyhow::Result;
use capture::{add_link_handlers, add_row_handlers, finish_unclosed};
use lol_html::{html_content::DocumentEnd, DocumentContentHandlers, Settings};
use std::{cell::RefCell, rc::Rc};

const MAX_ROWS: usize = 1000;
const MAX_LINKS: usize = 16;
const MAX_NAME_BYTES: usize = 512;
type HandlerError = Box<dyn std::error::Error + Send + Sync + 'static>;
type HandlerResult = std::result::Result<(), HandlerError>;

struct Row {
    index: usize,
    identity: Option<AthleteId>,
    selected: Option<ProfileUrl>,
    display_name: String,
    snippet: String,
    pending: String,
    issue: Option<String>,
    links: usize,
    saw_text: bool,
}

struct Link {
    pending: String,
    display_name: String,
}

struct State {
    sport: Sport,
    start: u32,
    digest: EvidenceDigest,
    seen_rows: usize,
    row: Option<Row>,
    link: Option<Link>,
    candidates: Vec<SearchCandidate>,
    issues: Vec<SearchIssue>,
    outside_athlete_link: bool,
}

impl State {
    fn evidence(&self, index: usize) -> EvidenceRef {
        EvidenceRef {
            document: self.digest.clone(),
            locator: format!("/d/results/page/{}/row/{index}", self.start),
        }
    }

    fn row_issue(&mut self, message: impl Into<String>) {
        if let Some(row) = self.row.as_mut() {
            if row.issue.is_none() {
                row.issue = Some(message.into());
            }
        }
    }

    fn row_matches(&self, index: usize) -> bool {
        self.row.as_ref().is_some_and(|row| row.index == index)
    }
}

pub(super) fn handler_error(message: impl Into<String>) -> HandlerError {
    Box::new(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        message.into(),
    ))
}

pub(super) fn rows(
    query: &SearchQuery,
    start: u32,
    digest: &EvidenceDigest,
    html: &str,
) -> Result<(Vec<SearchCandidate>, Vec<SearchIssue>)> {
    let shared = Rc::new(RefCell::new(State {
        sport: query.sport,
        start,
        digest: digest.clone(),
        seen_rows: 0,
        row: None,
        link: None,
        candidates: Vec::with_capacity(32),
        issues: Vec::with_capacity(32),
        outside_athlete_link: false,
    }));
    let mut settings = Settings::new();
    add_row_handlers(&mut settings, Rc::clone(&shared))?;
    add_link_handlers(&mut settings, Rc::clone(&shared))?;
    let end_state = Rc::clone(&shared);
    settings.document_content_handlers.push(
        DocumentContentHandlers::default()
            .end(move |_end: &mut DocumentEnd<'_>| finish_unclosed(&end_state)),
    );
    rewrite_bounded(html, settings)?;
    let state = Rc::try_unwrap(shared)
        .map_err(|_| anyhow::anyhow!("streaming search parser retained callback state"))?
        .into_inner();
    let mut issues = state.issues;
    if state.outside_athlete_link {
        issues.push(SearchIssue {
            code: "unrecognized_result_layout".into(),
            message: "athlete link outside a result row".into(),
            evidence: EvidenceRef {
                document: state.digest,
                locator: "/d/results".into(),
            },
        });
    }
    Ok((state.candidates, issues))
}
