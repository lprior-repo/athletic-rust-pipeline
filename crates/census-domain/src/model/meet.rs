use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalMeet {
    pub id: MeetId,
    pub name: String,
    pub normalized_name: String,
    pub date: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(
        default,
        serialize_with = "serialize_meet_state",
        deserialize_with = "deserialize_meet_state"
    )]
    pub state: Option<UsJurisdiction>,
    pub level: CompetitionLevel,
    pub sports: Vec<Sport>,
    pub source_identities: Vec<SourceIdentity>,
    pub source_urls: Vec<String>,
    pub evidence: Vec<Evidence>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub retained_conflicts: Vec<RetainedConflict>,
}

pub const MEET_STATE_UNRESOLVED: &str = "??";

pub(super) fn serialize_meet_state<S>(
    state: &Option<UsJurisdiction>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(state.map_or(MEET_STATE_UNRESOLVED, UsJurisdiction::code))
}

pub(super) fn deserialize_meet_state<'de, D>(
    deserializer: D,
) -> Result<Option<UsJurisdiction>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = String::deserialize(deserializer)?;
    match raw.as_str() {
        MEET_STATE_UNRESOLVED => Ok(None),
        code => UsJurisdiction::parse(code).map(Some).ok_or_else(|| {
            serde::de::Error::custom(format_args!(
                "meet state {code:?} is not one of the 50 states or the District of Columbia"
            ))
        }),
    }
}

impl CanonicalMeet {
    pub fn mint(
        state: Option<UsJurisdiction>,
        date: &str,
        name: &str,
        _location: Option<&str>,
    ) -> MeetId {
        Id::mint(
            "meet",
            &[
                state
                    .map(UsJurisdiction::code)
                    .unwrap_or(MEET_STATE_UNRESOLVED),
                date,
                &normalize_name(name),
            ],
        )
    }

    pub fn new(
        state: Option<UsJurisdiction>,
        name: impl Into<String>,
        date: impl Into<String>,
        level: CompetitionLevel,
    ) -> Self {
        let name = name.into();
        let date = date.into();
        let normalized_name = normalize_name(&name);
        let id = CanonicalMeet::mint(state, &date, &name, None);
        Self {
            id,
            name,
            normalized_name,
            date,
            end_date: None,
            location: None,
            state,
            level,
            sports: Vec::new(),
            source_identities: Vec::new(),
            source_urls: Vec::new(),
            evidence: Vec::new(),
            retained_conflicts: Vec::new(),
        }
    }
}
