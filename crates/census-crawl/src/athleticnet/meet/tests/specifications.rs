use super::super::wire::FlatEvent;
use super::*;
use census_domain::model::{EventKind, SpecificationError};

pub(super) fn blocks(labels: &[(&str, &str, &str)]) -> TestResult<(MeetData, AllResults)> {
    let meet = serde_json::from_str(MEET_DATA)?;
    let mut results: AllResults = serde_json::from_str(ALL_RESULTS)?;
    let template = results
        .blocks
        .iter()
        .find(|block| {
            block.results.iter().any(|row| {
                row.athlete_id.is_some()
                    && row.team_id.is_some()
                    && row.name().is_some()
                    && grade_of(row.grade.as_deref()).is_some()
            })
        })
        .ok_or("graded individual fixture")?
        .clone();
    let row = template
        .results
        .iter()
        .find(|row| {
            row.athlete_id.is_some()
                && row.team_id.is_some()
                && row.name().is_some()
                && grade_of(row.grade.as_deref()).is_some()
        })
        .ok_or("graded fixture row")?
        .clone();
    results.legs.clear();
    results.blocks = labels
        .iter()
        .enumerate()
        .map(|(index, (label, short, mark))| {
            let mut block: FlatEvent = template.clone();
            block.label = (*label).into();
            block.short = (*short).into();
            let mut row = row.clone();
            row.result_id = i64::try_from(index)?;
            row.result = (*mark).into();
            block.results = vec![row];
            Ok(block)
        })
        .collect::<TestResult<_>>()?;
    Ok((meet, results))
}

fn event_for<'a>(
    walk: &'a Walk,
    result: i64,
) -> TestResult<&'a census_domain::model::CanonicalEvent> {
    let suffix = format!("-{result}");
    let performance = walk
        .accumulated
        .performances
        .values()
        .find(|row| row.source_key.ends_with(&suffix))
        .ok_or("projected result")?;
    Ok(walk
        .accumulated
        .events
        .get(performance.event.as_str())
        .ok_or("event reference")?)
}

#[test]
fn same_meet_implement_masses_keep_separate_refs_and_equivalent_units_share_context() -> TestResult
{
    let (meet, results) = blocks(&[
        ("Shot Put (4kg)", "SP", "40-0"),
        ("Shot Put (3kg)", "SP", "45-0"),
        ("Shot Put (4000g)", "SP", "42-0"),
        ("Shot Put", "SP", "50-0"),
    ])?;
    let walk = absorb(&meet, &results, None)?;
    let (four, three, equivalent, unknown) = (
        event_for(&walk, 0)?,
        event_for(&walk, 1)?,
        event_for(&walk, 2)?,
        event_for(&walk, 3)?,
    );
    check!(eq; four.kind, EventKind::ShotPut);
    check!(eq; four.id, equivalent.id);
    check!(four.id != three.id);
    check!(four.id != unknown.id);
    check!(eq; four.specification.implement.ok_or("published mass")?.micrograms(), 4_000_000_000);
    check!(eq; three.specification.implement.ok_or("published mass")?.micrograms(), 3_000_000_000);
    check!(eq; unknown.specification.implement, None);
    check!(eq; four.resolved_specification()?, equivalent.resolved_specification()?);
    check!(four
        .source_labels
        .iter()
        .any(|row| row.label == "Shot Put (4000g)"));
    Ok(())
}

