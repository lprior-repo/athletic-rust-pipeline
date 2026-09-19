use super::verification::{load, outcome};
use crate::{
    domain::{identity::EvidenceDigest, name::CanonicalName},
    runtime::{
        protocol::{FetchOutcome, RankingsCapture, SourceResource},
        rankings::{parse_page_response, ExpectedPageContext, PageObservation, RankingsPlan},
        rankings_collection::{CollectionFinalSnapshot, RankingsPageCheckpoint},
    },
    store::{ArtifactStore, RankingCandidateKind, RankingRecordRef},
};
use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::collections::HashMap;
use url::Url;

pub(super) fn verify(
    records: &[RankingRecordRef],
    name: &CanonicalName,
    snapshot: &CollectionFinalSnapshot,
    plan: &RankingsPlan,
    origin: &Url,
    store: &ArtifactStore,
) -> Result<()> {
    let mut grouped: HashMap<&EvidenceDigest, Vec<&RankingRecordRef>> = HashMap::new();
    records
        .iter()
        .for_each(|record| grouped.entry(&record.checkpoint).or_default().push(record));
    grouped.into_iter().try_for_each(|(digest, records)| {
        let checkpoint: RankingsPageCheckpoint = load(store, digest)?;
        chain(digest, &checkpoint, snapshot, store)?;
        let event = plan
            .for_event(&checkpoint.event_short)
            .context("ranking checkpoint event absent from plan")?;
        let resource = SourceResource::Rankings {
            collection: snapshot.collection.clone(),
            list_id: snapshot.scope.list_id,
            gender: snapshot.scope.gender.clone(),
            grade: if event.is_relay {
                None
            } else {
                Some(snapshot.scope.projection_grade)
            },
            event_short: event.short.clone(),
            page: checkpoint.page,
            capture: RankingsCapture::Results,
        };
        let raw = outcome(&checkpoint.outcome, &resource, origin, store)?;
        let parsed = parse_page_response(
            &raw,
            &ExpectedPageContext {
                division_id: snapshot.scope.list_id,
                season_id: snapshot.scope.season,
                gender: &snapshot.scope.gender,
                event_short: &event.short,
                event_id: Some(event.event_id),
                is_relay: event.is_relay,
                requested_grade: if event.is_relay {
                    None
                } else {
                    Some(snapshot.scope.projection_grade)
                },
                page: checkpoint.page,
            },
        )?;
        let retained: PageObservation = load(store, &checkpoint.observation_digest)?;
        if serde_json::to_value(&parsed)? != serde_json::to_value(retained)? {
            bail!("ranking observation differs from retained raw source response");
        }
        records
            .into_iter()
            .try_for_each(|record| candidate(record, name, &parsed, &raw))
    })
}

fn chain(
    target: &EvidenceDigest,
    target_checkpoint: &RankingsPageCheckpoint,
    snapshot: &CollectionFinalSnapshot,
    store: &ArtifactStore,
) -> Result<()> {
    let head = snapshot
        .event_heads
        .iter()
        .find(|event| event.event_short == target_checkpoint.event_short)
        .context("ranking checkpoint has no sealed event head")?;
    let mut current = head.head_checkpoint.clone();
    let mut found = false;
    for page in (1..=head.page_count).rev() {
        let digest = current
            .take()
            .context("ranking checkpoint chain ended prematurely")?;
        let checkpoint: RankingsPageCheckpoint = load(store, &digest)?;
        if checkpoint.collection != snapshot.collection
            || checkpoint.revision != snapshot.scope.revision
            || checkpoint.event_short != head.event_short
            || checkpoint.page != page
        {
            bail!("ranking checkpoint chain has a foreign, reordered, or repeated page");
        }
        let FetchOutcome::Retrieved { receipt, .. } = &checkpoint.outcome else {
            bail!("checkpoint contains failed acquisition");
        };
        let capture = receipt
            .rankings
            .as_ref()
            .context("checkpoint has no pagination observation")?;
        let next = if page == head.page_count {
            None
        } else {
            page.checked_add(1)
        };
        if capture.next_page != next {
            bail!("checkpoint pagination differs from its sealed chain");
        }
        found |= digest == *target;
        current = checkpoint.previous_checkpoint;
    }
    if current.is_some() || !found {
        bail!("ranking witness is outside the sealed checkpoint chain");
    }
    Ok(())
}

fn candidate(
    record: &RankingRecordRef,
    name: &CanonicalName,
    parsed: &PageObservation,
    raw: &Value,
) -> Result<()> {
    let index = usize::try_from(record.record_index)?;
    match record.kind {
        RankingCandidateKind::Individual => {
            let candidate = parsed
                .grade_11_candidates_list
                .get(index)
                .context("individual record index outside observation")?;
            if candidate.athlete_id != record.athlete_id
                || candidate.name != *name
                || candidate.grade_id != 11
            {
                bail!("individual ranking record differs from source identity or grade");
            }
            let row = raw
                .pointer(
                    candidate
                        .source_locator
                        .as_deref()
                        .context("individual source locator missing")?,
                )
                .context("individual source locator does not resolve")?;
            raw_identity(row, "AthleteID", record, name)?;
            if row.get("IDResult").and_then(Value::as_u64) != Some(candidate.id_result)
                || !parsed
                    .source_rows
                    .iter()
                    .any(|source| source.result_id == candidate.id_result)
            {
                bail!("individual ranking result differs from raw source row");
            }
        }
        RankingCandidateKind::RelayMember => {
            let member = parsed
                .verified_relay_members
                .get(index)
                .context("relay record index outside observation")?;
            if member.athlete_id != record.athlete_id
                || member.name != *name
                || member.grade_id != 11
            {
                bail!("relay ranking record differs from source identity or grade");
            }
            let row = raw
                .pointer(
                    member
                        .row_locator
                        .as_deref()
                        .context("relay row locator missing")?,
                )
                .context("relay row locator does not resolve")?;
            let team = raw
                .pointer(&format!("/relayTeams/{}", member.id_result))
                .context("relay roster missing")?;
            let index = member
                .member_locator
                .context("relay member locator missing")?;
            let retained_member = team
                .get("Members")
                .and_then(Value::as_array)
                .and_then(|members| members.get(index))
                .context("relay member locator does not resolve")?;
            raw_identity(retained_member, "IDAthlete", record, name)?;
            if row.get("IDResult").and_then(Value::as_u64) != Some(member.id_result)
                || team.get("IDResult").and_then(Value::as_u64) != Some(member.id_result)
                || team.get("RelayTeamID").and_then(Value::as_u64)
                    != Some(member.roster_relay_team_id)
                || row.get("AthleteID").and_then(Value::as_u64) != Some(member.row_athlete_id)
                || member.roster_relay_team_id != member.row_athlete_id
            {
                bail!("relay raw roster join differs from the source ranking row");
            }
        }
    }
    Ok(())
}

fn raw_identity(
    raw: &Value,
    id_field: &str,
    record: &RankingRecordRef,
    name: &CanonicalName,
) -> Result<()> {
    let raw_name = raw
        .get("AthleteName")
        .and_then(Value::as_str)
        .context("source candidate name missing")?;
    if raw.get(id_field).and_then(Value::as_u64) != Some(record.athlete_id.get())
        || raw.get("GradeID").and_then(Value::as_u64) != Some(11)
        || CanonicalName::parse(raw_name)? != *name
    {
        bail!("raw ranking candidate identity differs from canonical lookup witness");
    }
    Ok(())
}
