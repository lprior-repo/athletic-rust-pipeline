use super::judge::resolves_sport;
use super::*;
use census_domain::model::{ContactClaimEvidence, ContactProofField};
use std::collections::BTreeSet;
use tempfile::NamedTempFile;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn make_row(fields: Vec<&str>) -> Row {
    let s: Vec<String> = fields.iter().map(|s| s.to_string()).collect();
    Row::from_fields(s)
}

#[test]
fn multi_url_trims_to_first() {
    let mut row = make_row(vec![
        "School",
        "",
        "OH",
        "Cross Country",
        "Head Coach",
        "Coach",
        "",
        "",
        "",
        "https://first.example.com https://second.example.com",
        "2026-09-22",
    ]);
    row.normalize();
    assert_eq!(row.source_url, "https://first.example.com");
}

#[test]
fn professional_mail_kept() {
    let mut row = make_row(vec![
        "School",
        "",
        "OH",
        "Cross Country",
        "Head Coach",
        "Coach",
        "coach@school.k12.oh.us",
        "",
        "ad@school.k12.oh.us",
        "",
        "2026-09-22",
    ]);
    row.normalize();
    assert_eq!(row.public_professional_email, "coach@school.k12.oh.us");
    assert_eq!(row.ad_email, "ad@school.k12.oh.us");
}

#[test]
fn url_with_no_https_stays_untrimmed() {
    let mut row = make_row(vec![
        "School",
        "",
        "OH",
        "Cross Country",
        "Head Coach",
        "Coach",
        "",
        "",
        "",
        "not-a-url",
        "2026-09-22",
    ]);
    row.normalize();
    assert_eq!(row.source_url, "not-a-url");
}

#[test]
fn director_with_empty_sport_passes() {
    let row = make_row(vec![
        "School",
        "City",
        "OH",
        "",
        "Athletic Director",
        "",
        "",
        "",
        "ad@school.edu",
        "https://school.edu",
        "2026-09-22",
    ]);
    assert!(row.judge("OH").is_none());
}

#[test]
fn director_with_nonempty_sport_rejected() -> TestResult {
    let row = make_row(vec![
        "School",
        "City",
        "OH",
        "Track",
        "Athletic Director",
        "",
        "",
        "",
        "ad@school.edu",
        "https://school.edu",
        "2026-09-22",
    ]);
    let reason = row.judge("OH").ok_or("expected rejection")?;
    check!(reason.contains("director row must leave sport empty"));
    Ok(())
}

#[test]
fn coach_with_resolvable_sport_passes() {
    let row = make_row(vec![
        "School",
        "City",
        "OH",
        "Boys Cross Country",
        "Head Coach",
        "Coach Name",
        "coach@school.edu",
        "",
        "",
        "https://school.edu",
        "2026-09-22",
    ]);
    assert!(row.judge("OH").is_none());
}

#[test]
fn coach_with_indoor_sport_passes() {
    let row = make_row(vec![
        "School",
        "City",
        "OH",
        "Indoor Track",
        "Head Coach",
        "Coach Name",
        "coach@school.edu",
        "",
        "",
        "https://school.edu",
        "2026-09-22",
    ]);
    assert!(row.judge("OH").is_none());
}

#[test]
fn coach_with_unrelated_sport_rejected() -> TestResult {
    let row = make_row(vec![
        "School",
        "City",
        "OH",
        "Football",
        "Head Coach",
        "Coach Name",
        "coach@school.edu",
        "",
        "",
        "https://school.edu",
        "2026-09-22",
    ]);
    let reason = row.judge("OH").ok_or("expected rejection")?;
    check!(reason.contains("no sport resolvable"));
    Ok(())
}

#[test]
fn coach_with_no_name_rejected() -> TestResult {
    let row = make_row(vec![
        "School",
        "City",
        "OH",
        "Cross Country",
        "Head Coach",
        "",
        "",
        "",
        "",
        "https://school.edu",
        "2026-09-22",
    ]);
    let reason = row.judge("OH").ok_or("expected rejection")?;
    check!(reason.contains("no coach name"));
    Ok(())
}

