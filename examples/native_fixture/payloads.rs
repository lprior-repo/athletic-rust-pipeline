use super::scenarios::Scenario;
use axum::{
    body::Body,
    http::{header, HeaderValue, StatusCode},
    response::Response,
};
use serde_json::{json, Value};

pub fn candidate_ids(case: Scenario) -> Vec<u64> {
    match case {
        Scenario::Match => vec![1001],
        Scenario::Duplicate => vec![1002],
        Scenario::Ambiguous => vec![1003, 1004],
        Scenario::MissingCohort => vec![1005],
        Scenario::Conflict => vec![1006],
        Scenario::SplitLocation => vec![1012],
        Scenario::GenericSchool => vec![1013],
        Scenario::NameExclusion => vec![1014, 1015],
        Scenario::ProbeFailure => vec![1016, 1017],
        Scenario::BioIdentityConflict => vec![1018],
        Scenario::HtmlIdentityUnknown => vec![1019],
        Scenario::IncompleteIdentity => vec![1020],
        Scenario::WrongBioId => vec![1021],
        Scenario::MisleadingSearchName => vec![1022],
        Scenario::MissingHtmlHint => vec![1023],
        Scenario::RawIdentityConflict => vec![1024],
        Scenario::HtmlAliasConflict => vec![1025],
        _ => Vec::new(),
    }
}
pub fn scenario_for_text(text: &str) -> Scenario {
    let normalized = text.to_ascii_lowercase();
    if let Some(case) = Scenario::identity_query(&normalized) {
        return case;
    }
    if normalized.contains("casey") {
        Scenario::Duplicate
    } else if normalized.contains("sam") {
        Scenario::Ambiguous
    } else if normalized.contains("morgan") {
        Scenario::MissingCohort
    } else if normalized.contains("taylor") {
        Scenario::Conflict
    } else if normalized.contains("empty") {
        Scenario::EmptySearch
    } else if normalized.contains("broken") {
        Scenario::Malformed
    } else if normalized.contains("retry") {
        Scenario::RetryExhaustion
    } else if normalized.contains("huge") {
        Scenario::PayloadLimit
    } else if normalized.contains("isolated") {
        Scenario::AccessDenied
    } else if normalized.contains("riley") {
        Scenario::SplitLocation
    } else if normalized.contains("gene") {
        Scenario::GenericSchool
    } else {
        Scenario::Match
    }
}
pub fn scenario_for_id(id: u64) -> Scenario {
    match id {
        1001 => Scenario::Match,
        1002 => Scenario::Duplicate,
        1003 | 1004 => Scenario::Ambiguous,
        1005 => Scenario::MissingCohort,
        1006 => Scenario::Conflict,
        1008 => Scenario::Malformed,
        1009 => Scenario::RetryExhaustion,
        1010 => Scenario::PayloadLimit,
        1011 => Scenario::AccessDenied,
        1012 => Scenario::SplitLocation,
        1013 => Scenario::GenericSchool,
        1014 | 1015 => Scenario::NameExclusion,
        1016 | 1017 => Scenario::ProbeFailure,
        1018 => Scenario::BioIdentityConflict,
        1019 => Scenario::HtmlIdentityUnknown,
        1020 => Scenario::IncompleteIdentity,
        1021 => Scenario::WrongBioId,
        1022 => Scenario::MisleadingSearchName,
        1023 => Scenario::MissingHtmlHint,
        1024 => Scenario::RawIdentityConflict,
        1025 => Scenario::HtmlAliasConflict,
        _ => Scenario::Match,
    }
}
pub fn scenario_for_team(id: u64) -> Scenario {
    match id {
        501 => Scenario::Match,
        502 => Scenario::Duplicate,
        503 | 504 => Scenario::Ambiguous,
        505 => Scenario::MissingCohort,
        506 => Scenario::Conflict,
        509 => Scenario::RetryExhaustion,
        510 => Scenario::PayloadLimit,
        511 => Scenario::AccessDenied,
        512 | 612 => Scenario::SplitLocation,
        513 => Scenario::GenericSchool,
        514 | 515 => Scenario::NameExclusion,
        516 | 517 => Scenario::ProbeFailure,
        518 => Scenario::BioIdentityConflict,
        519 => Scenario::HtmlIdentityUnknown,
        520 => Scenario::IncompleteIdentity,
        521 => Scenario::WrongBioId,
        522 => Scenario::MisleadingSearchName,
        523 => Scenario::MissingHtmlHint,
        524 => Scenario::RawIdentityConflict,
        525 => Scenario::HtmlAliasConflict,
        _ => Scenario::Match,
    }
}
pub fn display_name(id: u64) -> &'static str {
    match id {
        1001 => "Ada Runner",
        1002 => "Casey Copy",
        1003 | 1004 => "Sam Same",
        1005 => "Morgan Missing",
        1006 => "Taylor Clash",
        1012 => "Riley Split",
        1013 => "Gene Generic",
        1014 => "Alex Coverage",
        1015 => "Bert Other",
        1016 => "Rae Failurecase",
        1017 => "Chris Remote",
        1018 => "Drew Bioclash",
        1019 => "Safe Different",
        1020 => "- Other",
        1021 => "Wrong Binding",
        1022 => "Lena Displaycase",
        1023 => "Safe Nohint",
        1024 | 1025 => "José Runner",
        _ => "Synthetic Runner",
    }
}
pub fn search_row(id: u64, case: Scenario, sport: &str) -> String {
    let default_suffix = if sport == "xc" {
        "cross-country/all"
    } else {
        "track-and-field/all"
    };
    let suffix = if matches!(case, Scenario::MissingCohort | Scenario::Conflict) {
        default_suffix.trim_end_matches("/all")
    } else {
        default_suffix
    };
    let label = if case == Scenario::MisleadingSearchName {
        "Somebody Else"
    } else {
        display_name(id)
    };
    format!("<tr><td><a href=\"https://www.athletic.net/athlete/{id}/{suffix}\">{label}</a></td><td>Synthetic result</td></tr>")
}
pub fn model_response(verdict: Value) -> Response {
    response(
        StatusCode::OK,
        "application/json",
        json!({"choices":[{"message":{"content":verdict.to_string()},"finish_reason":"stop"}]})
            .to_string()
            .into_bytes(),
    )
}
pub fn response(status: StatusCode, content_type: &'static str, body: Vec<u8>) -> Response {
    let mut response = Response::new(Body::from(body));
    *response.status_mut() = status;
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    response
}
pub fn bio_body(id: u64, case: Scenario, sport: &str) -> String {
    let (team, first, last) = bio_identity(id, case, sport);
    let reported_id = if case == Scenario::WrongBioId {
        9999
    } else {
        id
    };
    let teams = bio_teams(team, case, sport, identity_context(case));
    let seasons = bio_seasons(id, case, team);
    let (results_tf, results_xc, events, distances) = bio_blocks(id, case, sport, team);
    json!({
        "athlete":{"IDAthlete":reported_id,"FirstName":first,"LastName":last},
        "allSeasons":seasons,
        "allTeams":teams,
        "grades":{},
        "meets":{"9":{"MeetName":"Synthetic Meet"}},
        "eventsTF":events,
        "distancesXC":distances,
        "resultsTF":results_tf,
        "resultsXC":results_xc
    })
    .to_string()
}

