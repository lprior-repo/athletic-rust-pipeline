use super::{apply_review, measured_pair, retained_rows, seed, Frozen, Measured, TestResult};
use census_domain::model::{
    person_key, AppliedIdentityKind, AthleteIdentityIndex, CanonicalAthlete, Gender, GradYear,
};
use std::collections::{BTreeMap, BTreeSet};
use std::io::BufReader;

type Counts = (usize, usize, usize, usize, usize);

fn input() -> TestResult<(Measured, Counts)> {
    match std::env::var("O8RM_FROZEN_INPUT") {
        Ok(path) => Ok((
            serde_json::from_reader::<_, Frozen>(BufReader::new(std::fs::File::open(path)?))?.data,
            (367, 742, 364, 2, 1),
        )),
        Err(std::env::VarError::NotPresent) => Ok((measured_pair()?, (1, 2, 0, 0, 1))),
        Err(error) => Err(error.into()),
    }
}

fn validate_pair(measured: &Measured) -> TestResult {
    let mut actual: Vec<_> = measured
        .athletes
        .iter()
        .filter(|row| row.canonical_name == "Nyah'Chii Bryant")
        .cloned()
        .collect();
    actual.sort_by(|left, right| left.id.cmp(&right.id));
    let mut fixture = measured_pair()?.athletes;
    fixture.sort_by(|left, right| left.id.cmp(&right.id));
    check!(eq; actual, fixture, "the public fixture matches the frozen observations exactly");
    let members: BTreeSet<_> = fixture.iter().map(|row| row.id.as_str()).collect();
    check!(eq; measured.review_cases.iter().filter(|case| case.member_ids.iter().any(|id| members.contains(id.as_str()))).count(), 0);
    check!(eq; measured.verdicts.iter().filter(|verdict| verdict.member_ids.iter().any(|id| members.contains(id.as_str()))).count(), 0);
    check!(eq; measured.identity_decisions.iter().filter(|decision| decision.members.iter().any(|member| members.contains(member.subject.as_str()))).count(), 0);
    Ok(())
}

fn groups(measured: &Measured) -> TestResult<Vec<Vec<&CanonicalAthlete>>> {
    let mut grouped = BTreeMap::new();
    for row in &measured.athletes {
        if row.grad_year != GradYear::CO2027 {
            continue;
        }
        let school = measured
            .schools
            .get(&row.school)
            .ok_or("frozen school is present")?;
        grouped
            .entry((&row.canonical_name, &school.name, school.state))
            .or_insert_with(Vec::new)
            .push(row);
    }
    Ok(grouped
        .into_values()
        .filter(|rows| rows.len() > 1)
        .collect())
}

fn disposition(rows: &[&CanonicalAthlete]) -> TestResult<(bool, bool, bool)> {
    let keys: BTreeSet<_> = rows
        .iter()
        .map(|row| {
            row.source
                .as_ref()
                .and_then(person_key)
                .ok_or("parsed primary provider key")
        })
        .collect::<Result<_, _>>()?;
    let members: Vec<_> = rows.iter().map(|row| row.id.cast()).collect();
    let mut index = AthleteIdentityIndex::default();
    for row in rows {
        index.observe(row)?;
    }
    let supports = index.supports_identity(AppliedIdentityKind::SamePerson, &members);
    let disjoint = keys.len() == rows.len();
    let contradictory = !disjoint
        && rows.iter().any(|row| row.gender == Gender::Boys)
        && rows.iter().any(|row| row.gender == Gender::Girls);
    check!(eq; supports, !disjoint && !contradictory, "every measured group must have a supported disposition");
    Ok((disjoint, contradictory, supports))
}

fn counts(groups: &[Vec<&CanonicalAthlete>]) -> TestResult<Counts> {
    let (mut rows, mut disjoint, mut contradictory, mut admissible) =
        (0usize, 0usize, 0usize, 0usize);
    for group in groups {
        let (separate, conflict, merge) = disposition(group)?;
        rows = rows
            .checked_add(group.len())
            .ok_or("source count overflow")?;
        disjoint = disjoint
            .checked_add(usize::from(separate))
            .ok_or("disjoint count overflow")?;
        contradictory = contradictory
            .checked_add(usize::from(conflict))
            .ok_or("contradictory count overflow")?;
        admissible = admissible
            .checked_add(usize::from(merge))
            .ok_or("admissible count overflow")?;
    }
    Ok((groups.len(), rows, disjoint, contradictory, admissible))
}

#[test]
fn o8rm_measured_groups_retain_disjoint_and_contradictory_identities() -> TestResult {
    let (measured, expected) = input()?;
    validate_pair(&measured)?;
    let groups = groups(&measured)?;
    let observed = counts(&groups)?;
    check!(eq; observed, expected);
    println!("measured_before_reconciliation={observed:?}");
    let retained: Vec<_> = groups.iter().flatten().map(|row| (*row).clone()).collect();
    let (_dir, store) = seed(&measured, &retained)?;
    apply_review(&store, (observed.4, observed.3))?;
    retained_rows(&store, &retained)?;
    let projection = store.athlete_identity_projection()?;
    for rows in &groups {
        let roots: BTreeSet<_> = rows
            .iter()
            .map(|row| projection.canonical_id(row.id.as_str()))
            .collect();
        let (_, _, admissible) = disposition(rows)?;
        check!(eq; roots.len(), if admissible { 1 } else { rows.len() });
        for row in rows {
            let expected = if admissible {
                "ath_subject_5bfe83d3634b1784"
            } else {
                row.id.as_str()
            };
            check!(eq; projection.canonical_id(row.id.as_str()), expected);
        }
    }
    println!(
        "groups={} source_rows={} retain_disjoint={} retain_contradictory={} merged={}",
        observed.0, observed.1, observed.2, observed.3, observed.4
    );
    Ok(())
}