#[test]
fn role_neither_coach_nor_director_rejected() -> TestResult {
    let row = make_row(vec![
        "School",
        "City",
        "OH",
        "Cross Country",
        "Athletic Trainer",
        "Coach",
        "coach@school.edu",
        "",
        "",
        "https://school.edu",
        "2026-09-22",
    ]);
    let reason = row.judge("OH").ok_or("expected rejection")?;
    check!(reason.contains("neither a coach nor a director"));
    Ok(())
}

#[test]
fn role_both_coach_and_director_rejected() -> TestResult {
    let row = make_row(vec![
        "School",
        "City",
        "OH",
        "",
        "Assistant Athletic Director Coach",
        "Coach",
        "",
        "",
        "",
        "https://school.edu",
        "2026-09-22",
    ]);
    let reason = row.judge("OH").ok_or("expected rejection")?;
    check!(reason.contains("both a coach and a director"));
    Ok(())
}

#[test]
fn placeholder_names_rejected() -> TestResult {
    for placeholder in &[
        "vacant", "TBA", "TBD", "NA", "n/a", "none", "unknown", "---",
    ] {
        let row = make_row(vec![
            "School",
            "City",
            "OH",
            "Cross Country",
            "Head Coach",
            placeholder,
            "",
            "",
            "",
            "https://school.edu",
            "2026-09-22",
        ]);
        let reason = row.judge("OH").ok_or("expected rejection")?;
        check!(
            reason.contains("placeholder name"),
            "expected placeholder rejection for {placeholder}, got: {reason}"
        );
    }
    Ok(())
}

#[test]
fn phone_in_school_rejected() -> TestResult {
    let row = make_row(vec![
        "555-123-4567",
        "City",
        "OH",
        "Cross Country",
        "Head Coach",
        "Coach",
        "coach@school.edu",
        "",
        "",
        "https://school.edu",
        "2026-09-22",
    ]);
    let reason = row.judge("OH").ok_or("expected rejection")?;
    check!(reason.contains("phone-like value in school"));
    Ok(())
}

#[test]
fn invalid_email_rejected() -> TestResult {
    let row = make_row(vec![
        "School",
        "City",
        "OH",
        "Cross Country",
        "Head Coach",
        "Coach",
        "not-an-email",
        "",
        "",
        "https://school.edu",
        "2026-09-22",
    ]);
    let reason = row.judge("OH").ok_or("expected rejection")?;
    check!(reason.contains("not an email"));
    Ok(())
}

#[test]
fn personal_mail_domain_in_email_rejected() -> TestResult {
    let row = make_row(vec![
        "School",
        "City",
        "OH",
        "Cross Country",
        "Head Coach",
        "Coach",
        "coach@gmail.com",
        "",
        "",
        "https://school.edu",
        "2026-09-22",
    ]);
    let reason = row.judge("OH").ok_or("expected rejection")?;
    check!(reason.contains("personal mail domain"));
    Ok(())
}

#[test]
fn missing_school_rejected() -> TestResult {
    let row = make_row(vec![
        "",
        "City",
        "OH",
        "Cross Country",
        "Head Coach",
        "Coach",
        "coach@school.edu",
        "",
        "",
        "https://school.edu",
        "2026-09-22",
    ]);
    let reason = row.judge("OH").ok_or("expected rejection")?;
    check!(reason.contains("missing school"));
    Ok(())
}

#[test]
fn no_contact_published_rejected() -> TestResult {
    let row = make_row(vec![
        "School",
        "City",
        "OH",
        "",
        "Athletic Director",
        "",
        "",
        "",
        "",
        "https://school.edu",
        "2026-09-22",
    ]);
    let reason = row.judge("OH").ok_or("expected rejection")?;
    check!(reason.contains("no contact published"));
    Ok(())
}

#[test]
fn state_mismatch_rejected() -> TestResult {
    let row = make_row(vec![
        "School",
        "City",
        "IN",
        "Cross Country",
        "Head Coach",
        "Coach",
        "coach@school.edu",
        "",
        "",
        "https://school.edu",
        "2026-09-22",
    ]);
    let reason = row.judge("OH").ok_or("expected rejection")?;
    check!(reason.contains("does not match fragment"));
    Ok(())
}

#[test]
fn valid_url_passes() {
    let row = make_row(vec![
        "School",
        "City",
        "OH",
        "Cross Country",
        "Head Coach",
        "Coach",
        "coach@school.edu",
        "",
        "",
        "https://school.edu/coaches",
        "2026-09-22",
    ]);
    assert!(row.judge("OH").is_none());
}

