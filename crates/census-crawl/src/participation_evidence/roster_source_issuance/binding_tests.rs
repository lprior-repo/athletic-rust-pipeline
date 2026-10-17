use super::*;
use census_domain::model::{Gender, SchoolYear, SourceSubject, Sport};
use census_domain::UsJurisdiction;
use serde_json::json;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn fixture() -> TestResult<(CanonicalSchool, CanonicalTeam, RosterExistingBindingClaimV1)> {
    let (mut school, id) = CanonicalSchool::new(UsJurisdiction::Michigan, "School", "school", Some("City"));
    school.source_identities.push(SourceIdentity::new(SourceNamespace::MilesplitSchool, "12439"));
    let year = SchoolYear::new(2026).ok_or("school year")?;
    let program = CanonicalTeam {
        id: CanonicalTeam::mint(&id, Sport::CrossCountry, Gender::Boys, year),
        school: id.clone(), sport: Sport::CrossCountry, gender: Gender::Boys,
        school_year: year, level: None,
        source_identities: vec![SourceIdentity::new(SourceNamespace::MilesplitTeam, "12439")],
        evidence: Vec::new(), retained_conflicts: Vec::new(),
    };
    let contract = json!({"version":1, "revision":1, "digest":"a".repeat(64)});
    let object = json!({"digest":"b".repeat(64), "bytes":64});
    let binding = serde_json::from_value(json!({
        "school":id, "program":program.id,
        "school_source":SourceSubject::new(SourceNamespace::MilesplitSchool, "12439".into())?,
        "program_source":SourceSubject::new(SourceNamespace::MilesplitTeam, "12439".into())?,
        "sport":"cross_country", "category":"boys", "school_year":2026,
        "year_convention":contract, "projection_merge_contract":contract,
        "school_projection":object, "program_projection":object, "provider_packet":object,
        "physical_projection_inventory":object, "store":"c".repeat(64), "snapshot_revision":1
    }))?;
    Ok((school, program, binding))
}

#[test]
fn exact_existing_school_and_program_pass_pure_binding_checks() -> TestResult {
    let (school, team, binding) = fixture()?;
    school_row(&school, &binding, &school)?;
    team_row(&team, &binding)?;
    Ok(())
}

#[test]
fn alternate_owner_and_embedded_conflict_refuse() -> TestResult {
    let (school, team, binding) = fixture()?;
    let mut alternate = school.clone();
    alternate.id = census_domain::model::SchoolId::mint("sch", &["different"]);
    check!(school_row(&alternate, &binding, &school).is_err());
    let mut conflicted = team;
    conflicted.retained_conflicts.push(RetainedConflict::new("controlled", "unrelated", "x", "x"));
    check!(team_row(&conflicted, &binding).is_err());
    Ok(())
}

#[test]
fn another_team_in_same_scope_refuses_but_other_scope_is_distinct() -> TestResult {
    let (_, team, binding) = fixture()?;
    let mut alternate = team.clone();
    alternate.id = census_domain::model::TeamId::mint("team", &["different"]);
    check!(team_row(&alternate, &binding).is_err());
    alternate.gender = Gender::Girls;
    team_row(&alternate, &binding)?;
    Ok(())
}

#[test]
fn observation_subject_cannot_hide_behind_unrestricted_id() -> TestResult {
    let expected = SourceSchoolObservation::new(SourceNamespace::MilesplitSchool, "12439", "packet-row", "School", "2026-10-16");
    check!(observation(&SourceObservation::School(expected.clone()), &expected)?);
    let mut hidden = expected.clone();
    hidden.id = "arbitrary-other-observation".into();
    check!(observation(&SourceObservation::School(hidden), &expected).is_err());
    let mut changed = expected.clone();
    changed.source_row_key = "different-packet".into();
    check!(observation(&SourceObservation::School(changed), &expected).is_err());
    Ok(())
}

#[test]
fn canonical_entity_collision_retains_refusal_evidence() -> TestResult {
    let (mut school, _, binding) = fixture()?;
    let packet = school.clone();
    let mut conflicting = school.clone();
    conflicting.normalized_name = "another school".into();
    school.merge(conflicting);
    school.publish();
    check!(school_row(&school, &binding, &packet).is_err());
    Ok(())
}

#[test]
fn global_conflict_absence_policy_also_refuses_unrelated_conflicts() -> TestResult {
    let unrelated = RetainedConflict::new("unrelated family", "other subject", "other", "retained");
    check!(no_global_conflict(&unrelated).is_err());
    Ok(())
}

#[test]
fn merged_projection_reference_requires_exact_digest_and_length() -> TestResult {
    let (school, _, _) = fixture()?;
    let bytes = serde_json::to_vec(&school)?;
    let exact: RosterAuthorityObjectRefV1 = serde_json::from_value(json!({
        "digest":hash(&bytes), "bytes":bytes.len()
    }))?;
    projection_reference(&school, &exact, &mut AttachmentReplayGrantV1::new())?;
    for forged in [
        json!({"digest":"d".repeat(64), "bytes":bytes.len()}),
        json!({"digest":hash(&bytes), "bytes":bytes.len().checked_add(1).ok_or("length")?}),
    ] {
        let forged = serde_json::from_value(forged)?;
        check!(projection_reference(&school, &forged, &mut AttachmentReplayGrantV1::new()).is_err());
    }
    Ok(())
}

#[test]
fn inventory_bytes_reference_rejects_tampered_payload() -> TestResult {
    let bytes = b"controlled canonical manifest";
    let exact = serde_json::from_value(json!({"digest":hash(bytes), "bytes":bytes.len()}))?;
    require_reference(bytes, &exact, &mut AttachmentReplayGrantV1::new())?;
    check!(require_reference(b"controlled canonical manifesz", &exact,
        &mut AttachmentReplayGrantV1::new()).is_err());
    Ok(())
}
