use super::*;
use census_domain::model::{Evidence, SchoolPostalAddress};
use census_domain::school_directory::{CityName, PostalAddress, SourceLabel, StreetLine, ZipCode};
use sha2::{Digest, Sha256};

const RAW: &[u8] = include_bytes!(
    "../../../../../census-crawl/tests/fixtures/coach_directories/nc_staff_summary_zcum49.json"
);
const URL: &str = "https://maxinfosite-api-live.dragonflyathletics.com/schools/ZCUM49/summary";

fn school_from_capture() -> CanonicalSchool {
    let value: serde_json::Value = serde_json::from_slice(RAW).unwrap();
    assert_eq!(value["id"], "6285bff87f770402d0000006");
    assert_eq!(value["shortCode"], "ZCUM49");
    let state = value["stateCode"]
        .as_str()
        .unwrap()
        .parse::<UsJurisdiction>()
        .unwrap();
    let name = value["name"].as_str().unwrap();
    let (mut school, _) =
        CanonicalSchool::new(state, name, census_domain::model::normalize_name(name));
    let owner = SourceIdentity::new(
        SourceNamespace::association_school("nchsaa"),
        value["shortCode"].as_str().unwrap(),
    );
    school.source_identities.push(owner.clone());
    let address = &value["address"];
    let address = PostalAddress::of(
        Some(StreetLine::parse(address["address1"].as_str().unwrap()).unwrap()),
        None,
        Some(CityName::parse(address["city"].as_str().unwrap()).unwrap()),
        Some(state),
        Some(ZipCode::parse(address["zip"].as_str().unwrap()).unwrap()),
    )
    .unwrap();
    let claim = SchoolPostalAddress::new(
        address,
        owner,
        SourceLabel::AthleticAssociation { state },
        Evidence::parsed(SourceRef::new("nchsaa", Some(URL.into())), "2026-09-27"),
        format!("{:x}", Sha256::digest(RAW)),
    )
    .unwrap();
    school.add_postal_address(claim).unwrap();
    school
}

#[test]
fn captured_zcum49_postal_claim_publishes_on_school_metadata_and_recruiting_csv() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path().join("store")).unwrap();
    let school = school_from_capture();
    store.append(Table::Schools, &school).unwrap();
    let athlete = CanonicalAthlete::new(
        &school.id,
        "Capture Consumer",
        GradYear::CO2027,
        Gender::Girls,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "postal-capture-consumer"),
    );
    store.append(Table::Athletes, &athlete).unwrap();
    let data = dir.path().join("data");
    run_export_data(
        &store,
        &ExportDataArgs {
            data: data.clone(),
            school_year: Some(2026),
        },
    )
    .unwrap();
    for file in ["canonical-schools.csv", "recruiting-co2027.csv"] {
        let mut reader = ::csv::Reader::from_path(data.join(file)).unwrap();
        let headers = reader.headers().unwrap().clone();
        let record = reader.records().next().unwrap().unwrap();
        for (column, expected) in [
            ("postal_school_id", school.id.to_string()),
            ("postal_street", "1 Rocket Drive".into()),
            ("postal_second_line", "".into()),
            ("postal_city", "Asheville".into()),
            ("postal_state", "NC".into()),
            ("postal_zip", "28803".into()),
            ("postal_owner_namespace", "association_school:nchsaa".into()),
            ("postal_owner_id", "ZCUM49".into()),
            ("postal_source", "athletic-association:NC".into()),
            ("postal_source_url", URL.into()),
            ("postal_observed_date", "2026-09-27".into()),
            (
                "postal_capture_sha256",
                format!("{:x}", Sha256::digest(RAW)),
            ),
        ] {
            let index = headers.iter().position(|field| field == column).unwrap();
            assert_eq!(
                record.get(index),
                Some(expected.as_str()),
                "{file} {column}"
            );
        }
    }
}

#[test]
fn foreign_postal_claim_refuses_school_metadata_export_instead_of_publishing_a_fact() {
    let dir = tempfile::tempdir().unwrap();
    let mut school = school_from_capture();
    school.source_identities.clear();
    let error = schools::write_canonical_schools(&[school], dir.path())
        .unwrap_err()
        .to_string();
    assert!(error.contains("requires review"), "{error}");
    assert_eq!(dir.path().join("canonical-schools.csv").exists(), false);
}
