use super::{IncludedFixture, TestResult};
use crate::export::{ExportDataset, ExportJobRequest};
use census_domain::model::{
    CanonicalAthlete, CanonicalSchool, Gender, GradYear, ProfileSelectionV1, Sport,
};
use census_domain::UsJurisdiction;
use census_store::{RetainedCaptureRootSchemaV1, Table};

#[test]
fn selected_job_ignores_malformed_optional_prs_for_existing_out_of_scope_athletes() -> TestResult {
    for case in ["year", "category", "state", "sport"] {
        let fixture = IncludedFixture::new()?;
        let foreign = foreign(&fixture, case)?;
        fixture.store.append(Table::Athletes, &foreign)?;
        let packet = crate::export::profile_pr_test_source::packet_with_profile(
            "9510",
            "2026-09-30T00:50:27Z",
            |html| Ok(html.replacen("id=\"prTable\"", "id=\"wrongTable\"", 1)),
        )?;
        check!(packet.read_profile_prs_if_published().is_err());
        fixture.store.journal_done(
            RetainedCaptureRootSchemaV1::ProviderTargetPacketV1.phase(),
            packet.reference()?.as_str(),
            &packet,
        )?;
        let valid = crate::export::profile_pr_test_source::packet("9501", "2026-09-30T00:50:27Z")?;
        fixture.store.journal_done(
            RetainedCaptureRootSchemaV1::ProviderTargetPacketV1.phase(),
            valid.reference()?.as_str(),
            &valid,
        )?;
        let selection = ProfileSelectionV1::new(
            fixture.store.store_identity()?,
            fixture
                .dataset
                .selected_profile_id()
                .ok_or("profile")?
                .clone(),
        );
        let selected = ExportDataset::for_job_request(
            &fixture.store,
            &format!("foreign-profile-{case}"),
            &ExportJobRequest::SelectedPublicationV1 { selection },
        )?;
        check!(eq; selected.reported_profile_prs_for(&fixture.first).count(), 1);
        check!(eq; selected.reported_profile_prs_for(&foreign).count(), 0);
    }
    Ok(())
}

fn foreign(fixture: &IncludedFixture, case: &str) -> TestResult<CanonicalAthlete> {
    let mut foreign =
        super::support::athlete(&fixture.first.school, "Outside selected scope", "9510")?;
    match case {
        "year" => {
            foreign.grad_year = GradYear::new(2028).ok_or("year")?;
            for source in &mut foreign.published_graduations {
                source.grad_year = foreign.grad_year;
            }
        }
        "category" => foreign.gender = Gender::Boys,
        "state" => {
            let (school, id) = CanonicalSchool::new(
                UsJurisdiction::Colorado,
                "Controlled Colorado High",
                "controlled colorado high",
                None,
            );
            fixture.store.append(Table::Schools, &school)?;
            foreign.school = id;
        }
        "sport" => foreign.sports = vec![Sport::IndoorTrack],
        _ => return Err("unknown scope case".into()),
    }
    Ok(foreign)
}

#[test]
fn selected_job_retains_profile_pr_for_an_athlete_scoped_through_a_performance() -> TestResult {
    let fixture = IncludedFixture::new()?;
    let mut foreign = foreign(&fixture, "state")?;
    foreign.sports = vec![Sport::IndoorTrack];
    fixture.store.append(Table::Athletes, &foreign)?;
    let original = &fixture.first_performance;
    let team = fixture
        .dataset
        .teams
        .get(&original.team)
        .ok_or("selected team")?;
    let event = fixture
        .dataset
        .events
        .iter()
        .find(|row| row.id == original.event)
        .ok_or("selected event")?;
    let meet = fixture
        .dataset
        .meets
        .iter()
        .find(|row| row.id == original.meet)
        .ok_or("selected meet")?;
    let performance = super::support::performance(
        &foreign,
        team,
        meet,
        event,
        "controlled-foreign-roster-selected-performance",
        "60.00",
    )?;
    fixture.store.append(Table::Performances, &performance)?;
    let packet = crate::export::profile_pr_test_source::packet("9510", "2026-09-30T00:50:27Z")?;
    fixture.store.journal_done(
        RetainedCaptureRootSchemaV1::ProviderTargetPacketV1.phase(),
        packet.reference()?.as_str(),
        &packet,
    )?;
    let selection = ProfileSelectionV1::new(
        fixture.store.store_identity()?,
        fixture
            .dataset
            .selected_profile_id()
            .ok_or("profile")?
            .clone(),
    );
    let selected = ExportDataset::for_job_request(
        &fixture.store,
        "performance-scoped-profile",
        &ExportJobRequest::SelectedPublicationV1 { selection },
    )?;
    check!(eq; selected.reported_profile_prs_for(&foreign).count(), 1);
    Ok(())
}
