use census_domain::model::{ContactClaimEvidence, ContactProofField, RawContactRow};
use std::collections::HashMap;

use super::limits;

type Binding<'a> = [&'a str; 9];

pub struct ContactEvidenceIndex<'a> {
    claims: &'a [ContactClaimEvidence],
    bindings: HashMap<Binding<'a>, Vec<usize>>,
}

impl<'a> ContactEvidenceIndex<'a> {
    pub fn build(claims: &'a [ContactClaimEvidence]) -> anyhow::Result<Self> {
        limits::check("contact claims", claims.len(), limits::MAX_ROWS)?;
        let mut bytes = 0usize;
        for claim in claims {
            bytes = bytes
                .checked_add(limits::claim_bytes(claim)?)
                .ok_or_else(|| anyhow::anyhow!("contact evidence byte count overflow"))?;
            limits::check("contact evidence bytes", bytes, limits::MAX_BYTES)?;
        }
        let mut bindings: HashMap<Binding<'a>, Vec<usize>> = HashMap::new();
        bindings.try_reserve(claims.len())?;
        for (position, claim) in claims.iter().enumerate() {
            let positions = bindings.entry(claim_binding(claim)).or_default();
            positions.try_reserve(1)?;
            positions.push(position);
        }
        Ok(Self { claims, bindings })
    }

    pub fn claims_for_row(&self, row: &RawContactRow) -> anyhow::Result<Vec<ContactClaimEvidence>> {
        limits::row_bytes(row)?;
        let mut urls = std::collections::HashSet::new();
        urls.try_reserve(row.source_urls.len())?;
        urls.extend(row.source_urls.iter().map(String::as_str));
        let mut positions = Vec::new();
        for field in [
            ContactProofField::CoachName,
            ContactProofField::PublicProfessionalEmail,
            ContactProofField::AdName,
            ContactProofField::AdEmail,
        ] {
            for url in &urls {
                let key = row_binding(row, field, url, &row.sport);
                self.append(&mut positions, &key)?;
                if census_domain::model::is_director(field) && !row.sport.is_empty() {
                    self.append(&mut positions, &row_binding(row, field, url, ""))?;
                }
            }
        }
        positions.sort_unstable();
        positions.dedup();
        let mut covering = Vec::new();
        covering.try_reserve(positions.len())?;
        for position in positions {
            let claim = self.claims.get(position).ok_or_else(|| {
                anyhow::anyhow!(
                    "contact index position {position} outside {}",
                    self.claims.len()
                )
            })?;
            #[cfg(test)]
            super::output::metrics::comparison();
            if census_domain::model::claim_binds_to_row(row, claim) {
                covering.push(claim.clone());
            }
        }
        Ok(covering)
    }

    fn append(&self, positions: &mut Vec<usize>, key: &Binding<'_>) -> anyhow::Result<()> {
        if let Some(matches) = self.bindings.get(key) {
            let count = positions
                .len()
                .checked_add(matches.len())
                .ok_or_else(|| anyhow::anyhow!("contact row claim count overflow"))?;
            limits::check("contact row claims", count, 256)?;
            positions.try_reserve(matches.len())?;
            positions.extend(matches);
        }
        Ok(())
    }
}

fn field_name(field: ContactProofField) -> &'static str {
    match field {
        ContactProofField::CoachName => "coach_name",
        ContactProofField::PublicProfessionalEmail => "public_professional_email",
        ContactProofField::AdName => "ad_name",
        ContactProofField::AdEmail => "ad_email",
    }
}

fn claim_binding(claim: &ContactClaimEvidence) -> Binding<'_> {
    [
        field_name(claim.field),
        &claim.value,
        &claim.person,
        &claim.role,
        &claim.sport,
        &claim.school,
        &claim.state,
        &claim.claimed_observed_on,
        &claim.source_url,
    ]
}

fn row_binding<'a>(
    row: &'a RawContactRow,
    field: ContactProofField,
    url: &'a str,
    sport: &'a str,
) -> Binding<'a> {
    let (value, person, role) = match field {
        ContactProofField::CoachName => (&row.coach_name, &row.coach_name, row.role.as_str()),
        ContactProofField::PublicProfessionalEmail => (
            &row.public_professional_email,
            &row.coach_name,
            row.role.as_str(),
        ),
        ContactProofField::AdName => (&row.ad_name, &row.ad_name, "Athletic Director"),
        ContactProofField::AdEmail => (&row.ad_email, &row.ad_name, "Athletic Director"),
    };
    [
        field_name(field),
        value,
        person,
        role,
        sport,
        &row.school,
        &row.state,
        &row.last_observed,
        url,
    ]
}
