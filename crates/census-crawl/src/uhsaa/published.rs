use super::owner::{artifact, school_id};
use super::parse::SchoolLink;
use super::STATE;
use crate::directory::ReadOutcome;
use crate::{CrawlError, CrawlResult};
use census_domain::school_directory::{
    IdentifiedKey, SchoolDirectoryEntry, SchoolName, SourceLabel,
};
use html5gum::{DefaultEmitter, HtmlString, StartTag, Token, Tokenizer};

#[derive(Default)]
pub(super) struct Published {
    pub name: String,
    pub address: String,
    pub links: Vec<SchoolLink>,
    names: usize,
    addresses: usize,
    ignored: Vec<HtmlString>,
    heading: Option<String>,
    details: bool,
    street: Option<String>,
    item: bool,
}

pub(super) fn read(text: &str) -> CrawlResult<Published> {
    if text.len() > 1024 * 1024 {
        return Err(CrawlError::Resource {
            resource: "UHSAA body bytes",
            requested: text.len(),
            limit: 1024 * 1024,
        });
    }
    let mut emitter = DefaultEmitter::default();
    emitter.naively_switch_states(true);
    let mut published = Published::default();
    for token in Tokenizer::new_with_emitter(text, emitter) {
        let token = token.map_err(|error| artifact(error.to_string()))?;
        published.absorb(token)?;
    }
    if published.names > 1 || published.addresses > 1 {
        return Err(artifact(
            "UHSAA profile has conflicting published headings or addresses",
        ));
    }
    Ok(published)
}

impl Published {
    fn absorb(&mut self, token: Token) -> CrawlResult<()> {
        match token {
            Token::StartTag(tag) => self.start(tag)?,
            Token::EndTag(tag) if self.ignored.last().is_some_and(|name| *name == tag.name) => {
                self.ignored.pop();
            }
            Token::EndTag(tag) => self.end(&tag.name)?,
            Token::String(text) if self.ignored.is_empty() => {
                let text =
                    std::str::from_utf8(&text).map_err(|error| artifact(error.to_string()))?;
                if let Some(heading) = self.heading.as_mut() {
                    heading.push_str(text);
                }
                if let Some(street) = self.street.as_mut() {
                    street.push_str(text);
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn start(&mut self, tag: StartTag<()>) -> CrawlResult<()> {
        if matches!(
            tag.name.as_slice(),
            b"script" | b"style" | b"template" | b"svg" | b"math" | b"noscript"
        ) {
            if self.ignored.len() >= 128 {
                return Err(artifact("UHSAA inert markup nesting exceeds 128"));
            }
            self.ignored.push(tag.name);
            return Ok(());
        }
        if !self.ignored.is_empty() {
            return Ok(());
        }
        match tag.name.as_slice() {
            b"h1" if class(&tag, b"school-name") => self.heading = Some(String::new()),
            b"ul" if class(&tag, b"school-details") => self.details = true,
            b"li" if self.details => {
                self.item = true;
                self.street = None;
            }
            b"i" if self.item && class(&tag, b"fa-map-marker-alt") => {
                self.street = Some(String::new())
            }
            b"a" => self.link(&tag)?,
            _ => {}
        }
        Ok(())
    }

    fn end(&mut self, name: &[u8]) -> CrawlResult<()> {
        match name {
            b"h1" => {
                if let Some(heading) = self.heading.take() {
                    self.names = self
                        .names
                        .checked_add(1)
                        .ok_or_else(|| artifact("UHSAA heading count overflow"))?;
                    self.name = heading.split_whitespace().collect::<Vec<_>>().join(" ");
                }
            }
            b"ul" => self.details = false,
            b"li" => {
                self.item = false;
                if let Some(street) = self.street.take() {
                    self.addresses = self
                        .addresses
                        .checked_add(1)
                        .ok_or_else(|| artifact("UHSAA address count overflow"))?;
                    self.address = street.split_whitespace().collect::<Vec<_>>().join(" ");
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn link(&mut self, tag: &StartTag<()>) -> CrawlResult<()> {
        let Some(href) = attribute(tag, b"href")? else {
            return Ok(());
        };
        if !href.contains("school-directory/?") {
            return Ok(());
        }
        school_id(href)?;
        let name = attribute(tag, b"title")?
            .or(attribute(tag, b"data-school")?)
            .ok_or_else(|| artifact("UHSAA indexed link has no published name"))?;
        SchoolName::parse(name).map_err(|error| artifact(error.to_string()))?;
        if self
            .links
            .iter()
            .any(|link| link.url == href && link.name == name)
        {
            return Ok(());
        }
        if self.links.len() >= 20_000 {
            return Err(artifact("UHSAA indexed school limit exceeded"));
        }
        self.links
            .try_reserve(1)
            .map_err(|_| artifact("UHSAA indexed school allocation failed"))?;
        self.links.push(SchoolLink {
            name: name.to_string(),
            url: href.to_string(),
        });
        Ok(())
    }
}

fn attribute<'a>(tag: &'a StartTag<()>, name: &[u8]) -> CrawlResult<Option<&'a str>> {
    tag.attributes
        .get(name)
        .map(|value| std::str::from_utf8(value).map_err(|error| artifact(error.to_string())))
        .transpose()
}

fn class(tag: &StartTag<()>, name: &[u8]) -> bool {
    tag.attributes
        .get(b"class".as_slice())
        .is_some_and(|value| {
            value
                .split(|byte| byte.is_ascii_whitespace())
                .any(|part| part == name)
        })
}

pub fn parse_school_list(text: &str) -> CrawlResult<ReadOutcome> {
    let published = read(text)?;
    if published.links.is_empty() {
        return Err(artifact(
            "UHSAA capture has no published directory school owners",
        ));
    }
    let mut outcome = ReadOutcome::new();
    let mut seen = std::collections::BTreeMap::new();
    for (ordinal, link) in published.links.iter().enumerate() {
        let id = school_id(&link.url)?;
        let name = SchoolName::parse(&link.name).map_err(|error| artifact(error.to_string()))?;
        if let Some(previous) = seen.insert(id.clone(), name.clone()) {
            if previous != name {
                return Err(artifact("UHSAA schoolID has conflicting published names"));
            }
            continue;
        }
        outcome.push(SchoolDirectoryEntry::identified(
            IdentifiedKey::StateRecord { state: STATE, id },
            SourceLabel::AthleticAssociation { state: STATE },
            Some(name),
        ))?;
        outcome.note(ordinal.saturating_add(1), "address", "directory publishes identity only; profile capture qualification is required for postal ownership")?;
    }
    outcome.finish();
    Ok(outcome)
}
