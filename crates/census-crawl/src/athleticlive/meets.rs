//! Canonical meet construction: one meet per platform identity, carrying every tenant that
//! published it and the Athletic.net id the row names.

use std::collections::BTreeMap;

use super::parse::{infer_level, MeetRow};
use census_domain::model::{
    CanonicalMeet, Evidence, MeetId, SourceIdentity, SourceNamespace, SourceRef,
};

/// Build canonical meets from parsed rows, merging tenants that publish the same meet.
pub fn build_meets(rows: &[MeetRow], observed_on: &str, source_label: &str) -> Vec<CanonicalMeet> {
    let mut meets: BTreeMap<MeetId, CanonicalMeet> = BTreeMap::new();
    for row in rows {
        let id = CanonicalMeet::mint(
            Some(row.state_code),
            &row.start,
            &row.name,
            row.city_state.as_deref(),
        );
        let entry = meets
            .entry(id.clone())
            .or_insert_with(|| meet_from_row(row, observed_on, source_label));
        if entry.end_date.is_none() {
            entry.end_date = row.end.clone();
        }
        if entry.location.is_none() {
            entry.location = row.city_state.clone();
        }
        let timer_identity = SourceIdentity::new(
            SourceNamespace::TimerMeet {
                provider: row.tenant.clone(),
            },
            row.athleticlive_meet_id.clone(),
        );
        push_identity(&mut entry.source_identities, timer_identity);
        if let Some(an_id) = &row.athleticnet_meet_id {
            let an_identity = SourceIdentity::new(
                SourceNamespace::LegacyAthleticNet {
                    kind: "meet".to_string(),
                },
                an_id.clone(),
            )
            .with_url(format!(
                "https://www.athletic.net/TrackAndField/meet/{an_id}/info"
            ));
            push_identity(&mut entry.source_identities, an_identity);
        }
    }
    meets.into_values().collect()
}

/// One canonical meet as a single harvest row publishes it.
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

/// Record a source identity once, whichever row or tenant published it first.
fn push_identity(identities: &mut Vec<SourceIdentity>, identity: SourceIdentity) {
    if !identities.contains(&identity) {
        identities.push(identity);
    }
}
