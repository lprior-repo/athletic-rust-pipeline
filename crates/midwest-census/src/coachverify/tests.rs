//! Rule tests for the coach-fragment gate: the matching and windowing rules are pure functions, so
//! every claim the gate makes about a page is pinned here without a network.

use super::verdict::RowEvidence;
use super::*;

fn row(role: &str, coach: &str, coach_email: &str, ad_name: &str, ad_email: &str) -> FragmentRow {
    FragmentRow {
        school: "Mosinee High School".to_string(),
        city: "Mosinee".to_string(),
        state: "WI".to_string(),
        sport: "Track & Field".to_string(),
        role: role.to_string(),
        coach_name: coach.to_string(),
        public_professional_email: coach_email.to_string(),
        ad_name: ad_name.to_string(),
        ad_email: ad_email.to_string(),
        source_urls: vec!["https://example.org/staff".to_string()],
        last_observed: "2026-09-21".to_string(),
    }
}

#[test]
fn flatten_collapses_every_whitespace_run() {
    assert_eq!(flatten("a\n\nb\tc\r\n  d"), "a b c d");
    assert_eq!(flatten("  leading"), " leading");
}

#[test]
fn value_in_matches_a_value_broken_across_lines() {
    let row = row("Head Coach", "Dana Reid", "", "", "");
    let page = "Staff Directory\nDana\n   Reid\nMosinee High School";
    assert!(!value_in(page, &row));
    assert!(value_in(&flatten(page), &row));
}

#[test]
fn value_in_ignores_empty_cells() {
    let row = row("Head Coach", "", "", "", "");
    assert!(!value_in(&flatten("anything at all"), &row));
}

#[test]
fn role_near_accepts_a_title_immediately_before_the_value() {
    let row = row("Head Coach", "Dana Reid", "", "", "");
    let page = flatten("Dana Reid, Head Coach — Mosinee High School");
    assert!(role_near(&page, &row));
    assert!(!role_contradicted(&page, &row));
}

#[test]
fn role_near_rejects_a_role_outside_its_window() {
    let row = row("Head Coach", "Dana Reid", "", "", "");
    // "Coach" sits more than 200 bytes before the value: outside the window that opens 200 before.
    let filler = "x".repeat(ROLE_BEFORE + 1);
    let page = flatten(&format!("Coach {filler} Dana Reid"));
    assert!(!role_near(&page, &row));
}

#[test]
fn role_near_window_never_splits_a_multibyte_character() {
    let row = row("Head Coach", "Dana Reid", "", "", "");
    // ~300 bytes of two-byte characters before the title, so the window's start lands mid-character
    // and has to snap forward to a boundary instead of slicing a UTF-8 sequence in half.
    let filler = "é".repeat(ROLE_BEFORE / 2 + 50);
    let page = flatten(&format!("{filler}Coach Dana Reid"));
    assert!(role_near(&page, &row));
}

#[test]
fn an_athletic_director_row_is_not_corroborated_by_a_band_director() {
    let row = row("Athletic Director", "", "", "Robin Vale", "");
    // The band director is named far more than 200 bytes before the value — outside the window that
    // opens before it — while the value's own title row says otherwise.
    let filler = "x".repeat(ROLE_BEFORE + 20);
    let page = flatten(&format!(
        "Band Director: Lee Park. {filler} Office staff: Robin Vale, Administrative Assistant."
    ));
    assert!(!role_near(&page, &row));
    assert!(role_contradicted(&page, &row));
    assert_eq!(
        RowEvidence {
            found: true,
            role_near: false,
            contradicted: true,
            body: true,
            ..Default::default()
        }
        .verdict(),
        Verdict::RoleContradicted
    );
}

#[test]
fn an_athletic_director_row_is_corroborated_by_a_title_column() {
    let row = row("Athletic Director", "", "", "Robin Vale", "");
    let page = flatten("Robin Vale</td><td>Athletic Director</td>");
    assert!(role_near(&page, &row));
    assert!(!role_contradicted(&page, &row));
}

