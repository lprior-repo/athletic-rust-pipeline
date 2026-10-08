use crate::hytek;
use crate::result_file::ParsedRow;
use census_domain::model::{EventKind, Mark, TimingMethod};

const BOUNDS: usize = 5;

#[derive(Debug, Clone, Copy)]
pub(super) struct Columns {
    name: usize,
    grade: Option<usize>,
    team: usize,
    mark: MarkColumn,
}

#[derive(Debug, Clone, Copy)]
struct MarkColumn {
    start: usize,
    end: usize,
}

impl MarkColumn {
    fn from_label(label: (usize, usize)) -> Option<Self> {
        let (label_start, label_end) = label;
        let width = label_end.checked_sub(label_start)?;
        Some(Self {
            start: label_start.saturating_sub(width.saturating_add(1)),
            end: label_end,
        })
    }
}

pub(super) fn header_columns(line: &str) -> Option<Columns> {
    let first = line.split_whitespace().next()?;
    if !["Pl", "Place", "Name", "Athlete"]
        .iter()
        .any(|label| first.eq_ignore_ascii_case(label))
    {
        return None;
    }
    let mut labels = HeaderLabels::default();
    let mut start = None;
    for (column, (byte, ch)) in line.char_indices().enumerate() {
        if ch.is_whitespace() {
            if let Some((begin, offset)) = start.take() {
                labels.read(line.get(begin..byte)?, offset);
            }
        } else if start.is_none() {
            start = Some((byte, column));
        }
    }
    if let Some((begin, offset)) = start {
        labels.read(line.get(begin..)?, offset);
    }
    let columns = Columns {
        name: labels.name?,
        grade: labels.grade,
        team: labels.team?,
        mark: MarkColumn::from_label(labels.mark?)?,
    };
    (columns.name < columns.team
        && columns.team < columns.mark.start
        && columns
            .grade
            .is_none_or(|offset| columns.name < offset && offset < columns.team))
    .then_some(columns)
}

#[derive(Default)]
struct HeaderLabels {
    name: Option<usize>,
    grade: Option<usize>,
    team: Option<usize>,
    mark: Option<(usize, usize)>,
}

impl HeaderLabels {
    fn read(&mut self, label: &str, offset: usize) {
        if label.eq_ignore_ascii_case("Name") || label.eq_ignore_ascii_case("Athlete") {
            self.name = Some(offset);
        } else if label.eq_ignore_ascii_case("Yr") || label.eq_ignore_ascii_case("Grade") {
            self.grade = Some(offset);
        } else if label.eq_ignore_ascii_case("Team") || label.eq_ignore_ascii_case("School") {
            self.team = Some(offset);
        } else if label.eq_ignore_ascii_case("Time") || label.eq_ignore_ascii_case("Mark") {
            self.mark = Some((offset, offset.saturating_add(label.chars().count())));
        }
    }
}

pub(super) struct Cells<'a> {
    place: &'a str,
    name: &'a str,
    pub(super) grade: &'a str,
    team: &'a str,
    mark: Option<&'a str>,
    heat: Option<&'a str>,
    overflowed: bool,
}

pub(super) fn row_cells(line: &str, columns: Columns) -> Cells<'_> {
    let grade_end = columns.grade.map_or(columns.team, |value| value);
    let ends = [
        columns.name,
        grade_end,
        columns.team,
        columns.mark.start,
        columns.mark.end,
    ];
    let [name, grade, team, mark_start, mark_end] = boundary_bytes(line, ends);
    let token = mark_token(line, mark_end);
    let team_end = token.map_or(mark_start, |(start, _)| start.min(mark_start).max(team));
    let mark = token.and_then(|(_, token)| mark_shaped(token).then_some(token));
    let tail = line
        .get(mark_end..)
        .map_or(Default::default(), core::convert::identity);
    Cells {
        place: line
            .get(..name)
            .map_or(Default::default(), core::convert::identity)
            .trim(),
        name: line
            .get(name..grade)
            .map_or(Default::default(), core::convert::identity)
            .trim(),
        grade: line
            .get(grade..team)
            .map_or(Default::default(), core::convert::identity)
            .trim(),
        team: line
            .get(team..team_end)
            .map_or(Default::default(), core::convert::identity)
            .trim(),
        heat: mark.and_then(|_| heat_token(tail)),
        overflowed: layout_overflows(line, mark_start, mark_end, token),
        mark,
    }
}

