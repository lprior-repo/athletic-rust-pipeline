use std::collections::BTreeMap;

use super::parse::{infer_level, MeetRow};
use census_domain::model::{
    CanonicalMeet, Evidence, MeetId, SourceIdentity, SourceNamespace, SourceRef,
};

pub fn build_meets(rows: &[MeetRow], observed_on: &str, source_label: &str) -> Vec<CanonicalMeet> {
    let mut meets: BTreeMap<MeetId, CanonicalMeet> = BTreeMap::new();
    for row in rows {
        let id = CanonicalMeet::mint(
            Some(row.state_code),
            &row.start,
            &row.name,
            row.city_state.as_deref(),
        );
        match meets.entry(id) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(project(row, observed_on, source_label));
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                merge_row(entry.get_mut(), row)
            }
        }
    }
    meets.into_values().collect()
}

pub(super) fn project(row: &MeetRow, observed_on: &str, source_label: &str) -> CanonicalMeet {
    let mut meet = meet_from_row(row, observed_on, source_label);
    merge_row(&mut meet, row);
    meet
}

fn merge_row(meet: &mut CanonicalMeet, row: &MeetRow) {
    if meet.end_date.is_none() {
        meet.end_date = row.end.clone();
    }
    if meet.location.is_none() {
        meet.location = row.city_state.clone();
    }
    let timer = SourceIdentity::new(
        SourceNamespace::TimerMeet {
            provider: row.tenant.clone(),
        },
        row.athleticlive_meet_id.clone(),
    );
    push_identity(&mut meet.source_identities, timer);
    if let Some(an_id) = &row.athleticnet_meet_id {
        let identity = SourceIdentity::new(SourceNamespace::athletic_net("meet"), an_id.clone())
            .with_url(format!(
                "https://www.athletic.net/TrackAndField/meet/{an_id}/info"
            ));
        push_identity(&mut meet.source_identities, identity);
    }
}

fn meet_from_row(row: &MeetRow, observed_on: &str, source_label: &str) -> CanonicalMeet {
    let mut meet = CanonicalMeet::new(
        Some(row.state_code),
        row.name.clone(),
        row.start.clone(),
        infer_level(&row.name),
    );
    meet.end_date = row.end.clone();
    meet.location = row.city_state.clone();
    meet.evidence.push(Evidence::parsed(
        SourceRef::new(source_label, None),
        observed_on,
    ));
    meet
}

fn push_identity(identities: &mut Vec<SourceIdentity>, identity: SourceIdentity) {
    if !identities.contains(&identity) {
        identities.push(identity);
    }
}
