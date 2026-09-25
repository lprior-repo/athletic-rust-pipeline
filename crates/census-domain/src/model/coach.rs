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
    /// The address a source published on a school, district, association or organisation domain.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub professional_email: Option<String>,
    /// The address a source published on a consumer mailbox (see `CONSUMER_MAIL_DOMAINS`).
    ///
    /// Both fields publish an address a source carried, each with the kind of domain it sits on, so a
    /// reader can take the school contact alone or every address the coach ever published. Nothing is
    /// withheld: [`publish`](crate::model::CanonicalCoach) routes an address to its field by its own
    /// domain and never trusts the field it arrived in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub personal_email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    pub source_identities: Vec<SourceIdentity>,
    pub evidence: Vec<Evidence>,
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
            personal_email: None,
            phone: None,
            source_identities: Vec::new(),
            evidence: Vec::new(),
            retained_conflicts: Vec::new(),
        }
    }

    /// Whether the row carries an address a source published, whichever field it landed in.
    ///
    /// A coach's published contact is the school-domain address when a source published one, and the
    /// coach's own mailbox when that is all any source published. A reader asking whether the school
    /// can be reached at all asks this, rather than reading one field and answering "no contact".
    pub fn has_published_email(&self) -> bool {
        self.professional_email.is_some() || self.personal_email.is_some()
    }

    /// Record an address a source published, in the field its own domain belongs to.
    ///
    /// The kind is derived from the domain, never from the caller's belief: a consumer mailbox lands in
    /// [`personal_email`](Self::personal_email) and an organisation mailbox in
    /// [`professional_email`](Self::professional_email). The first address of a kind wins, so two rows
    /// publishing two addresses of one kind leave the row to the order the source published them in.
    /// A malformed address is refused — a source that published one published no contact — and nothing
    /// is ever dropped for its domain.
    pub fn set_published_email(&mut self, address: &str) {
        match published_email(address) {
            Some((address, MailboxKind::Professional)) => {
                if self.professional_email.is_none() {
                    self.professional_email = Some(address);
                }
            }
            Some((address, MailboxKind::Personal)) => {
                if self.personal_email.is_none() {
                    self.personal_email = Some(address);
                }
            }
            None => {}
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
