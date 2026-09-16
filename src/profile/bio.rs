mod events;

use anyhow::{anyhow, bail, Context, Result};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};

use crate::domain::evidence::{
    BestClaim, EvidenceIssue, EvidenceRef, GradeAtSeason, Observed, ProfileEvidence,
    ResultAttribution, ResultEvidence, Sport, SportAvailability, TeamEvidence,
};
use crate::domain::facts::{AthleteName, SchoolName};
use crate::domain::identity::{AthleteId, EvidenceDigest, ProfileUrl};

const MAX_BIO_BYTES: usize = 32 * 1024 * 1024;
const MAX_ITEMS: usize = 100_000;
const MAX_TEXT_BYTES: usize = 4_096;
const MAX_SOURCE_ID: u64 = 10_000_000_000_000;

pub fn parse_bio(
    id: AthleteId,
    sport: Sport,
    digest: EvidenceDigest,
    bytes: &[u8],
) -> Result<ProfileEvidence> {
    if bytes.len() > MAX_BIO_BYTES {
        bail!("bio document exceeds {} bytes", MAX_BIO_BYTES);
    }
    let root: Value = serde_json::from_slice(bytes).context("bio JSON is invalid")?;
    let object = root
        .as_object()
        .ok_or_else(|| anyhow!("bio envelope is not an object"))?;
    let mut issues = Vec::new();
    let name = parse_identity(object, id, &digest, &mut issues)?;
    let (teams, grades) = parse_teams(object, &digest, &mut issues);
    let parsed = parse_results(object, id, sport, &digest, &mut issues);
    Ok(ProfileEvidence {
        athlete_id: id,
        profile_url: profile_url(id, sport)?,
        name,
        teams,
        graduation_years: Vec::new(),
        grades,
        sports: vec![availability(sport, parsed.count, parsed.present)],
        results: parsed.results,
        issues,
        documents: vec![digest],
    })
}

fn profile_url(id: AthleteId, sport: Sport) -> Result<ProfileUrl> {
    let suffix = match sport {
        Sport::TrackField => "track-and-field",
        Sport::CrossCountry => "cross-country",
    };
    ProfileUrl::parse(&format!(
        "https://www.athletic.net/athlete/{}/{suffix}",
        id.get()
    ))
    .context("profile URL is invalid")
}

fn parse_identity(
    root: &Map<String, Value>,
    requested: AthleteId,
    digest: &EvidenceDigest,
    issues: &mut Vec<EvidenceIssue>,
) -> Result<Observed<AthleteName>> {
    let athlete = root
        .get("athlete")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("bio athlete object missing"))?;
    let observed = athlete
        .get("IDAthlete")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow!("athlete ID is not an unsigned integer"))?;
    if observed != requested.get() {
        bail!(
            "bio athlete ID {} does not match requested {}",
            observed,
            requested.get()
        );
    }
    let first = athlete_text(athlete.get("FirstName"), "FirstName")?;
    let last = athlete_text(athlete.get("LastName"), "LastName")?;
    let value =
        AthleteName::parse(format!("{first} {last}").trim()).context("athlete name is invalid")?;
    let evidence = ev(digest, "/athlete/FirstName+/LastName");
    if first.is_empty() || last.is_empty() {
        issues.push(issue(
            "identity_incomplete",
            "athlete name has an empty component",
            Some(evidence.clone()),
        ));
    }
    Ok(Observed { value, evidence })
}

fn athlete_text(value: Option<&Value>, key: &str) -> Result<String> {
    let text = value
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("athlete {key} is not text"))?;
    if text.len() > MAX_TEXT_BYTES {
        bail!("athlete {key} exceeds {} bytes", MAX_TEXT_BYTES);
    }
    Ok(text.trim().to_owned())
}