#[test]
fn invalid_url_rejected() -> TestResult {
    let row = make_row(vec![
        "School",
        "City",
        "OH",
        "Cross Country",
        "Head Coach",
        "Coach",
        "coach@school.edu",
        "",
        "",
        "not-a-url",
        "2026-09-22",
    ]);
    let reason = row.judge("OH").ok_or("expected rejection")?;
    check!(reason.contains("not a URL"));
    Ok(())
}

#[test]
fn resolves_track() {
    assert!(resolves_sport("Boys Track"));
    assert!(resolves_sport("Girls Track and Field"));
}

#[test]
fn resolves_cross_country() {
    assert!(resolves_sport("Cross Country"));
    assert!(resolves_sport("Boys Cross Country"));
    assert!(resolves_sport("Girls Cross-country"));
}

#[test]
fn resolves_indoor() {
    assert!(resolves_sport("Indoor Track"));
}

#[test]
fn does_not_resolve_soccer() {
    assert!(!resolves_sport("Soccer"));
    assert!(!resolves_sport("Basketball"));
}

struct ClaimSpec<'a> {
    field: ContactProofField,
    value: &'a str,
    person: &'a str,
    sport: &'a str,
    school: &'a str,
    state: &'a str,
    source_url: &'a str,
}

impl<'a> ClaimSpec<'a> {
    fn new(
        field: ContactProofField,
        value: &'a str,
        person: &'a str,
        sport: &'a str,
        school: &'a str,
        state: &'a str,
        source_url: &'a str,
    ) -> Self {
        ClaimSpec {
            field,
            value,
            person,
            sport,
            school,
            state,
            source_url,
        }
    }
}

fn claim(spec: ClaimSpec<'_>) -> ContactClaimEvidence {
    ContactClaimEvidence {
        field: spec.field,
        value: spec.value.to_string(),
        person: spec.person.to_string(),
        role: "Head Coach".to_string(),
        sport: spec.sport.to_string(),
        school: spec.school.to_string(),
        state: spec.state.to_string(),
        source_url: spec.source_url.to_string(),
        claimed_observed_on: "2026-09-22".to_string(),
        source_sha256: "a".repeat(64),
        fetched_at: "2026-09-23T10:00:00Z".to_string(),
        span: format!("<span>{}</span>", spec.value),
    }
}

fn write_evidence(path: &std::path::Path, claims: &[ContactClaimEvidence]) -> TestResult {
    let mut body = String::new();
    for claim in claims {
        body.push_str(&serde_json::to_string(claim)?);
        body.push('\n');
    }
    std::fs::write(path, body)?;
    Ok(())
}

fn proven_digest(
    fields: &[&str],
    claims: &[ContactClaimEvidence],
) -> Result<String, Box<dyn std::error::Error>> {
    let mut row = make_row(fields.to_vec());
    row.normalize();
    let raw = raw_contact_row(&row);
    let covering = census_service::coachverify::claims_for_row(claims, &raw);
    Ok(census_domain::model::compute_contact_proof(
        &raw, &covering,
    )?)
}

fn write_fragment_csv(
    path: &std::path::Path,
    rows: &[Vec<&str>],
    claims: &[ContactClaimEvidence],
) -> TestResult {
    let header = "school,city,state,sport,role,coach_name,public_professional_email,ad_name,ad_email,source_url,last_observed,verified_proof_digest";
    let mut body = String::from(header);
    body.push('\n');
    for fields in rows {
        body.push_str(&fields.join(","));
        body.push(',');
        body.push_str(&proven_digest(fields, claims)?);
        body.push('\n');
    }
    std::fs::write(path, body)?;
    Ok(())
}

fn tamper_cell(path: &std::path::Path, from: &str, to: &str) -> TestResult {
    let body = std::fs::read_to_string(path)?;
    check!(body.contains(from), "fixture cell {from} is present");
    std::fs::write(path, body.replacen(from, to, 1))?;
    Ok(())
}

