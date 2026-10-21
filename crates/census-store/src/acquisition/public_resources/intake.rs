use super::{codec, refused};
use crate::{PublicResourceCaptureOwnerV1, Receipt, StoreResult};
use census_domain::model::{AcquisitionDigest, AcquisitionOccurrenceRefV1};
use serde::{Deserialize, Serialize};

mod admission;
mod capture;
mod packet;
mod read;
pub(crate) mod startup;
mod write;

pub(crate) use admission::read as read_admission;
pub use admission::PublicSchoolIntakeAdmissionV1;
pub use capture::PublicSchoolIntakeCaptureV1;
pub use packet::{PublicSchoolIntakeChildEvidenceV1, PublicSchoolIntakePacketV1};

pub(crate) const PHASE: &str = "public_resource_school_intake_v1";
pub(crate) const MINT_PREFIX: &str = "public_resource_school_intake/v1/mint/";
pub(crate) const OPERATION_PREFIX: &str = "public_resource_school_intake_v1:";
const INTERPRETATION: &str = "milesplit_owned_directory_roster_school_v1";
const MAX_PACKET: usize = crate::evidence_subject_attachment::MAX_RETAINED_CAPTURE_ENVELOPE_BYTES;
const MAX_BODY: usize = 2 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PublicSchoolIntakeRefV1(AcquisitionDigest);

impl PublicSchoolIntakeRefV1 {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

pub struct PreparedPublicSchoolIntakeV1 {
    owner: PublicResourceCaptureOwnerV1,
    packet: PublicSchoolIntakePacketV1,
}

impl PreparedPublicSchoolIntakeV1 {
    pub fn packet(&self) -> &PublicSchoolIntakePacketV1 {
        &self.packet
    }
    pub fn reference(&self) -> PublicSchoolIntakeRefV1 {
        self.packet.reference()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Mint {
    version: u8,
    packet: PublicSchoolIntakeRefV1,
    packet_bytes: usize,
    store: census_domain::model::StoreIdentity,
    directory: AcquisitionOccurrenceRefV1,
    roster: AcquisitionOccurrenceRefV1,
}

impl Mint {
    fn from_packet(packet: &PublicSchoolIntakePacketV1) -> StoreResult<Self> {
        Ok(Self {
            version: 1,
            packet: packet.reference(),
            packet_bytes: packet.encode()?.len(),
            store: packet.directory().work().slot().store_identity().clone(),
            directory: packet.directory().occurrence().clone(),
            roster: packet.roster().occurrence().clone(),
        })
    }

    fn key(&self) -> String {
        format!("{MINT_PREFIX}{}", self.packet.as_str())
    }

    fn validate(&self, packet: &PublicSchoolIntakePacketV1, receipt: &Receipt) -> StoreResult<()> {
        if self != &Self::from_packet(packet)?
            || receipt.operation != format!("{OPERATION_PREFIX}{}", self.packet.as_str())
            || receipt.digest != codec::object_digest(self)?.as_str()
            || receipt.appended != 0
            || receipt.credited
        {
            return Err(refused("public school intake mint differs"));
        }
        let at = chrono::DateTime::parse_from_rfc3339(&receipt.at)
            .map_err(|_| refused("public school intake mint time"))?;
        for capture in [packet.directory(), packet.roster()] {
            let retained = terminal_day(&capture.terminal_receipt)?;
            if at.with_timezone(&chrono::Utc).date_naive() < retained {
                return Err(refused("public school intake mint precedes original"));
            }
        }
        Ok(())
    }
}

fn terminal_day(receipt: &Receipt) -> StoreResult<chrono::NaiveDate> {
    let day = chrono::NaiveDate::parse_from_str(&receipt.at, "%Y-%m-%d")
        .map_err(|_| refused("public school intake terminal receipt day"))?;
    if day.to_string() != receipt.at {
        return Err(refused("public school intake terminal receipt day"));
    }
    Ok(day)
}
