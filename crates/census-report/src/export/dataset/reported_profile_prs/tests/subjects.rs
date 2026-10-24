use super::{source, ReportedProfilePrs, TestResult};
use crate::export::dataset::initial_objects::InitialObjects;
use crate::export::dataset::ExportDataset;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::{
    CanonicalAthlete, Gender, GradYear, ProfileDate, SchoolId, SourceIdentity, SourceNamespace,
};
use census_store::{RetainedCaptureRootSchemaV1, Store, Table};
use std::path::Path;

const SUBJECT_A: &str = "7223777";
const SUBJECT_B: &str = "7223778";

#[test]
fn reported_tables_include_each_distinct_subject_once_when_locator_aliases_are_preserved(
) -> TestResult {
    let directory = tempfile::tempdir()?;
    let objects = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let (expected_roots, expected_pairs) = retain_subjects(&store)?;
    let canonical_before = store.snapshot().tables_digest(&Table::ALL)?;
    let athlete = aliased_athlete()?;
    let aliases_before = serde_json::to_vec(&athlete)?;
    let mut dataset = ExportDataset::load(&store)?;
    check!(dataset.athletes.is_empty());
    check!(dataset.reported_profile_prs.is_empty());
    dataset.reported_profile_prs = ReportedProfilePrs::capture(
        &store.snapshot(),
        [&athlete],
        &ProfileDate::parse("2026-10-21")?,
        objects.path(),
        &mut AttachmentReplayGrantV1::new(),
        &InitialObjects::fresh(),
    )?;
    check!(eq; dataset.reported_profile_prs.roots.len(), 2);
    authenticate_roots(
        &dataset.reported_profile_prs,
        objects.path(),
        &expected_roots,
    )?;
    check!(eq; dataset.reported_profile_prs.tables.keys().cloned().collect::<Vec<_>>(),
        vec![SUBJECT_A.to_owned(), SUBJECT_B.to_owned()]);
    check!(eq; dataset.reported_profile_prs.tables.values().flatten().count(), 2);
    require_namespace_and_first_seen_order(&dataset, &expected_roots, &expected_pairs)?;
    let tables = dataset
        .reported_profile_prs_for(&athlete)
        .collect::<Vec<_>>();
    check!(eq; serde_json::to_vec(&athlete)?, aliases_before);
    check!(eq; store.snapshot().tables_digest(&Table::ALL)?, canonical_before);
    check!(dataset.athletes.is_empty());
    check!(eq; tables.len(), 2);
    let mut returned_roots = tables
        .iter()
        .map(|table| table.root.key.clone())
        .collect::<Vec<_>>();
    returned_roots.sort_unstable();
    check!(eq; returned_roots, expected_roots);
    let mut returned_subjects = tables
        .iter()
        .map(|table| table.source.id.clone())
        .collect::<Vec<_>>();
    returned_subjects.sort_unstable();
    check!(eq; returned_subjects, vec![SUBJECT_A.to_owned(), SUBJECT_B.to_owned()]);
    let mut returned_pairs = tables
        .iter()
        .map(|table| (table.source.id.clone(), table.root.key.clone()))
        .collect::<Vec<_>>();
    returned_pairs.sort_unstable();
    check!(eq; returned_pairs, expected_pairs);
    Ok(())
}

type RetainedSubjects = (Vec<String>, Vec<(String, String)>);

fn retain_subjects(store: &Store) -> TestResult<RetainedSubjects> {
    let mut references = Vec::new();
    let mut pairs = Vec::new();
    for subject in [SUBJECT_A, SUBJECT_B] {
        let packet = source::packet(subject, "2026-10-16T00:50:27Z")?;
        let reference = packet.reference()?.as_str().to_owned();
        let source_subject = packet.read()?.profile().person().id.clone();
        pairs.push((source_subject, reference.clone()));
        store.journal_done(
            RetainedCaptureRootSchemaV1::ProviderTargetPacketV1.phase(),
            &reference,
            &packet,
        )?;
        references.push(reference);
    }
    references.sort_unstable();
    pairs.sort_unstable();
    Ok((references, pairs))
}

