use super::super::super::RetainedCaptureRootSchemaV1 as Schema;
use super::{capture, Error, RetainedCaptureSlotV1 as Slot};
use crate::{StoreError, StoreResult};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use serde::Deserialize;
use serde_json::value::RawValue;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Participation<'a> {
    version: u16,
    #[serde(borrow)]
    status: &'a RawValue,
    #[serde(borrow)]
    schedule: &'a RawValue,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Provider<'a> {
    version: u16,
    interpretation: &'a str,
    #[serde(borrow)]
    profile: &'a RawValue,
    #[serde(borrow)]
    linked_results: &'a RawValue,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct School<'a> {
    version: u16,
    interpretation: &'a str,
    #[serde(borrow)]
    directory: &'a RawValue,
    #[serde(borrow)]
    roster: &'a RawValue,
}

pub(super) fn select<'a>(
    raw: &'a RawValue,
    schema: Schema,
    slot: Slot,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<&'a RawValue> {
    match schema {
        Schema::ParticipationPacketV1 => {
            let packet: Participation<'a> = parse(raw, grant)?;
            if packet.version != 1 {
                return Err(Error::Corrupt("participation root version").into());
            }
            capture::validate(packet.status, grant)?;
            capture::validate(packet.schedule, grant)?;
            match slot {
                Slot::ParticipationStatus => Ok(packet.status),
                Slot::ParticipationSchedule => Ok(packet.schedule),
                _ => Err(Error::Corrupt("external root capture slot").into()),
            }
        }
        Schema::ProviderTargetPacketV1 => {
            let packet: Provider<'a> = parse(raw, grant)?;
            if packet.version != 1
                || packet.interpretation != "milesplit_profile_linked_competition_target_v1"
            {
                return Err(Error::Corrupt("provider root interpretation").into());
            }
            capture::validate(packet.profile, grant)?;
            capture::validate(packet.linked_results, grant)?;
            match slot {
                Slot::ProviderProfile => Ok(packet.profile),
                Slot::ProviderLinkedResults => Ok(packet.linked_results),
                _ => Err(Error::Corrupt("external root capture slot").into()),
            }
        }
        Schema::ProviderSchoolPacketV1 => school(raw, slot, grant),
        Schema::HtmlResultCaptureV1 => {
            capture::validate(raw, grant)?;
            if slot != Slot::SelfCapture {
                return Err(Error::Corrupt("external root capture slot").into());
            }
            Ok(raw)
        }
    }
}

pub(super) fn parse<'a, T: Deserialize<'a>>(
    raw: &'a RawValue,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<T> {
    grant.charge_steps(1).map_err(Error::Domain)?;
    if !raw.get().trim_start().starts_with('{') {
        return Err(Error::Corrupt("external schema requires a JSON object").into());
    }
    serde_json::from_str(raw.get()).map_err(|source| StoreError::Decode {
        key: "attachment external capture schema".to_owned(),
        source,
    })
}

fn school<'a>(
    raw: &'a RawValue,
    slot: Slot,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<&'a RawValue> {
    let packet: School<'a> = parse(raw, grant)?;
    if packet.version != 1 || packet.interpretation != "milesplit_directory_roster_school_v1" {
        return Err(Error::Corrupt("provider school root interpretation").into());
    }
    capture::validate(packet.directory, grant)?;
    capture::validate(packet.roster, grant)?;
    match slot {
        Slot::ProviderSchoolDirectory => Ok(packet.directory),
        Slot::ProviderSchoolRoster => Ok(packet.roster),
        _ => Err(Error::Corrupt("external root capture slot").into()),
    }
}
