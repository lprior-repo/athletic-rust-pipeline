//! The registry's entries from `mshsl` on, in the table's published order — the other half of the
//! positional cut described in `through_milesplit`.

use super::super::policy::{
    fetched, CRAWL_DELAY_TEN_RPS, FETCHER_RPS, SCHOOL_COACH_CONTACT, SCHOOL_COACH_NAMES,
};
use super::super::SourceCapabilities as Caps;
use super::super::{SourceDescriptor, TransportKind};

/// The adapters whose slugs sort from `mshsl` on.
pub(super) const FROM_MSHSL: [SourceDescriptor; 9] = [
    SourceDescriptor {
        slug: "mpa",
        provider: "Maine Principals' Association school directory",
        transport: TransportKind::Html,
        capabilities: SCHOOL_COACH_NAMES,
        admission: fetched("www.mpa.cc", FETCHER_RPS),
    },
    SourceDescriptor {
        slug: "mshsl",
        provider: "Minnesota State High School League (MSHSL)",
        transport: TransportKind::Html,
        capabilities: SCHOOL_COACH_CONTACT,
        admission: fetched("www.mshsl.org", FETCHER_RPS),
    },
    SourceDescriptor {
        slug: "ohsaa",
        provider: "OHSAA myOHSAA officials portal",
        transport: TransportKind::Html,
        capabilities: SCHOOL_COACH_CONTACT,
        admission: fetched("officials.myohsaa.org", FETCHER_RPS),
    },
    SourceDescriptor {
        slug: "plain_names",
        provider: "NDHSAA and NSAA member directories (North Dakota, Nebraska)",
        transport: TransportKind::Html,
        capabilities: SCHOOL_COACH_NAMES,
        admission: fetched("ndhsaa.com", FETCHER_RPS),
    },
    SourceDescriptor {
        slug: "riil",
        provider: "Rhode Island Interscholastic League directory",
        transport: TransportKind::Html,
        capabilities: SCHOOL_COACH_NAMES,
        admission: fetched("riil.org", FETCHER_RPS),
    },
    SourceDescriptor {
        slug: "tfrrs",
        provider: "TFRRS high-school performance lists (state instances)",
        transport: TransportKind::Html,
        capabilities: Caps {
            athlete_discovery: true,
            bulk_results: true,
            grade_evidence: true,
            school_evidence: true,
            ..Caps::NONE
        },
        admission: fetched("tfrrs.org", FETCHER_RPS),
    },
    SourceDescriptor {
        slug: "wiaa",
        provider: "WIAA school directory (Wisconsin)",
        transport: TransportKind::Html,
        capabilities: SCHOOL_COACH_CONTACT,
        admission: fetched("schools.wiaawi.org", FETCHER_RPS),
    },
    SourceDescriptor {
        slug: "wiaa_results",
        provider: "WIAA state result archive",
        transport: TransportKind::Html,
        capabilities: Caps {
            bulk_results: true,
            meet_discovery: true,
            grade_evidence: true,
            ..Caps::NONE
        },
        admission: fetched("www.wiaawi.org", FETCHER_RPS),
    },
    SourceDescriptor {
        slug: "wayzata",
        provider: "Wayzata Results",
        transport: TransportKind::Html,
        capabilities: Caps {
            meet_discovery: true,
            ..Caps::NONE
        },
        admission: fetched("www.wayzataresults.com", CRAWL_DELAY_TEN_RPS),
    },
];
