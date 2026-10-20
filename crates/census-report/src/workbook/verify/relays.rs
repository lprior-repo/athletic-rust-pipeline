use super::canonical::Value;
use super::expectations::Expectations;
use super::read::{compare, verify_header, verify_width, Book, Budget};
use super::report::Findings;
use crate::report::{RelayProjection, RelaySelection, ReportError, ReportResult};

const TITLE: &str = "Relays";
const HEADERS: [&str; 12] = [
    "Relay ID",
    "Team ID",
    "Event ID",
    "Event",
    "Meet ID",
    "Published Mark",
    "Place",
    "Round",
    "Timing",
    "Published Members",
    "Source Key",
    "Source Team",
];

pub(super) fn verify(
    book: &mut Book,
    expectations: &Expectations<'_>,
    findings: &mut Findings,
) -> ReportResult<()> {
    let projection = RelayProjection::of(&expectations.derivation, &expectations.admission)?;
    let mut encoding_error = None;
    let mut seen = 0usize;
    let mut budget = Budget::family(projection.selections.len())?;
    budget.columns = HEADERS.len();
    let shape = book.read_family(TITLE, budget, |row| {
        verify_width(row, TITLE, HEADERS.len(), findings);
        if row.index() == 0 {
            verify_header(row, TITLE, &HEADERS, findings);
            return;
        }
        let position = row.index().saturating_sub(1);
        let Some(expected) = projection.selections.get(position) else {
            findings.note(format!(
                "Relays has an unexpected accepted row at {}",
                row.index()
            ));
            return;
        };
        match values(expected) {
            Ok(values) => {
                for (column, value) in values.into_iter().enumerate() {
                    compare(row, TITLE, column, &value, findings);
                }
                seen = seen.saturating_add(1);
            }
            Err(error) => {
                if encoding_error.is_none() {
                    encoding_error = Some(error);
                }
            }
        }
    })?;
    if let Some(error) = encoding_error {
        return Err(error);
    }
    if shape.is_none_or(|shape| shape.rows != projection.selections.len().saturating_add(1))
        || seen != projection.selections.len()
    {
        findings.note("Relays has missing or extra accepted rows".into());
    }
    findings.rows(seen);
    Ok(())
}

fn values(row: &RelaySelection<'_>) -> ReportResult<[Value; 12]> {
    let result = row.result;
    Ok([
        Value::text(result.id.as_str()),
        Value::text(result.team.as_str()),
        Value::text(result.event.as_str()),
        Value::text(row.event_kind.stable_key()),
        Value::text(result.meet.as_str()),
        Value::text(crate::bests::mark_text(&result.mark)),
        result
            .place
            .map_or(Value::Empty, |place| Value::text(place.to_string())),
        Value::optional(result.round.as_deref()),
        result
            .timing
            .map_or(Value::Empty, |timing| Value::text(timing.stable_key())),
        Value::text(encode(&result.members)?),
        Value::text(&result.source_key),
        Value::text(encode(&result.source_team)?),
    ])
}

fn encode(value: &impl serde::Serialize) -> ReportResult<String> {
    serde_json::to_string(value).map_err(|source| ReportError::Invariant {
        detail: format!("encoding independent relay expectation: {source}"),
    })
}
