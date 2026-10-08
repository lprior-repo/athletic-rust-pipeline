use super::{rows, PAGES, SEASON};

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
fn every_row_carries_a_well_formed_date_of_the_season_it_was_read_for(
) -> Result<(), Box<dyn std::error::Error>> {
    for (name, body) in PAGES {
        for season in [SEASON, 2019] {
            let parsed = rows(body, season)?;
            check!(!parsed.is_empty(), "{name}: the capture publishes rows");
            for row in &parsed {
                check!(
                    date_is_well_formed(&row.date, season),
                    "{name}: {} is the season {season} plus a published month and day",
                    row.date
                );
            }
        }
    }
    Ok(())
}

#[test]
fn every_row_carries_a_label_and_a_venue_and_only_real_keys(
) -> Result<(), Box<dyn std::error::Error>> {
    for (name, body) in PAGES {
        let parsed = rows(body, SEASON)?;
        for row in &parsed {
            check!(is_text(&row.name), "{name}: the name is text: {row:?}");
            check!(is_text(&row.location), "{name}: the venue is text: {row:?}");
            if let Some(slug) = &row.slug {
                check!(
                    !slug.is_empty() && !slug.contains(['/', '?', '#', '"']),
                    "{name}: the provider key is one path segment: {row:?}"
                );
            }
            if let Some(label) = &row.aria_label {
                check!(is_text(label), "{name}: the link label is text: {row:?}");
            }
        }
    }
    Ok(())
}
