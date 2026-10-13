use super::publication_tests::journal::Journal;
use super::NationalCensus;
use census_domain::model::{
    AppliedAthleteIdentity, CanonicalAthlete, IdentityStatus, ReviewCase, ReviewState,
    ReviewVerdictRecord,
};
use census_report::export::ExportDataset;
use census_report::report::{Derivation, Scope};
use census_store::{Store, Table};
use restate_sdk::prelude::*;
use std::sync::Arc;
use std::time::Duration;

mod deterministic;
mod fixtures;
mod transfers;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[tokio::test]
async fn b1ui_national_applies_source_identity_before_publication() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Arc::new(Store::open(directory.path())?);
    let rows = fixtures::seed(&store)?;
    let unrelated = retained_case("unrelated");
    store.replace_many(Table::ReviewCases, std::slice::from_ref(&unrelated))?;
    drive(Arc::clone(&store)).await?;
    let projection = store.athlete_identity_projection()?;
    let first = rows.first().ok_or("fixture athlete missing")?;
    check!(eq; projection.status(first.id.as_str())?, IdentityStatus::Verified);
    check!(!store
        .scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?
        .is_empty());
    let retained = store.scan::<ReviewCase>(Table::ReviewCases)?;
    check!(eq; retained.iter().filter(|case| case.id == unrelated.id && case.state == ReviewState::Retained).count(), 1);
    Ok(())
}

#[tokio::test]
async fn r2ox_obsolete_retained_case_is_superseded_when_gender_becomes_compatible() -> TestResult {
    r2ox_supersession(false).await?;
    r2ox_supersession(true).await
}

async fn r2ox_supersession(reversed: bool) -> TestResult {
    use census_domain::model::{
        Evidence, Gender, GradYear, SourceIdentity, SourceNamespace, SourceRef,
    };
    use census_domain::UsJurisdiction;

    let directory = tempfile::tempdir()?;
    let store = Arc::new(Store::open(directory.path())?);

    let (school, _) = census_domain::model::CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Superior High",
        "superior high",
        Some("Superior"),
    );
    let shared = SourceIdentity::new(SourceNamespace::TfrrsAthlete, "551234")
        .with_url("https://timer.test/athletes/551234");

    let mut athletes = vec![
        census_domain::model::CanonicalAthlete::new(
            &school.id,
            "Alex Morgan",
            GradYear::CO2027,
            Gender::Girls,
            shared.clone(),
        ),
        census_domain::model::CanonicalAthlete::new(
            &school.id,
            "Alex Morgan",
            GradYear::CO2027,
            Gender::Unknown,
            shared.clone(),
        ),
    ];
    for athlete in &mut athletes {
        athlete.evidence.push(Evidence::parsed(
            SourceRef::new(
                shared.id.clone(),
                Some("https://timer.test/athletes/551234".to_string()),
            ),
            "2026-04-01",
        ));
    }
    if reversed {
        athletes.reverse();
    }
    let subject = athletes.first().ok_or("subject athlete missing")?.clone();
    let other = athletes.get(1).ok_or("second athlete missing")?.clone();

    store.append(Table::Schools, &school)?;
    store.append_many(Table::Athletes, &athletes)?;

    let mut old_case = ReviewCase::minted(
        "Athlete identity",
        subject.id.as_str(),
        "Alex Morgan",
        "The rows disagree on name, class or gender, so the object does not settle that they are one athlete.",
    );
    let (_, digest) = old_case
        .id
        .rsplit_once(':')
        .ok_or("case id has no digest")?;
    old_case.id = format!("Athlete identity:{}:p1:{digest}", subject.id.as_str());
    old_case.state = ReviewState::Retained;
    old_case.member_ids = vec![subject.id.cast(), other.id.cast()];
    if reversed {
        old_case.member_ids.reverse();
    }
    let old_case_id = old_case.id.clone();
    store.replace_many(Table::ReviewCases, std::slice::from_ref(&old_case))?;

    drive(Arc::clone(&store)).await?;

    for pass in 0..2 {
        drive(Arc::clone(&store)).await?;

        let projection = store.athlete_identity_projection()?;
        check!(eq; projection.status(subject.id.as_str())?, IdentityStatus::Verified);
        check!(eq; projection.status(other.id.as_str())?, IdentityStatus::Verified);

        let cases = store.scan::<ReviewCase>(Table::ReviewCases)?;
        let superseded = cases
            .iter()
            .find(|case| case.id == old_case_id)
            .ok_or("the obsolete retained case vanished from the published set")?;
        check!(
            eq; superseded.state, ReviewState::Superseded,
            "pass {pass}: the obsolete retained case must be superseded, not left evidence-current"
        );

        let provenance = store
            .journal_payload("identity_retained_supersession_v1", &old_case_id)?
            .ok_or("supersession provenance missing")?;
        let new_id = provenance
            .get(0)
            .and_then(serde_json::Value::as_str)
            .ok_or("provenance must record the new case id")?;
        let resolved = cases
            .iter()
            .find(|case| case.id == new_id)
            .ok_or("the resolved successor case is missing")?;
        check!(eq; resolved.state, ReviewState::Resolved);
        check!(eq; resolved.member_ids.len(), 2);
        let verdicts = store.scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)?;
        check!(!verdicts.iter().any(|row| row.case_id == old_case_id));
        let decisions = store.scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?;
        check!(decisions.iter().any(|row| {
            row.case_id.as_deref() == Some(new_id)
                && row.members.len() == 2
                && row
                    .members
                    .iter()
                    .all(|member| resolved.member_ids.contains(&member.subject))
        }));
    }

    Ok(())
}

