pub(crate) fn invalid_token(raw: &str) -> Option<&str> {
    let token = raw.trim();
    [
        "DQ", "DNF", "DNS", "NH", "FOUL", "NM", "NT", "SCR", "X", "--",
    ]
    .iter()
    .any(|invalid| token.eq_ignore_ascii_case(invalid))
    .then_some(token)
}
