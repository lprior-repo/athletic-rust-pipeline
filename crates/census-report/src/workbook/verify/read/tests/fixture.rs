use super::TestResult;
use crate::export::ExportDataset;
use crate::workbook::Options;
use census_domain::model::*;
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use std::path::{Path, PathBuf};

pub(in crate::workbook::verify) fn publication(
    root: &Path,
) -> TestResult<(PathBuf, ExportDataset, Options)> {
    let store = Store::open(root.join("store"))?;
    let (school, _) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Oracle High",
        "oracle high",
        Some("Madison"),
    );
    store.append(Table::Schools, &school)?;
    let (other, _) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Oracle High",
        "oracle high",
        Some("Milwaukee"),
    );
    store.append(Table::Schools, &other)?;
    let mut athlete = CanonicalAthlete::new(
        &school.id,
        "Oracle Runner",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "oracle-1"),
    );
    athlete.published_graduations.push(PublishedGraduation {
        grad_year: GradYear::CO2027,
        source: SourceRef::new(
            "synthetic_published_roster",
            Some("https://fixture.test/roster".into()),
        ),
    });
    athlete.sports.push(Sport::OutdoorTrack);
    store.append(Table::Athletes, &athlete)?;
    performance(&store, &athlete)?;
    coaches(&store, &school)?;
    store.replace(
        Table::ReviewCases,
        &ReviewCase::pending(
            SCHOOL_IDENTITY_FAMILY,
            school.id.as_str(),
            "Oracle High",
            "ambiguous school candidates",
        ),
    )?;
    let options = Options {
        grad_year: Some(2027),
        out: None,
        limit: None,
        scope: crate::report::Scope::AllSources,
        school_year: SchoolYear::new(2026).ok_or("fixture season")?,
    };
    let path = crate::workbook::build(&store, &options)?;
    let dataset = ExportDataset::reopen_frozen(
        &path.parent().ok_or("generation")?.join("frozen-input.json"),
    )?;
    crate::workbook::verify::verify_frozen(&path, &dataset, &options)?;
    crate::workbook::publication::verify_published(&path)?;
    crate::workbook::publication::verify_for_seal(
        &path,
        &dataset,
        options.scope,
        GradYear::CO2027,
    )?;
    Ok((path, dataset, options))
}

fn performance(store: &Store, athlete: &CanonicalAthlete) -> TestResult {
    let mut meet = CanonicalMeet::new(
        Some(UsJurisdiction::Wisconsin),
        "Oracle Invite",
        "2026-05-02",
        CompetitionLevel::Unknown,
    );
    meet.sports.push(Sport::OutdoorTrack);
    store.append(Table::Meets, &meet)?;
    let kind = EventKind::Track100m;
    let event = CanonicalEvent::new(
        EventIdentity {
            meet: &meet.id,
            kind,
            gender: Gender::Boys,
            division: None,
            round: None,
        },
        EventSpecification::default(),
    )?;
    store.append(Table::Events, &event)?;
    let team = Id::mint("team", &["oracle-team"]);
    let mut performance = CanonicalPerformance::new(
        PerformanceIdentity {
            athlete: &athlete.id,
            event: &event.id,
            meet: &meet.id,
            date: "2026-05-02",
            source_key: "oracle:1",
        },
        PerformanceResult {
            team: &team,
            mark: Mark::TimeSeconds(ExactSeconds::parse("11.00")?),
            wind_mps: Some(1.0),
            place: Some(1),
        },
    )?;
    performance.timing = Some(TimingMethod::Fat);
    performance.source_athlete = athlete.source.clone();
    store.append(Table::Performances, &performance)?;
    Ok(())
}

fn coaches(store: &Store, school: &CanonicalSchool) -> TestResult {
    for name in ["Alpha Coach", "Beta Coach"] {
        let mut coach = CanonicalCoach::new(
            &school.id,
            name,
            Some(Sport::OutdoorTrack),
            Gender::Boys,
            CoachRole::HeadCoach,
        );
        coach.professional_email = Some(format!("{}@fixture.test", name.replace(' ', "-")));
        coach.evidence.push(Evidence::parsed(
            SourceRef::new(
                "coach_contacts_csv",
                Some("https://fixture.test/staff".into()),
            ),
            "2026-09-20",
        ));
        store.append(Table::Coaches, &coach)?;
        if name == "Alpha Coach" {
            coach.professional_email = Some("second-observation@fixture.test".into());
            store.append(Table::Coaches, &coach)?;
        }
    }
    Ok(())
}
