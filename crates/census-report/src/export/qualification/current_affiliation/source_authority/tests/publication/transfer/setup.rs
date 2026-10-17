use super::*;
use census_crawl::net::FetchOutcome;
use census_domain::school_directory::{CityName, PostalAddress, SourceLabel, StreetLine, ZipCode};

pub(super) struct Transfer {
    pub fixture: fixture::Fixture,
    pub old: fixture::Capture,
    pub current: fixture::Capture,
    pub address_digest: String,
    pub case: Case,
}

pub(super) fn prepare(case: Case) -> TestResult<Transfer> {
    let fixture = fixture::Fixture::new()?;
    let old_at = if matches!(case, Case::Conflict) {
        fixture::AT
    } else {
        "2026-10-15T00:49:43Z"
    };
    let old = fixture::install_dated(
        &fixture,
        "12439",
        "42",
        Sport::OutdoorTrack,
        true,
        UsJurisdiction::Wisconsin,
        old_at,
    )?;
    fixture.accept(&old.athlete.id)?;
    let mut current = fixture::install_dated(
        &fixture,
        "12440",
        "42",
        Sport::IndoorTrack,
        !matches!(case, Case::Unknown),
        UsJurisdiction::Michigan,
        fixture::AT,
    )?;
    let address_digest = postal(&fixture.store, &mut current.school)?;
    fixture.store.append(Table::Schools, &current.school)?;
    super::super::super::history::append_performance(&fixture.store, &old.athlete, &old.school.id)?;
    crate::export::qualification::current_affiliation::tests::identity::accept_pair(
        &fixture.store,
        &fixture.writer,
        &old.athlete.id,
        &current.athlete.id,
    )?;
    issuance::review(
        &fixture,
        &old,
        "controlled-transfer-old-reviewed-occurrence",
    )
    .map_err(|error| format!("old roster review: {error}"))?;
    issuance::review(
        &fixture,
        &current,
        "controlled-transfer-current-reviewed-occurrence",
    )
    .map_err(|error| format!("current roster review: {error}"))?;
    Ok(Transfer {
        fixture,
        old,
        current,
        address_digest,
        case,
    })
}

fn postal(store: &Store, school: &mut CanonicalSchool) -> TestResult<String> {
    let body = serde_json::to_vec(
        &json!({"controlled_fixture":true,"school":school.name,"street":"123 Controlled Transfer Lane","city":"Novi","state":"MI","zip":"48375"}),
    )?;
    let digest = crate::export::qualification::current_affiliation::source_authority::hash(&body);
    let url = "https://controlled-association.test/schools/current-transfer";
    let source = SourceIdentity::new(
        SourceNamespace::association_school("controlled-transfer-association"),
        "current-transfer",
    );
    let address = PostalAddress::of(
        Some(StreetLine::parse("123 Controlled Transfer Lane")?),
        None,
        Some(CityName::parse("Novi")?),
        Some(UsJurisdiction::Michigan),
        Some(ZipCode::parse("48375")?),
    )
    .ok_or("controlled postal address")?;
    school.source_identities.push(source.clone());
    school.add_postal_address(SchoolPostalAddress::new(
        address,
        source,
        SourceLabel::AthleticAssociation {
            state: UsJurisdiction::Michigan,
        },
        Evidence::parsed(
            SourceRef::new("controlled-transfer-association", Some(url.into())),
            fixture::AT,
        ),
        digest.clone(),
    )?)?;
    let capture = FetchOutcome {
        url: url.into(),
        response_url: None,
        method: "GET".into(),
        status: 200,
        content_digest: digest.clone(),
        bytes: body.len(),
        fetched_at: fixture::AT.into(),
        from_cache: false,
        content_type: Some("application/json".into()),
        body,
    };
    crate::export::dataset::selected::tests::fixture::retain(store, &capture)?;
    Ok(digest)
}