fn parse_teams(
    root: &Map<String, Value>,
    digest: &EvidenceDigest,
    issues: &mut Vec<EvidenceIssue>,
) -> (Vec<TeamEvidence>, Vec<GradeAtSeason>) {
    let mut affiliations: BTreeMap<u64, Vec<u16>> = BTreeMap::new();
    if root
        .get("allSeasons")
        .is_some_and(|value| !value.is_array() && !value.is_null())
    {
        issues.push(issue(
            "unknown_seasons_shape",
            "allSeasons is not an array",
            Some(ev(digest, "/allSeasons")),
        ));
    }
    if let Some(items) = root.get("allSeasons").and_then(Value::as_array) {
        retain_cap_issue(
            items.len(),
            "seasons_truncated",
            "/allSeasons",
            digest,
            issues,
        );
    }
    root.get("allSeasons")
        .and_then(Value::as_array)
        .into_iter()
        .flat_map(|items| items.iter().take(MAX_ITEMS))
        .enumerate()
        .for_each(|(index, item)| {
            let Some(obj) = item.as_object() else {
                issues.push(issue(
                    "unknown_season_shape",
                    "allSeasons item is not an object",
                    Some(ev(digest, format!("/allSeasons/{index}"))),
                ));
                return;
            };
            let team = obj.get("SchoolID").and_then(Value::as_u64);
            let season = obj
                .get("IDSeason")
                .and_then(Value::as_u64)
                .and_then(|value| u16::try_from(value).ok());
            match (team, season) {
                (Some(team), Some(season)) if bounded_id(team) => {
                    affiliations.entry(team).or_default().push(season);
                }
                _ => issues.push(issue(
                    "invalid_season",
                    "season affiliation has invalid identifiers",
                    Some(ev(digest, format!("/allSeasons/{index}"))),
                )),
            }
        });
    let teams = match root.get("allTeams").and_then(Value::as_object) {
        Some(map) => {
            retain_cap_issue(map.len(), "teams_truncated", "/allTeams", digest, issues);
            map.iter()
                .take(MAX_ITEMS)
                .filter_map(|(key, value)| parse_team(key, value, &affiliations, digest, issues))
                .collect()
        }
        None => {
            issues.push(issue(
                "unknown_teams_shape",
                "allTeams is absent or not an object",
                Some(ev(digest, "/allTeams")),
            ));
            Vec::new()
        }
    };
    (teams, parse_grades(root.get("grades"), digest, issues))
}

fn parse_team(
    key: &str,
    value: &Value,
    affiliations: &BTreeMap<u64, Vec<u16>>,
    digest: &EvidenceDigest,
    issues: &mut Vec<EvidenceIssue>,
) -> Option<TeamEvidence> {
    let id = match key.parse::<u64>().ok().filter(|value| *value > 0) {
        Some(id) => id,
        None => {
            issues.push(issue(
                "invalid_team_id",
                "allTeams key is not a positive integer",
                Some(ev(digest, format!("/allTeams/{key}"))),
            ));
            return None;
        }
    };
    let Some(obj) = value.as_object() else {
        issues.push(issue(
            "unknown_team_shape",
            "allTeams item is not an object",
            Some(ev(digest, format!("/allTeams/{key}"))),
        ));
        return None;
    };
    let name = obj
        .get("SchoolName")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .and_then(|value| SchoolName::parse(value).ok());
    let Some(name) = name else {
        issues.push(issue(
            "team_name_missing",
            "team has no valid SchoolName",
            Some(ev(digest, format!("/allTeams/{key}/SchoolName"))),
        ));
        return None;
    };
    let location = match super::parse_location(obj) {
        Ok(value) => value.map(|value| Observed {
            value,
            evidence: ev(digest, format!("/allTeams/{key}")),
        }),
        Err(error) => {
            issues.push(issue(
                "invalid_team_location",
                &error.to_string(),
                Some(ev(digest, format!("/allTeams/{key}"))),
            ));
            None
        }
    };
    Some(TeamEvidence {
        team_id: id,
        name: Observed {
            value: name,
            evidence: ev(digest, format!("/allTeams/{key}/SchoolName")),
        },
        location,
        seasons: affiliations.get(&id).map_or_else(Vec::new, Clone::clone),
        level: obj
            .get("Level")
            .and_then(Value::as_u64)
            .and_then(|value| u8::try_from(value).ok()),
    })
}

