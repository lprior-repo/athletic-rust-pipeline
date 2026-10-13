use super::{Family, QueueRow};
use crate::report::{EventAdmission, ReportError, ReportResult};
use census_domain::model::EventDisposition;

pub(super) fn unresolved_events(admission: &EventAdmission<'_>) -> ReportResult<Family> {
    let mut family = Family::new("Recruiting event unresolved");
    for row in admission.rows() {
        let EventDisposition::Unresolved(reason) = &row.disposition else {
            continue;
        };
        let event = admission
            .event(row.event)
            .ok_or_else(|| ReportError::Invariant {
                detail: format!("admitted event {} has no raw observation", row.event),
            })?;
        let detail = serde_json::to_string(&(reason, &event.source_labels)).map_err(|source| {
            ReportError::Invariant {
                detail: format!("encoding retained catalog review reason: {source}"),
            }
        })?;
        family.push(QueueRow {
            subject_id: row.event.to_string(),
            subject: event.kind.stable_key().into_owned(),
            detail,
        });
    }
    Ok(family)
}
