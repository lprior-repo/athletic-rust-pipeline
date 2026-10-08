use super::*;
use census_domain::model::{CanonicalPerformance, EventKind, ExactSeconds, SpecificationError};

fn controlled_individuals(event_ids: &[i64]) -> TestResult<(MeetData, AllResults)> {
    let meet = serde_json::from_str(MEET_DATA)?;
    let mut results: AllResults = serde_json::from_str(ALL_RESULTS)?;
    results.blocks = event_ids
        .iter()
        .map(|event_id| {
            let mut block = results
                .blocks
                .iter()
                .find(|block| block.event_id == *event_id && block.gender == "M")
                .ok_or("captured boys event")?
                .clone();
            let row = block
                .results
                .iter()
                .find(|row| {
                    row.athlete_id.is_some()
                        && row.team_id.is_some()
                        && row.name().is_some()
                        && grade_of(row.grade.as_deref()).is_some()
                        && matches!(
                            super::super::read::meet_mark(
                                &EventKind::from_source_label(&block.short),
                                &row.result,
                                Some("T")
                            ),
                            Some((Mark::TimeSeconds(_), _))
                        )
                })
                .ok_or("captured numeric individual")?
                .clone();
            block.results = vec![row];
            Ok(block)
        })
        .collect::<TestResult<_>>()?;
    results.legs.clear();
    Ok((meet, results))
}

fn performance_for<'a>(walk: &'a Walk, source_key: &str) -> TestResult<&'a CanonicalPerformance> {
    Ok(walk
        .accumulated
        .performances
        .values()
        .find(|row| row.source_key == source_key)
        .ok_or("source-owned result must remain projected")?)
}

fn assert_subject(walk: &Walk, performance: &CanonicalPerformance, owner: i64) -> TestResult {
    let athlete = walk
        .accumulated
        .athletes
        .values()
        .find(|row| row.id == performance.athlete)
        .ok_or("result athlete must remain in population")?;
    let source_owner = performance
        .source_athlete
        .as_ref()
        .ok_or("published result owner")?;
    check!(eq; source_owner.namespace, SourceNamespace::athletic_net("athlete"));
    check!(eq; source_owner.id, owner.to_string());
    check!(athlete
        .identities()
        .any(|identity| identity == source_owner));
    let evidence = performance
        .evidence
        .first()
        .ok_or("retained parsed result evidence")?;
    check!(eq; evidence.source, SourceRef::new("athleticnet", None));
    check!(eq; evidence.observed_on, OBSERVED_ON);
    Ok(())
}

#[test]
fn cen17_controlled_sole_dns_retains_individual_subject_and_nonnumeric_result() -> TestResult {
    let (meet, mut results) = controlled_individuals(&[1])?;
    let row = results
        .blocks
        .first_mut()
        .and_then(|block| block.results.first_mut())
        .ok_or("controlled sole individual")?;
    row.result = "DNS".into();
    let owner = row.athlete_id.ok_or("captured athlete owner")?;
    let source_key = format!("athleticnet:{owner}-{}", row.result_id);
    let walk = absorb(&meet, &results, None)?;
    let performance = performance_for(&walk, &source_key)?;
    assert_subject(&walk, performance, owner)?;
    check!(eq; performance.mark, Mark::Raw("DNS".into()));
    let event = walk
        .accumulated
        .events
        .values()
        .find(|event| event.id == performance.event)
        .ok_or("retained DNS event")?;
    check!(eq; event.kind, EventKind::Track100m);
    Ok(())
}

