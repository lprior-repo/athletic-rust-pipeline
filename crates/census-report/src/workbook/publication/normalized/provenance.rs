use super::{digest, encode, text, write_csv, Rows};
use crate::report::{Derivation, ReportResult};
use census_domain::model::{CanonicalAthlete, Evidence, SourceIdentity, SourceRef};
use std::collections::BTreeSet;
use std::path::Path;

const HEADER: [&str; 15] = [
    "evidence_id",
    "entity_type",
    "entity_id",
    "source_namespace",
    "source_url",
    "source_reference",
    "capture_id",
    "capture_time",
    "interpretation_version",
    "evidence_type",
    "observed_on",
    "capture_status",
    "interpretation_status",
    "input_generation",
    "retained_fact",
];

pub(super) fn references(kind: &str, owner: &str, evidence: &[Evidence]) -> ReportResult<String> {
    let mut ids = BTreeSet::new();
    for fact in evidence {
        ids.insert(digest(&(kind, owner, fact))?);
    }
    encode(&ids)
}

pub(super) fn cohort_references(athlete: &CanonicalAthlete) -> ReportResult<String> {
    let mut ids = BTreeSet::new();
    for claim in &athlete.published_graduations {
        ids.insert(digest(&(
            "published_graduation",
            athlete.id.as_str(),
            claim,
        ))?);
    }
    for claim in &athlete.observed_grades {
        ids.insert(digest(&("observed_grade", athlete.id.as_str(), claim))?);
    }
    encode(&ids)
}

pub(super) fn address_references(
    school: &census_domain::model::CanonicalSchool,
) -> ReportResult<(String, String)> {
    let mut owners = BTreeSet::new();
    let mut evidence = BTreeSet::new();
    for claim in &school.postal_addresses {
        let owner = digest(claim)?;
        evidence.insert(digest(&(
            "school_address",
            owner.as_str(),
            claim.evidence(),
        ))?);
        owners.insert(owner);
    }
    Ok((encode(&owners)?, encode(&evidence)?))
}

pub(super) fn write(
    directory: &Path,
    derivation: &Derivation<'_>,
    schools: &super::schools::Schools<'_>,
    bests: &[crate::bests::SharedSelection],
    performances: &super::PerformanceIndex<'_>,
) -> ReportResult<()> {
    write_csv(&directory.join("provenance.csv"), &HEADER, |writer| {
        let mut provenance = Provenance {
            writer,
            seen: BTreeSet::new(),
            budget: crate::grouping::Admission::new("normalized evidence keys"),
            generation: &derivation.dataset().lineage.input_generation,
        };
        for athlete in derivation.athletes() {
            provenance.evidence("athlete", athlete.id.as_str(), &athlete.evidence)?;
            provenance.cohort(athlete)?;
        }
        for reference in schools.values() {
            let super::schools::Reference::Resolved(school) = reference else {
                continue;
            };
            provenance.evidence("school", school.id.as_str(), &school.evidence)?;
            for claim in &school.postal_addresses {
                provenance.evidence(
                    "school_address",
                    &digest(claim)?,
                    std::slice::from_ref(claim.evidence()),
                )?;
            }
        }
        for coach in super::coach_contacts::claims(derivation)? {
            provenance.evidence("coach", coach.id.as_str(), &coach.evidence)?;
        }
        let ids: BTreeSet<_> = bests
            .iter()
            .flat_map(|best| best.tied_performance_ids.iter())
            .collect();
        for id in ids {
            let performance = performances
                .get(id)
                .copied()
                .ok_or_else(|| super::defect(format!("missing provenance performance {id}")))?;
            provenance.evidence(
                "performance",
                performance.id.as_str(),
                &performance.evidence,
            )?;
            if let Some(source) = &performance.source_athlete {
                provenance.identity(performance.id.as_str(), source)?;
            }
        }
        Ok(())
    })
}

struct Provenance<'a, 'w> {
    writer: &'w mut Rows<'a>,
    seen: BTreeSet<String>,
    budget: crate::grouping::Admission,
    generation: &'w str,
}

impl Provenance<'_, '_> {
    fn evidence(&mut self, kind: &str, owner: &str, facts: &[Evidence]) -> ReportResult<()> {
        for fact in facts {
            let id = digest(&(kind, owner, fact))?;
            if self.admit(&id)? {
                self.writer.push([
                    id,
                    kind.into(),
                    owner.into(),
                    "unavailable".into(),
                    text(fact.source.url.as_deref()),
                    text(Some(&fact.source.id)),
                    "unavailable".into(),
                    "unavailable".into(),
                    "unavailable".into(),
                    encode(&fact.method)?,
                    text(Some(&fact.observed_on)),
                    "capture_link_not_retained".into(),
                    "interpretation_version_not_retained".into(),
                    self.generation.into(),
                    encode(fact)?,
                ])?;
            }
        }
        Ok(())
    }

    fn identity(&mut self, owner: &str, identity: &SourceIdentity) -> ReportResult<()> {
        let id = digest(&("performance_source_identity", owner, identity))?;
        if self.admit(&id)? {
            self.writer.push([
                id,
                "performance".into(),
                owner.into(),
                identity.namespace.to_string(),
                text(identity.url.as_deref()),
                text(Some(&identity.id)),
                "unavailable".into(),
                "unavailable".into(),
                "unavailable".into(),
                "athlete_source_identity_not_result_capture".into(),
                "unavailable".into(),
                "capture_link_not_retained".into(),
                "interpretation_version_not_retained".into(),
                self.generation.into(),
                encode(identity)?,
            ])?;
        }
        Ok(())
    }

    fn cohort(&mut self, athlete: &CanonicalAthlete) -> ReportResult<()> {
        for claim in &athlete.published_graduations {
            self.source_fact(
                "published_graduation",
                athlete.id.as_str(),
                &claim.source,
                claim,
            )?;
        }
        for claim in &athlete.observed_grades {
            self.source_fact("observed_grade", athlete.id.as_str(), &claim.source, claim)?;
        }
        Ok(())
    }

    fn source_fact(
        &mut self,
        kind: &str,
        owner: &str,
        source: &SourceRef,
        fact: &impl serde::Serialize,
    ) -> ReportResult<()> {
        let id = digest(&(kind, owner, fact))?;
        if self.admit(&id)? {
            self.writer.push([
                id,
                "athlete".into(),
                owner.into(),
                "unavailable".into(),
                text(source.url.as_deref()),
                text(Some(&source.id)),
                "unavailable".into(),
                "unavailable".into(),
                "unavailable".into(),
                kind.into(),
                "unavailable".into(),
                "capture_link_not_retained".into(),
                "interpretation_version_not_retained".into(),
                self.generation.into(),
                encode(fact)?,
            ])?;
        }
        Ok(())
    }

    fn admit(&mut self, id: &str) -> ReportResult<bool> {
        if self.seen.contains(id) {
            return Ok(false);
        }
        self.budget.record(&id)?;
        self.seen.insert(id.into());
        Ok(true)
    }
}
