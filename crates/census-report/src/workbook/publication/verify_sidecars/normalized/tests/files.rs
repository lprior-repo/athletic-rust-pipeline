use super::super::schema;
use super::{population::Population, TestResult};
use census_domain::model::performance_comparison::{TimingClass, WindClass};
use std::path::Path;

pub(super) fn write(given: &Population, directory: &Path) -> TestResult {
    athletes(given, directory)?;
    marks(given, directory)?;
    schools(given, directory)?;
    contacts(given, directory)?;
    classes(given, directory)?;
    provenance(given, directory)?;
    coverage(given, directory)?;
    aliases(given, directory)
}

pub(super) fn put(
    directory: &Path,
    name: &str,
    header: &[&str],
    records: &[Vec<String>],
) -> TestResult {
    let mut writer = csv::WriterBuilder::new()
        .has_headers(false)
        .from_path(directory.join(name))?;
    writer.write_record(header)?;
    for record in records {
        writer.write_record(record)?;
    }
    writer.flush()?;
    Ok(())
}

fn json(value: &impl serde::Serialize) -> TestResult<String> {
    Ok(serde_json::to_string(value)?)
}
fn digest(value: &impl serde::Serialize) -> TestResult<String> {
    Ok(census_domain::model::serialized_digest(value)?)
}
fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).into()).collect()
}

fn athletes(given: &Population, directory: &Path) -> TestResult {
    let mut records = Vec::new();
    for (index, athlete) in given.athletes.iter().enumerate() {
        let claim = athlete
            .published_graduations
            .first()
            .ok_or("missing graduation fixture")?;
        let refs = json(&[digest(&(
            "published_graduation",
            athlete.id.as_str(),
            claim,
        ))?])?;
        let identities = json(&(&athlete.source, &athlete.source_links))?;
        let claims = json(&(&athlete.published_graduations, &athlete.observed_grades))?;
        records.push(strings(&[
            athlete.id.as_str(),
            if index == 0 { "Ada Runner" } else { "No Mark" },
            "2027",
            "resolved_by_shared_cohort_policy",
            "Girls",
            "observed",
            given.school.id.as_str(),
            "canonical_school_currentness_unavailable",
            &refs,
            "unavailable",
            if index == 0 {
                "eligible_selected_mark"
            } else {
                "no_eligible_selected_mark_in_snapshot"
            },
            &identities,
            "unavailable",
            "record_revision_not_retained",
            &given.dataset.lineage.input_generation,
            &claims,
            "unverified",
            "[]",
            "claim_level_affiliation_links_not_retained",
        ]));
    }
    put(directory, "athletes.csv", schema::ATHLETES, &records)
}

fn marks(given: &Population, directory: &Path) -> TestResult {
    let performance = given.performances.first().ok_or("missing manual winner")?;
    let meet = given.meets.first().ok_or("missing manual meet")?;
    let evidence = performance
        .evidence
        .first()
        .ok_or("missing manual evidence")?;
    let refs = json(&[digest(&("performance", performance.id.as_str(), evidence))?])?;
    let ties = json(
        &given
            .performances
            .iter()
            .take(2)
            .map(|fact| &fact.id)
            .collect::<Vec<_>>(),
    )?;
    let specification = json(&given.specification)?;
    let policy = given.dataset.lineage.policy_revision.to_string();
    put(
        directory,
        "best_marks.csv",
        schema::BESTS,
        &[strings(&[
            given.athletes.first().ok_or("missing Ada")?.id.as_str(),
            &given.class,
            performance.id.as_str(),
            "unavailable",
            "immutable_performance_version_not_retained",
            "Track400m",
            "unavailable",
            "source_text_only_retained_for_raw_or_imperial_marks",
            "50.00",
            "50000000000",
            "ns",
            "1",
            "fat",
            "na",
            &specification,
            meet.id.as_str(),
            "2026-05-01",
            "Readback Meet 0",
            "not_applicable",
            given.school.id.as_str(),
            "performance_team_school",
            "https://example.test/result",
            "unavailable",
            "capture_link_not_retained",
            &refs,
            "2",
            "3",
            &ties,
            &policy,
            "outdoor",
            &given.dataset.lineage.input_generation,
            "normalized_integer_quantum",
            "2",
            "retained_evidence_capture_links_unavailable",
            "retained_result_locator",
        ])],
    )
}

fn schools(given: &Population, directory: &Path) -> TestResult {
    put(
        directory,
        "schools.csv",
        schema::SCHOOLS,
        &[strings(&[
            given.school.id.as_str(),
            "Readback High",
            "[]",
            "institution_type_unavailable",
            "currentness_unavailable",
            "not_collected",
            "observed_date_is_not_verification_date",
            "[]",
            &given.dataset.lineage.input_generation,
            "record_revision_not_retained",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "unavailable",
            "[]",
            "[]",
        ])],
    )
}

