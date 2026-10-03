use census_domain::model::SchoolYear;
use census_domain::school_directory::SchoolName;

use crate::directory::{compile_pattern, group};
use crate::CrawlResult;

use super::{fields, parse};

pub(super) struct PublishedStaffYear {
    pub year: SchoolYear,
    pub statement: String,
}

pub(super) fn read(text: &str, school: &SchoolName) -> CrawlResult<Option<PublishedStaffYear>> {
    let pattern = compile_pattern(
        r"(?s)<em>\s*(Staff information for the ([0-9]{4})-([0-9]{4}) school year is displayed as it is entered/verified by ([^<]+) administration\.)\s*</em>",
        "tssaa published staff year",
    )?;
    let captures: Vec<_> = pattern.captures_iter(text).take(2).collect();
    let Some(capture) = captures.first() else {
        return if text.contains("Staff information for") {
            Err(parse::artifact("malformed published staff-year statement"))
        } else {
            Ok(None)
        };
    };
    if captures.len() != 1 {
        return Err(parse::artifact("multiple published staff-year statements"));
    }
    let claim = parse_claim(capture, school)?;
    verify_administration(text, capture, claim.year)?;
    Ok(Some(claim))
}

fn parse_claim(
    capture: &regex::Captures<'_>,
    school: &SchoolName,
) -> CrawlResult<PublishedStaffYear> {
    let start = group(capture, 2)
        .parse::<i16>()
        .map_err(|error| parse::artifact(error.to_string()))?;
    let end = group(capture, 3)
        .parse::<i16>()
        .map_err(|error| parse::artifact(error.to_string()))?;
    if start.checked_add(1) != Some(end) {
        return Err(parse::artifact(
            "published staff-year range is not consecutive",
        ));
    }
    let owner = SchoolName::parse(&fields::unescape_js(group(capture, 4)))
        .map_err(|error| parse::artifact(error.to_string()))?;
    if &owner != school {
        return Err(parse::artifact(
            "published staff-year owner differs from school",
        ));
    }
    Ok(PublishedStaffYear {
        year: SchoolYear::new(start)
            .ok_or_else(|| parse::artifact("published staff-year start is out of range"))?,
        statement: fields::unescape_js(group(capture, 1)),
    })
}

fn verify_administration(
    text: &str,
    statement: &regex::Captures<'_>,
    year: SchoolYear,
) -> CrawlResult<()> {
    let headers = compile_pattern(fields::CARD_HEADER, "tssaa staff-year header")?;
    let tags = compile_pattern(fields::TAG, "tssaa staff-year header tags")?;
    let expected = format!(
        "{}-{} School Administration",
        year.get(),
        year.get()
            .checked_add(1)
            .ok_or_else(|| parse::artifact("staff-year end overflow"))?
    );
    let administration: Vec<_> = headers
        .captures_iter(text)
        .filter(|capture| group(capture, 1).contains("School Administration"))
        .take(2)
        .collect();
    let Some(header) = administration.first() else {
        return Err(parse::artifact(
            "published staff year has no administration header",
        ));
    };
    let label = fields::strip_tags(&tags, group(header, 1));
    let precedes = statement
        .get(0)
        .zip(header.get(0))
        .is_some_and(|(claim, card)| claim.end() <= card.start());
    if administration.len() != 1 || label.trim() != expected || !precedes {
        return Err(parse::artifact(
            "published staff year contradicts administration scope",
        ));
    }
    Ok(())
}
