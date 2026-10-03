use super::*;

use crate::athlete_packet::{packet as athlete_packet, AthleteIndex};

#[test]
fn a_group_is_every_row_the_merge_kept_apart_under_one_key() -> TestResult {
    let (boys, girls, other) = rows()?;
    let index = AthleteIndex::read(vec![boys.clone(), girls.clone(), other.clone()]);
    let mut expected = vec![boys.id.to_string(), girls.id.to_string()];
    expected.sort();
    let (subject, sibling, group) = index
        .compare_members(boys.id.as_str(), &[])
        .ok_or("a group of two")?;
    check!(eq; subject.id, boys.id);
    check!(eq; sibling.id, girls.id);
    check!(eq; group.to_vec(), expected, "the group names every colliding id");
    check!(
        index.compare_members(other.id.as_str(), &[]).is_none(),
        "a row nobody collides with is not a case: there is no second side to compare"
    );
    Ok(())
}

#[test]
fn a_packet_carries_both_sides_source_identity_fields() -> TestResult {
    let (boys, girls, _) = rows()?;
    let case = case_for(&boys);
    let group = vec![boys.id.to_string(), girls.id.to_string()];
    let packet = athlete_packet(&case, &boys, &girls, &group)?;
    check!(eq; packet.subject_id, boys.id.as_str());
    check!(eq; packet.cases.len(), 1);
    check!(eq; packet.cases[0].case_id, case.id);
    for (side, row) in [("side_a", &boys), ("side_b", &girls)] {
        check!(eq; stated(&packet, "census", &format!("{side}_id")), Some(row.id.to_string()), "{side} is stated by its canonical id");
        check!(eq; stated(&packet, "census", &format!("{side}_name")), Some("Jordan Smith".to_string()));
        check!(eq; stated(&packet, "census", &format!("{side}_school")), Some(row.school.as_str().to_string()));
        check!(eq; stated(&packet, "census", &format!("{side}_grad_year")), Some("2027".to_string()));
        check!(eq; stated(&packet, "census", &format!("{side}_athlete_id")), None, "an athlete id is stated under the namespace that issued it, never under census");
    }
    check!(eq; stated(&packet, "milesplit_athlete", "side_a_athlete_id"), Some("14399169".to_string()));
    check!(eq; stated(&packet, "milesplit_athlete", "side_b_athlete_id"), Some("14399169".to_string()));
    check!(eq; stated(&packet, "milesplit_athlete", "side_b_profile_url"), Some("https://wi.milesplit.com/athletes/14399169/jordan-smith".to_string()));
    check!(eq; stated(&packet, "athleticnet:athlete", "side_b_athlete_id"), Some("998877".to_string()), "a provider only one side is known by still rides with that side");
    check!(eq; stated(&packet, "tfrrs_athlete", "side_a_profile_url"), None, "a provider that published no profile URL states none rather than an empty one");
    check!(eq; stated(&packet, "census", "side_a_gender"), Some("Boys".to_string()));
    check!(eq; stated(&packet, "census", "side_b_gender"), Some("Girls".to_string()));
    check!(eq; stated(&packet, "census", "answer_field"), Some("identity".to_string()));
    check!(eq; stated(&packet, "census", "answer_values"), Some("same_person | different_person".to_string()));
    check!(eq; stated(&packet, "census", "candidate_ids"), Some(group.join(", ")));
    Ok(())
}
