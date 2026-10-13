use super::policy::{
    AMBIGUOUS, EXCLUSIONS, INCLUDED, PAIRS, QUALIFIERS, SEPARATORS, TERRAIN, UNITS,
};
use super::{CatalogError, SourceEventLabel};
use super::{
    EventDisposition, EventExclusionReason as Ex, EventKind, EventResolutionReason as Why,
};
use crate::model::{EventSpecification, SpecificationError};

pub(super) fn classify(source: &SourceEventLabel) -> Result<EventDisposition, CatalogError> {
    let label = &source.label;
    if label.chars().any(|ch| {
        !ch.is_ascii()
            || !(ch.is_ascii_alphanumeric() || ch.is_whitespace() || SEPARATORS.contains(ch))
    }) {
        return Ok(EventDisposition::Unresolved(
            Why::UnknownOrMixedSourceLabels,
        ));
    }
    let kind = EventKind::from_source_label(label);
    if let Some(reason) = category_fragments(label, &kind) {
        return Ok(EventDisposition::Unresolved(reason));
    }
    if let Some(reason) = excluded(label) {
        return Ok(EventDisposition::Excluded(reason));
    }
    if tokens(label).any(|word| member(word, AMBIGUOUS)) {
        return Ok(EventDisposition::Unresolved(
            Why::AmbiguousAdaptiveOrSpecialLabel,
        ));
    }
    if missing(label, &kind) {
        return Ok(EventDisposition::Unresolved(
            Why::UnsupportedScholasticVariant,
        ));
    }
    if !INCLUDED.contains(&kind) || malformed_number(label) || !permitted(label, &kind) {
        return Ok(EventDisposition::Unresolved(
            Why::UnknownOrMixedSourceLabels,
        ));
    }
    if !super::coherent_components(label, &kind)? || !super::coherent_pairs(label, &kind)? {
        return Ok(EventDisposition::Unresolved(Why::ContradictoryDeclarations));
    }
    Ok(EventDisposition::Included(kind))
}

fn category_fragments(label: &str, kind: &EventKind) -> Option<Why> {
    let mut fragmented = None;
    for raw in label.split_whitespace() {
        let original = match EventSpecification::from_published_label(raw, kind) {
            Ok(specification) => specification.category,
            Err(SpecificationError::ConflictingSpecification) => {
                return Some(Why::ContradictoryDeclarations)
            }
            Err(_) => None,
        };
        for word in tokens(raw) {
            if raw == word {
                continue;
            }
            let fragment = match EventSpecification::from_published_label(word, kind) {
                Ok(specification) => specification.category,
                Err(SpecificationError::ConflictingSpecification) => {
                    return Some(Why::ContradictoryDeclarations)
                }
                Err(_) => None,
            };
            if fragment.is_some() && original != fragment {
                fragmented = Some(Why::UnresolvedCategoryOrContext);
            }
        }
    }
    fragmented
}

pub(super) fn tokens(label: &str) -> impl Iterator<Item = &str> + Clone {
    label
        .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '.')
        .filter(|word| !word.is_empty())
}

pub(super) fn member(word: &str, choices: &[&str]) -> bool {
    choices
        .iter()
        .any(|choice| word.eq_ignore_ascii_case(choice))
}

fn pair(label: &str, left: &str, right: &str) -> bool {
    tokens(label)
        .zip(tokens(label).skip(1))
        .any(|(one, two)| one.eq_ignore_ascii_case(left) && two.eq_ignore_ascii_case(right))
}

fn cross_country(label: &str) -> bool {
    tokens(label).any(|word| member(word, &["xc", "crosscountry", "crosscountryrace"]))
        || pair(label, "cross", "country")
}

fn excluded(label: &str) -> Option<Ex> {
    let direct = EXCLUSIONS.iter().filter_map(|(alias, reason)| {
        tokens(label)
            .zip(std::iter::once(None).chain(tokens(label).map(Some)))
            .any(|(word, previous)| {
                discipline(word, alias)
                    && !(word.eq_ignore_ascii_case("cycling")
                        && previous.is_some_and(|previous| previous.eq_ignore_ascii_case("hand")))
            })
            .then_some(*reason)
    });
    let paired = PAIRS
        .iter()
        .filter_map(|(left, right, reason)| pair(label, left, right).then_some(*reason));
    let terrain = (!cross_country(label) && tokens(label).any(|word| member(word, TERRAIN)))
        .then_some(Ex::NonCrossCountryTrailOrMountain);
    direct.chain(paired).chain(terrain).reduce(|left, right| {
        if left == right {
            left
        } else {
            Ex::MultipleExcludedDisciplines
        }
    })
}

fn discipline(word: &str, alias: &str) -> bool {
    word.eq_ignore_ascii_case(alias)
        || word
            .get(..alias.len())
            .is_some_and(|head| head.eq_ignore_ascii_case(alias))
            && word.get(alias.len()..).is_some_and(quantity)
        || word.len().checked_sub(alias.len()).is_some_and(|start| {
            word.get(start..)
                .is_some_and(|tail| tail.eq_ignore_ascii_case(alias))
                && word.get(..start).is_some_and(quantity)
        })
}

pub(super) fn quantity(word: &str) -> bool {
    let number_end = word.find(|ch: char| !ch.is_ascii_digit() && ch != '.');
    let (number, unit) = match number_end {
        Some(end) => match (word.get(..end), word.get(end..)) {
            (Some(number), Some(unit)) => (number, unit),
            _ => return false,
        },
        None => (word, ""),
    };
    !number.is_empty()
        && number.bytes().any(|byte| byte.is_ascii_digit())
        && number.bytes().filter(|byte| *byte == b'.').count() <= 1
        && (unit.is_empty() || member(unit, UNITS))
}

