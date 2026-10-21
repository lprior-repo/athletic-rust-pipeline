use super::PublicSourceResourceV1;
use crate::model::ProfileDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicResourceObligationV1 {
    resource: PublicSourceResourceV1,
    as_of: ProfileDate,
}

impl PublicResourceObligationV1 {
    pub fn new(resource: PublicSourceResourceV1, as_of: ProfileDate) -> Self {
        Self { resource, as_of }
    }

    pub fn resource(&self) -> &PublicSourceResourceV1 {
        &self.resource
    }

    pub fn as_of(&self) -> &ProfileDate {
        &self.as_of
    }
}
