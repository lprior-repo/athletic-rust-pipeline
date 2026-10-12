use super::*;
use crate::records::codec::check_digest;
use crate::StoreError;
fn validate_revision(revision: u64, epoch: Option<u64>) -> StoreResult<()> {
    if revision == 0 || epoch == Some(0) {
        return Err(StoreError::Refused {
            detail: "record revision and lease epoch must be nonzero".into(),
        });
    }
    Ok(())
}
impl InputRecord {
    pub fn new(digest: &str, bytes: u64, chunks: &[String]) -> StoreResult<Self> {
        Ok(Self {
            digest: Digest::parse(digest)?,
            state: InputState::Requested,
            revision: 1,
            lease_epoch: None,
            bytes,
            inventory: Inventory::new(chunks)?,
        })
    }
    pub fn bytes(&self) -> u64 {
        self.bytes
    }
    pub fn chunks(&self) -> &[String] {
        &self.inventory.children
    }
    pub(super) fn validate_fields(&self) -> StoreResult<()> {
        check_digest(self.digest.as_str())?;
        self.inventory.validate()?;
        validate_revision(self.revision, self.lease_epoch)
    }
    pub(super) fn is_initial(&self) -> bool {
        self.state == InputState::Requested && self.revision == 1 && self.lease_epoch.is_none()
    }
    pub(super) fn child_inventory(&self) -> Option<&Inventory> {
        Some(&self.inventory)
    }
    pub(super) fn child_inventory_mut(&mut self) -> Option<&mut Inventory> {
        Some(&mut self.inventory)
    }
}
impl BuildRecord {
    pub fn new(digest: &str, input_digest: &str) -> StoreResult<Self> {
        Self::with_inventory(digest, input_digest, &[])
    }
    pub fn with_inventory(
        digest: &str,
        input_digest: &str,
        required: &[String],
    ) -> StoreResult<Self> {
        Ok(Self {
            digest: Digest::parse(digest)?,
            state: BuildState::Frozen,
            revision: 1,
            input_digest: Digest::parse(input_digest)?,
            lease_epoch: None,
            inventory: Inventory::new(required)?,
        })
    }
    pub fn input_digest(&self) -> &str {
        self.input_digest.as_str()
    }
    pub fn required_units(&self) -> &[String] {
        &self.inventory.children
    }
    pub(super) fn validate_fields(&self) -> StoreResult<()> {
        self.inventory.validate()?;
        validate_revision(self.revision, self.lease_epoch)
    }
    pub(super) fn is_initial(&self) -> bool {
        self.state == BuildState::Frozen && self.revision == 1 && self.lease_epoch.is_none()
    }
    pub(super) fn child_inventory(&self) -> Option<&Inventory> {
        Some(&self.inventory)
    }
    pub(super) fn child_inventory_mut(&mut self) -> Option<&mut Inventory> {
        Some(&mut self.inventory)
    }
}
impl UnitRecord {
    pub fn new(digest: &str, input_digest: &str, build_digest: &str) -> StoreResult<Self> {
        Ok(Self {
            digest: Digest::parse(digest)?,
            state: UnitState::Pending,
            revision: 1,
            input_digest: Digest::parse(input_digest)?,
            build_digest: Digest::parse(build_digest)?,
            lease_epoch: None,
        })
    }
    pub fn input_digest(&self) -> &str {
        self.input_digest.as_str()
    }
    pub fn build_digest(&self) -> &str {
        self.build_digest.as_str()
    }
    pub(super) fn validate_fields(&self) -> StoreResult<()> {
        validate_revision(self.revision, self.lease_epoch)
    }
    pub(super) fn is_initial(&self) -> bool {
        self.state == UnitState::Pending && self.revision == 1 && self.lease_epoch.is_none()
    }
    pub(super) fn child_inventory(&self) -> Option<&Inventory> {
        None
    }
    pub(super) fn child_inventory_mut(&mut self) -> Option<&mut Inventory> {
        None
    }
}
impl PublishRecord {
    pub fn new(digest: &str, unit_digest: &str) -> StoreResult<Self> {
        Ok(Self {
            digest: Digest::parse(digest)?,
            state: PublishState::Intent,
            revision: 1,
            unit_digest: Digest::parse(unit_digest)?,
            lease_epoch: None,
        })
    }
    pub fn unit_digest(&self) -> &str {
        self.unit_digest.as_str()
    }
    pub(super) fn validate_fields(&self) -> StoreResult<()> {
        validate_revision(self.revision, self.lease_epoch)
    }
    pub(super) fn is_initial(&self) -> bool {
        self.state == PublishState::Intent && self.revision == 1 && self.lease_epoch.is_none()
    }
    pub(super) fn child_inventory(&self) -> Option<&Inventory> {
        None
    }
    pub(super) fn child_inventory_mut(&mut self) -> Option<&mut Inventory> {
        None
    }
}
impl LeaseRecord {
    pub fn owner(&self) -> &str {
        &self.owner
    }
    pub fn expires_at(&self) -> &str {
        &self.expires_at
    }
    pub fn epoch(&self) -> u64 {
        self.epoch
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn object_root(&self) -> &Digest {
        &self.object_root
    }
    pub fn owner_boot(&self) -> &Digest {
        &self.owner_boot
    }
    pub fn owner_task(&self) -> &Digest {
        &self.owner_task
    }
    pub fn mode(&self) -> LeaseMode {
        self.mode
    }
    pub fn reserved_bytes(&self) -> u64 {
        self.reserved_bytes
    }
}