fn parse_grades(
    value: Option<&Value>,
    digest: &EvidenceDigest,
    issues: &mut Vec<EvidenceIssue>,
) -> Vec<GradeAtSeason> {
    let Some(map) = value.and_then(Value::as_object) else {
        if value.is_some() {
            issues.push(issue(
                "unknown_grades_shape",
                "grades is not an object",
                Some(ev(digest, "/grades")),
            ));
        }
        return Vec::new();
    };
    retain_cap_issue(map.len(), "grades_truncated", "/grades", digest, issues);
    map.iter()
        .take(MAX_ITEMS)
        .filter_map(|(key, value)| {
            let parsed = key.split_once('_').and_then(|(team, season)| {
                team.parse::<u64>().ok().zip(season.parse::<u16>().ok())
            });
            let Some((team_id, season)) = parsed else {
                issues.push(issue(
                    "invalid_grade_key",
                    "grade key is not SchoolID_IDSeason",
                    Some(ev(digest, format!("/grades/{key}"))),
                ));
                return None;
            };
            let Some(grade) = value.as_u64().and_then(|item| u8::try_from(item).ok()) else {
                issues.push(issue(
                    "invalid_grade",
                    "grade is not an unsigned integer",
                    Some(ev(digest, format!("/grades/{key}"))),
                ));
                return None;
            };
            Some(GradeAtSeason {
                team_id,
                season,
                grade,
                evidence: ev(digest, format!("/grades/{key}")),
            })
        })
        .collect()
}

struct ParsedResults {
    results: Vec<ResultEvidence>,
    count: u64,
    present: bool,
}
type Distance = (String, Option<String>);
struct ResultContext<'a> {
    athlete: AthleteId,
    sport: Sport,
    events: &'a events::Index,
    distances: &'a BTreeMap<u64, Distance>,
    meets: &'a BTreeMap<u64, Option<String>>,
    teams: &'a BTreeSet<u64>,
    relays: &'a BTreeMap<u64, BTreeSet<u64>>,
    digest: &'a EvidenceDigest,
}

fn parse_results(
    root: &Map<String, Value>,
    athlete: AthleteId,
    sport: Sport,
    digest: &EvidenceDigest,
    issues: &mut Vec<EvidenceIssue>,
) -> ParsedResults {
    let key = match sport {
        Sport::TrackField => "resultsTF",
        Sport::CrossCountry => "resultsXC",
    };
    let Some(value) = root.get(key) else {
        issues.push(issue(
            "missing_sport_results",
            "sport result array is absent",
            Some(ev(digest, format!("/{key}"))),
        ));
        return ParsedResults {
            results: Vec::new(),
            count: 0,
            present: false,
        };
    };
    if value.is_null() {
        return ParsedResults {
            results: Vec::new(),
            count: 0,
            present: false,
        };
    }
    let Some(array) = value.as_array() else {
        issues.push(issue(
            "unknown_results_shape",
            "sport result field is neither null nor an array",
            Some(ev(digest, format!("/{key}"))),
        ));
        return ParsedResults {
            results: Vec::new(),
            count: 0,
            present: false,
        };
    };
    retain_cap_issue(
        array.len(),
        "results_truncated",
        &format!("/{key}"),
        digest,
        issues,
    );
    if root
        .get("eventsTF")
        .is_some_and(|value| !value.is_array() && !value.is_null())
    {
        issues.push(issue(
            "unknown_events_shape",
            "eventsTF is neither null nor an array",
            Some(ev(digest, "/eventsTF")),
        ));
    }
    if root
        .get("distancesXC")
        .is_some_and(|value| !value.is_array() && !value.is_null())
    {
        issues.push(issue(
            "unknown_distances_shape",
            "distancesXC is neither null nor an array",
            Some(ev(digest, "/distancesXC")),
        ));
    }
    if root
        .get("meets")
        .is_some_and(|value| !value.is_object() && !value.is_null())
    {
        issues.push(issue(
            "unknown_meets_shape",
            "meets is neither null nor an object",
            Some(ev(digest, "/meets")),
        ));
    }
    let events = root
        .get("eventsTF")
        .and_then(Value::as_array)
        .map_or_else(BTreeMap::new, |items| events::index(items, digest, issues));
    let distances = root
        .get("distancesXC")
        .and_then(Value::as_array)
        .map_or_else(BTreeMap::new, |items| index_distances(items));
    let meets = root
        .get("meets")
        .and_then(Value::as_object)
        .map_or_else(BTreeMap::new, |map| {
            map.iter()
                .take(MAX_ITEMS)
                .filter_map(|(key, value)| {
                    key.parse::<u64>().ok().map(|id| {
                        (
                            id,
                            value
                                .as_object()
                                .and_then(|obj| optional_text(obj.get("MeetName"))),
                        )
                    })
                })
                .collect::<BTreeMap<_, _>>()
        });
    let relays = relay_members(root.get("relayTeamMembers"), digest, issues);
    let teams = root
        .get("allTeams")
        .and_then(Value::as_object)
        .map_or_else(BTreeSet::new, |map| {
            map.keys()
                .take(MAX_ITEMS)
                .filter_map(|key| key.parse::<u64>().ok().filter(|value| bounded_id(*value)))
                .collect::<BTreeSet<_>>()
        });
    let context = ResultContext {
        athlete,
        sport,
        events: &events,
        distances: &distances,
        meets: &meets,
        teams: &teams,
        relays: &relays,
        digest,
    };
    let results = array
        .iter()
        .take(MAX_ITEMS)
        .enumerate()
        .scan(0_usize, |bytes, (index, item)| {
            let record = parse_result(item, index, &context, issues);
            let size = record.as_ref().map_or(Some(0), result_size);
            let total = size
                .and_then(|size| bytes.checked_add(size))
                .filter(|total| *total <= 8 * 1024 * 1024);
            match total {
                Some(total) => {
                    *bytes = total;
                    Some(record)
                }
                None => {
                    issues.push(issue(
                        "joined_results_truncated",
                        "materialized result evidence exceeds 8 MiB; raw document retained",
                        Some(ev(digest, format!("/{key}/{index}"))),
                    ));
                    None
                }
            }
        })
        .flatten()
        .collect::<Vec<_>>();
    ParsedResults {
        count: u64::try_from(array.len().min(MAX_ITEMS)).map_or(0, |value| value),
        results,
        present: true,
    }
}

