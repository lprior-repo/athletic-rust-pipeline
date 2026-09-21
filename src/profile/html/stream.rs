use anyhow::{anyhow, bail, Context, Result};
use html_escape::decode_html_entities;
use lol_html::html_content::{Element, EndTag, TextChunk, TextType};
use lol_html::Selector;
use lol_html::{DocumentContentHandlers, ElementContentHandlers, Settings};
use std::borrow::Cow;
use std::cell::RefCell;
use std::rc::Rc;

use crate::domain::identity::AthleteId;
use crate::html_bounds::rewrite_bounded;

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

struct Scope {
    locator_index: usize,
    text: String,
}

struct Frame {
    tag: String,
    owner: Option<usize>,
    excluded: bool,
    script: bool,
}

struct StreamState {
    requested: AthleteId,
    canonical: Vec<String>,
    og_url: Vec<String>,
    scripts: Vec<String>,
    scopes: Vec<Scope>,
    frames: Vec<Frame>,
    container_count: usize,
    script_buffer: Option<String>,
    text_node: String,
    text_type: Option<TextType>,
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

impl StreamState {
    fn new(requested: AthleteId) -> Self {
        Self {
            requested,
            canonical: Vec::new(),
            og_url: Vec::new(),
            scripts: Vec::new(),
            scopes: Vec::new(),
            frames: Vec::new(),
            container_count: 0,
            script_buffer: None,
            text_node: String::new(),
            text_type: None,
        }
    }

    fn start(&mut self, element: &mut Element<'_, '_>) -> Result<()> {
        let tag = element.tag_name().to_ascii_lowercase();
        self.reject_implicit_close(&tag)?;
        self.collect_metadata(&tag, element)?;
        let (owner, inherited_excluded) = self.new_owner(element)?;
        let excluded = inherited_excluded
            || element.get_attribute("hidden").is_some()
            || matches!(tag.as_str(), "script" | "style" | "template" | "noscript");
        let script = tag == "script";
        if script {
            if self.script_buffer.is_some() {
                bail!("nested script scope is ambiguous");
            }
            self.ensure_capacity(self.scripts.len(), 1024, "script count")?;
            self.script_buffer = Some(String::new());
        }
        if element.can_have_content() {
            if self.frames.len() >= MAX_DEPTH {
                bail!("profile HTML ancestry exceeds bound");
            }
            self.frames.push(Frame {
                tag,
                owner,
                excluded,
                script,
            });
        }
        Ok(())
    }

    fn end(&mut self, end: &mut EndTag<'_>) -> Result<()> {
        let expected = self
            .frames
            .last()
            .map(|frame| frame.tag.as_str())
            .context("profile HTML end tag has no open element")?;
        if !expected.eq_ignore_ascii_case(&end.name()) {
            bail!("profile HTML scope closed ambiguously");
        }
        let frame = self.frames.pop().context("profile HTML frame underflow")?;
        if frame.script {
            self.finish_script()?;
        }
        Ok(())
    }

    fn text(&mut self, text: &mut TextChunk<'_>) -> Result<()> {
        self.text_type = Some(text.text_type());
        self.capture_first_chunk(text)?;
        if text.last_in_text_node() {
            self.finish_text_node()?;
        }
        Ok(())
    }

    fn capture_first_chunk(&mut self, text: &TextChunk<'_>) -> Result<()> {
        if let Some(script) = self.script_buffer.as_mut() {
            append_bounded(script, text.as_str())?;
            return Ok(());
        }
        let Some(frame) = self.frames.last() else {
            return Ok(());
        };
        if frame.excluded {
            return Ok(());
        }
        let Some(owner) = frame.owner else {
            return Ok(());
        };
        if text.text_type().allows_html_entities() {
            append_bounded(&mut self.text_node, text.as_str())?;
            Ok(())
        } else {
            self.append_scope(owner, text.as_str())
        }
    }

    fn finish_text_node(&mut self) -> Result<()> {
        let Some(frame) = self.frames.last() else {
            self.text_node.clear();
            self.text_type = None;
            return Ok(());
        };
        if !frame.excluded {
            if let Some(owner) = frame.owner {
                if self.text_type.is_some_and(TextType::allows_html_entities) {
                    let raw = std::mem::take(&mut self.text_node);
                    let decoded = decode_html_entities(&raw);
                    self.append_scope(owner, &decoded)?;
                    self.text_node = raw;
                }
                self.append_scope(owner, " ")?;
            }
        }
        self.text_node.clear();
        self.text_type = None;
        Ok(())
    }

