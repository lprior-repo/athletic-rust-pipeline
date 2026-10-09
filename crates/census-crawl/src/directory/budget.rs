#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum ReadBudget {
    #[default]
    Page,
    NationalFile,
}

impl ReadBudget {
    pub(super) const fn input_bytes(self) -> usize {
        match self {
            Self::Page => 8 * 1024 * 1024,
            Self::NationalFile => 128 * 1024 * 1024,
        }
    }

    pub(super) const fn rows(self) -> usize {
        match self {
            Self::Page => 20_000,
            Self::NationalFile => 200_000,
        }
    }

    pub(super) const fn source_rows(self) -> usize {
        match self {
            Self::Page => 65_536,
            Self::NationalFile => 200_000,
        }
    }

    pub(super) const fn retained_bytes(self) -> usize {
        match self {
            Self::Page => 8 * 1024 * 1024,
            Self::NationalFile => 512 * 1024 * 1024,
        }
    }
}