fn tamper_digest(path: &std::path::Path, replacement: &str) -> TestResult {
    let body = std::fs::read_to_string(path)?;
    let mut lines: Vec<String> = body.lines().map(str::to_string).collect();
    let last = lines.len().saturating_sub(1);
    if let Some(line) = lines.get_mut(last) {
        if let Some((head, _)) = line.rsplit_once(',') {
            *line = format!("{head},{replacement}");
        }
    }
    std::fs::write(path, lines.join("\n") + "\n")?;
    Ok(())
}

#[test]
fn merge_round_trip_keeps_every_row_importable_and_distinct() -> TestResult {
    let dir = tempfile::tempdir()?;
    let wi_claims = vec![
        claim(ClaimSpec::new(
            ContactProofField::CoachName,
            "Dana Reed",
            "Dana Reed",
            "Track & Field",
            "Madison West High School",
            "WI",
            "https://madisonwest.example.org/athletics",
        )),
        claim(ClaimSpec::new(
            ContactProofField::PublicProfessionalEmail,
            "dana.reed@madisonwest.example.org",
            "Dana Reed",
            "Track & Field",
            "Madison West High School",
            "WI",
            "https://madisonwest.example.org/athletics",
        )),
        claim(ClaimSpec::new(
            ContactProofField::CoachName,
            "Sam Ellery",
            "Sam Ellery",
            "Cross Country",
            "Madison West High School",
            "WI",
            "https://madisonwest.example.org/athletics",
        )),
        claim(ClaimSpec::new(
            ContactProofField::PublicProfessionalEmail,
            "sam.ellery@madisonwest.example.org",
            "Sam Ellery",
            "Cross Country",
            "Madison West High School",
            "WI",
            "https://madisonwest.example.org/athletics",
        )),
    ];
    let mn_claims = vec![
        claim(ClaimSpec::new(
            ContactProofField::CoachName,
            "Rae Lindqvist",
            "Rae Lindqvist",
            "Track & Field",
            "Washburn High School",
            "MN",
            "https://washburn.example.org/athletics",
        )),
        claim(ClaimSpec::new(
            ContactProofField::PublicProfessionalEmail,
            "rae.lindqvist@washburn.example.org",
            "Rae Lindqvist",
            "Track & Field",
            "Washburn High School",
            "MN",
            "https://washburn.example.org/athletics",
        )),
    ];
    write_fragment_csv(
        &dir.path().join("WI.csv"),
        &[
            vec![
                "Madison West High School",
                "Madison",
                "WI",
                "Track & Field",
                "Head Coach",
                "Dana Reed",
                "",
                "",
                "",
                "https://madisonwest.example.org/athletics",
                "2026-09-22",
            ],
            vec![
                "Madison West High School",
                "Madison",
                "WI",
                "Track & Field",
                "Head Coach",
                "Dana Reed",
                "dana.reed@madisonwest.example.org",
                "",
                "",
                "https://madisonwest.example.org/athletics",
                "2026-09-22",
            ],
            vec![
                "Madison West High School",
                "Madison",
                "WI",
                "Cross Country",
                "Head Coach",
                "Sam Ellery",
                "sam.ellery@madisonwest.example.org",
                "",
                "",
                "https://madisonwest.example.org/athletics",
                "2026-09-22",
            ],
        ],
        &wi_claims,
    )?;
    write_fragment_csv(
        &dir.path().join("MN.csv"),
        &[vec![
            "Washburn High School",
            "Minneapolis",
            "MN",
            "Track & Field",
            "Head Coach",
            "Rae Lindqvist",
            "rae.lindqvist@washburn.example.org",
            "",
            "",
            "https://washburn.example.org/athletics",
            "2026-09-22",
        ]],
        &mn_claims,
    )?;
    write_evidence(&dir.path().join("WI.csv.evidence.jsonl"), &wi_claims)?;
    write_evidence(&dir.path().join("MN.csv.evidence.jsonl"), &mn_claims)?;

    let out = NamedTempFile::new()?;
    let report = NamedTempFile::new()?;
    let args = MergeCoachesArgs {
        fragments: dir.path().to_path_buf(),
        out: out.path().to_path_buf(),
        report: report.path().to_path_buf(),
    };
    run_merge_coaches(&args)?;

    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(out.path())?;
    let mut seen: BTreeSet<(String, String, String, String)> = BTreeSet::new();
    let mut data_rows = 0usize;
    for record in reader.records() {
        let record = record?;
        let fields: Vec<String> = record.iter().map(str::to_owned).collect();
        let mut row = Row::from_fields(fields);
        row.normalize();
        data_rows += 1;
        check!(
            super::judge::judge(&row, &row.state).is_none(),
            "merged row {data_rows} is not importable: {row:?}",
        );
        check!(
            seen.insert(row.dedupe_key()),
            "merged row {data_rows} duplicates an earlier row after round-trip: {row:?}",
        );
    }
    check!(eq; data_rows, 3, "two states, one deduped pair");
    let report = std::fs::read_to_string(report.path())?;
    check!(
        report.contains("WI") && report.contains("MN"),
        "the report accounts for both states: {report}"
    );
    Ok(())
}

