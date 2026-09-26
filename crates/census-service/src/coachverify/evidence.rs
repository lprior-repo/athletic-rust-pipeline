use super::{claims, ClaimEvidence, FragmentRow, Verdict};

#[derive(Debug, Clone, Default)]
pub(super) struct RowEvidence {
    pub found: bool,
    pub role_near: bool,
    pub contradicted: bool,
    pub body: bool,
    pub robots: bool,
    pub failed: bool,
    pub script: bool,
    pub claims: Vec<ClaimEvidence>,
    required: usize,
    verified: [bool; 4],
}

const FIELDS: [&str; 4] = ["coach_name", "public_professional_email", "ad_name", "ad_email"];

impl RowEvidence {
    pub fn absorb(&mut self, text: &str, row: &FragmentRow, url: &str, at: &str) -> anyhow::Result<()> {
        self.body = true;
        self.script |= text.to_ascii_lowercase().contains("<script");
        self.required = row.values().iter().filter(|value| !value.trim().is_empty()).count();
        let page = claims::inspect(text, row, url, at)?;
        self.found |= page.found;
        self.contradicted |= page.contradicted;
        page.fields.into_iter().for_each(|claim| {
            if let Some(index) = FIELDS.iter().position(|field| *field == claim.field) {
                self.verified[index] = true;
            }
            self.claims.push(claim);
        });
        self.role_near = eligible_claim(row, at) && self.required > 0
            && self.verified.iter().filter(|value| **value).count() == self.required;
        Ok(())
    }

    pub fn verdict(&self) -> Verdict {
        if self.contradicted { return Verdict::RoleContradicted; }
        if self.required > 0 && self.role_near { return Verdict::Ok; }
        if self.found { return Verdict::OkRoleContext; }
        if !self.body {
            return match (self.robots, self.failed) {
                (true, _) => Verdict::RobotsBlocked,
                (_, true) => Verdict::FetchFailed,
                _ => Verdict::Empty,
            };
        }
        if self.script { Verdict::RenderRequired } else { Verdict::Mismatch }
    }
}

fn eligible_claim(row: &FragmentRow, at: &str) -> bool {
    let role = super::normalize(&row.role);
    let person = (role.contains("coach") && !row.coach_name.trim().is_empty())
        || (role.contains("director") && !row.ad_name.trim().is_empty());
    let claimed = chrono::NaiveDate::parse_from_str(&row.last_observed, "%Y-%m-%d");
    let retrieved = at.get(..10).and_then(|day| chrono::NaiveDate::parse_from_str(day, "%Y-%m-%d").ok());
    person && matches!((claimed, retrieved), (Ok(claimed), Some(retrieved)) if claimed <= retrieved)
}