#[tokio::test]
async fn q3pt_boys_girls_still_blocks_identity_resolution() -> TestResult {
    use census_domain::model::{Gender, GradYear, SourceIdentity, SourceNamespace};
    use census_domain::UsJurisdiction;

    let directory = tempfile::tempdir()?;
    let store = Arc::new(Store::open(directory.path())?);

    let (school, _) = census_domain::model::CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Superior High",
        "superior high",
        Some("Superior"),
    );
    let shared = SourceIdentity::new(SourceNamespace::TfrrsAthlete, "551235")
        .with_url("https://timer.test/athletes/551235");

    let boys = census_domain::model::CanonicalAthlete::new(
        &school.id,
        "Alex Morgan",
        GradYear::CO2027,
        Gender::Boys,
        shared.clone(),
    );
    let girls = census_domain::model::CanonicalAthlete::new(
        &school.id,
        "Alex Morgan",
        GradYear::CO2027,
        Gender::Girls,
        shared.clone(),
    );

    store.append(Table::Schools, &school)?;
    store.append_many(Table::Athletes, &[boys.clone(), girls.clone()])?;

    drive(Arc::clone(&store)).await?;

    let projection = store.athlete_identity_projection()?;
    check!(
        matches!(
            projection.status(boys.id.as_str())?,
            IdentityStatus::RetainedConflict | IdentityStatus::Pending
        ),
        "boys vs girls must still block identity resolution"
    );
    let decisions = store.scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?;
    let boys_girls_accepted = decisions.iter().any(|decision| {
        decision
            .members
            .iter()
            .any(|m| m.subject.as_str() == boys.id.as_str())
            && decision
                .members
                .iter()
                .any(|m| m.subject.as_str() == girls.id.as_str())
    });
    check!(
        !boys_girls_accepted,
        "boys vs girls must not be accepted as same person"
    );

    Ok(())
}

