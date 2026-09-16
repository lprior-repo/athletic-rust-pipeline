pub use super::bio::parse_bio;
pub use super::html::{parse_profile_html, HtmlProfileEvidence, TreeHint};
pub use super::merge::merge_profiles;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::evidence::{BestClaim, Sport, SportAvailability};
    use crate::domain::identity::{AthleteId, EvidenceDigest};

    fn digest() -> EvidenceDigest {
        EvidenceDigest::parse(&"a".repeat(64)).expect("test digest")
    }
    fn athlete() -> AthleteId {
        AthleteId::new(123).expect("test athlete")
    }

    #[test]
    fn bio_preserves_joins_and_opaque_flags() {
        let body = br#"{"athlete":{"IDAthlete":123,"FirstName":"Redacted","LastName":"Runner"},"allSeasons":[{"SchoolID":7,"IDSeason":12025}],"allTeams":{"7":{"SchoolName":"Fictional High","Level":4}},"grades":{"7_12025":12},"meets":{"9":{"MeetName":"Synthetic Meet"}},"resultsTF":[{"IDResult":11,"AthleteID":123,"Result":"10.72a","SchoolID":7,"MeetID":9,"SeasonID":12025,"EventID":1,"PersonalBest":14,"SeasonBest":1,"FAT":1,"shortCode":"token"}],"eventsTF":[{"IDEvent":1,"Event":"100 Meters","Type":"T","PersonalEvent":true}],"resultsXC":null}"#;
        let profile =
            parse_bio(athlete(), Sport::TrackField, digest(), body).expect("synthetic bio");
        assert_eq!(profile.results.len(), 1);
        assert_eq!(
            profile.results[0].meet_name.as_deref(),
            Some("Synthetic Meet")
        );
        assert_eq!(profile.results[0].personal_best, BestClaim::OpaqueFlags(14));
        assert_eq!(
            profile.results[0].result_url.as_deref(),
            Some("http://www.athletic.net/result/token")
        );
        assert_eq!(
            profile.sports,
            vec![SportAvailability::ResultsObserved {
                sport: Sport::TrackField,
                count: 1
            }]
        );
        let merged = merge_profiles(profile.clone(), profile).expect("duplicate merge");
        assert_eq!(merged.results.len(), 1);
    }

    #[test]
    fn html_ignores_unscoped_and_script_cohort_text() {
        let body = br#"<link rel="canonical" href="https://www.athletic.net/athlete/123/track-and-field/all"><script>window.anetSiteAppParams={"tree":[{"type":"athlete","id":123,"title":"Redacted Runner"}],"note":"Class of 2027"};</script><p>Class of 2027</p><div data-athlete-id="123">Class of 2027</div>"#;
        let profile = parse_profile_html(athlete(), digest(), body).expect("synthetic html");
        assert_eq!(profile.cohort_witnesses.len(), 1);
        assert_eq!(profile.identity_hints.len(), 1);
        assert_eq!(profile.embedded_state.len(), 1);
    }

    #[test]
    fn bio_rejects_cross_athlete_response() {
        let body = br#"{"athlete":{"IDAthlete":999,"FirstName":"Other","LastName":"Runner"},"resultsTF":null}"#;
        assert!(parse_bio(athlete(), Sport::TrackField, digest(), body).is_err());
    }

    #[test]
    fn foreign_nested_cohort_and_hidden_text_do_not_belong_to_the_athlete() {
        let body = br#"<link href="https://www.athletic.net/athlete/123/track-and-field/all" rel="canonical"><div data-athlete-id="123"><span>Class of </span><strong>2026</strong><div data-athlete-id="999">Class of 2027</div><span hidden>Class of 2027</span><script>Class of 2027</script></div>"#;
        let parsed = parse_profile_html(athlete(), digest(), body).expect("scoped HTML");
        assert_eq!(
            parsed
                .cohort_witnesses
                .iter()
                .map(|year| year.value.get())
                .collect::<Vec<_>>(),
            vec![2026]
        );
    }

    #[test]
    fn reordered_canonical_attributes_cannot_hide_a_foreign_identity() {
        let body = br#"<link href="https://www.athletic.net/athlete/999/track-and-field/all" rel="canonical"><div data-athlete-id="123">Class of 2027</div>"#;
        assert!(parse_profile_html(athlete(), digest(), body).is_err());
    }

    #[test]
    fn embedded_json_escapes_do_not_truncate_athlete_identity() {
        let body = br#"<link rel="canonical" href="https://www.athletic.net/athlete/123/track-and-field/all"><script>window.anetSiteAppParams={"note":"escaped \" } brace", "tree":[{"type":"athlete","id":123,"title":"Synthetic Runner"}]}; trailingCall();</script>"#;
        let parsed = parse_profile_html(athlete(), digest(), body).expect("embedded JSON");
        assert_eq!(
            parsed
                .identity_hints
                .iter()
                .map(|name| name.value.as_str())
                .collect::<Vec<_>>(),
            vec!["Synthetic Runner"]
        );
        assert!(parsed.cohort_witnesses.is_empty());
    }

    #[test]
    fn embedded_identity_contradiction_is_retained() {
        let body = br#"<link rel="canonical" href="https://www.athletic.net/athlete/123/track-and-field/all"><script>window.anetSiteAppParams={"tree":[{"type":"athlete","id":999,"title":"Other Runner"}]};</script>"#;
        let parsed = parse_profile_html(athlete(), digest(), body).expect("canonical HTML");
        assert!(parsed.identity_hints.is_empty());
        assert!(parsed
            .issues
            .iter()
            .any(|issue| issue.code == "embedded_state_shape"));
    }
}
