use super::super::policy::{
    artifact, fetched, FETCHER_RPS, SCHOOL_COACH_CONTACT, SCHOOL_COACH_NAMES,
};
use super::super::SourceCapabilities as Caps;
use super::super::{SourceDescriptor, TransportKind};

pub(super) const THROUGH_MILESPLIT: [SourceDescriptor; 9] = [
    SourceDescriptor {
        slug: "athleticlive",
        provider: "AthleticLIVE meet harvest and result-plane captures (research artifacts)",
        transport: TransportKind::Csv,
        capabilities: Caps {
            meet_discovery: true,
            bulk_results: true,
            grade_evidence: true,
            ..Caps::NONE
        },
        admission: artifact(),
    },
    SourceDescriptor {
        slug: "athleticlive_athletes",
        provider: "AthleticLIVE athlete index (Elasticsearch)",
        transport: TransportKind::StructuredApi,
        capabilities: Caps {
            athlete_discovery: true,
            grade_evidence: true,
            school_evidence: true,
            ..Caps::NONE
        },
        admission: fetched("search.athletic.live", FETCHER_RPS),
    },
    SourceDescriptor {
        slug: "athleticnet",
        provider: "Athletic.net athlete bio API and whole-meet pull",
        transport: TransportKind::Browser,
        capabilities: Caps {
            athlete_profile: true,
            bulk_results: true,
            grade_evidence: true,
            school_evidence: true,
            ..Caps::NONE
        },
        admission: fetched("www.athletic.net", FETCHER_RPS),
    },
    SourceDescriptor {
        slug: "ciac",
        provider: "CIAC sports directory (Connecticut)",
        transport: TransportKind::Html,
        capabilities: SCHOOL_COACH_NAMES,
        admission: fetched("ciacsports.com", FETCHER_RPS),
    },
    SourceDescriptor {
        slug: "coach_contacts",
        provider: "Researched official coach-contact dataset",
        transport: TransportKind::Csv,
        capabilities: SCHOOL_COACH_CONTACT,
        admission: artifact(),
    },
    SourceDescriptor {
        slug: "ihsa",
        provider: "Illinois High School Association (IHSA) API",
        transport: TransportKind::StructuredApi,
        capabilities: SCHOOL_COACH_CONTACT,
        admission: fetched("api.ihsa.org", FETCHER_RPS),
    },
    SourceDescriptor {
        slug: "ks",
        provider: "Kansas State High School Activities Association (KSHSAA) directory",
        transport: TransportKind::StructuredApi,
        capabilities: SCHOOL_COACH_CONTACT,
        admission: fetched("kshsaa-api.kshsaa.org", FETCHER_RPS),
    },
    SourceDescriptor {
        slug: "milesplit",
        provider: "MileSplit state sites",
        transport: TransportKind::Html,
        capabilities: Caps {
            athlete_discovery: true,
            bulk_results: true,
            grade_evidence: true,
            graduation_evidence: true,
            meet_discovery: true,
            school_evidence: true,
            ..Caps::NONE
        },
        admission: fetched("milesplit.com", FETCHER_RPS),
    },
    SourceDescriptor {
        slug: "coach_directories",
        provider: "DragonFly Athletics association member directories",
        transport: TransportKind::StructuredApi,
        capabilities: Caps {
            school_evidence: true,
            coach_directory: true,
            public_professional_contact: true,
            ..Caps::NONE
        },
        admission: fetched("maxinfosite-api-live.dragonflyathletics.com", FETCHER_RPS),
    },
];