#[test]
fn dedupe_keeps_richer_row() {
    let mut row1 = make_row(vec![
        "School",
        "City",
        "OH",
        "Cross Country",
        "Head Coach",
        "Coach",
        "",
        "",
        "",
        "https://school.edu",
        "2026-09-22",
    ]);
    let mut row2 = make_row(vec![
        "School",
        "City",
        "OH",
        "Cross Country",
        "Head Coach",
        "Coach B",
        "coach@school.edu",
        "",
        "",
        "https://school.edu",
        "2026-09-22",
    ]);
    row1.normalize();
    row2.normalize();
    let key = row1.dedupe_key();
    assert_eq!(key, row2.dedupe_key());
    assert!(
        pick_richer(&row2, &row1),
        "the row that publishes a professional email is the richer one"
    );
    assert!(
        !pick_richer(&row1, &row2),
        "a row without an email never replaces the row that publishes one"
    );
    let kept = if pick_richer(&row2, &row1) {
        &row2
    } else {
        &row1
    };
    assert_eq!(kept.coach_name, "Coach B");
}

#[test]
fn dedupe_keeps_newer_row() {
    let mut row1 = make_row(vec![
        "School",
        "City",
        "OH",
        "Cross Country",
        "Head Coach",
        "Coach",
        "coach@school.edu",
        "",
        "",
        "https://school.edu",
        "2026-09-21",
    ]);
    let mut row2 = make_row(vec![
        "School",
        "City",
        "OH",
        "Cross Country",
        "Head Coach",
        "Coach B",
        "coach2@school.edu",
        "",
        "",
        "https://school.edu",
        "2026-09-22",
    ]);
    row1.normalize();
    row2.normalize();
    let key = row1.dedupe_key();
    assert_eq!(key, row2.dedupe_key());
    assert!(
        pick_richer(&row2, &row1),
        "the later observation wins when both publish an email"
    );
    assert!(
        !pick_richer(&row1, &row2),
        "an older observation never replaces a newer one"
    );
    let kept = if pick_richer(&row2, &row1) {
        &row2
    } else {
        &row1
    };
    assert_eq!(kept.coach_name, "Coach B");
}

fn verified_director(state: &str, school: &str) -> census_service::coachverify::FragmentOutcome {
    use census_domain::model::{ContactClaimEvidence, ContactProofField, RawContactRow};
    use census_service::coachverify::{FragmentOutcome, RowOutcome, Verdict};
    let row = RawContactRow {
        school: school.to_string(),
        city: "Alpha".to_string(),
        state: state.to_string(),
        sport: String::new(),
        role: "Athletic Director".to_string(),
        coach_name: String::new(),
        public_professional_email: String::new(),
        ad_name: "Dana Reid".to_string(),
        ad_email: String::new(),
        source_urls: vec!["https://example.org/staff".to_string()],
        last_observed: "2026-09-21".to_string(),
    };
    let claims = vec![ContactClaimEvidence {
        field: ContactProofField::AdName,
        value: row.ad_name.clone(),
        person: row.ad_name.clone(),
        role: row.role.clone(),
        sport: String::new(),
        school: row.school.clone(),
        state: row.state.clone(),
        source_url: row.source_urls[0].clone(),
        claimed_observed_on: row.last_observed.clone(),
        source_sha256: "a".repeat(64),
        fetched_at: "2026-09-22T10:00:00Z".to_string(),
        span: "<span class=\"name\">Dana Reid</span>".to_string(),
    }];
    FragmentOutcome {
        file: format!("{state}.csv"),
        rows: vec![RowOutcome {
            row,
            verdict: Verdict::Ok,
            evidence: claims,
        }],
        counts: Default::default(),
    }
}

