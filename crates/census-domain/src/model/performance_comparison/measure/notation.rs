fn split_feet_inches(feet_mark: &str) -> Option<(&str, &str)> {
    let trimmed = feet_mark.trim();
    let (feet, inches) = match trimmed.split_once('\'') {
        Some((feet, rest)) => (feet, rest),
        None => trimmed
            .split_once('-')
            .map_or((trimmed, ""), std::convert::identity),
    };
    let inches = inches.trim().trim_end_matches('"').trim();
    Some((feet, inches))
}

fn parse_inches_hundredths(s: &str) -> Option<i64> {
    let s = s.trim();
    if s.is_empty() {
        return Some(0);
    }
    let (whole, frac) = match s.split_once('.') {
        Some((whole, frac)) => (whole.trim(), frac),
        None => (s, ""),
    };
    let frac = frac.trim_end_matches('0');
    if frac.len() > 2 || !frac.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let whole = parse_whole_inches(whole, s)?;
    let frac = parse_fractional_inches(frac)?;
    if whole < 0 || frac < 0 {
        return None;
    }
    whole.checked_mul(100)?.checked_add(frac)
}

fn parse_whole_inches(whole: &str, source: &str) -> Option<i64> {
    if whole.is_empty() {
        if source.get(1..).is_none_or(str::is_empty) {
            return None;
        }
        Some(0)
    } else {
        whole.parse().ok()
    }
}

fn parse_fractional_inches(fraction: &str) -> Option<i64> {
    match fraction.len() {
        0 => Some(0),
        1 => fraction.parse::<i64>().ok()?.checked_mul(10),
        2 => fraction.parse().ok(),
        _ => None,
    }
}

pub(crate) fn parse_field_imperial(feet_mark: &str) -> Option<i64> {
    let (feet_text, inches_text) = split_feet_inches(feet_mark)?;
    let feet: i64 = feet_text.trim().parse().ok()?;
    let hundredths = parse_inches_hundredths(inches_text)?;
    if feet < 0 || hundredths >= 1200 {
        return None;
    }
    let hundredths_total = feet.checked_mul(1200)?.checked_add(hundredths)?;
    hundredths_total.checked_mul(254)
}

#[cfg(test)]
mod tests {
    use super::parse_field_imperial;

    #[test]
    fn a_published_omitted_zero_inch_part_remains_a_measurable_jump() {
        assert_eq!(parse_field_imperial("19-.25"), Some(5_797_550));
        assert_eq!(parse_field_imperial("19-00.25"), Some(5_797_550));
        assert_eq!(parse_field_imperial("19-."), None);
        assert_eq!(parse_field_imperial("19-.25x"), None);
        assert_eq!(parse_field_imperial("19-12.25"), None);
        assert_eq!(parse_field_imperial("19-.005"), None);
        assert_eq!(parse_field_imperial("19-.250"), Some(5_797_550));
    }
}
