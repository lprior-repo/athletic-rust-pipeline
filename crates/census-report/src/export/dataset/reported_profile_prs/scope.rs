use crate::export::dataset::Loaded;
use crate::report::population;
use crate::report::ReportResult;
use census_domain::model::{CanonicalAthlete, EventId, QueryProfileV1, SchoolId, TeamId};
use census_domain::UsJurisdiction;
use census_store::AcceptedPublicIdentitySnapshot;
use std::collections::{BTreeMap, BTreeSet, HashMap};

pub(in crate::export::dataset) struct SourceScope<'a> {
    pub(in crate::export::dataset) profile: &'a QueryProfileV1,
    pub(in crate::export::dataset) accepted: &'a AcceptedPublicIdentitySnapshot,
}

pub(super) fn athletes<'a>(
    loaded: &'a Loaded,
    scope: SourceScope<'_>,
) -> ReportResult<Vec<&'a CanonicalAthlete>> {
    let profile = scope.profile;
    let aliases = crate::export::public_identity::source_aliases(scope.accepted, &loaded.athletes)?;
    let collapsed = population::accepted_members(&loaded.athletes, &aliases);
    let states = loaded
        .schools
        .iter()
        .map(|school| (&school.id, school.state))
        .collect::<BTreeMap<_, _>>();
    let teams = loaded
        .teams
        .iter()
        .filter(|team| {
            states
                .get(&team.school)
                .copied()
                .flatten()
                .is_some_and(|state| profile.admits_jurisdiction(state))
        })
        .map(|team| &team.id)
        .collect::<BTreeSet<_>>();
    let performers = performers(loaded, profile, &teams, &aliases);
    let rosters = rosters(loaded, profile, &states, &aliases);
    let selected = collapsed
        .iter()
        .filter(|athlete| {
            population::cohort(athlete, profile)
                && (rosters.contains(athlete.id.as_str())
                    || performers.contains(athlete.id.as_str()))
        })
        .map(|athlete| athlete.id.as_str())
        .collect::<BTreeSet<_>>();
    Ok(loaded
        .athletes
        .iter()
        .filter(|athlete| {
            let subject = athlete.id.as_str();
            selected.contains(aliases.get(subject).map_or(subject, String::as_str))
        })
        .collect())
}

fn rosters<'a>(
    loaded: &'a Loaded,
    profile: &QueryProfileV1,
    states: &BTreeMap<&SchoolId, Option<UsJurisdiction>>,
    aliases: &'a HashMap<String, String>,
) -> BTreeSet<&'a str> {
    loaded
        .athletes
        .iter()
        .filter(|athlete| {
            population::roster(
                athlete,
                profile,
                states.get(&athlete.school).copied().flatten(),
            )
        })
        .map(|athlete| {
            let subject = athlete.id.as_str();
            aliases.get(subject).map_or(subject, String::as_str)
        })
        .collect()
}

fn performers<'a>(
    loaded: &'a Loaded,
    profile: &QueryProfileV1,
    teams: &BTreeSet<&TeamId>,
    aliases: &'a HashMap<String, String>,
) -> BTreeSet<&'a str> {
    let events = events(loaded, profile);
    loaded
        .performances
        .iter()
        .filter(|performance| {
            teams.contains(&performance.team) && events.contains(&performance.event)
        })
        .map(|performance| {
            let subject = performance.athlete.as_str();
            aliases.get(subject).map_or(subject, String::as_str)
        })
        .collect()
}

fn events<'a>(loaded: &'a Loaded, profile: &QueryProfileV1) -> BTreeSet<&'a EventId> {
    let meets = loaded
        .meets
        .iter()
        .filter(|meet| {
            profile
                .input()
                .sports
                .contains(&population::meet_sport(meet))
        })
        .map(|meet| &meet.id)
        .collect::<BTreeSet<_>>();
    loaded
        .events
        .iter()
        .filter(|event| {
            meets.contains(&event.meet)
                && profile
                    .input()
                    .competition_categories
                    .contains(&event.gender)
        })
        .map(|event| &event.id)
        .collect()
}
