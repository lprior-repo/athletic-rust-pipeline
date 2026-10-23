use super::*;
use census_domain::model::{Evidence, SchoolYear, SourceIdentity, SourceRef};
use census_domain::UsJurisdiction;

fn note(role: &str, packet: &str) -> String {
    serde_json::json!({"role": role, "phase": OWNED_PROVIDER_SCHOOL_INTAKE_PHASE,
        "interpretation": OWNED_PROVIDER_SCHOOL_INTERPRETATION,
        "packet": packet, "capture_sha256": "b".repeat(64)})
    .to_string()
}

#[test]
fn team_scan_budget_covers_retained_physical_row_count() -> Result<()> {
    check!(TEAM_BUDGET.rows >= 1_050_405);
    check!(eq; TEAM_BUDGET.rows, 2_097_152);
    Ok(())
}

#[test]
fn team_scan_budget_covers_observed_encoded_requirement() -> Result<()> {
    check!(TEAM_BUDGET.bytes >= 268_437_966);
    check!(eq; TEAM_BUDGET.bytes, 8_589_934_592);
    check!(eq; TEAM_BUDGET.record_bytes, 1_048_576);
    Ok(())
}

#[test]
fn emitted_program_limit_remains_independent_of_physical_scan_budget() -> Result<()> {
    let retained = school("Quota", &"a".repeat(64), &["918"]);
    let mut schools = BTreeMap::new();
    let mut packet_count = 0;
    collect_school(retained.clone(), &mut schools, &mut packet_count)?;
    let mut count = 131_071;
    collect_team(team(&retained, &["918"]), &mut schools, &mut count)?;
    check!(eq; count, 131_072);
    check!(eq;
        schools.get(retained.id.as_str()).context("school missing")?.programs.len(),
        1
    );
    let before = serde_json::to_vec(&schools)?;
    let mut overflow = team(&retained, &["918"]);
    overflow.gender = Gender::Boys;
    overflow.id = CanonicalTeam::mint(
        &retained.id,
        overflow.sport,
        overflow.gender,
        overflow.school_year,
    );
    let error = collect_team(overflow, &mut schools, &mut count)
        .err()
        .context("expected retained-program quota refusal")?;
    check!(matches!(
        error,
        StoreError::BudgetExceeded {
            what: "roster inventory retained programs",
            needed: 131_073,
            available: 131_072,
        }
    ));
    check!(eq; count, 131_072);
    check!(eq; serde_json::to_vec(&schools)?, before);
    Ok(())
}

#[test]
fn parses_packet_field_and_ignores_unrelated_notes() -> Result<()> {
    check!(eq; parse_known_note("ordinary non-JSON evidence")?, None);
    check!(eq; parse_known_note(&note("directory_name_city", "a"))?, None);
    check!(eq;
        parse_known_note(&note("provider_roster_owner", &"a".repeat(64)))?,
        Some("a".repeat(64))
    );
    check!(parse_known_note(&note("provider_roster_owner", "bad-ref")).is_err());
    check!(parse_known_note(OWNED_PROVIDER_SCHOOL_INTAKE_PHASE).is_err());
    Ok(())
}

#[test]
fn original_owned_note_remains_a_discovery_hint_after_current_interpretation_changes() -> Result<()>
{
    let reference = "a".repeat(64);
    let original = serde_json::json!({
        "role": "provider_roster_owner",
        "phase": "public_resource_school_intake_v1",
        "interpretation": "milesplit_owned_directory_roster_school_v1",
        "packet": reference,
    });
    check!(eq; parse_known_note(&original.to_string())?, Some(reference));
    let mut unknown = original;
    unknown["interpretation"] = serde_json::json!("unknown-owned-interpretation");
    check!(eq; parse_known_note(&unknown.to_string())?, None);
    check!(parse_known_note(RETAINED_OWNED_SCHOOL_NOTE_V1).is_err());
    Ok(())
}

