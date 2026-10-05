use super::super::policy::{
    artifact, fetched, CRAWL_DELAY_THIRTY_RPS, FETCHER_RPS, HOME_CAMPUS_RPS, SCHOOL_ADDRESS,
    SCHOOL_COACH_CONTACT,
};
use super::super::{SourceDescriptor, TransportKind};

pub(super) const DIRECTORIES: [SourceDescriptor; 6] = [
    SourceDescriptor {
        slug: "nces",
        provider: "NCES Common Core of Data and Private School Survey school files",
        transport: TransportKind::Csv,
        capabilities: SCHOOL_ADDRESS,
        admission: artifact(),
    },
    SourceDescriptor {
        slug: "state_ed",
        provider: "New York State Education Department school directory (data.nysed.gov)",
        transport: TransportKind::Html,
        capabilities: SCHOOL_ADDRESS,
        admission: artifact(),
    },
    SourceDescriptor {
        slug: "tssaa",
        provider: "Tennessee Secondary School Athletic Association school directory",
        transport: TransportKind::Html,
        capabilities: SCHOOL_COACH_CONTACT,
        admission: fetched("portal.tssaa.org", FETCHER_RPS),
    },
    SourceDescriptor {
        slug: "uhsaa",
        provider: "Utah High School Activities Association school directory",
        transport: TransportKind::Html,
        capabilities: SCHOOL_COACH_CONTACT,
        admission: fetched("uhsaa.org", FETCHER_RPS),
    },
    SourceDescriptor {
        slug: "home_campus",
        provider: "Home Campus CIF, FHSAA and NJSIAA school directory (www.cifsshome.org)",
        transport: TransportKind::Html,
        capabilities: SCHOOL_COACH_CONTACT,
        admission: fetched("www.cifsshome.org", HOME_CAMPUS_RPS),
    },
    SourceDescriptor {
        slug: "sidearm_staff",
        provider: "SIDEARM Miramonte High School staff directory (gomats.org)",
        transport: TransportKind::Html,
        capabilities: SCHOOL_COACH_CONTACT,
        admission: fetched("gomats.org", CRAWL_DELAY_THIRTY_RPS),
    },
];