#[test]
fn same_meet_hurdle_heights_keep_separate_refs_and_exact_time_marks() -> TestResult {
    let (meet, results) = blocks(&[
        ("110 Meter Hurdles (39in)", "110H", "14.251"),
        ("110 Meter Hurdles (42in)", "110H", "14.249"),
        ("110 Meter Hurdles (99.06cm)", "110H", "14.251"),
    ])?;
    let walk = absorb(&meet, &results, None)?;
    let (low, high, equivalent) = (
        event_for(&walk, 0)?,
        event_for(&walk, 1)?,
        event_for(&walk, 2)?,
    );
    check!(eq; low.id, equivalent.id);
    check!(low.id != high.id);
    check!(eq; low.specification.hurdles.ok_or("height")?.height_micrometres, 990_600);
    check!(eq; high.specification.hurdles.ok_or("height")?.height_micrometres, 1_066_800);
    let expected = census_domain::model::ExactSeconds::parse("14.251")?;
    let performance = walk
        .accumulated
        .performances
        .values()
        .find(|row| row.source_key.ends_with("-0"))
        .ok_or("first mark")?;
    check!(eq; performance.mark, Mark::TimeSeconds(expected));
    Ok(())
}

#[test]
fn invalid_published_mass_refuses_numeric_projection_without_hiding_later_valid_records(
) -> TestResult {
    let (meet, results) = blocks(&[
        ("Shot Put (0kg)", "SP", "60-0"),
        ("Shot Put (4kg)", "SP", "40-0"),
    ])?;
    let mut accumulated = Accumulator::default();
    let mut stats = Stats::default();
    let source = SourceRef::new(
        "athleticnet",
        Some("https://www.athletic.net/TrackAndField/meet/634313".into()),
    );
    match absorb_meet(
        &meet,
        &results,
        None,
        &source,
        (
            OBSERVED_ON,
            chrono::NaiveDate::from_ymd_opt(2026, 9, 30).ok_or("snapshot date")?,
        ),
        &SchoolIndex::from_schools(&[]),
        &mut HashMap::new(),
        &mut stats,
        &mut accumulated,
    ) {
        Err(crate::CrawlError::Specification(SpecificationError::InvalidMass)) => {}
        outcome => return Err(format!("expected invalid mass, received {outcome:?}").into()),
    }
    let keys: Vec<_> = accumulated
        .performances
        .values()
        .map(|row| row.source_key.clone())
        .collect();
    let valid_owner = results
        .blocks
        .get(1)
        .and_then(|block| block.results.first())
        .and_then(|row| row.athlete_id)
        .ok_or("fixture owner")?;
    check!(eq; keys, vec![format!("athleticnet:{valid_owner}-1")]);
    let event = accumulated
        .events
        .values()
        .next()
        .ok_or("later valid event")?;
    check!(eq; event.specification.implement.ok_or("later valid mass")?.micrograms(), 4_000_000_000);
    Ok(())
}

#[test]
fn contradictory_published_aliases_cannot_mint_a_numeric_event() -> TestResult {
    let (meet, results) = blocks(&[("Shot Put (4kg)", "DT", "40-0")])?;
    let mut accumulated = Accumulator::default();
    let source = SourceRef::new(
        "athleticnet",
        Some("https://www.athletic.net/TrackAndField/meet/634313".into()),
    );
    match absorb_meet(
        &meet,
        &results,
        None,
        &source,
        (
            OBSERVED_ON,
            chrono::NaiveDate::from_ymd_opt(2026, 9, 30).ok_or("snapshot date")?,
        ),
        &SchoolIndex::from_schools(&[]),
        &mut HashMap::new(),
        &mut Stats::default(),
        &mut accumulated,
    ) {
        Err(crate::CrawlError::Specification(SpecificationError::ConflictingSpecification)) => {}
        outcome => {
            return Err(format!(
                "contradictory aliases must refuse numeric projection: {outcome:?}"
            )
            .into())
        }
    }
    check!(accumulated.performances.is_empty());
    let event = accumulated
        .events
        .values()
        .next()
        .ok_or("retained event context")?;
    check!(eq; event.source_labels.iter().map(|row| row.label.as_str()).collect::<Vec<_>>(),
        ["Shot Put (4kg)", "DT"]);
    check!(event.source_labels.iter().all(|row| row.source == source));
    check!(event
        .evidence
        .iter()
        .any(|evidence| evidence.source == source && evidence.observed_on == OBSERVED_ON));
    check!(eq; event.resolved_specification(), Err(SpecificationError::ConflictingSpecification));
    Ok(())
}
