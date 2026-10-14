use super::{SurfaceClass, TimingClass, WindClass};
impl SurfaceClass {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Indoor => "indoor",
            Self::Outdoor => "outdoor",
            Self::CrossCountry => "xc",
            Self::Unresolved => "unresolved",
        }
    }

    pub const fn season_label(self) -> &'static str {
        match self {
            Self::Indoor => "Winter",
            Self::Outdoor => "Spring",
            Self::CrossCountry => "Fall",
            Self::Unresolved => "Unresolved",
        }
    }
}

impl WindClass {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Legal => "legal",
            Self::Assisted => "assisted",
            Self::Unknown => "unknown",
            Self::NotApplicable => "na",
        }
    }
}

impl TimingClass {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Fat => "fat",
            Self::Hand => "hand",
            Self::Unknown => "unknown",
            Self::NonTime => "non_time",
        }
    }
}
