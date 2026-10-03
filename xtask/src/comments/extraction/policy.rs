pub(super) fn forbidden_method(name: &str) -> bool {
    matches!(
        name,
        "unwrap"
            | "expect"
            | "unwrap_err"
            | "expect_err"
            | "unwrap_unchecked"
            | "unwrap_or"
            | "unwrap_or_else"
            | "unwrap_or_default"
    )
}