fn bio_identity(id: u64, case: Scenario, sport: &str) -> (u64, &'static str, &'static str) {
    let team = 500_u64.saturating_add(id.saturating_sub(1000));
    let team = if matches!(case, Scenario::SplitLocation) && sport == "xc" {
        612
    } else {
        team
    };
    let name = display_name(id);
    let (first, last) = name.split_once(' ').map_or((name, "Runner"), |value| value);
    let (first, last) = match (case, sport) {
        (Scenario::BioIdentityConflict, "xc") => ("Different", "Person"),
        (Scenario::RawIdentityConflict, "xc") => ("Jose", "Runner"),
        _ => (first, last),
    };
    (team, first, last)
}

fn bio_teams(team: u64, case: Scenario, sport: &str, identity: (&str, &str, &str)) -> Value {
    let (school, city, region) = identity;
    match (case, sport) {
        (Scenario::Match, _) | (Scenario::Duplicate | Scenario::SplitLocation, "tf") => {
            json!({team.to_string(): {"SchoolName": school, "City": city, "Level": 4}})
        }
        (Scenario::Duplicate | Scenario::SplitLocation, _) => {
            json!({team.to_string(): {"SchoolName": school, "State": region, "Level": 4}})
        }
        _ => {
            json!({team.to_string(): {"SchoolName": school, "City": city, "State": region, "Level": 4}})
        }
    }
}

