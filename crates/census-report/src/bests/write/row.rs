use super::SharedSelection;
use crate::csv_safety::Protected;
use crate::export::current_school::CurrentSchoolFields;
use crate::export::public_identity::output::AcceptedIdentityFields;
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

pub(super) struct Row<'a> {
    selection: &'a SharedSelection,
    identity: Option<AcceptedIdentityFields<'a>>,
    current: Option<CurrentSchoolFields<'a>>,
}

impl<'a> From<&'a SharedSelection> for Row<'a> {
    fn from(row: &'a SharedSelection) -> Self {
        Self {
            selection: row,
            identity: None,
            current: None,
        }
    }
}

impl<'a> Row<'a> {
    pub(super) fn selected(
        selection: &'a SharedSelection,
        identity: AcceptedIdentityFields<'a>,
        current: CurrentSchoolFields<'a>,
    ) -> Self {
        Self {
            selection,
            identity: Some(identity),
            current: Some(current),
        }
    }
}

impl Serialize for Row<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut wire = serializer.serialize_struct(
            "best-result",
            33_usize
                .checked_add(self.selection.inventory.serialized_field_count())
                .and_then(|count| count.checked_add(usize::from(self.identity.is_some())))
                .and_then(|count| count.checked_add(if self.current.is_some() { 4 } else { 0 }))
                .ok_or_else(|| {
                    <S::Error as serde::ser::Error>::custom("row field count overflow")
                })?,
        )?;
        let athlete_id = self
            .identity
            .as_ref()
            .map_or(self.selection.athlete_id().as_str(), |identity| {
                identity.athlete_id().as_str()
            });
        identity(&mut wire, self.selection, athlete_id)?;
        result(&mut wire, self.selection)?;
        specifications(&mut wire, self.selection)?;
        inventory(&mut wire, self.selection)?;
        if let Some(identity) = &self.identity {
            text(
                &mut wire,
                "canonical_candidate_id",
                identity.canonical_candidate_id().as_str(),
            )?;
        }
        if let Some(current) = self.current {
            text(&mut wire, "current_school_id", current.current_school_id)?;
            text(&mut wire, "current_school", current.current_school)?;
            text(&mut wire, "current_state", current.current_state)?;
            text(
                &mut wire,
                "current_school_as_of",
                current.current_school_as_of,
            )?;
        }
        wire.end()
    }
}

fn identity<S: SerializeStruct>(
    wire: &mut S,
    row: &SharedSelection,
    athlete_id: &str,
) -> Result<(), S::Error> {
    text(wire, "athlete_id", athlete_id)?;
    text(wire, "name", &row.athlete.name)?;
    wire.serialize_field("school", &row.athlete.school.as_deref().map(Protected))?;
    text(wire, "state", row.athlete.athlete_state.code())?;
    wire.serialize_field("grad_year", &row.athlete.grad_year)?;
    text(wire, "gender", row.athlete.gender.stable_key())?;
    text(wire, "sport", row.sport())?;
    text(wire, "event", row.key.event_kind.stable_key().as_ref())?;
    text(wire, "surface", row.key.surface.label())?;
    text(wire, "wind_class", row.key.wind_class.label())?;
    text(wire, "timing_class", row.key.timing.label())?;
    Ok(())
}

fn result<S: SerializeStruct>(wire: &mut S, row: &SharedSelection) -> Result<(), S::Error> {
    text(wire, "best_mark", &row.mark_text())?;
    wire.serialize_field(
        "original_mark",
        &row.result
            .original_mark
            .as_ref()
            .map(crate::bests::mark_text),
    )?;
    wire.serialize_field("best_value", &row.result.value)?;
    text(wire, "best_value_unit", row.measure().value_unit())?;
    text(wire, "measure", row.measure().as_str())?;
    text(wire, "date", &row.meet.date)?;
    text(wire, "meet", &row.meet.name)?;
    wire.serialize_field("place", &row.result.place)?;
    wire.serialize_field("wind_mps", &row.result.wind_mps)?;
    wire.serialize_field(
        "timing",
        &row.result
            .timing
            .map(|timing| Protected(timing.stable_key())),
    )?;
    wire.serialize_field("marks_in_event", &row.population.marks)?;
    wire.serialize_field(
        "profile_url",
        &row.athlete.profile_url.as_deref().map(Protected),
    )?;
    text(wire, "performance_id", row.source.performance_id.as_str())?;
    text(wire, "source_athlete", &row.source.source_athlete)?;
    text(wire, "source_key", &row.source.source_key)?;
    Ok(())
}

fn specifications<S: SerializeStruct>(wire: &mut S, row: &SharedSelection) -> Result<(), S::Error> {
    wire.serialize_field("comparison_policy", &row.key.comparison)?;
    let comparison =
        serde_json::to_string(&row.key.specification).map_err(serde::ser::Error::custom)?;
    let source =
        serde_json::to_string(&row.source.specification).map_err(serde::ser::Error::custom)?;
    text(wire, "comparison_specification", &comparison)?;
    text(wire, "source_specification", &source)?;
    wire.serialize_field(
        "adjustment_kind",
        &row.result
            .adjustment_kind
            .map(|kind| Protected(kind.label())),
    )?;
    wire.serialize_field(
        "adjustment_basis",
        &row.result
            .adjustment_basis
            .map(|basis| Protected(basis.label())),
    )?;
    wire.serialize_field(
        "adjustment_note",
        &row.result.adjustment_note.as_deref().map(Protected),
    )?;
    let adjustment_source = row
        .result
        .adjustment_source
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(serde::ser::Error::custom)?;
    match adjustment_source {
        Some(source) => text(wire, "adjustment_source", &source),
        None => wire.serialize_field("adjustment_source", &None::<String>),
    }?;
    Ok(())
}

fn inventory<S: SerializeStruct>(wire: &mut S, row: &SharedSelection) -> Result<(), S::Error> {
    let current = matches!(
        row.inventory,
        crate::bests::SelectionInventory::ResultGroups(_)
    );
    let (eligible, tied, ids) = if current {
        (
            "eligible_result_group_count",
            "tied_result_group_count",
            "tied_result_group_representative_ids",
        )
    } else {
        (
            "eligible_candidate_count",
            "tied_performance_count",
            "tied_performance_ids",
        )
    };
    wire.serialize_field(eligible, &row.inventory.eligible_group_count())?;
    wire.serialize_field(tied, &row.inventory.tied_group_count())?;
    let tied_ids =
        row.inventory
            .tied_representative_ids()
            .iter()
            .fold(String::new(), |mut encoded, id| {
                if !encoded.is_empty() {
                    encoded.push(';');
                }
                encoded.push_str(id.as_str());
                encoded
            });
    text(wire, ids, &tied_ids)?;
    if current {
        text(wire, "result_group_basis", "canonical_performance_id")?;
        text(wire, "physical_performance_count_status", "unknown")?;
        wire.serialize_field(
            "selection_policy_version",
            &row.inventory.policy().revision(),
        )?;
    }
    Ok(())
}

fn text<S: SerializeStruct>(
    wire: &mut S,
    field: &'static str,
    value: &str,
) -> Result<(), S::Error> {
    wire.serialize_field(field, &Protected(value))
}
