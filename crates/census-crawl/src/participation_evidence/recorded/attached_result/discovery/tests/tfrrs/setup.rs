use super::*;
use census_store::read::JournalNamespace;
use census_store::{ReadBudget, MAX_JOURNAL_VALUE_BYTES};

pub(super) struct Target {
    pub athlete: CanonicalAthlete,
    pub public: PublicAthleteId,
}

pub(super) struct Input {
    pub context: String,
    pub targets: Vec<Target>,
}

pub(super) fn install() -> TestResult<(tempfile::TempDir, Store, Input)> {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("store");
    Store::create_public_identity_qualification_v3(&root)?;
    let (store, writer) = Store::open_public_identity_qualification_v3(&root)?.into_parts();
    writer.enroll_graph_admission_v1(&store, store.snapshot().revision())?;
    let capture = common_fixture::capture(CheckedHtmlResultSourceV1::Tfrrs)?;
    let checked = read_retained_tfrrs_list_rows(&capture, &locators())?;
    let recording = Recording::new();
    record_retained_tfrrs_list_capture(&store, &recording, &checked)?;
    let recorded = recording.drain();
    check!(recorded.rows.is_empty());
    check!(recorded.effects.is_empty());
    check!(eq; recorded.journal.len(), 1);
    crate::recording::prepare_recorded(recorded, "controlled-tfrrs-discovery-original-source")?
        .apply(&store)?;
    let context = checked.context_reference().to_owned();
    inventory(&store, &context)?;
    let mut targets = Vec::new();
    for index in 0..checked.selected_count() {
        let row = checked.row(index).ok_or("original TFRRS row")?;
        let athlete = target(&row)?;
        store.append(Table::Athletes, &athlete)?;
        let birth = writer.establish_source_bound(
            &store,
            &format!("controlled-tfrrs-native-target-{}", row.athlete_id()),
            &athlete.id.cast(),
            store.snapshot().revision(),
            &writer.head(&store)?,
        )?;
        check!(!birth.replayed());
        targets.push(Target {
            athlete,
            public: birth.public_id().clone(),
        });
    }
    drop(writer);
    Ok((directory, store, Input { context, targets }))
}

pub(super) fn target(
    row: &crate::participation_evidence::result_ownership::tfrrs::CheckedRetainedTfrrsRowV1<'_>,
) -> TestResult<CanonicalAthlete> {
    let school = SchoolId::mint("sch", &["controlled-tfrrs-review-target", row.team_path()]);
    let url = url::Url::parse("https://indiana.tfrrs.org")?.join(row.athlete_path())?;
    let mut athlete = CanonicalAthlete::new(
        &school,
        row.athlete_name(),
        GradYear::CO2027,
        row.gender(),
        SourceIdentity::new(SourceNamespace::TfrrsAthlete, row.athlete_id()).with_url(url.as_str()),
    )?;
    let mut evidence = Evidence::parsed(
        SourceRef::new(
            "controlled-tfrrs-attachment-target/v1",
            Some(url.to_string()),
        ),
        "2026-10-21",
    );
    evidence.note = Some("Test-only target with exact original native identity; cohort and school identifiers are controlled review inputs, without cohort, membership or eligibility admission".into());
    athlete.evidence.push(evidence);
    check!(athlete.observed_grades.is_empty());
    check!(athlete.published_graduations.is_empty());
    check!(athlete.derived_cohort_confidence().is_none());
    Ok(athlete)
}

pub(super) fn locators() -> Vec<SourceRowLocator> {
    common_fixture::locators(CheckedHtmlResultSourceV1::Tfrrs)
}

fn inventory(store: &Store, key: &str) -> TestResult {
    let refused = store.snapshot().for_each_journal_root_bounded(
        JournalNamespace::new(
            RetainedCaptureRootSchemaV1::HtmlResultCaptureV1.phase(),
            key,
        )?,
        ReadBudget {
            rows: 6,
            bytes: 4 * 1024 * 1024,
            record_bytes: MAX_JOURNAL_VALUE_BYTES,
        },
        |_| Ok(()),
    );
    check!(matches!(
        refused,
        Err(census_store::StoreError::BudgetExceeded {
            what: "journal physical rows",
            needed: 7,
            available: 6,
        })
    ));
    let mut keys = Vec::new();
    let inventory = store.snapshot().for_each_journal_root_bounded(
        JournalNamespace::new(
            RetainedCaptureRootSchemaV1::HtmlResultCaptureV1.phase(),
            key,
        )?,
        ReadBudget {
            rows: 7,
            bytes: 4 * 1024 * 1024,
            record_bytes: MAX_JOURNAL_VALUE_BYTES,
        },
        |found| {
            keys.push(found.to_owned());
            Ok(())
        },
    )?;
    check!(eq; keys, vec![key.to_owned()]);
    check!(eq; inventory.logical_roots(), 1);
    check!(eq; inventory.fingerprint().physical_rows(), 7);
    Ok(())
}

pub(super) fn state(
    store: &Store,
    key: &str,
) -> TestResult<(Vec<u8>, census_store::PublicIdentityInputRevisionsV2)> {
    let snapshot = store.snapshot();
    let mut receipts = Vec::new();
    store.for_each_receipt(16, |receipt| {
        receipts.push(receipt);
        Ok(())
    })?;
    receipts.sort_by(|left, right| left.operation.cmp(&right.operation));
    let raw = snapshot
        .journal_payload_raw_bounded(
            RetainedCaptureRootSchemaV1::HtmlResultCaptureV1.phase(),
            key,
            JournalReadBudget::new(4 * 1024 * 1024)?,
        )?
        .ok_or("normal retained TFRRS root")?;
    let bytes = serde_json::to_vec(&(
        snapshot.sequence(),
        snapshot.evidence_generation(),
        snapshot.derived_generation(),
        snapshot.tables_digest(&Table::ALL)?,
        snapshot
            .accepted_public_identity_snapshot()?
            .ok_or("accepted target ledger")?
            .head(),
        receipts,
        raw.get(),
    ))?;
    Ok((bytes, snapshot.public_identity_input_revisions_v2()?))
}
