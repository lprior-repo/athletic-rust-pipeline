use census_domain::model::{GradYear, ProfileDate, SchoolYear};
use census_store::{StoreError, StoreResult};

pub(super) struct ParsedBoundary {
    pub(super) from: ProfileDate,
    pub(super) through: ProfileDate,
    pub(super) documented_summer_end: ProfileDate,
    pub(super) school_year: SchoolYear,
    pub(super) cohort: GradYear,
}

fn invalid(detail: &'static str) -> StoreError {
    StoreError::Refused {
        detail: detail.into(),
    }
}

fn field<'a>(fields: &mut std::str::Split<'a, &'static str>, label: &str) -> StoreResult<&'a str> {
    fields
        .next()
        .and_then(|value| value.strip_prefix(label))
        .ok_or_else(|| invalid("declared window field missing, repeated or unordered"))
}

fn school_year(value: &str) -> StoreResult<SchoolYear> {
    let (start, end) = value
        .split_once('-')
        .ok_or_else(|| invalid("declared window school year missing endpoint"))?;
    if start.len() != 4
        || end.len() != 2
        || !start
            .bytes()
            .chain(end.bytes())
            .all(|byte| byte.is_ascii_digit())
    {
        return Err(invalid("declared window school year notation unsupported"));
    }
    let start = start
        .parse::<i16>()
        .map_err(|_| invalid("declared window school year invalid"))?;
    let year = SchoolYear::new(start)
        .ok_or_else(|| invalid("declared window school year out of range"))?;
    let end = end
        .parse::<i16>()
        .map_err(|_| invalid("declared window school year endpoint invalid"))?;
    let expected = year
        .get()
        .checked_add(1)
        .ok_or(StoreError::CounterOverflow)?
        % 100;
    if end != expected {
        return Err(invalid("declared window school year endpoints disagree"));
    }
    Ok(year)
}

fn date(value: &str) -> StoreResult<ProfileDate> {
    ProfileDate::parse(value).map_err(|_| invalid("declared window date invalid"))
}

pub(super) fn parse(value: &str) -> StoreResult<ParsedBoundary> {
    let mut fields = value.split(" | ");
    if fields.next() != Some("Full documented participation window") {
        return Err(invalid(
            "explicit full participation window declaration missing",
        ));
    }
    let school_year = school_year(field(&mut fields, "school year: ")?)?;
    let cohort = field(&mut fields, "cohort: ")?
        .parse::<i16>()
        .map_err(|_| invalid("declared window cohort invalid"))?;
    let cohort =
        GradYear::new(cohort).ok_or_else(|| invalid("declared window cohort out of range"))?;
    let from = date(field(&mut fields, "from: ")?)?;
    let through = date(field(&mut fields, "through: ")?)?;
    let documented_summer_end = date(field(&mut fields, "documented summer boundary: ")?)?;
    if fields.next().is_some() || from > through || through != documented_summer_end {
        return Err(invalid("declared full window boundaries disagree"));
    }
    Ok(ParsedBoundary {
        from,
        through,
        documented_summer_end,
        school_year,
        cohort,
    })
}

#[cfg(test)]
mod tests;
