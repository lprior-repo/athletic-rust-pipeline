use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalCoach {
    pub id: CoachId,
    pub name: String,
    pub school: SchoolId,
    /// `None` for school-wide roles (athletic director) that are not bound to a single sport.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sport: Option<Sport>,
    pub gender: Gender,
    pub role: CoachRole,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub professional_email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    pub source_identities: Vec<SourceIdentity>,
    pub evidence: Vec<Evidence>,
    /// Set when the merge dropped a consumer mailbox from `professional_email`.
    ///
    /// A withheld row ships the key, so a consumer reading a consolidated entity file can tell "no
    /// mailbox was ever observed" from "one was observed and withheld" instead of reading the second
    /// as the first. The key is never omitted, as it is not for a meet's unplaced state.
    ///
    /// On the way back in the flag is trusted only where nothing is left to derive it from: a row that
    /// still carries a mailbox has the flag re-derived from that mailbox on the next publish, and a row
    /// whose mailbox was dropped keeps what the file says. A row written before this key existed
    /// decodes to `false`.
    #[serde(default)]
    pub email_withheld: bool,
    /// Canonical-id collisions this row's merge retained: another natural key minted this id, so the
    /// row below is the one that survived and the other subject's facts were not absorbed. Empty on
    /// every row whose fields still state the id they minted, which is every row until one collides.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub retained_conflicts: Vec<RetainedConflict>,
}

impl CanonicalCoach {
    pub fn new(
        school: &SchoolId,
        name: impl Into<String>,
        sport: Option<Sport>,
        gender: Gender,
        role: CoachRole,
    ) -> Self {
        let name = name.into();
        // `Some(Sport)`/`None` is the byte shape the `Debug` this replaced printed for an
        // `Option<Sport>`, so a coach id minted before the key was typed keeps its id.
        let sport_key = match sport {
            Some(sport) => format!("Some({})", sport.stable_key()),
            None => "None".to_string(),
        };
        let id = Id::mint(
            "coa",
            &[
                school.as_str(),
                &normalize_name(&name),
                &sport_key,
                gender.stable_key(),
                role.stable_key(),
            ],
        );
        Self {
            id,
            name,
            school: school.clone(),
            sport,
            gender,
            role,
            professional_email: None,
            phone: None,
            source_identities: Vec::new(),
            evidence: Vec::new(),
            email_withheld: false,
            retained_conflicts: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoachRole {
    HeadCoach,
    AssistantCoach,
    AthleticDirector,
    Unknown,
}

impl CoachRole {
    /// The byte spelling of this role inside a minted canonical coach id; see
    /// [`Gender::stable_key`].
    pub const fn stable_key(self) -> &'static str {
        match self {
            Self::HeadCoach => "HeadCoach",
            Self::AssistantCoach => "AssistantCoach",
            Self::AthleticDirector => "AthleticDirector",
            Self::Unknown => "Unknown",
        }
    }
}
