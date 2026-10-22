use census_domain::model::{SourceIdentity, SourceNamespace};
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Serialize)]
pub(super) struct ProviderOwners {
    pub(super) provider_id: Option<String>,
    pub(super) provider_ids: BTreeSet<String>,
    pub(super) owner_diagnostic: Option<&'static str>,
}

impl ProviderOwners {
    pub(super) fn new(identities: &[SourceIdentity], namespace: &SourceNamespace) -> Self {
        let mut ids = BTreeSet::new();
        for ident in identities {
            if ident.namespace == *namespace {
                ids.insert(ident.id.clone());
            }
        }
        let provider_id = if ids.len() == 1 {
            ids.first().cloned()
        } else {
            None
        };
        let owner_diagnostic = match ids.len() {
            0 => Some("missing_provider_owner"),
            1 => None,
            _ => Some("ambiguous_provider_owners"),
        };
        ProviderOwners {
            provider_id,
            provider_ids: ids,
            owner_diagnostic,
        }
    }

    pub(super) fn join(&self, program: &Self) -> &'static str {
        match (&self.provider_id, &program.provider_id) {
            (Some(a), Some(b)) if a == b => "same_unique_provider_owner",
            (Some(_), Some(_)) => "different_provider_owners",
            _ => "provider_owner_unresolved",
        }
    }
}