#[test]
fn merged_product_carries_reconcilable_evidence() -> TestResult {
    let dir = tempfile::tempdir()?;
    let union = dir.path().join("union");
    let verified = verified_director("OH", "Alpha High School");
    census_service::coachverify::write_state_union(&union, std::slice::from_ref(&verified))?;
    let out = dir.path().join("coach-contacts.csv");
    run_merge_coaches(&MergeCoachesArgs {
        fragments: union,
        out: out.clone(),
        report: dir.path().join("merge.md"),
    })?;
    check!(eq; std::fs::read_to_string(&out)?.lines().count(), 2);
    let sidecar = census_service::coachverify::evidence_path(&out);
    check!(
        eq;
        census_service::coachverify::read_evidence_jsonl(&sidecar)?,
        verified.rows[0].evidence
    );
    let reconciliation =
        census_service::coachverify::reconcile(&out, std::slice::from_ref(&verified))?;
    check!(eq; reconciliation.published, 1);
    check!(eq; reconciliation.unmatched_total(), 0);
    check!(eq; reconciliation.tampered_total(), 0);
    Ok(())
}

#[test]
fn merged_product_preserves_repeated_claims_for_digest_fidelity() -> TestResult {
    let dir = tempfile::tempdir()?;
    let union = dir.path().join("union");
    let mut verified = verified_director("OH", "Alpha High School");
    let repeated = verified.rows[0].evidence[0].clone();
    verified.rows[0].evidence.push(repeated);
    census_service::coachverify::write_state_union(&union, std::slice::from_ref(&verified))?;
    let out = dir.path().join("coach-contacts.csv");
    run_merge_coaches(&MergeCoachesArgs {
        fragments: union,
        out: out.clone(),
        report: dir.path().join("merge.md"),
    })?;
    check!(
        eq;
        census_service::coachverify::read_evidence_jsonl(
            &census_service::coachverify::evidence_path(&out)
        )?,
        verified.rows[0].evidence,
        "the merged sidecar must carry the row's claims verbatim, repeated claims included"
    );
    let reconciliation =
        census_service::coachverify::reconcile(&out, std::slice::from_ref(&verified))?;
    check!(eq; reconciliation.tampered_total(), 0);
    Ok(())
}

#[test]
fn merge_keeps_a_state_cells_case_so_the_proof_still_validates() -> TestResult {
    let dir = tempfile::tempdir()?;
    let union = dir.path().join("union");
    let verified = verified_director("oh", "Alpha High School");
    census_service::coachverify::write_state_union(&union, std::slice::from_ref(&verified))?;
    check!(
        union.join("OH.csv").is_file(),
        "the union files a row under the uppercased state"
    );
    let out = dir.path().join("coach-contacts.csv");
    run_merge_coaches(&MergeCoachesArgs {
        fragments: union,
        out: out.clone(),
        report: dir.path().join("merge.md"),
    })?;
    let published = census_service::coachverify::read_fragment(&out)?;
    check!(eq; published.len(), 1);
    check!(
        eq;
        published[0].state,
        "oh",
        "the merge publishes the state cell as the union wrote it"
    );
    let reconciliation =
        census_service::coachverify::reconcile(&out, std::slice::from_ref(&verified))?;
    check!(eq; reconciliation.published, 1);
    check!(eq; reconciliation.tampered_total(), 0);
    Ok(())
}

#[test]
fn merge_refuses_a_fragment_without_evidence() -> TestResult {
    let dir = tempfile::tempdir()?;
    let union = dir.path().join("union");
    std::fs::create_dir_all(&union)?;
    std::fs::write(
        union.join("OH.csv"),
        format!(
            "school,city,state,sport,role,coach_name,public_professional_email,ad_name,ad_email,source_url,last_observed,verified_proof_digest\nAlpha High School,Alpha,OH,,Athletic Director,,,Dana Reid,,https://example.org/staff,2026-09-21,{}\n",
            "a".repeat(64)
        ),
    )?;
    let refused = run_merge_coaches(&MergeCoachesArgs {
        fragments: union,
        out: dir.path().join("coach-contacts.csv"),
        report: dir.path().join("merge.md"),
    });
    check!(
        refused.is_err(),
        "a fragment without its evidence sidecar cannot be merged"
    );
    check!(!dir.path().join("coach-contacts.csv").exists());
    Ok(())
}

