use super::super::input::Inputs;
use super::{rows, schema, schools::Schools};
use crate::bests::SharedSelection;
use crate::report::ReportResult;
use census_domain::model::{CanonicalCoach, Evidence, SourceRef};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub(super) struct Fact<'a> {
    entity: &'static str,
    owner: String,
    namespace: String,
    url: Option<&'a str>,
    reference: &'a str,
    observed: &'a str,
    kind: String,
    retained: serde_json::Value,
}

pub(super) struct Facts<'a> {
    facts: BTreeMap<String, Fact<'a>>,
    budget: crate::grouping::Admission,
}

pub(super) fn references(
    kind: &str,
    owner: &str,
    evidence: &[Evidence],
) -> ReportResult<BTreeSet<String>> {
    evidence
        .iter()
        .map(|fact| rows::digest(&(kind, owner, fact)))
        .collect()
}

pub(super) fn inventory<'a>(
    inputs: &'a Inputs<'_>,
    schools: &Schools<'a>,
    coaches: &'a [CanonicalCoach],
    bests: &[SharedSelection],
) -> ReportResult<Facts<'a>> {
    let mut facts = Facts {
        facts: BTreeMap::new(),
        budget: crate::grouping::Admission::new("normalized evidence readback"),
    };
    inventory_athletes(inputs, &mut facts)?;
    inventory_schools(schools, &mut facts)?;
    inventory_coaches(coaches, &mut facts)?;
    inventory_bests(inputs, bests, &mut facts)?;
    Ok(facts)
}

fn inventory_athletes<'a>(inputs: &'a Inputs<'_>, facts: &mut Facts<'a>) -> ReportResult<()> {
    inputs.derivation.athletes().iter().try_for_each(|athlete| {
        let owner = athlete.id.as_str();
        facts.evidence("athlete", owner, &athlete.evidence)?;
        athlete.published_graduations.iter().try_for_each(|claim| {
            facts.cohort("published_graduation", owner, &claim.source, claim)
        })?;
        athlete
            .observed_grades
            .iter()
            .try_for_each(|claim| facts.cohort("observed_grade", owner, &claim.source, claim))
    })
}

fn inventory_schools<'a>(schools: &Schools<'a>, facts: &mut Facts<'a>) -> ReportResult<()> {
    schools.values().try_for_each(|reference| {
        let super::schools::Reference::Resolved(school) = reference else {
            return Ok(());
        };
        facts.evidence("school", school.id.as_str(), &school.evidence)?;
        school.postal_addresses.iter().try_for_each(|claim| {
            facts.evidence(
                "school_address",
                &rows::digest(claim)?,
                std::slice::from_ref(claim.evidence()),
            )
        })
    })
}

fn inventory_coaches<'a>(coaches: &'a [CanonicalCoach], facts: &mut Facts<'a>) -> ReportResult<()> {
    coaches
        .iter()
        .try_for_each(|coach| facts.evidence("coach", coach.id.as_str(), &coach.evidence))
}

fn inventory_bests<'a>(
    inputs: &'a Inputs<'_>,
    bests: &[SharedSelection],
    facts: &mut Facts<'a>,
) -> ReportResult<()> {
    let performances: BTreeMap<_, _> = inputs
        .derivation
        .performances()
        .iter()
        .map(|performance| (&performance.id, *performance))
        .collect();
    let ids: BTreeSet<_> = bests
        .iter()
        .flat_map(|best| best.tied_performance_ids.iter())
        .collect();
    ids.into_iter().try_for_each(|id| {
        let performance = performances
            .get(id)
            .ok_or_else(|| rows::missing("evidence performance", id.as_str()))?;
        facts.evidence("performance", id.as_str(), &performance.evidence)?;
        if let Some(identity) = &performance.source_athlete {
            facts.identity(id.as_str(), identity)?;
        }
        Ok::<_, crate::report::ReportError>(())
    })
}

