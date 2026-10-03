#[derive(Debug)]
pub(crate) struct CheckFailure(pub(crate) String);

impl std::fmt::Display for CheckFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for CheckFailure {}

macro_rules! check {
    (eq; $left:expr, $right:expr $(,)?) => {
        match (&$left, &$right) {
            (left_value, right_value) => {
                if !(*left_value == *right_value) {
                    return Err($crate::fallible_checks::CheckFailure(format!("left={left_value:?} right={right_value:?}")).into());
                }
            }
        }
    };
    (eq; $left:expr, $right:expr, $($message:tt)+) => {
        match (&$left, &$right) {
            (left_value, right_value) => {
                if !(*left_value == *right_value) {
                    return Err($crate::fallible_checks::CheckFailure(format!(
                        "{}; left={left_value:?} right={right_value:?}",
                        format_args!($($message)+)
                    )).into());
                }
            }
        }
    };
    (ne; $left:expr, $right:expr $(,)?) => {
        match (&$left, &$right) {
            (left_value, right_value) => {
                if *left_value == *right_value {
                    return Err($crate::fallible_checks::CheckFailure(format!("unexpected equality: left={left_value:?} right={right_value:?}")).into());
                }
            }
        }
    };
    (ne; $left:expr, $right:expr, $($message:tt)+) => {
        match (&$left, &$right) {
            (left_value, right_value) => {
                if *left_value == *right_value {
                    return Err($crate::fallible_checks::CheckFailure(format!(
                        "{}; unexpected equality: left={left_value:?} right={right_value:?}",
                        format_args!($($message)+)
                    )).into());
                }
            }
        }
    };
    ($condition:expr $(,)?) => {
        match $condition {
            true => (),
            false => return Err($crate::fallible_checks::CheckFailure(format!("condition failed: {}", stringify!($condition))).into()),
        }
    };
    ($condition:expr, $($message:tt)+) => {
        match $condition {
            true => (),
            false => return Err($crate::fallible_checks::CheckFailure(format!($($message)+)).into()),
        }
    };
}
