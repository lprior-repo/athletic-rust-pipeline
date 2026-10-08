use super::*;

#[path = "corroboration/captures.rs"]
mod captures;
#[path = "corroboration/cen14.rs"]
mod cen14;

use captures::{admits, captured_claim, independent_pair};

fn milesplit(id: &str) -> SourceIdentity {
    SourceIdentity::new(SourceNamespace::MilesplitAthlete, id)
        .with_url(format!("https://milesplit.test/{id}"))
}

fn tfrrs(id: &str) -> SourceIdentity {
    SourceIdentity::new(SourceNamespace::TfrrsAthlete, id)
        .with_url(format!("https://tfrrs.test/{id}"))
}

fn observed(school: &str, source: SourceIdentity, links: &[SourceIdentity]) -> CanonicalAthlete {
    let school = SchoolId::mint("sch", &["WI", school]);
    let mut athlete = CanonicalAthlete::new(
        &school,
        "Synthetic Runner",
        GradYear::CO2027,
        Gender::Girls,
        source,
    );
    athlete.evidence.push(Evidence::parsed(
        SourceRef::new("fixture", Some(format!("https://fixture.test/{school}"))),
        "2026-10-07T12:00:00Z",
    ));
    for link in links {
        athlete.add_identity(link.clone());
    }
    athlete
}

#[test]
fn linked_object_corroborates_when_attested_independently() -> Result<(), Box<dyn std::error::Error>>
{
    check!(admits(&independent_pair()?)?);
    Ok(())
}

#[test]
fn a_link_shared_pair_without_attestation_stays_review() -> Result<(), Box<dyn std::error::Error>> {
    let mut rows = independent_pair()?;
    for row in &mut rows {
        row.identity_attestations.clear();
    }
    check!(!admits(&rows)?);
    rows.reverse();
    check!(!admits(&rows)?);
    Ok(())
}

#[test]
fn two_ids_from_one_provider_withhold() -> Result<(), Box<dyn std::error::Error>> {
    let [first, owner] = independent_pair()?;
    let mut second = observed("School C", milesplit("222"), &[tfrrs("999")]);
    second.identity_attestations.push(captured_claim(
        &tfrrs("999"),
        "other-roster-fixture",
        "other-school-fixture",
    )?);
    check!(!admits(&[first, second, owner])?);
    Ok(())
}

#[test]
fn an_unparsed_member_withholds_corroboration() -> Result<(), Box<dyn std::error::Error>> {
    let [linked, mut owner] = independent_pair()?;
    for evidence in &mut owner.evidence {
        evidence.method = EvidenceMethod::Fetched;
    }
    check!(!admits(&[linked, owner])?);
    Ok(())
}

#[test]
fn a_contradictory_third_member_withholds() -> Result<(), Box<dyn std::error::Error>> {
    let rows = independent_pair()?;
    check!(admits(&rows)?);
    let [linked, owner] = rows;
    let mut third = observed("School C", tfrrs("777"), &[]);
    third.gender = Gender::Boys;
    check!(!admits(&[linked, owner, third])?);
    Ok(())
}

#[test]
fn name_school_and_cohort_agreement_alone_is_not_positive_evidence(
) -> Result<(), Box<dyn std::error::Error>> {
    check!(!admits(&[
        observed("School A", milesplit("111"), &[]),
        observed("School A", milesplit("222"), &[]),
    ])?);
    Ok(())
}