fn bio_seasons(id: u64, case: Scenario, team: u64) -> Value {
    if id == 1015 || case == Scenario::MissingHtmlHint {
        let mut entries = (12018_u16..=12025)
            .map(|season| json!({"SchoolID":team,"IDSeason":season}))
            .collect::<Vec<_>>();
        entries.push(json!({"SchoolID":0,"IDSeason":0}));
        Value::Array(entries)
    } else {
        json!([{"SchoolID":team,"IDSeason":12025}])
    }
}

fn bio_blocks(id: u64, case: Scenario, sport: &str, team: u64) -> (Value, Value, Value, Value) {
    let results_tf = json!([{"IDResult":id,"AthleteID":id,"Result":"10.72","SchoolID":team,"MeetID":9,"SeasonID":12025,"EventID":1,"EventTypeID":7,"PersonalBest":14,"SeasonBest":1,"FAT":1,"shortCode":format!("synthetic-tf-{id}") }]);
    let results_xc = json!([{"IDResult":id.saturating_add(100_000),"AthleteID":id,"Result":"17:42","SchoolID":team,"MeetID":9,"SeasonID":12025,"Distance":5000,"PersonalBest":1,"SeasonBest":1,"shortCode":format!("synthetic-xc-{id}") }]);
    let events = json!([{"IDEvent":1,"IDEventType":7,"Event":"100 Meters","Type":"T","FieldMeasureType":"S","PersonalEvent":true}]);
    let distances = json!([{"Meters":5000,"Distance":5,"Units":"km"}]);
    // Public-API regression fixture: Match poisons metadata for the unselected sport.
    let (events, distances) = match (case, sport) {
        (Scenario::Match, "tf") => (events, json!({"malformed": true})),
        (Scenario::Match, "xc") => (json!({"malformed": true}), distances),
        _ => (events, distances),
    };
    (results_tf, results_xc, events, distances)
}
pub fn team_body(id: u64, case: Scenario) -> String {
    let (name, city, region) = identity_context(case);
    match case {
        Scenario::Match => json!({"team":{"ID":id,"Name":name,"State":region,"Level":4}}),
        Scenario::Duplicate | Scenario::SplitLocation => {
            json!({"team":{"ID":id,"Name":name,"Level":4}})
        }
        _ => json!({"team":{"ID":id,"Name":name,"City":city,"State":region,"Level":4}}),
    }
    .to_string()
}

fn identity_context(case: Scenario) -> (&'static str, &'static str, &'static str) {
    match case {
        Scenario::Duplicate => ("Duplicate High", "Denver", "CO"),
        Scenario::Ambiguous => ("Twin High", "Boston", "MA"),
        Scenario::MissingCohort => ("No Class High", "Reno", "NV"),
        Scenario::Conflict => ("Conflict High", "Portland", "OR"),
        Scenario::GenericSchool => ("High School", "Austin", "TX"),
        _ => ("Central", "Austin", "TX"),
    }
}
pub fn profile_html(id: u64, case: Scenario) -> String {
    let canonical = format!("https://www.athletic.net/athlete/{id}/track-and-field/all");
    let cohort = match case {
        Scenario::MissingCohort => String::new(),
        Scenario::Conflict => "<span>Class of 2024</span>".to_owned(),
        Scenario::Match => "<span>Class of 2026</span>".to_owned(),
        _ => "<span>Class of 2027</span>".to_owned(),
    };
    let scoped_name = match case {
        Scenario::Conflict => "Jordan Other",
        Scenario::HtmlAliasConflict => "Jose Runner",
        _ => display_name(id),
    };
    let embedded_id = if case == Scenario::HtmlIdentityUnknown {
        9999
    } else {
        id
    };
    let script = if case == Scenario::MissingHtmlHint {
        String::new()
    } else {
        format!("<script>window.anetSiteAppParams={{\"tree\":[{{\"type\":\"athlete\",\"id\":{embedded_id},\"title\":\"{scoped_name}\"}}]}};</script>")
    };
    format!("<html><head><link rel=\"canonical\" href=\"{canonical}\"></head><body><div data-athlete-id=\"{id}\">{cohort}</div>{script}</body></html>")
}
