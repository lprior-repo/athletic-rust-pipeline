use super::TestResult;
use crate::restate_services::wire::NationalRequest;
use census_domain::model::{
    AttestationQualification, CanonicalAthlete, CanonicalSchool, Evidence, Gender, GradYear,
    IdentityAttestation, PublishedGraduation, SourceIdentity, SourceNamespace, SourceRef,
};
use census_domain::UsJurisdiction;
use census_store::clock::Clock;
use census_store::{Store, Table};
use sha2::{Digest, Sha256};

pub struct FixtureClock;

impl Clock for FixtureClock {
    fn today(&self) -> String {
        "2026-10-12".to_owned()
    }
    fn today_iso8601(&self) -> String {
        "2026-10-12T00:00:00Z".to_owned()
    }
    fn now(&self) -> tokio::time::Instant {
        tokio::time::Instant::now()
    }
}

pub fn request() -> TestResult<NationalRequest> {
    Ok(serde_json::from_value(serde_json::json!({
        "season": 2026, "revision": 1,
        "history": {"first_calendar_year": 2023, "last_calendar_year": 2026, "as_of": "2026-10-12"},
        "pass_budget": 0
    }))?)
}

pub fn seed(store: &Store) -> TestResult<Vec<CanonicalAthlete>> {
    let (school, _) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Fixture High",
        "fixture high",
        Some("Madison"),
    );
    let shared = SourceIdentity::new(SourceNamespace::TfrrsAthlete, "991114")
        .with_url("https://timer.test/athletes/991114");
    let mut first = CanonicalAthlete::new(
        &school.id,
        "Synthetic Runner",
        GradYear::CO2027,
        Gender::Girls,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "14399169")
            .with_url("https://roster.test/athletes/14399169"),
    );
    first.add_identity(shared.clone());
    let mut second = CanonicalAthlete::new(
        &school.id,
        "Synthetic Runner",
        GradYear::CO2027,
        Gender::Girls,
        shared.clone(),
    );
    bind(&mut first, &shared, "roster", "official_roster")?;
    bind(&mut second, &shared, "timer", "independent_timer")?;
    store.append(Table::Schools, &school)?;
    let rows = vec![first, second];
    store.append_many(Table::Athletes, &rows)?;
    Ok(rows)
}

pub fn seed_transfer(store: &Store) -> TestResult<Vec<CanonicalAthlete>> {
    let mut rows = seed(store)?;
    let (school, _) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Transfer High",
        "transfer high",
        Some("Verona"),
    );
    let owner = SourceIdentity::new(SourceNamespace::TfrrsAthlete, "991114")
        .with_url("https://transfer.test/athletes/991114");
    let mut transferred = CanonicalAthlete::new(
        &school.id,
        "Synthetic Runner",
        GradYear::CO2027,
        Gender::Girls,
        owner.clone(),
    );
    bind(&mut transferred, &owner, "transfer", "independent_timer")?;
    store.append(Table::Schools, &school)?;
    store.append(Table::Athletes, &transferred)?;
    rows.push(transferred);
    Ok(rows)
}

fn bind(
    athlete: &mut CanonicalAthlete,
    subject: &SourceIdentity,
    family: &str,
    producer: &str,
) -> TestResult {
    let url = format!("https://{family}.test/capture");
    let source = SourceRef::new(family, Some(url));
    let body = serde_json::to_vec(
        &serde_json::json!({"family": family, "producer": producer, "subject": subject}),
    )?;
    let claim = IdentityAttestation {
        subject: subject.clone(),
        source: source.clone(),
        capture_sha256: format!("{:x}", Sha256::digest(&body)),
        acquired_at: "2026-10-12T00:00:00Z".to_owned(),
        source_family: family.to_owned(),
        upstream_producer: producer.to_owned(),
        subject_locator: "subject".to_owned(),
        qualification: AttestationQualification::IndependentPublished,
    };
    claim.validate()?;
    athlete.published_graduations.push(PublishedGraduation {
        grad_year: GradYear::CO2027,
        source: source.clone(),
    });
    athlete.identity_attestations.push(claim);
    athlete
        .evidence
        .push(Evidence::parsed(source, "2026-10-12"));
    Ok(())
}
