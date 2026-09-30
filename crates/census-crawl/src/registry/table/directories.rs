use super::super::policy::{artifact, fetched, FETCHER_RPS, SCHOOL_ADDRESS, SCHOOL_COACH_CONTACT};
use super::super::{SourceDescriptor, TransportKind};

pub(super) const DIRECTORIES: [SourceDescriptor; 3] = [
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
];
