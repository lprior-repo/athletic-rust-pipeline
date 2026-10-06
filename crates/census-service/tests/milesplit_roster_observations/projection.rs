use super::OBSERVED_ON;
use census_domain::model::{
    CanonicalAthlete, CanonicalSchool, Confidence, Gender, GradYear, PublishedGraduation,
    SourceIdentity, SourceNamespace, SourceObservation, SourceRef,
};
use census_domain::UsJurisdiction;
use census_service::census::{self, CollectOptions};
use census_store::{Store, Table};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

const TEAM_URL: &str = "https://wi.milesplit.com/teams/52649-abbotsford";
const ROSTER_URL: &str = "https://wi.milesplit.com/teams/52649-abbotsford/roster";

const PUBLISHED: [(&str, &str, &str, i16, Gender); 25] = [
    (
        "14399169",
        "Julian Aguilera",
        "julian-aguilera",
        2027,
        Gender::Boys,
    ),
    (
        "14174132",
        "Esmeralda Altamirano",
        "esmeralda-altamirano",
        2027,
        Gender::Girls,
    ),
    (
        "16061058",
        "Landen Auberg",
        "landen-auberg",
        2028,
        Gender::Boys,
    ),
    (
        "16061056",
        "Carlos Beltran",
        "carlos-beltran",
        2028,
        Gender::Boys,
    ),
    ("14399147", "Ava Bleck", "ava-bleck", 2027, Gender::Girls),
    (
        "17920084",
        "Nadia Bravo",
        "nadia-bravo",
        2027,
        Gender::Girls,
    ),
    (
        "14576392",
        "Campbell Brodhagen",
        "campbell-brodhagen",
        2027,
        Gender::Girls,
    ),
    (
        "14399173",
        "Carter Cihlar",
        "carter-cihlar",
        2027,
        Gender::Boys,
    ),
    (
        "14399175",
        "Edwin Cossio Aguilera",
        "edwin-cossio-aguilera",
        2027,
        Gender::Boys,
    ),
    (
        "14399180",
        "Jovanny Cruz",
        "jovanny-cruz",
        2027,
        Gender::Boys,
    ),
    ("16061062", "Tavo Diaz", "tavo-diaz", 2027, Gender::Boys),
    (
        "14399187",
        "Gustavo Diaz Tamoyo",
        "gustavo-diaz-tamoyo",
        2027,
        Gender::Boys,
    ),
    (
        "17715621",
        "Marcella Dundikow",
        "marcella-dundikow",
        2030,
        Gender::Girls,
    ),
    (
        "17715638",
        "Emiliano Espino",
        "emiliano-espino",
        2030,
        Gender::Boys,
    ),
    (
        "17715636",
        "Guillermo Garcia",
        "guillermo-garcia",
        2028,
        Gender::Boys,
    ),
    (
        "16061077",
        "Emily Gonzalez",
        "emily-gonzalez",
        2027,
        Gender::Girls,
    ),
    (
        "14399191",
        "Luis Gonzalez Lujan",
        "luis-gonzalez-lujan",
        2027,
        Gender::Boys,
    ),
    (
        "13698781",
        "Marilyn Hammock",
        "marilyn-hammock",
        2027,
        Gender::Girls,
    ),
    (
        "16061070",
        "Lauryn Harris",
        "lauryn-harris",
        2028,
        Gender::Girls,
    ),
    (
        "14399152",
        "Oriah Harris",
        "oriah-harris",
        2027,
        Gender::Girls,
    ),
    (
        "17016950",
        "Jocelyn Jimenez",
        "jocelyn-jimenez",
        2029,
        Gender::Girls,
    ),
    (
        "16061071",
        "Kinsley Kalepp",
        "kinsley-kalepp",
        2028,
        Gender::Girls,
    ),
    (
        "16061057",
        "Caleb Kulesa",
        "caleb-kulesa",
        2028,
        Gender::Boys,
    ),
    ("17715629", "Manny Lopez", "manny-lopez", 2029, Gender::Boys),
    (
        "14721418",
        "Manuel Lopez",
        "manuel-lopez",
        2028,
        Gender::Boys,
    ),
];

