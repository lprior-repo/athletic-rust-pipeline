use super::*;
use crate::export::public_identity::output::ResolutionAudit;
use crate::export::public_identity::tests::fixture::{merge, source_births, TestResult};
use crate::export::ExportDataset;
use crate::report::{Derivation, ReportError, Scope};
use crate::workbook::Options;
use census_domain::model::SchoolYear;
use census_domain::public_identity_history::PublicResolution;

#[path = "../../../normalized/identity_aliases/tests/fixture.rs"]
mod fixture;

type Fields = BTreeMap<String, String>;

fn selected_alias_headers() -> Vec<&'static str> {
    vec![
        "original_id",
        "canonical_id",
        "entity_type",
        "historical_automatic_identity_status",
        "successor_ids",
        "effective_revision",
        "resolution_reference",
        "input_generation",
        "publication_membership",
        "historical_state_status",
        "resolved_id",
        "resolution_reference_status",
        "public_athlete_id",
        "publication_identity_status",
        "identity_resolution",
    ]
}

fn options() -> TestResult<Options> {
    Ok(Options {
        grad_year: None,
        out: None,
        limit: None,
        scope: Scope::AllSources,
        jurisdictions: Vec::new(),
        school_year: SchoolYear::new(2026).ok_or("school year")?,
    })
}

fn selected_inputs<'a>(dataset: &'a ExportDataset, options: &'a Options) -> TestResult<Inputs<'a>> {
    let mut inputs = Inputs::new(dataset, options)?;
    inputs.derivation = Derivation::for_publication(
        dataset,
        options.scope,
        options.grad_year,
        &options.jurisdictions,
    )?;
    Ok(inputs)
}

fn alias_records(inputs: &Inputs<'_>) -> TestResult<Vec<Fields>> {
    let view = inputs.dataset.verified_public_identities()?;
    let projection = inputs.dataset.identities();
    let members: BTreeSet<_> = inputs
        .derivation
        .athletes()
        .iter()
        .map(|row| &row.id)
        .collect();
    inputs
        .dataset
        .athletes
        .iter()
        .map(|athlete| {
            let id = athlete.id.as_str();
            let group = view.group(id).ok_or("retained alias group")?;
            let canonical = group.canonical_candidate();
            let included = members
                .iter()
                .any(|member| member.as_str() == canonical.as_str());
            let resolution = view
                .candidate_resolution(id)
                .ok_or("retained alias resolution")?;
            let state = match resolution {
                PublicResolution::Canonical(_) => "canonical",
                PublicResolution::Redirect { .. } => "redirect",
                PublicResolution::RevokedRedirect { .. } => "revoked_redirect",
                PublicResolution::Unknown => "unknown",
                PublicResolution::Unresolved => "unresolved",
                PublicResolution::Ambiguous { .. } => "ambiguous",
            };
            let references: BTreeSet<_> = projection
                .alias_transitions()
                .iter()
                .filter(|transition| transition.subject() == id)
                .map(|transition| transition.decision_id())
                .collect();
            let fields = [
                ("original_id", id.to_owned()),
                (
                    "canonical_id",
                    if included {
                        canonical.as_str()
                    } else {
                        "unavailable"
                    }
                    .into(),
                ),
                ("entity_type", "athlete".into()),
                (
                    "historical_automatic_identity_status",
                    projection.status(id)?.as_str().into(),
                ),
                ("successor_ids", "unavailable".into()),
                ("effective_revision", "unavailable".into()),
                ("resolution_reference", serde_json::to_string(&references)?),
                (
                    "input_generation",
                    inputs.dataset.lineage.input_generation.clone(),
                ),
                (
                    "publication_membership",
                    if included {
                        "canonical_in_publication"
                    } else {
                        "canonical_outside_publication"
                    }
                    .into(),
                ),
                (
                    "historical_state_status",
                    "effective_revision_split_successors_supersession_not_retained".into(),
                ),
                ("resolved_id", canonical.to_string()),
                (
                    "resolution_reference_status",
                    if references.is_empty() {
                        "no_retained_alias_transition"
                    } else {
                        "retained_projection_transitions"
                    }
                    .into(),
                ),
                (
                    "public_athlete_id",
                    group
                        .public_id()
                        .map_or_else(String::new, |id| id.as_str().to_owned()),
                ),
                ("publication_identity_status", state.into()),
                (
                    "identity_resolution",
                    serde_json::to_string(&ResolutionAudit(resolution))?,
                ),
            ];
            Ok(fields
                .into_iter()
                .map(|(name, value)| (name.to_owned(), value))
                .collect())
        })
        .collect()
}

fn write_records(path: &Path, header: &[&str], records: &[Fields]) -> TestResult {
    let mut writer = csv::Writer::from_path(path)?;
    writer.write_record(header)?;
    for record in records {
        let values = header
            .iter()
            .map(|column| {
                record
                    .get(*column)
                    .map(String::as_str)
                    .ok_or("missing fixture column")
            })
            .collect::<Result<Vec<_>, _>>()?;
        writer.write_record(values)?;
    }
    writer.flush()?;
    Ok(())
}

