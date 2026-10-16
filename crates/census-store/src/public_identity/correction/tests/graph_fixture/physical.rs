use super::*;

pub(super) fn later_page_results(
    store: &Store,
    members: &[CanonicalAthlete],
) -> TestResult<Vec<String>> {
    let unrelated = subject("graph-correction-unrelated", "109")?;
    store.append(Table::Athletes, &unrelated)?;
    let event = EventId::mint("event", &["graph-correction-later"]);
    let meet = MeetId::mint("meet", &["graph-correction-later"]);
    let team = TeamId::mint("team", &["graph-correction-later"]);
    let performance = CanonicalPerformance::new(
        PerformanceIdentity {
            athlete: &unrelated.id,
            event: &event,
            meet: &meet,
            date: "2026-10-14",
            source_key: "graph-correction-physical-variants",
        },
        PerformanceResult {
            team: &team,
            mark: Mark::Raw("12.10".into()),
            wind_mps: None,
            place: Some(1),
            adjustment: None,
        },
    )?;
    let relay = RelayResult::new(
        &team,
        &event,
        &meet,
        Mark::Raw("48.10".into()),
        "graph-correction-physical-variants",
    );
    let mut performances = Vec::new();
    let mut relays = Vec::new();
    for ordinal in 0..520 {
        let mut row = performance.clone();
        row.mark = Mark::Raw(format!("variant-{ordinal}"));
        performances.push(row);
        let mut row = relay.clone();
        row.members =
            vec![RelayMember::new(1, format!("unrelated-{ordinal}"))
                .with_athlete(unrelated.id.clone())];
        relays.push(row);
    }
    let mut later = performance.clone();
    later.athlete = members.first().ok_or("member")?.id.clone();
    performances.push(later);
    let mut later = relay.clone();
    later.members = vec![RelayMember::new(1, "retained later-page member")
        .with_athlete(members.get(2).ok_or("member")?.id.clone())];
    relays.push(later);
    store.append_many(Table::Performances, &performances)?;
    store.append_many(Table::RelayResults, &relays)?;
    Ok(vec![
        format!("performances:{}", performance.id),
        format!("relay_results:{}", relay.id),
    ])
}
