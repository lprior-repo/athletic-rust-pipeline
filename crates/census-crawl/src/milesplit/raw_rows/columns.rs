use crate::hytek;
use crate::result_file::ParsedRow;
use census_domain::model::{EventKind, Grade, Mark};

pub(super) const PLACE_START: usize = 0;
const PLACE_WIDTH: usize = 4;
pub(super) const NAME_START: usize = 5;
const NAME_WIDTH: usize = 25;
const GRADE_START: usize = 31;
const GRADE_WIDTH: usize = 2;
const TEAM_START: usize = 34;
const TEAM_WIDTH: usize = 40;
pub(super) const MARK_START: usize = 75;
const MARK_WIDTH: usize = 9;
const HEAT_START: usize = 90;
const HEAT_WIDTH: usize = 2;
pub(super) const MARK_SEPARATOR: usize = 74;
const MAX_GRADE: u8 = 12;

pub(super) fn row_candidate(line: &str) -> Option<Cells<'_>> {
    let mark = cell(line, MARK_START, MARK_WIDTH)?;
    if mark.trim().is_empty() {
        return None;
    }
    Some(Cells {
        place: cell(line, PLACE_START, PLACE_WIDTH),
        name: cell(line, NAME_START, NAME_WIDTH),
        grade: cell(line, GRADE_START, GRADE_WIDTH),
        team: cell(line, TEAM_START, TEAM_WIDTH),
        mark,
        heat: cell(line, HEAT_START, HEAT_WIDTH),
        separator: cell(line, MARK_SEPARATOR, 1),
    })
}

pub(super) struct Cells<'a> {
    pub(super) place: Option<&'a str>,
    pub(super) name: Option<&'a str>,
    grade: Option<&'a str>,
    team: Option<&'a str>,
    mark: &'a str,
    heat: Option<&'a str>,
    separator: Option<&'a str>,
}

pub(super) fn build_row(cells: &Cells<'_>, kind: &EventKind) -> Option<ParsedRow> {
    let separator_blank = cells
        .separator
        .is_none_or(|column| column.trim().is_empty());
    let mark = cells.mark.trim();
    if !separator_blank || !mark.starts_with(|ch: char| ch.is_ascii_alphanumeric()) {
        return None;
    }
    let place = cells
        .place
        .map(str::trim)
        .and_then(|value| value.parse::<u16>().ok());
    let name = cells.name.map(str::trim).unwrap_or_default();
    if place.is_none() || name.is_empty() {
        return None;
    }
    Some(ParsedRow {
        place,
        name: name.to_string(),
        grade: cells.grade.map(str::trim).and_then(grade_of),
        school: cells.team.map(str::trim).unwrap_or_default().to_string(),
        mark: mark_of(mark, kind),
        wind_mps: None,
        heat: cells
            .heat
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        points: None,
        legs: Vec::new(),
    })
}

fn grade_of(value: &str) -> Option<Grade> {
    let parsed = value.parse::<u8>().ok()?;
    (parsed <= MAX_GRADE).then_some(parsed).and_then(Grade::new)
}

fn mark_of(value: &str, kind: &EventKind) -> Mark {
    if hytek::NO_MARK.contains(&value.to_ascii_uppercase().as_str()) {
        return Mark::Raw(value.to_string());
    }
    let field = || hytek::parse_field_mark(value);
    let time = || hytek::parse_time(value).map(Mark::TimeSeconds);
    let parsed = if kind.is_field() {
        field().or_else(time)
    } else {
        time().or_else(field)
    };
    parsed.unwrap_or_else(|| Mark::Raw(value.to_string()))
}

fn cell(line: &str, start: usize, width: usize) -> Option<&str> {
    if width == 0 {
        return None;
    }
    let mut offsets = line.char_indices().map(|(index, _)| index);
    let begin = offsets.nth(start)?;
    let end = offsets.nth(width.saturating_sub(1)).unwrap_or(line.len());
    line.get(begin..end)
}
