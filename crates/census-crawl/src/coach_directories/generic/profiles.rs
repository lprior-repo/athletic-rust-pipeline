use super::markup::{self, Failure};
use crate::school_sites::{Rules, StaffContactHit};
use crate::CrawlResult;
use census_domain::model::{CoachRole, Gender};

#[derive(Default)]
pub(super) struct Collector {
    pub recognized: usize,
    pub rows: Vec<Profile>,
    current: Option<Profile>,
    stack: Vec<Vec<u8>>,
}

#[derive(Default)]
pub(super) struct Profile {
    cells: Vec<Cell>,
    invalid: bool,
}

#[derive(Default)]
struct Cell {
    text: String,
    mailboxes: Vec<String>,
}

impl Collector {
    pub(super) fn start(&mut self, tag: &html5gum::StartTag<usize>) -> Result<(), Failure> {
        if self.current.is_none() {
            let recognized = tag.name.as_ref() == b"ul"
                && tag
                    .attributes
                    .get(b"class".as_slice())
                    .is_some_and(|value| {
                        value
                            .split(|byte| byte.is_ascii_whitespace())
                            .any(|class| class == b"profile-list-box")
                    });
            if !recognized {
                return Ok(());
            }
            if self.recognized >= 320 {
                return Err(Failure::Budget(
                    "school contact page exceeds 320 staff profiles",
                ));
            }
            self.recognized += 1;
            self.current = Some(Profile::default());
            self.stack.push(b"ul".to_vec());
            return Ok(());
        }
        if self.stack.len() >= 32 {
            return Err(Failure::Budget("staff profile exceeds 32 nested elements"));
        }
        let Some(profile) = self.current.as_mut() else {
            return Ok(());
        };
        let name: &[u8] = tag.name.as_ref();
        if name == b"li" && self.stack.len() == 1 {
            if profile.cells.len() >= 3 {
                profile.invalid = true;
            } else {
                profile.cells.push(Cell::default());
            }
        } else if self.stack.len() < 2
            || !matches!(
                name,
                b"a" | b"span" | b"strong" | b"em" | b"b" | b"i" | b"br"
            )
        {
            profile.invalid = true;
        }
        if name == b"a" {
            if let Some(href) = tag.attributes.get(b"href".as_slice()) {
                let href = std::str::from_utf8(href)
                    .map_err(|_| Failure::Decode("staff profile link is not UTF-8"))?;
                if href.len() > 4096 {
                    return Err(Failure::Budget("staff profile link exceeds 4096 bytes"));
                }
                if let Some(address) = markup::after(href, "mailto:") {
                    if let Some(cell) = profile.cells.last_mut() {
                        if cell.mailboxes.len() >= 2 {
                            profile.invalid = true;
                        } else {
                            cell.mailboxes.push(
                                address
                                    .split('?')
                                    .next()
                                    .map_or(address, |address| address)
                                    .to_owned(),
                            );
                        }
                    }
                }
            }
        }
        if name == b"br" {
            self.text(b" ")?;
        } else {
            self.stack.push(name.to_vec());
        }
        Ok(())
    }

    pub(super) fn end(&mut self, name: &[u8]) {
        if self.current.is_none() {
            return;
        }
        if self
            .stack
            .last()
            .is_some_and(|open| open.as_slice() == name)
        {
            self.stack.pop();
            if self.stack.is_empty() {
                if let Some(profile) = self.current.take() {
                    if !profile.invalid && profile.cells.len() == 3 {
                        self.rows.push(profile);
                    }
                }
            }
        } else {
            self.invalidate();
        }
    }

    pub(super) fn invalidate(&mut self) {
        if let Some(profile) = self.current.as_mut() {
            profile.invalid = true;
        }
    }

    pub(super) fn unfinished(&self) -> bool {
        self.current.is_some()
    }

    pub(super) fn text(&mut self, bytes: &[u8]) -> Result<(), Failure> {
        let Some(profile) = self.current.as_mut() else {
            return Ok(());
        };
        let text = std::str::from_utf8(bytes)
            .map_err(|_| Failure::Decode("staff profile text is not UTF-8"))?;
        if self.stack.len() < 2 {
            profile.invalid |= !text.trim().is_empty();
            return Ok(());
        }
        if let Some(cell) = profile.cells.last_mut() {
            if cell
                .text
                .len()
                .checked_add(text.len())
                .is_none_or(|size| size > 4096)
            {
                return Err(Failure::Budget("staff profile field exceeds 4096 bytes"));
            }
            cell.text.push_str(text);
        }
        Ok(())
    }
}

pub(crate) fn candidates(
    rules: &Rules,
    url: &str,
    body: &[u8],
) -> CrawlResult<Option<Vec<StaffContactHit>>> {
    let page = super::checked_page(body, "staff profile page")?;
    if page.profiles.recognized == 0 {
        return Ok(None);
    }
    let mut hits = Vec::new();
    for profile in page.profiles.rows {
        let [name, role, mailbox] = profile.cells.as_slice() else {
            continue;
        };
        if !name.mailboxes.is_empty() || !role.mailboxes.is_empty() || mailbox.mailboxes.len() != 1
        {
            continue;
        }
        let published_role = crate::row_hygiene::clean_text(&role.text);
        if ![
            "athletic director",
            "athletics director",
            "director of athletics",
        ]
        .iter()
        .any(|label| published_role.eq_ignore_ascii_case(label))
            || crate::coach_contacts::parse_role(&published_role)
                != Some(CoachRole::AthleticDirector)
        {
            continue;
        }
        let published_name = crate::row_hygiene::clean_text(&name.text);
        let Some(name) = crate::school_sites::parse::clean_person(rules, &published_name)
            .filter(|name| name == &published_name)
        else {
            continue;
        };
        let Some(name) = crate::row_hygiene::sanitize_person(&name)? else {
            continue;
        };
        let Some(email) = mailbox
            .mailboxes
            .first()
            .and_then(|email| rules.sanitize_email(email))
        else {
            continue;
        };
        if !crate::row_hygiene::clean_text(&mailbox.text).eq_ignore_ascii_case(&email) {
            continue;
        }
        hits.push(StaffContactHit {
            published_school: String::new(),
            member_id: String::new(),
            category_id: String::new(),
            name,
            published_role,
            role: CoachRole::AthleticDirector,
            published_sport: String::new(),
            sport: None,
            gender: Gender::Unknown,
            published_level: String::new(),
            email,
            url: url.to_owned(),
        });
    }
    Ok(Some(hits))
}
