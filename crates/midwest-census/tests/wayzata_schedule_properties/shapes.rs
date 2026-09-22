//! The published shape of a schedule row.
//!
//! A row is the provider's identity for one competition day: the season it was read for, a month and
//! day from the page, a label for the event and its venue, and the provider's own key when it printed
//! one. Every part of that has a consumer — the date orders the refresh, the venue places the meet in
//! a state, the slug keys later requests — so each is asserted on the captures and on woven pages.

use super::{rows, PAGES, SEASON};
use proptest::prelude::*;

/// A row's date is the season it was read for plus a month and a day the page stated, zero-padded,
/// with nothing else in it.
fn date_is_well_formed(date: &str, year: i16) -> bool {
    let prefix = format!("{year:04}-");
    let Some(rest) = date.strip_prefix(&prefix) else {
        return false;
    };
    let mut parts = rest.split('-');
    let (Some(month), Some(day), None) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    let (Ok(month_number), Ok(day_number)) = (month.parse::<u8>(), day.parse::<u8>()) else {
        return false;
    };
    month.len() == 2
        && day.len() == 2
        && (1..=12).contains(&month_number)
        && (1..=31).contains(&day_number)
}

/// A label the reader published is text: stripped of tags, trimmed, and with its internal whitespace
/// collapsed, so a venue can be looked up in the state table verbatim.
fn is_text(label: &str) -> bool {
    !label.is_empty()
        && label == label.trim()
        && !label.contains("  ")
        && !label.contains('<')
        && !label.contains('>')
        && !label.contains('\n')
        && !label.contains('\t')
}

#[test]
fn every_row_carries_a_well_formed_date_of_the_season_it_was_read_for() {
    for (name, body) in PAGES {
        for season in [SEASON, 2019] {
            let parsed = rows(body, season).unwrap_or_else(|error| panic!("{name}: {error:?}"));
            assert!(!parsed.is_empty(), "{name}: the capture publishes rows");
            for row in &parsed {
                assert!(
                    date_is_well_formed(&row.date, season),
                    "{name}: {} is the season {season} plus a published month and day",
                    row.date
                );
            }
        }
    }
}

#[test]
fn every_row_carries_a_label_and_a_venue_and_only_real_keys() {
    for (name, body) in PAGES {
        let parsed = rows(body, SEASON).unwrap_or_else(|error| panic!("{name}: {error:?}"));
        for row in &parsed {
            assert!(is_text(&row.name), "{name}: the name is text: {row:?}");
            assert!(is_text(&row.location), "{name}: the venue is text: {row:?}");
            if let Some(slug) = &row.slug {
                assert!(
                    !slug.is_empty() && !slug.contains(['/', '?', '#', '"']),
                    "{name}: the provider key is one path segment: {row:?}"
                );
            }
            if let Some(label) = &row.aria_label {
                assert!(is_text(label), "{name}: the link label is text: {row:?}");
            }
        }
    }
}

/// The captures publish the provider key on every competition row, and the key is what later requests
/// are built from — a row that loses it is a meet the collector can list but never open.
#[test]
fn the_captures_publish_a_key_on_every_row() {
    for (name, body) in PAGES {
        let parsed = rows(body, SEASON).unwrap_or_else(|error| panic!("{name}: {error:?}"));
        let with_keys = parsed.iter().filter(|row| row.slug.is_some()).count();
        assert_eq!(
            with_keys,
            parsed.len(),
            "{name}: every published row carries its `/links/<slug>` key"
        );
    }
}

/// The same shape laws on a page no capture has: whatever order headings and rows arrive in, a
/// published row is still a date of the season, two text labels and a single-segment key.
fn shape_holds(body: &str) -> Result<(), TestCaseError> {
    let Ok(parsed) = rows(body, SEASON) else {
        return Ok(());
    };
    for row in &parsed {
        prop_assert!(
            date_is_well_formed(&row.date, SEASON),
            "{} is not a date of the season: {:?}",
            row.date,
            row
        );
        prop_assert!(is_text(&row.name), "the name is text: {:?}", row);
        prop_assert!(is_text(&row.location), "the venue is text: {:?}", row);
        if let Some(slug) = &row.slug {
            prop_assert!(
                !slug.is_empty() && !slug.contains(['/', '?', '#', '"']),
                "the provider key is one path segment: {:?}",
                row
            );
        }
    }
    Ok(())
}

proptest! {
    #![proptest_config(super::seam_config())]

    #[test]
    fn woven_pages_publish_rows_of_the_same_shape(body in super::shaped_body()) {
        shape_holds(&body)?;
    }
}
