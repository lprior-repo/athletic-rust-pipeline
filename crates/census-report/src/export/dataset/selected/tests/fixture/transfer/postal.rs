use super::super::roster::{hash, retain};
use super::super::{Store, TestResult, UsJurisdiction};
use census_crawl::net::FetchOutcome;
use census_domain::model::{
    CanonicalSchool, Evidence, SchoolPostalAddress, SourceIdentity, SourceNamespace, SourceRef,
};
use census_domain::school_directory::{CityName, PostalAddress, SourceLabel, StreetLine, ZipCode};
use serde::Deserialize;

#[derive(Deserialize)]
struct SyntheticAddress {
    school: String,
    street: String,
    city: String,
    state: String,
    zip: String,
}

pub(super) fn attach(store: &Store, school: &mut CanonicalSchool) -> TestResult {
    let body = br#"{"synthetic_fixture":true,"school":"Synthetic Transfer High","street":"123 Synthetic Transfer Lane","city":"Madison","state":"WI","zip":"53703"}"#.to_vec();
    let captured: SyntheticAddress = serde_json::from_slice(&body)?;
    check!(eq; captured.school, school.name);
    check!(eq; UsJurisdiction::parse(&captured.state), school.state);
    let capture = FetchOutcome {
        url: "https://synthetic-association.test/schools/transfer".into(),
        response_url: None,
        method: "GET".into(),
        status: 200,
        content_digest: hash(&body),
        bytes: body.len(),
        fetched_at: "2026-10-01T23:49:00Z".into(),
        from_cache: false,
        content_type: Some("application/json".into()),
        body,
    };
    let owner = SourceIdentity::new(
        SourceNamespace::association_school("synthetic-transfer-association"),
        "transfer",
    );
    school.source_identities.push(owner.clone());
    let address = PostalAddress::of(
        Some(StreetLine::parse(&captured.street)?),
        None,
        Some(CityName::parse(&captured.city)?),
        Some(UsJurisdiction::Wisconsin),
        Some(ZipCode::parse(&captured.zip)?),
    )
    .ok_or("synthetic captured address")?;
    school.add_postal_address(SchoolPostalAddress::new(
        address,
        owner,
        SourceLabel::AthleticAssociation {
            state: UsJurisdiction::Wisconsin,
        },
        Evidence::parsed(
            SourceRef::new("synthetic-transfer-association", Some(capture.url.clone())),
            &capture.fetched_at,
        ),
        capture.content_digest.clone(),
    )?)?;
    retain(store, &capture)?;
    Ok(())
}
