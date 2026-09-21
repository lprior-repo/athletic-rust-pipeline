//! Result-row capture: row start, bounded row text, and row finalization.

use super::super::{SearchCandidate, SearchIssue};
use super::bounds::{append_raw, flush_node};
use super::link::finish_link;
use super::{handler_error, HandlerResult, Row, State, MAX_NAME_BYTES, MAX_ROWS};
use lol_html::html_content::{Element, EndTag, TextChunk};
use std::{cell::RefCell, rc::Rc};

pub(super) fn start_row(
    shared: &Rc<RefCell<State>>,
    element: &mut Element<'_, '_>,
) -> HandlerResult {
    let mut state = shared.borrow_mut();
    if state.row.is_some() {
        state.row_issue("result row ended implicitly before the next row");
        finish_row(&mut state).map_err(handler_error)?;
    }
    if state.seen_rows >= MAX_ROWS {
        return Err(handler_error("search page exceeds row bound"));
    }
    let index = state.seen_rows;
    // `MAX_ROWS` guards this counter above, so saturation is unreachable; saturating keeps it total.
    state.seen_rows = index.saturating_add(1);
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

pub(super) fn append_row_text(
    shared: &Rc<RefCell<State>>,
    chunk: &mut TextChunk<'_>,
) -> HandlerResult {
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

pub(super) fn finish_row(state: &mut State) -> std::result::Result<(), String> {
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
    if let Some(message) = issue {
        state.issues.push(SearchIssue {
            code: "invalid_athlete_row".into(),
            message,
            evidence: state.evidence(row.index),
        });
        return Ok(());
    }
    let Some(athlete_id) = row.identity else {
        return Ok(());
    };
    let evidence = state.evidence(row.index);
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
