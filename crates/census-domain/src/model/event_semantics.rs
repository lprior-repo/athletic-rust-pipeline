#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventSemantics {
    Legacy18,
    FlatEvents19,
    CombinedEvents20,
    ResultGroups21,
    ResultGroups22,
}

impl EventSemantics {
    pub const CURRENT: Self = Self::ResultGroups22;

    pub const fn revision(self) -> u32 {
        match self {
            Self::Legacy18 => 18,
            Self::FlatEvents19 => 19,
            Self::CombinedEvents20 => 20,
            Self::ResultGroups21 => 21,
            Self::ResultGroups22 => 22,
        }
    }

    pub const fn from_revision(revision: u32) -> Option<Self> {
        match revision {
            18 => Some(Self::Legacy18),
            19 => Some(Self::FlatEvents19),
            20 => Some(Self::CombinedEvents20),
            21 => Some(Self::ResultGroups21),
            22 => Some(Self::ResultGroups22),
            _ => None,
        }
    }

    pub const fn requires_completed_combined(self) -> bool {
        match self {
            Self::Legacy18 | Self::FlatEvents19 => false,
            Self::CombinedEvents20 | Self::ResultGroups21 | Self::ResultGroups22 => true,
        }
    }

    pub(super) const fn supports_flat_extensions(self) -> bool {
        match self {
            Self::Legacy18 => false,
            Self::FlatEvents19
            | Self::CombinedEvents20
            | Self::ResultGroups21
            | Self::ResultGroups22 => true,
        }
    }

    pub const fn uses_result_groups(self) -> bool {
        matches!(self, Self::ResultGroups21 | Self::ResultGroups22)
    }

    pub(super) const fn rejects_relay_split_labels(self) -> bool {
        matches!(self, Self::ResultGroups22)
    }

    pub(super) fn admits_flat_extension(self, kind: &super::EventKind) -> bool {
        self.supports_flat_extensions()
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
