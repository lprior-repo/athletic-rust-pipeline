#[derive(Debug, thiserror::Error)]
pub enum PageParseError {
    #[error("missing division in response")]
    MissingDivision,
    #[error("missing division ID")]
    MissingDivisionId,
    #[error("missing season ID")]
    MissingSeasonId,
    #[error("missing BaseDiv in division")]
    MissingBaseDiv,
    #[error("missing country in BaseDiv")]
    MissingCountry,
    #[error("missing level in BaseDiv")]
    MissingLevel,
    #[error("missing gender field")]
    MissingGender,
    #[error("missing eventShort field")]
    MissingEventShort,
    #[error("missing eventId")]
    MissingEventId,
    #[error("missing settings")]
    MissingSettings,
    #[error("missing page setting")]
    MissingPage,
    #[error("missing depth setting")]
    MissingDepth,
    #[error("missing grades array")]
    MissingGrades,
    #[error("page number is outside the supported range")]
    InvalidPageNumber,
    #[error("missing numeric minCount")]
    MissingMinCount,
    #[error("missing groupedRankings array")]
    MissingGroupedRankings,
    #[error("relayTeams must be object type")]
    WrongRelayTeamsType,
    #[error("relay team IDResult or RelayTeamID missing")]
    WrongRelayTeamId,
    #[error("Members field must be array type")]
    WrongMembersType,
    #[error("non-array group at index {index}")]
    NonArrayGroup { index: usize },
    #[error("non-numeric grade in settings")]
    NonNumericGrade,
    #[error("missing rowNum in row")]
    MissingRowNum,
    #[error("missing IDResult in row")]
    MissingRowIdResult,
    #[error("row number must be non-zero")]
    ZeroRowNumber,
    #[error("IDResult must be non-zero")]
    ZeroResultId,
    #[error("page row limit exceeded")]
    PageRowsLimitExceeded,
    #[error("page candidate limit exceeded")]
    CandidateLimitExceeded,
    #[error("page counter overflow")]
    CounterOverflow,
    #[error("relay team limit exceeded")]
    RelayTeamLimitExceeded,
    #[error("division ID mismatch: expected {expected}, got {actual}")]
    DivisionMismatch { expected: u64, actual: u64 },
    #[error("season ID mismatch: expected {expected}, got {actual}")]
    SeasonMismatch { expected: u64, actual: u64 },
    #[error("wrong country: expected {expected}, got {actual}")]
    WrongCountry { expected: String, actual: String },
    #[error("wrong level: expected {expected}, got {actual}")]
    WrongLevel { expected: u64, actual: u64 },
    #[error("gender mismatch: expected {expected}, got {actual}")]
    GenderMismatch { expected: String, actual: String },
    #[error("eventShort mismatch: expected {expected}, got {actual}")]
    EventShortMismatch { expected: String, actual: String },
    #[error("eventId mismatch: expected {expected}, got {actual}")]
    EventIdMismatch { expected: u64, actual: u64 },
    #[error("page mismatch: expected {expected}, got {actual}")]
    PageMismatch { expected: u32, actual: u32 },
    #[error("zero page depth in settings")]
    ZeroPageDepth,
    #[error("relay has grades filter: expected empty")]
    RelayHasGrades,
    #[error("wrong grade filter: expected {expected:?}, got {actual:?}")]
    WrongGradeFilter {
        expected: Vec<u64>,
        actual: Vec<u64>,
    },
    #[error("wrong relay join: roster.IDResult={roster_id_result} row.IDResult={row_id_result} roster.RelayTeamID={roster_relay_team_id} row.AthleteID={row_athlete_id}")]
    WrongRelayJoin {
        roster_id_result: u64,
        row_id_result: u64,
        roster_relay_team_id: u64,
        row_athlete_id: u64,
    },
}
