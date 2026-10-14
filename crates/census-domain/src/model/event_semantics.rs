#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventSemantics {
    Legacy18,
    FlatEvents19,
}

impl EventSemantics {
    pub const fn revision(self) -> u32 {
        match self {
            Self::Legacy18 => 18,
            Self::FlatEvents19 => 19,
        }
    }

    pub const fn from_revision(revision: u32) -> Option<Self> {
        match revision {
            18 => Some(Self::Legacy18),
            19 => Some(Self::FlatEvents19),
            _ => None,
        }
    }

    pub(super) fn admits_flat_extension(self, kind: &super::EventKind) -> bool {
        self == Self::FlatEvents19
            || !matches!(
                kind,
                super::EventKind::Track300m
                    | super::EventKind::Track500m
                    | super::EventKind::Track600m
                    | super::EventKind::Track2Miles
                    | super::EventKind::Track2000m
                    | super::EventKind::Track10000m
            )
    }
}
