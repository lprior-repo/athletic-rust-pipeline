use super::{
    html_unescape, profile_canonical_regex, profile_cell_regex, profile_city_regex,
    profile_gender_regex, profile_grad_regex, profile_name_regex, profile_row_regex,
    profile_school_regex, profile_table_regex, profile_team_id_regex,
};
use crate::{CrawlError, CrawlResult};
use census_domain::model::{Gender, GradYear};

const MAX_BODY_BYTES: usize = 8 * 1024 * 1024;
const MAX_ROWS: usize = 500;
const MAX_ROW_BYTES: usize = 64 * 1024;
const MAX_FIELD_BYTES: usize = 4096;
const PR_CELLS: usize = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileHeader {
    pub athlete_id: String,
    pub profile_url: String,
    pub name: String,
    pub school_name: String,
    pub team_id: String,
    pub team_url: String,
    pub grad_year: GradYear,
    pub city_state: Option<String>,
    pub gender: Gender,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfilePersonalRecord {
    pub mode: String,
    pub level: String,
    pub event_label: String,
    pub mark_label: String,
    pub state_rank: Option<String>,
    pub national_rank: Option<String>,
    pub date_label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedProfile {
    pub header: ProfileHeader,
    pub records: Vec<ProfilePersonalRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileVerdict {
    Complete { profile: ParsedProfile },
    Partial {
        profile: ParsedProfile,
        rejected: Vec<ProfileRowRejection>,
    },
    Quarantined {
        reason: ProfileQuarantine,
        rejected: Vec<ProfileRowRejection>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileQuarantine {
    MissingIdentity,
    MissingName,
    MissingSchool,
    MissingGraduationYear,
    InvalidGraduationYear,
    UnknownTemplate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileRowRejection {
    pub ordinal: u32,
    pub kind: ProfileRowRejectionKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileRowRejectionKind {
    MissingEvent,
    MissingMark,
    RowTooLarge,
    FieldTooLarge,
}

/// Parse one captured MileSplit athlete profile page.
///
/// The header binds the athlete to a school team, graduation year and
/// gender; the `prTable` rows carry the visible Personal Records table.
/// Event and mark labels stay textual here: the map layer owns the
/// `EventKind`/mark interpretation it already applies to result rows.
pub fn parse_profile(html: &str) -> CrawlResult<ProfileVerdict> {
    body_bound(html)?;
    let header = match read_header(html)? {
        HeaderRead::Header(header) => header,
        HeaderRead::Quarantined(reason) => {
            return Ok(ProfileVerdict::Quarantined {
                reason,
                rejected: Vec::new(),
            });
        }
    };
    let Some(table) = profile_table_regex()?
        .captures(html)
        .and_then(|capture| capture.get(1))
        .map(|body| body.as_str())
    else {
        return Ok(ProfileVerdict::Quarantined {
            reason: ProfileQuarantine::UnknownTemplate,
            rejected: Vec::new(),
        });
    };
    let mut parsed = Parsed::default();
    for (ordinal, capture) in profile_row_regex()?.captures_iter(table).enumerate() {
        let ordinal = u32::try_from(ordinal)
            .map_err(|_| resource("profile row ordinal", ordinal, MAX_ROWS))?;
        if ordinal >= MAX_ROWS as u32 {
            break;
        }
        let row = capture.get(3).map_or("", |row| row.as_str());
        if row.len() > MAX_ROW_BYTES {
            parsed.reject(ordinal, ProfileRowRejectionKind::RowTooLarge);
            continue;
        }
        let mode = field(capture.get(1));
        let level = field(capture.get(2));
        match read_record(row, mode, level) {
            Ok(Some(record)) => parsed.admit(record)?,
            Ok(None) => {}
            Err(kind) => parsed.reject(ordinal, kind),
        }
    }
    Ok(parsed.verdict(header))
}

#[derive(Default)]
struct Parsed {
    records: Vec<ProfilePersonalRecord>,
    rejected: Vec<ProfileRowRejection>,
}

impl Parsed {
    fn admit(&mut self, record: ProfilePersonalRecord) -> CrawlResult<()> {
        if self.records.len() >= MAX_ROWS {
            return Err(resource("profile records", self.records.len() + 1, MAX_ROWS));
        }
        self.records
            .try_reserve(1)
            .map_err(|_| resource("profile record allocation", self.records.len() + 1, MAX_ROWS))?;
        self.records.push(record);
        Ok(())
    }

    fn reject(&mut self, ordinal: u32, kind: ProfileRowRejectionKind) {
        if self.rejected.len() < MAX_ROWS {
            self.rejected.push(ProfileRowRejection { ordinal, kind });
        }
    }

    fn verdict(self, header: ProfileHeader) -> ProfileVerdict {
        let profile = ParsedProfile {
            header,
            records: self.records,
        };
        if self.rejected.is_empty() {
            ProfileVerdict::Complete { profile }
        } else {
            ProfileVerdict::Partial {
                profile,
                rejected: self.rejected,
            }
        }
    }
}

enum HeaderRead {
    Header(ProfileHeader),
    Quarantined(ProfileQuarantine),
}

fn read_header(html: &str) -> CrawlResult<HeaderRead> {
    let Some(canonical) = profile_canonical_regex()?.captures(html) else {
        return Ok(HeaderRead::Quarantined(ProfileQuarantine::MissingIdentity));
    };
    let Some(profile_url) = bounded_field(canonical.get(1)) else {
        return Ok(HeaderRead::Quarantined(ProfileQuarantine::MissingIdentity));
    };
    let athlete_id = canonical
        .get(2)
        .map_or("", |id| id.as_str())
        .to_string();
    if athlete_id.is_empty() {
        return Ok(HeaderRead::Quarantined(ProfileQuarantine::MissingIdentity));
    }
    let Some(name) = profile_name_regex()?
        .captures(html)
        .and_then(|capture| capture.get(1))
        .map(|name| html_unescape(name.as_str()).trim().to_string())
        .filter(|name| !name.is_empty())
    else {
        return Ok(HeaderRead::Quarantined(ProfileQuarantine::MissingName));
    };
    if name.len() > MAX_FIELD_BYTES {
        return Ok(HeaderRead::Quarantined(ProfileQuarantine::MissingName));
    }
    let Some(school) = profile_school_regex()?.captures(html) else {
        return Ok(HeaderRead::Quarantined(ProfileQuarantine::MissingSchool));
    };
    let Some(team_url) = bounded_field(school.get(1)) else {
        return Ok(HeaderRead::Quarantined(ProfileQuarantine::MissingSchool));
    };
    let school_name = school
        .get(2)
        .map(|name| html_unescape(name.as_str()).trim().to_string())
        .filter(|name| !name.is_empty() && name.len() <= MAX_FIELD_BYTES)
        .ok_or(ProfileQuarantine::MissingSchool);
    let school_name = match school_name {
        Ok(name) => name,
        Err(reason) => return Ok(HeaderRead::Quarantined(reason)),
    };
    let Some(team_id) = profile_team_id_regex()?
        .captures(&team_url)
        .and_then(|capture| capture.get(1))
        .map(|id| id.as_str().to_string())
    else {
        return Ok(HeaderRead::Quarantined(ProfileQuarantine::MissingSchool));
    };
    let grad = profile_grad_regex()?
        .captures(html)
        .and_then(|capture| capture.get(1))
        .map(|grad| grad.as_str());
    let grad_year = match grad {
        None => return Ok(HeaderRead::Quarantined(ProfileQuarantine::MissingGraduationYear)),
        Some(raw) => raw
            .parse::<i16>()
            .ok()
            .and_then(GradYear::new)
            .ok_or(ProfileQuarantine::InvalidGraduationYear),
    };
    let grad_year = match grad_year {
        Ok(year) => year,
        Err(reason) => return Ok(HeaderRead::Quarantined(reason)),
    };
    let city_state = profile_city_regex()?
        .captures(html)
        .and_then(|capture| capture.get(1))
        .map(|city| html_unescape(city.as_str()).trim().to_string())
        .filter(|city| !city.is_empty() && city.len() <= MAX_FIELD_BYTES);
    let gender = profile_gender_regex()?
        .captures(html)
        .and_then(|capture| capture.get(1))
        .map_or(Gender::Unknown, |value| {
            Gender::parse_milesplit(value.as_str())
        });
    Ok(HeaderRead::Header(ProfileHeader {
        athlete_id,
        profile_url,
        name,
        school_name,
        team_id,
        team_url,
        grad_year,
        city_state,
        gender,
    }))
}

fn read_record(
    row: &str,
    mode: String,
    level: String,
) -> Result<Option<ProfilePersonalRecord>, ProfileRowRejectionKind> {
    let cells: Vec<String> = profile_cell_regex()
        .map_err(|_| ProfileRowRejectionKind::MissingEvent)?
        .captures_iter(row)
        .take(PR_CELLS + 1)
        .map(|capture| {
            capture
                .get(1)
                .map_or(String::new(), |cell| clean_cell(cell.as_str()))
        })
        .collect();
    if cells.len() != PR_CELLS {
        return Ok(None);
    }
    let mut cells = cells.into_iter();
    let event_label = cells.next().unwrap_or_default();
    let mark_label = cells.next().unwrap_or_default();
    let state_rank = optional_cell(cells.next().unwrap_or_default());
    let national_rank = optional_cell(cells.next().unwrap_or_default());
    let date_label = optional_cell(cells.next().unwrap_or_default());
    if event_label.is_empty() {
        return Err(ProfileRowRejectionKind::MissingEvent);
    }
    if mark_label.is_empty() {
        return Err(ProfileRowRejectionKind::MissingMark);
    }
    if event_label.len() > MAX_FIELD_BYTES || mark_label.len() > MAX_FIELD_BYTES {
        return Err(ProfileRowRejectionKind::FieldTooLarge);
    }
    Ok(Some(ProfilePersonalRecord {
        mode,
        level,
        event_label,
        mark_label,
        state_rank,
        national_rank,
        date_label,
    }))
}

fn clean_cell(raw: &str) -> String {
    let mut text = String::with_capacity(raw.len());
    let mut in_tag = false;
    for ch in raw.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => text.push(ch),
            _ => {}
        }
    }
    html_unescape(&text).trim().to_string()
}

fn optional_cell(cell: String) -> Option<String> {
    let cell = cell.trim().to_string();
    if cell.is_empty()
        || cell == "—"
        || cell == "&mdash;"
        || cell == "-"
        || cell.len() > MAX_FIELD_BYTES
    {
        None
    } else {
        Some(cell)
    }
}

fn field(capture: Option<regex::Match<'_>>) -> String {
    capture.map_or(String::new(), |value| value.as_str().to_string())
}

fn bounded_field(capture: Option<regex::Match<'_>>) -> Option<String> {
    let value = capture.map_or("", |value| value.as_str()).to_string();
    if value.is_empty() || value.len() > MAX_FIELD_BYTES {
        None
    } else {
        Some(value)
    }
}

fn body_bound(html: &str) -> CrawlResult<()> {
    if html.len() > MAX_BODY_BYTES {
        return Err(resource("profile body bytes", html.len(), MAX_BODY_BYTES));
    }
    Ok(())
}

fn resource(resource: &'static str, requested: usize, limit: usize) -> CrawlError {
    CrawlError::Resource {
        resource,
        requested,
        limit,
    }
}

#[cfg(test)]
mod tests;