#[test]
fn a_coach_row_contradicted_by_an_ad_title_column_is_dropped() {
    let row = row("Head Track Coach", "Dana Reid", "", "", "");
    // A title column that states the AD role and never says "coach": nothing corroborates the coach
    // label, the neighbouring title contradicts it, and the row is dropped rather than relabelled.
    let page = flatten(r#"{"Name":"Dana Reid","Title":"Athletic Director"}"#);
    assert!(value_in(&page, &row));
    assert!(!role_near(&page, &row));
    assert!(role_contradicted(&page, &row));
    assert_eq!(
        RowEvidence {
            found: true,
            role_near: false,
            contradicted: true,
            body: true,
            ..Default::default()
        }
        .verdict(),
        Verdict::RoleContradicted
    );
}

#[test]
fn a_coach_row_with_a_coach_title_ships() {
    let row = row("Head Track Coach", "Dana Reid", "", "", "");
    let page = flatten(r#"{"Name":"Dana Reid","Title":"Head Coach, Track & Field"}"#);
    assert!(value_in(&page, &row));
    assert!(role_near(&page, &row));
    assert!(!role_contradicted(&page, &row));
}

#[test]
fn the_contradiction_window_is_tighter_than_the_role_window() {
    let row = row("Head Coach", "Dana Reid", "", "", "");
    // "principal" 150 bytes before the value: outside the 100-byte contradiction window, inside the
    // 200-byte role window would not matter — there is no coach word either.
    let filler = "x".repeat(CONTRA_BEFORE + 50);
    let page = flatten(&format!("principal {filler} Dana Reid"));
    assert!(!role_contradicted(&page, &row));
    let close = flatten("principal Dana Reid");
    assert!(role_contradicted(&close, &row));
}

#[test]
fn a_role_without_vocabulary_needs_no_corroboration() {
    let row = row("Athletics Secretary", "Dana Reid", "", "", "");
    let page = flatten("Dana Reid");
    assert!(role_near(&page, &row));
    assert!(!role_contradicted(&page, &row));
}

#[test]
fn verdicts_follow_the_documented_precedence() {
    let found_role = RowEvidence {
        found: true,
        role_near: true,
        body: true,
        ..Default::default()
    };
    assert_eq!(found_role.verdict(), Verdict::Ok);

    let found_only = RowEvidence {
        found: true,
        body: true,
        ..Default::default()
    };
    assert_eq!(found_only.verdict(), Verdict::OkRoleContext);

    let nothing = RowEvidence::default();
    assert_eq!(nothing.verdict(), Verdict::Empty);

    let blocked = RowEvidence {
        robots: true,
        ..Default::default()
    };
    assert_eq!(blocked.verdict(), Verdict::RobotsBlocked);

    let failed = RowEvidence {
        failed: true,
        ..Default::default()
    };
    assert_eq!(failed.verdict(), Verdict::FetchFailed);

    let shell = RowEvidence {
        body: true,
        script: true,
        ..Default::default()
    };
    assert_eq!(shell.verdict(), Verdict::RenderRequired);

    let real = RowEvidence {
        body: true,
        ..Default::default()
    };
    assert_eq!(real.verdict(), Verdict::Mismatch);
}

#[test]
fn only_ok_and_ok_role_context_ship() {
    assert!(Verdict::Ok.shipped());
    assert!(Verdict::OkRoleContext.shipped());
    for verdict in Verdict::ALL.iter().filter(|v| !v.shipped()) {
        assert_ne!(*verdict, Verdict::Ok);
        assert_ne!(*verdict, Verdict::OkRoleContext);
    }
}

#[test]
fn nsaa_school_drops_the_trailing_school_word() {
    assert_eq!(nsaa_school("Omaha Central High School"), "Omaha Central");
    assert_eq!(nsaa_school("Crete HS"), "Crete");
    assert_eq!(nsaa_school("Lincoln High"), "Lincoln");
    assert_eq!(nsaa_school("Wayne School"), "Wayne");
    assert_eq!(nsaa_school("Elkhorn"), "Elkhorn");
    // Only one suffix is stripped: "North Platte High School" -> "North Platte", never "North".
    assert_eq!(nsaa_school("North Platte High School"), "North Platte");
}

#[test]
fn identity_is_whitespace_and_case_insensitive() {
    let a = row("Head Coach", "Dana  Reid", "", "", "");
    let b = row("head coach", "dana reid", "", "", "");
    assert_eq!(a.identity(), b.identity());
    let mut c = b.clone();
    c.school = "Other School".to_string();
    assert_ne!(a.identity(), c.identity());
}

#[test]
fn body_text_falls_back_to_lossy_decoding() {
    let latin1 = b"Ren\xe9 Dupont, Head Coach";
    let text = body_text(latin1, None);
    assert!(text.contains("Head Coach"));
    assert!(text.contains("Ren"));
}

#[test]
fn read_fragment_takes_eleven_columns_and_ignores_a_verdict_cell() {
    let path =
        std::env::temp_dir().join(format!("coachverify-test-{}-read.csv", std::process::id()));
    let body = "school,city,state,sport,role,coach_name,public_professional_email,ad_name,ad_email,source_url,last_observed,verify\r\n\
Mosinee High School,Mosinee,WI,Track & Field,Head Coach,Dana Reid,dana@mosinee.k12.wi.us,,,https://example.org/staff  https://example.org/tf,2026-09-21,ok\r\n";
    std::fs::write(&path, body).expect("scratch fragment is writable");
    let rows = read_fragment(&path).expect("scratch fragment parses");
    let _ = std::fs::remove_file(&path);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].coach_name, "Dana Reid");
    assert_eq!(rows[0].last_observed, "2026-09-21");
    assert_eq!(rows[0].source_urls.len(), 2);
}

#[test]
fn write_fragment_emits_only_shipped_rows_in_the_input_shape() {
    let dir = std::env::temp_dir().join(format!("coachverify-test-{}-write", std::process::id()));
    let path = dir.join("WI.csv");
    let kept = FragmentRow {
        school: "Mosinee High School".to_string(),
        ..row("Head Coach", "Dana Reid", "", "", "")
    };
    let dropped = FragmentRow {
        school: "Wausau East".to_string(),
        ..row("Head Coach", "Kim Alvarez", "", "", "")
    };
    let outcomes = vec![
        RowOutcome {
            row: kept,
            verdict: Verdict::Ok,
        },
        RowOutcome {
            row: dropped,
            verdict: Verdict::RoleContradicted,
        },
    ];
    write_fragment(&path, &outcomes).expect("verified fragment is writable");
    let written = std::fs::read_to_string(&path).expect("verified fragment is readable");
    let _ = std::fs::remove_dir_all(&dir);
    // A fragment `merge-coaches` can read: eleven columns, shipped rows only, the verdict in `--csv`.
    let mut lines = written.lines();
    assert_eq!(lines.next(), Some(FRAGMENT_COLUMNS.join(",").as_str()));
    assert!(written.contains("Dana Reid"));
    assert!(!written.contains("Kim Alvarez"));
    assert_eq!(lines.count(), 1, "one shipped row");
}