fn rejected<T>(result: ReportResult<T>, filename: &str) -> TestResult {
    match result {
        Err(ReportError::Invariant { detail }) => check!(detail.contains(filename)),
        Err(error) => return Err(format!("unexpected readback refusal: {error}").into()),
        Ok(_) => return Err(format!("tampered {filename} passed independent readback").into()),
    }
    Ok(())
}

#[test]
fn independent_alias_readback_accepts_mixed_public_births_and_unknown_membership() -> TestResult {
    let source = source_births()?;
    let dataset = fixture::selected_dataset(&source.store)?;
    let options = options()?;
    let inputs = selected_inputs(&dataset, &options)?;
    check!(eq; inputs.derivation.athletes().len(), 2);
    let records = alias_records(&inputs)?;
    write_records(
        &source.directory.path().join("identity_aliases.csv"),
        &selected_alias_headers(),
        &records,
    )?;
    check!(eq; aliases(source.directory.path(), &inputs)?, 3);
    Ok(())
}

#[test]
fn independent_alias_readback_accepts_committed_merge_candidate_and_public_joins() -> TestResult {
    let source = source_births()?;
    merge(&source)?;
    let dataset = fixture::selected_dataset(&source.store)?;
    let options = options()?;
    let inputs = selected_inputs(&dataset, &options)?;
    check!(eq; inputs.derivation.athletes().len(), 1);
    let records = alias_records(&inputs)?;
    write_records(
        &source.directory.path().join("identity_aliases.csv"),
        &selected_alias_headers(),
        &records,
    )?;
    check!(eq; aliases(source.directory.path(), &inputs)?, 3);
    Ok(())
}

#[test]
fn independent_alias_readback_rejects_swapped_candidate_public_keys_and_foreign_births(
) -> TestResult {
    let source = source_births()?;
    let dataset = fixture::selected_dataset(&source.store)?;
    let options = options()?;
    let inputs = selected_inputs(&dataset, &options)?;
    let original = alias_records(&inputs)?;
    let [first, _, _] = &source.athletes;
    let [birth, other] = &source.births;
    let path = source.directory.path().join("identity_aliases.csv");
    let header = selected_alias_headers();
    write_records(&path, &header, &original)?;
    check!(eq; aliases(source.directory.path(), &inputs)?, 3);
    for mutation in [
        vec![("canonical_id", birth.public_id().as_str())],
        vec![("resolved_id", birth.public_id().as_str())],
        vec![("public_athlete_id", first.id.as_str())],
        vec![("public_athlete_id", other.public_id().as_str())],
        vec![
            ("canonical_id", birth.public_id().as_str()),
            ("resolved_id", birth.public_id().as_str()),
            ("public_athlete_id", first.id.as_str()),
        ],
    ] {
        let mut corrupted = original.clone();
        let row = corrupted
            .iter_mut()
            .find(|row| row.get("original_id").map(String::as_str) == Some(first.id.as_str()))
            .ok_or("candidate row to corrupt")?;
        for (column, value) in mutation {
            row.insert(column.into(), value.into());
        }
        write_records(&path, &header, &corrupted)?;
        rejected(
            aliases(source.directory.path(), &inputs),
            "identity_aliases.csv",
        )?;
    }
    write_records(&path, &header, &original)?;
    check!(eq; aliases(source.directory.path(), &inputs)?, 3);
    Ok(())
}

#[test]
fn independent_alias_readback_rejects_unknown_candidate_claiming_publication_membership(
) -> TestResult {
    let source = source_births()?;
    let dataset = fixture::selected_dataset(&source.store)?;
    let options = options()?;
    let inputs = selected_inputs(&dataset, &options)?;
    let [_, _, unknown] = &source.athletes;
    let mut records = alias_records(&inputs)?;
    let row = records
        .iter_mut()
        .find(|row| row.get("original_id").map(String::as_str) == Some(unknown.id.as_str()))
        .ok_or("unknown alias row")?;
    row.insert("canonical_id".into(), unknown.id.to_string());
    row.insert(
        "publication_membership".into(),
        "canonical_in_publication".into(),
    );
    write_records(
        &source.directory.path().join("identity_aliases.csv"),
        &selected_alias_headers(),
        &records,
    )?;
    rejected(
        aliases(source.directory.path(), &inputs),
        "identity_aliases.csv",
    )
}

