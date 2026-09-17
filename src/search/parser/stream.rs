use super::{
    athlete_link, link_profile, normalize, profile_matches_sport, selector, SearchCandidate,
    SearchIssue, SearchQuery, MAX_TEXT_BYTES,
};
use crate::domain::{
    evidence::{EvidenceRef, Sport},
    identity::{AthleteId, EvidenceDigest, ProfileUrl},
};
use crate::html_bounds::rewrite_bounded;
use anyhow::Result;
use lol_html::{
    html_content::{DocumentEnd, Element, EndTag, TextChunk},
    DocumentContentHandlers, ElementContentHandlers, Settings,
};
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

fn add_row_handlers(
    settings: &mut Settings<'static, 'static>,
    shared: Rc<RefCell<State>>,
) -> Result<()> {
    let start_state = Rc::clone(&shared);
    let text_state = Rc::clone(&shared);
    let handlers = ElementContentHandlers::default()
        .element(move |element: &mut Element<'_, '_>| start_row(&start_state, element))
        .text(move |chunk: &mut TextChunk<'_>| append_row_text(&text_state, chunk));
    settings
        .element_content_handlers
        .push((selector("tr")?, handlers));
    Ok(())
}

fn add_link_handlers(
    settings: &mut Settings<'static, 'static>,
    shared: Rc<RefCell<State>>,
) -> Result<()> {
    let start_state = Rc::clone(&shared);
    let text_state = Rc::clone(&shared);
    let handlers = ElementContentHandlers::default()
        .element(move |element: &mut Element<'_, '_>| start_link(&start_state, element))
        .text(move |chunk: &mut TextChunk<'_>| append_link_text(&text_state, chunk));
    settings
        .element_content_handlers
        .push((selector("a[href]")?, handlers));
    Ok(())
}

fn start_row(shared: &Rc<RefCell<State>>, element: &mut Element<'_, '_>) -> HandlerResult {
    let mut state = shared.borrow_mut();
    if state.row.is_some() {
        state.row_issue("result row ended implicitly before the next row");
        finish_row(&mut state).map_err(handler_error)?;
    }
    if state.seen_rows >= MAX_ROWS {
        return Err(handler_error("search page exceeds row bound"));
    }
    let index = state.seen_rows;
    state.seen_rows += 1;
    state.row = Some(Row {
        index,
        identity: None,
        selected: None,
        display_name: String::new(),
        snippet: String::new(),
        pending: String::new(),
        issue: None,
        links: 0,
        saw_text: false,
    });
    let end_state = Rc::clone(shared);
    element.on_end_tag(Box::new(move |end: &mut EndTag<'_>| {
        let mut state = end_state.borrow_mut();
        if !state.row_matches(index) {
            return Ok(());
        }
        if end.source_location().bytes().is_empty() {
            state.row_issue("result row ended implicitly");
        }
        finish_row(&mut state).map_err(handler_error)
    }))
}

fn append_row_text(shared: &Rc<RefCell<State>>, chunk: &mut TextChunk<'_>) -> HandlerResult {
    let mut state = shared.borrow_mut();
    let Some(row) = state.row.as_mut() else {
        return Ok(());
    };
    append_raw(&mut row.pending, chunk.as_str(), &mut row.issue);
    if chunk.last_in_text_node() {
        let pending = std::mem::take(&mut row.pending);
        flush_node(
            &mut row.snippet,
            &mut row.saw_text,
            &pending,
            &mut row.issue,
        );
    }
    Ok(())
}

fn start_link(shared: &Rc<RefCell<State>>, element: &mut Element<'_, '_>) -> HandlerResult {
    let href = element.get_attribute("href");
    let Some(href) = href else {
        return Ok(());
    };
    let mut state = shared.borrow_mut();
    if state.row.is_none() {
        if athlete_link(&href) {
            state.outside_athlete_link = true;
        }
        return Ok(());
    }
    if state.link.is_some() {
        state.row_issue("athlete link ended implicitly before the next link");
        finish_link(&mut state).map_err(handler_error)?;
    }
    if !athlete_link(&href) {
        return Ok(());
    }
    let profile = match link_profile(&href) {
        Ok(profile) => profile,
        Err(error) => {
            state.row_issue(error.to_string());
            return Ok(());
        }
    };
    let sport = state.sport;
    let row = state
        .row
        .as_mut()
        .ok_or_else(|| handler_error("streaming search parser lost result row"))?;
    row.links = row.links.saturating_add(1);
    if row.links > MAX_LINKS {
        row.issue = Some("result row exceeds athlete link bound".into());
        return Ok(());
    }
    if row.identity.is_some_and(|id| id != profile.athlete_id()) && row.issue.is_none() {
        row.issue = Some("result row has conflicting athlete identity links".into());
    }
    row.identity = Some(profile.athlete_id());
    if profile_matches_sport(&profile, sport) {
        row.selected = Some(profile);
    }
    let end_state = Rc::clone(shared);
    state.link = Some(Link {
        pending: String::new(),
        display_name: String::new(),
    });
    element.on_end_tag(Box::new(move |end: &mut EndTag<'_>| {
        let mut state = end_state.borrow_mut();
        if end.source_location().bytes().is_empty() {
            state.row_issue("athlete link ended implicitly");
        }
        finish_link(&mut state).map_err(handler_error)
    }))
}

