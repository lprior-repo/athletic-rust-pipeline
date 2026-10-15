use super::evidence::RowEvidence;
use super::*;
use census_domain::model::{ContactProofField, RawContactRow};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn fragment() -> RawContactRow {
    RawContactRow {
        school: "Mosinee High School".to_string(),
        city: "Mosinee".to_string(),
        state: "WI".to_string(),
        sport: "Cross Country".to_string(),
        role: "Head XC Coach".to_string(),
        coach_name: "Dana Reid".to_string(),
        public_professional_email: "dana@example.org".to_string(),
        ad_name: String::new(),
        ad_email: String::new(),
        source_urls: vec!["https://example.org/staff".to_string()],
        last_observed: "2026-09-21".to_string(),
    }
}

fn staff(body: &str) -> String {
    format!("<h1>Mosinee High School WI</h1><table>{body}</table>")
}

fn evaluate(row: &RawContactRow, body: &str) -> TestResult<RowEvidence> {
    let mut evidence = RowEvidence::default();
    let sha256 = format!("{:x}", Sha256::digest(body.as_bytes()));
    evidence.absorb(
        body,
        row,
        &row.source_urls[0],
        "2026-09-26T12:00:00Z",
        &sha256,
    )?;
    Ok(evidence)
}

#[test]
fn real_name_does_not_verify_an_absent_address() -> TestResult {
    let evidence = evaluate(
        &fragment(),
        &staff("<tr><td>Dana Reid</td><td>Head XC Coach</td></tr>"),
    )?;
    check!(!evidence.verdict().shipped());
    check!(!evidence
        .claims
        .iter()
        .any(|claim| claim.field == ContactProofField::PublicProfessionalEmail));
    Ok(())
}

#[test]
fn valid_dual_sport_appointment_accepted() -> TestResult {
    let body = staff("<tr><td>Dana Reid Head XC and Basketball Coach dana@example.org</td></tr>");
    let evidence = evaluate(&fragment(), &body)?;
    check!(eq; evidence.verdict(), Verdict::Ok);
    check!(evidence
        .claims
        .iter()
        .any(|claim| claim.field == ContactProofField::PublicProfessionalEmail));
    Ok(())
}

#[test]
fn basketball_only_does_not_establish_xc() -> TestResult {
    let body = staff("<tr><td>Dana Reid Head Basketball Coach dana@example.org</td></tr>");
    let evidence = evaluate(&fragment(), &body)?;
    check!(!evidence.verdict().shipped());
    check!(!evidence
        .claims
        .iter()
        .any(|c| c.field == ContactProofField::PublicProfessionalEmail));
    Ok(())
}

#[test]
fn contradiction_overrides_a_matching_role() -> TestResult {
    let mut evidence = evaluate(
        &fragment(),
        &staff("<tr><td>Dana Reid Head XC Coach dana@example.org</td></tr>"),
    )?;
    check!(eq; evidence.verdict(), Verdict::Ok);
    evidence.contradicted = true;
    check!(eq; evidence.verdict(), Verdict::RoleContradicted);
    Ok(())
}

#[test]
fn adjacent_staff_cards_cannot_supply_someone_elses_email() -> TestResult {
    let body = staff("<tr><td>Dana Reid Head XC Coach</td></tr><tr><td>Other Person Head XC Coach dana@example.org</td></tr>");
    check!(!evaluate(&fragment(), &body)?.verdict().shipped());
    Ok(())
}

#[test]
fn exact_record_preserves_field_relationship_and_source_bytes() -> TestResult {
    let body = staff("<tr><td>Dana Reid</td><td>Head XC Coach</td><td>dana@example.org</td></tr>");
    let evidence = evaluate(&fragment(), &body)?;
    check!(eq; evidence.verdict(), Verdict::Ok);
    let email = evidence
        .claims
        .iter()
        .find(|claim| claim.field == ContactProofField::PublicProfessionalEmail)
        .ok_or("missing verified address")?;
    check!(eq; email.value, "dana@example.org");
    check!(eq; email.person, "Dana Reid");
    check!(eq; email.source_url, "https://example.org/staff");
    check!(eq; email.fetched_at, "2026-09-26T12:00:00Z");
    Ok(())
}

#[test]
fn public_role_consumer_mailbox_is_not_discarded() -> TestResult {
    let mut row = fragment();
    row.public_professional_email = "schooltrack@gmail.com".to_string();
    let evidence = evaluate(
        &row,
        &staff("<tr><td>Dana Reid Head XC Coach schooltrack@gmail.com</td></tr>"),
    )?;
    check!(eq; evidence.verdict(), Verdict::Ok);
    Ok(())
}

