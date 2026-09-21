use super::*;

fn source() -> SourceRef {
    SourceRef::new("wiaa_results", None)
}

/// State meet layout: team score blocks that print each scorer's place, grade and time.
const STATE: &str = r#"
11/1/25, 1:38 PM                                                     WIAA State Cross Country Championships
                                                     WIAA State Cross Country Championships
                                                  The Ridges Golf Course, Wisconsin Rapids, WI
                                                                  11/1/2025
                                                      ========== BOYS TEAM SCORE ==========
                                                                  Division 1
    1.    69 SPASH                             (16:09.3 80:46.1 0:43.4)
  ===============================================
    1      6 Cooper Erickson                 12 15:50.2     5     28 Bennett Story               12   16:33.6
    2      9 Garrett Strong                  10 15:59.9     6   ( 32) Alex Dziak                 11   16:41.3
    3     10 Fisher Carroll                  9    16:03.1   7   ( 58) Donald Voetberg            12   17:06.6
"#;

/// Sectional layout: a padded table whose header carries a grade column.
const TABLE: &str = r#"
WIAA D3 Sectional @ Sheboygan Lutheran
Overall Results
Place   Points   Bib   Name                        School                        Gender   Grade   Time      Pace
Boys Varsity
1       1        574   Wyatt See                   Poynette                      M        12      16:44.1   5:23
2       2        546   Nicholas Schubert           Ozaukee                       M        11      16:55.5   5:26
3       3        621   Eddy Giebler                Sheboygan Area Lutheran       M        10      17:00.7   5:28
4       4        573   Paceler Moll                Poynette                      M        10      17:18.1   5:34
"#;

/// AccuRace layout: columns are stated by a rule line, not by a labelled header.
const ACCURACE: &str = r#"
                           WIAA Division 3 Sectional Championship Meet
                   Baertschi & Keepers Property - Hosted by Albany High School
                                        Albany, Wisconsin
                                        October 25, 2025
                           Results provided by AccuRace Timing Services
                                      www.accuracetiming.com
                                  **** Boys' 5000 Meter Run ****
      Team Team                                                                         Avg   State
Place Pts Place Bib#    Name                  Gr   Team                         Time    Mile Qual
===== ==== ===== ====   ===================== ==   ============================ ======= ===== =====
    1    1 1/7 8223     Jonathan Simon        10   St. Ambrose/Abundant Life    16:21.6 5:16 t
    2    2 1/7 8156     Will Rzentkowski      11   Madison Country Day          16:45.7 5:24 t
    3    3 2/7 8222     David Simon           11   St. Ambrose/Abundant Life    16:55.4 5:27 t
"#;

#[test]
fn state_blocks_carry_place_grade_and_time() -> CrawlResult<()> {
    let meet = parse(
        &crate::sources::hytek::lines_from_pdf_text(STATE),
        source(),
        2025,
    )
    .ok_or_else(|| CrawlError::Invariant {
        detail: "the state file has a meet header".to_string(),
    })?;
    assert_eq!(meet.name, "WIAA State Cross Country Championships");
    assert_eq!(meet.date, "2025-11-01");
    let event = meet.events.first().ok_or_else(|| CrawlError::Invariant {
        detail: "the state file publishes a race".to_string(),
    })?;
    assert_eq!(event.kind, EventKind::CrossCountry);
    assert_eq!(event.gender, Gender::Boys);
    assert_eq!(event.division.as_deref(), Some("Division 1"));
    let first = event.rows.first().ok_or_else(|| CrawlError::Invariant {
        detail: "the state file publishes a scorer".to_string(),
    })?;
    assert_eq!(first.name, "Cooper Erickson");
    assert_eq!(first.school, "SPASH", "the team block names the school");
    assert_eq!(first.grade, census_domain::model::Grade::new(12));
    assert_eq!(first.place, Some(6), "the place is the overall place");
    assert_eq!(first.mark, Mark::TimeSeconds(950.2));
    // A row that prints no school of its own must not be guessed into an athlete.
    assert!(event.rows.iter().all(|row| !row.school.is_empty()));
    Ok(())
}

#[test]
fn padded_table_rows_parse_with_and_without_team_points() -> CrawlResult<()> {
    let meet = parse(
        &crate::sources::hytek::lines_from_pdf_text(TABLE),
        source(),
        2025,
    )
    .ok_or_else(|| CrawlError::Invariant {
        detail: "the sectional file has a meet header".to_string(),
    })?;
    assert_eq!(
        meet.date, "2025",
        "this sectional family publishes no date at all, so the archive year is used"
    );
    let rows: Vec<&ParsedRow> = meet.events.iter().flat_map(|event| &event.rows).collect();
    assert_eq!(rows.len(), 4, "every table row is read: {rows:?}");
    let first = rows.first().ok_or_else(|| CrawlError::Invariant {
        detail: "the table publishes four rows".to_string(),
    })?;
    let second = rows.get(1).ok_or_else(|| CrawlError::Invariant {
        detail: "the table publishes four rows".to_string(),
    })?;
    assert_eq!(first.name, "Wyatt See");
    assert_eq!(first.school, "Poynette");
    assert_eq!(first.grade, census_domain::model::Grade::new(12));
    assert_eq!(first.mark, Mark::TimeSeconds(1004.1));
    assert_eq!(second.grade, census_domain::model::Grade::new(11));
    assert_eq!(second.school, "Ozaukee");
    Ok(())
}

#[test]
fn accurace_rows_are_read_through_the_rule_line() -> CrawlResult<()> {
    let meet = parse(
        &crate::sources::hytek::lines_from_pdf_text(ACCURACE),
        source(),
        2025,
    )
    .ok_or_else(|| CrawlError::Invariant {
        detail: "the AccuRace file has a meet header".to_string(),
    })?;
    assert_eq!(meet.date, "2025-10-25");
    assert_eq!(meet.name, "WIAA Division 3 Sectional Championship Meet");
    let event = meet.events.first().ok_or_else(|| CrawlError::Invariant {
        detail: "the AccuRace file publishes a race".to_string(),
    })?;
    assert_eq!(event.label, "5000 Meter Run");
    let first = event.rows.first().ok_or_else(|| CrawlError::Invariant {
        detail: "the AccuRace file publishes a row".to_string(),
    })?;
    assert_eq!(first.name, "Jonathan Simon");
    assert_eq!(first.school, "St. Ambrose/Abundant Life");
    assert_eq!(first.grade, census_domain::model::Grade::new(10));
    assert_eq!(first.place, Some(1));
    assert_eq!(first.mark, Mark::TimeSeconds(981.6));
    Ok(())
}
