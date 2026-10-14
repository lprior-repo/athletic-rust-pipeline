use super::{encode, provenance, write_csv};
use crate::report::{Derivation, ReportResult};
use census_domain::model::{CanonicalSchool, SchoolId};
use std::collections::BTreeMap;
use std::path::Path;

pub(super) enum Reference<'a> {
    Resolved(&'a CanonicalSchool),
    UnresolvedHistorical,
}

pub(super) type Schools<'a> = BTreeMap<&'a SchoolId, Reference<'a>>;

const HEADER: [&str; 26] = [
    "school_id",
    "canonical_name",
    "aliases",
    "school_type",
    "school_status",
    "address_status",
    "address_verification_status",
    "evidence_refs",
    "input_generation",
    "record_revision_status",
    "postal_school_id",
    "postal_street",
    "postal_second_line",
    "postal_city",
    "postal_state",
    "postal_zip",
    "postal_owner_namespace",
    "postal_owner_id",
    "postal_source",
    "postal_source_url",
    "postal_observed_date",
    "postal_capture_sha256",
    "postal_address_kind",
    "address_verification_date",
    "address_claim_ids",
    "address_evidence_refs",
];

pub(super) fn write(directory: &Path, schools: &Schools<'_>, generation: &str) -> ReportResult<()> {
    write_csv(&directory.join("schools.csv"), &HEADER, |writer| {
        schools.iter().try_for_each(|(id, reference)| {
            let fields = match reference {
                Reference::Resolved(school) => row(school, generation)?,
                Reference::UnresolvedHistorical => unresolved_row(id, generation),
            };
            writer.push(fields)
        })?;
        Ok(())
    })
}

pub(super) fn included<'a>(
    derivation: &'a Derivation<'_>,
    bests: &[crate::bests::SharedSelection],
    performances: &super::PerformanceIndex<'_>,
) -> ReportResult<Schools<'a>> {
    let dataset = derivation.dataset();
    let mut schools: Schools<'_> = derivation
        .schools()
        .iter()
        .map(|school| (&school.id, Reference::Resolved(school)))
        .collect();
    for athlete in derivation.athletes() {
        let school = dataset
            .schools
            .get(&athlete.school)
            .ok_or_else(|| super::defect("missing athlete school"))?;
        schools.insert(&school.id, Reference::Resolved(school));
    }
    for best in bests {
        let performance = performances
            .get(&best.source.performance_id)
            .copied()
            .ok_or_else(|| super::defect("missing selected performance"))?;
        if let Some(team) = dataset.teams.get(&performance.team) {
            let reference = match dataset.schools.get(&team.school) {
                Some(school) => Reference::Resolved(school),
                None => Reference::UnresolvedHistorical,
            };
            schools.insert(&team.school, reference);
        }
    }
    Ok(schools)
}

fn unresolved_row(id: &SchoolId, generation: &str) -> [String; 26] {
    HEADER.map(|name| {
        match name {
            "school_id" => id.as_str(),
            "school_status" => "unresolved_historical_reference",
            "input_generation" => generation,
            "aliases" | "evidence_refs" | "address_claim_ids" | "address_evidence_refs" => "[]",
            _ => "unavailable",
        }
        .to_owned()
    })
}

fn row(school: &CanonicalSchool, generation: &str) -> ReportResult<[String; 26]> {
    let [school_id, street, line2, city, state, zip, namespace, owner, source, url, observed, sha, kind] =
        crate::export::postal::postal_fields(std::iter::once(school))?;
    let (address_ids, address_evidence) = provenance::address_references(school)?;
    Ok([
        school.id.to_string(),
        school.name.clone(),
        encode(&school.aliases)?,
        if school.co_op {
            "co_op"
        } else {
            "institution_type_unavailable"
        }
        .into(),
        "currentness_unavailable".into(),
        if school.postal_addresses.is_empty() {
            "not_collected"
        } else {
            "retained_owner_bound_claims"
        }
        .into(),
        "observed_date_is_not_verification_date".into(),
        provenance::references("school", school.id.as_str(), &school.evidence)?,
        generation.into(),
        "record_revision_not_retained".into(),
        school_id,
        street,
        line2,
        city,
        state,
        zip,
        namespace,
        owner,
        source,
        url,
        observed,
        sha,
        kind,
        "unavailable".into(),
        address_ids,
        address_evidence,
    ])
}