#[test]
fn a_different_school_cannot_verify_the_claimed_institution() -> TestResult {
    let body = "<h1>Other High School WI</h1><table><tr><td>Dana Reid Head XC Coach dana@example.org</td></tr></table>";
    check!(!evaluate(&fragment(), body)?.verdict().shipped());
    Ok(())
}

fn director_fragment() -> RawContactRow {
    RawContactRow {
        role: "Athletic Director".to_string(),
        coach_name: String::new(),
        public_professional_email: String::new(),
        ad_name: "Dana Reid".to_string(),
        ad_email: "dana@example.org".to_string(),
        ..fragment()
    }
}

#[test]
fn a_current_same_school_director_verifies() -> TestResult {
    let evidence = evaluate(
        &director_fragment(),
        &staff("<tr><td>Dana Reid Athletic Director dana@example.org</td></tr>"),
    )?;
    check!(eq; evidence.verdict(), Verdict::Ok);
    check!(evidence
        .claims
        .iter()
        .any(|claim| claim.field == ContactProofField::AdEmail));
    Ok(())
}

#[test]
fn a_foreign_school_cannot_verify_the_claimed_director() -> TestResult {
    let body = "<h1>Other High School WI</h1><table><tr><td>Dana Reid Athletic Director dana@example.org</td></tr></table>";
    check!(!evaluate(&director_fragment(), body)?.verdict().shipped());
    Ok(())
}

#[test]
fn a_foreign_state_cannot_verify_the_claimed_director() -> TestResult {
    let body = "<h1>Mosinee High School MN</h1><table><tr><td>Dana Reid Athletic Director dana@example.org</td></tr></table>";
    check!(!evaluate(&director_fragment(), body)?.verdict().shipped());
    Ok(())
}

#[test]
fn combined_coach_and_ad_row_emits_director_role_for_ad_email() -> TestResult {
    let row = RawContactRow {
        role: "Head XC Coach".to_string(),
        coach_name: "Dana Reid".to_string(),
        public_professional_email: "dana@example.org".to_string(),
        ad_name: "Pat Nolan".to_string(),
        ad_email: "pat@example.org".to_string(),
        ..fragment()
    };
    let body = staff(
        "<tr><td>Dana Reid Head XC Coach dana@example.org</td></tr><tr><td>Pat Nolan Athletic Director pat@example.org</td></tr>",
    );
    let evidence = evaluate(&row, &body)?;
    check!(eq; evidence.verdict(), Verdict::Ok);
    let director_email = evidence
        .claims
        .iter()
        .find(|claim| claim.field == ContactProofField::AdEmail)
        .ok_or("the combined row publishes its director address")?;
    check!(eq; director_email.role, "Athletic Director");
    check!(eq; director_email.person, "Pat Nolan");
    let proof = census_domain::model::compute_contact_proof(&row, &evidence.claims)?;
    check!(eq; proof.len(), 64);
    Ok(())
}

#[test]
fn former_athletic_director_is_contradicted() -> TestResult {
    let evidence = evaluate(
        &director_fragment(),
        &staff("<tr><td>Dana Reid Former Athletic Director dana@example.org</td></tr>"),
    )?;
    check!(evidence.contradicted);
    check!(eq; evidence.verdict(), Verdict::RoleContradicted);
    check!(evidence.claims.is_empty());
    Ok(())
}

#[test]
fn a_not_current_director_title_is_contradicted() -> TestResult {
    let evidence = evaluate(
        &director_fragment(),
        &staff("<tr><td>Dana Reid Athletic Director no longer dana@example.org</td></tr>"),
    )?;
    check!(evidence.contradicted);
    Ok(())
}

#[test]
fn former_role_escapes_as_contradiction() -> TestResult {
    let body = staff("<tr><td>Dana Reid former Head XC Coach dana@example.org</td></tr>");
    let evidence = evaluate(&fragment(), &body)?;
    check!(evidence.contradicted);
    check!(!evidence.verdict().shipped());
    Ok(())
}

#[test]
fn not_current_role_triggers_contradiction() -> TestResult {
    let body = staff("<tr><td>Dana Reid not current Head XC Coach dana@example.org</td></tr>");
    let evidence = evaluate(&fragment(), &body)?;
    check!(evidence.contradicted);
    Ok(())
}

