use super as controlled;
use census_crawl::milesplit::SourceRowLocator;
use census_crawl::participation_evidence::provider_school::{
    ProviderSchoolPacketRefV1, ProviderSchoolPacketV1,
};
use census_crawl::participation_evidence::recorded::{
    discover_current_html_results, read_recorded_tfrrs_list_capture,
    record_retained_tfrrs_list_capture,
};
use census_crawl::participation_evidence::result_ownership::tfrrs::read_retained_tfrrs_list_rows;
use census_crawl::participation_evidence::{
    IndependentParticipationPacketV1, ParticipationEvidenceRefV1, RetainedBodyEncodingV1,
    RetainedHtmlCaptureV1, RetainedHttpProvenanceV1,
};
use census_crawl::Recording;
use census_domain::evidence_subject_attachment::*;
use census_domain::model::{
    CanonicalAthlete, CanonicalSchool, CanonicalTeam, Gender, GradYear, ProfileDate, SchoolYear,
    SourceIdentity, SourceNamespace, Sport,
};
use census_domain::public_identity_history::PublicAthleteId;
use census_store::{
    Application, AttachmentInspectionRequestV1, EvidenceSubjectAttachmentInspectionV1,
    EvidenceSubjectAttachmentPreparationV1, JournalReadBudget, RetainedCaptureLeafKindV1,
    RetainedCaptureLeafV1, RetainedCaptureRootSchemaV1, RetainedCaptureSlotV1, Store, Table,
};
use chrono::{DateTime, Utc};
use controlled::objects::{self, Objects};
use controlled::source::TestResult;

mod authority;
mod captures;
mod identity;
mod material;
mod output;
mod profile;
mod review;
mod roncalli_fixture;
mod roster;
mod source;

fn text(value: &str) -> TestResult<AttachmentTextV1> {
    Ok(AttachmentTextV1::parse(value)?)
}
fn list<T>(values: Vec<T>) -> TestResult<AttachmentListV1<T>> {
    Ok(AttachmentListV1::new(values)?)
}
fn state(store: &Store) -> TestResult<serde_json::Value> {
    let snapshot = store.snapshot();
    Ok(
        serde_json::json!({"sequence":snapshot.sequence(),"evidence":snapshot.evidence_generation(),"derived":snapshot.derived_generation(),"receipts":store.receipt_count()?,"tables":snapshot.tables_digest(&Table::ALL)?}),
    )
}

#[test]
fn genuine_tessa_adjusted_eligible_best_publishes_revision_eight_and_replays_cold() -> TestResult {
    let installed = source::install()?;
    identity::separate(
        &installed.store,
        &installed.athlete,
        &installed.native_candidate,
        &installed.public,
    )?;
    roster::accept(&installed)?;
    roster::require_manifest(&installed.store)?;
    identity::unattached(&installed.store)?;
    let result = review::attach(&installed, false)?;
    let status = review::attach(&installed, true)?;
    let accepted = authority::accept(&installed)?;
    let native_source =
        identity::linked(&installed.store, &installed.athlete, &result.0, &result.1)?;
    review::replay(&installed.store, result.0.clone(), &result.1)?;
    review::replay(&installed.store, status.0.clone(), &status.1)?;
    let profile = profile::query()?;
    installed.store.register_profile(&profile)?;
    let objects = tempfile::tempdir()?;
    let output = tempfile::tempdir()?;
    let dataset = controlled::selection::capture(&installed.store, &profile, objects.path())?;
    let workbook = controlled::publish::publish(
        &dataset,
        &installed.store,
        &profile,
        &output.path().join("publication"),
    )?;
    output::verify(
        &dataset,
        &workbook,
        &installed.public,
        &installed.athlete.id,
        &installed.school.id,
    )?;
    controlled::proof_refusal::assert_required_proof_refuses(&dataset)?;
    let lineage = dataset.lineage.clone();
    let protected = state(&installed.store)?;
    source::originals(&installed.store, &installed.originals)?;
    drop(dataset);
    let source::Installed {
        directory,
        store,
        identity_writer,
        school,
        athlete,
        native_candidate,
        public,
        originals,
        ..
    } = installed;
    store.flush()?;
    drop(identity_writer);
    drop(store);
    let opened = crate::workbook::publication::open_publication(
        &workbook,
        &lineage,
        crate::report::Scope::AllSources,
        GradYear::CO2027,
    )?;
    output::verify(&opened.dataset, &workbook, &public, &athlete.id, &school.id)?;
    drop(opened);
    let (store, writer) =
        Store::open_public_identity_qualification_v3(directory.path().join("store"))?.into_parts();
    drop(writer);
    check!(eq; state(&store)?, protected);
    source::originals(&store, &originals)?;
    roster::require_manifest(&store)?;
    identity::separate(&store, &athlete, &native_candidate, &public)?;
    check!(eq; identity::linked(&store,&athlete,&result.0,&result.1)?,native_source);
    review::replay(&store, result.0, &result.1)?;
    review::replay(&store, status.0, &status.1)?;
    authority::require_replay(&store, &accepted)?;
    let objects = tempfile::tempdir()?;
    let dataset = controlled::selection::capture(&store, &profile, objects.path())?;
    output::verify(&dataset, &workbook, &public, &athlete.id, &school.id)?;
    check!(eq; state(&store)?, protected);
    Ok(())
}