#[tokio::test]
async fn u4al_national_records_review_verdicts_without_manual_review() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Arc::new(Store::open(directory.path())?);
    let athletes = fixtures::seed(&store)?;
    let generic: Vec<_> = athletes
        .iter()
        .map(|athlete| {
            ReviewCase::minted(
                "Athlete identity",
                athlete.id.as_str(),
                &athlete.canonical_name,
                "unresolved generic identity obligation",
            )
        })
        .collect();
    check!(eq; generic.len(), 2);
    store.replace_many(Table::ReviewCases, &generic)?;
    drive(Arc::clone(&store)).await?;
    let verdicts = store.scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)?;
    check!(eq; verdicts.iter().filter(|row| row.accepted && row.value == "same_person").count(), 1);
    check!(eq; verdicts.iter().filter(|row| !row.accepted && row.kind == "insufficient_evidence").count(), 0);
    let cases = store.scan::<ReviewCase>(Table::ReviewCases)?;
    check!(cases.iter().any(|case| case.state == ReviewState::Resolved));
    check!(eq; cases.iter().filter(|case| case.family == "Athlete identity" && case.member_ids.is_empty() && case.state == ReviewState::Superseded).count(), 2);
    for case in cases.iter().filter(|case| {
        case.family == "Athlete identity"
            && case.member_ids.is_empty()
            && case.state == ReviewState::Superseded
    }) {
        let reason = store
            .journal_payload("identity_generic_supersession_v1", &case.id)?
            .ok_or("supersession reason missing")?;
        check!(eq; reason.get(2).and_then(serde_json::Value::as_str), Some("bound identity applied; generic advice superseded"));
    }
    Ok(())
}

#[tokio::test]
async fn st7m_durable_cross_source_identity_publishes_one_athlete_with_both_keys() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Arc::new(Store::open(directory.path())?);
    let rows = fixtures::seed(&store)?;
    drive(Arc::clone(&store)).await?;
    let dataset = ExportDataset::load(&store)?;
    let derivation = Derivation::of(&dataset, Scope::AllSources, Some(2027));
    check!(eq; derivation.athletes().len(), 1);
    let athlete = derivation
        .athletes()
        .first()
        .ok_or("published athlete missing")?;
    let mut namespaces: Vec<_> = athlete
        .identities()
        .map(|key| key.namespace.to_string())
        .collect();
    namespaces.sort();
    check!(eq; namespaces, vec!["milesplit_athlete", "tfrrs_athlete"]);
    let projection = store.athlete_identity_projection()?;
    for row in &rows {
        check!(eq; projection.canonical_id(row.id.as_str()), athlete.id.as_str());
        check!(eq; projection.status(row.id.as_str())?, IdentityStatus::Verified);
    }
    check!(eq; store.scan::<CanonicalAthlete>(Table::Athletes)?.len(), 2);
    let path = directory.path().join("cross-source-identity.json");
    let artifact = serde_json::json!({
        "athletes": derivation.athletes(), "status": "verified",
        "review_cases": dataset.review_cases, "verdicts": dataset.verdicts,
        "identity_decisions": dataset.identity_decisions,
    });
    std::fs::write(&path, serde_json::to_vec(&artifact)?)?;
    check!(eq; serde_json::from_slice::<serde_json::Value>(&std::fs::read(&path)?)?, artifact);
    println!("st7m artifact={}", artifact);
    Ok(())
}

#[tokio::test]
async fn u4al_national_visits_all_review_checkpoints_and_replays_without_new_jobs() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Arc::new(Store::open(directory.path())?);
    fixtures::seed(&store)?;
    let cases: Vec<_> = (0..258)
        .map(|index| retained_case(&format!("unrelated:{index:03}")))
        .collect();
    store.replace_many(Table::ReviewCases, &cases)?;
    let journal = drive(Arc::clone(&store)).await?;
    let mut names = Vec::new();
    for entry in journal.entries.iter().filter(|entry| entry.kind == 0x0411) {
        let Some(name) = super::publication_tests::codec::field(&entry.payload, 12)? else {
            continue;
        };
        names.push(std::str::from_utf8(name)?.to_owned());
    }
    check!(names.contains(&"identity:advice:0".to_owned()));
    check!(names.contains(&"identity:advice:1".to_owned()));
    let stages: Vec<_> = names
        .iter()
        .filter(|name| name.starts_with("identity:"))
        .map(String::as_str)
        .collect();
    check!(eq; stages, vec!["identity:rules", "identity:advice:0", "identity:advice:1", "identity:advice:2", "identity:reconcile"]);
    let mut cursors = Vec::new();
    for entry in journal.entries.iter().filter(|entry| entry.kind == 0x0403) {
        if entry.text(1)? == "identity:advice" {
            let value = super::publication_tests::codec::field(entry.bytes(3)?, 1)?
                .ok_or("advice state value missing")?;
            let (_, cursor): (String, Option<String>) = serde_json::from_slice(value)?;
            cursors.push(cursor);
        }
    }
    check!(matches!(cursors.as_slice(), [Some(first), Some(second), None] if second > first));
    let verdicts = store.scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)?;
    for case in &cases {
        check!(verdicts
            .iter()
            .any(|verdict| verdict.case_id == case.id && !verdict.accepted));
    }
    Ok(())
}