fn athlete_record(inputs: &Inputs<'_>) -> TestResult<Fields> {
    let [athlete] = inputs.derivation.athletes() else {
        return Err("expected one selected normalized athlete".into());
    };
    let view = inputs.dataset.verified_public_identities()?;
    let public_id = view.public_id(athlete.id.as_str()).ok_or("public birth")?;
    let resolution = view
        .candidate_resolution(athlete.id.as_str())
        .ok_or("birth resolution")?;
    let affiliation = inputs
        .derivation
        .current_affiliation(&athlete.id)
        .ok_or("affiliation")?;
    let cohort: BTreeSet<_> = athlete
        .published_graduations
        .iter()
        .map(|fact| rows::digest(&("published_graduation", athlete.id.as_str(), fact)))
        .chain(
            athlete
                .observed_grades
                .iter()
                .map(|fact| rows::digest(&("observed_grade", athlete.id.as_str(), fact))),
        )
        .collect::<ReportResult<_>>()?;
    let fields = [
        ("athlete_id", public_id.as_str().to_owned()),
        ("display_name", athlete.canonical_name.clone()),
        ("graduation_year", athlete.grad_year.get().to_string()),
        (
            "graduation_status",
            "resolved_by_shared_cohort_policy".into(),
        ),
        ("competition_category", athlete.gender.stable_key().into()),
        ("competition_category_status", "observed".into()),
        ("current_school_id", athlete.school.to_string()),
        (
            "current_school_status",
            "resolved_at_selected_cutoff".into(),
        ),
        ("cohort_evidence_refs", serde_json::to_string(&cohort)?),
        (
            "affiliation_evidence_refs",
            serde_json::to_string(&affiliation.evidence_refs())?,
        ),
        (
            "performance_availability",
            "no_eligible_selected_mark_in_snapshot".into(),
        ),
        (
            "source_identities",
            serde_json::to_string(&(&athlete.source, &athlete.source_links))?,
        ),
        ("record_revision", "unavailable".into()),
        (
            "record_revision_status",
            "record_revision_not_retained".into(),
        ),
        (
            "input_generation",
            inputs.dataset.lineage.input_generation.clone(),
        ),
        (
            "cohort_claims",
            serde_json::to_string(&(&athlete.published_graduations, &athlete.observed_grades))?,
        ),
        ("identity_status", "accepted_public_identity".into()),
        (
            "entity_evidence_refs",
            serde_json::to_string(&facts::references(
                "athlete",
                athlete.id.as_str(),
                &athlete.evidence,
            )?)?,
        ),
        (
            "affiliation_evidence_status",
            "retained_selected_affiliation_evidence".into(),
        ),
        ("canonical_candidate_id", athlete.id.to_string()),
        (
            "identity_resolution",
            serde_json::to_string(&ResolutionAudit(resolution))?,
        ),
    ];
    Ok(fields
        .into_iter()
        .map(|(name, value)| (name.to_owned(), value))
        .collect())
}

#[test]
fn normalized_athlete_readback_accepts_retained_public_status_and_rejects_legacy_labels(
) -> TestResult {
    let fixture = crate::workbook::public_identity::fixture::Fixture::new()?;
    let inputs = selected_inputs(&fixture.dataset, &fixture.options)?;
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("athletes.csv");
    let header = public_identity::headers(&fixture.dataset, schema::ATHLETES);
    let original = athlete_record(&inputs)?;
    write_records(&path, &header, std::slice::from_ref(&original))?;
    check!(eq; verify(directory.path(), &inputs, &[])?, 1);
    for legacy in ["verified", "review", "retained_conflict"] {
        let mut row = original.clone();
        row.insert("identity_status".into(), legacy.into());
        write_records(&path, &header, &[row])?;
        rejected(verify(directory.path(), &inputs, &[]), "athletes.csv")?;
    }
    write_records(&path, &header, &[original])?;
    check!(eq; verify(directory.path(), &inputs, &[])?, 1);
    Ok(())
}

#[test]
fn recruiting_csv_readback_accepts_retained_public_status_and_rejects_legacy_labels() -> TestResult
{
    let fixture = crate::workbook::public_identity::fixture::Fixture::new()?;
    let inputs = selected_inputs(&fixture.dataset, &fixture.options)?;
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("recruiting.csv");
    crate::workbook::write_recruiting_csv(&inputs.derivation, inputs.school_year, &path)?;
    super::super::super::recruiting::verify(directory.path(), &inputs)?;
    let mut reader = csv::Reader::from_path(&path)?;
    let header = reader.headers()?.clone();
    let records = reader
        .records()
        .map(|record| {
            let record = record?;
            Ok(header
                .iter()
                .zip(record.iter())
                .map(|(name, value)| (name.to_owned(), value.to_owned()))
                .collect())
        })
        .collect::<TestResult<Vec<Fields>>>()?;
    let [original] = records.as_slice() else {
        return Err("expected one selected recruiting CSV row".into());
    };
    check!(eq; original.get("identity_status").map(String::as_str), Some("accepted_public_identity"));
    let header: Vec<_> = header.iter().collect();
    for legacy in ["verified", "review", "retained_conflict"] {
        let mut row = original.clone();
        row.insert("identity_status".into(), legacy.into());
        write_records(&path, &header, &[row])?;
        rejected(
            super::super::super::recruiting::verify(directory.path(), &inputs),
            "recruiting.csv",
        )?;
    }
    write_records(&path, &header, &records)?;
    super::super::super::recruiting::verify(directory.path(), &inputs)?;
    Ok(())
}
