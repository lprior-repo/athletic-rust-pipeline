//! US jurisdictions: the fifty states and the District of Columbia.
//!
//! The census is national, so a jurisdiction is a *validated domain value* rather than a free-form
//! state string. Every adapter, workflow identity and Fjall key that needs a state carries a
//! [`UsJurisdiction`]; the only place a raw string is acceptable is the parse boundary itself.
//!
//! Territories (PR, GU, VI, AS, MP) and freely associated states are deliberately absent: parsing
//! `"PR"` fails instead of silently widening the census, so bringing one in later is an explicit
//! domain change — a new variant with its code and name in [`UsJurisdiction::ALL`] — and cannot
//! happen by accident of string handling. The cohort is high-school TF/XC, and the territorial
//! associations publish under different systems; admitting them silently would corrupt coverage
//! denominators, which is the one number the census is judged on.
//!
//! The two lookup functions are data tables, not logic: a 51-arm constant match is the fastest
//! correct form for a value that ends up in every store key, and splitting it for line-count
//! aesthetics would only obscure which variant maps to which code.

use crate::error::DomainError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

/// One of the fifty US states or the District of Columbia.
///
/// Variants are declared in the order [`UsJurisdiction::ALL`] lists them (alphabetical by name,
/// with the District of Columbia last), so `Ord` orders jurisdictions the way a report reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum UsJurisdiction {
    Alabama,
    Alaska,
    Arizona,
    Arkansas,
    California,
    Colorado,
    Connecticut,
    Delaware,
    Florida,
    Georgia,
    Hawaii,
    Idaho,
    Illinois,
    Indiana,
    Iowa,
    Kansas,
    Kentucky,
    Louisiana,
    Maine,
    Maryland,
    Massachusetts,
    Michigan,
    Minnesota,
    Mississippi,
    Missouri,
    Montana,
    Nebraska,
    Nevada,
    NewHampshire,
    NewJersey,
    NewMexico,
    NewYork,
    NorthCarolina,
    NorthDakota,
    Ohio,
    Oklahoma,
    Oregon,
    Pennsylvania,
    RhodeIsland,
    SouthCarolina,
    SouthDakota,
    Tennessee,
    Texas,
    Utah,
    Vermont,
    Virginia,
    Washington,
    WestVirginia,
    Wisconsin,
    Wyoming,
    DistrictOfColumbia,
}

impl UsJurisdiction {
    /// Every jurisdiction the census covers, in declaration order.
    pub const ALL: [Self; 51] = [
        Self::Alabama,
        Self::Alaska,
        Self::Arizona,
        Self::Arkansas,
        Self::California,
        Self::Colorado,
        Self::Connecticut,
        Self::Delaware,
        Self::Florida,
        Self::Georgia,
        Self::Hawaii,
        Self::Idaho,
        Self::Illinois,
        Self::Indiana,
        Self::Iowa,
        Self::Kansas,
        Self::Kentucky,
        Self::Louisiana,
        Self::Maine,
        Self::Maryland,
        Self::Massachusetts,
        Self::Michigan,
        Self::Minnesota,
        Self::Mississippi,
        Self::Missouri,
        Self::Montana,
        Self::Nebraska,
        Self::Nevada,
        Self::NewHampshire,
        Self::NewJersey,
        Self::NewMexico,
        Self::NewYork,
        Self::NorthCarolina,
        Self::NorthDakota,
        Self::Ohio,
        Self::Oklahoma,
        Self::Oregon,
        Self::Pennsylvania,
        Self::RhodeIsland,
        Self::SouthCarolina,
        Self::SouthDakota,
        Self::Tennessee,
        Self::Texas,
        Self::Utah,
        Self::Vermont,
        Self::Virginia,
        Self::Washington,
        Self::WestVirginia,
        Self::Wisconsin,
        Self::Wyoming,
        Self::DistrictOfColumbia,
    ];

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

impl Serialize for UsJurisdiction {
    /// Serializes as the two-letter code, delegating to the [`fmt::Display`] impl the tests pin.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for UsJurisdiction {
    /// Deserialization is a domain boundary: an unknown or territorial code is a decode error, not
    /// a silently retained string.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Self::parse(&raw).ok_or_else(|| {
            serde::de::Error::custom(format_args!(
                "jurisdiction {raw:?} is not one of the 50 states or the District of Columbia"
            ))
        })
    }
}

#[cfg(test)]
#[path = "jurisdiction_tests.rs"]
mod tests;
