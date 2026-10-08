use super::*;
#[test]
fn the_probe_meet_reads_every_published_row() -> TestResult {
    let walk = walk(false)?;
    check!(eq; walk.meet.meet.id, 634313);
    check!(eq; walk.meet.meet.name, "Big 8 Conference");
    check!(eq; walk.meet.meet.season_id, Some(2026));
    check!(eq; walk.meet.sport2.as_deref(), Some("tfo"), "an outdoor meet");
    check!(eq;
        walk.meet.divisions.len(),
        2,
        "`tfDivisions` names both divisions the payload declares"
    );
    check!(eq; walk.counts.blocks, 49, "49 blocks: {}", walk.counts);
    check!(eq; walk.counts.rows_seen, 758, "758 published rows");
    check!(eq; walk.counts.relay_rows, 72, "72 of them are relay squads");
    check!(eq; walk.counts.legs_seen, 288, "288 relay legs");
    check!(eq; walk.counts.meets_pulled, 1);
    check!(eq; walk.counts.meets_unplaced, 0);
    check!(eq; walk.counts.meets_without_date, 0);
    check!(eq; walk.counts.meets_without_season, 0);
    check!(eq;
        walk.counts.blocks_without_division, 0,
        "every block publishes its division"
    );
    check!(eq;
        walk.counts.blocks_gender_unknown, 0,
        "every block publishes M or F"
    );
    check!(eq;
        walk.counts.rows_unknown_school, 0,
        "every row's TeamID resolves through the payload's own team list"
    );
    check!(eq;
        walk.counts.relay_rows_without_legs, 0,
        "every squad row has legs keyed to it by ResultID"
    );
    check!(eq; walk.counts.legs_no_name, 0, "every leg publishes a name");
    check!(eq;
        walk.counts.legs_no_athlete, 0,
        "every leg publishes an athlete id"
    );
    check!(eq;
        walk.counts.legs_no_grade, 0,
        "every leg's ShortDesc is a real grade"
    );
    check!(eq; walk.stats.fetches_failed, 0);
    check!(eq; walk.counts.rows_no_grade, 0);
    check!(eq; walk.counts.rows_no_name, 0);
    check!(eq; walk.counts.rows_no_athlete, 0);
    check!(eq;
        walk.counts.rows_unmapped_event, 0,
        "no row needed the metadata document to read its mark"
    );
    Ok(())
}
#[test]
fn the_meet_row_carries_what_the_document_publishes_and_nothing_it_does_not() -> TestResult {
    let walk = walk(false)?;
    check!(eq; walk.accumulated.meets.len(), 1);
    check!(eq; walk.accumulated.schools.len(), 10);
    check!(eq;
        walk.accumulated.events.len(),
        49,
        "one event per block: this capture publishes prelims and finals of the same event four \
         times, and the round is part of the event identity"
    );
    let row = walk
        .accumulated
        .meets
        .values()
        .next()
        .ok_or("one meet row")?;
    check!(eq; row.state, Some(UsJurisdiction::Wisconsin));
    check!(eq; row.date, "2026-05-15");
    check!(eq;
        row.end_date, None,
        "`EndDate` repeats the start date on this capture, so it is not published as a range"
    );
    check!(eq; row.location.as_deref(), Some("Sun Prairie HS"));
    check!(eq; row.sports, vec![Sport::OutdoorTrack]);
    check!(eq;
        row.level,
        CompetitionLevel::Unknown,
        "the level stays what the bio path mints for the same meet: no capture maps the payload's \
         `LevelMask` onto the platform's levels"
    );
    check!(eq;
        row.source_urls,
        vec!["https://www.athletic.net/TrackAndField/meet/634313/info".to_string()]
    );
    let identities = meet_identities(row)?;
    check!(eq;
        identities.get("meet"),
        Some(&"634313"),
        "the published meet id"
    );
    check!(eq;
        identities.get("live"),
        Some(&"73767"),
        "`LiveID` is persisted for the AthleticLIVE join"
    );
    Ok(())
}
#[test]
fn every_published_team_id_mints_a_school_and_a_team() -> TestResult {
    let walk = walk(false)?;
    check!(eq;
        walk.accumulated.schools.len(),
        10,
        "the capture's rows name 10 distinct TeamIDs"
    );
    check!(eq;
        walk.accumulated.teams.len(),
        20,
        "10 schools, each with the boys and the girls team this meet published"
    );
    let team_ids: BTreeSet<&str> = walk
        .accumulated
        .teams
        .values()
        .map(|team| team.id.as_str())
        .collect();
    let performance_teams: BTreeSet<&str> = walk
        .accumulated
        .performances
        .values()
        .map(|row| row.team.as_str())
        .collect();
    check!(eq;
        team_ids, performance_teams,
        "every performance hangs off a minted team"
    );
    check!(eq;
        walk.stats.rows_unknown_school, 0,
        "no row fell through the school reader"
    );
    Ok(())
}
#[test]
fn a_relay_squad_becomes_one_performance_per_leg_and_never_a_person() -> TestResult {
    let walk = walk(false)?;
    let rows = performances(&walk);
    let legs: Vec<_> = rows
        .iter()
        .filter(|row| leg_position(row).is_some())
        .collect();
    check!(
        legs.iter().all(|row| !row.source_key.contains("-0")),
        "a squad's own row is not a performance"
    );
    let positions: BTreeSet<u8> = legs.iter().filter_map(|row| leg_position(row)).collect();
    check!(eq;
        positions,
        BTreeSet::from([1, 2, 3, 4]),
        "the leg's 1-based position in its squad is its published order"
    );
    for row in &legs {
        let note = row
            .evidence
            .first()
            .and_then(|evidence| evidence.note.as_deref());
        check!(
            note.is_some_and(|note| note.starts_with("relay leg ")),
            "{}: a leg carries its own evidence note: {note:?}",
            row.source_key
        );
    }
    check!(eq;
        walk.counts.rows_no_grade, 0,
        "the 71 squad rows publishing `-` never reach the grade reader: their legs are routed to \
         the relay reader by ResultID membership"
    );
    let names: BTreeSet<&str> = walk
        .accumulated
        .athletes
        .values()
        .map(|athlete| athlete.canonical_name.as_str())
        .collect();
    check!(
        names.iter().all(|name| !name.contains("<BR>")),
        "a squad's `<BR>`-joined members are never minted as one athlete"
    );
    check!(
        rows.iter().all(|row| !row.source_key.contains("<BR>")),
        "no performance is keyed on the squad's padded name"
    );
    Ok(())
}
#[test]
fn every_performance_is_keyed_by_the_published_result_it_came_from() -> TestResult {
    let walk = walk(false)?;
    let results: AllResults = serde_json::from_str(ALL_RESULTS)?;
    let expected = results
        .blocks
        .iter()
        .flat_map(|block| &block.results)
        .map(|row| -> TestResult<Vec<String>> {
            let legs: Vec<_> = results
                .legs
                .iter()
                .filter(|leg| leg.result_id == row.result_id)
                .collect();
            if legs.is_empty() {
                let owner = row.athlete_id.ok_or("captured individual owner")?;
                return Ok(vec![format!("athleticnet:{owner}-{}", row.result_id)]);
            }
            legs.iter()
                .enumerate()
                .map(|(index, leg)| {
                    let owner = leg.athlete_id.ok_or("captured relay owner")?;
                    Ok(format!(
                        "athleticnet:{owner}-{}:leg{}",
                        row.result_id,
                        index.saturating_add(1)
                    ))
                })
                .collect()
        })
        .collect::<TestResult<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect::<BTreeSet<_>>();
    let projected = performances(&walk)
        .iter()
        .map(|row| row.source_key.clone())
        .collect::<BTreeSet<_>>();
    check!(eq; projected, expected);
    for row in performances(&walk) {
        check!(eq;
            row.evidence.len(),
            1,
            "one parsed-evidence entry per row: {}",
            row.source_key
        );
        let evidence = row.evidence.first().ok_or("one evidence entry")?;
        check!(eq; evidence.method, EvidenceMethod::Parsed);
        check!(eq; evidence.observed_on, OBSERVED_ON);
        check!(eq; evidence.source.id, "athleticnet");
        let _: &PerformanceId = &row.id;
    }
    Ok(())
}