#[test]
fn different_school_card_does_not_verify() -> TestResult {
    let body = "<h1>Mosinee High School WI</h1><table><tr><td>Dana Reid Head XC Coach</td></tr><tr><td>Jane Smith Head XC Coach dana@example.org</td></tr></table>";
    let evidence = evaluate(&fragment(), body)?;
    check!(!evidence.verdict().shipped());
    Ok(())
}

#[test]
fn exact_email_bytes_preserved_not_casefolded() -> TestResult {
    let body = staff("<tr><td>Dana Reid Head XC Coach Dana@Example.ORG</td></tr>");
    let mut row = fragment();
    row.public_professional_email = "Dana@Example.ORG".to_string();
    let evidence = evaluate(&row, &body)?;
    check!(eq; evidence.verdict(), Verdict::Ok);
    let email = evidence
        .claims
        .iter()
        .find(|c| c.field == ContactProofField::PublicProfessionalEmail)
        .ok_or("missing verified address")?;
    check!(eq; email.value, "Dana@Example.ORG");
    Ok(())
}

#[test]
fn a_repeated_pass_records_the_same_claim_once() -> TestResult {
    let body = staff("<tr><td>Dana Reid Head XC Coach dana@example.org</td></tr>");
    let row = fragment();
    let mut evidence = evaluate(&row, &body)?;
    check!(!evidence.claims.is_empty(), "the first pass records claims");
    let first = evidence.claims.clone();
    let sha256 = format!("{:x}", Sha256::digest(body.as_bytes()));
    evidence.absorb(
        &body,
        &row,
        &row.source_urls[0],
        "2026-09-26T12:00:00Z",
        &sha256,
    )?;
    check!(
        eq;
        evidence.claims,
        first,
        "absorbing the same page twice must not duplicate a claim"
    );
    check!(
        evidence.role_near,
        "the corroborating flag survives a repeat pass"
    );
    Ok(())
}

#[test]
fn proof_digest_matches_evidence() -> TestResult {
    let body = staff("<tr><td>Dana Reid Head XC Coach dana@example.org</td></tr>");
    let row = fragment();
    let evidence = evaluate(&row, &body)?;
    let digest = census_domain::model::compute_contact_proof(&row, &evidence.claims)?;
    check!(!digest.is_empty());
    let mut mutated_claims = evidence.claims.clone();
    if let Some(claim) = mutated_claims.first_mut() {
        claim.value = "mutated@example.org".to_string();
    }
    let result = census_domain::model::compute_contact_proof(&row, &mutated_claims);
    check!(result.is_err());
    Ok(())
}

#[test]
fn uncertain_or_failed_evidence_never_ships() {
    [
        Verdict::OkRoleContext,
        Verdict::RoleContradicted,
        Verdict::AttributionRequired,
        Verdict::FetchFailed,
        Verdict::Empty,
        Verdict::Mismatch,
        Verdict::RenderRequired,
    ]
    .into_iter()
    .for_each(|verdict| assert!(!verdict.shipped()));
}

#[test]
fn a_body_that_already_carries_the_claimed_person_is_an_attribution_gap() -> TestResult {
    let evidence = evaluate(
        &fragment(),
        &staff("<tr><td>Dana Reid</td></tr><script>window.staff=[]</script>"),
    )?;
    check!(eq; evidence.verdict(), Verdict::AttributionRequired);
    Ok(())
}

#[test]
fn a_body_without_the_claimed_person_stays_a_rendering_gap() -> TestResult {
    let evidence = evaluate(
        &fragment(),
        &staff("<tr><td>Somebody Else</td></tr><script>window.staff=[]</script>"),
    )?;
    check!(eq; evidence.verdict(), Verdict::RenderRequired);
    Ok(())
}

#[test]
fn a_namesake_outside_the_rows_school_stays_unattributed() -> TestResult {
    let evidence = evaluate(
        &fragment(),
        "<table><tr><td>Dana Reid</td></tr></table><script>window.staff=[]</script>",
    )?;
    check!(eq; evidence.verdict(), Verdict::RenderRequired);
    Ok(())
}

#[test]
fn future_observation_dates_cannot_be_verified() -> TestResult {
    let mut row = fragment();
    row.last_observed = "2027-09-21".to_string();
    check!(!evaluate(
        &row,
        &staff("<tr><td>Dana Reid Head XC Coach dana@example.org</td></tr>")
    )?
    .verdict()
    .shipped());
    Ok(())
}