impl<'a> Facts<'a> {
    fn insert(&mut self, id: String, fact: Fact<'a>) -> ReportResult<()> {
        if self.facts.contains_key(&id) {
            return Ok(());
        }
        self.budget.record(&(&id, &fact.retained))?;
        self.facts.insert(id, fact);
        Ok(())
    }

    fn evidence(
        &mut self,
        kind: &'static str,
        owner: &str,
        evidence: &'a [Evidence],
    ) -> ReportResult<()> {
        evidence.iter().try_for_each(|fact| {
            let id = rows::digest(&(kind, owner, fact))?;
            let retained = json(fact)?;
            let kind_name = serde_json::to_string(&fact.method)
                .map_err(|error| super::super::defect(error.to_string()))?;
            self.insert(
                id,
                Fact {
                    entity: kind,
                    owner: owner.into(),
                    namespace: "unavailable".into(),
                    url: fact.source.url.as_deref(),
                    reference: &fact.source.id,
                    observed: &fact.observed_on,
                    kind: kind_name,
                    retained,
                },
            )
        })
    }

    fn cohort(
        &mut self,
        kind: &'static str,
        owner: &str,
        source: &'a SourceRef,
        claim: &impl serde::Serialize,
    ) -> ReportResult<()> {
        self.insert(
            rows::digest(&(kind, owner, claim))?,
            Fact {
                entity: "athlete",
                owner: owner.into(),
                namespace: "unavailable".into(),
                url: source.url.as_deref(),
                reference: &source.id,
                observed: "unavailable",
                kind: kind.into(),
                retained: json(claim)?,
            },
        )
    }

    fn identity(
        &mut self,
        owner: &str,
        identity: &'a census_domain::model::SourceIdentity,
    ) -> ReportResult<()> {
        self.insert(
            rows::digest(&("performance_source_identity", owner, identity))?,
            Fact {
                entity: "performance",
                owner: owner.into(),
                namespace: identity.namespace.to_string(),
                url: identity.url.as_deref(),
                reference: &identity.id,
                observed: "unavailable",
                kind: "athlete_source_identity_not_result_capture".into(),
                retained: json(identity)?,
            },
        )
    }
}

fn json(value: &impl serde::Serialize) -> ReportResult<serde_json::Value> {
    serde_json::to_value(value).map_err(|error| super::super::defect(error.to_string()))
}

pub(super) fn verify(directory: &Path, inputs: &Inputs<'_>, facts: &Facts<'_>) -> ReportResult<()> {
    rows::table(
        directory,
        ("provenance.csv", schema::PROVENANCE),
        facts.facts.len(),
        &["evidence_id"],
        |row| {
            let id = row.get("evidence_id")?;
            let fact = facts
                .facts
                .get(id)
                .ok_or_else(|| rows::missing("evidence", id))?;
            row.text("entity_type", fact.entity)?;
            row.text("entity_id", &fact.owner)?;
            row.text("source_namespace", &fact.namespace)?;
            row.text("source_url", rows::available(fact.url))?;
            row.text("source_reference", rows::available(Some(fact.reference)))?;
            row.text("observed_on", rows::available(Some(fact.observed)))?;
            row.text("evidence_type", &fact.kind)?;
            compare_fact(row, &fact.retained)?;
            row.text("input_generation", &inputs.dataset.lineage.input_generation)?;
            row.literals(&[
                ("capture_id", "unavailable"),
                ("capture_time", "unavailable"),
                ("interpretation_version", "unavailable"),
                ("capture_status", "capture_link_not_retained"),
                (
                    "interpretation_status",
                    "interpretation_version_not_retained",
                ),
            ])
        },
    )
}

fn compare_fact(row: &rows::Row<'_>, expected: &serde_json::Value) -> ReportResult<()> {
    let found = row.get("retained_fact")?;
    let found: serde_json::Value =
        serde_json::from_str(found).map_err(|error| super::super::defect(error.to_string()))?;
    if &found == expected {
        Ok(())
    } else {
        Err(super::super::defect(
            "normalized retained evidence fact differs from frozen input".into(),
        ))
    }
}
