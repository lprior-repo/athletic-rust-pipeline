use crate::report::{ReportError, ReportResult};
use census_domain::model::QueryProfileV1;
use census_domain::public_identity_history::RegistryHead;
use census_domain::public_identity_qualification::PublicIdentityQualificationVersion;
use census_store::{AcceptedPublicIdentitySnapshot, StoreSnapshot, Table};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const TABLES: [Table; 5] = [
    Table::Athletes,
    Table::Performances,
    Table::Teams,
    Table::Meets,
    Table::Events,
];

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SelectedAuthority {
    profile: QueryProfileV1,
    fingerprints: BTreeMap<String, String>,
    identity_sequence: u64,
    identity_digest: String,
    qualification_lifecycle: u32,
}

impl SelectedAuthority {
    pub(super) fn capture(
        profile: QueryProfileV1,
        snapshot: &StoreSnapshot<'_>,
        accepted: &AcceptedPublicIdentitySnapshot,
    ) -> ReportResult<Self> {
        require_current(accepted)?;
        let fingerprints = TABLES
            .into_iter()
            .map(|table| Ok((table.file().to_owned(), snapshot.tables_digest(&[table])?)))
            .collect::<ReportResult<BTreeMap<_, _>>>()?;
        Ok(Self {
            profile,
            fingerprints,
            identity_sequence: accepted.head().sequence,
            identity_digest: accepted.head().digest.clone(),
            qualification_lifecycle: accepted.qualification_version().lifecycle_revision(),
        })
    }

    pub(crate) fn qualification_lifecycle(&self) -> u32 {
        self.qualification_lifecycle
    }

    pub(crate) fn profile(&self) -> &QueryProfileV1 {
        &self.profile
    }
    pub(crate) fn fingerprints(&self) -> &BTreeMap<String, String> {
        &self.fingerprints
    }
    pub(crate) fn registry_head(&self) -> RegistryHead {
        RegistryHead {
            sequence: self.identity_sequence,
            digest: self.identity_digest.clone(),
        }
    }

    pub(crate) fn ensure_identity(
        &self,
        accepted: &AcceptedPublicIdentitySnapshot,
    ) -> ReportResult<()> {
        require_current(accepted)?;
        self.ensure_binding(accepted)
    }

    pub(crate) fn ensure_historical_identity(
        &self,
        accepted: &AcceptedPublicIdentitySnapshot,
    ) -> ReportResult<()> {
        if accepted.qualification_version()
            != PublicIdentityQualificationVersion::RetainedAuthorityContextV2
        {
            return Err(invalid(
                "historical selected identity requires qualification V2",
            ));
        }
        self.ensure_binding(accepted)
    }

    fn ensure_binding(&self, accepted: &AcceptedPublicIdentitySnapshot) -> ReportResult<()> {
        if accepted.head() != &self.registry_head()
            || accepted.qualification_version().lifecycle_revision() != self.qualification_lifecycle
        {
            return Err(invalid("selected public identity authority changed"));
        }
        Ok(())
    }

    pub(crate) fn validate(&self) -> ReportResult<()> {
        if !matches!(self.qualification_lifecycle, 6 | 7)
            || !digest(&self.identity_digest)
            || self.fingerprints.len() != TABLES.len()
            || TABLES.iter().any(|table| {
                self.fingerprints
                    .get(table.file())
                    .is_none_or(|value| !digest(value))
            })
        {
            return Err(invalid("selected authority inventory is malformed"));
        }
        self.profile.id().map_err(census_store::StoreError::from)?;
        Ok(())
    }
}

fn require_current(accepted: &AcceptedPublicIdentitySnapshot) -> ReportResult<()> {
    if accepted.qualification_version()
        != PublicIdentityQualificationVersion::RetainedAuthorityGraphV3
    {
        return Err(invalid(
            "selected publication requires retained public identity V3",
        ));
    }
    Ok(())
}

fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn invalid(detail: &str) -> ReportError {
    ReportError::Invariant {
        detail: detail.into(),
    }
}
