use super::super::roster::{
    RosterQuarantine, RosterRejection, RosterRejectionKind, RosterVerdict, SourceRowLocator,
};
use super::super::wire::{Roster, RosterAthlete, TeamRef};
use super::{
    athlete_link_regex, athlete_row_regex, gender_cell_regex, grad_cell_regex, html_unescape,
    season_cell_regex,
};
use crate::{CrawlError, CrawlResult};
use census_domain::model::{flip_last_first, Gender, GradYear};
use regex::Regex;

mod owner;
const MAX_BODY_BYTES: usize = 8 * 1024 * 1024;
const MAX_ROWS: usize = 20_000;
const MAX_ROW_BYTES: usize = 64 * 1024;
const MAX_FIELD_BYTES: usize = 4096;

pub(in crate::milesplit) fn parse_captured_roster(
    html: &str,
    team: TeamRef,
    capture: &crate::net::FetchOutcome,
) -> CrawlResult<RosterVerdict> {
    body_bound(html)?;
    owner::validate_capture(html, &team, capture)?;
    parse_rows(html, team)
}

pub fn parse_roster(html: &str, team: TeamRef) -> CrawlResult<RosterVerdict> {
    body_bound(html)?;
    owner::validate_published(html, &team)?;
    parse_rows(html, team)
}

#[derive(Default)]
struct Parsed {
    athletes: Vec<RosterAthlete>,
    rejected: Vec<RosterRejection>,
    unfinished: Option<SourceRowLocator>,
}

fn parse_rows(html: &str, team: TeamRef) -> CrawlResult<RosterVerdict> {
    let fields = Fields {
        link: athlete_link_regex()?,
        gender: gender_cell_regex()?,
        graduation: grad_cell_regex()?,
        seasons: season_cell_regex()?,
    };
    let parsed = athlete_row_regex()?
        .captures_iter(html)
        .take(MAX_ROWS + 1)
        .enumerate()
        .try_fold(Parsed::default(), |mut parsed, (ordinal, capture)| {
            let row = capture.get(1).ok_or_else(|| CrawlError::Invariant {
                detail: "roster expression omitted row capture".into(),
            })?;
            let locator = SourceRowLocator {
                ordinal: u32::try_from(ordinal)
                    .map_err(|_| resource("roster row ordinal", ordinal, MAX_ROWS))?,
                byte_offset: row.start(),
                byte_length: row.len(),
            };
            if ordinal == MAX_ROWS {
                parsed.unfinished = Some(locator);
            } else {
                parsed.admit(&fields, row.as_str(), locator)?;
            }
            Ok::<_, CrawlError>(parsed)
        })?;
    Ok(verdict(html, team, parsed))
}

impl Parsed {
    fn admit(
        &mut self,
        fields: &Fields<'_>,
        html: &str,
        locator: SourceRowLocator,
    ) -> CrawlResult<()> {
        let value = if html.len() > MAX_ROW_BYTES {
            Err(RosterRejection {
                row: locator,
                athlete_id: None,
                kind: RosterRejectionKind::RowTooLarge,
            })
        } else {
            fields.athlete(html, locator)
        };
        match value {
            Ok(athlete) => {
                reserve(&mut self.athletes)?;
                self.athletes.push(athlete);
            }
            Err(rejection) => {
                reserve(&mut self.rejected)?;
                self.rejected.push(rejection);
            }
        }
        Ok(())
    }
}

fn verdict(html: &str, team: TeamRef, parsed: Parsed) -> RosterVerdict {
    let Parsed {
        athletes,
        rejected,
        unfinished,
    } = parsed;
    if athletes.is_empty() {
        let reason = if unfinished.is_some() {
            RosterQuarantine::RowCapacity
        } else if rejected.is_empty() && !html.contains("rosterDataset") {
            RosterQuarantine::UnknownTemplate
        } else {
            RosterQuarantine::NoReadableRows
        };
        return RosterVerdict::Quarantined {
            reason,
            rejected,
            unfinished,
        };
    }
    let roster = Roster { team, athletes };
    if rejected.is_empty() && unfinished.is_none() {
        RosterVerdict::Complete { roster }
    } else {
        RosterVerdict::Partial {
            roster,
            rejected,
            unfinished,
        }
    }
}

