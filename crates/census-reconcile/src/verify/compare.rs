use std::collections::HashMap;

use census_domain::model::{CanonicalAthlete, CanonicalSchool};

use census_store::{Store, Table};

#[derive(Debug)]
pub struct Discrepancy {
    pub message: String,
}

#[derive(Debug)]
pub struct EntityCheck {
    pub passed: usize,
    pub sampled_indices: Vec<usize>,
}

pub(super) fn field<'a>(
    col_map: &'a HashMap<&str, usize>,
    row: &'a [String],
    name: &str,
) -> &'a str {
    col_map
        .get(name)
        .and_then(|&i| row.get(i))
        .map(|s| s.trim())
        .map_or("", |value| value)
}

pub fn verify_athletes(
    store: &Store,
    rows: &[Vec<String>],
    sampled: &[usize],
    col_map: &HashMap<&str, usize>,
) -> Result<EntityCheck, Discrepancy> {
    let athletes: Vec<CanonicalAthlete> =
        store.snapshot().athletes().map_err(|source| Discrepancy {
            message: format!("reading athletes from store: {source}"),
        })?;

    let school_names: HashMap<String, String> = store
        .scan(Table::Schools)
        .map_err(|source| Discrepancy {
            message: format!("reading schools from store: {source}"),
        })?
        .into_iter()
        .map(|school: CanonicalSchool| (school.id.as_str().to_string(), school.name))
        .collect();

    let mut passed: usize = 0;

    for &idx in sampled {
        let row = rows.get(idx).ok_or_else(|| Discrepancy {
            message: "row index out of range".to_string(),
        })?;
        check_athlete_row(idx, row, &athletes, &school_names, col_map)?;
        passed = passed.saturating_add(1);
    }

    Ok(EntityCheck {
        passed,
        sampled_indices: sampled.to_vec(),
    })
}

fn check_athlete_row(
    idx: usize,
    row: &[String],
    athletes: &[CanonicalAthlete],
    school_names: &HashMap<String, String>,
    col_map: &HashMap<&str, usize>,
) -> Result<(), Discrepancy> {
    let aid = field(col_map, row, "Athlete ID");
    let name = field(col_map, row, "Name");
    let store_athlete = athletes
        .iter()
        .find(|a| a.id.as_str() == aid)
        .ok_or_else(|| Discrepancy {
            message: format!("athletes row {idx}: id {aid} not in store"),
        })?;

    if store_athlete.canonical_name != name {
        return Err(Discrepancy {
            message: format!(
                "athletes row {idx}: id {aid} name '{name}' != store '{}'",
                store_athlete.canonical_name,
            ),
        });
    }
    check_athlete_school(
        idx,
        aid,
        field(col_map, row, "School"),
        store_athlete,
        school_names,
    )?;
    check_athlete_cohort(idx, aid, row, col_map)
}

fn check_athlete_school(
    idx: usize,
    aid: &str,
    school: &str,
    store_athlete: &CanonicalAthlete,
    school_names: &HashMap<String, String>,
) -> Result<(), Discrepancy> {
    let school_id = store_athlete.school.as_str();
    match school_names.get(school_id) {
        Some(store_school) if store_school == school => Ok(()),
        Some(store_school) => Err(Discrepancy {
            message: format!(
                "athletes row {idx}: id {aid} school '{school}' != store '{store_school}'"
            ),
        }),
        None if school == school_id => Ok(()),
        None => Err(Discrepancy {
            message: format!(
                "athletes row {idx}: id {aid} school row '{school_id}' not in store, and the sheet \
                 prints '{school}'"
            ),
        }),
    }
}

fn check_athlete_cohort(
    idx: usize,
    aid: &str,
    row: &[String],
    col_map: &HashMap<&str, usize>,
) -> Result<(), Discrepancy> {
    let grad_year = col_map
        .get("Graduation Year")
        .and_then(|&i| row.get(i))
        .and_then(|s| s.trim().parse::<i16>().ok());
    if grad_year == Some(2027) {
        return Ok(());
    }
    Err(Discrepancy {
        message: format!(
            "athletes row {idx}: id {aid} grad_year {} != 2027",
            grad_year.map_or("?".to_string(), |y| y.to_string()),
        ),
    })
}
