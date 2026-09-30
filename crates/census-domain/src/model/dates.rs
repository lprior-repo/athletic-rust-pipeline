pub(super) fn valid_date(date: &str) -> bool {
    chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").is_ok() || valid_year(date)
}

fn valid_year(date: &str) -> bool {
    if date.len() != 4 || !date.bytes().all(|byte| byte.is_ascii_digit()) {
        return false;
    }
    matches!(date.parse::<i16>(), Ok(year) if (1900..=2100).contains(&year))
}

#[cfg(test)]
mod tests {
    use super::valid_date;

    #[test]
    fn published_iso_dates_and_years_are_accepted() {
        for date in [
            "2026-09-27",
            "1900-01-01",
            "2100-12-31",
            "1900",
            "2023",
            "2100",
        ] {
            assert!(valid_date(date), "{date:?} is a published date or year");
        }
    }

    #[test]
    fn malformed_and_out_of_range_dates_are_rejected() {
        for date in [
            "",
            "1899",
            "2101",
            "qnqtrf",
            "2026-13-01",
            "2026-02-30",
            "2026-09",
        ] {
            assert!(!valid_date(date), "{date:?} must be rejected");
        }
    }
}