fn index_distances(items: &[Value]) -> BTreeMap<u64, Distance> {
    items
        .iter()
        .take(MAX_ITEMS)
        .filter_map(Value::as_object)
        .filter_map(|obj| {
            let id = obj
                .get("Distance")
                .and_then(Value::as_u64)
                .filter(|value| bounded_id(*value))?;
            let distance = display_value(obj.get("Distance")?);
            let units = optional_text(obj.get("Units"));
            let name = units
                .as_ref()
                .map_or_else(|| distance.clone(), |unit| format!("{distance} {unit}"));
            Some((id, (name, units)))
        })
        .collect()
}

fn parse_result(
    item: &Value,
    index: usize,
    context: &ResultContext<'_>,
    issues: &mut Vec<EvidenceIssue>,
) -> Option<ResultEvidence> {
    let Some(obj) = item.as_object() else {
        issues.push(issue(
            "unknown_result_shape",
            "result is not an object",
            Some(ev(context.digest, format!("/results/{index}"))),
        ));
        return None;
    };
    let key = if context.sport == Sport::TrackField {
        "resultsTF"
    } else {
        "resultsXC"
    };
    let locator = format!("/{key}/{index}");
    let result_id = required_u64(obj, "IDResult", &locator, context.digest, issues);
    let reported = obj
        .get("AthleteID")
        .and_then(Value::as_u64)
        .filter(|value| bounded_id(*value));
    let season = required_u16(obj, "SeasonID", &locator, context.digest, issues);
    let team_id = obj
        .get("SchoolID")
        .and_then(Value::as_u64)
        .filter(|value| bounded_id(*value))
        .map_or(0, |value| value);
    let meet_id = obj
        .get("MeetID")
        .and_then(Value::as_u64)
        .filter(|value| bounded_id(*value))
        .map_or(0, |value| value);
    if team_id == 0 || !context.teams.contains(&team_id) {
        issues.push(issue(
            "missing_team_join",
            "result SchoolID does not join allTeams",
            Some(ev(context.digest, locator.clone())),
        ));
    }
    if meet_id == 0 || !context.meets.contains_key(&meet_id) {
        issues.push(issue(
            "missing_meet_join",
            "result MeetID does not join meets",
            Some(ev(context.digest, locator.clone())),
        ));
    }
    let (event_id, event_name, event_description, event_type, units, personal_event) =
        event_info(obj, context, &locator, issues);
    let mark = obj
        .get("Result")
        .filter(|value| !value.is_null())
        .map(display_value)
        .unwrap_or_else(|| {
            issues.push(issue(
                "missing_result_mark",
                "result has no Result display mark",
                Some(ev(context.digest, locator.clone())),
            ));
            String::new()
        });
    let short_code = obj
        .get("shortCode")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| valid_short_code(value))
        .map(str::to_owned);
    if short_code.is_none() {
        issues.push(issue(
            "missing_short_code",
            "result has no valid shortCode",
            Some(ev(context.digest, locator.clone())),
        ));
    }
    let result_url = short_code
        .as_ref()
        .filter(|_| reported.is_some_and(|value| value > 0) && personal_event && meet_id > 0)
        .map(|code| format!("http://www.athletic.net/result/{code}"));
    Some(ResultEvidence {
        result_id,
        sport: context.sport,
        event_id,
        event_name,
        event_description,
        event_type,
        mark,
        units,
        season,
        team_id,
        meet_id,
        meet_name: optional_text(obj.get("meetName"))
            .or_else(|| context.meets.get(&meet_id).cloned().flatten()),
        date: optional_text(obj.get("ResultDate")),
        wind: optional_display(obj.get("Wind")),
        timing: timing(obj.get("FAT")),
        personal_best: best_claim(obj.get("PersonalBest"), context.sport),
        season_best: best_claim(obj.get("SeasonBest"), context.sport),
        attribution: attribution(context.athlete, reported, personal_event, context.relays),
        short_code,
        result_url,
        evidence: ev(context.digest, locator),
    })
}
fn event_info(
    obj: &Map<String, Value>,
    context: &ResultContext<'_>,
    locator: &str,
    issues: &mut Vec<EvidenceIssue>,
) -> (
    Option<u64>,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    bool,
) {
    match context.sport {
        Sport::TrackField => {
            let id = obj
                .get("EventID")
                .and_then(Value::as_u64)
                .filter(|value| bounded_id(*value));
            let event = id.and_then(|id| events::lookup(context.events, id, obj));
            match event {
                Some(event) => (
                    id,
                    event.name.clone(),
                    event.description.clone(),
                    event.kind.clone(),
                    event.units.clone(),
                    event.personal,
                ),
                None => {
                    issues.push(issue(
                        "missing_event_join",
                        "TF result event and variant do not uniquely join eventsTF",
                        Some(ev(context.digest, locator)),
                    ));
                    (id, "Unknown event".to_owned(), None, None, None, false)
                }
            }
        }
        Sport::CrossCountry => {
            let id = obj
                .get("Distance")
                .and_then(Value::as_u64)
                .filter(|value| bounded_id(*value));
            let Some(id) = id else {
                issues.push(issue(
                    "missing_distance_join",
                    "XC result has no Distance",
                    Some(ev(context.digest, locator)),
                ));
                return (None, "Unknown distance".to_owned(), None, None, None, true);
            };
            let Some((name, units)) = context.distances.get(&id) else {
                issues.push(issue(
                    "missing_distance_join",
                    "XC result Distance does not join distancesXC",
                    Some(ev(context.digest, locator)),
                ));
                return (None, "Unknown distance".to_owned(), None, None, None, true);
            };
            (None, name.clone(), None, None, units.clone(), true)
        }
    }
}

