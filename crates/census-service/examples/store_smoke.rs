#![forbid(unsafe_code)]

use census_domain::model::{
    normalize_name, CanonicalAthlete, CanonicalSchool, Evidence, Gender, GradYear,
    PublishedGraduation, SourceIdentity, SourceNamespace, SourceRef, Sport,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let (Some(mode), Some(dir)) = (args.next(), args.next()) else {
        return Err(
            "usage: store_smoke <seed|seed-campus-coop|seed-athlete|dump> <store-dir>".into(),
        );
    };
    let store = Store::open(&dir)?;
    match mode.as_str() {
        "seed" => seed(&store)?,
        "seed-campus-coop" => seed_campus_coop(&store)?,
        "seed-athlete" => seed_athlete(&store, args.next(), args.next())?,
        "dump" => dump(&store)?,
        other => {
            return Err(format!(
                "unknown mode {other:?}; expected seed, seed-campus-coop, seed-athlete or dump"
            )
            .into())
        }
    }
    Ok(())
}

fn seed(store: &Store) -> Result<(), Box<dyn std::error::Error>> {
    for (state, name, city) in [
        (UsJurisdiction::Tennessee, "Page High School", "Franklin"),
        (
            UsJurisdiction::NewYork,
            "A A KINGSTON MIDDLE SCHOOL",
            "Potsdam",
        ),
    ] {
        let (mut school, _) = CanonicalSchool::new(state, name, normalize_name(name));
        school.city = Some(city.to_string());
        store.append(Table::Schools, &school)?;
        println!("seeded\t{name}\t{city}\t{state}");
    }
    Ok(())
}

fn seed_campus_coop(store: &Store) -> Result<(), Box<dyn std::error::Error>> {
    for name in ["Page High School", "Page High School (East)"] {
        let (mut school, _) =
            CanonicalSchool::new(UsJurisdiction::Tennessee, name, normalize_name(name));
        school.city = Some("Franklin".to_string());
        store.append(Table::Schools, &school)?;
        println!(
            "seeded\t{name}\tFranklin\t{state}",
            state = UsJurisdiction::Tennessee
        );
    }
    let (mut co_op, _) = CanonicalSchool::new(
        UsJurisdiction::Tennessee,
        "Music City Coop",
        normalize_name("Music City Coop"),
    );
    co_op.co_op = true;
    co_op.city = Some("Nashville".to_string());
    co_op.aliases = vec![
        "Lipscomb Academy".to_string(),
        "Davidson Academy".to_string(),
    ];
    store.append(Table::Schools, &co_op)?;
    println!(
        "seeded\t{}\tNashville\t{state}",
        co_op.name,
        state = UsJurisdiction::Tennessee
    );
    Ok(())
}

fn seed_athlete(
    store: &Store,
    school_name: Option<String>,
    athlete_name: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (Some(school_name), Some(athlete_name)) = (school_name, athlete_name) else {
        return Err("usage: store_smoke seed-athlete <store-dir> <school> <athlete>".into());
    };
    let Some(school) = store
        .scan::<CanonicalSchool>(Table::Schools)?
        .into_iter()
        .find(|school| school.name == school_name)
    else {
        return Err(format!("no school named {school_name}").into());
    };
    let mut athlete = CanonicalAthlete::new(
        &school.id,
        &athlete_name,
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, &athlete_name),
    );
    athlete.sports = vec![Sport::OutdoorTrack];
    let source = SourceRef::new(
        "store-smoke",
        Some(format!(
            "https://fixtures.invalid/store-smoke/{}/2027",
            athlete.id
        )),
    );
    athlete.published_graduations.push(PublishedGraduation {
        grad_year: GradYear::CO2027,
        source: source.clone(),
    });
    let mut claim = Evidence::parsed(source, "2026-10-04T00:00:00Z");
    claim.note = Some("Synthetic store-smoke seed for consumer readback".to_owned());
    athlete.evidence.push(claim);
    store.append(Table::Athletes, &athlete)?;
    println!(
        "seeded-athlete\t{athlete_name}\t{}\t{}",
        school.name, athlete.id
    );
    Ok(())
}

fn dump(store: &Store) -> Result<(), Box<dyn std::error::Error>> {
    for school in store.scan::<CanonicalSchool>(Table::Schools)? {
        println!("{}", serde_json::to_string(&school)?);
    }
    Ok(())
}
