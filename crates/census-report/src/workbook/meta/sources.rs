use crate::report::{Census, ReportResult};
use census_crawl::{descriptors, SourceCapabilities, SourceDescriptor, TransportKind};
use std::collections::BTreeMap;

use crate::workbook::cells::{row, Cell};

use super::sorted_counts;

pub(super) const SOURCE_WIDTHS: [u16; 8] = [22, 46, 15, 62, 24, 10, 9, 11];

pub(super) fn sources_sheet(census: &Census) -> ReportResult<Vec<Vec<Cell>>> {
    let mut cells = vec![source_header()];
    for descriptor in descriptors() {
        cells.push(descriptor_row(descriptor)?);
    }
    append_evidence_channels(&mut cells, census)?;
    Ok(cells)
}

fn source_header() -> Vec<Cell> {
    row!(
        "Source",
        "Provider",
        "Transport",
        "Capabilities",
        "Origin",
        "Requests/s",
        "In flight",
        "Crawl delay",
    )
}

fn append_evidence_channels(cells: &mut Vec<Vec<Cell>>, census: &Census) -> ReportResult<()> {
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
    Ok(())
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
    let flags = capability_flags(capabilities);
    let labels: Vec<&str> = flags
        .into_iter()
        .filter_map(|(claimed, label)| claimed.then_some(label))
        .collect();
    if labels.is_empty() {
        return "none".to_string();
    }
    labels.join(", ")
}

fn capability_flags(capabilities: &SourceCapabilities) -> [(bool, &'static str); 10] {
    [
        (capabilities.athlete_discovery, "athlete_discovery"),
        (capabilities.athlete_profile, "athlete_profile"),
        (capabilities.meet_discovery, "meet_discovery"),
        (capabilities.bulk_results, "bulk_results"),
        (capabilities.grade_evidence, "grade_evidence"),
        (capabilities.graduation_evidence, "graduation_evidence"),
        (capabilities.school_evidence, "school_evidence"),
        (capabilities.coach_directory, "coach_directory"),
        (
            capabilities.public_professional_contact,
            "public_professional_contact",
        ),
        (capabilities.pr_evidence, "pr_evidence"),
    ]
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
