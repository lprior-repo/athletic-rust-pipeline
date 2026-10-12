use super::codec::{check_inventory, decode, encode};
use super::model::Inventory;
use super::publish::Digest;
use crate::{Store, StoreError, StoreResult};
use sha2::{Digest as _, Sha256};

const PAGE_CHILDREN: usize = 64;
#[derive(serde::Serialize, serde::Deserialize)]
struct Page<T> {
    at: String,
    payload: T,
}

impl Inventory {
    pub(super) fn new(children: &[String]) -> StoreResult<Self> {
        check_inventory(children)?;
        let mut hash = Sha256::new();
        children
            .iter()
            .for_each(|child| hash.update(child.as_bytes()));
        Ok(Self {
            root: Digest::parse(&format!("{:x}", hash.finalize()))?,
            count: children.len(),
            children: children.to_vec(),
        })
    }
    pub(super) fn validate(&self) -> StoreResult<()> {
        check_inventory(&self.children)?;
        let mut hash = Sha256::new();
        self.children
            .iter()
            .for_each(|child| hash.update(child.as_bytes()));
        if self.count != self.children.len()
            || self.root.as_str() != format!("{:x}", hash.finalize())
        {
            return Err(StoreError::CorruptArtifact {
                digest: self.root.as_str().into(),
            });
        }
        Ok(())
    }
}
fn page_key(root: &Digest, page: usize) -> String {
    format!("record-inventory-v1\0{}/{page:04x}", root.as_str())
}
impl Store {
    pub(super) fn inventory_write(
        &self,
        inventory: &Inventory,
        batch: &mut fjall::OwnedWriteBatch,
    ) -> StoreResult<()> {
        inventory.validate()?;
        inventory
            .children
            .chunks(PAGE_CHILDREN)
            .enumerate()
            .try_for_each(|(page, children)| {
                let value = Page {
                    at: chrono::Utc::now().format("%Y-%m-%d").to_string(),
                    payload: children,
                };
                batch.insert(
                    &self.journal,
                    page_key(&inventory.root, page).as_bytes(),
                    encode(&value)?,
                );
                Ok(())
            })
    }
    pub(super) fn inventory_read(&self, inventory: &mut Inventory) -> StoreResult<()> {
        if inventory.count > super::codec::MAX_INVENTORY {
            return Err(StoreError::BudgetExceeded {
                what: "inventory children",
                needed: u64::try_from(inventory.count).map_err(|_| StoreError::CounterExhausted)?,
                available: 4096,
            });
        }
        let pages = inventory.count.div_ceil(PAGE_CHILDREN);
        inventory
            .children
            .try_reserve_exact(inventory.count)
            .map_err(|_| StoreError::BudgetExceeded {
                what: "inventory allocation",
                needed: 4096,
                available: 0,
            })?;
        (0..pages).try_for_each(|page| {
            let key = page_key(&inventory.root, page);
            let raw = self
                .journal
                .get(key.as_bytes())
                .map_err(|source| StoreError::Read { source })?
                .ok_or_else(|| StoreError::CorruptArtifact {
                    digest: inventory.root.as_str().into(),
                })?;
            let Page {
                payload: children, ..
            }: Page<Vec<String>> = decode(&key, &raw)?;
            let remaining = inventory
                .count
                .checked_sub(inventory.children.len())
                .ok_or(StoreError::CounterExhausted)?;
            if children.len() != remaining.min(PAGE_CHILDREN) {
                return Err(StoreError::CorruptArtifact {
                    digest: inventory.root.as_str().into(),
                });
            }
            inventory.children.extend(children);
            Ok(())
        })?;
        inventory.validate()
    }
}