#[test]
fn final_reconciliation_detects_email_or_citation_mutation() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("contacts.csv");
    let row = fragment();
    let evidence = evaluate(
        &row,
        &staff("<tr><td>Dana Reid Head XC Coach dana@example.org</td></tr>"),
    )?;
    let outcome = RowOutcome {
        row,
        verdict: evidence.verdict(),
        evidence: evidence.claims,
    };
    let verified = FragmentOutcome {
        file: "WI.csv".to_string(),
        rows: vec![outcome.clone()],
        counts: Default::default(),
    };
    write_fragment(&path, std::slice::from_ref(&outcome))?;
    check!(eq;
        reconcile(&path, std::slice::from_ref(&verified))?
            .unmatched_total(),
        0
    );
    let mut mutated = outcome.clone();
    mutated.row.public_professional_email = "someoneelse@example.org".to_string();
    write_fragment(&path, &[mutated])?;
    check!(eq;
        reconcile(&path, std::slice::from_ref(&verified))?
            .unmatched_total(),
        1
    );
    let mut mutated = outcome;
    mutated.row.source_urls = vec!["https://different.example.org/staff".to_string()];
    write_fragment(&path, &[mutated])?;
    check!(eq;
        reconcile(&path, &[verified])?
            .unmatched_total(),
        1
    );
    Ok(())
}

#[test]
fn freeze_manifest_covers_every_published_union_file() -> TestResult {
    let dir = tempfile::tempdir()?;
    let union = dir.path().join("union");
    std::fs::create_dir_all(&union)?;
    let state = union.join("WI.csv");
    let sidecar = union.join("WI.csv.evidence.jsonl");
    std::fs::write(&state, "school,city,state\n")?;
    std::fs::write(&sidecar, "{}\n")?;
    let listed = dir.path().join("manifest.txt");
    write_manifest(&listed, &[], &[], Some(&union))?;
    let manifest = std::fs::read_to_string(&listed)?;
    check!(manifest.contains(&format!("{:x}", Sha256::digest(std::fs::read(&state)?))));
    check!(manifest.contains(&format!("{:x}", Sha256::digest(std::fs::read(&sidecar)?))));
    Ok(())
}

#[test]
fn the_state_union_publishes_only_a_same_school_current_director() -> TestResult {
    let dir = tempfile::tempdir()?;
    let cases = [
        (
            "Casey Control",
            "casey@example.org",
            "https://example.org/staff",
            staff("<tr><td>Casey Control Athletic Director casey@example.org</td></tr>"),
        ),
        (
            "Fay Foreign",
            "fay@example.org",
            "https://example.org/foreign",
            "<h1>Other High School WI</h1><table><tr><td>Fay Foreign Athletic Director fay@example.org</td></tr></table>".to_string(),
        ),
        (
            "Sam Staten",
            "sam@example.org",
            "https://example.org/staten",
            "<h1>Mosinee High School MN</h1><table><tr><td>Sam Staten Athletic Director sam@example.org</td></tr></table>".to_string(),
        ),
        (
            "Fran Former",
            "fran@example.org",
            "https://example.org/former",
            staff("<tr><td>Fran Former Former Athletic Director fran@example.org</td></tr>"),
        ),
    ];
    let mut rows = Vec::new();
    for (name, email, url, body) in &cases {
        let mut row = director_fragment();
        row.ad_name = name.to_string();
        row.ad_email = email.to_string();
        row.source_urls = vec![url.to_string()];
        let evidence = evaluate(&row, body)?;
        rows.push(RowOutcome {
            row,
            verdict: evidence.verdict(),
            evidence: evidence.claims.clone(),
        });
    }
    let union = dir.path().join("union");
    let staged = write_state_union(
        &union,
        &[FragmentOutcome {
            file: "WI.csv".to_string(),
            rows,
            counts: Default::default(),
        }],
    )?;
    check!(eq; staged.get("WI").copied(), Some(1));
    let published = std::fs::read_to_string(union.join("WI.csv"))?;
    check!(published.contains("casey@example.org"));
    check!(!published.contains("fay@example.org"));
    check!(!published.contains("sam@example.org"));
    check!(!published.contains("fran@example.org"));
    Ok(())
}

