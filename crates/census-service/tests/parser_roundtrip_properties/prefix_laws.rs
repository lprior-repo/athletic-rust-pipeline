//! Prefix stability: tails append, truncation shortens, neither rewrites what was parsed.

use super::*;

proptest! {
    #![proptest_config(seam_config())]

    #[test]
    fn a_trailing_text_report_never_disturbs_the_parsed_prefix(blocks in tail_blocks()) {
        let body = format!("{DASH_TEXT}\n{}", blocks.join("\n"));
        let extended = parse(&body, ArtifactFormat::HytekText, ARCHIVE_YEAR)
            .expect("the header of the report is untouched");
        prefix_survives(
            &parse(DASH_TEXT, ArtifactFormat::HytekText, ARCHIVE_YEAR).expect("the fixture parses"),
            &extended,
        )?;
    }

    #[test]
    fn trailing_paragraphs_never_disturb_the_parsed_prefix(blocks in tail_blocks()) {
        let mut body = SECTIONS_HTML.to_string();
        for block in &blocks {
            body.push_str(&format!("<p>{block}</p>"));
        }
        let extended = parse(&body, ArtifactFormat::HytekHtml, ARCHIVE_YEAR)
            .expect("the report paragraphs are untouched");
        prefix_survives(
            &parse(SECTIONS_HTML, ArtifactFormat::HytekHtml, ARCHIVE_YEAR)
                .expect("the fixture parses"),
            &extended,
        )?;
    }

    #[test]
    fn trailing_markup_never_disturbs_the_raceday_prefix(blocks in tail_blocks()) {
        let noise = blocks.join(" ");
        let body = format!("{RACEDAY_HTML}<div class=\"footer\">{noise}</div><!-- {noise} -->");
        let extended = parse(&body, ArtifactFormat::RaceDay, ARCHIVE_YEAR)
            .expect("the race title and its table are untouched");
        prop_assert_eq!(
            parse(RACEDAY_HTML, ArtifactFormat::RaceDay, ARCHIVE_YEAR).expect("the fixture parses"),
            extended
        );
    }

    #[test]
    fn a_line_prefix_parses_to_a_prefix_of_the_full_parse(
        fixture in 0usize..FIXTURES.len(),
        keep in 1usize..200,
    ) {
        let (name, body, format) = FIXTURES[fixture];
        let lines: Vec<&str> = body.lines().collect();
        let keep = keep.min(lines.len());
        let truncated = lines[..keep].join("\n");
        let full = parse(body, format, ARCHIVE_YEAR)
            .unwrap_or_else(|| panic!("{name} parses in full"));
        match parse(&truncated, format, ARCHIVE_YEAR) {
            None => prop_assert!(full.rows_parsed > 0),
            Some(partial) => prefix_survives(&partial, &full)?,
        }
    }

    #[test]
    fn raceday_publishes_the_archive_year_as_the_meet_date(year in 1990i16..=2035) {
        let meet = parse(RACEDAY_HTML, ArtifactFormat::RaceDay, year)
            .expect("the fixture parses for any archive year");
        prop_assert_eq!(&meet.date, &format!("{year:04}"));

        let shifted = parse(RACEDAY_HTML, ArtifactFormat::RaceDay, year.saturating_add(1))
            .expect("the fixture parses for any archive year");
        prop_assert_eq!(shifted, ParsedMeet { date: format!("{:04}", year.saturating_add(1)), ..meet });
    }
}
