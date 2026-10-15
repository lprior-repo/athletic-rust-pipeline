use super::super::input::Inputs;
use super::{facts, rows, schema};
use crate::bests::SharedSelection;
use crate::report::ReportResult;
use census_domain::model::CanonicalSchool;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub(super) enum Reference<'a> {
    Resolved(&'a CanonicalSchool),
    UnresolvedHistorical,
}

pub(super) type Schools<'a> = BTreeMap<&'a str, Reference<'a>>;

pub(super) fn inventory<'a>(
    inputs: &'a Inputs<'_>,
    bests: &[SharedSelection],
) -> ReportResult<Schools<'a>> {
    let mut schools: Schools<'_> = inputs
        .derivation
        .schools()
        .iter()
        .map(|school| (school.id.as_str(), Reference::Resolved(school)))
        .collect();
    inputs
        .derivation
        .athletes()
        .iter()
        .try_for_each(|athlete| {
            let school = inputs
                .dataset
                .schools
                .get(&athlete.school)
                .ok_or_else(|| rows::missing("school", athlete.school.as_str()))?;
            schools.insert(school.id.as_str(), Reference::Resolved(school));
            Ok::<_, crate::report::ReportError>(())
        })?;
    let performances: BTreeMap<_, _> = inputs
        .dataset
        .performances
        .iter()
        .map(|performance| (&performance.id, performance))
        .collect();
    bests.iter().try_for_each(|best| {
        let performance = performances
            .get(&best.source.performance_id)
            .ok_or_else(|| rows::missing("performance", best.source.performance_id.as_str()))?;
        if let Some(team) = inputs.dataset.teams.get(&performance.team) {
            let reference = match inputs.dataset.schools.get(&team.school) {
                Some(school) => Reference::Resolved(school),
                None => Reference::UnresolvedHistorical,
            };
            schools.insert(team.school.as_str(), reference);
        }
        Ok::<_, crate::report::ReportError>(())
    })?;
    Ok(schools)
}

pub(super) fn verify(
    directory: &Path,
    inputs: &Inputs<'_>,
    schools: &Schools<'_>,
) -> ReportResult<u64> {
    rows::table(
        directory,
        ("schools.csv", schema::SCHOOLS),
        schools.len(),
        &["school_id"],
        |row| {
            let id = row.get("school_id")?;
            let school = schools.get(id).ok_or_else(|| rows::missing("school", id))?;
            row.text("input_generation", &inputs.dataset.lineage.input_generation)?;
            let Reference::Resolved(school) = school else {
                return unresolved(row);
            };
            row.text("canonical_name", &school.name)?;
            row.json("aliases", &school.aliases)?;
            row.text(
                "school_type",
                if school.co_op {
                    "co_op"
                } else {
                    "institution_type_unavailable"
                },
            )?;
            row.text(
                "address_status",
                if school.postal_addresses.is_empty() {
                    "not_collected"
                } else {
                    "retained_owner_bound_claims"
                },
            )?;
            row.json(
                "evidence_refs",
                &facts::references("school", id, &school.evidence)?,
            )?;
            address(row, school)?;
            row.literals(&[
                ("school_status", "currentness_unavailable"),
                (
                    "address_verification_status",
                    "observed_date_is_not_verification_date",
                ),
                ("record_revision_status", "record_revision_not_retained"),
                ("address_verification_date", "unavailable"),
            ])
        },
    )
}

fn unresolved(row: &rows::Row<'_>) -> ReportResult<()> {
    schema::SCHOOLS
        .iter()
        .filter(|name| !matches!(**name, "school_id" | "input_generation"))
        .try_for_each(|name| {
            row.text(
                name,
                match *name {
                    "school_status" => "unresolved_historical_reference",
                    "aliases" | "evidence_refs" | "address_claim_ids" | "address_evidence_refs" => {
                        "[]"
                    }
                    _ => "unavailable",
                },
            )
        })
}

fn address(row: &rows::Row<'_>, school: &CanonicalSchool) -> ReportResult<()> {
    let fields = crate::export::postal::postal_fields(std::iter::once(school))?;
    crate::export::postal::POSTAL_CSV_HEADERS
        .iter()
        .zip(fields)
        .try_for_each(|(name, value)| row.text(name, value))?;
    let ids = school
        .postal_addresses
        .iter()
        .map(rows::digest)
        .collect::<ReportResult<BTreeSet<_>>>()?;
    let evidence = school
        .postal_addresses
        .iter()
        .map(|claim| {
            rows::digest(&(
                "school_address",
                rows::digest(claim)?.as_str(),
                claim.evidence(),
            ))
        })
        .collect::<ReportResult<BTreeSet<_>>>()?;
    row.json("address_claim_ids", &ids)?;
    row.json("address_evidence_refs", &evidence)
}