fn retained_case(subject: &str) -> ReviewCase {
    let mut case = ReviewCase::minted(
        "Athlete identity",
        subject,
        "Unrelated athlete",
        "unresolved independent case",
    );
    case.state = ReviewState::Retained;
    case
}

async fn drive(store: Arc<Store>) -> TestResult<Journal> {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
    let url = format!(
        "http://{}/invoke/NationalCensus/run",
        listener.local_addr()?
    );
    let region = Arc::new(crate::spawn::Spawner::new());
    let jobs = crate::restate_services::Jobs::new(Arc::clone(&store), Arc::clone(&region));
    let mut national = NationalCensus::new(Arc::new(fixtures::FixtureClock), jobs);
    national.review_lanes = Ok([
        census_review::ModelClient::new(
            census_review::ModelOptions::local("http://127.0.0.1:1", "fixture")?
                .with_timeout(Duration::from_millis(100)),
        )?,
        census_review::ModelClient::new(
            census_review::ModelOptions::local("http://127.0.0.1:2", "fixture")?
                .with_timeout(Duration::from_millis(100)),
        )?,
    ]);
    let endpoint = Endpoint::builder().bind(national).build();
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
    let server = HttpServer::new(endpoint).serve_with_cancel(listener, async move {
        match stop_rx.await {
            Ok(()) | Err(_) => {}
        }
    });
    let assertions = async move {
        let result = tokio::time::timeout(Duration::from_secs(30), async {
            let client = reqwest::Client::builder()
                .http2_prior_knowledge()
                .timeout(Duration::from_secs(5))
                .build()?;
            let mut journal = Journal::new(&fixtures::request()?)?;
            journal.hold_stage = Some("Workbook".to_owned());
            journal.drive(&client, &url).await?;
            check!(journal.held.is_some());
            let entries = journal.entries.clone();
            let cases = store.scan::<ReviewCase>(Table::ReviewCases)?;
            let verdicts = store.scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)?;
            let decisions = store.scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?;
            let receipts = store.receipt_count()?;
            let generation = store.evidence_generation();
            let coverage = store.scan::<census_domain::model::CoverageRow>(Table::Coverage)?;
            let sweeps: Vec<_> = coverage
                .iter()
                .filter(|row| {
                    row.scope == census_domain::model::CoverageScope::Jurisdiction
                        && census_domain::UsJurisdiction::CENSUS_SCOPE
                            .iter()
                            .any(|jurisdiction| jurisdiction.code() == row.subject)
                })
                .collect();
            check!(eq; sweeps.len(), 49);
            for row in sweeps {
                check!(eq; row.metrics.len(), 3);
                check!(eq; row.metrics.get("registered"), Some(&1));
                check!(eq; row.metrics.get("applicable"), Some(&1));
                check!(eq; row.metrics.get("attempted"), Some(&1));
            }
            let replay = journal.step(&client, &url).await?;
            check!(!replay.iter().any(|entry| entry.kind == 0x0411));
            check!(eq; journal.entries, entries);
            check!(eq; store.scan::<ReviewCase>(Table::ReviewCases)?, cases);
            check!(eq; store.scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)?, verdicts);
            check!(eq; store.scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?, decisions);
            check!(eq; store.receipt_count()?, receipts);
            check!(eq; store.evidence_generation(), generation);
            check!(eq; store.scan::<census_domain::model::CoverageRow>(Table::Coverage)?, coverage);
            journal.release()?;
            journal.drive(&client, &url).await?;
            check!(journal.output.is_some());
            drop(store);
            Ok::<_, Box<dyn std::error::Error>>(journal)
        })
        .await;
        stop_tx
            .send(())
            .map_err(|_| "fixture endpoint lost its shutdown owner")?;
        result?
    };
    let ((), result) = tokio::join!(server, assertions);
    region.drain(Duration::from_secs(5)).await?;
    result
}
