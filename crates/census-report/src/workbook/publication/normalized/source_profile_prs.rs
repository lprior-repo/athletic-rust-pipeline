use crate::bests::SelectionOutcome;
use crate::report::{Derivation, EventAdmission, ReportResult};
use crate::workbook::reported::Projection;
use std::collections::BTreeMap;
use std::path::Path;

pub(in crate::workbook::publication) const HEADER: &[&str] = &[
    "canonical_athlete_id",
    "source_namespace",
    "source_athlete_id",
    "event",
    "source_declared_pr",
    "source_mode",
    "source_level",
    "published_date",
    "source_profile_url",
    "capture_acquired_at",
    "capture_body_sha256",
    "retained_root",
    "retained_payload_sha256",
    "retained_payload_bytes",
    "interpretation",
    "row_ordinal",
    "byte_offset",
    "byte_length",
    "record_basis",
    "verification_status",
];

pub(super) fn write(
    directory: &Path,
    derivation: &Derivation<'_>,
    outcome: &SelectionOutcome,
) -> ReportResult<u64> {
    let dataset = derivation.dataset();
    let admission = EventAdmission::of(dataset)?;
    let reported = Projection::of(dataset, derivation, &admission, &outcome.selections)?;
    let population = derivation
        .athletes()
        .iter()
        .chain(reported.iter().flat_map(|p| p.athletes.iter().copied()))
        .map(|athlete| (&athlete.id, athlete))
        .collect::<BTreeMap<_, _>>();
    super::write_csv(
        &directory.join("source_profile_prs.csv"),
        HEADER,
        |writer| {
            for athlete in population.values() {
                for table in dataset.reported_profile_prs_for(athlete) {
                    for row in &table.rows {
                        let locator = row.locator();
                        writer.push([
                            athlete.id.as_str().to_owned(),
                            super::encode(&table.source.namespace)?,
                            table.source.id.clone(),
                            row.event().to_owned(),
                            row.published_mark().to_owned(),
                            row.source_mode().to_owned(),
                            row.source_level().to_owned(),
                            row.published_date().to_owned(),
                            table.profile_url.clone(),
                            table.acquired_at.to_rfc3339(),
                            table.body_sha256.clone(),
                            super::encode(&table.root)?,
                            table.root.payload.digest.as_str().to_owned(),
                            table.root.payload.bytes.to_string(),
                            table.interpretation.clone(),
                            locator.ordinal().to_string(),
                            locator.byte_offset().to_string(),
                            locator.byte_length().to_string(),
                            "source_reported".to_owned(),
                            "source_declared_pr_unverified".to_owned(),
                        ])?;
                    }
                }
            }
            Ok(())
        },
    )
}
