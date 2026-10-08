use crate::{CrawlError, CrawlResult};
use census_domain::model::CompetitionLevel;
use census_domain::UsJurisdiction;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeetRow {
    pub tenant: String,
    pub athleticlive_meet_id: String,
    pub athleticnet_meet_id: Option<String>,
    pub name: String,
    pub city_state: Option<String>,
    pub state_code: UsJurisdiction,
    pub start: String,
    pub end: Option<String>,
    pub has_results: bool,
}

pub fn implausible_year(date: &str) -> bool {
    date.get(..4)
        .and_then(|year| year.parse::<u16>().ok())
        .is_none_or(|year| !(2015..=2030).contains(&year))
}

fn date_prefix(value: &str) -> Option<String> {
    let value = value.trim().get(..10)?;
    chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .ok()
        .filter(|date| date.to_string() == value)?;
    Some(value.to_string())
}

pub fn infer_level(name: &str) -> CompetitionLevel {
    let lowered = name.to_ascii_lowercase();
    let state = lowered.contains("state")
        && ["final", "championship", "champ", "meet"]
            .iter()
            .any(|part| lowered.contains(part));
    [
        (state, CompetitionLevel::State),
        (lowered.contains("sectional"), CompetitionLevel::Sectional),
        (lowered.contains("regional"), CompetitionLevel::Regional),
        (lowered.contains("district"), CompetitionLevel::District),
        (
            ["conference", "conf "]
                .iter()
                .any(|part| lowered.contains(part)),
            CompetitionLevel::Conference,
        ),
        (
            [
                "invitational",
                "invite",
                "classic",
                "relays",
                "festival",
                "open",
            ]
            .iter()
            .any(|part| lowered.contains(part)),
            CompetitionLevel::Invitational,
        ),
        (lowered.contains("dual"), CompetitionLevel::Dual),
    ]
    .into_iter()
    .find(|(matches, _)| *matches)
    .map_or(CompetitionLevel::Unknown, |(_, level)| level)
}

fn harvest_schema(detail: String) -> CrawlError {
    CrawlError::Schema {
        url: "athleticlive harvest CSV".to_string(),
        detail,
    }
}

fn field<'a>(
    columns: &csv::StringRecord,
    fields: &'a csv::StringRecord,
    want: &str,
) -> Option<&'a str> {
    columns
        .iter()
        .position(|column| column.eq_ignore_ascii_case(want))
        .and_then(|index| fields.get(index))
        .map(str::trim)
}

pub(super) fn require_columns(columns: &csv::StringRecord) -> CrawlResult<()> {
    ["tenant", "athleticlive_meet_id", "name", "state", "start"]
        .into_iter()
        .try_for_each(|required| {
            if columns
                .iter()
                .any(|column| column.eq_ignore_ascii_case(required))
            {
                Ok(())
            } else {
                Err(harvest_schema(format!(
                    "AthleticLIVE CSV is missing required column `{required}`"
                )))
            }
        })
}

fn required_field(
    columns: &csv::StringRecord,
    fields: &csv::StringRecord,
    column: &str,
    row: usize,
) -> CrawlResult<String> {
    field(columns, fields, column)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| harvest_schema(format!("row {row} has no {column}")))
}

pub(super) fn meet_row(
    columns: &csv::StringRecord,
    fields: &csv::StringRecord,
    row: usize,
) -> CrawlResult<MeetRow> {
    let tenant = required_field(columns, fields, "tenant", row)?;
    let athleticlive_meet_id = required_field(columns, fields, "athleticlive_meet_id", row)?;
    let name = required_field(columns, fields, "name", row)?;
    let state_code = field(columns, fields, "state")
        .and_then(UsJurisdiction::parse)
        .ok_or_else(|| harvest_schema(format!("row {row} has an invalid state")))?;
    let start = field(columns, fields, "start")
        .and_then(date_prefix)
        .ok_or_else(|| harvest_schema(format!("row {row} has an invalid start date")))?;
    let end = field(columns, fields, "end")
        .filter(|value| !value.is_empty())
        .map(|value| {
            date_prefix(value)
                .ok_or_else(|| harvest_schema(format!("row {row} has an invalid end date")))
        })
        .transpose()?
        .filter(|end| end != &start);
    Ok(MeetRow {
        tenant,
        athleticlive_meet_id,
        name,
        state_code,
        start,
        end,
        athleticnet_meet_id: field(columns, fields, "athleticnet_meet_id")
            .filter(|value| !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()))
            .map(str::to_string),
        city_state: field(columns, fields, "city_state")
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        has_results: field(columns, fields, "has_results")
            .is_some_and(|value| value.eq_ignore_ascii_case("true")),
    })
}

pub fn parse_meets_csv(body: &str) -> CrawlResult<Vec<MeetRow>> {
    if body.len() > crate::coach_contacts::artifact::MAX_RECORD_BYTES {
        return Err(CrawlError::Resource {
            resource: "LIVE CSV parse bytes",
            requested: body.len(),
            limit: crate::coach_contacts::artifact::MAX_RECORD_BYTES,
        });
    }
    let mut reader = csv::Reader::from_reader(body.as_bytes());
    let columns = reader
        .headers()
        .map_err(|error| harvest_schema(error.to_string()))?
        .clone();
    require_columns(&columns)?;
    reader
        .records()
        .enumerate()
        .try_fold(Vec::new(), |mut rows, (index, record)| {
            let record = record.map_err(|error| harvest_schema(error.to_string()))?;
            let number = index
                .checked_add(2)
                .ok_or_else(|| harvest_schema("CSV row number overflow".to_string()))?;
            let row = meet_row(&columns, &record, number)?;
            rows.try_reserve(1).map_err(|_| CrawlError::Resource {
                resource: "LIVE CSV rows",
                requested: number,
                limit: body.len(),
            })?;
            rows.push(row);
            Ok(rows)
        })
}
