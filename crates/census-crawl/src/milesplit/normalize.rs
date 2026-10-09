use super::wire::{RosterAthlete, Site, TeamRef};
use crate::{CrawlError, CrawlResult};
use census_domain::model::{
    normalize_name, CanonicalAthlete, CanonicalSchool, CanonicalTeam, Evidence, Gender,
    PublishedGraduation, SchoolId, SchoolYear, SourceIdentity, SourceNamespace, SourceRef, Sport,
};

const MAX_WINDOW_ROWS: usize = 64;
const MAX_WINDOW_BYTES: usize = 8 * 1024 * 1024;

impl RosterAthlete {
    pub fn sports(&self) -> Vec<Sport> {
        sport_iter(self).collect()
    }
}

fn sport_iter(entry: &RosterAthlete) -> impl Iterator<Item = Sport> {
    [
        (Sport::IndoorTrack, entry.indoor),
        (Sport::OutdoorTrack, entry.outdoor),
        (Sport::CrossCountry, entry.xc),
    ]
    .into_iter()
    .filter_map(|(sport, active)| active.then_some(sport))
}

struct Projection<'a> {
    school: &'a SchoolId,
    source: &'a SourceRef,
    team: &'a TeamRef,
    year: SchoolYear,
    at: &'a str,
    site: &'a Site,
}

pub fn roster_entities(
    team: &TeamRef,
    entries: &[RosterAthlete],
    school_year: SchoolYear,
    observed_on: &str,
    site: &Site,
) -> CrawlResult<(CanonicalSchool, Vec<CanonicalAthlete>, Vec<CanonicalTeam>)> {
    admit(team, entries, observed_on)?;
    let mut athletes = Vec::new();
    athletes
        .try_reserve_exact(entries.len())
        .map_err(|_| resource("roster projection rows", entries.len(), MAX_WINDOW_ROWS))?;
    let (school, school_id, source) = roster_school(team, observed_on, site)?;
    let projection = Projection {
        school: &school_id,
        source: &source,
        team,
        year: school_year,
        at: observed_on,
        site,
    };
    let programs = entries.iter().try_fold(0u16, |programs, entry| {
        athletes.push(roster_athlete_entity(entry, &projection)?);
        Ok::<_, CrawlError>(sport_iter(entry).fold(programs, |mask, sport| {
            mask | program_bit(sport, entry.gender)
        }))
    })?;
    Ok((school, athletes, roster_teams(programs, &projection)?))
}

fn admit(team: &TeamRef, entries: &[RosterAthlete], at: &str) -> CrawlResult<()> {
    if entries.len() > MAX_WINDOW_ROWS {
        return Err(resource(
            "roster projection rows",
            entries.len(),
            MAX_WINDOW_ROWS,
        ));
    }
    let initial = [
        &team.id,
        &team.slug,
        &team.url,
        &team.name,
        &team.city_state,
    ]
    .into_iter()
    .try_fold(at.len(), |total, value| add(total, value.len()))?;
    let bytes = entries.iter().try_fold(initial, |total, row| {
        [
            &row.name,
            &row.roster_name,
            &row.athlete_id,
            &row.profile_url,
        ]
        .into_iter()
        .try_fold(total, |total, value| add(total, value.len()))
    })?;
    let estimated = bytes
        .checked_mul(16)
        .ok_or_else(|| resource("roster projection bytes", usize::MAX, MAX_WINDOW_BYTES))?;
    if estimated > MAX_WINDOW_BYTES {
        return Err(resource(
            "roster projection bytes",
            estimated,
            MAX_WINDOW_BYTES,
        ));
    }
    if team.name.trim().is_empty() {
        return Err(CrawlError::Invariant {
            detail: "roster school name is empty".into(),
        });
    }
    Ok(())
}

fn roster_school(
    team: &TeamRef,
    at: &str,
    site: &Site,
) -> CrawlResult<(CanonicalSchool, SchoolId, SourceRef)> {
    let source = SourceRef::new(site.source_id(), Some(format!("{}/roster", team.url)));
    let owner = owner_name(team);
    let city = city_of(&team.city_state)?;
    let (mut school, school_id) = CanonicalSchool::new(
        site.jurisdiction(),
        owner,
        normalize_name(owner),
        city.as_deref(),
    );
    reserve(&mut school.source_identities, 1)?;
    reserve(&mut school.evidence, 1)?;
    school.source_identities.push(
        SourceIdentity::new(SourceNamespace::MilesplitSchool, team.id.clone())
            .with_url(team.url.clone()),
    );
    school
        .evidence
        .push(Evidence::parsed(source.clone(), at.to_owned()));
    Ok((school, school_id, source))
}

