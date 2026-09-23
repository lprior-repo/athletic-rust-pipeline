//! The `UsJurisdiction` impl block: scope guard, code/name tables, parse, and Display/FromStr.
//!
//! Rust does not allow split inherent impls, so this file owns the entire impl despite the enum
//! declaration sitting in [`super::table`].

use std::fmt;

use crate::error::DomainError;
use std::str::FromStr;

use super::table::UsJurisdiction;

impl UsJurisdiction {
    /// Accept a jurisdiction only if a census run may cover it (ADR-009), returning it so a caller
    /// can map a slice through this and collect the survivors.
    pub fn require_census_scope(self) -> Result<Self, super::scope::OutsideCensusScope> {
        if self.is_in_census_scope() {
            Ok(self)
        } else {
            Err(super::scope::OutsideCensusScope(self))
        }
    }

    /// Whether a census run may cover this jurisdiction: the rule [`Self::CENSUS_SCOPE`] encodes,
    /// in one place so no caller has to re-derive it.
    pub const fn is_in_census_scope(self) -> bool {
        !matches!(self, Self::Alaska | Self::Hawaii)
    }

    /// The USPS two-letter code — the jurisdiction's stable key form.
    pub const fn code(self) -> &'static str {
        match self {
            Self::Alabama => "AL",
            Self::Alaska => "AK",
            Self::Arizona => "AZ",
            Self::Arkansas => "AR",
            Self::California => "CA",
            Self::Colorado => "CO",
            Self::Connecticut => "CT",
            Self::Delaware => "DE",
            Self::Florida => "FL",
            Self::Georgia => "GA",
            Self::Hawaii => "HI",
            Self::Idaho => "ID",
            Self::Illinois => "IL",
            Self::Indiana => "IN",
            Self::Iowa => "IA",
            Self::Kansas => "KS",
            Self::Kentucky => "KY",
            Self::Louisiana => "LA",
            Self::Maine => "ME",
            Self::Maryland => "MD",
            Self::Massachusetts => "MA",
            Self::Michigan => "MI",
            Self::Minnesota => "MN",
            Self::Mississippi => "MS",
            Self::Missouri => "MO",
            Self::Montana => "MT",
            Self::Nebraska => "NE",
            Self::Nevada => "NV",
            Self::NewHampshire => "NH",
            Self::NewJersey => "NJ",
            Self::NewMexico => "NM",
            Self::NewYork => "NY",
            Self::NorthCarolina => "NC",
            Self::NorthDakota => "ND",
            Self::Ohio => "OH",
            Self::Oklahoma => "OK",
            Self::Oregon => "OR",
            Self::Pennsylvania => "PA",
            Self::RhodeIsland => "RI",
            Self::SouthCarolina => "SC",
            Self::SouthDakota => "SD",
            Self::Tennessee => "TN",
            Self::Texas => "TX",
            Self::Utah => "UT",
            Self::Vermont => "VT",
            Self::Virginia => "VA",
            Self::Washington => "WA",
            Self::WestVirginia => "WV",
            Self::Wisconsin => "WI",
            Self::Wyoming => "WY",
            Self::DistrictOfColumbia => "DC",
        }
    }

    /// The jurisdiction's English name as reports and workbooks spell it.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Alabama => "Alabama",
            Self::Alaska => "Alaska",
            Self::Arizona => "Arizona",
            Self::Arkansas => "Arkansas",
            Self::California => "California",
            Self::Colorado => "Colorado",
            Self::Connecticut => "Connecticut",
            Self::Delaware => "Delaware",
            Self::Florida => "Florida",
            Self::Georgia => "Georgia",
            Self::Hawaii => "Hawaii",
            Self::Idaho => "Idaho",
            Self::Illinois => "Illinois",
            Self::Indiana => "Indiana",
            Self::Iowa => "Iowa",
            Self::Kansas => "Kansas",
            Self::Kentucky => "Kentucky",
            Self::Louisiana => "Louisiana",
            Self::Maine => "Maine",
            Self::Maryland => "Maryland",
            Self::Massachusetts => "Massachusetts",
            Self::Michigan => "Michigan",
            Self::Minnesota => "Minnesota",
            Self::Mississippi => "Mississippi",
            Self::Missouri => "Missouri",
            Self::Montana => "Montana",
            Self::Nebraska => "Nebraska",
            Self::Nevada => "Nevada",
            Self::NewHampshire => "New Hampshire",
            Self::NewJersey => "New Jersey",
            Self::NewMexico => "New Mexico",
            Self::NewYork => "New York",
            Self::NorthCarolina => "North Carolina",
            Self::NorthDakota => "North Dakota",
            Self::Ohio => "Ohio",
            Self::Oklahoma => "Oklahoma",
            Self::Oregon => "Oregon",
            Self::Pennsylvania => "Pennsylvania",
            Self::RhodeIsland => "Rhode Island",
            Self::SouthCarolina => "South Carolina",
            Self::SouthDakota => "South Dakota",
            Self::Tennessee => "Tennessee",
            Self::Texas => "Texas",
            Self::Utah => "Utah",
            Self::Vermont => "Vermont",
            Self::Virginia => "Virginia",
            Self::Washington => "Washington",
            Self::WestVirginia => "West Virginia",
            Self::Wisconsin => "Wisconsin",
            Self::Wyoming => "Wyoming",
            Self::DistrictOfColumbia => "District of Columbia",
        }
    }

    /// Parse a two-letter USPS code, ignoring surrounding whitespace and ASCII case.
    ///
    /// Use this where the source publishes a code (`"WI"`); [`Self::parse`] additionally accepts a
    /// spelled-out name for directory pages that never print codes.
    pub fn from_code(code: &str) -> Option<Self> {
        let trimmed = code.trim();
        if trimmed.is_empty() {
            return None;
        }
        Self::ALL
            .iter()
            .copied()
            .find(|jurisdiction| jurisdiction.code().eq_ignore_ascii_case(trimmed))
    }

    /// Parse either a USPS code or a spelled-out jurisdiction name, case-insensitively.
    ///
    /// Returns `None` for anything else — including territories — so a caller that wants to refuse
    /// an unknown jurisdiction can, and a caller that wants to *record* the refusal has the raw
    /// string it was given.
    pub fn parse(raw: &str) -> Option<Self> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return None;
        }
        Self::ALL.iter().copied().find(|jurisdiction| {
            jurisdiction.code().eq_ignore_ascii_case(trimmed)
                || jurisdiction.name().eq_ignore_ascii_case(trimmed)
        })
    }
}

impl fmt::Display for UsJurisdiction {
    /// Displays the USPS code, which is the form every key and workflow identity uses.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl FromStr for UsJurisdiction {
    type Err = DomainError;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        Self::parse(raw).ok_or(DomainError::Unsupported {
            field: "jurisdiction",
        })
    }
}
