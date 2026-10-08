use anyhow::{ensure, Context};
use census_crawl::coach_directories;
use census_domain::model::{CanonicalSchool, SchoolPostalAddress, SourceNamespace};
use census_domain::school_directory::SourceLabel;
use census_domain::UsJurisdiction;
use census_report::export::ExportDataset;
use std::collections::BTreeSet;

use super::acquisition::{DIRECTORY, SUMMARY};
use super::artifacts::{sha256, Result};
use super::NAME;

pub(super) fn verify_stored(dataset: &ExportDataset, captured: &str) -> Result<()> {
    ensure!(
        dataset.schools.len() == 1
            && dataset.coaches.len() == 16
            && dataset.coach_observations.len() == 16
            && dataset.athletes.is_empty(),
        "stored population must be one school, 16 coaches and zero athletes"
    );
    let school = dataset
        .schools
        .values()
        .next()
        .context("stored school absent")?;
    ensure!(
        school.name == NAME
            && school.state == Some(UsJurisdiction::NorthCarolina)
            && school.postal_addresses.len() == 2,
        "stored school identity or postal count differs"
    );
    verify_coaches(dataset, school, captured)?;
    let expected = expected_claims(school, captured);
    school
        .postal_addresses
        .iter()
        .try_for_each(|claim| verify_claim(claim, school, captured, &expected))?;
    ensure!(
        school
            .postal_addresses
            .iter()
            .map(|claim| &claim.evidence().source.url)
            .collect::<BTreeSet<_>>()
            .len()
            == 2,
        "claims do not retain two distinct same-provider captures"
    );
    Ok(())
}

fn verify_coaches(dataset: &ExportDataset, school: &CanonicalSchool, captured: &str) -> Result<()> {
    ensure!(
        dataset
            .coaches
            .iter()
            .map(|coach| &coach.id)
            .collect::<BTreeSet<_>>()
            .len()
            == 16,
        "coach canonical IDs are not distinct"
    );
    let summary_url = coach_directories::summary_url("ZCUM49");
    ensure!(
        dataset.coaches.iter().all(|coach| coach.school == school.id
            && coach
                .evidence
                .iter()
                .any(|e| e.source.id == "coach_directories"
                    && e.source.url.as_deref() == Some(summary_url.as_str())
                    && e.observed_on == captured)),
        "coach linkage or summary provenance differs"
    );
    Ok(())
}

fn verify_claim(
    claim: &SchoolPostalAddress,
    school: &CanonicalSchool,
    captured: &str,
    expected: &[[String; 13]; 2],
) -> Result<()> {
    claim.belongs_to(school)?;
    let matched = expected
        .iter()
        .find(|fields| fields.get(9).map(String::as_str) == claim.evidence().source.url.as_deref())
        .context("unexpected claim source URL")?;
    ensure!(
        claim.owner().namespace == SourceNamespace::association_school("coach_directories")
            && claim.owner().id == "ZCUM49"
            && claim.evidence().source.id == "coach_directories"
            && claim.source_label()
                == &(SourceLabel::AthleticAssociation {
                    state: UsJurisdiction::NorthCarolina
                })
            && claim.evidence().observed_on == captured
            && Some(claim.capture_sha256()) == matched.get(11).map(String::as_str),
        "claim provenance differs"
    );
    ensure!(
        claim.address().line1().map(|line| line.as_str()) == Some("1 Rocket Drive")
            && claim.address().line2().is_none()
            && claim.address().city().map(|city| city.as_str()) == Some("Asheville")
            && claim.address().state() == Some(UsJurisdiction::NorthCarolina)
            && claim
                .address()
                .zip()
                .map_or_else(String::new, ToString::to_string)
                == *matched.get(5).context("ZIP expectation absent")?,
        "claim address differs from its source capture"
    );
    Ok(())
}

fn expected_claims(school: &CanonicalSchool, captured: &str) -> [[String; 13]; 2] {
    [
        (
            coach_directories::directory_page_url("NCHSAA", 1),
            "",
            DIRECTORY,
        ),
        (coach_directories::summary_url("ZCUM49"), "28803", SUMMARY),
    ]
    .map(|(url, zip, body)| {
        [
            school.id.to_string(),
            "1 Rocket Drive".into(),
            String::new(),
            "Asheville".into(),
            "NC".into(),
            zip.into(),
            "association_school:coach_directories".into(),
            "ZCUM49".into(),
            "athletic-association:NC".into(),
            url,
            captured.to_owned(),
            sha256(body),
            "unknown".into(),
        ]
    })
}

pub(super) fn verify_positions(
    fields: &[String; 13],
    school: &CanonicalSchool,
    captured: &str,
) -> Result<()> {
    ensure!(
        fields.iter().all(|field| field.split('\n').count() == 2),
        "postal fields do not all retain two claim positions"
    );
    let urls = fields.get(9).context("postal source URL column absent")?;
    expected_claims(school, captured)
        .iter()
        .try_for_each(|claim| -> Result<()> {
            let url = claim.get(9).context("expected source URL absent")?;
            let position = urls
                .split('\n')
                .position(|actual| actual == url)
                .context("capture URL absent in product")?;
            fields
                .iter()
                .zip(claim)
                .enumerate()
                .try_for_each(|(column, (actual, expected))| {
                    ensure!(
                        actual.split('\n').nth(position) == Some(expected.as_str()),
                        "postal column {column} misaligns claim position {position} for {url}"
                    );
                    Ok(())
                })
        })
}