    fn collect_metadata(&mut self, tag: &str, element: &mut Element<'_, '_>) -> Result<()> {
        let value = |name: &str| element.get_attribute(name);
        if tag == "link" {
            let rel = value("rel").unwrap_or_default();
            if rel
                .split_ascii_whitespace()
                .any(|token| token.eq_ignore_ascii_case("canonical"))
            {
                if let Some(url) = value("href") {
                    Self::push_candidate(&mut self.canonical, url)?;
                }
            }
        }
        if tag == "meta" {
            let property = value("property");
            let name = value("name");
            if property.as_deref() == Some("og:url") || name.as_deref() == Some("og:url") {
                if let Some(url) = value("content") {
                    Self::push_candidate(&mut self.og_url, url)?;
                }
            }
        }
        Ok(())
    }

    fn new_owner(&mut self, element: &mut Element<'_, '_>) -> Result<(Option<usize>, bool)> {
        let inherited = self
            .frames
            .last()
            .map(|frame| (frame.owner, frame.excluded));
        let Some(raw_id) = element.get_attribute("data-athlete-id") else {
            return Ok(inherited.unwrap_or((None, false)));
        };
        let locator_index = self.container_count;
        self.container_count = self
            .container_count
            .checked_add(1)
            .context("profile athlete container count overflow")?;
        if self.container_count > MAX_CONTAINERS {
            bail!("profile athlete container count exceeds bound");
        }
        let owner = raw_id
            .parse::<u64>()
            .ok()
            .and_then(|value| (value == self.requested.get()).then_some(()));
        let scope = owner.map(|_| {
            let index = self.scopes.len();
            self.scopes.push(Scope {
                locator_index,
                text: String::new(),
            });
            index
        });
        Ok((scope, inherited.is_some_and(|(_, excluded)| excluded)))
    }

    fn push_candidate(output: &mut Vec<String>, value: String) -> Result<()> {
        if value.len() > MAX_CAPTURE_BYTES {
            bail!("profile identity URL exceeds capture bound");
        }
        if output.len() >= MAX_CANDIDATES {
            bail!("profile identity URL count exceeds bound");
        }
        output
            .try_reserve(1)
            .context("allocating identity URL evidence")?;
        output.push(value);
        Ok(())
    }

    fn append_scope(&mut self, owner: usize, value: &str) -> Result<()> {
        let scope = self
            .scopes
            .get_mut(owner)
            .context("profile cohort scope disappeared")?;
        append_bounded(&mut scope.text, value)
    }

    fn finish_script(&mut self) -> Result<()> {
        let script = self
            .script_buffer
            .take()
            .context("script scope disappeared")?;
        self.scripts
            .try_reserve(1)
            .context("allocating script evidence")?;
        self.scripts.push(script);
        Ok(())
    }

    fn reject_implicit_close(&self, next: &str) -> Result<()> {
        let Some(current) = self.frames.last().map(|frame| frame.tag.as_str()) else {
            return Ok(());
        };
        if implicitly_closes(current, next) {
            bail!("profile HTML contains an implicit element close");
        }
        Ok(())
    }

    fn ensure_capacity(&self, current: usize, max: usize, label: &str) -> Result<()> {
        if current >= max {
            bail!("profile {label} exceeds bound");
        }
        Ok(())
    }

    fn finish(self) -> Result<Parsed> {
        if !self.frames.is_empty() || self.script_buffer.is_some() || !self.text_node.is_empty() {
            bail!("profile HTML ended with an ambiguous open scope");
        }
        let cohorts = self
            .scopes
            .into_iter()
            .map(|scope| (scope.locator_index, scope.text))
            .collect();
        Ok(Parsed {
            canonical: self.canonical,
            og_url: self.og_url,
            scripts: self.scripts,
            cohorts,
        })
    }
}

fn append_bounded(output: &mut String, value: &str) -> Result<()> {
    let next = output
        .len()
        .checked_add(value.len())
        .context("profile capture byte count overflow")?;
    if next > MAX_CAPTURE_BYTES {
        bail!("profile capture exceeds bound");
    }
    output
        .try_reserve(value.len())
        .context("allocating profile capture")?;
    output.push_str(value);
    Ok(())
}

fn implicitly_closes(current: &str, next: &str) -> bool {
    matches!(
        (current, next),
        (
            "p",
            "address"
                | "article"
                | "aside"
                | "blockquote"
                | "div"
                | "dl"
                | "fieldset"
                | "footer"
                | "form"
                | "h1"
                | "h2"
                | "h3"
                | "h4"
                | "h5"
                | "h6"
                | "header"
                | "hgroup"
                | "hr"
                | "main"
                | "menu"
                | "nav"
                | "ol"
                | "p"
                | "pre"
                | "section"
                | "table"
                | "ul"
        ) | ("li", "li")
            | ("dt" | "dd", "dt" | "dd")
            | ("rt" | "rp", "rt" | "rp")
            | ("option", "option" | "optgroup")
            | ("optgroup", "optgroup")
            | ("thead" | "tbody" | "tfoot", "thead" | "tbody" | "tfoot")
            | ("tr", "tr")
            | ("td" | "th", "td" | "th")
    )
}