pub(super) fn assert_published(store: &Store) -> super::TestResult {
    let schools: Vec<CanonicalSchool> = store.scan(Table::Schools)?;
    check!(eq; schools.len(), 1);
    let school = &schools[0];
    let school_id = CanonicalSchool::mint(
        UsJurisdiction::Wisconsin,
        "Abbotsford",
        "abbotsford",
        Some("Abbotsford"),
    );
    check!(eq; school.id, school_id);
    check!(eq; school.name, "Abbotsford");
    check!(eq; school.source_identities,
    vec![SourceIdentity::new(SourceNamespace::MilesplitSchool, "52649").with_url(TEAM_URL)]);
    let mut observations = BTreeMap::new();
    let mut physical_athletes = 0;
    store
        .snapshot()
        .for_each_observation(Table::SourceObservations, |row| {
            if let SourceObservation::Athlete(row) = row {
                physical_athletes += 1;
                observations.insert(row.source_athlete_id.clone(), row);
            }
            Ok(())
        })?;
    let athletes: Vec<CanonicalAthlete> = store.snapshot().athletes()?;
    check!(eq; physical_athletes, PUBLISHED.len());
    check!(eq; athletes.len(), PUBLISHED.len());
    let expected_ids: std::collections::BTreeSet<_> = PUBLISHED.iter().map(|row| row.0).collect();
    check!(eq; observations
        .keys()
        .map(String::as_str)
        .collect::<std::collections::BTreeSet<_>>(),
    expected_ids);
    for (id, name, slug, year, gender) in PUBLISHED {
        let profile = format!("https://wi.milesplit.com/athletes/{id}-{slug}");
        let identity =
            SourceIdentity::new(SourceNamespace::MilesplitAthlete, id).with_url(&profile);
        let year = GradYear::new(year).ok_or("invalid published graduation year")?;
        let person_id = CanonicalAthlete::mint(&school_id, name, year, gender, &identity);
        let athlete = athletes
            .iter()
            .find(|athlete| athlete.id == person_id)
            .ok_or("published athlete has no canonical person id")?;
        check!(eq; athlete.school, school_id);
        check!(eq; athlete.canonical_name, name);
        check!(eq; athlete.gender, gender);
        check!(eq; athlete.grad_year, year);
        check!(eq; athlete.source.as_ref(), Some(&identity));
        check!(eq; athlete.public_profile_urls, vec![profile.clone()]);
        check!(eq; athlete.observed_grades, Vec::new());
        let graduation = PublishedGraduation {
            grad_year: year,
            source: SourceRef::new("milesplit_wi", Some(ROSTER_URL.to_string())),
        };
        check!(eq; athlete.published_graduations, vec![graduation]);
        check!(eq; athlete.derived_cohort_confidence(), Some(Confidence::HIGH));
        let observation = observations.get(id).ok_or("missing published source UID")?;
        check!(eq; observation.id, format!("milesplit_athlete:{id}"));
        check!(eq; observation.observed_name, name);
        check!(eq; observation.observed_school.as_deref(), Some("Abbotsford"));
        check!(eq; observation.gender, gender);
        check!(eq; observation.observed_grade, None);
        check!(eq; observation.profile_url.as_deref(), Some(profile.as_str()));
        check!(eq; observation.source_row_key, profile);
        check!(eq; observation.observed_on, OBSERVED_ON);
    }
    Ok(())
}

pub(super) fn assert_capture(
    store: &Store,
    options: &CollectOptions,
    synthetic_body: &str,
) -> super::TestResult {
    let phase = census::rosters_phase(
        UsJurisdiction::Wisconsin,
        options.school_year,
        options.revision,
    );
    let journal = store
        .journal_payload(&phase, "WI:52649")?
        .ok_or("missing committed capture binding")?;
    check!(eq; journal["team_id"], "52649");
    let school_id = CanonicalSchool::mint(
        UsJurisdiction::Wisconsin,
        "Abbotsford",
        "abbotsford",
        Some("Abbotsford"),
    );
    check!(eq; journal["school"], school_id.as_str());
    check!(eq; journal["observed_on"], OBSERVED_ON);
    check!(eq; journal["capture"]["url"], ROSTER_URL);
    check!(eq; journal["capture"]["status"], 200);
    check!(eq; journal["capture"]["content_digest"],
    format!("{:x}", Sha256::digest(synthetic_body.as_bytes())));
    check!(eq; journal["capture"]["bytes"], synthetic_body.len());
    check!(eq; journal["capture"].get("response_url"), None);
    check!(eq; journal["capture"]["fetched_at"], "2026-09-20T00:00:00Z");
    Ok(())
}
