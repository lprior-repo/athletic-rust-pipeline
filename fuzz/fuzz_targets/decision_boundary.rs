#![no_main]

use athletic_rust_pipeline::{
    domain::{
        candidate::CandidateEvidence,
        decision::{apply_review, assess, ReviewChoice, SearchCompleteness},
        evidence::ProfileEvidence,
        identity::{AthleteId, EvidenceDigest},
    },
    model::SourceRecord,
};
use libfuzzer_sys::fuzz_target;
use serde_json::json;

const MAX_INPUT_BYTES: usize = 256;
const SEARCH_DIGEST: &str = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
const DOCUMENT_DIGEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn source() -> SourceRecord {
    serde_json::from_value(json!({
        "source_key": "Synthetic:2",
        "sheet": "Synthetic",
        "excel_row": 2,
        "fields": {
            "Person First": "Ada",
            "Person Last": "Runner",
            "Schools Name": "Central High",
            "Address Mailing / Permanent City": "Austin",
            "Address Mailing / Permanent Region": "TX"
        }
    }))
    .expect("static decision fuzz source fixture must deserialize")
}

fn profile(issue: Option<&str>) -> ProfileEvidence {
    let issues = issue.map_or_else(Vec::new, |code| {
        vec![json!({
            "code": code,
            "message": "synthetic hard identity contradiction",
            "evidence": null
        })]
    });
    serde_json::from_value(json!({
        "athlete_id": 7,
        "profile_url": "https://www.athletic.net/athlete/7/track-and-field",
        "name": {
            "value": "Ada Runner",
            "evidence": {"document": DOCUMENT_DIGEST, "locator": "/athlete/name"}
        },
        "teams": [{
            "team_id": 11,
            "name": {
                "value": "Central High",
                "evidence": {"document": DOCUMENT_DIGEST, "locator": "/team/name"}
            },
            "location": {
                "value": {"city_region": {"city": "Austin", "region": "TX"}},
                "evidence": {"document": DOCUMENT_DIGEST, "locator": "/team/location"}
            },
            "seasons": [2024],
            "level": null
        }],
        "graduation_years": [],
        "grades": [],
        "sports": [{"state": "results_observed", "sport": "track_field", "count": 1}],
        "results": [{
            "result_id": 42,
            "sport": "track_field",
            "event_id": 100,
            "event_name": "100 Meter",
            "event_description": null,
            "event_type": null,
            "mark": "12.34",
            "units": "s",
            "season": 2024,
            "team_id": 11,
            "meet_id": 12,
            "meet_name": "Synthetic Meet",
            "date": null,
            "wind": null,
            "timing": null,
            "personal_best": {"state": "unavailable"},
            "season_best": {"state": "unavailable"},
            "attribution": {"kind": "individual"},
            "short_code": null,
            "result_url": null,
            "evidence": {"document": DOCUMENT_DIGEST, "locator": "/results/0"}
        }],
        "issues": issues,
        "documents": [DOCUMENT_DIGEST]
    }))
    .expect("static decision fuzz profile fixture must deserialize")
}

fn exercise(choice: &ReviewChoice, contradictory: bool) {
    let source = source();
    let evidence = EvidenceDigest::parse(SEARCH_DIGEST)
        .expect("static decision fuzz search digest must parse");
    let profiles = vec![profile(contradictory.then_some("identity_conflict"))];
    let assessment = assess(
        &source,
        profiles.iter().map(CandidateEvidence::Complete),
        SearchCompleteness::Complete { evidence },
    )
    .expect("static decision fuzz assessment fixture must be valid");
    let known_athlete_id =
        AthleteId::new(7).expect("static decision fuzz athlete ID fixture must be valid");

    if contradictory {
        assert!(
            !assessment.is_deterministic_acceptance(),
            "hard contradiction must block deterministic acceptance"
        );
        assert_eq!(assessment.accepted_athlete_id(), None);
    } else {
        assert!(
            assessment.is_deterministic_acceptance(),
            "known synthetic profile must be deterministic"
        );
        assert_eq!(assessment.accepted_athlete_id(), Some(known_athlete_id));
    }

    let outcome = apply_review(&assessment, choice.clone());

    match (choice, outcome) {
        (ReviewChoice::Select(requested_id), Ok(final_decision)) => {
            assert!(
                !contradictory && *requested_id == known_athlete_id,
                "selection accepted despite unknown ID or hard contradiction"
            );
            assert_eq!(
                final_decision.accepted_athlete_id(),
                Some(*requested_id),
                "accepted ID must equal the requested eligible ID"
            );
        }
        (ReviewChoice::Select(requested_id), Err(error)) => {
            assert!(
                contradictory || *requested_id != known_athlete_id,
                "known valid selection was rejected: {error}"
            );
        }
        (ReviewChoice::Unresolved, Ok(final_decision)) => {
            if contradictory {
                assert_eq!(final_decision.accepted_athlete_id(), None);
            } else {
                assert_eq!(final_decision.accepted_athlete_id(), Some(known_athlete_id));
            }
        }
        (ReviewChoice::Unresolved, Err(error)) => {
            panic!("unresolved review choice unexpectedly failed: {error}");
        }
    }
}

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_INPUT_BYTES {
        return;
    }
    let Ok(choice) = serde_json::from_slice::<ReviewChoice>(data) else {
        return;
    };
    // The choice is parsed by the production serde contract and then passed
    // through the production assessment/selection gates twice: once with a
    // valid identity and once with a hard contradiction.
    exercise(&choice, false);
    exercise(&choice, true);
});
