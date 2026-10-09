use super::{team_row_regex, TeamRef};
use crate::{CollectionDisposition, CrawlError, CrawlResult};
use regex::{Captures, Regex};
use std::sync::LazyLock;

const MAX_BODY_BYTES: usize = 8 * 1024 * 1024;
const MAX_ROWS: usize = 20_000;
const MAX_WORK: usize = 65_536;
const MAX_ROW_BYTES: usize = 65_536;
const MAX_FIELD_BYTES: usize = 4096;
const MAX_RETAINED_BYTES: usize = 4 * 1024 * 1024;

static ROWS: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?s)<tr(?:\s[^>]*)?>.*?</tr>"));

#[derive(Debug)]
pub struct TeamIndexRead {
    pub teams: Vec<TeamRef>,
    pub disposition: CollectionDisposition,
    pub unfinished: Vec<String>,
    pub errors: usize,
}

pub fn parse_team_index(html: &str) -> CrawlResult<TeamIndexRead> {
    admit_body(html)?;
    let regex = ROWS.as_ref().map_err(|source| CrawlError::RegexInit {
        pattern: "TEAM_INDEX_ROWS",
        source: source.clone(),
    })?;
    let mut run = Run {
        read: TeamIndexRead {
            teams: Vec::new(),
            disposition: CollectionDisposition::Unknown,
            unfinished: Vec::new(),
            errors: 0,
        },
        rows: 0,
        retained: 0,
    };
    regex
        .find_iter(html)
        .enumerate()
        .find_map(|(ordinal, row)| run.visit(ordinal, row).transpose())
        .transpose()?;
    run.finish(html)
}

fn admit_body(html: &str) -> CrawlResult<()> {
    if html.len() > MAX_BODY_BYTES {
        return Err(resource(
            "team-index body bytes",
            html.len(),
            MAX_BODY_BYTES,
        ));
    }
    let nodes = html
        .bytes()
        .filter(|byte| *byte == b'<')
        .take(MAX_WORK + 1)
        .count();
    if nodes > MAX_WORK {
        return Err(resource("team-index markup nodes", nodes, MAX_WORK));
    }
    Ok(())
}

struct Run {
    read: TeamIndexRead,
    rows: usize,
    retained: usize,
}

impl Run {
    fn visit(&mut self, ordinal: usize, row: regex::Match<'_>) -> CrawlResult<Option<()>> {
        if ordinal >= MAX_WORK {
            self.reject(row.start(), row.end(), "work capacity")?;
            return Ok(Some(()));
        }
        if !row.as_str().contains(".milesplit.com/teams/") {
            return Ok(None);
        }
        if self.rows >= MAX_ROWS {
            self.reject(row.start(), row.end(), "row capacity")?;
            return Ok(Some(()));
        }
        self.rows = self.rows.checked_add(1).ok_or_else(counter_error)?;
        match fields(row.as_str()).and_then(|fields| self.accept(fields)) {
            Ok(()) => Ok(None),
            Err(error) => {
                self.reject(row.start(), row.end(), &error.to_string())?;
                Ok(if matches!(error, CrawlError::Resource { .. }) {
                    Some(())
                } else {
                    None
                })
            }
        }
    }

    fn accept(&mut self, fields: Fields<'_>) -> CrawlResult<()> {
        let requested = self
            .retained
            .checked_add(fields.bytes()?)
            .ok_or_else(counter_error)?;
        if requested > MAX_RETAINED_BYTES {
            return Err(resource(
                "team-index retained bytes",
                requested,
                MAX_RETAINED_BYTES,
            ));
        }
        self.read.teams.try_reserve(1).map_err(|_| {
            resource(
                "team-index allocation",
                self.read.teams.len().saturating_add(1),
                MAX_ROWS,
            )
        })?;
        self.read.teams.push(TeamRef {
            id: fields.id.to_string(),
            slug: fields.slug.to_string(),
            url: fields.url.to_string(),
            name: fields.name.to_string(),
            city_state: fields.city.to_string(),
        });
        self.retained = requested;
        Ok(())
    }

    fn reject(&mut self, start: usize, end: usize, reason: &str) -> CrawlResult<()> {
        self.read.errors = self.read.errors.checked_add(1).ok_or_else(counter_error)?;
        self.read.unfinished.try_reserve(1).map_err(|_| {
            resource(
                "team-index unfinished allocation",
                self.read.unfinished.len().saturating_add(1),
                MAX_ROWS + 1,
            )
        })?;
        let reason = reason
            .get(..reason.len().min(512))
            .map_or("invalid row", |value| value);
        self.read
            .unfinished
            .push(format!("milesplit:teams#bytes={start}-{end}: {reason}"));
        self.read.disposition = CollectionDisposition::Partial;
        Ok(())
    }

    fn finish(mut self, html: &str) -> CrawlResult<TeamIndexRead> {
        let opens = html.matches("<tr").count();
        let closes = html.matches("</tr>").count();
        if opens != closes {
            self.reject(0, html.len(), "unbalanced/truncated row representation")?;
        }
        if self.rows == 0 {
            self.reject(0, html.len(), "no validated team inventory")?;
        }
        if self.read.errors == 0 {
            self.read.disposition = CollectionDisposition::Complete;
        } else if self.read.teams.is_empty() {
            self.read.disposition = CollectionDisposition::Quarantined;
        }
        Ok(self.read)
    }
}

struct Fields<'a> {
    url: &'a str,
    id: &'a str,
    slug: &'a str,
    name: &'a str,
    city: &'a str,
}

fn fields(row: &str) -> CrawlResult<Fields<'_>> {
    if row.len() > MAX_ROW_BYTES {
        return Err(resource("team-index row bytes", row.len(), MAX_ROW_BYTES));
    }
    let capture = team_row_regex()?
        .captures(row)
        .ok_or_else(|| schema("recognized team row does not match its published columns"))?;
    Ok(Fields {
        url: value(&capture, 1)?,
        id: value(&capture, 2)?,
        slug: value(&capture, 3)?,
        name: value(&capture, 4)?,
        city: value(&capture, 5)?,
    })
}

fn value<'a>(capture: &Captures<'a>, index: usize) -> CrawlResult<&'a str> {
    let value = capture
        .get(index)
        .ok_or_else(|| schema("recognized team row is missing a captured column"))?
        .as_str()
        .trim();
    if value.len() > MAX_FIELD_BYTES {
        return Err(resource(
            "team-index field bytes",
            value.len(),
            MAX_FIELD_BYTES,
        ));
    }
    if value.is_empty() && index != 5 {
        return Err(schema("recognized team row has an empty required column"));
    }
    Ok(value)
}

impl Fields<'_> {
    fn bytes(&self) -> CrawlResult<usize> {
        [self.url, self.id, self.slug, self.name, self.city]
            .iter()
            .try_fold(1024_usize, |total, field| {
                total.checked_add(field.len()).ok_or_else(counter_error)
            })
    }
}

fn resource(resource: &'static str, requested: usize, limit: usize) -> CrawlError {
    CrawlError::Resource {
        resource,
        requested,
        limit,
    }
}
fn schema(detail: &str) -> CrawlError {
    CrawlError::Schema {
        url: "milesplit:teams".to_string(),
        detail: detail.to_string(),
    }
}
fn counter_error() -> CrawlError {
    CrawlError::Arithmetic {
        detail: "team-index accounting overflow".to_string(),
    }
}
