//! Streaming `lol_html` adapter for profile HTML: bounded capture of the
//! canonical identity, embedded scripts, and athlete cohort scopes.

mod bounds;
mod buffer;
mod structure;

use crate::domain::identity::AthleteId;
use crate::html_bounds::rewrite_bounded;
use anyhow::{anyhow, Result};
use buffer::StreamState;
use lol_html::html_content::{Element, EndTag, TextChunk};
use lol_html::Selector;
use lol_html::{DocumentContentHandlers, ElementContentHandlers, Settings};
use std::borrow::Cow;
use std::cell::RefCell;
use std::rc::Rc;

const MAX_CAPTURE_BYTES: usize = 2 * 1024 * 1024;
const MAX_CONTAINERS: usize = 4096;
const MAX_CANDIDATES: usize = 4096;
const MAX_DEPTH: usize = 128;

pub(super) struct Parsed {
    pub canonical: Vec<String>,
    pub og_url: Vec<String>,
    pub scripts: Vec<String>,
    pub cohorts: Vec<(usize, String)>,
}

pub(super) fn parse(source: &str, requested: AthleteId) -> Result<Parsed> {
    let state = Rc::new(RefCell::new(StreamState::new(requested)));
    let selector = "*"
        .parse::<Selector>()
        .map_err(|error| anyhow!("profile selector: {error:?}"))?;
    let element_state = Rc::clone(&state);
    let text_state = Rc::clone(&state);
    let handlers =
        ElementContentHandlers::default().element(move |element: &mut Element<'_, '_>| {
            element_state
                .borrow_mut()
                .start(element)
                .map_err(handler_error)?;
            if element.can_have_content() {
                let end_state = Rc::clone(&element_state);
                element.on_end_tag(Box::new(move |end: &mut EndTag<'_>| {
                    end_state.borrow_mut().end(end).map_err(handler_error)
                }))?;
            }
            Ok(())
        });
    let settings = Settings {
        element_content_handlers: vec![(Cow::Owned(selector), handlers)],
        document_content_handlers: vec![DocumentContentHandlers::default().text(
            move |text: &mut TextChunk<'_>| {
                text_state.borrow_mut().text(text).map_err(handler_error)
            },
        )],
        ..Settings::new()
    };
    rewrite_bounded(source, settings)?;
    Rc::try_unwrap(state)
        .map_err(|_| anyhow!("profile parser retained callback state"))?
        .into_inner()
        .finish()
}

fn handler_error(error: anyhow::Error) -> Box<dyn std::error::Error + Send + Sync> {
    error.into()
}
