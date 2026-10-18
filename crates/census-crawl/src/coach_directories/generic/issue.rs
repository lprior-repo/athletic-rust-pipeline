#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Issue {
    Budget(&'static str),
    Malformed(String),
}

impl std::ops::Deref for Issue {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Budget(reason) => reason,
            Self::Malformed(reason) => reason,
        }
    }
}
