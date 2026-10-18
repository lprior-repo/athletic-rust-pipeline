use super::markup::Failure;

#[derive(Debug)]
pub(crate) struct Block {
    pub text: String,
    pub heading: Option<String>,
}

#[derive(Default)]
pub(super) struct Collector {
    pub(super) blocks: Vec<Block>,
    text: String,
    heading: Option<String>,
    heading_tag: Option<u8>,
    chrome: Vec<&'static [u8]>,
    scopes: Vec<Scope>,
}

struct Scope {
    tag: &'static [u8],
    heading: Option<String>,
}

impl Collector {
    pub(super) fn start(&mut self, tag: &html5gum::StartTag<usize>) -> Result<(), Failure> {
        if let Some(kind) = chrome(&tag.name) {
            self.finish()?;
            self.heading = None;
            if self.chrome.len() >= 128 {
                return Err(Failure::Budget(
                    "school coach page exceeds 128 chrome scopes",
                ));
            }
            self.chrome.push(kind);
            return Ok(());
        }
        if !self.chrome.is_empty() {
            return Ok(());
        }
        if let Some(level) = heading(&tag.name) {
            self.finish()?;
            self.heading = None;
            self.heading_tag = Some(level);
        } else if boundary(&tag.name) {
            self.finish()?;
            if let Some(tag) = scope(&tag.name) {
                if self.scopes.len() >= 128 {
                    return Err(Failure::Budget(
                        "school coach page exceeds 128 section scopes",
                    ));
                }
                self.scopes.push(Scope {
                    tag,
                    heading: self.heading.clone(),
                });
            }
        }
        Ok(())
    }

    pub(super) fn end(&mut self, name: &[u8]) -> Result<(), Failure> {
        if let Some(kind) = chrome(name) {
            if let Some(position) = self.chrome.iter().rposition(|open| *open == kind) {
                self.chrome.truncate(position);
            }
            self.heading = None;
            return Ok(());
        }
        if !self.chrome.is_empty() {
            return Ok(());
        }
        if heading(name).is_some() || boundary(name) {
            self.finish()?;
        }
        if let Some(position) = self.scopes.iter().rposition(|open| open.tag == name) {
            self.heading = self
                .scopes
                .get(position)
                .and_then(|open| open.heading.clone());
            self.scopes.truncate(position);
        }
        Ok(())
    }

    pub(super) fn text(&mut self, bytes: &[u8]) -> Result<(), Failure> {
        if !self.chrome.is_empty() {
            return Ok(());
        }
        let text = std::str::from_utf8(bytes)
            .map_err(|_| Failure::Decode("school coach text is not UTF-8"))?;
        if self
            .text
            .len()
            .checked_add(text.len())
            .is_none_or(|len| len > 4096)
        {
            return Err(Failure::Budget("school coach block exceeds 4096 bytes"));
        }
        self.text.push_str(text);
        Ok(())
    }

    pub(super) fn finish(&mut self) -> Result<(), Failure> {
        let text = std::mem::take(&mut self.text);
        let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
        if self.heading_tag.take().is_some() {
            self.heading = (!text.is_empty()).then_some(text);
        } else if !text.is_empty() {
            if self.blocks.len() >= 4096 {
                return Err(Failure::Budget(
                    "school coach page exceeds 4096 text blocks",
                ));
            }
            self.blocks.push(Block {
                text,
                heading: self.heading.clone(),
            });
        }
        Ok(())
    }
}

fn heading(name: &[u8]) -> Option<u8> {
    match name {
        [b'h', level @ b'1'..=b'6'] => Some(*level),
        _ => None,
    }
}

fn chrome(name: &[u8]) -> Option<&'static [u8]> {
    match name {
        b"nav" => Some(b"nav"),
        b"header" => Some(b"header"),
        b"footer" => Some(b"footer"),
        _ => None,
    }
}

fn scope(name: &[u8]) -> Option<&'static [u8]> {
    match name {
        b"section" => Some(b"section"),
        b"article" => Some(b"article"),
        b"main" => Some(b"main"),
        _ => None,
    }
}

fn boundary(name: &[u8]) -> bool {
    matches!(
        name,
        b"p" | b"div"
            | b"li"
            | b"address"
            | b"td"
            | b"th"
            | b"tr"
            | b"table"
            | b"section"
            | b"article"
            | b"main"
            | b"br"
            | b"hr"
    )
}
