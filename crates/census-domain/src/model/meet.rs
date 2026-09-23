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
    /// The jurisdiction the meet was held in.
    ///
    /// `None` is a *coverage gap*, not an unknown state: no evidence placed the venue in any
    /// jurisdiction, and [`MEET_STATE_UNRESOLVED`] is how a report spells that bucket. Both
    /// directions keep the free-string wire form - `Some` is the jurisdiction's code, so a placed
    /// meet still writes `"state":"WI"`, and `None` is the sentinel, so an unplaced one still writes
    /// `"state":"??"` instead of dropping the key - and the sentinel decodes back to `None`, so a
    /// meet row written before this field was typed still reads.
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
    /// Canonical-id collisions this row's merge retained: another natural key minted this id, so the
    /// row below is the one that survived and the other subject's facts were not absorbed. Empty on
    /// every row whose fields still state the id they minted, which is every row until one collides.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub retained_conflicts: Vec<RetainedConflict>,
}

/// The pre-cutover wire sentinel for a meet whose venue was never placed in a jurisdiction.
///
/// Meet identity hashes it exactly as the free-string era hashed the string `"??"`, so every meet
/// already stored under that sentinel keeps its id; a report renders the unresolved bucket with it.
pub const MEET_STATE_UNRESOLVED: &str = "??";

/// Write a meet's `state` in the form the store has always carried: code, or the sentinel.
///
/// The key is never omitted. `skip_serializing_if` would shrink an unplaced meet's row by one line,
/// which is a silent wire change for every reader that counts keys and a visible one for the golden
/// captures, so `None` is written as [`MEET_STATE_UNRESOLVED`] exactly as the free-form era wrote it.
pub(super) fn serialize_meet_state<S>(state: &Option<UsJurisdiction>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(state.map_or(MEET_STATE_UNRESOLVED, UsJurisdiction::code))
}

/// Decode a meet's `state`: the legacy [`MEET_STATE_UNRESOLVED`] sentinel or a jurisdiction.
///
/// This is the one place the sentinel is legal. Everything downstream sees
/// `Option<UsJurisdiction>`, so no reader can mistake "never placed" for a jurisdiction named `??`,
/// while a genuinely unknown code stays a decode error instead of a retained string. An absent key
/// never reaches this function: the field's `default` supplies `None` for it.
pub(super) fn deserialize_meet_state<'de, D>(deserializer: D) -> Result<Option<UsJurisdiction>, D::Error>
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
    /// Meet identity = jurisdiction + date + normalized name.
    ///
    /// Location is deliberately excluded: providers spell the same venue differently ("UW-La Crosse"
    /// vs "La Crosse, WI"), and a meet that one source publishes with a location and another without
    /// must still be one canonical meet. The observed location is retained on the record as a field.
    ///
    /// The key carries the jurisdiction's USPS code — byte-identical to the uppercase state string
    /// this parameter used to hold — and [`MEET_STATE_UNRESOLVED`] when the venue was never placed,
    /// which is the string the free-form era hashed, so typing the parameter re-mints no meet id.
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

