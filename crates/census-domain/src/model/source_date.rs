pub(super) fn valid_source_date(date: &str) -> bool {
    chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").is_ok() || year_only(date)
}

fn year_only(date: &str) -> bool {
    date.len() == 4 && date.bytes().all(|byte| byte.is_ascii_digit())
}
