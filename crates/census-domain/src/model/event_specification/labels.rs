use super::*;
use crate::model::EventKind;

mod category;
mod course;
mod quantity;
mod tokens;

const HURDLE_ANCHORS: &[&str] = &[
    "hurdles", "hurdle", "55mh", "60mh", "100mh", "110mh", "300mh", "400mh", "55h", "60h", "100h",
    "110h", "300h", "400h",
];
const RESULT_QUALIFIERS: &[&str] = &[
    "final",
    "finals",
    "prelim",
    "prelims",
    "preliminaries",
    "semifinal",
    "semifinals",
    "qualification",
    "qualifications",
    "heat",
    "heats",
    "results",
    "varsity",
    "jv",
    "open",
];

impl EventSpecification {
    pub fn from_published_label(label: &str, kind: &EventKind) -> Result<Self, SpecificationError> {
        if label.len() > 512 || label.chars().any(char::is_control) {
            return Err(SpecificationError::InvalidLabel);
        }
        let storage = tokens::Tokens::parse(label)?;
        let tokens = storage.values()?;
        Ok(Self {
            implement: implement(tokens, kind)?,
            hurdles: hurdles(tokens, kind)?,
            indoor_track: indoor(tokens, label)?,
            category: category::parse(tokens)?,
            cross_country: course::parse(tokens, kind)?,
        })
    }
}

fn tail<'a>(tokens: &'a [&'a str], anchors: &[&str]) -> &'a [&'a str] {
    let start = tokens
        .iter()
        .rposition(|token| anchors.iter().any(|anchor| anchor_matches(token, anchor)))
        .and_then(|index| index.checked_add(1));
    match start.and_then(|index| tokens.get(index..)) {
        Some(tail) => tail,
        None => &[],
    }
}

fn anchor_matches(token: &str, anchor: &str) -> bool {
    compact_matches(token, anchor, false)
        || (anchor == "hurdles"
            && [
                "mhurdles",
                "meterhurdles",
                "metershurdles",
                "metrehurdles",
                "metreshurdles",
            ]
            .iter()
            .any(|suffix| compact_matches(token, suffix, true)))
}

fn compact_matches(token: &str, expected: &str, skip_distance: bool) -> bool {
    token
        .chars()
        .filter(|ch| !matches!(ch, '-' | '_' | '\'' | '’'))
        .map(|ch| ch.to_ascii_lowercase())
        .skip_while(|ch| skip_distance && ch.is_ascii_digit())
        .eq(expected.chars())
}

fn quantity_parts<'a>(
    tokens: &[&'a str],
) -> Result<Option<(&'a str, Option<&'a str>)>, SpecificationError> {
    let mut values = tokens.iter().copied().filter(|token| {
        !category::is_descriptor(token)
            && !RESULT_QUALIFIERS
                .iter()
                .any(|qualifier| token.eq_ignore_ascii_case(qualifier))
    });
    match (values.next(), values.next(), values.next()) {
        (None, None, None) => Ok(None),
        (Some(one), None, None) => Ok(Some((one, None))),
        (Some(number), Some(unit), None) => Ok(Some((number, Some(unit)))),
        _ => Err(SpecificationError::InvalidLabel),
    }
}

fn implement(
    tokens: &[&str],
    kind: &EventKind,
) -> Result<Option<ImplementMass>, SpecificationError> {
    let anchors: &[&str] = match kind {
        EventKind::ShotPut => &["put", "shot", "shotput", "sp"],
        EventKind::Discus => &["discus", "disc", "dt"],
        EventKind::Javelin => &["javelin", "jav", "jt"],
        EventKind::Hammer => &["hammer", "ht"],
        EventKind::WeightThrow => &["throw", "weightthrow", "wt"],
        _ => return Ok(None),
    };
    quantity_parts(tail(tokens, anchors))?
        .map(|(number, unit)| ImplementMass::try_from(quantity::mass(number, unit)?))
        .transpose()
}

fn hurdles(
    tokens: &[&str],
    kind: &EventKind,
) -> Result<Option<HurdleSpecification>, SpecificationError> {
    if !kind.is_hurdles() {
        return Ok(None);
    }
    let Some((text, unit)) = quantity_parts(tail(tokens, HURDLE_ANCHORS))? else {
        return Ok(None);
    };
    let (height, spacing) = match text.split_once('/') {
        Some((height, spacing)) if unit.is_none() => {
            (height, Some(quantity::length(spacing, None)?))
        }
        Some(_) => return Err(SpecificationError::InvalidHurdles),
        None => (text, None),
    };
    let height = u32::try_from(quantity::length(height, unit)?)
        .map_err(|_| SpecificationError::InvalidHurdles)?;
    HurdleSpecification::new(height, spacing).map(Some)
}

fn indoor(
    tokens: &[&str],
    label: &str,
) -> Result<Option<IndoorTrackSpecification>, SpecificationError> {
    let Some((index, banking)) = banking(tokens)? else {
        return Ok(None);
    };
    let length = index
        .checked_sub(1)
        .and_then(|index| tokens.get(index))
        .ok_or(SpecificationError::InvalidTrack)?;
    if !facility_context(label, tokens, length) {
        return Err(SpecificationError::InvalidTrack);
    }
    IndoorTrackSpecification::new(quantity::length(length, None)?, banking).map(Some)
}

fn banking(tokens: &[&str]) -> Result<Option<(usize, TrackBanking)>, SpecificationError> {
    let banking = tokens
        .iter()
        .enumerate()
        .filter_map(|(index, token)| {
            if token.eq_ignore_ascii_case("banked") {
                Some((index, TrackBanking::Banked))
            } else if token.eq_ignore_ascii_case("flat") {
                Some((index, TrackBanking::Flat))
            } else {
                None
            }
        })
        .try_fold(None, |old, value| {
            if old.is_some() {
                return Err(SpecificationError::ConflictingSpecification);
            }
            Ok(Some(value))
        })?;
    Ok(banking.or_else(|| {
        tokens
            .iter()
            .position(|token| token.eq_ignore_ascii_case("track"))
            .map(|index| (index, TrackBanking::Unknown))
    }))
}

fn facility_context(label: &str, tokens: &[&str], length: &str) -> bool {
    tokens
        .iter()
        .any(|token| token.eq_ignore_ascii_case("track"))
        || label
            .split(['(', '['])
            .skip(1)
            .filter_map(|section| section.split_once([')', ']']).map(|(body, _)| body))
            .any(|body| {
                body.split_whitespace().any(|token| token == length)
                    && body.split_whitespace().any(|token| {
                        token.eq_ignore_ascii_case("banked") || token.eq_ignore_ascii_case("flat")
                    })
            })
}