struct Fields<'a> {
    link: &'a Regex,
    gender: &'a Regex,
    graduation: &'a Regex,
    seasons: &'a Regex,
}
struct Reject<'a> {
    row: SourceRowLocator,
    id: Option<&'a str>,
}

impl Reject<'_> {
    fn make(&self, kind: RosterRejectionKind) -> RosterRejection {
        let athlete_id = self
            .id
            .filter(|id| id.len() <= MAX_FIELD_BYTES)
            .map(str::to_owned);
        RosterRejection {
            row: self.row,
            athlete_id,
            kind,
        }
    }
}

impl Fields<'_> {
    fn athlete(&self, html: &str, row: SourceRowLocator) -> Result<RosterAthlete, RosterRejection> {
        let capture = self.link.captures(html);
        let id = capture
            .as_ref()
            .and_then(|capture| capture.get(2))
            .map(|id| id.as_str());
        let reject = Reject { row, id };
        let capture = capture
            .as_ref()
            .ok_or_else(|| reject.make(RosterRejectionKind::MissingIdentity))?;
        let link = capture
            .get(1)
            .ok_or_else(|| reject.make(RosterRejectionKind::MissingIdentity))?
            .as_str();
        let id = id.ok_or_else(|| reject.make(RosterRejectionKind::MissingIdentity))?;
        if id.len() > MAX_FIELD_BYTES || link.len() > MAX_FIELD_BYTES {
            return Err(reject.make(RosterRejectionKind::FieldTooLarge));
        }
        let raw_name = capture
            .get(3)
            .ok_or_else(|| reject.make(RosterRejectionKind::MissingName))?
            .as_str()
            .trim();
        let roster_name = parsed_name(raw_name, &reject)?;
        let grad_year = self.year(html, &reject)?;
        let gender = self
            .gender
            .captures(html)
            .and_then(|capture| capture.get(1))
            .map_or(Gender::Unknown, |value| {
                Gender::parse_milesplit(value.as_str())
            });
        let [indoor, outdoor, xc] = active_seasons(self.seasons, html);
        Ok(RosterAthlete {
            name: flip_last_first(&roster_name),
            roster_name,
            gender,
            grad_year,
            athlete_id: id.to_owned(),
            profile_url: link.to_owned(),
            indoor,
            outdoor,
            xc,
        })
    }
    fn year(&self, html: &str, reject: &Reject<'_>) -> Result<GradYear, RosterRejection> {
        let capture = self.graduation.captures(html);
        let value = capture
            .as_ref()
            .and_then(|capture| capture.get(1))
            .map(|value| value.as_str().trim())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| reject.make(RosterRejectionKind::MissingGraduationYear))?;
        value
            .parse::<i16>()
            .ok()
            .and_then(GradYear::new)
            .ok_or_else(|| reject.make(RosterRejectionKind::InvalidGraduationYear))
    }
}

fn parsed_name(raw: &str, reject: &Reject<'_>) -> Result<String, RosterRejection> {
    if raw.len() > MAX_FIELD_BYTES {
        return Err(reject.make(RosterRejectionKind::FieldTooLarge));
    }
    let name = html_unescape(raw);
    if name.trim().is_empty() {
        return Err(reject.make(RosterRejectionKind::MissingName));
    }
    Ok(name)
}

fn active_seasons(regex: &Regex, html: &str) -> [bool; 3] {
    let mut seasons = [false; 3];
    seasons
        .iter_mut()
        .zip(regex.captures_iter(html).take(3))
        .for_each(|(season, capture)| {
            *season = capture.get(2).is_some_and(|value| value.as_str() == "yes");
        });
    seasons
}

fn body_bound(html: &str) -> CrawlResult<()> {
    if html.len() > MAX_BODY_BYTES {
        return Err(resource("roster body bytes", html.len(), MAX_BODY_BYTES));
    }
    Ok(())
}

fn reserve<T>(rows: &mut Vec<T>) -> CrawlResult<()> {
    if rows.len() >= MAX_ROWS {
        return Err(resource("roster rows", rows.len() + 1, MAX_ROWS));
    }
    rows.try_reserve(1)
        .map_err(|_| resource("roster row allocation", rows.len() + 1, MAX_ROWS))
}

fn resource(resource: &'static str, requested: usize, limit: usize) -> CrawlError {
    CrawlError::Resource {
        resource,
        requested,
        limit,
    }
}

#[cfg(test)]
mod ownership_tests;
