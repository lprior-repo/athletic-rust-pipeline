//! Athlete-link capture: link start, bounded link text, and link finalization.

use super::super::{athlete_link, link_profile, placeholder_link, profile_matches_sport};
use super::bounds::{append_raw, flush_node};
use super::{handler_error, HandlerResult, Link, State, MAX_LINKS};
use crate::domain::{evidence::Sport, identity::ProfileUrl};
use lol_html::html_content::{Element, EndTag, TextChunk};
use std::{cell::RefCell, rc::Rc};

pub(super) fn start_link(
    shared: &Rc<RefCell<State>>,
    element: &mut Element<'_, '_>,
) -> HandlerResult {
    let href = element.get_attribute("href");
    let Some(href) = href else {
        return Ok(());
    };
    {
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
        if placeholder_link(&href) {
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
        drop(state);
        let end_state = Rc::clone(shared);
        resolve_link(shared, &profile, sport, element, end_state)
    }
}

/// Validate the link profile against the current row and set up link capture.
fn resolve_link(
    shared: &Rc<RefCell<State>>,
    profile: &ProfileUrl,
    sport: Sport,
    element: &mut Element<'_, '_>,
    end_state: Rc<RefCell<State>>,
) -> HandlerResult {
    let mut state = shared.borrow_mut();
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
    if profile_matches_sport(profile, sport) {
        row.selected = Some(profile.clone());
    }
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

pub(super) fn append_link_text(
    shared: &Rc<RefCell<State>>,
    chunk: &mut TextChunk<'_>,
) -> HandlerResult {
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

pub(super) fn finish_link(state: &mut State) -> std::result::Result<(), String> {
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
