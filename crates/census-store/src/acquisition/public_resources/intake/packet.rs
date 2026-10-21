use super::{codec, refused, PublicSchoolIntakeCaptureV1, PublicSchoolIntakeRefV1};
use super::{INTERPRETATION, MAX_PACKET};
use crate::acquisition::public_resources::{child::Entry, PublicResourceDeclarationV1};
use crate::acquisition::public_resources::{
    format, PublicResourceFrontierEntryV1, PublicResourceRootKindV1,
};
use crate::{Receipt, StoreResult};
use census_domain::model::AcquisitionDigest;
use census_domain::roster_affiliation_authority::SourceRowLocator;
use serde::{Deserialize, Serialize};

mod encode;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Wire {
    pub(super) version: u8,
    pub(super) interpretation: String,
    pub(super) declaration: PublicResourceDeclarationV1,
    pub(super) declaration_receipt: Receipt,
    pub(super) directory_frontier: PublicResourceFrontierEntryV1,
    pub(super) directory_receipt: Receipt,
    pub(super) child: Entry,
    pub(super) child_receipt: Receipt,
    pub(super) directory: PublicSchoolIntakeCaptureV1,
    pub(super) roster: PublicSchoolIntakeCaptureV1,
}

#[derive(Debug, Clone)]
pub struct PublicSchoolIntakePacketV1 {
    pub(super) wire: Wire,
    reference: PublicSchoolIntakeRefV1,
}

impl Serialize for PublicSchoolIntakePacketV1 {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.wire.serialize(serializer)
    }
}

impl PublicSchoolIntakePacketV1 {
    pub fn decode(bytes: &[u8]) -> StoreResult<Self> {
        if bytes.is_empty() || bytes.len() > MAX_PACKET {
            return Err(refused("public school intake packet bytes"));
        }
        let wire = serde_json::from_slice(bytes)
            .map_err(|_| refused("public school intake packet decoding"))?;
        let packet = Self::from_wire(wire)?;
        if packet.encode()? != bytes {
            return Err(refused("public school intake packet is not canonical"));
        }
        Ok(packet)
    }

    pub fn encode(&self) -> StoreResult<Vec<u8>> {
        encode::wire(&self.wire)
    }
    pub fn reference(&self) -> PublicSchoolIntakeRefV1 {
        self.reference.clone()
    }
    pub fn directory(&self) -> &PublicSchoolIntakeCaptureV1 {
        &self.wire.directory
    }
    pub fn roster(&self) -> &PublicSchoolIntakeCaptureV1 {
        &self.wire.roster
    }
    pub fn child_team_id(&self) -> &str {
        &self.wire.child.team_id
    }
    pub fn child_evidence(&self) -> PublicSchoolIntakeChildEvidenceV1<'_> {
        PublicSchoolIntakeChildEvidenceV1 { packet: self }
    }

    pub(super) fn from_wire(wire: Wire) -> StoreResult<Self> {
        wire.validate()?;
        let reference = PublicSchoolIntakeRefV1(codec::digest(&encode::wire(&wire)?)?);
        Ok(Self { wire, reference })
    }
}

impl Wire {
    fn validate(&self) -> StoreResult<()> {
        if self.version != 1 || self.interpretation != INTERPRETATION {
            return Err(refused("public school intake interpretation"));
        }
        self.declaration.validate(&self.declaration_receipt)?;
        self.directory_frontier.validate(&self.declaration)?;
        format::validate_entry_receipt(
            &self.declaration,
            &self.directory_frontier,
            &self.directory_receipt,
        )?;
        self.child
            .validate_receipt(&self.declaration, &self.child_receipt)?;
        self.directory.validate()?;
        self.roster.validate()?;
        self.validate_chronology()?;
        self.validate_pair()
    }

    fn validate_pair(&self) -> StoreResult<()> {
        let directory = &self.directory;
        let roster = &self.roster;
        let child = &self.child;
        if self.directory_frontier.kind != PublicResourceRootKindV1::MileSplitTeamDirectory
            || self.directory_frontier.work != directory.work
            || child.work != roster.work
            || child.parent != *directory.occurrence()
            || child.body_digest != *directory.prepared().content_digest()
            || child.body_bytes != directory.prepared().content_bytes()
            || child.metadata_digest != *directory.prepared().metadata_digest()
            || directory.work.slot().store_identity() != &self.declaration.binding.store
            || roster.work.slot().store_identity() != &self.declaration.binding.store
            || directory.work.slot().owner_key() != roster.work.slot().owner_key()
            || directory.work.slot().source_site() != roster.work.slot().source_site()
        {
            return Err(refused("public school intake original pair differs"));
        }
        let start = child.row.byte_offset();
        let end = start
            .checked_add(child.row.byte_length())
            .ok_or(crate::StoreError::CounterOverflow)?;
        let row = directory
            .body()
            .get(start..end)
            .ok_or_else(|| refused("public school intake directory row bounds"))?;
        if codec::digest(row)? != child.row_digest {
            return Err(refused("public school intake directory row digest"));
        }
        Ok(())
    }

    fn validate_chronology(&self) -> StoreResult<()> {
        let parse = |receipt: &Receipt| {
            chrono::DateTime::parse_from_rfc3339(&receipt.at)
                .map(|instant| instant.with_timezone(&chrono::Utc).date_naive())
                .map_err(|_| refused("public school intake source receipt time"))
        };
        let directory = super::terminal_day(&self.directory.terminal_receipt)?;
        let child = parse(&self.child_receipt)?;
        let roster = super::terminal_day(&self.roster.terminal_receipt)?;
        if parse(&self.directory_receipt)? > directory || directory > child || child > roster {
            return Err(refused("public school intake source receipt chronology"));
        }
        Ok(())
    }
}

pub struct PublicSchoolIntakeChildEvidenceV1<'a> {
    packet: &'a PublicSchoolIntakePacketV1,
}

impl PublicSchoolIntakeChildEvidenceV1<'_> {
    pub fn team_id(&self) -> &str {
        &self.packet.wire.child.team_id
    }
    pub fn team_url(&self) -> &str {
        &self.packet.wire.child.team_url
    }
    pub fn roster_url(&self) -> &str {
        self.packet.roster().work().request().original_url()
    }
    pub fn parsed_teams(&self) -> usize {
        self.packet.wire.child.parsed_teams
    }
    pub fn row(&self) -> SourceRowLocator {
        self.packet.wire.child.row
    }
    pub fn name(&self) -> SourceRowLocator {
        self.packet.wire.child.name
    }
    pub fn city(&self) -> SourceRowLocator {
        self.packet.wire.child.city
    }
    pub fn anchor(&self) -> SourceRowLocator {
        self.packet.wire.child.anchor
    }
    pub fn row_digest(&self) -> &AcquisitionDigest {
        &self.packet.wire.child.row_digest
    }
    pub fn kernel_digest(&self) -> &AcquisitionDigest {
        &self.packet.wire.child.kernel_digest
    }
}
