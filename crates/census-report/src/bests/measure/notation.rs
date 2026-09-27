
fn split_feet_inches(feet_mark: &str) -> Option<(&str, &str)> {
    let trimmed = feet_mark.trim();
    let (feet, inches) = match trimmed.split_once('\'') {
        Some((feet, rest)) => (feet, rest),
        None => trimmed.split_once('-').unwrap_or((trimmed, "")),
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
    let whole: i64 = whole.parse().ok()?;
    let frac = match frac.len() {
        0 => 0,
        1 => frac.parse::<i64>().ok()?.checked_mul(10)?,
        2 => frac.parse::<i64>().ok()?,
        _ => return None,
    };
    if whole < 0 || frac < 0 {
        return None;
    }
    whole.checked_mul(100)?.checked_add(frac)
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