fn boundary_bytes(line: &str, ends: [usize; BOUNDS]) -> [usize; BOUNDS] {
    if line.is_ascii() {
        return ends.map(|offset| offset.min(line.len()));
    }
    let limit = ends.last().copied().map_or(usize::MAX, |value| value);
    let mut bytes = [line.len(); BOUNDS];
    for (column, (byte, _)) in line.char_indices().enumerate() {
        for (end, boundary) in ends.iter().zip(bytes.iter_mut()) {
            if column == *end {
                *boundary = byte;
            }
        }
        if column >= limit {
            break;
        }
    }
    bytes
}

fn mark_token(line: &str, mark_end: usize) -> Option<(usize, &str)> {
    let head = line.get(..mark_end)?;
    if head.chars().next_back()?.is_whitespace() {
        return None;
    }
    let start = head
        .char_indices()
        .rev()
        .find_map(|(byte, ch)| {
            ch.is_whitespace()
                .then_some(byte.saturating_add(ch.len_utf8()))
        })
        .map_or(0, |value| value);
    line.get(start..mark_end).map(|token| (start, token))
}

fn layout_overflows(
    line: &str,
    mark_start: usize,
    mark_end: usize,
    token: Option<(usize, &str)>,
) -> bool {
    let trailing = line
        .get(mark_end..)
        .and_then(|tail| tail.chars().next())
        .is_some_and(|ch| !ch.is_whitespace());
    let scattered = line
        .get(mark_start..mark_end)
        .map_or(Default::default(), core::convert::identity)
        .split_whitespace()
        .nth(1)
        .is_some();
    let covered = token.is_some_and(|(start, _)| start < mark_start);
    let touching = line
        .get(..mark_start)
        .and_then(|head| head.chars().next_back())
        .is_some_and(|ch| !ch.is_whitespace());
    trailing || scattered || (touching && !covered)
}

fn heat_token(tail: &str) -> Option<&str> {
    tail.split_whitespace()
        .next()
        .filter(|token| !token.starts_with('('))
}

fn mark_shaped(token: &str) -> bool {
    if token.eq_ignore_ascii_case("NT")
        || hytek::NO_MARK
            .iter()
            .any(|value| token.eq_ignore_ascii_case(value))
    {
        return true;
    }
    let numeric = token.trim_start_matches(['J', 'j']);
    numeric.starts_with(|ch: char| ch.is_ascii_digit()) && numeric.contains([':', '.', '-', '\''])
}

pub(super) fn build_row(cells: &Cells<'_>, kind: &EventKind) -> Result<ParsedRow, &'static str> {
    if cells.overflowed {
        return Err("row did not fit the column map");
    }
    if cells.name.is_empty() {
        return Err("missing name");
    }
    let mark = cells.mark.ok_or("missing or malformed mark")?;
    let place = match cells.place {
        "" | "--" => None,
        value => Some(value.parse::<u16>().map_err(|_| "invalid place")?),
    };
    let (mark, timing) = mark_of(mark, kind);
    Ok(ParsedRow {
        place,
        name: cells.name.to_string(),
        grade: hytek::grade_from_token(cells.grade),
        school: cells.team.to_string(),
        mark,
        timing,
        wind_mps: None,
        heat: cells.heat.map(str::to_string),
        points: None,
        legs: Vec::new(),
    })
}

fn mark_of(value: &str, kind: &EventKind) -> (Mark, Option<TimingMethod>) {
    if value.eq_ignore_ascii_case("NT")
        || hytek::NO_MARK
            .iter()
            .any(|mark| value.eq_ignore_ascii_case(mark))
    {
        return (Mark::Raw(value.to_string()), None);
    }
    let parsed = if kind.is_field() {
        field_mark(value)
    } else {
        crate::milesplit::parse_published_time(value)
            .map(|(seconds, timing)| (Mark::TimeSeconds(seconds), timing))
    };
    match parsed {
        Some(parsed_mark) => parsed_mark,
        None => (Mark::Raw(value.to_string()), None),
    }
}

fn field_mark(value: &str) -> Option<(Mark, Option<TimingMethod>)> {
    crate::milesplit::parse_published_metric_distance(value)
        .map(Mark::DistanceMetres)
        .or_else(|| hytek::parse_field_mark(value))
        .map(|mark| (mark, None))
}
