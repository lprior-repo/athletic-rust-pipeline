use super::*;
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalSchool,
    CanonicalTeam, CentiSeconds, CompetitionLevel, EventKind, Evidence, Gender, GradYear, Mark,
    SchoolYear, SourceIdentity, SourceNamespace, SourceRef, Sport, TimingMethod,
};
use census_domain::UsJurisdiction;
use census_report::workbook::ProjectedValue;
use census_store::Table;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

type Fixture = (
    tempfile::TempDir,
    Store,
    Vec<Vec<String>>,
    HashMap<&'static str, usize>,
);

fn fixture() -> TestResult<Fixture> {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let (school, school_id) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Synthetic High",
        "synthetichigh",
        None,
    );
    store.append(Table::Schools, &school)?;
    let athlete = CanonicalAthlete::new(
        &school_id,
        "Synthetic Runner",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::athletic_net("athlete"), "101"),
    );
    store.append(Table::Athletes, &athlete)?;
    for date in ["2026-05-01", "2026-05-02"] {
        let meet = CanonicalMeet::new(
            Some(UsJurisdiction::Wisconsin),
            format!("Invitational {date}"),
            date,
            CompetitionLevel::Invitational,
        );
        store.append(Table::Meets, &meet)?;
        let event = CanonicalEvent::new(
            &meet.id,
            EventKind::Track100m,
            Gender::Boys,
            None,
            Some("Finals"),
        );
        store.append(Table::Events, &event)?;
        let source_key = format!("result:{date}:101");
        let performance = CanonicalPerformance {
            id: CanonicalPerformance::mint(
                &athlete.id,
                &meet.id,
                &EventKind::Track100m,
                date,
                &source_key,
            ),
            athlete: athlete.id.clone(),
            team: CanonicalTeam::mint(
                &school_id,
                Sport::OutdoorTrack,
                Gender::Boys,
                SchoolYear::new(2025).ok_or("supported school year")?,
            ),
            event: event.id,
            meet: meet.id,
            date: date.into(),
            mark: Mark::TimeSeconds(CentiSeconds::new(1086)),
            wind_mps: Some(0.4),
            place: Some(2),
            heat: None,
            round: None,
            timing: Some(TimingMethod::Fat),
            observed_grade: None,
            evidence: vec![Evidence::parsed(
                SourceRef::new(
                    "athleticnet",
                    Some(format!("https://example.invalid/{date}")),
                ),
                "2026-09-30",
            )],
            source_key,
            source_athlete: athlete.source.clone(),
            retained_conflicts: Vec::new(),
        };
        store.append(Table::Performances, &performance)?;
    }
    let dataset = ExportDataset::load(&store)?;
    let derivation = Derivation::of(&dataset, Scope::AllSources, None);
    let projection = PerformanceProjection::of(&derivation);
    let mut rows: Vec<Vec<String>> = derivation
        .performances()
        .iter()
        .map(|performance| {
            projection
                .row(performance)
                .values()
                .into_iter()
                .map(|value| match value {
                    ProjectedValue::Text(value) => value.to_owned(),
                    ProjectedValue::Number(value) => {
                        value.map_or_else(String::new, |number| number.to_string())
                    }
                })
                .collect()
        })
        .collect();
    rows.sort_by(|left, right| left.get(7).cmp(&right.get(7)));
    let columns = PERFORMANCES_REQUIRED
        .iter()
        .enumerate()
        .map(|(index, &name)| (name, index))
        .collect();
    Ok((dir, store, rows, columns))
}

#[test]
fn equal_marks_at_different_meets_cannot_be_substituted() -> TestResult {
    let (_dir, store, mut rows, columns) = fixture()?;
    let baseline = verify_performances(&store, &rows, &[0, 1], &columns, Scope::AllSources)
        .map_err(|error| error.message)?;
    check!(eq; baseline.passed, 2);
    check!(eq; rows[0][columns["Athlete ID"]], rows[1][columns["Athlete ID"]]);
    check!(eq; rows[0][columns["Event"]], rows[1][columns["Event"]]);
    check!(eq; rows[0][columns["Mark"]], rows[1][columns["Mark"]]);
    let wrong_meet = rows[1][columns["Meet ID"]].clone();
    rows[0][columns["Meet ID"]] = wrong_meet;
    let error = match verify_performances(&store, &rows, &[0], &columns, Scope::AllSources) {
        Err(error) => error,
        Ok(_) => return Err("substituted meet accepted".into()),
    };
    check!(error.message.contains("Meet ID"), "{}", error.message);
    Ok(())
}

#[test]
fn altered_conditions_dates_and_source_references_are_rejected() -> TestResult {
    let (_dir, store, rows, columns) = fixture()?;
    for (name, changed) in [
        ("Date", "2026-05-03"),
        ("Timing", "Hand"),
        ("Wind", "2.4"),
        ("Wind", "NaN"),
        ("Wind", ""),
        ("Round", "Prelims"),
        ("Source", "unrelated"),
        ("Source ResultID", "different-row"),
        ("Source URL", "https://example.invalid/unrelated"),
        ("Normalized Mark", "10.87"),
        ("Place", "3"),
        ("Graduation Year", "2028"),
    ] {
        let mut changed_rows = rows.clone();
        changed_rows[0][columns[name]] = changed.into();
        let error =
            match verify_performances(&store, &changed_rows, &[0], &columns, Scope::AllSources) {
                Err(error) => error,
                Ok(_) => return Err(format!("altered {name} accepted").into()),
            };
        check!(error.message.contains(name), "{name}: {}", error.message);
    }
    Ok(())
}

#[test]
fn duplicate_unsampled_result_cannot_hide_a_missing_result() -> TestResult {
    let (_dir, store, mut rows, columns) = fixture()?;
    rows[1] = rows[0].clone();
    let error = match verify_performances(&store, &rows, &[0], &columns, Scope::AllSources) {
        Err(error) => error,
        Ok(_) => return Err("duplicate result accepted".into()),
    };
    check!(
        error.message.contains("duplicate result ID"),
        "{}",
        error.message
    );
    Ok(())
}

#[test]
fn unknown_result_id_and_missing_numeric_cell_are_rejected() -> TestResult {
    let (_dir, store, rows, columns) = fixture()?;
    let mut unknown = rows.clone();
    unknown[0][columns["Canonical Result ID"]] = "missing-result".into();
    let error = match verify_performances(&store, &unknown, &[0], &columns, Scope::AllSources) {
        Err(error) => error,
        Ok(_) => return Err("unknown result accepted".into()),
    };
    check!(error.message.contains("absent"), "{}", error.message);
    let mut truncated = rows;
    truncated[0].truncate(columns["Normalized Mark"]);
    let error = match verify_performances(&store, &truncated, &[0], &columns, Scope::AllSources) {
        Err(error) => error,
        Ok(_) => return Err("missing numeric cell accepted".into()),
    };
    check!(error.message.contains("missing cell"), "{}", error.message);
    Ok(())
}