fn missing(label: &str, kind: &EventKind) -> bool {
    if matches!(
        kind,
        EventKind::CrossCountry | EventKind::SprintMedley | EventKind::DistanceMedley
    ) {
        return false;
    }
    [
        ("300", "300m"),
        ("500", "500m"),
        ("600", "600m"),
        ("2000", "2000m"),
        ("10000", "10000m"),
    ]
    .iter()
    .any(|(number, compact)| {
        let present = tokens(label).any(|word| word.eq_ignore_ascii_case(compact))
            || ["m", "meter", "meters", "metre", "metres"]
                .iter()
                .any(|unit| pair(label, number, unit));
        present
            && !matches!(
                (*number, kind),
                ("300", EventKind::Track300mHurdles) | ("2000", EventKind::Track2000mSteeplechase)
            )
    }) || tokens(label).any(|word| member(word, &["10k", "2mile", "2miles", "twomile", "twomiles"]))
        || ["2", "two"]
            .iter()
            .any(|number| pair(label, number, "mile") || pair(label, number, "miles"))
        || pair(label, "10", "000m")
}

fn malformed_number(label: &str) -> bool {
    label.split('-').skip(1).any(|tail| {
        tail.trim_start_matches(|ch: char| !ch.is_ascii_alphanumeric())
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_digit())
    })
}

fn permitted(label: &str, kind: &EventKind) -> bool {
    tokens(label)
        .zip(tokens(label).skip(1).map(Some).chain(std::iter::once(None)))
        .all(|(word, next)| {
            if word
                .bytes()
                .all(|byte| byte.is_ascii_digit() || byte == b'.')
            {
                return next.is_some_and(|next| permitted_unit(next, kind))
                    || kind.is_relay() && next.is_some_and(|next| next.eq_ignore_ascii_case("x"));
            }
            member(word, QUALIFIERS)
                || permitted_unit(word, kind)
                || permitted_quantity(word, kind)
                || !matches!(
                    EventKind::from_source_label(word),
                    EventKind::Unmapped { .. }
                )
                || context_word(word, kind)
                || word.eq_ignore_ascii_case("high") && pair(label, "high", "school")
        })
}

fn permitted_quantity(word: &str, kind: &EventKind) -> bool {
    quantity(word)
        && word
            .find(|ch: char| !ch.is_ascii_digit() && ch != '.')
            .and_then(|end| word.get(end..))
            .is_some_and(|unit| permitted_unit(unit, kind))
}

fn permitted_unit(word: &str, kind: &EventKind) -> bool {
    member(
        word,
        &[
            "m", "meter", "meters", "metre", "metres", "k", "km", "mile", "miles",
        ],
    ) || kind.is_hurdles()
        && member(
            word,
            &["mm", "cm", "in", "inch", "inches", "ft", "feet", "foot"],
        )
        || matches!(
            kind,
            EventKind::ShotPut
                | EventKind::Discus
                | EventKind::Javelin
                | EventKind::Hammer
                | EventKind::WeightThrow
        ) && member(word, &["kg", "g", "lb", "lbs"])
}

pub(super) fn excluded_component(word: &str) -> Option<EventKind> {
    let direct = EventKind::from_source_label(word);
    if !matches!(direct, EventKind::Unmapped { .. }) || super::distance_quantity(word) {
        return Some(direct);
    }
    EXCLUSIONS.iter().find_map(|(alias, _)| {
        let start = word.len().checked_sub(alias.len())?;
        let head = word
            .get(..alias.len())
            .filter(|head| head.eq_ignore_ascii_case(alias))
            .and_then(|_| word.get(alias.len()..));
        let tail = word
            .get(start..)
            .filter(|tail| tail.eq_ignore_ascii_case(alias))
            .and_then(|_| word.get(..start));
        head.or(tail)
            .filter(|part| quantity(part))
            .map(EventKind::from_source_label)
    })
}

fn context_word(word: &str, kind: &EventKind) -> bool {
    match kind {
        EventKind::CrossCountry => member(word, &["cross", "country", "race", "trail", "mountain"]),
        EventKind::HighJump => member(word, &["high", "jump"]),
        EventKind::LongJump => member(word, &["long", "jump"]),
        EventKind::TripleJump => member(word, &["triple", "jump"]),
        EventKind::PoleVault => member(word, &["pole", "vault"]),
        EventKind::ShotPut => word.eq_ignore_ascii_case("put"),
        EventKind::WeightThrow | EventKind::Discus | EventKind::Javelin | EventKind::Hammer => {
            member(word, &["weight", "throw"])
        }
        EventKind::SprintMedley | EventKind::DistanceMedley => {
            member(word, &["sprint", "distance", "medley", "relay", "x"])
        }
        EventKind::Relay4x100
        | EventKind::Relay4x200
        | EventKind::Relay4x400
        | EventKind::Relay4x800 => member(word, &["relay", "x"]),
        _ if kind.is_hurdles() => member(word, &["hurdles", "hurdle", "high", "low"]),
        EventKind::Track2000mSteeplechase | EventKind::Track3000mSteeplechase => {
            member(word, &["steeple", "chase", "steeplechase"])
        }
        _ => member(word, &["banked", "flat", "track"]),
    }
}
