use super::{codec, refused, OPERATION_PREFIX};
use crate::{Receipt, StoreResult};
use census_domain::model::*;
use census_domain::UsJurisdiction;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

mod binding;

pub(crate) const DECLARATION_SCHEMA: &str = "public-resource-capture-declaration-v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicResourceDeclarationBindingV1 {
    pub version: u8,
    pub store: StoreIdentity,
    pub run: CensusRun,
    pub capture_declaration_digest: AcquisitionDigest,
    pub run_manifest_digest: AcquisitionDigest,
    pub selection: ProfileSelectionV1,
    pub national_owner_key: String,
    pub national_request_digest: AcquisitionDigest,
    pub owners: Vec<PublicResourceOwnerBindingV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicResourceOwnerBindingV1 {
    pub owner_key: String,
    pub site: UsJurisdiction,
    pub jurisdiction_request_digest: AcquisitionDigest,
    pub source_plan_fingerprint: AcquisitionDigest,
    pub history: HistoryWindow,
    pub cache_intent: CaptureCacheIntent,
    pub directory_request: RequestRepresentationV1,
    pub robots_request: RequestRepresentationV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PublicResourceDeclarationV1 {
    pub(crate) schema: String,
    pub(crate) binding: PublicResourceDeclarationBindingV1,
    pub(crate) admitted_at: DateTime<Utc>,
}

impl PublicResourceDeclarationV1 {
    pub(crate) fn new(
        binding: PublicResourceDeclarationBindingV1,
        admitted_at: DateTime<Utc>,
    ) -> StoreResult<Self> {
        binding.validate_shape()?;
        Ok(Self {
            schema: DECLARATION_SCHEMA.to_string(),
            binding,
            admitted_at,
        })
    }

    pub(crate) fn digest(&self) -> StoreResult<AcquisitionDigest> {
        codec::object_digest(self)
    }

    pub(crate) fn receipt(&self) -> StoreResult<Receipt> {
        self.validate_shape()?;
        Ok(Receipt {
            operation: format!("{OPERATION_PREFIX}{}", self.binding.plan_digest()?),
            digest: self.digest()?.as_str().to_string(),
            at: self.admitted_at.to_rfc3339(),
            appended: 0,
            credited: false,
        })
    }

    pub(crate) fn validate(&self, receipt: &Receipt) -> StoreResult<()> {
        if receipt != &self.receipt()? {
            return Err(refused("public resource declaration receipt"));
        }
        Ok(())
    }

    pub(crate) fn validate_shape(&self) -> StoreResult<()> {
        if self.schema != DECLARATION_SCHEMA {
            return Err(refused("public resource declaration schema"));
        }
        self.binding.validate_shape()
    }

    pub(crate) fn replay<'a>(
        &self,
        requested: &PublicResourceDeclarationBindingV1,
        receipt: &'a Receipt,
    ) -> StoreResult<&'a Receipt> {
        self.validate(receipt)?;
        if &self.binding != requested {
            return Err(refused("public resource declaration binding conflict"));
        }
        Ok(receipt)
    }
}
