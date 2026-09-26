use super::*;

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
    /// Canonical-id collisions this row's merge retained: another natural key minted this id, so the
    /// row below is the one that survived and the other subject's facts were not absorbed. Empty on
    /// every row whose fields still state the id they minted, which is every row until one collides.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub retained_conflicts: Vec<RetainedConflict>,
}

impl CanonicalSchool {
    /// Build a school from its natural key (jurisdiction + normalized name) so that id minting is
    /// deterministic and identical no matter which adapter saw the school first.
    ///
    /// The key carries the jurisdiction's USPS code — byte-identical to the uppercase state string
    /// this parameter used to hold — so typing the parameter re-mints no school id.
    pub fn mint(state: UsJurisdiction, name: &str, normalized_name: &str) -> SchoolId {
        let _ = name;
        let compressed: String = normalized_name
            .chars()
            .filter(char::is_ascii_alphanumeric)
            .collect();
        Id::mint("sch", &[state.code(), &compressed])
    }

    pub fn new(
        state: UsJurisdiction,
        name: impl Into<String>,
        normalized_name: impl Into<String>,
    ) -> (Self, SchoolId) {
        let name = name.into();
        let normalized_name = normalized_name.into();
        let id = CanonicalSchool::mint(state, &name, &normalized_name);
        (
            Self {
                id: id.clone(),
                name,
                normalized_name,
                city: None,
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
                retained_conflicts: Vec::new(),
            },
            id,
        )
    }
}