#[test]
fn captured_tfo_numeric_result_keeps_exact_value_and_fat() -> TestResult {
    let results: AllResults = serde_json::from_str(ALL_RESULTS)?;
    let row = results
        .blocks
        .iter()
        .flat_map(|block| &block.results)
        .find(|row| row.result == "10.41a")
        .ok_or("captured automatic 10.41 result")?;
    let owner = row.athlete_id.ok_or("captured numeric owner")?;
    let walk = walk(true)?;
    let key = format!("athleticnet:{owner}-{}", row.result_id);
    let performance = walk
        .accumulated
        .performances
        .values()
        .find(|performance| performance.source_key == key)
        .ok_or("captured numeric performance")?;
    check!(eq; performance.mark,
        Mark::TimeSeconds(census_domain::model::ExactSeconds::parse("10.41")?));
    check!(eq; performance.timing, Some(TimingMethod::Fat));
    let team = walk
        .accumulated
        .teams
        .values()
        .find(|team| team.id == performance.team)
        .ok_or("captured numeric team")?;
    check!(eq; team.sport, Sport::OutdoorTrack);
    Ok(())
}

fn meet_identities(row: &census_domain::model::CanonicalMeet) -> TestResult<BTreeMap<&str, &str>> {
    row.source_identities
        .iter()
        .map(|identity| {
            let kind = match &identity.namespace {
                SourceNamespace::AthleticNet { kind } => kind.as_str(),
                other => {
                    return Err(format!(
                        "meet identity is not under this adapter's namespace: {other:?}"
                    )
                    .into())
                }
            };
            Ok((kind, identity.id.as_str()))
        })
        .collect()
}
