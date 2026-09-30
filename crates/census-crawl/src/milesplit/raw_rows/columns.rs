use crate::hytek;
use crate::result_file::ParsedRow;
use census_domain::model::{EventKind, Mark};

const PLACE_START: usize = 0;
const PLACE_WIDTH: usize = 4;
const NAME_START: usize = 5;
const GRADE_WIDTH: usize = 2;
const MARK_WIDTH: usize = 9;
const HEAT_WIDTH: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Layout {
    Ohio,
    NC,
}

impl Layout {
    fn name_width(&self) -> usize {
        match self {
            Layout::Ohio => 25,
            Layout::NC => 21,
        }
    }

    fn grade_start(&self) -> usize {
        match self {
            Layout::Ohio => 31,
            Layout::NC => 26,
        }
    }

    fn team_start(&self) -> usize {
        match self {
            Layout::Ohio => 34,
            Layout::NC => 29,
        }
    }

    fn team_width(&self) -> usize {
        match self {
            Layout::Ohio => 40,
            Layout::NC => 23,
        }
    }

    fn mark_start(&self) -> usize {
        match self {
            Layout::Ohio => 75,
            Layout::NC => 52,
        }
    }

    fn heat_start(&self) -> usize {
        match self {
            Layout::Ohio => 90,
            Layout::NC => 63,
        }
    }
}

pub(super) fn detect_layout(block: &str) -> Layout {
    for line in block.lines() {
        if line.trim_start().starts_with("====") {
            return if line.trim().len() < 80 {
                Layout::NC
            } else {
                Layout::Ohio
            };
        }
    }
    Layout::Ohio
}

pub(super) fn row_candidate(line: &str, layout: Layout) -> Option<Cells<'_>> {
    let mark = cell(line, layout.mark_start(), MARK_WIDTH)?;
    if mark.trim().is_empty() {
        return None;
    }
    Some(Cells {
        place: cell(line, PLACE_START, PLACE_WIDTH),
        name: cell(line, NAME_START, layout.name_width()),
        grade: cell(line, layout.grade_start(), GRADE_WIDTH),
        team: cell(line, layout.team_start(), layout.team_width()),
        mark,
        heat: cell(line, layout.heat_start(), HEAT_WIDTH),
        separator: cell(line, layout.mark_start() - 1, 1),
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
    if name.is_empty() {
        return None;
    }
    Some(ParsedRow {
        place,
        name: name.to_string(),
        grade: cells.grade.map(str::trim).and_then(hytek::grade_from_token),
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
