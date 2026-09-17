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
        _ => Vec::new(),
    }
}
pub fn scenario_for_text(text: &str) -> Scenario {
    let normalized = text.to_ascii_lowercase();
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
    format!("<tr><td><a href=\"https://www.athletic.net/athlete/{id}/{suffix}\">{}</a></td><td>Synthetic result</td></tr>", display_name(id))
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
pub fn bio_body(id: u64, case: Scenario, _sport: &str) -> String {
    let team = 500 + id.saturating_sub(1000);
    let name = display_name(id);
    let (first, last) = name.split_once(' ').map_or((name, "Runner"), |value| value);
    let (school, city, region) = identity_context(case);
    let location = (city, region);
    let teams = json!({team.to_string(): {"SchoolName": school, "City": location.0, "State": location.1, "Level": 4}});
    let seasons = json!([{"SchoolID":team,"IDSeason":12025}]);
    let tf_results = json!([{"IDResult":id,"AthleteID":id,"Result":"10.72","SchoolID":team,"MeetID":9,"SeasonID":12025,"EventID":1,"EventTypeID":7,"PersonalBest":14,"SeasonBest":1,"FAT":1,"shortCode":format!("synthetic-tf-{id}") }]);
    let xc_results = json!([{"IDResult":id.saturating_add(100_000),"AthleteID":id,"Result":"17:42","SchoolID":team,"MeetID":9,"SeasonID":12025,"Distance":5000,"PersonalBest":1,"SeasonBest":1,"shortCode":format!("synthetic-xc-{id}") }]);
    let events = json!([{"IDEvent":1,"IDEventType":7,"Event":"100 Meters","Type":"T","FieldMeasureType":"S","PersonalEvent":true}]);
    let distances = json!([{"Meters":5000,"Distance":5,"Units":"km"}]);
    json!({
        "athlete":{"IDAthlete":id,"FirstName":first,"LastName":last},
        "allSeasons":seasons,
        "allTeams":teams,
        "grades":{},
        "meets":{"9":{"MeetName":"Synthetic Meet"}},
        "eventsTF":events,
        "distancesXC":distances,
        "resultsTF":tf_results,
        "resultsXC":xc_results
    })
    .to_string()
}
pub fn team_body(id: u64, case: Scenario) -> String {
    let (name, city, region) = identity_context(case);
    let location = (city, region);
    json!({"team":{"ID":id,"Name":name,"City":location.0,"State":location.1,"Level":4}}).to_string()
}

fn identity_context(case: Scenario) -> (&'static str, &'static str, &'static str) {
    match case {
        Scenario::Duplicate => ("Duplicate High", "Denver", "CO"),
        Scenario::Ambiguous => ("Twin High", "Boston", "MA"),
        Scenario::MissingCohort => ("No Class High", "Reno", "NV"),
        Scenario::Conflict => ("Conflict High", "Portland", "OR"),
        _ => ("Central High", "Austin", "TX"),
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
    let scoped_name = if matches!(case, Scenario::Conflict) {
        "Jordan Other"
    } else {
        display_name(id)
    };
    format!("<html><head><link rel=\"canonical\" href=\"{canonical}\"></head><body><div data-athlete-id=\"{id}\">{cohort}</div><script>window.anetSiteAppParams={{\"tree\":[{{\"type\":\"athlete\",\"id\":{id},\"title\":\"{scoped_name}\"}}]}};</script></body></html>")
}
