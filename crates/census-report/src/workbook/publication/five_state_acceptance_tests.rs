use crate::export::ExportDataset;
use crate::report::Scope;
use crate::workbook::{self, Censuses, Options};
use census_domain::model::*;
use census_domain::school_directory::{
    AddressKind, CityName, PostalAddress, SourceLabel, StreetLine, ZipCode,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use serde_json::Value;
use std::path::Path;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const STATES: [UsJurisdiction; 5] = [
    UsJurisdiction::Colorado,
    UsJurisdiction::Illinois,
    UsJurisdiction::Iowa,
    UsJurisdiction::Michigan,
    UsJurisdiction::Minnesota,
];

#[test]
fn synthetic_five_state_workbook_keeps_provenance_wind_and_relay_facts_after_frozen_reopen(
) -> TestResult {
    let directory = tempfile::tempdir()?;
    let store_path = directory.path().join("store");
    let store = Store::open(&store_path)?;
    let (athletes, teams, relay_only, relay) = seed(&store)?;
    let dataset = ExportDataset::load(&store)?;
    let options = Options {
        address_corpus: None,
        grad_year: Some(2027),
        out: None,
        limit: None,
        scope: Scope::AllSources,
        school_year: SchoolYear::new(2026).ok_or("fixture school year")?,
        jurisdictions: Vec::new(),
    };
    let censuses = Censuses::of(&dataset, &store.out_dir(), &options.jurisdictions)?;
    let workbook_path = workbook::build_from(&dataset, &store, &options, &censuses)?;
    let generated = workbook_path
        .parent()
        .ok_or("published workbook generation")?;
    let workbook_name = workbook_path
        .file_name()
        .ok_or("published workbook filename")?;
    let generation = directory.path().join("retained-generation");
    std::fs::rename(generated, &generation)?;
    let workbook_path = generation.join(workbook_name);
    let frozen_path = generation.join("frozen-input.json");

    super::verify_sidecars::verify(&generation, &dataset, &options, 6)?;
    workbook::verify::verify_frozen(&workbook_path, &dataset, &options)?;
    let initial = super::verify_published(&workbook_path)?;
    let initial_mapped = initial.workbook.mapped_athletes;
    check!(initial_mapped >= STATES.len() as u64);

    let schools = read_csv(&generation.join("schools.csv"))?;
    check!(eq; schools.len(), STATES.len());
    for state in STATES {
        let code = state.to_string();
        check!(schools
            .iter()
            .any(|row| row.get("postal_state") == Some(&code)));
    }
    check!(schools.iter().all(|row| {
        row.get("postal_street")
            .is_some_and(|value| value.contains("Synthetic Way"))
    }));
    check!(schools.iter().all(|row| {
        row.get("postal_city")
            .is_some_and(|value| value.ends_with("City"))
    }));
    check!(schools
        .iter()
        .all(|row| row.get("postal_zip").is_some_and(|value| value.len() == 5)));
    check!(schools.iter().all(|row| {
        row.get("postal_owner_namespace")
            .is_some_and(|value| value == "association_school:synthetic-association")
    }));
    check!(schools.iter().all(|row| {
        row.get("postal_source_url")
            .is_some_and(|url| url.starts_with("https://synthetic.example.test/address/"))
    }));
    check!(schools
        .iter()
        .all(|row| row.get("postal_observed_date") == Some(&"2026-09-01".to_owned())));
    check!(schools.iter().all(|row| {
        row.get("postal_capture_sha256")
            .is_some_and(|value| value.len() == 64)
    }));
    check!(schools.iter().all(|row| row
        .get("postal_owner_id")
        .is_some_and(|value| value.starts_with("synthetic-school-"))));
    check!(schools.iter().all(|row| row
        .get("postal_source")
        .is_some_and(|value| value.starts_with("athletic-association:"))));
    check!(schools
        .iter()
        .all(|row| row.get("postal_address_kind") == Some(&"physical".to_owned())));
    for row in &schools {
        let claim_ids: Value = serde_json::from_str(
            row.get("address_claim_ids")
                .ok_or("normalized address claim ids")?,
        )?;
        let evidence_refs: Value = serde_json::from_str(
            row.get("address_evidence_refs")
                .ok_or("normalized address evidence refs")?,
        )?;
        check!(eq; claim_ids.as_array().map(Vec::len), Some(1));
        check!(eq; evidence_refs.as_array().map(Vec::len), Some(1));
    }
    let original_address_digests = address_digests(&dataset)?;

    let contacts = read_csv(&generation.join("coach_contacts.csv"))?;
    check!(eq; contacts.len(), STATES.len());
    check!(contacts.iter().all(|row| {
        row.get("source_url")
            .is_some_and(|url| url.starts_with("https://synthetic.example.test/coach/"))
    }));
    check!(contacts.iter().all(|row| {
        row.get("capture_content_sha256")
            .is_some_and(|value| value.len() == 64)
    }));
    check!(contacts
        .iter()
        .all(|row| row.get("retrieved_at") == Some(&"2026-09-01T12:00:00Z".to_owned())));

    let best_marks = read_csv(&generation.join("best_marks.csv"))?;
    check!(eq; best_marks.len(), STATES.len());
    check!(best_marks.iter().all(|row| {
        row.get("source_url")
            .is_some_and(|url| url.starts_with("https://synthetic.example.test/results/"))
    }));
    check!(best_marks
        .iter()
        .any(|row| row.get("wind_class").is_some_and(|class| class == "legal")));
    check!(best_marks.iter().any(|row| {
        row.get("wind_class")
            .is_some_and(|class| class == "unknown")
    }));
    check!(!best_marks.iter().any(|row| {
        row.get("event_label")
            .is_some_and(|label| label.contains("400m Split"))
    }));
    check!(!best_marks
        .iter()
        .any(|row| row.get("athlete_id") == Some(&relay_only.to_string())));

    let relays = read_jsonl(&generation.join("relays.jsonl"))?;
    let retained_relay = relays
        .iter()
        .find(|row| row.pointer("/result/id").and_then(Value::as_str) == Some(relay.as_str()))
        .ok_or("synthetic member-only relay was published")?;
    check!(
        retained_relay
            .pointer("/result/members/0/athlete")
            .and_then(Value::as_str)
            == Some(athletes[0].as_str())
    );
    check!(
        retained_relay
            .pointer("/result/members/1/athlete")
            .and_then(Value::as_str)
            == Some(relay_only.as_str())
    );
    check!(eq; teams.len(), STATES.len());

    let published_digest = initial.generation_digest;
    drop(censuses);
    drop(dataset);
    drop(store);
    std::fs::remove_dir_all(&store_path)?;

    let reopened = ExportDataset::reopen_frozen(&frozen_path)?;
    check!(eq; reopened.athletes.len(), STATES.len() + 1);
    check!(eq; reopened.relays.len(), 1);
    check!(eq; address_digests(&reopened)?, original_address_digests);
    check!(!reopened
        .performances
        .iter()
        .any(|performance| performance.athlete == relay_only));
    super::verify_sidecars::verify(&generation, &reopened, &options, 6)?;
    workbook::verify::verify_frozen(&workbook_path, &reopened, &options)?;
    let reopened_publication = super::verify_published(&workbook_path)?;
    check!(eq; reopened_publication.generation_digest, published_digest);
    check!(eq; reopened_publication.workbook.mapped_athletes, initial_mapped);
    Ok(())
}

fn address_digests(dataset: &ExportDataset) -> TestResult<Vec<(UsJurisdiction, String)>> {
    STATES
        .into_iter()
        .map(|state| {
            let school = dataset
                .schools
                .values()
                .find(|school| school.state == Some(state))
                .ok_or("synthetic state school in frozen dataset")?;
            if school.postal_addresses.len() != 1 {
                return Err(format!("expected one synthetic address claim for {state}").into());
            }
            Ok((
                state,
                census_domain::model::serialized_digest(&school.postal_addresses)?,
            ))
        })
        .collect()
}

fn seed(store: &Store) -> TestResult<([AthleteId; 5], [TeamId; 5], AthleteId, RelayId)> {
    let mut athletes = Vec::with_capacity(STATES.len());
    let mut teams = Vec::with_capacity(STATES.len());
    let mut first_athlete = None;
    let mut first_school = None;
    let mut first_meet = None;
    let mut first_team = None;

    for (index, state) in STATES.into_iter().enumerate() {
        let city = format!("{} City", state);
        let (mut school, school_id) = CanonicalSchool::new(
            state,
            format!("Synthetic {state} High"),
            format!("synthetic {state} high").to_ascii_lowercase(),
            Some(&city),
        );
        school.source_identities.push(SourceIdentity::new(
            SourceNamespace::association_school("synthetic-association"),
            format!("synthetic-school-{index}"),
        ));
        school.add_postal_address(postal_address(state, index)?)?;
        store.append(Table::Schools, &school)?;

        let coach_source = SourceRef::new(
            format!("synthetic_coach_{index}"),
            Some(format!("https://synthetic.example.test/coach/{state}")),
        );
        let mut coach = CanonicalCoach::new(
            &school_id,
            format!("Synthetic Coach {state}"),
            Some(Sport::OutdoorTrack),
            Gender::Girls,
            CoachRole::HeadCoach,
        );
        coach.professional_email = Some(format!("coach{index}@synthetic.example.test"));
        coach
            .evidence
            .push(Evidence::parsed(coach_source.clone(), "2026-09-01"));
        coach.tenure_evidence.push(CoachTenureEvidence {
            tenure: CoachTenure::Current {
                school_year: SchoolYear::new(2026).ok_or("fixture school year")?,
            },
            source: coach_source,
            source_sha256: format!("{index:064x}"),
            retrieved_at: "2026-09-01T12:00:00Z".into(),
            statement: "Synthetic test fixture coaching claim".into(),
            claim: Some(CoachContactClaim {
                coach: coach.id.clone(),
                school: school_id.clone(),
                role: CoachRole::HeadCoach,
                program: CoachContactProgram::Team {
                    sport: Sport::OutdoorTrack,
                    gender: Gender::Girls,
                },
                mailbox: coach.professional_email.clone(),
            }),
        });
        store.append(Table::Coaches, &coach)?;

        let mut athlete = CanonicalAthlete::new(
            &school_id,
            format!("Synthetic Runner {state}"),
            GradYear::CO2027,
            Gender::Girls,
            SourceIdentity::new(
                SourceNamespace::MilesplitAthlete,
                format!("synthetic-{index}"),
            ),
        )?;
        athlete.sports.push(Sport::OutdoorTrack);
        athlete.published_graduations.push(PublishedGraduation {
            grad_year: GradYear::CO2027,
            source: SourceRef::new(
                format!("synthetic_roster_{index}"),
                Some(format!("https://synthetic.example.test/roster/{state}")),
            ),
        });
        store.append(Table::Athletes, &athlete)?;

        let meet_date = "2026-05-02";
        let mut meet = CanonicalMeet::new(
            Some(state),
            format!("Synthetic {state} Invite"),
            meet_date,
            CompetitionLevel::Unknown,
        );
        meet.sports = vec![Sport::OutdoorTrack];
        store.append(Table::Meets, &meet)?;
        let year = SchoolYear::from_date(meet_date).ok_or("fixture season")?;
        let owner = SourceIdentity::new(
            SourceNamespace::MilesplitTeam,
            format!("synthetic-team-{index}"),
        );
        let team = CanonicalTeam {
            id: CanonicalTeam::mint(&school_id, Sport::OutdoorTrack, Gender::Girls, year),
            school: school_id.clone(),
            sport: Sport::OutdoorTrack,
            gender: Gender::Girls,
            school_year: year,
            level: None,
            source_identities: vec![owner.clone()],
            evidence: vec![Evidence::parsed(result_source(state), meet_date)],
            retained_conflicts: Vec::new(),
        };
        store.append(Table::Teams, &team)?;
        let event = event(&meet, EventKind::Track100m, "100m", state)?;
        store.append(Table::Events, &event)?;
        let wind = if index == 0 { Some(1.0) } else { None };
        let performance = performance(&athlete, &team.id, &event, state, wind)?;
        store.append(Table::Performances, &performance)?;

        if index == 0 {
            first_school = Some((school_id.clone(), state));
            first_meet = Some(meet);
            first_team = Some((team.clone(), owner));
            first_athlete = Some(athlete.clone());
        }
        athletes.push(athlete.id);
        teams.push(team.id);
    }

    let (school_id, state) = first_school.ok_or("first synthetic school")?;
    let first_athlete = first_athlete.ok_or("first synthetic athlete")?;
    let meet = first_meet.ok_or("first synthetic meet")?;
    let (team, owner) = first_team.ok_or("first synthetic team")?;
    let mut relay_only = CanonicalAthlete::new(
        &school_id,
        "Synthetic Relay Only Runner",
        GradYear::CO2027,
        Gender::Girls,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "synthetic-relay-only"),
    )?;
    relay_only.published_graduations.push(PublishedGraduation {
        grad_year: GradYear::CO2027,
        source: SourceRef::new("synthetic_relay_roster", None),
    });
    store.append(Table::Athletes, &relay_only)?;

    let split = event(
        &meet,
        EventKind::Unmapped {
            label: "400m Split".into(),
        },
        "400m Split",
        state,
    )?;
    store.append(Table::Events, &split)?;
    store.append(
        Table::Performances,
        &performance(&first_athlete, &team.id, &split, state, None)?,
    )?;

    let relay_event = event(&meet, EventKind::Relay4x100, "4x100 Relay", state)?;
    store.append(Table::Events, &relay_event)?;
    let relay = RelayResult::new(
        &team.id,
        &relay_event.id,
        &meet.id,
        Mark::TimeSeconds(ExactSeconds::parse("52.00")?),
        "synthetic:member-only-relay",
    )
    .with_source_team(owner)
    .with_member(RelayMember::new(1, "Synthetic Runner Colorado").with_athlete(athletes[0].clone()))
    .with_member(
        RelayMember::new(2, "Synthetic Relay Only Runner").with_athlete(relay_only.id.clone()),
    )
    .with_member(RelayMember::new(3, "Synthetic Runner Three"))
    .with_member(RelayMember::new(4, "Synthetic Runner Four"))
    .with_evidence(Evidence::parsed(result_source(state), &meet.date));
    let relay_id = relay.id.clone();
    store.append(Table::RelayResults, &relay)?;

    let athlete_ids: [AthleteId; 5] = athletes
        .try_into()
        .map_err(|_| "five synthetic state athletes required")?;
    let team_ids: [TeamId; 5] = teams
        .try_into()
        .map_err(|_| "five synthetic state teams required")?;
    Ok((athlete_ids, team_ids, relay_only.id, relay_id))
}

