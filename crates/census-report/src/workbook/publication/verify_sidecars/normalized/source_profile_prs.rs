use super::super::input::Inputs;
use super::rows;
use crate::bests::SelectionOutcome;
use crate::export::ExportDataset;
use crate::report::ReportResult;
use crate::workbook::reported::Projection;
use census_crawl::participation_evidence::provider_target::profile_pr::SourceProfilePrV1;
use census_domain::model::CanonicalAthlete;
use std::collections::BTreeMap;
use std::path::Path;

pub(super) fn verify(
    directory: &Path,
    inputs: &Inputs<'_>,
    outcome: &SelectionOutcome,
) -> ReportResult<u64> {
    let reported = Projection::of(
        inputs.dataset,
        &inputs.derivation,
        &inputs.admission,
        &outcome.selections,
    )?;
    let population = inputs
        .derivation
        .athletes()
        .iter()
        .chain(reported.iter().flat_map(|p| p.athletes.iter().copied()))
        .map(|athlete| (athlete.id.as_str(), athlete))
        .collect::<BTreeMap<_, _>>();
    let count = count_claims(inputs.dataset, population.values().copied())?;
    let mut claims = population.values().flat_map(|athlete| {
        inputs
            .dataset
            .reported_profile_prs_for(athlete)
            .flat_map(move |table| table.rows.iter().map(move |claim| (*athlete, table, claim)))
    });
    rows::read_table(
        &directory.join("source_profile_prs.csv"),
        crate::workbook::publication::normalized::source_profile_prs::HEADER,
        count,
        |row| {
            let (athlete, table, claim) = claims
                .next()
                .ok_or_else(|| super::super::defect("source PR exceeds frozen claims".into()))?;
            verify_claim(row, athlete, table, claim)
        },
    )
}

fn verify_claim(
    row: &rows::Row<'_>,
    athlete: &CanonicalAthlete,
    table: &crate::export::ReportedProfilePrTable,
    claim: &SourceProfilePrV1,
) -> ReportResult<()> {
    row.literals(&[
        ("canonical_athlete_id", athlete.id.as_str()),
        ("source_athlete_id", &table.source.id),
        ("event", claim.event()),
        ("source_declared_pr", claim.published_mark()),
        ("source_mode", claim.source_mode()),
        ("source_level", claim.source_level()),
        ("published_date", claim.published_date()),
        ("source_profile_url", &table.profile_url),
        ("capture_body_sha256", &table.body_sha256),
        (
            "retained_payload_sha256",
            table.root.payload.digest.as_str(),
        ),
        ("interpretation", &table.interpretation),
        ("record_basis", "source_reported"),
        ("verification_status", "source_declared_pr_unverified"),
    ])?;
    row.json("source_namespace", &table.source.namespace)?;
    row.json("retained_root", &table.root)?;
    row.text("capture_acquired_at", table.acquired_at.to_rfc3339())?;
    row.number("retained_payload_bytes", table.root.payload.bytes)?;
    row.number("row_ordinal", claim.locator().ordinal())?;
    row.number("byte_offset", claim.locator().byte_offset())?;
    row.number("byte_length", claim.locator().byte_length())
}

fn count_claims<'a>(
    dataset: &'a ExportDataset,
    athletes: impl Iterator<Item = &'a CanonicalAthlete>,
) -> ReportResult<usize> {
    let mut count = 0_usize;
    for athlete in athletes {
        for table in dataset.reported_profile_prs_for(athlete) {
            count = count
                .checked_add(table.rows.len())
                .ok_or_else(|| super::super::defect("source PR row count overflow".into()))?;
        }
    }
    Ok(count)
}
