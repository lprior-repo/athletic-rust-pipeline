use super::context::ContextualLabel;
use super::{
    aliases, policy, CanonicalEvent, CatalogError, EventDisposition, EventExclusionReason,
    EventKind, EventResolutionReason,
};

pub(super) fn coherent_components(
    label: &ContextualLabel<'_>,
    kind: &EventKind,
) -> Result<bool, CatalogError> {
    label.tokens().try_fold(true, |coherent, token| {
        let Some(word) = token.declaration() else {
            return Ok(coherent);
        };
        let component = EventKind::from_source_label(word);
        let matches = match component {
            EventKind::Unmapped { .. } if distance_quantity(word) => coherent_distance(word, kind)?,
            EventKind::Unmapped { .. } => true,
            _ => component == *kind || distance_component(kind, &component),
        };
        Ok(coherent && matches)
    })
}

pub(super) fn excluded_coherence(
    event: &CanonicalEvent,
    reason: EventExclusionReason,
) -> Result<EventDisposition, CatalogError> {
    if matches!(
        event.resolved_specification(),
        Err(crate::model::SpecificationError::ConflictingSpecification)
    ) {
        return Ok(EventDisposition::Unresolved(
            EventResolutionReason::ContradictoryDeclarations,
        ));
    }
    let mut kind = None;
    for label in &event.source_labels {
        let mut preceding: Option<&str> = None;
        for word in aliases::tokens(&label.label) {
            let unit = is_distance_unit_word(word, preceding);
            preceding = Some(word);
            if unit {
                continue;
            }
            if let Some(component) = aliases::excluded_component(word) {
                if !update_kind(&mut kind, component) {
                    return Ok(EventDisposition::Unresolved(
                        EventResolutionReason::ContradictoryDeclarations,
                    ));
                }
            }
        }
        let mut buffer = String::new();
        for (left, right) in
            aliases::tokens(&label.label).zip(aliases::tokens(&label.label).skip(1))
        {
            if !is_distance_pair(left, right) {
                continue;
            }
            let component = build_distance_component(&mut buffer, left, right)?;
            if !update_kind(&mut kind, component) {
                return Ok(EventDisposition::Unresolved(
                    EventResolutionReason::ContradictoryDeclarations,
                ));
            }
        }
    }
    Ok(EventDisposition::Excluded(reason))
}

fn update_kind(kind: &mut Option<EventKind>, new_kind: EventKind) -> bool {
    if kind.as_ref().is_some_and(|old| old != &new_kind) {
        return false;
    }
    *kind = Some(new_kind);
    true
}

fn is_distance_unit_word(word: &str, preceding: Option<&str>) -> bool {
    aliases::member(word, policy::UNITS)
        && preceding.is_some_and(|previous| {
            previous
                .bytes()
                .all(|byte| byte.is_ascii_digit() || byte == b'.')
                || previous.eq_ignore_ascii_case("two")
        })
}

fn is_distance_pair(left: &str, right: &str) -> bool {
    (left
        .bytes()
        .all(|byte| byte.is_ascii_digit() || byte == b'.')
        || left.eq_ignore_ascii_case("two"))
        && aliases::member(right, policy::UNITS)
}

fn build_distance_component(
    buffer: &mut String,
    left: &str,
    right: &str,
) -> Result<EventKind, CatalogError> {
    let bytes = left
        .len()
        .checked_add(right.len())
        .ok_or(CatalogError::AggregateLabelBytesOverflow)?;
    buffer.clear();
    buffer
        .try_reserve(bytes)
        .map_err(|_| CatalogError::ResourceExhausted)?;
    buffer.push_str(left);
    buffer.push_str(right);
    Ok(EventKind::from_source_label(buffer))
}

pub(super) fn coherent_pairs(
    label: &ContextualLabel<'_>,
    kind: &EventKind,
) -> Result<bool, CatalogError> {
    let mut buffer = String::new();
    for (left, right) in label.tokens().zip(label.tokens().skip(1)) {
        let (Some(left), Some(right)) = (left.declaration(), right.declaration()) else {
            continue;
        };
        let bytes = left
            .len()
            .checked_add(right.len())
            .ok_or(CatalogError::AggregateLabelBytesOverflow)?;
        buffer.clear();
        buffer
            .try_reserve(bytes)
            .map_err(|_| CatalogError::ResourceExhausted)?;
        buffer.push_str(left);
        buffer.push_str(right);
        let component = EventKind::from_source_label(&buffer);
        match component {
            EventKind::Unmapped { .. } if distance_quantity(&buffer) => {
                if !coherent_distance(&buffer, kind)? {
                    return Ok(false);
                }
            }
            EventKind::Unmapped { .. } => {}
            _ if component == *kind || distance_component(kind, &component) => {}
            _ => return Ok(false),
        }
    }
    Ok(true)
}

pub(super) fn distance_quantity(word: &str) -> bool {
    aliases::quantity(word)
        && word
            .find(|ch: char| !ch.is_ascii_digit() && ch != '.')
            .and_then(|end| word.get(end..))
            .is_some_and(|unit| {
                aliases::member(
                    unit,
                    &[
                        "m", "meter", "meters", "metre", "metres", "k", "km", "mile", "miles",
                    ],
                )
            })
}

fn coherent_distance(word: &str, kind: &EventKind) -> Result<bool, CatalogError> {
    let suffix = match kind {
        EventKind::CrossCountry | EventKind::SprintMedley | EventKind::DistanceMedley => {
            return Ok(true)
        }
        EventKind::Track2000mSteeplechase | EventKind::Track3000mSteeplechase => "steeplechase",
        _ if kind.is_hurdles() => "hurdles",
        _ => return Ok(false),
    };
    let bytes = word
        .len()
        .checked_add(suffix.len())
        .ok_or(CatalogError::AggregateLabelBytesOverflow)?;
    let mut qualified = String::new();
    qualified
        .try_reserve(bytes)
        .map_err(|_| CatalogError::ResourceExhausted)?;
    qualified.push_str(word);
    qualified.push_str(suffix);
    Ok(EventKind::from_source_label(&qualified) == *kind)
}

fn distance_component(kind: &EventKind, component: &EventKind) -> bool {
    match kind {
        EventKind::Track55mHurdles => component == &EventKind::Track55m,
        EventKind::Track60mHurdles => component == &EventKind::Track60m,
        EventKind::Track100mHurdles => component == &EventKind::Track100m,
        EventKind::Track400mHurdles => component == &EventKind::Track400m,
        EventKind::Track3000mSteeplechase => component == &EventKind::Track3000m,
        EventKind::CrossCountry | EventKind::SprintMedley | EventKind::DistanceMedley => {
            matches!(
                component,
                EventKind::Track55m
                    | EventKind::Track60m
                    | EventKind::Track100m
                    | EventKind::Track200m
                    | EventKind::Track400m
                    | EventKind::Track800m
                    | EventKind::Track1000m
                    | EventKind::Track1500m
                    | EventKind::Track1600m
                    | EventKind::Track1Mile
                    | EventKind::Track3000m
                    | EventKind::Track3200m
                    | EventKind::Track5000m
            )
        }
        _ => false,
    }
}
