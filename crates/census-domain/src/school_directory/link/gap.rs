use crate::UsJurisdiction;

use super::super::key::IdentifiedKey;
use super::super::name::MatchForm;
use super::attest::corroborated;
use super::forms::{candidate_forms, census_is_middle};
use super::{AttestedRecord, DirectoryIndex};

impl DirectoryIndex {
    pub fn corroboration_gap(
        &self,
        school_name: &str,
        normalized_name: &str,
        city: Option<&str>,
        state: UsJurisdiction,
        aliases: &[String],
        attested: &[AttestedRecord],
    ) -> Option<&'static str> {
        let primary = MatchForm::of(school_name);
        let normalized = MatchForm::of(normalized_name);
        let middle = census_is_middle(&primary) || census_is_middle(&normalized);
        let city_form = city.map(MatchForm::of).filter(|form| !form.is_empty());
        for form in candidate_forms(school_name, &primary, &normalized, aliases) {
            for entry in self.select(&form.form, state, middle) {
                if !matches!(entry.key, IdentifiedKey::Nces(_) | IdentifiedKey::Pss(_))
                    || corroborated(entry, city_form.as_ref(), attested)
                {
                    continue;
                }
                return Some(if city_form.is_none() {
                    "missing_city_corroboration"
                } else if entry.city.is_none() {
                    "missing_published_city"
                } else {
                    "conflicting_city"
                });
            }
        }
        None
    }
}
