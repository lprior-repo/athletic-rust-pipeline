use super::*;
use census_domain::model::{Evidence, SchoolPostalAddress};
use census_domain::school_directory::{CityName, PostalAddress, SourceLabel, StreetLine, ZipCode};
use sha2::{Digest, Sha256};

const RAW: &[u8] = include_bytes!(
    "../../../../../census-crawl/tests/fixtures/coach_directories/nc_staff_summary_zcum49.json"
);
const URL: &str = "https://maxinfosite-api-live.dragonflyathletics.com/schools/ZCUM49/summary";

fn school_from_capture() -> TestResult<CanonicalSchool> {
    let value: serde_json::Value = serde_json::from_slice(RAW)?;
    check!(eq; value["id"], "6285bff87f770402d0000006");
    check!(eq; value["shortCode"], "ZCUM49");
    let state = value["stateCode"]
        .as_str()
        .ok_or("missing capture state")?
        .parse::<UsJurisdiction>()?;
    let name = value["name"]
        .as_str()
        .ok_or("missing capture school name")?;
    let (mut school, _) =
        CanonicalSchool::new(state, name, census_domain::model::normalize_name(name));
    let owner = SourceIdentity::new(
        SourceNamespace::association_school("nchsaa"),
        value["shortCode"]
            .as_str()
            .ok_or("missing capture owner id")?,
    );
    school.source_identities.push(owner.clone());
    let address = &value["address"];
    let address = PostalAddress::of(
        Some(StreetLine::parse(
            address["address1"]
                .as_str()
                .ok_or("missing capture street")?,
        )?),
        None,
        Some(CityName::parse(
            address["city"].as_str().ok_or("missing capture city")?,
        )?),
        Some(state),
        Some(ZipCode::parse(
            address["zip"].as_str().ok_or("missing capture zip")?,
        )?),
    )
    .ok_or("capture address is empty")?;
    let claim = SchoolPostalAddress::new(
        address,
        owner,
        SourceLabel::AthleticAssociation { state },
        Evidence::parsed(SourceRef::new("nchsaa", Some(URL.into())), "2026-09-27"),
        format!("{:x}", Sha256::digest(RAW)),
    )?;
    school.add_postal_address(claim)?;
    Ok(school)
}

#[test]
fn captured_zcum49_postal_claim_publishes_on_school_metadata_and_recruiting_csv() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path().join("store"))?;
    let school = school_from_capture()?;
    store.append(Table::Schools, &school)?;
    let mut athlete = CanonicalAthlete::new(
        &school.id,
        "Capture Consumer",
        GradYear::CO2027,
        Gender::Girls,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "postal-capture-consumer"),
    );
    athlete
        .published_graduations
        .push(census_domain::model::PublishedGraduation {
            grad_year: GradYear::CO2027,
            source: SourceRef::id("postal_capture_fixture"),
        });
    store.append(Table::Athletes, &athlete)?;
    let data = dir.path().join("data");
    run_export_data(
        &store,
        &ExportDataArgs {
            data: data.clone(),
            school_year: Some(2026),
        },
    )?;
    for file in ["canonical-schools.csv", "recruiting-co2027.csv"] {
        let mut reader = ::csv::Reader::from_path(data.join(file))?;
        let headers = reader.headers()?.clone();
        let record = reader.records().next().ok_or("missing exported row")??;
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
            let index = headers
                .iter()
                .position(|field| field == column)
                .ok_or_else(|| format!("missing postal column {column}"))?;
            check!(eq;
                record.get(index),
                Some(expected.as_str()),
                "{file} {column}"
            );
        }
    }
    Ok(())
}

#[test]
fn foreign_postal_claim_refuses_school_metadata_export_instead_of_publishing_a_fact() -> TestResult
{
    let dir = tempfile::tempdir()?;
    let mut school = school_from_capture()?;
    school.source_identities.clear();
    let error = match schools::write_canonical_schools(&[school], dir.path()) {
        Err(error) => error.to_string(),
        Ok(_) => return Err("foreign postal claim published".into()),
    };
    check!(error.contains("requires review"), "{error}");
    check!(eq; dir.path().join("canonical-schools.csv").exists(), false);
    Ok(())
}