fn contacts(given: &Population, directory: &Path) -> TestResult {
    let records = given
        .coaches
        .iter()
        .map(|coach| {
            strings(&[
                coach.id.as_str(),
                given.school.id.as_str(),
                coach.role.stable_key(),
                coach.role.stable_key(),
                "OutdoorTrack",
                "Girls",
                "withheld_not_admitted",
                "withheld_not_admitted",
                "unavailable",
                "no_current_claim",
                "unknown",
                "unavailable",
                "unavailable",
                "unavailable",
                "unavailable",
                "not_applicable",
                "[]",
                "unavailable",
                "unavailable",
                "[]",
                &given.dataset.lineage.input_generation,
                "phone_contact_url_effective_dates_verification_not_retained_as_admitted_claims",
            ])
        })
        .collect::<Vec<_>>();
    put(directory, "coach_contacts.csv", schema::CONTACTS, &records)
}

fn classes(given: &Population, directory: &Path) -> TestResult {
    let conditions = json(&(
        TimingClass::Fat,
        WindClass::NotApplicable,
        &given.specification,
    ))?;
    let specification = json(&given.specification)?;
    let policy = given.dataset.lineage.policy_revision.to_string();
    let catalog = census_domain::model::RecruitingCatalogV1::new().digest()?;
    put(
        directory,
        "comparison_classes.csv",
        schema::CLASSES,
        &[strings(&[
            &given.class,
            "Track400m",
            "Track",
            "Girls",
            "lower_is_better",
            "ns",
            "1",
            &conditions,
            "\"standard\"",
            "outdoor",
            &policy,
            &specification,
            "not_applicable",
            "null",
            &catalog,
            &given.dataset.lineage.input_generation,
            "normalized_integer_quantum",
            "unavailable_result_status_schema_not_retained",
        ])],
    )
}

fn provenance(given: &Population, directory: &Path) -> TestResult {
    let mut records = Vec::new();
    for athlete in &given.athletes {
        let claim = athlete
            .published_graduations
            .first()
            .ok_or("missing manual cohort")?;
        records.push(strings(&[
            &digest(&("published_graduation", athlete.id.as_str(), claim))?,
            "athlete",
            athlete.id.as_str(),
            "unavailable",
            "https://example.test/result",
            "manual-readback",
            "unavailable",
            "unavailable",
            "unavailable",
            "published_graduation",
            "unavailable",
            "capture_link_not_retained",
            "interpretation_version_not_retained",
            &given.dataset.lineage.input_generation,
            &json(claim)?,
        ]));
    }
    for performance in given.performances.iter().take(2) {
        let fact = performance
            .evidence
            .first()
            .ok_or("missing manual result evidence")?;
        records.push(strings(&[
            &digest(&("performance", performance.id.as_str(), fact))?,
            "performance",
            performance.id.as_str(),
            "unavailable",
            "https://example.test/result",
            "manual-readback",
            "unavailable",
            "unavailable",
            "unavailable",
            &json(&fact.method)?,
            "2026-05-04",
            "capture_link_not_retained",
            "interpretation_version_not_retained",
            &given.dataset.lineage.input_generation,
            &json(fact)?,
        ]));
    }
    put(directory, "provenance.csv", schema::PROVENANCE, &records)
}

fn coverage(given: &Population, directory: &Path) -> TestResult {
    let fact = given
        .dataset
        .coverage_evidence
        .rows
        .first()
        .ok_or("missing genuine coverage evidence")?;
    let cohort = given
        .dataset
        .lineage
        .cohort
        .map_or_else(|| "unavailable".into(), |year| year.get().to_string());
    put(
        directory,
        "coverage.csv",
        schema::COVERAGE,
        &[strings(&[
            &format!("metrics:{}", fact.id),
            fact.scope.slug(),
            &fact.subject,
            &cohort,
            "unavailable",
            "unavailable",
            "unavailable",
            "unavailable",
            "unavailable",
            "unavailable",
            &json(&fact.metrics)?,
            "unavailable",
            "scope_gap_metadata_not_retained",
            "unavailable_known_universe_not_retained",
            "retained_metrics_scope_not_qualified",
            &given.dataset.lineage.input_generation,
            &json(fact)?,
        ])],
    )
}

fn aliases(given: &Population, directory: &Path) -> TestResult {
    let records = given
        .dataset
        .athletes
        .iter()
        .map(|athlete| {
            let included = athlete.school == given.school.id;
            strings(&[
                athlete.id.as_str(),
                if included {
                    athlete.id.as_str()
                } else {
                    "unavailable"
                },
                "athlete",
                "unverified",
                "unavailable",
                "unavailable",
                "[]",
                &given.dataset.lineage.input_generation,
                if included {
                    "canonical_in_publication"
                } else {
                    "canonical_outside_publication"
                },
                "effective_revision_split_successors_supersession_not_retained",
                athlete.id.as_str(),
                "no_retained_alias_transition",
            ])
        })
        .collect::<Vec<_>>();
    put(directory, "identity_aliases.csv", schema::ALIASES, &records)
}