#[test]
fn union_staging_publishes_reconcilable_evidence() -> TestResult {
    let dir = tempfile::tempdir()?;
    let row = fragment();
    let evidence = evaluate(
        &row,
        &staff("<tr><td>Dana Reid Head XC Coach dana@example.org</td></tr>"),
    )?;
    let outcome = RowOutcome {
        row,
        verdict: evidence.verdict(),
        evidence: evidence.claims.clone(),
    };
    let verified = FragmentOutcome {
        file: "WI.csv".to_string(),
        rows: vec![outcome.clone()],
        counts: Default::default(),
    };
    let union = dir.path().join("union");
    let staged = write_state_union(&union, std::slice::from_ref(&verified))?;
    check!(eq; staged.get("WI").copied(), Some(1));
    let sidecar = union.join("WI.csv.evidence.jsonl");
    check!(sidecar.is_file());
    check!(eq; read_evidence_jsonl(&sidecar)?, outcome.evidence);
    let reconciliation = reconcile(&union.join("WI.csv"), std::slice::from_ref(&verified))?;
    check!(eq; reconciliation.unmatched_total(), 0);
    check!(eq; reconciliation.tampered_total(), 0);
    Ok(())
}

fn scratch_script(scratch: &Path, tag: &str, body: &str) -> TestResult<PathBuf> {
    use std::os::unix::fs::PermissionsExt;

    let path = scratch.join(format!("{tag}.sh"));
    std::fs::write(&path, body)?;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700))?;
    Ok(path)
}

#[test]
fn an_over_cap_coach_converter_refuses_while_admitted_output_survives() -> TestResult {
    let scratch = tempfile::tempdir()?;
    let converter = scratch_script(
        scratch.path(),
        "over-cap",
        "#!/bin/sh\nhead -c 300000 /dev/zero | tr '\\0' 'a'\n",
    )?;
    let binary = converter.to_string_lossy().into_owned();
    let body = b"%PDF-1.4 coach body";
    let refused = super::fetch::inflate_pdf_under(&binary, body, Duration::from_secs(60), 4096);
    check!(
        refused.is_none(),
        "converted text over the cap must be refused: {refused:?}"
    );
    let inflated =
        super::fetch::inflate_pdf_under(&binary, body, Duration::from_secs(60), 1_048_576)
            .ok_or("bounded converter refused admitted output")?;
    check!(eq; inflated.len(), 300_000);
    check!(inflated.bytes().all(|byte| byte == b'a'));
    scratch.close()?;
    Ok(())
}

#[test]
fn a_stalled_coach_converter_refuses_at_its_deadline() -> TestResult {
    let scratch = tempfile::tempdir()?;
    let converter = scratch_script(scratch.path(), "stall", "#!/bin/sh\nexec sleep 300\n")?;
    let started = Instant::now();
    let refused = super::fetch::inflate_pdf_under(
        &converter.to_string_lossy(),
        b"%PDF-1.4 coach body",
        Duration::from_millis(200),
        1_048_576,
    );
    check!(
        refused.is_none(),
        "a stalled coach converter must be refused: {refused:?}"
    );
    check!(
        started.elapsed() < Duration::from_secs(20),
        "a stalled coach converter must be killed at its deadline"
    );
    scratch.close()?;
    Ok(())
}

#[test]
fn reconciliation_opens_sidecar_once() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("contacts.csv");
    let evidence = evaluate(
        &fragment(),
        &staff("<tr><td>Dana Reid Head XC Coach dana@example.org</td></tr>"),
    )?;
    let outcome = RowOutcome {
        row: fragment(),
        verdict: evidence.verdict(),
        evidence: evidence.claims,
    };
    let mut rows = Vec::new();
    for index in 0..32 {
        let mut unique = outcome.clone();
        unique.row.school = format!("School {index}");
        for claim in &mut unique.evidence {
            claim.school = unique.row.school.clone();
        }
        rows.push(unique);
    }
    write_fragment(&path, &rows)?;
    let verified = FragmentOutcome {
        file: "WI.csv".to_string(),
        rows,
        counts: Default::default(),
    };
    super::output::metrics::OPENS.with(|count| count.set(0));
    super::output::metrics::COMPARISONS.with(|count| count.set(0));
    let result = reconcile(&path, &[verified]);
    let opens = super::output::metrics::OPENS.with(|count| count.get());
    let comparisons = super::output::metrics::COMPARISONS.with(|count| count.get());
    println!("CONTACT-11 R=32 opens={opens} comparisons={comparisons}");
    check!(eq; opens, 1);
    check!(eq; comparisons, 64);
    let report = result?;
    check!(eq; report.published, 32);
    check!(eq; report.unmatched_total(), 0);
    Ok(())
}