fn append_link_text(shared: &Rc<RefCell<State>>, chunk: &mut TextChunk<'_>) -> HandlerResult {
    let mut state = shared.borrow_mut();
    let Some(link) = state.link.as_mut() else {
        return Ok(());
    };
    let mut issue = None;
    append_raw(&mut link.pending, chunk.as_str(), &mut issue);
    if chunk.last_in_text_node() {
        let pending = std::mem::take(&mut link.pending);
        let mut issue = issue.take();
        let mut saw_text = !link.display_name.is_empty();
        flush_node(&mut link.display_name, &mut saw_text, &pending, &mut issue);
        if issue.is_some() {
            state.row_issue("search row text exceeds bound");
        }
    } else if issue.is_some() {
        state.row_issue("search row text exceeds bound");
    }
    Ok(())
}

fn finish_link(state: &mut State) -> std::result::Result<(), String> {
    let Some(mut link) = state.link.take() else {
        return Ok(());
    };
    if !link.pending.is_empty() {
        state.row_issue("athlete link text is incomplete");
        return Ok(());
    }
    let Some(row) = state.row.as_mut() else {
        return Err("streaming search parser lost result row".into());
    };
    if !link.display_name.is_empty() {
        if !row.display_name.is_empty()
            && row.display_name != link.display_name
            && row.issue.is_none()
        {
            row.issue = Some("result row has conflicting display names".into());
        }
        row.display_name = std::mem::take(&mut link.display_name);
    }
    Ok(())
}

fn finish_row(state: &mut State) -> std::result::Result<(), String> {
    if state.link.is_some() {
        state.row_issue("athlete link ended implicitly with its result row");
        finish_link(state)?;
    }
    let Some(mut row) = state.row.take() else {
        return Ok(());
    };
    if !row.pending.is_empty() {
        row.issue = Some("result row text is incomplete".into());
    }
    let issue = row.issue.take();
    let Some(athlete_id) = row.identity else {
        return Ok(());
    };
    let evidence = state.evidence(row.index);
    if let Some(message) = issue {
        state.issues.push(SearchIssue {
            code: "invalid_athlete_row".into(),
            message,
            evidence,
        });
        return Ok(());
    }
    let Some(profile_url) = row.selected else {
        state.issues.push(SearchIssue {
            code: "invalid_athlete_row".into(),
            message: "result row belongs to another or unspecified sport".into(),
            evidence,
        });
        return Ok(());
    };
    if row.display_name.is_empty() || row.display_name.len() > MAX_NAME_BYTES {
        state.issues.push(SearchIssue {
            code: "invalid_athlete_row".into(),
            message: "athlete display name is absent or exceeds bound".into(),
            evidence,
        });
        return Ok(());
    }
    state.candidates.push(SearchCandidate {
        athlete_id,
        profile_url,
        display_name: row.display_name,
        snippet: row.snippet,
        sport: state.sport,
        evidence,
    });
    Ok(())
}

fn finish_unclosed(shared: &Rc<RefCell<State>>) -> HandlerResult {
    let mut state = shared.borrow_mut();
    if state.row.is_some() {
        state.row_issue("result row is unclosed at end of input");
    }
    finish_row(&mut state).map_err(handler_error)
}

fn append_raw(target: &mut String, text: &str, issue: &mut Option<String>) {
    let Some(size) = target.len().checked_add(text.len()) else {
        *issue = Some("search row text exceeds bound".into());
        return;
    };
    if size > MAX_TEXT_BYTES {
        *issue = Some("search row text exceeds bound".into());
        return;
    }
    if target.try_reserve(text.len()).is_err() {
        *issue = Some("allocating bounded search row text".into());
        return;
    }
    target.push_str(text);
}

fn flush_node(target: &mut String, saw_text: &mut bool, raw: &str, issue: &mut Option<String>) {
    let normalized = match normalize(raw) {
        Ok(value) => value,
        Err(error) => {
            *issue = Some(error.to_string());
            return;
        }
    };
    if normalized.is_empty() {
        return;
    }
    let separator = usize::from(*saw_text);
    let Some(size) = target
        .len()
        .checked_add(separator)
        .and_then(|size| size.checked_add(normalized.len()))
    else {
        *issue = Some("search row text exceeds bound".into());
        return;
    };
    if size > MAX_TEXT_BYTES || target.try_reserve(separator + normalized.len()).is_err() {
        *issue = Some("search row text exceeds bound".into());
        return;
    }
    if separator != 0 {
        target.push(' ');
    }
    target.push_str(&normalized);
    *saw_text = true;
}
