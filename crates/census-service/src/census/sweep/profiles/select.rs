use super::{inventory, Cursor, Selection, Target};
use census_crawl::participation_evidence::provider_school::OwnedProviderSchoolPacketV1;
use census_crawl::CrawlResult;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::{AcquisitionDigest, GradYear};
use census_domain::UsJurisdiction;
use census_store::{PublicSchoolIntakePacketV1, Store, StoreError};

pub fn select(
    store: &Store,
    site: UsJurisdiction,
    cohort: GradYear,
    cursor: Option<&Cursor>,
) -> CrawlResult<Selection> {
    let snapshot = store.snapshot();
    let next = match cursor {
        Some(cursor) if !cursor.finished => Some((
            cursor.packet.clone(),
            inventory::read_packet(
                &snapshot,
                cursor.packet.as_str(),
                &mut AttachmentReplayGrantV1::new(),
            )?,
        )),
        _ => inventory::next_owned_roster_packet(
            &snapshot,
            site,
            cursor.map(|cursor| &cursor.packet),
        )?,
    };
    let Some((key, packet)) = next else {
        return Ok(Selection::End);
    };
    if packet.roster().work().slot().source_site() != site {
        return Err(refused("profile cursor belongs to another jurisdiction").into());
    }
    let member = cursor
        .filter(|cursor| !cursor.finished)
        .map_or(0, |cursor| cursor.member);
    select_member(packet, key, cohort, member)
}

fn select_member(
    packet: PublicSchoolIntakePacketV1,
    key: AcquisitionDigest,
    cohort: GradYear,
    after: usize,
) -> CrawlResult<Selection> {
    let checked = OwnedProviderSchoolPacketV1::from_packet(packet).read()?;
    let verdict = checked.roster_verdict();
    let members = verdict
        .roster()
        .map_or(&[][..], |roster| roster.athletes.as_slice());
    if after > members.len() {
        return Err(refused("profile member cursor is outside original roster").into());
    }
    for (index, member) in members.iter().enumerate().skip(after) {
        if member.athlete().grad_year == cohort {
            let next_member = index.checked_add(1).ok_or(StoreError::CounterOverflow)?;
            return Ok(Selection::Profile(Target {
                packet: key.clone(),
                athlete_id: member.athlete().athlete_id.clone(),
                next: Cursor {
                    packet: key,
                    member: next_member,
                    finished: false,
                },
            }));
        }
    }
    let gaps = verdict
        .rejections()
        .len()
        .checked_add(usize::from(verdict.unfinished().is_some()))
        .and_then(|count| count.checked_add(usize::from(verdict.roster().is_none())))
        .ok_or(StoreError::CounterOverflow)?;
    Ok(Selection::Advance {
        next: Cursor {
            packet: key,
            member: members.len(),
            finished: true,
        },
        gaps,
    })
}

fn refused(detail: &str) -> StoreError {
    StoreError::Refused {
        detail: detail.to_owned(),
    }
}
