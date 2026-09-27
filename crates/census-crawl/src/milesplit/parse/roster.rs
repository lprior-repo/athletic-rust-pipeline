use census_domain::model::{flip_last_first, Gender, GradYear};
use regex::Regex;

use super::super::roster::{
    RosterQuarantine, RosterRejection, RosterRejectionKind, RosterRowLocator, RosterVerdict,
};
use super::super::wire::{Roster, RosterAthlete, TeamRef};
use super::{
    athlete_link_regex, athlete_row_regex, gender_cell_regex, grad_cell_regex, html_unescape,
    season_cell_regex,
};
use crate::{CrawlError, CrawlResult};

pub fn parse_roster(html: &str, team: TeamRef) -> CrawlResult<RosterVerdict> {
    let row_regex = athlete_row_regex()?;
    let fields = Fields {
        link: athlete_link_regex()?,
        gender: gender_cell_regex()?,
        graduation: grad_cell_regex()?,
        seasons: season_cell_regex()?,
    };
    let (athletes, rejected) = row_regex.captures_iter(html).enumerate().try_fold(
        (Vec::new(), Vec::new()),
        |(mut athletes, mut rejected), (ordinal, row)| {
            let row = row.get(1).ok_or_else(|| CrawlError::Invariant {
                detail: "roster row expression omitted its required capture".to_string(),
            })?;
            let locator = RosterRowLocator {
                ordinal: u32::try_from(ordinal).map_err(|_| CrawlError::Arithmetic {
                    detail: "roster row ordinal exceeds u32".to_string(),
                })?,
                byte_offset: row.start(),
                byte_length: row.len(),
            };
            match fields.athlete(row.as_str(), locator) {
                Ok(athlete) => athletes.push(athlete),
                Err(rejection) => rejected.push(rejection),
            }
            Ok::<_, CrawlError>((athletes, rejected))
        },
    )?;
    if athletes.is_empty() {
        return Ok(RosterVerdict::Quarantined {
            reason: if rejected.is_empty() && !html.contains("rosterDataset") {
                RosterQuarantine::UnknownTemplate
            } else {
                RosterQuarantine::NoReadableRows
            },
            rejected,
        });
    }
    let roster = Roster { team, athletes };
    Ok(if rejected.is_empty() {
        RosterVerdict::Complete { roster }
    } else {
        RosterVerdict::Partial { roster, rejected }
    })
}

struct Fields<'a> {
    link: &'a Regex,
    gender: &'a Regex,
    graduation: &'a Regex,
    seasons: &'a Regex,
}

impl Fields<'_> {
    fn athlete(&self, html: &str, row: RosterRowLocator) -> Result<RosterAthlete, RosterRejection> {
        let athlete = self.link.captures(html);
        let id = athlete.as_ref().and_then(|link| link.get(2));
        let reject = |kind| RosterRejection {
            row,
            athlete_id: id.map(|id| id.as_str().to_string()),
            kind,
        };
        let Some((link, id)) = athlete
            .as_ref()
            .and_then(|link| Some((link.get(1)?, link.get(2)?)))
        else {
            return Err(reject(RosterRejectionKind::MissingIdentity));
        };
        let roster_name = athlete
            .as_ref()
            .and_then(|link| link.get(3))
            .ok_or_else(|| reject(RosterRejectionKind::MissingName))?;
        let roster_name = html_unescape(roster_name.as_str().trim());
        if roster_name.trim().is_empty() {
            return Err(reject(RosterRejectionKind::MissingName));
        }
        let graduation = self.graduation.captures(html);
        let graduation = graduation
            .as_ref()
            .and_then(|capture| capture.get(1))
            .map(|value| value.as_str().trim())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| reject(RosterRejectionKind::MissingGraduationYear))?;
        let grad_year = graduation
            .parse::<i16>()
            .ok()
            .and_then(GradYear::new)
            .ok_or_else(|| reject(RosterRejectionKind::InvalidGraduationYear))?;
        let gender = self
            .gender
            .captures(html)
            .and_then(|capture| {
                capture
                    .get(1)
                    .map(|value| Gender::parse_milesplit(value.as_str()))
            })
            .map_or(Gender::Unknown, |gender| gender);
        let [indoor, outdoor, xc] = active_seasons(self.seasons, html);
        Ok(RosterAthlete {
            name: flip_last_first(&roster_name),
            roster_name,
            gender,
            grad_year,
            athlete_id: id.as_str().to_string(),
            profile_url: link.as_str().to_string(),
            indoor,
            outdoor,
            xc,
        })
    }
}

fn active_seasons(regex: &Regex, html: &str) -> [bool; 3] {
    regex.captures_iter(html).take(3).enumerate().fold(
        [false; 3],
        |mut seasons, (index, capture)| {
            seasons[index] = capture.get(2).is_some_and(|value| value.as_str() == "yes");
            seasons
        },
    )
}
