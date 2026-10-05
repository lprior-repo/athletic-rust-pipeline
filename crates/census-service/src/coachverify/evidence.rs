use super::{claims, Verdict};
use census_domain::model::{ContactClaimEvidence, RawContactRow};

#[derive(Debug, Clone, Default)]
pub(super) struct RowEvidence {
    pub found: bool,
    pub role_near: bool,
    pub contradicted: bool,
    pub body: bool,
    pub failed: bool,
    pub script: bool,
    pub claims: Vec<ContactClaimEvidence>,
    required: usize,
    verified: [bool; 4],
}

const FIELDS: [&str; 4] = [
    "coach_name",
    "public_professional_email",
    "ad_name",
    "ad_email",
];

impl RowEvidence {
    pub fn absorb(
        &mut self,
        text: &str,
        row: &RawContactRow,
        url: &str,
        fetched_at: &str,
        source_sha256: &str,
    ) -> anyhow::Result<()> {
        self.body = true;
        self.script |= text.to_ascii_lowercase().contains("<script");
        self.required = [
            &row.coach_name,
            &row.public_professional_email,
            &row.ad_name,
            &row.ad_email,
        ]
        .iter()
        .filter(|value| !value.trim().is_empty())
        .count();
        let page = claims::inspect(text, row, url, source_sha256, fetched_at);
        self.found |= page.found;
        self.contradicted |= page.contradicted;
        page.fields.into_iter().for_each(|claim| {
            let field_str = contact_proof_field_str(claim.field);
            if let Some(index) = FIELDS.iter().position(|field| *field == field_str) {
                if let Some(slot) = self.verified.get_mut(index) {
                    *slot = true;
                }
            }
            if !self.claims.contains(&claim) {
                self.claims.push(claim);
            }
        });
        self.role_near = eligible_claim(row, fetched_at)
            && self.required > 0
            && self.verified.iter().filter(|value| **value).count() == self.required;
        Ok(())
    }

    pub fn verdict(&self) -> Verdict {
        if self.contradicted {
            return Verdict::RoleContradicted;
        }
        if self.required > 0 && self.role_near {
            return Verdict::Ok;
        }
        if self.found {
            return Verdict::OkRoleContext;
        }
        if !self.body {
            return if self.failed {
                Verdict::FetchFailed
            } else {
                Verdict::Empty
            };
        }
        if self.script {
            Verdict::RenderRequired
        } else {
            Verdict::Mismatch
        }
    }
}

fn contact_proof_field_str(field: census_domain::model::ContactProofField) -> &'static str {
    match field {
        census_domain::model::ContactProofField::CoachName => "coach_name",
        census_domain::model::ContactProofField::PublicProfessionalEmail => {
            "public_professional_email"
        }
        census_domain::model::ContactProofField::AdName => "ad_name",
        census_domain::model::ContactProofField::AdEmail => "ad_email",
    }
}

fn eligible_claim(row: &RawContactRow, at: &str) -> bool {
    let role = super::normalize(&row.role);
    let person = (role.contains("coach") && !row.coach_name.trim().is_empty())
        || (role.contains("director") && !row.ad_name.trim().is_empty());
    let claimed = chrono::NaiveDate::parse_from_str(&row.last_observed, "%Y-%m-%d");
    let retrieved = at
        .get(..10)
        .and_then(|day| chrono::NaiveDate::parse_from_str(day, "%Y-%m-%d").ok());
    person && matches!((claimed, retrieved), (Ok(claimed), Some(retrieved)) if claimed <= retrieved)
}