fn roster_athlete_entity(
    entry: &RosterAthlete,
    projection: &Projection<'_>,
) -> CrawlResult<CanonicalAthlete> {
    let mut athlete = CanonicalAthlete::new_checked(
        projection.school,
        entry.name.clone(),
        entry.grad_year,
        entry.gender,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, entry.athlete_id.clone())
            .with_url(entry.profile_url.clone()),
    )
    .map_err(|detail| CrawlError::Invariant { detail })?;
    reserve_athlete(&mut athlete)?;
    if entry.roster_name != entry.name {
        athlete.known_names.push(entry.roster_name.clone());
    }
    athlete.sports.extend(sport_iter(entry));
    athlete.published_graduations.push(PublishedGraduation {
        grad_year: entry.grad_year,
        source: projection.source.clone(),
    });
    athlete.public_profile_urls.push(entry.profile_url.clone());
    athlete.evidence.push(Evidence::parsed(
        SourceRef::new(projection.site.source_id(), Some(entry.profile_url.clone())),
        projection.at.to_owned(),
    ));
    Ok(athlete)
}

fn reserve_athlete(athlete: &mut CanonicalAthlete) -> CrawlResult<()> {
    reserve(&mut athlete.known_names, 1)?;
    reserve(&mut athlete.sports, 3)?;
    reserve(&mut athlete.published_graduations, 1)?;
    reserve(&mut athlete.public_profile_urls, 1)?;
    reserve(&mut athlete.evidence, 1)
}

fn roster_teams(mask: u16, projection: &Projection<'_>) -> CrawlResult<Vec<CanonicalTeam>> {
    let mut teams = Vec::new();
    let programs = usize::try_from(mask.count_ones()).map_err(|_| CrawlError::Arithmetic {
        detail: format!("milesplit roster program mask {mask} population exceeds usize"),
    })?;
    reserve(&mut teams, programs)?;
    [Sport::IndoorTrack, Sport::OutdoorTrack, Sport::CrossCountry]
        .into_iter()
        .for_each(|sport| {
            [Gender::Boys, Gender::Girls, Gender::Mixed, Gender::Unknown]
                .into_iter()
                .filter(|gender| mask & program_bit(sport, *gender) != 0)
                .for_each(|gender| teams.push(roster_team((sport, gender), projection)));
        });
    Ok(teams)
}

fn roster_team((sport, gender): (Sport, Gender), projection: &Projection<'_>) -> CanonicalTeam {
    CanonicalTeam {
        id: CanonicalTeam::mint(projection.school, sport, gender, projection.year),
        school: projection.school.clone(),
        sport,
        gender,
        school_year: projection.year,
        level: Some("high_school".into()),
        source_identities: vec![SourceIdentity::new(
            SourceNamespace::MilesplitTeam,
            projection.team.id.clone(),
        )
        .with_url(projection.team.url.clone())],
        evidence: vec![Evidence::parsed(
            projection.source.clone(),
            projection.at.to_owned(),
        )],
        retained_conflicts: Vec::new(),
    }
}

fn program_bit(sport: Sport, gender: Gender) -> u16 {
    let sport: u32 = match sport {
        Sport::IndoorTrack => 0,
        Sport::OutdoorTrack => 1,
        Sport::CrossCountry => 2,
        Sport::Unknown => 3,
    };
    let gender: u32 = match gender {
        Gender::Boys => 0,
        Gender::Girls => 1,
        Gender::Mixed => 2,
        Gender::Unknown => 3,
    };
    let shift = sport.saturating_mul(4).saturating_add(gender);
    1u16 << shift
}

fn owner_name(team: &TeamRef) -> &str {
    let name = team.name.trim();
    [" Boys", " Girls", " (B)", " (G)"]
        .into_iter()
        .find_map(|suffix| name.strip_suffix(suffix))
        .map_or(name, str::trim)
}

fn city_of(city_state: &str) -> CrawlResult<Option<String>> {
    let first = city_state.split(',').next().map_or("", str::trim);
    if first.is_empty() {
        Ok(None)
    } else {
        title_case(first).map(Some)
    }
}

fn title_case(value: &str) -> CrawlResult<String> {
    let capacity = value
        .len()
        .checked_mul(3)
        .ok_or_else(|| resource("roster city bytes", usize::MAX, MAX_WINDOW_BYTES))?;
    let mut output = String::new();
    output
        .try_reserve_exact(capacity)
        .map_err(|_| resource("roster city allocation", capacity, MAX_WINDOW_BYTES))?;
    value.split_whitespace().for_each(|word| {
        if !output.is_empty() {
            output.push(' ');
        }
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            output.extend(first.to_uppercase());
        }
        output.extend(chars.flat_map(char::to_lowercase));
    });
    Ok(output)
}

fn reserve<T>(values: &mut Vec<T>, extra: usize) -> CrawlResult<()> {
    values
        .try_reserve_exact(extra)
        .map_err(|_| resource("roster projection allocation", extra, MAX_WINDOW_ROWS))
}

fn add(left: usize, right: usize) -> CrawlResult<usize> {
    left.checked_add(right)
        .ok_or_else(|| resource("roster projection bytes", usize::MAX, MAX_WINDOW_BYTES))
}

fn resource(resource: &'static str, requested: usize, limit: usize) -> CrawlError {
    CrawlError::Resource {
        resource,
        requested,
        limit,
    }
}