#[test]
fn contact_existing_fixture_digest() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("contacts.csv");
    let evidence = evaluate(
        &fragment(),
        &staff("<tr><td>Dana Reid Head XC Coach dana@example.org</td></tr>"),
    )?;
    let outcome = RowOutcome {
        row: fragment(),
        verdict: evidence.verdict(),
        evidence: evidence.claims,
    };
    write_fragment(&path, std::slice::from_ref(&outcome))?;
    let mut hasher = Sha256::new();
    hasher.update(std::fs::read(&path)?);
    hasher.update(std::fs::read(evidence_path(&path))?);
    let digest = format!("{:x}", hasher.finalize());
    println!("CONTACT-11 contact fixture sha256={digest}");
    check!(eq; digest, "5a9681333926d40f83dedeb228f8c64591d74c944a89f257a95a633ba23b9c2a");
    Ok(())
}

#[test]
fn indexed_evidence_preserves_duplicate_urls_claims_and_order() -> TestResult {
    let mut row = fragment();
    let url = row.source_urls.first().ok_or("source URL")?.clone();
    row.source_urls.push(url);
    let evidence = evaluate(
        &row,
        &staff("<tr><td>Dana Reid Head XC Coach dana@example.org</td></tr>"),
    )?;
    let claims: Vec<_> = evidence.claims.iter().cycle().take(256).cloned().collect();
    let index = super::ContactEvidenceIndex::build(&claims)?;
    check!(eq; index.claims_for_row(&row)?, claims);
    Ok(())
}

#[test]
fn evidence_read_refuses_an_oversized_record_before_parsing() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("oversized.evidence.jsonl");
    std::fs::write(&path, vec![b' '; 1_048_577])?;
    let error = match read_evidence_jsonl(&path) {
        Err(error) => error,
        Ok(_) => return Err("oversized evidence accepted".into()),
    };
    check!(eq; error.root_cause().to_string(), "contact record bytes: 1048577 exceeds 1048576");
    Ok(())
}

#[test]
fn evidence_read_reports_a_malformed_later_record() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("malformed.evidence.jsonl");
    let evidence = evaluate(
        &fragment(),
        &staff("<tr><td>Dana Reid Head XC Coach dana@example.org</td></tr>"),
    )?;
    let claim = evidence.claims.first().ok_or("claim")?;
    let body = format!("{}\n{{\n", serde_json::to_string(claim)?);
    std::fs::write(&path, body)?;
    let error = match read_evidence_jsonl(&path) {
        Err(error) => error,
        Ok(_) => return Err("malformed later evidence discarded".into()),
    };
    check!(error.to_string().contains("parse evidence"));
    Ok(())
}

#[test]
fn reconciliation_keeps_distinct_subjects_at_one_program() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("contacts.csv");
    let evidence = evaluate(
        &fragment(),
        &staff("<tr><td>Dana Reid Head XC Coach dana@example.org</td></tr>"),
    )?;
    let first = RowOutcome {
        row: fragment(),
        verdict: evidence.verdict(),
        evidence: evidence.claims,
    };
    let mut second = first.clone();
    second.row.coach_name = "Casey Reid".to_string();
    for claim in &mut second.evidence {
        claim.person = second.row.coach_name.clone();
        if claim.field == ContactProofField::CoachName {
            claim.value = second.row.coach_name.clone();
        }
    }
    let verified = FragmentOutcome {
        file: "WI.csv".to_string(),
        rows: vec![first, second],
        counts: Default::default(),
    };
    write_fragment(&path, &verified.rows)?;
    let report = reconcile(&path, &[verified])?;
    check!(eq; report.verified, 2);
    check!(eq; report.published, 2);
    check!(eq; report.unmatched_total(), 0);
    check!(eq; report.tampered_total(), 0);
    Ok(())
}

#[test]
fn reconciliation_work_budget_is_a_reported_error() -> TestResult {
    let mut work = 8_000_000;
    let error = super::limits::charge_work(&mut work, &fragment(), 2)
        .err()
        .ok_or("work limit was not enforced")?;
    check!(eq; error.to_string(), "contact reconciliation work: 8000008 exceeds 8000000");
    Ok(())
}

#[cfg(unix)]
#[path = "tests/pdf_security.rs"]
mod pdf_security;