fn aliased_athlete() -> TestResult<CanonicalAthlete> {
    let mut athlete = CanonicalAthlete::new(
        &SchoolId::mint("sch", &["profile subject alias control"]),
        "Profile subject alias control",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, SUBJECT_A)
            .with_url("https://mi.milesplit.com/athletes/7223777-zacchaeus-brocks"),
    )?;
    for (subject, url) in [
        (
            SUBJECT_B,
            "https://mi.milesplit.com/athletes/7223778-zacchaeus-brocks",
        ),
        (SUBJECT_A, "https://mi.milesplit.com/athletes/7223777"),
        (SUBJECT_B, "https://mi.milesplit.com/athletes/7223778"),
    ] {
        athlete.add_identity(
            SourceIdentity::new(SourceNamespace::MilesplitAthlete, subject).with_url(url),
        );
    }
    athlete.add_identity(
        SourceIdentity::new(SourceNamespace::TfrrsAthlete, SUBJECT_A)
            .with_url("https://www.tfrrs.org/athletes/7223777.html"),
    );
    check!(eq; athlete.identities().count(), 5);
    let mut subjects = athlete
        .identities()
        .filter(|source| source.namespace == SourceNamespace::MilesplitAthlete)
        .map(|source| source.id.as_str())
        .collect::<Vec<_>>();
    check!(eq; subjects, vec![SUBJECT_A, SUBJECT_B, SUBJECT_A, SUBJECT_B]);
    subjects.sort_unstable();
    check!(eq; subjects, vec![SUBJECT_A, SUBJECT_A, SUBJECT_B, SUBJECT_B]);
    check!(athlete.identities().all(|source| source.url.is_some()));
    Ok(athlete)
}

fn require_namespace_and_first_seen_order(
    dataset: &ExportDataset,
    expected_roots: &[String],
    expected_pairs: &[(String, String)],
) -> TestResult {
    let foreign = CanonicalAthlete::new(
        &SchoolId::mint("sch", &["namespace-only alias control"]),
        "Namespace-only alias control",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::TfrrsAthlete, SUBJECT_A)
            .with_url("https://www.tfrrs.org/athletes/7223777.html"),
    )?;
    let foreign_before = serde_json::to_vec(&foreign)?;
    check!(eq; dataset.reported_profile_prs_for(&foreign).count(), 0);
    check!(eq; serde_json::to_vec(&foreign)?, foreign_before);
    let reverse = reverse_aliased_athlete()?;
    let reverse_before = serde_json::to_vec(&reverse)?;
    let tables = dataset
        .reported_profile_prs_for(&reverse)
        .collect::<Vec<_>>();
    let subjects = tables
        .iter()
        .map(|table| table.source.id.as_str())
        .collect::<Vec<_>>();
    check!(eq; subjects, vec![SUBJECT_B, SUBJECT_A]);
    let mut roots = tables
        .iter()
        .map(|table| table.root.key.clone())
        .collect::<Vec<_>>();
    roots.sort_unstable();
    check!(eq; roots.as_slice(), expected_roots);
    let mut pairs = tables
        .iter()
        .map(|table| (table.source.id.clone(), table.root.key.clone()))
        .collect::<Vec<_>>();
    pairs.sort_unstable();
    check!(eq; pairs.as_slice(), expected_pairs);
    check!(eq; serde_json::to_vec(&reverse)?, reverse_before);
    Ok(())
}

fn reverse_aliased_athlete() -> TestResult<CanonicalAthlete> {
    let mut athlete = CanonicalAthlete::new(
        &SchoolId::mint("sch", &["reverse subject alias control"]),
        "Reverse subject alias control",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, SUBJECT_B)
            .with_url("https://mi.milesplit.com/athletes/7223778-zacchaeus-brocks"),
    )?;
    for (subject, url) in [
        (
            SUBJECT_A,
            "https://mi.milesplit.com/athletes/7223777-zacchaeus-brocks",
        ),
        (SUBJECT_B, "https://mi.milesplit.com/athletes/7223778"),
        (SUBJECT_A, "https://mi.milesplit.com/athletes/7223777"),
    ] {
        athlete.add_identity(
            SourceIdentity::new(SourceNamespace::MilesplitAthlete, subject).with_url(url),
        );
    }
    let subjects = athlete
        .identities()
        .map(|source| source.id.as_str())
        .collect::<Vec<_>>();
    check!(eq; subjects, vec![SUBJECT_B, SUBJECT_A, SUBJECT_B, SUBJECT_A]);
    check!(athlete.identities().all(|source| source.url.is_some()));
    Ok(athlete)
}

fn authenticate_roots(
    reported: &ReportedProfilePrs,
    directory: &Path,
    expected: &[String],
) -> TestResult {
    reported.require_initial_objects(directory, &InitialObjects::fresh())?;
    let mut keys = reported
        .roots
        .iter()
        .map(|root| root.key.clone())
        .collect::<Vec<_>>();
    keys.sort_unstable();
    check!(eq; keys.as_slice(), expected);
    let mut subjects = Vec::new();
    for root in &reported.roots {
        let bytes = crate::export::proof_objects::read(
            directory,
            root.payload.digest.as_str(),
            root.payload.bytes,
        )?;
        let (subject, table) = super::super::replay::read(root, &bytes)?;
        check!(eq; table.root.key.as_str(), root.key.as_str());
        check!(eq; &table.source.namespace, &SourceNamespace::MilesplitAthlete);
        check!(eq; table.source.id.as_str(), subject.as_str());
        check!(eq; table.rows.len(), 46);
        subjects.push(subject);
    }
    subjects.sort_unstable();
    check!(eq; subjects, vec![SUBJECT_A.to_owned(), SUBJECT_B.to_owned()]);
    Ok(())
}
