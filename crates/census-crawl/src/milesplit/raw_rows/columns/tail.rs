#[derive(Debug, Clone, Copy)]
pub(super) enum Layout {
    Unlabelled,
    Qualified {
        heat: Option<usize>,
        wind: Option<usize>,
    },
    Invalid,
}

#[derive(Default)]
pub(super) struct Header {
    heat: Option<usize>,
    wind: Option<usize>,
    other_end: Option<usize>,
    invalid: bool,
}

impl Header {
    pub(super) fn read(&mut self, label: &str, offset: usize) {
        let Some(end) = offset.checked_add(label.chars().count()) else {
            self.invalid = true;
            return;
        };
        if label.eq_ignore_ascii_case("Wind") {
            self.invalid |= self.wind.replace(end).is_some();
        } else if label.eq_ignore_ascii_case("H#") || label.eq_ignore_ascii_case("Heat") {
            self.invalid |= self.heat.replace(end).is_some();
        } else if starts(label, "Wind") || starts(label, "H#") || starts(label, "Heat") {
            self.invalid = true;
        } else {
            self.other_end = Some(self.other_end.map_or(end, |prior| prior.max(end)));
        }
    }

    pub(super) fn finish(&self, mark_end: usize) -> Layout {
        let qualified = self.heat.is_some() || self.wind.is_some();
        if self.invalid
            || qualified && self.other_end.is_some_and(|end| end > mark_end)
            || self.heat.is_some_and(|end| end <= mark_end)
            || self.wind.is_some_and(|end| end <= mark_end)
            || self.heat.is_some() && self.heat == self.wind
        {
            Layout::Invalid
        } else if self.heat.is_none() && self.wind.is_none() {
            Layout::Unlabelled
        } else {
            Layout::Qualified {
                heat: self.heat,
                wind: self.wind,
            }
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct Cells<'a> {
    pub(super) heat: Option<&'a str>,
    pub(super) wind: Option<f64>,
}

pub(super) fn read(line: &str, mark_end: usize, layout: Layout) -> Result<Cells<'_>, &'static str> {
    match layout {
        Layout::Invalid => Err("invalid declared tail columns"),
        Layout::Unlabelled => {
            let tail = line
                .get(column_byte(line, mark_end)..)
                .map_or("", core::convert::identity);
            Ok(Cells {
                heat: tail
                    .split_whitespace()
                    .next()
                    .filter(|token| !token.starts_with('(')),
                wind: None,
            })
        }
        Layout::Qualified { heat, wind } => declared(line, mark_end, heat, wind),
    }
}

fn declared(
    line: &str,
    mark_end: usize,
    heat: Option<usize>,
    wind: Option<usize>,
) -> Result<Cells<'_>, &'static str> {
    let last = heat
        .into_iter()
        .chain(wind)
        .max()
        .ok_or("invalid declared tail columns")?;
    let suffix = line
        .get(column_byte(line, last)..)
        .map_or("", core::convert::identity);
    if !suffix.trim().is_empty() {
        return Err("undeclared result tail content");
    }
    let heat_cell = heat
        .map(|end| field(line, previous(mark_end, end, wind), end))
        .transpose()?;
    let wind_cell = wind
        .map(|end| field(line, previous(mark_end, end, heat), end))
        .transpose()?;
    Ok(Cells {
        heat: parse_heat(heat_cell.map_or("", core::convert::identity))?,
        wind: parse_wind(wind_cell.map_or("", core::convert::identity))?,
    })
}

fn previous(mark: usize, end: usize, other: Option<usize>) -> usize {
    other
        .filter(|other| *other < end)
        .map_or(mark, core::convert::identity)
}

fn field(line: &str, start: usize, end: usize) -> Result<&str, &'static str> {
    let begin = column_byte(line, start);
    let end = column_byte(line, end);
    for boundary in [begin, end] {
        let left = line
            .get(..boundary)
            .and_then(|value| value.chars().next_back());
        let right = line.get(boundary..).and_then(|value| value.chars().next());
        if left.is_some_and(|ch| !ch.is_whitespace()) && right.is_some_and(|ch| !ch.is_whitespace())
        {
            return Err("invalid declared tail columns");
        }
    }
    line.get(begin..end)
        .map(str::trim)
        .ok_or("invalid declared tail columns")
}

fn parse_heat(value: &str) -> Result<Option<&str>, &'static str> {
    if value.is_empty() {
        return Ok(None);
    }
    if value.len() > 16 || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("invalid declared heat field");
    }
    Ok(Some(value))
}

fn parse_wind(value: &str) -> Result<Option<f64>, &'static str> {
    if value.is_empty() {
        return Ok(None);
    }
    if value.len() > 16 {
        return Err("invalid declared wind field");
    }
    let unsigned = value
        .strip_prefix(['+', '-'])
        .map_or(value, core::convert::identity);
    let (whole, fraction) = unsigned
        .split_once('.')
        .map_or((unsigned, None), |(whole, fraction)| {
            (whole, Some(fraction))
        });
    if !digits(whole) || fraction.is_some_and(|fraction| !digits(fraction)) {
        return Err("invalid declared wind field");
    }
    let wind = value
        .parse::<f64>()
        .map_err(|_| "invalid declared wind field")?;
    if !wind.is_finite() {
        return Err("invalid declared wind field");
    }
    Ok(Some(wind))
}

fn digits(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit())
}

fn column_byte(line: &str, column: usize) -> usize {
    if line.is_ascii() {
        column.min(line.len())
    } else {
        line.char_indices()
            .nth(column)
            .map_or(line.len(), |(byte, _)| byte)
    }
}

fn starts(value: &str, prefix: &str) -> bool {
    value
        .get(..prefix.len())
        .is_some_and(|start| start.eq_ignore_ascii_case(prefix))
}
