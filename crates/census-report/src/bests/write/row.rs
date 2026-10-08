use super::SharedSelection;
use crate::csv_safety::Protected;
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

pub(super) struct Row<'a>(&'a SharedSelection);

impl<'a> From<&'a SharedSelection> for Row<'a> {
    fn from(row: &'a SharedSelection) -> Self {
        Self(row)
    }
}

impl Serialize for Row<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut wire = serializer.serialize_struct("best-result", super::HEADER.len())?;
        identity(&mut wire, self.0)?;
        result(&mut wire, self.0)?;
        specifications(&mut wire, self.0)?;
        wire.end()
    }
}

fn identity<S: SerializeStruct>(wire: &mut S, row: &SharedSelection) -> Result<(), S::Error> {
    text(wire, "athlete_id", row.key.athlete_id.as_str())?;
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
    text(wire, "source_specification", &source)
}

fn text<S: SerializeStruct>(
    wire: &mut S,
    field: &'static str,
    value: &str,
) -> Result<(), S::Error> {
    wire.serialize_field(field, &Protected(value))
}
