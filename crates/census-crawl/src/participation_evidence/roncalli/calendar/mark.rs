use super::super::{bound, invalid};
use census_store::{StoreError, StoreResult};

pub(super) struct Performance {
    pub(super) event_label: String,
    pub(super) original_mark: String,
    pub(super) centiseconds: u32,
    pub(super) place: u16,
}

pub(super) fn read(text: &str, name: &str) -> StoreResult<Performance> {
    let prefix = format!("{name} set a new PR in the ");
    let mut selected = None;
    for line in text.split(" | ") {
        let Some(statement) = line.strip_prefix(&prefix) else {
            continue;
        };
        if selected.is_some() {
            return Err(invalid("Roncalli named performance statement ambiguous"));
        }
        selected = Some(parse(statement)?);
    }
    selected.ok_or_else(|| invalid("Roncalli named performance statement absent"))
}

fn parse(statement: &str) -> StoreResult<Performance> {
    let (event, result) = statement
        .split_once(" of ")
        .ok_or_else(|| invalid("Roncalli named event statement unsupported"))?;
    bound(event.len(), 8, "Roncalli named event label bytes")?;
    if event.is_empty()
        || event.starts_with('0')
        || !event.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(invalid("Roncalli named event label unsupported"));
    }
    let (mark, rank) = result
        .split_once(" to place ")
        .ok_or_else(|| invalid("Roncalli named mark and place unsupported"))?;
    Ok(Performance {
        event_label: event.into(),
        original_mark: mark.into(),
        centiseconds: centiseconds(mark)?,
        place: place(rank)?,
    })
}

fn centiseconds(mark: &str) -> StoreResult<u32> {
    bound(mark.len(), 12, "Roncalli named original mark bytes")?;
    let (minutes, remaining) = mark
        .split_once(':')
        .ok_or_else(|| invalid("Roncalli named original mark unsupported"))?;
    let (seconds, fraction) = remaining
        .split_once('.')
        .ok_or_else(|| invalid("Roncalli named original mark unsupported"))?;
    if minutes.is_empty()
        || minutes.len() > 3
        || seconds.len() != 2
        || fraction.len() != 2
        || !minutes
            .bytes()
            .chain(seconds.bytes())
            .chain(fraction.bytes())
            .all(|byte| byte.is_ascii_digit())
    {
        return Err(invalid("Roncalli named original mark unsupported"));
    }
    let minute: u32 = minutes
        .parse()
        .map_err(|_| invalid("Roncalli mark minute invalid"))?;
    let second: u32 = seconds
        .parse()
        .map_err(|_| invalid("Roncalli mark second invalid"))?;
    let fraction: u32 = fraction
        .parse()
        .map_err(|_| invalid("Roncalli mark fraction invalid"))?;
    if second >= 60 {
        return Err(invalid("Roncalli mark second invalid"));
    }
    minute
        .checked_mul(6000)
        .and_then(|value| {
            second
                .checked_mul(100)
                .and_then(|scaled| value.checked_add(scaled))
        })
        .and_then(|value| value.checked_add(fraction))
        .ok_or(StoreError::CounterOverflow)
}

fn place(rank: &str) -> StoreResult<u16> {
    let digits = rank.chars().take_while(char::is_ascii_digit).count();
    let number = rank
        .get(..digits)
        .ok_or_else(|| invalid("Roncalli place span invalid"))?;
    let value: u16 = number
        .parse()
        .map_err(|_| invalid("Roncalli named place unsupported"))?;
    let suffix = match (value % 100, value % 10) {
        (11..=13, _) => "th",
        (_, 1) => "st",
        (_, 2) => "nd",
        (_, 3) => "rd",
        _ => "th",
    };
    if value == 0 || value > 512 || number.starts_with('0') || rank.get(digits..) != Some(suffix) {
        return Err(invalid("Roncalli named place unsupported"));
    }
    Ok(value)
}