#[test]
fn cen17_controlled_dq_relay_retains_each_published_leg_without_individual_split() -> TestResult {
    let meet: MeetData = serde_json::from_str(MEET_DATA)?;
    let mut results: AllResults = serde_json::from_str(ALL_RESULTS)?;
    let (mut block, mut squad) = results
        .blocks
        .iter()
        .find_map(|block| {
            block
                .results
                .iter()
                .find(|row| {
                    results
                        .legs
                        .iter()
                        .filter(|leg| leg.result_id == row.result_id)
                        .count()
                        == 4
                })
                .map(|row| (block.clone(), row.clone()))
        })
        .ok_or("captured four-leg relay")?;
    squad.result = "DQ".into();
    results.legs.retain(|leg| leg.result_id == squad.result_id);
    block.results = vec![squad.clone()];
    results.blocks = vec![block];
    let walk = absorb(&meet, &results, None)?;
    results
        .legs
        .iter()
        .enumerate()
        .try_for_each(|(index, leg)| -> TestResult {
            let owner = leg.athlete_id.ok_or("captured leg owner")?;
            let position = index.saturating_add(1);
            let source_key = format!("athleticnet:{owner}-{}:leg{position}", squad.result_id);
            let performance = performance_for(&walk, &source_key)?;
            assert_subject(&walk, performance, owner)?;
            check!(eq; performance.mark, Mark::Raw("DQ".into()));
            check!(eq; leg_position(performance), Some(u8::try_from(position)?));
            let athlete = walk
                .accumulated
                .athletes
                .values()
                .find(|row| row.id == performance.athlete)
                .ok_or("relay member")?;
            check!(eq; athlete.canonical_name, leg.name.trim());
            Ok(())
        })?;
    let squad_key = format!(
        "athleticnet:{}-{}",
        squad.athlete_id.ok_or("squad owner")?,
        squad.result_id
    );
    check!(!walk
        .accumulated
        .performances
        .values()
        .any(|row| row.source_key == squad_key));
    Ok(())
}

fn controlled_surface(published: Option<serde_json::Value>) -> TestResult {
    let (_, mut results) = controlled_individuals(&[1])?;
    results
        .blocks
        .first_mut()
        .and_then(|block| block.results.first_mut())
        .ok_or("controlled numeric individual")?
        .result = "10.941".into();
    let mut document: serde_json::Value = serde_json::from_str(MEET_DATA)?;
    let fields = document.as_object_mut().ok_or("captured meet object")?;
    match published {
        Some(value) => {
            fields.insert("sport2".into(), value);
        }
        None => {
            fields.remove("sport2");
        }
    }
    let meet: MeetData = serde_json::from_value(document)?;
    let walk = absorb(&meet, &results, None)?;
    let row = results
        .blocks
        .first()
        .and_then(|block| block.results.first())
        .ok_or("captured individual")?;
    let owner = row.athlete_id.ok_or("captured owner")?;
    let performance = performance_for(&walk, &format!("athleticnet:{owner}-{}", row.result_id))?;
    assert_subject(&walk, performance, owner)?;
    check!(eq; performance.mark, Mark::TimeSeconds(ExactSeconds::parse("10.941")?));
    let canonical_meet = walk
        .accumulated
        .meets
        .values()
        .find(|meet| meet.id == performance.meet)
        .ok_or("retained unresolved meet")?;
    check!(eq; canonical_meet.sports, Vec::<Sport>::new());
    let team = walk
        .accumulated
        .teams
        .values()
        .find(|team| team.id == performance.team)
        .ok_or("retained unresolved team")?;
    check!(eq; team.sport, Sport::Unknown);
    let athlete = walk
        .accumulated
        .athletes
        .values()
        .find(|row| row.id == performance.athlete)
        .ok_or("retained unresolved athlete")?;
    check!(eq; athlete.sports, Vec::<Sport>::new());
    Ok(())
}

#[test]
fn cen17_controlled_absent_sport_retains_numeric_facts_without_guessed_outdoor() -> TestResult {
    controlled_surface(None)
}

#[test]
fn cen17_controlled_null_sport_retains_numeric_facts_without_guessed_outdoor() -> TestResult {
    controlled_surface(Some(serde_json::Value::Null))
}

#[test]
fn cen17_controlled_unknown_sport_retains_numeric_facts_without_guessed_outdoor() -> TestResult {
    controlled_surface(Some(serde_json::Value::String("unknown".into())))
}

