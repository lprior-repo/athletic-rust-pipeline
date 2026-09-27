
use crate::report::{Census, ReportResult};
use census_crawl::{descriptors, SourceCapabilities, SourceDescriptor, TransportKind};
use std::collections::BTreeMap;

use crate::workbook::cells::{row, Cell};

use super::sorted_counts;

pub(super) const SOURCE_WIDTHS: [u16; 8] = [22, 46, 15, 62, 24, 10, 9, 11];

pub(super) fn sources_sheet(census: &Census) -> ReportResult<Vec<Vec<Cell>>> {
    let mut cells = vec![row!(
        "Source",
        "Provider",
        "Transport",
        "Capabilities",
        "Origin",
        "Requests/s",
        "In flight",
        "Crawl delay",
    )];
    for descriptor in descriptors() {
        cells.push(descriptor_row(descriptor)?);
    }
    cells.push(row!());
    cells.push(row!("Evidence channel", "Source id", "Records", ""));
    for (channel, counts) in evidence_channels(census) {
        for (id, count) in sorted_counts(counts) {
            cells.push(row!(
                Cell::text(channel),
                Cell::text(id.clone()),
                Cell::number(*count)?,
                Cell::Empty,
            ));
        }
    }
    Ok(cells)
}

fn descriptor_row(descriptor: &SourceDescriptor) -> ReportResult<Vec<Cell>> {
    let admission = &descriptor.admission;
    Ok(row!(
        Cell::text(descriptor.slug),
        Cell::text(descriptor.provider),
        Cell::text(transport_label(descriptor.transport)),
        Cell::text(capability_labels(&descriptor.capabilities)),
        Cell::text(admission.origin),
        Cell::Number(admission.target_requests_per_second),
        Cell::number(admission.maximum_in_flight.get())?,
        Cell::text(if admission.robots_crawl_delay_respected {
            "respected"
        } else {
            "not declared"
        }),
    ))
}

fn evidence_channels(census: &Census) -> Vec<(&'static str, &BTreeMap<String, usize>)> {
    vec![
        ("Meet provider namespace", &census.meets.by_provider),
        ("Coach evidence source", &census.coach_sources),
        (
            "Grade-evidence source",
            &census.providers.grade_evidence_sources,
        ),
        ("Athlete source namespace", &census.providers.namespaces),
    ]
}

fn capability_labels(capabilities: &SourceCapabilities) -> String {
    let mut labels: Vec<&str> = Vec::new();
    push_flag(
        &mut labels,
        capabilities.athlete_discovery,
        "athlete_discovery",
    );
    push_flag(&mut labels, capabilities.athlete_profile, "athlete_profile");
    push_flag(&mut labels, capabilities.meet_discovery, "meet_discovery");
    push_flag(&mut labels, capabilities.bulk_results, "bulk_results");
    push_flag(&mut labels, capabilities.grade_evidence, "grade_evidence");
    push_flag(
        &mut labels,
        capabilities.graduation_evidence,
        "graduation_evidence",
    );
    push_flag(&mut labels, capabilities.school_evidence, "school_evidence");
    push_flag(&mut labels, capabilities.coach_directory, "coach_directory");
    push_flag(
        &mut labels,
        capabilities.public_professional_contact,
        "public_professional_contact",
    );
    push_flag(&mut labels, capabilities.pr_evidence, "pr_evidence");
    if labels.is_empty() {
        return "none".to_string();
    }
    labels.join(", ")
}

fn push_flag(labels: &mut Vec<&'static str>, claimed: bool, label: &'static str) {
    if claimed {
        labels.push(label);
    }
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
