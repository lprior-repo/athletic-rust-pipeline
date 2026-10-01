use census_domain::UsJurisdiction;
use std::collections::BTreeSet;

pub const ASSOCIATIONS: [(UsJurisdiction, &str); 51] = [
    (UsJurisdiction::Alaska, "ASAA"),
    (UsJurisdiction::Alabama, "AHSAA"),
    (UsJurisdiction::Arkansas, "ArkAA"),
    (UsJurisdiction::Arizona, "AIA"),
    (UsJurisdiction::California, "CIF"),
    (UsJurisdiction::Colorado, "CHSAA"),
    (UsJurisdiction::Connecticut, "CIAC"),
    (UsJurisdiction::DistrictOfColumbia, "DCSAA"),
    (UsJurisdiction::Delaware, "DIAA"),
    (UsJurisdiction::Florida, "FHSAA"),
    (UsJurisdiction::Georgia, "GHSA"),
    (UsJurisdiction::Hawaii, "HHSAA"),
    (UsJurisdiction::Iowa, "IoHSAA"),
    (UsJurisdiction::Idaho, "IdHSAA"),
    (UsJurisdiction::Illinois, "IHSA"),
    (UsJurisdiction::Indiana, "InHSAA"),
    (UsJurisdiction::Kansas, "KSHSAA"),
    (UsJurisdiction::Kentucky, "KHSAA"),
    (UsJurisdiction::Louisiana, "LHSAA"),
    (UsJurisdiction::Massachusetts, "MIAA"),
    (UsJurisdiction::Maryland, "MPSSAA"),
    (UsJurisdiction::Maine, "MPA"),
    (UsJurisdiction::Michigan, "MichHSAA"),
    (UsJurisdiction::Minnesota, "MSHSL"),
    (UsJurisdiction::Missouri, "MSHSAA"),
    (UsJurisdiction::Mississippi, "MHSAA"),
    (UsJurisdiction::Montana, "MHSA"),
    (UsJurisdiction::NorthCarolina, "NCHSAA"),
    (UsJurisdiction::NorthDakota, "NDHSAA"),
    (UsJurisdiction::Nebraska, "NSAA"),
    (UsJurisdiction::NewHampshire, "NHIAA"),
    (UsJurisdiction::NewJersey, "NJSIAA"),
    (UsJurisdiction::NewMexico, "NMAA"),
    (UsJurisdiction::Nevada, "NIAA"),
    (UsJurisdiction::NewYork, "NYSPHSAA"),
    (UsJurisdiction::Ohio, "OHSAA"),
    (UsJurisdiction::Oklahoma, "OSSAA"),
    (UsJurisdiction::Oregon, "OSAA"),
    (UsJurisdiction::Pennsylvania, "PIAA"),
    (UsJurisdiction::RhodeIsland, "RIIL"),
    (UsJurisdiction::SouthCarolina, "SCHSL"),
    (UsJurisdiction::SouthDakota, "SDHSAA"),
    (UsJurisdiction::Tennessee, "TSSAA"),
    (UsJurisdiction::Texas, "UIL"),
    (UsJurisdiction::Utah, "UHSAA"),
    (UsJurisdiction::Virginia, "VHSL"),
    (UsJurisdiction::Vermont, "VPA"),
    (UsJurisdiction::Washington, "WIAA"),
    (UsJurisdiction::Wisconsin, "WisIAA"),
    (UsJurisdiction::WestVirginia, "WVSSAC"),
    (UsJurisdiction::Wyoming, "WHSAA"),
];

pub const VERIFIED: [(UsJurisdiction, usize, usize, &str); 15] = [
    (
        UsJurisdiction::Alabama,
        793,
        1,
        "19.2 staff and 5.0 census rows per sampled school",
    ),
    (
        UsJurisdiction::Arkansas,
        521,
        1,
        "33.5 staff and 6.0 census rows per sampled school",
    ),
    (
        UsJurisdiction::DistrictOfColumbia,
        112,
        1,
        "14.5 staff and 1.5 census rows per sampled school",
    ),
    (
        UsJurisdiction::Delaware,
        320,
        1,
        "14.0 staff and 3.0 census rows per sampled school",
    ),
    (
        UsJurisdiction::Georgia,
        2825,
        3,
        "46.8 staff and 7.5 census rows per sampled school",
    ),
    (
        UsJurisdiction::Idaho,
        461,
        1,
        "21.5 staff and 6.5 census rows per sampled school",
    ),
    (
        UsJurisdiction::Maryland,
        215,
        1,
        "16.5 staff and 1.8 census rows per sampled school",
    ),
    (
        UsJurisdiction::Mississippi,
        1151,
        2,
        "24.8 staff and 4.2 census rows per sampled school",
    ),
    (
        UsJurisdiction::Montana,
        358,
        1,
        "28.8 staff and 11.2 census rows per sampled school",
    ),
    (
        UsJurisdiction::NorthCarolina,
        452,
        1,
        "53.5 staff and 20.5 census rows per sampled school",
    ),
    (
        UsJurisdiction::NorthDakota,
        548,
        1,
        "2.0 staff and 0.8 census rows per sampled school",
    ),
    (
        UsJurisdiction::NewMexico,
        751,
        1,
        "19.5 staff and 3.5 census rows per sampled school",
    ),
    (
        UsJurisdiction::SouthCarolina,
        1278,
        2,
        "24.0 staff and 3.0 census rows per sampled school",
    ),
    (
        UsJurisdiction::Tennessee,
        1675,
        2,
        "11.2 staff and 3.5 census rows per sampled school",
    ),
    (
        UsJurisdiction::Wyoming,
        93,
        1,
        "11.0 staff and 3.2 census rows per sampled school",
    ),
];

pub fn state_key(state: UsJurisdiction) -> String {
    match state {
        UsJurisdiction::DistrictOfColumbia => "DC".to_string(),
        other => other.to_string(),
    }
}

pub fn parse_state_filter(states: &str) -> BTreeSet<String> {
    states
        .split(',')
        .map(|part| part.trim().to_ascii_uppercase())
        .filter(|part| !part.is_empty())
        .collect()
}

pub fn selected_associations(wanted: &BTreeSet<String>) -> Vec<(UsJurisdiction, &'static str)> {
    ASSOCIATIONS
        .iter()
        .copied()
        .filter(|(state, _)| wanted.is_empty() || wanted.contains(&state_key(*state)))
        .collect()
}