#[test]
fn provider_owner_preserves_distinct_ids_without_selecting_ambiguity() {
    let namespace = SourceNamespace::MilesplitSchool;
    let mut identities = vec![
        SourceIdentity::new(namespace.clone(), "1228").with_url("https://a"),
        SourceIdentity::new(namespace.clone(), "1228").with_url("https://b"),
    ];
    let unique = ProviderOwners::new(&identities, &namespace);
    assert_eq!(unique.provider_id.as_deref(), Some("1228"));
    assert_eq!(unique.provider_ids.len(), 1);
    assert_eq!(unique.owner_diagnostic, None);
    identities.push(SourceIdentity::new(namespace.clone(), "37350"));
    let ambiguous = ProviderOwners::new(&identities, &namespace);
    assert_eq!(ambiguous.provider_id, None);
    assert_eq!(
        ambiguous.provider_ids.into_iter().collect::<Vec<_>>(),
        ["1228", "37350"]
    );
    assert_eq!(
        ambiguous.owner_diagnostic,
        Some("ambiguous_provider_owners")
    );
    let missing = ProviderOwners::new(&[], &namespace);
    assert_eq!(missing.provider_id, None);
    assert_eq!(missing.owner_diagnostic, Some("missing_provider_owner"));
}

#[test]
fn encoding_overflow_publishes_no_partial_inventory() -> Result<()> {
    let mut output = Vec::new();
    check!(emit(&mut output, &"oversized", 4).is_err());
    check!(output.is_empty());
    emit(&mut output, &"ok", 5)?;
    check!(eq; output, b"\"ok\"\n");
    Ok(())
}

fn school(name: &str, packet: &str, owners: &[&str]) -> CanonicalSchool {
    let (mut school, _) = CanonicalSchool::new(UsJurisdiction::Michigan, name, name, None);
    school.source_identities = owners
        .iter()
        .map(|id| SourceIdentity::new(SourceNamespace::MilesplitSchool, *id))
        .collect();
    let mut evidence = Evidence::parsed(SourceRef::id("fixture"), "2026-10-21T00:00:00Z");
    evidence.note = Some(note("provider_roster_owner", packet));
    school.evidence.push(evidence);
    school
}

fn team(school: &CanonicalSchool, owners: &[&str]) -> CanonicalTeam {
    let school_year = SchoolYear::DEFAULT;
    CanonicalTeam {
        id: CanonicalTeam::mint(&school.id, Sport::OutdoorTrack, Gender::Girls, school_year),
        school: school.id.clone(),
        sport: Sport::OutdoorTrack,
        gender: Gender::Girls,
        school_year,
        level: None,
        source_identities: owners
            .iter()
            .map(|id| SourceIdentity::new(SourceNamespace::MilesplitTeam, *id))
            .collect(),
        evidence: Vec::new(),
        retained_conflicts: Vec::new(),
    }
}

#[test]
fn ambiguous_school_and_program_preserve_packet_hints_and_ordinary_rows() -> Result<()> {
    let packet = "a".repeat(64);
    let ambiguous = school("Ambiguous", &packet, &["1228", "37350"]);
    let ordinary = school("Ordinary", &"c".repeat(64), &["918"]);
    let ambiguous_program = team(&ambiguous, &["1228", "37350"]);
    let ordinary_program = team(&ordinary, &["918"]);
    let mut schools = BTreeMap::new();
    let mut packet_count = 0;
    collect_school(ambiguous.clone(), &mut schools, &mut packet_count)?;
    collect_school(ordinary.clone(), &mut schools, &mut packet_count)?;
    let mut program_count = 0;
    collect_team(ambiguous_program, &mut schools, &mut program_count)?;
    collect_team(ordinary_program, &mut schools, &mut program_count)?;
    check!(eq; (schools.len(), packet_count, program_count), (2, 2, 2));
    let diagnostic = schools
        .get(ambiguous.id.as_str())
        .context("ambiguous school missing")?;
    check!(diagnostic.packets.contains_key(&packet));
    check!(eq; diagnostic.owner.provider_id, None);
    check!(eq; diagnostic.owner.provider_ids.len(), 2);
    check!(eq;
        diagnostic.owner.owner_diagnostic,
        Some("ambiguous_provider_owners")
    );
    let program = diagnostic
        .programs
        .first()
        .context("ambiguous program missing")?;
    check!(eq; program.owner.provider_id, None);
    check!(eq; program.owner.provider_ids.len(), 2);
    check!(eq;
        program.owner.owner_diagnostic,
        Some("ambiguous_provider_owners")
    );
    check!(eq; program.provider_join, "provider_owner_unresolved");
    let retained = schools
        .get(ordinary.id.as_str())
        .context("ordinary school missing")?;
    check!(eq; retained.owner.provider_id.as_deref(), Some("918"));
    let program = retained
        .programs
        .first()
        .context("ordinary program missing")?;
    check!(eq; program.owner.provider_id.as_deref(), Some("918"));
    check!(eq; program.provider_join, "same_unique_provider_owner");
    Ok(())
}