#[test]
fn merge_rejects_rows_whose_proof_does_not_verify() -> TestResult {
    let dir = tempfile::tempdir()?;
    let union = dir.path().join("union");
    let states = ["WI", "MN", "OH", "IA", "TX", "NV"];
    let outcomes: Vec<_> = states
        .iter()
        .map(|state| verified_director(state, &format!("{state} High School")))
        .collect();
    census_service::coachverify::write_state_union(&union, &outcomes)?;
    tamper_cell(
        &union.join("MN.csv"),
        "https://example.org/staff",
        "https://example.org/other",
    )?;
    tamper_cell(&union.join("OH.csv"), "2026-09-21", "2026-09-20")?;
    tamper_digest(&union.join("IA.csv"), &"b".repeat(64))?;
    tamper_digest(&union.join("TX.csv"), "nothex")?;
    std::fs::write(union.join("NV.csv.evidence.jsonl"), "")?;
    let ks_claims = vec![
        claim(ClaimSpec::new(
            ContactProofField::CoachName,
            "Kay Ross",
            "Kay Ross",
            "Cross Country",
            "KS High School",
            "KS",
            "https://example.org/ks",
        )),
        claim(ClaimSpec::new(
            ContactProofField::PublicProfessionalEmail,
            "kay.ross@example.org",
            "Kay Ross",
            "Cross Country",
            "KS High School",
            "KS",
            "https://example.org/ks",
        )),
    ];
    write_fragment_csv(
        &union.join("KS.csv"),
        &[vec![
            "KS High School",
            "Alpha",
            "KS",
            "Cross Country",
            "Head Coach",
            "Kay Ross",
            "kay.ross@example.org",
            "",
            "",
            "https://example.org/ks",
            "2026-09-22",
        ]],
        &ks_claims,
    )?;
    write_evidence(&union.join("KS.csv.evidence.jsonl"), &ks_claims)?;
    let ne_claims = vec![
        claim(ClaimSpec::new(
            ContactProofField::CoachName,
            "Nia Ellery",
            "Nia Ellery",
            "Track & Field",
            "NE High School",
            "NE",
            "https://example.org/ne",
        )),
        claim(ClaimSpec::new(
            ContactProofField::PublicProfessionalEmail,
            "nia.ellery@example.org",
            "Nia Ellery",
            "Track & Field",
            "NE High School",
            "NE",
            "https://example.org/ne",
        )),
    ];
    write_fragment_csv(
        &union.join("NE.csv"),
        &[vec![
            "NE High School",
            "Alpha",
            "NE",
            "Track & Field",
            "Head Coach",
            "Nia Ellery",
            "nia.ellery@example.org",
            "",
            "",
            "https://example.org/ne",
            "2026-09-22",
        ]],
        &ne_claims,
    )?;
    write_evidence(&union.join("NE.csv.evidence.jsonl"), &ne_claims)?;
    tamper_cell(
        &union.join("KS.csv"),
        "kay.ross@example.org",
        "attacker@example.org",
    )?;
    tamper_cell(&union.join("NE.csv"), "Head Coach", "Assistant Coach")?;

    let out = dir.path().join("coach-contacts.csv");
    run_merge_coaches(&MergeCoachesArgs {
        fragments: union,
        out: out.clone(),
        report: dir.path().join("merge.md"),
    })?;

    let published = census_service::coachverify::read_fragment(&out)?;
    check!(eq; published.len(), 1, "only the intact row publishes");
    check!(eq; published[0].school, "WI High School");
    let sidecar = census_service::coachverify::evidence_path(&out);
    check!(
        eq;
        census_service::coachverify::read_evidence_jsonl(&sidecar)?,
        outcomes[0].rows[0].evidence,
        "the sidecar carries only the verified row's claims"
    );
    let report = std::fs::read_to_string(dir.path().join("merge.md"))?;
    check!(
        eq;
        report.matches("proof not verified").count(),
        7,
        "every tampered row is rejected as unproven: {report}"
    );
    check!(
        report.contains("proof digest mismatch"),
        "the report names the digest mismatch: {report}"
    );
    Ok(())
}
