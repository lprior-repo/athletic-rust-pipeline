#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResultStatus<'a> {
    token: &'a str,
    kind: ResultStatusKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultStatusKind {
    ExplicitTerminal,
    AmbiguousPlaceholder,
}

impl<'a> ResultStatus<'a> {
    pub const fn token(self) -> &'a str {
        self.token
    }

    pub const fn kind(self) -> ResultStatusKind {
        self.kind
    }
}

pub fn parse_result_status(raw: &str) -> Option<ResultStatus<'_>> {
    let token = raw.trim();
    if ["DQ", "DNF", "DNS", "NH", "FOUL", "NM", "NT", "SCR"]
        .iter()
        .any(|status| token.eq_ignore_ascii_case(status))
    {
        return Some(ResultStatus {
            token,
            kind: ResultStatusKind::ExplicitTerminal,
        });
    }
    if ["X", "--"]
        .iter()
        .any(|status| token.eq_ignore_ascii_case(status))
    {
        return Some(ResultStatus {
            token,
            kind: ResultStatusKind::AmbiguousPlaceholder,
        });
    }
    None
}

#[cfg(test)]
#[path = "result_status_tests.rs"]
mod tests;
