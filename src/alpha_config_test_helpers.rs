use crate::alpha_config::AlphaConfig;
use crate::alpha_model::{AlphaApiConfig, AuthorizationConfig, PaginationConfig};
pub(crate) fn valid_config() -> AlphaConfig {
    AlphaConfig {
        authorization: AuthorizationConfig {
            enabled: false,
            permission_reference: "disabled-example-no-permission".to_owned(),
            allowed_routes: vec![
                "/api/v1/tfRankings/GetRankings".to_owned(),
                "/api/v1/tfRankings/GetNavInfo".to_owned(),
            ],
            allowed_sports: vec!["Track and Field".to_owned()],
            allowed_states: canonical_states(),
            allowed_seasons: vec![2026],
            allowed_genders: vec!["m".to_owned(), "f".to_owned()],
            allowed_fields: vec![
                "AthleteID".to_owned(),
                "AthleteName".to_owned(),
                "GradeID".to_owned(),
                "TeamName".to_owned(),
                "State".to_owned(),
                "MeetID".to_owned(),
                "MeetName".to_owned(),
                "IDResult".to_owned(),
                "EventShort".to_owned(),
                "Measure".to_owned(),
                "ResultDate".to_owned(),
                "SeasonID".to_owned(),
            ],
            allowed_profile_routes: vec![],
            allow_profile_enrichment: false,
            max_concurrent_requests: 1,
            min_delay_ms: 750,
            max_retry_delay_ms: 30_000,
        },
        api: AlphaApiConfig {
            base_url: "https://www.athletic.net".to_owned(),
            rankings_path: "/api/v1/tfRankings/GetRankings".to_owned(),
            nav_info_path: "/api/v1/tfRankings/GetNavInfo".to_owned(),
            timeout_seconds: 30,
            max_retries: 2,
            pagination: PaginationConfig::SingleResponse {
                complete_pointer: "/settings/complete".to_owned(),
            },
            cap_markers: vec![],
        },
    }
}

pub(crate) fn canonical_states() -> Vec<String> {
    [
        "AL", "AK", "AZ", "AR", "CA", "CO", "CT", "DE", "FL", "GA", "HI", "ID", "IL", "IN",
        "IA", "KS", "KY", "LA", "ME", "MD", "MA", "MI", "MN", "MS", "MO", "MT", "NE", "NV",
        "NH", "NJ", "NM", "NY", "NC", "ND", "OH", "OK", "OR", "PA", "RI", "SC", "SD", "TN",
        "TX", "UT", "VT", "VA", "WA", "WV", "WI", "WY",
    ]
    .into_iter()
    .map(|s| s.to_owned())
    .collect()
}