#[test]
fn cen10_controlled_metadata_type_conflict_withholds_only_affected_event_and_retains_facts(
) -> TestResult {
    let (meet, mut results) = controlled_individuals(&[1, 2])?;
    let mut metadata: EventDivisions = serde_json::from_str(EVENT_DIV)?;
    metadata
        .events
        .iter_mut()
        .find(|event| event.id == 1)
        .ok_or("captured 100m metadata")?
        .event_type = Some("F".into());
    results
        .blocks
        .first_mut()
        .and_then(|block| block.results.first_mut())
        .ok_or("controlled conflict row")?
        .result = "10.941".into();
    let published: AllResults = serde_json::from_str(ALL_RESULTS)?;
    let block = results
        .blocks
        .first_mut()
        .ok_or("controlled conflict block")?;
    let first = block.results.first().ok_or("controlled first result")?;
    let mut later = published
        .blocks
        .iter()
        .find(|candidate| candidate.event_id == block.event_id && candidate.gender == block.gender)
        .and_then(|candidate| {
            candidate.results.iter().find(|row| {
                row.result_id != first.result_id
                    && row.athlete_id.is_some()
                    && row.team_id.is_some()
                    && row.name().is_some()
                    && grade_of(row.grade.as_deref()).is_some()
            })
        })
        .cloned()
        .ok_or("captured later individual")?;
    later.result = "10.942a".into();
    block.results.push(later);
    let walk = absorb(&meet, &results, Some(&EventMetadata::new(&metadata)))?;
    let contested = results
        .blocks
        .first()
        .and_then(|block| block.results.first())
        .ok_or("controlled conflict row")?;
    let owner = contested.athlete_id.ok_or("captured conflict owner")?;
    let performance = performance_for(
        &walk,
        &format!("athleticnet:{owner}-{}", contested.result_id),
    )?;
    assert_subject(&walk, performance, owner)?;
    check!(eq; performance.mark, Mark::Raw("10.941".into()));
    let event = walk
        .accumulated
        .events
        .values()
        .find(|event| event.id == performance.event)
        .ok_or("retained contradicted event")?;
    check!(eq; event.resolved_specification(), Err(SpecificationError::ConflictingSpecification));
    check!(event
        .retained_conflicts
        .iter()
        .any(|conflict| conflict.subject_id == event.id.as_str()));
    let later = results
        .blocks
        .first()
        .and_then(|block| block.results.get(1))
        .ok_or("controlled later conflict row")?;
    let later_owner = later.athlete_id.ok_or("captured later owner")?;
    let later_performance = performance_for(
        &walk,
        &format!("athleticnet:{later_owner}-{}", later.result_id),
    )?;
    assert_subject(&walk, later_performance, later_owner)?;
    check!(eq; later_performance.event, performance.event);
    check!(eq; later_performance.mark, Mark::Raw("10.942a".into()));
    check!(eq; later_performance.timing, None);
    let neighbor = results
        .blocks
        .get(1)
        .and_then(|block| block.results.first())
        .ok_or("captured valid neighbor")?;
    let neighbor_owner = neighbor.athlete_id.ok_or("captured neighbor owner")?;
    let neighbor_performance = performance_for(
        &walk,
        &format!("athleticnet:{neighbor_owner}-{}", neighbor.result_id),
    )?;
    assert_subject(&walk, neighbor_performance, neighbor_owner)?;
    let neighbor_event = walk
        .accumulated
        .events
        .values()
        .find(|event| event.id == neighbor_performance.event)
        .ok_or("valid neighbor context")?;
    check!(eq; neighbor_event.kind, EventKind::Track200m);
    check!(eq; neighbor_event.resolved_specification()?.category,
        Some(census_domain::model::CompetitionCategory::Published("boys/varsity".into())));
    let (token, _) = super::super::super::parse::published_token(&neighbor.result)
        .ok_or("captured neighbor numeric token")?;
    check!(eq; neighbor_performance.mark, Mark::TimeSeconds(ExactSeconds::parse(token)?));
    Ok(())
}
