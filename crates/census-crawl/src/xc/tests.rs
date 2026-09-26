use super::CentiSeconds;
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
fn state_blocks_carry_place_grade_and_time() -> anyhow::Result<()> {
    let meet =
        parse(&crate::hytek::lines_from_pdf_text(STATE), source(), 2025).ok_or_else(|| {
            CrawlError::Invariant {
                detail: "the state file has a meet header".to_string(),
            }
        })?;
    anyhow::ensure!(
        meet.name == "WIAA State Cross Country Championships",
        "left={:?} right={:?}",
        &meet.name,
        &"WIAA State Cross Country Championships"
    );
    anyhow::ensure!(
        meet.date == "2025-11-01",
        "left={:?} right={:?}",
        &meet.date,
        &"2025-11-01"
    );
    let event = meet.events.first().ok_or_else(|| CrawlError::Invariant {
        detail: "the state file publishes a race".to_string(),
    })?;
    anyhow::ensure!(
        event.kind == EventKind::CrossCountry,
        "left={:?} right={:?}",
        &event.kind,
        &EventKind::CrossCountry
    );
    anyhow::ensure!(
        event.gender == Gender::Boys,
        "left={:?} right={:?}",
        &event.gender,
        &Gender::Boys
    );
    {
        let left_value = &event.division.as_deref();
        let right_value = &(Some("Division 1"));
        anyhow::ensure!(
            left_value == right_value,
            "left={left_value:?} right={right_value:?}"
        );
    }
    let first = event.rows.first().ok_or_else(|| CrawlError::Invariant {
        detail: "the state file publishes a scorer".to_string(),
    })?;
    anyhow::ensure!(
        first.name == "Cooper Erickson",
        "left={:?} right={:?}",
        &first.name,
        &"Cooper Erickson"
    );
    anyhow::ensure!(
        first.school == "SPASH",
        "the team block names the school — left={:?} right={:?}",
        &first.school,
        &"SPASH"
    );
    {
        let left_value = &first.grade;
        let right_value = &(census_domain::model::Grade::new(12));
        anyhow::ensure!(
            left_value == right_value,
            "left={left_value:?} right={right_value:?}"
        );
    }
    {
        let left_value = &first.place;
        let right_value = &(Some(6));
        anyhow::ensure!(
            left_value == right_value,
            "the place is the overall place — left={left_value:?} right={right_value:?}"
        );
    }
    {
        let left_value = &first.mark;
        let right_value = &(Mark::TimeSeconds(CentiSeconds::new(95020)));
        anyhow::ensure!(
            left_value == right_value,
            "left={left_value:?} right={right_value:?}"
        );
    }
    anyhow::ensure!(event.rows.iter().all(|row| !row.school.is_empty()));
    Ok(())
}

#[test]
fn padded_table_rows_parse_with_and_without_team_points() -> anyhow::Result<()> {
    let meet =
        parse(&crate::hytek::lines_from_pdf_text(TABLE), source(), 2025).ok_or_else(|| {
            CrawlError::Invariant {
                detail: "the sectional file has a meet header".to_string(),
            }
        })?;
    anyhow::ensure!(meet.date == "2025", "this sectional family publishes no date at all, so the archive year is used — left={:?} right={:?}", &meet.date, &"2025");
    let rows: Vec<&ParsedRow> = meet.events.iter().flat_map(|event| &event.rows).collect();
    anyhow::ensure!(
        rows.len() == 4,
        "every table row is read: {rows:?} — left={:?} right={:?}",
        &rows.len(),
        &4
    );
    let first = rows.first().ok_or_else(|| CrawlError::Invariant {
        detail: "the table publishes four rows".to_string(),
    })?;
    let second = rows.get(1).ok_or_else(|| CrawlError::Invariant {
        detail: "the table publishes four rows".to_string(),
    })?;
    anyhow::ensure!(
        first.name == "Wyatt See",
        "left={:?} right={:?}",
        &first.name,
        &"Wyatt See"
    );
    anyhow::ensure!(
        first.school == "Poynette",
        "left={:?} right={:?}",
        &first.school,
        &"Poynette"
    );
    {
        let left_value = &first.grade;
        let right_value = &(census_domain::model::Grade::new(12));
        anyhow::ensure!(
            left_value == right_value,
            "left={left_value:?} right={right_value:?}"
        );
    }
    {
        let left_value = &first.mark;
        let right_value = &(Mark::TimeSeconds(CentiSeconds::new(100410)));
        anyhow::ensure!(
            left_value == right_value,
            "left={left_value:?} right={right_value:?}"
        );
    }
    {
        let left_value = &second.grade;
        let right_value = &(census_domain::model::Grade::new(11));
        anyhow::ensure!(
            left_value == right_value,
            "left={left_value:?} right={right_value:?}"
        );
    }
    anyhow::ensure!(
        second.school == "Ozaukee",
        "left={:?} right={:?}",
        &second.school,
        &"Ozaukee"
    );
    Ok(())
}

#[test]
fn accurace_rows_are_read_through_the_rule_line() -> anyhow::Result<()> {
    let meet =
        parse(&crate::hytek::lines_from_pdf_text(ACCURACE), source(), 2025).ok_or_else(|| {
            CrawlError::Invariant {
                detail: "the AccuRace file has a meet header".to_string(),
            }
        })?;
    anyhow::ensure!(
        meet.date == "2025-10-25",
        "left={:?} right={:?}",
        &meet.date,
        &"2025-10-25"
    );
    anyhow::ensure!(
        meet.name == "WIAA Division 3 Sectional Championship Meet",
        "left={:?} right={:?}",
        &meet.name,
        &"WIAA Division 3 Sectional Championship Meet"
    );
    let event = meet.events.first().ok_or_else(|| CrawlError::Invariant {
        detail: "the AccuRace file publishes a race".to_string(),
    })?;
    anyhow::ensure!(
        event.label == "5000 Meter Run",
        "left={:?} right={:?}",
        &event.label,
        &"5000 Meter Run"
    );
    let first = event.rows.first().ok_or_else(|| CrawlError::Invariant {
        detail: "the AccuRace file publishes a row".to_string(),
    })?;
    anyhow::ensure!(
        first.name == "Jonathan Simon",
        "left={:?} right={:?}",
        &first.name,
        &"Jonathan Simon"
    );
    anyhow::ensure!(
        first.school == "St. Ambrose/Abundant Life",
        "left={:?} right={:?}",
        &first.school,
        &"St. Ambrose/Abundant Life"
    );
    {
        let left_value = &first.grade;
        let right_value = &(census_domain::model::Grade::new(10));
        anyhow::ensure!(
            left_value == right_value,
            "left={left_value:?} right={right_value:?}"
        );
    }
    {
        let left_value = &first.place;
        let right_value = &(Some(1));
        anyhow::ensure!(
            left_value == right_value,
            "left={left_value:?} right={right_value:?}"
        );
    }
    {
        let left_value = &first.mark;
        let right_value = &(Mark::TimeSeconds(CentiSeconds::new(98160)));
        anyhow::ensure!(
            left_value == right_value,
            "left={left_value:?} right={right_value:?}"
        );
    }
    Ok(())
}
