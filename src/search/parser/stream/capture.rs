//! `lol_html` handler wiring for the streaming capture: which selectors receive
//! element and text callbacks, and how an unclosed document is finalized.

use super::super::selector;
use super::link::{append_link_text, start_link};
use super::row::{append_row_text, finish_row, start_row};
use super::{handler_error, HandlerResult, State};
use anyhow::Result;
use lol_html::html_content::{Element, TextChunk};
use lol_html::{ElementContentHandlers, Settings};
use std::{cell::RefCell, rc::Rc};

pub(super) fn add_row_handlers(
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

pub(super) fn add_link_handlers(
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

pub(super) fn finish_unclosed(shared: &Rc<RefCell<State>>) -> HandlerResult {
    let mut state = shared.borrow_mut();
    if state.row.is_some() {
        state.row_issue("result row is unclosed at end of input");
    }
    finish_row(&mut state).map_err(handler_error)
}
