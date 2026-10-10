use crate::report::{Census, ReportResult};
use crate::workbook::Censuses;
use census_crawl::{descriptors, SourceCapabilities, SourceDescriptor, TransportKind};
use std::collections::BTreeMap;

use super::{header, sorted_counts, Expect, Sheet};

const HEADERS: [&str; 7] = [
    "Source",
    "Provider",
    "Transport",
    "Capabilities",
    "Origin",
    "Requests/s",
    "In flight",
];

const CHANNEL_HEADERS: [&str; 4] = ["Evidence channel", "Source id", "Records", ""];

const CHANNELS: [&str; 4] = [
    "Meet provider namespace",
    "Coach evidence source",
    "Grade-evidence source",
    "Athlete source namespace",
];

type CapabilityFlag = (&'static str, fn(&SourceCapabilities) -> bool);

const CAPABILITY_FLAGS: [CapabilityFlag; 10] = [
    ("athlete_discovery", |capabilities| {
        capabilities.athlete_discovery
    }),
    ("athlete_profile", |capabilities| {
        capabilities.athlete_profile
    }),
    ("meet_discovery", |capabilities| capabilities.meet_discovery),
    ("bulk_results", |capabilities| capabilities.bulk_results),
    ("grade_evidence", |capabilities| capabilities.grade_evidence),
    ("graduation_evidence", |capabilities| {
        capabilities.graduation_evidence
    }),
    ("school_evidence", |capabilities| {
        capabilities.school_evidence
    }),
    ("coach_directory", |capabilities| {
        capabilities.coach_directory
    }),
    ("public_professional_contact", |capabilities| {
        capabilities.public_professional_contact
    }),
    ("pr_evidence", |capabilities| capabilities.pr_evidence),
];

pub(super) fn expected(censuses: &Censuses) -> ReportResult<Sheet> {
    let mut rows = vec![header(&HEADERS)];
    for descriptor in descriptors() {
        rows.push(descriptor_row(descriptor)?);
    }
    rows.push(Vec::new());
    rows.push(header(&CHANNEL_HEADERS));
    rows.extend(channel_rows(&censuses.all_sources)?);
    Ok(("Sources", rows))
}

fn descriptor_row(descriptor: &SourceDescriptor) -> ReportResult<Vec<Expect>> {
    let admission = &descriptor.admission;
    Ok(vec![
        Expect::text(descriptor.slug),
        Expect::text(descriptor.provider),
        Expect::text(transport_label(descriptor.transport)),
        Expect::text(capability_labels(&descriptor.capabilities)),
        Expect::text(admission.origin),
        Expect::Number(admission.target_requests_per_second),
        Expect::count(admission.maximum_in_flight.get())?,
    ])
}

fn channel_rows(census: &Census) -> ReportResult<Vec<Vec<Expect>>> {
    let channels: [&BTreeMap<String, usize>; 4] = [
        &census.meets.by_provider,
        &census.coach_sources,
        &census.providers.grade_evidence_sources,
        &census.providers.namespaces,
    ];
    let mut rows = Vec::new();
    for (channel, counts) in CHANNELS.into_iter().zip(channels) {
        for (id, count) in sorted_counts(counts) {
            rows.push(vec![
                Expect::text(channel),
                Expect::text(id.as_str()),
                Expect::count(*count)?,
                Expect::Empty,
            ]);
        }
    }
    Ok(rows)
}

fn capability_labels(capabilities: &SourceCapabilities) -> String {
    let labels: Vec<&str> = CAPABILITY_FLAGS
        .iter()
        .filter(|(_, claimed)| claimed(capabilities))
        .map(|(label, _)| *label)
        .collect();
    if labels.is_empty() {
        return "none".to_string();
    }
    labels.join(", ")
}

fn transport_label(kind: TransportKind) -> &'static str {
    match kind {
        TransportKind::StructuredApi => "structured api",
        TransportKind::StaticJson => "static json",
        TransportKind::Csv => "csv",
        TransportKind::Xml => "xml",
        TransportKind::Xlsx => "xlsx",
        TransportKind::Html => "html",
        TransportKind::Pdf => "pdf",
        TransportKind::Browser => "browser",
    }
}