fn result_size(result: &ResultEvidence) -> Option<usize> {
    [
        Some(result.event_name.as_str()),
        result.event_description.as_deref(),
        result.event_type.as_deref(),
        Some(result.mark.as_str()),
        result.units.as_deref(),
        result.meet_name.as_deref(),
        result.date.as_deref(),
        result.wind.as_deref(),
        result.timing.as_deref(),
        result.short_code.as_deref(),
        result.result_url.as_deref(),
        Some(result.evidence.locator.as_str()),
        Some(result.evidence.document.as_str()),
    ]
    .into_iter()
    .flatten()
    .try_fold(std::mem::size_of::<ResultEvidence>(), |bytes, text| {
        bytes.checked_add(text.len())
    })
}

fn relay_members(
    value: Option<&Value>,
    digest: &EvidenceDigest,
    issues: &mut Vec<EvidenceIssue>,
) -> BTreeMap<u64, BTreeSet<u64>> {
    let Some(value) = value.filter(|item| !item.is_null()) else {
        return BTreeMap::new();
    };
    let Some(items) = value.as_array() else {
        issues.push(issue(
            "unknown_relay_members_shape",
            "relayTeamMembers is not an array",
            Some(ev(digest, "/relayTeamMembers")),
        ));
        return BTreeMap::new();
    };
    if items.len() > MAX_ITEMS {
        issues.push(issue(
            "relay_members_truncated",
            "relay membership exceeds parser bound",
            Some(ev(digest, "/relayTeamMembers")),
        ));
    }
    items
        .iter()
        .take(MAX_ITEMS)
        .enumerate()
        .fold(BTreeMap::new(), |mut members, (index, item)| {
            let team = item
                .get("TeamID")
                .and_then(Value::as_u64)
                .filter(|id| bounded_id(*id));
            let athlete = item
                .get("AthleteID")
                .and_then(Value::as_u64)
                .filter(|id| bounded_id(*id));
            match (team, athlete) {
                (Some(team), Some(athlete)) => {
                    members.entry(team).or_default().insert(athlete);
                }
                _ => issues.push(issue(
                    "invalid_relay_member",
                    "relay membership requires positive TeamID and AthleteID",
                    Some(ev(digest, format!("/relayTeamMembers/{index}"))),
                )),
            }
            members
        })
}
fn valid_short_code(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}
fn attribution(
    athlete: AthleteId,
    reported: Option<u64>,
    personal: bool,
    relays: &BTreeMap<u64, BTreeSet<u64>>,
) -> ResultAttribution {
    match (reported, personal) {
        (Some(id), true) if id == athlete.get() => ResultAttribution::Individual,
        (Some(id), false)
            if relays
                .get(&id)
                .is_some_and(|members| members.contains(&athlete.get())) =>
        {
            ResultAttribution::VerifiedRelayMember {
                relay_athlete_id: id,
            }
        }
        (id, _) => ResultAttribution::Unresolved {
            reported_athlete_id: id,
        },
    }
}