fn postal_address(state: UsJurisdiction, index: usize) -> TestResult<SchoolPostalAddress> {
    let owner = SourceIdentity::new(
        SourceNamespace::association_school("synthetic-association"),
        format!("synthetic-school-{index}"),
    );
    let street = format!("{} Synthetic Way", 100 + index);
    let city = format!("{} City", state);
    let zip = format!("{:05}", 10000 + index);
    let address = PostalAddress::of(
        Some(StreetLine::parse(&street)?),
        Some(StreetLine::parse("Athletics Office")?),
        Some(CityName::parse(&city)?),
        Some(state),
        Some(ZipCode::parse(&zip)?),
    )
    .ok_or("complete synthetic postal address")?
    .with_kind(AddressKind::Physical);
    Ok(SchoolPostalAddress::new(
        address,
        owner,
        SourceLabel::AthleticAssociation { state },
        Evidence::parsed(
            SourceRef::new(
                "synthetic-association",
                Some(format!("https://synthetic.example.test/address/{state}")),
            ),
            "2026-09-01",
        ),
        format!("{index:064x}"),
    )?)
}

fn result_source(state: UsJurisdiction) -> SourceRef {
    SourceRef::new(
        format!("synthetic_results_{state}"),
        Some(format!("https://synthetic.example.test/results/{state}")),
    )
}

