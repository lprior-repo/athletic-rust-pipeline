use super::*;

pub(super) fn city_key(city: &str) -> String {
    city.chars()
        .filter(char::is_ascii_alphanumeric)
        .collect::<String>()
        .to_ascii_lowercase()
}

pub(super) fn same_city(left: &str, right: &str) -> bool {
    left == right
        || left
            .chars()
            .filter(char::is_ascii_alphanumeric)
            .map(|letter| letter.to_ascii_lowercase())
            .eq(right
                .chars()
                .filter(char::is_ascii_alphanumeric)
                .map(|letter| letter.to_ascii_lowercase()))
}

pub(super) fn located_city(city: &str) -> Option<String> {
    let trimmed = city.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalSchool {
    pub id: SchoolId,
    pub name: String,
    pub normalized_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<UsJurisdiction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub association: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub classification: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enrollment: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub school_website: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub athletics_website: Option<String>,
    pub co_op: bool,
    pub aliases: Vec<String>,
    pub source_identities: Vec<SourceIdentity>,
    pub evidence: Vec<Evidence>,
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        deserialize_with = "super::school_address::deserialize_postal_addresses"
    )]
    pub postal_addresses: Vec<SchoolPostalAddress>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub retained_conflicts: Vec<RetainedConflict>,
}

impl CanonicalSchool {
    pub fn mint(
        state: UsJurisdiction,
        name: &str,
        normalized_name: &str,
        city: Option<&str>,
    ) -> SchoolId {
        let _ = name;
        let compressed: String = normalized_name
            .chars()
            .filter(char::is_ascii_alphanumeric)
            .collect();
        match city.map(city_key).filter(|key| !key.is_empty()) {
            Some(city) => Id::mint("sch", &[state.code(), &compressed, &city]),
            None => Id::mint("sch", &[state.code(), &compressed]),
        }
    }

    pub fn new(
        state: UsJurisdiction,
        name: impl Into<String>,
        normalized_name: impl Into<String>,
        city: Option<&str>,
    ) -> (Self, SchoolId) {
        let name = name.into();
        let normalized_name = normalized_name.into();
        let city = city.and_then(located_city);
        let id = CanonicalSchool::mint(state, &name, &normalized_name, city.as_deref());
        (
            Self {
                id: id.clone(),
                name,
                normalized_name,
                city,
                state: Some(state),
                association: None,
                classification: None,
                enrollment: None,
                school_website: None,
                athletics_website: None,
                co_op: false,
                aliases: Vec::new(),
                source_identities: Vec::new(),
                evidence: Vec::new(),
                postal_addresses: Vec::new(),
                retained_conflicts: Vec::new(),
            },
            id,
        )
    }

    pub fn add_postal_address(
        &mut self,
        claim: SchoolPostalAddress,
    ) -> Result<(), SchoolAddressError> {
        claim.belongs_to(self)?;
        if !self.postal_addresses.contains(&claim) {
            self.postal_addresses.push(claim);
            self.postal_addresses.sort_unstable();
        }
        Ok(())
    }
}