fn retain_cap_issue(
    length: usize,
    code: &str,
    locator: &str,
    digest: &EvidenceDigest,
    issues: &mut Vec<EvidenceIssue>,
) {
    if length > MAX_ITEMS {
        issues.push(issue(
            code,
            "source collection exceeded parser retention bound",
            Some(ev(digest, locator)),
        ));
    }
}
fn best_claim(value: Option<&Value>, sport: Sport) -> BestClaim {
    match sport {
        Sport::TrackField => value
            .and_then(Value::as_u64)
            .map_or(BestClaim::Unavailable, BestClaim::OpaqueFlags),
        Sport::CrossCountry => {
            value
                .and_then(Value::as_bool)
                .map_or(BestClaim::Unavailable, |flag| {
                    if flag {
                        BestClaim::Claimed
                    } else {
                        BestClaim::NotClaimed
                    }
                })
        }
    }
}
fn timing(value: Option<&Value>) -> Option<String> {
    value.and_then(Value::as_u64).map(|item| {
        if item == 1 {
            "FAT".to_owned()
        } else if item == 0 {
            "hand".to_owned()
        } else {
            item.to_string()
        }
    })
}
fn availability(sport: Sport, count: u64, present: bool) -> SportAvailability {
    if !present {
        SportAvailability::Unavailable { sport }
    } else if count == 0 {
        SportAvailability::EmptyResponse { sport }
    } else {
        SportAvailability::ResultsObserved { sport, count }
    }
}
fn required_u64(
    obj: &Map<String, Value>,
    key: &str,
    locator: &str,
    digest: &EvidenceDigest,
    issues: &mut Vec<EvidenceIssue>,
) -> u64 {
    obj.get(key)
        .and_then(Value::as_u64)
        .filter(|value| bounded_id(*value))
        .unwrap_or_else(|| {
            issues.push(issue(
                "invalid_result_id",
                &format!("result {key} is not a bounded positive integer"),
                Some(ev(digest, locator)),
            ));
            0
        })
}
fn required_u16(
    obj: &Map<String, Value>,
    key: &str,
    locator: &str,
    digest: &EvidenceDigest,
    issues: &mut Vec<EvidenceIssue>,
) -> u16 {
    obj.get(key)
        .and_then(Value::as_u64)
        .and_then(|value| u16::try_from(value).ok())
        .unwrap_or_else(|| {
            issues.push(issue(
                "invalid_season",
                "result SeasonID is invalid",
                Some(ev(digest, locator)),
            ));
            0
        })
}
fn optional_text(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}
fn optional_display(value: Option<&Value>) -> Option<String> {
    value.filter(|item| !item.is_null()).map(display_value)
}
fn display_value(value: &Value) -> String {
    value
        .as_str()
        .map_or_else(|| value.to_string(), str::to_owned)
}
fn ev(digest: &EvidenceDigest, locator: impl Into<String>) -> EvidenceRef {
    EvidenceRef {
        document: digest.clone(),
        locator: locator.into(),
    }
}
fn bounded_id(value: u64) -> bool {
    value > 0 && value <= MAX_SOURCE_ID
}
fn issue(code: &str, message: &str, evidence: Option<EvidenceRef>) -> EvidenceIssue {
    EvidenceIssue {
        code: code.to_owned(),
        message: message.to_owned(),
        evidence,
    }
}
