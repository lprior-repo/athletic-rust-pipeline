use super::{codec, encode, refused, Entry, INTERPRETATION, MAX_BYTES};
use crate::acquisition::public_resources::{
    intake, PublicResourceDeclarationV1, PublicSchoolIntakeCaptureV1,
};
use crate::{Receipt, StoreResult};
use census_domain::model::{AcquisitionDigest, PublicSourceResourceKindV1, PublicSourceResourceV1};
use census_domain::UsJurisdiction;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicProfileCapturePacketV1 {
    version: u8,
    interpretation: String,
    pub(super) declaration: PublicResourceDeclarationV1,
    declaration_receipt: Receipt,
    pub(super) child: Entry,
    child_receipt: Receipt,
    capture: PublicSchoolIntakeCaptureV1,
}

impl PublicProfileCapturePacketV1 {
    pub(super) fn new(
        declaration: PublicResourceDeclarationV1,
        child: Entry,
        child_receipt: Receipt,
        capture: PublicSchoolIntakeCaptureV1,
    ) -> StoreResult<Self> {
        let declaration_receipt = declaration.receipt()?;
        let packet = Self {
            version: 1,
            interpretation: INTERPRETATION.to_owned(),
            declaration,
            declaration_receipt,
            child,
            child_receipt,
            capture,
        };
        packet.validate()?;
        packet.encode()?;
        Ok(packet)
    }

    pub fn decode(bytes: &[u8]) -> StoreResult<Self> {
        if bytes.is_empty() || bytes.len() > MAX_BYTES {
            return Err(refused("public profile capture packet size"));
        }
        let packet: Self = serde_json::from_slice(bytes)
            .map_err(|_| refused("public profile capture packet decoding"))?;
        packet.validate()?;
        if packet.encode()? != bytes {
            return Err(refused("public profile capture packet is not canonical"));
        }
        Ok(packet)
    }

    pub fn encode(&self) -> StoreResult<Vec<u8>> {
        encode::packet(self)
    }

    pub fn reference(&self) -> StoreResult<AcquisitionDigest> {
        codec::digest(&self.encode()?)
    }

    pub fn capture(&self) -> &PublicSchoolIntakeCaptureV1 {
        &self.capture
    }

    pub fn athlete_id(&self) -> StoreResult<&str> {
        self.child
            .work
            .slot()
            .public_resource()
            .and_then(PublicSourceResourceV1::athlete_id)
            .ok_or_else(|| refused("public profile capture athlete identity absent"))
    }

    pub fn site(&self) -> UsJurisdiction {
        self.child.work.slot().source_site()
    }

    pub fn roster_reference(&self) -> &crate::PublicSchoolIntakeRefV1 {
        &self.child.packet
    }

    pub(super) fn validate(&self) -> StoreResult<()> {
        if self.version != 1 || self.interpretation != INTERPRETATION {
            return Err(refused("public profile capture interpretation differs"));
        }
        self.declaration.validate(&self.declaration_receipt)?;
        self.child
            .require_receipt(&self.declaration, &self.child_receipt)?;
        intake::startup::validate_capture(&self.capture)?;
        let resource = self.child.work.slot().public_resource();
        if self.child.work != *self.capture.work()
            || resource.map(PublicSourceResourceV1::kind)
                != Some(PublicSourceResourceKindV1::MileSplitAthleteProfile)
            || self.capture.work().slot().store_identity() != &self.declaration.binding.store
        {
            return Err(refused("public profile capture work or owner differs"));
        }
        Ok(())
    }
}
