use super::*;
use census_domain::UsJurisdiction;
use std::collections::BTreeSet;

fn target(tenant: &str) -> MeetTarget {
    MeetTarget {
        athleticlive_meet_id: 42, meet_id: "fixture-meet".to_owned(), tenant: tenant.to_owned(),
        name: "Example Meet".to_owned(), state: UsJurisdiction::Iowa, date: "2026-04-01".to_owned(),
    }
}

fn hit(row: Option<u64>, native: Option<u64>) -> AthleteHit {
    AthleteHit {
        i: row.map(Value::from), n: Some("Alex Rivera".to_owned()), y: Some(Value::from(11)),
        g: Some("M".to_owned()), mi: Some(Value::from(42)), ani: native.map(Value::from),
        t: Some(HitTeam { n: Some("Example School".to_owned()), ..HitTeam::default() }),
    }
}

#[test]
fn roster_homonyms_keep_distinct_owners_and_missing_subjects_do_not_fall_back_to_names() {
    let target = target("timer-a");
    let targets = HashMap::from([(42, &target)]);
    let hits = [hit(Some(1), None), hit(Some(2), None), hit(None, None), hit(None, Some(111)), hit(Some(9), Some(111))];
    let entities = build_entities(&hits, &targets, "2026-04-02", SchoolYear::new(2026).expect("year"));
    assert_eq!(entities.rows, 5);
    assert_eq!(entities.rows_without_subject_id, 1);
    assert_eq!(entities.athletes.len(), 3);
    let candidates: BTreeSet<_> = entities.athletes.iter().map(CanonicalAthlete::candidate_key).collect();
    assert_eq!(candidates.len(), 1);
    let owners: BTreeSet<_> = entities.athletes.iter().map(|athlete| &athlete.id).collect();
    assert_eq!(owners.len(), 3);
}

#[test]
fn the_same_roster_row_number_from_different_providers_does_not_merge() {
    let first = target("timer-a");
    let second = target("timer-b");
    let hits = [hit(Some(1), None)];
    let year = SchoolYear::new(2026).expect("year");
    let first = build_entities(&hits, &HashMap::from([(42, &first)]), "2026-04-02", year);
    let second = build_entities(&hits, &HashMap::from([(42, &second)]), "2026-04-02", year);
    assert_eq!(first.athletes.len(), 1);
    assert_eq!(second.athletes.len(), 1);
    assert_eq!(first.athletes[0].candidate_key(), second.athletes[0].candidate_key());
    assert_ne!(first.athletes[0].id, second.athletes[0].id);
}