fn event(
    meet: &CanonicalMeet,
    kind: EventKind,
    label: &str,
    state: UsJurisdiction,
) -> TestResult<CanonicalEvent> {
    let source = result_source(state);
    let mut event = CanonicalEvent::new(
        EventIdentity {
            meet: &meet.id,
            kind,
            gender: Gender::Girls,
            division: None,
            round: None,
        },
        EventSpecification::default(),
    )?;
    event.source_labels.push(SourceEventLabel {
        source: source.clone(),
        label: label.into(),
    });
    event.evidence.push(Evidence::parsed(source, &meet.date));
    Ok(event)
}

fn performance(
    athlete: &CanonicalAthlete,
    team: &TeamId,
    event: &CanonicalEvent,
    state: UsJurisdiction,
    wind_mps: Option<f64>,
) -> TestResult<CanonicalPerformance> {
    let mut performance = CanonicalPerformance::new(
        PerformanceIdentity {
            athlete: &athlete.id,
            meet: &event.meet,
            event: &event.id,
            date: "2026-05-02",
            source_key: event.id.as_str(),
        },
        PerformanceResult {
            team,
            mark: Mark::TimeSeconds(ExactSeconds::parse("13.20")?),
            adjustment: None,
            wind_mps,
            place: Some(1),
        },
    )?;
    performance.source_athlete = athlete.source.clone();
    performance.timing = Some(TimingMethod::Fat);
    performance
        .evidence
        .push(Evidence::parsed(result_source(state), "2026-05-02"));
    Ok(performance)
}

fn read_csv(path: &Path) -> TestResult<Vec<std::collections::BTreeMap<String, String>>> {
    let mut reader = csv::Reader::from_path(path)?;
    let headers = reader.headers()?.clone();
    reader
        .records()
        .map(|record| {
            let record = record?;
            Ok(headers
                .iter()
                .zip(record.iter())
                .map(|(header, value)| (header.to_owned(), value.to_owned()))
                .collect())
        })
        .collect()
}

fn read_jsonl(path: &Path) -> TestResult<Vec<Value>> {
    Ok(std::fs::read_to_string(path)?
        .lines()
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()?)
}
