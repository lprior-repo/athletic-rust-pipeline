use super::super::super::RetainedCaptureRootSchemaV1 as Schema;
use super::{capture, Error, RetainedCaptureSlotV1 as Slot};
use crate::{StoreError, StoreResult};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::roster_affiliation_authority::SourceRowLocator;
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
    result_row: Option<SourceRowLocator>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Window<'a> {
    version: u16,
    #[serde(borrow)]
    window: &'a RawValue,
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

pub(crate) struct CheckedSchool<'a> {
    pub(crate) directory: capture::CheckedCapture<'a>,
    pub(crate) roster: capture::CheckedCapture<'a>,
}

pub(crate) enum CheckedRoot<'a> {
    Participation {
        status: capture::CheckedCapture<'a>,
        schedule: capture::CheckedCapture<'a>,
        result_row: Option<SourceRowLocator>,
    },
    Provider {
        profile: capture::CheckedCapture<'a>,
        linked_results: capture::CheckedCapture<'a>,
    },
    School(CheckedSchool<'a>),
    Html(capture::CheckedCapture<'a>),
    Window(capture::CheckedCapture<'a>),
}

impl CheckedRoot<'_> {
    pub(in crate::evidence_subject_attachment::external) fn select(
        &self,
        slot: Slot,
    ) -> StoreResult<&capture::CheckedCapture<'_>> {
        match (self, slot) {
            (CheckedRoot::Participation { status, .. }, Slot::ParticipationStatus) => Ok(status),
            (CheckedRoot::Participation { schedule, .. }, Slot::ParticipationSchedule) => {
                Ok(schedule)
            }
            (CheckedRoot::Provider { profile, .. }, Slot::ProviderProfile) => Ok(profile),
            (
                CheckedRoot::Provider {
                    profile: _,
                    linked_results,
                },
                Slot::ProviderLinkedResults,
            ) => Ok(linked_results),
            (CheckedRoot::School(school), Slot::ProviderSchoolDirectory) => Ok(&school.directory),
            (CheckedRoot::School(school), Slot::ProviderSchoolRoster) => Ok(&school.roster),
            (CheckedRoot::Html(capture), Slot::SelfCapture) => Ok(capture),
            (CheckedRoot::Window(capture), Slot::ParticipationWindow) => Ok(capture),
            _ => Err(Error::Corrupt("external root capture slot").into()),
        }
    }
}
fn participation_packet<'a>(
    raw: &'a RawValue,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Participation<'a>> {
    let packet: Participation<'a> = parse(raw, grant)?;
    if packet.version != 1 {
        return Err(Error::Corrupt("participation root version").into());
    }
    if let Some(row) = packet.result_row {
        if row.byte_length() == 0 || row.byte_offset().checked_add(row.byte_length()).is_none() {
            return Err(Error::Corrupt("participation result row bounds").into());
        }
    }
    Ok(packet)
}

pub(in crate::evidence_subject_attachment::external) fn checked<'a>(
    raw: &'a RawValue,
    schema: Schema,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<CheckedRoot<'a>> {
    match schema {
        Schema::HtmlResultCaptureV1 => Ok(CheckedRoot::Html(capture::checked(raw, grant)?)),
        Schema::ParticipationWindowPacketV1 => {
            let packet = window_packet(raw, grant)?;
            Ok(CheckedRoot::Window(capture::checked(packet.window, grant)?))
        }
        Schema::ProviderSchoolPacketV1 => {
            let packet = school_packet(raw, grant)?;
            Ok(CheckedRoot::School(CheckedSchool {
                directory: capture::checked(packet.directory, grant)?,
                roster: capture::checked(packet.roster, grant)?,
            }))
        }
        Schema::ParticipationPacketV1 => {
            let packet = participation_packet(raw, grant)?;
            Ok(CheckedRoot::Participation {
                status: capture::checked(packet.status, grant)?,
                schedule: capture::checked(packet.schedule, grant)?,
                result_row: packet.result_row,
            })
        }
        Schema::ProviderTargetPacketV1 => {
            let packet: Provider<'a> = parse(raw, grant)?;
            if packet.version != 1
                || packet.interpretation != "milesplit_profile_linked_competition_target_v1"
            {
                return Err(Error::Corrupt("provider root interpretation").into());
            }
            Ok(CheckedRoot::Provider {
                profile: capture::checked(packet.profile, grant)?,
                linked_results: capture::checked(packet.linked_results, grant)?,
            })
        }
    }
}
pub(super) fn select<'a>(
    raw: &'a RawValue,
    schema: Schema,
    slot: Slot,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<&'a RawValue> {
    match schema {
        Schema::ParticipationPacketV1 => {
            let packet = participation_packet(raw, grant)?;
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
        Schema::ParticipationWindowPacketV1 => {
            let packet = window_packet(raw, grant)?;
            capture::validate(packet.window, grant)?;
            if slot != Slot::ParticipationWindow {
                return Err(Error::Corrupt("external root capture slot").into());
            }
            Ok(packet.window)
        }
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
    let packet = school_packet(raw, grant)?;
    capture::validate(packet.directory, grant)?;
    capture::validate(packet.roster, grant)?;
    match slot {
        Slot::ProviderSchoolDirectory => Ok(packet.directory),
        Slot::ProviderSchoolRoster => Ok(packet.roster),
        _ => Err(Error::Corrupt("external root capture slot").into()),
    }
}

fn school_packet<'a>(
    raw: &'a RawValue,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<School<'a>> {
    let packet: School<'a> = parse(raw, grant)?;
    if packet.version != 1 || packet.interpretation != "milesplit_directory_roster_school_v1" {
        return Err(Error::Corrupt("provider school root interpretation").into());
    }
    Ok(packet)
}

fn window_packet<'a>(
    raw: &'a RawValue,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Window<'a>> {
    let packet: Window<'a> = parse(raw, grant)?;
    if packet.version != 1 {
        return Err(Error::Corrupt("participation window root version").into());
    }
    Ok(packet)
}
