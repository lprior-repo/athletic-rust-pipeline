use super::{CalendarCandidate, RawEvidence};

pub(super) fn same_bytes(left: &[u8], right: &[u8]) -> bool {
    std::ptr::eq(left, right) || left == right
}

pub(super) fn comparison_bytes(left: &[u8], right: &[u8]) -> usize {
    if left.len() != right.len() || std::ptr::eq(left, right) {
        0
    } else {
        left.len()
    }
}

impl PartialEq for RawEvidence<'_> {
    fn eq(&self, other: &Self) -> bool {
        same_capture(self, other)
            && self.interpretation_key == other.interpretation_key
            && same_bytes(self.interpretation_bytes, other.interpretation_bytes)
            && self.parser_artifact == other.parser_artifact
            && self.extraction_schema == other.extraction_schema
            && self.extraction_config == other.extraction_config
            && self.reference_dependencies == other.reference_dependencies
    }
}

pub(super) fn same_capture(left: &RawEvidence<'_>, right: &RawEvidence<'_>) -> bool {
    left.occurrence == right.occurrence
        && left.manifest_key == right.manifest_key
        && same_bytes(left.manifest_bytes, right.manifest_bytes)
        && left.metadata == right.metadata
        && left.owner == right.owner
        && left.source == right.source
        && left.requested_url == right.requested_url
        && left.final_url == right.final_url
        && left.acquired_at == right.acquired_at
        && same_bytes(left.body, right.body)
}

impl PartialEq for CalendarCandidate<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.year == other.year
            && self.sport == other.sport
            && self.interval == other.interval
            && same_bytes(self.raw_label, other.raw_label)
            && self.label_convention == other.label_convention
            && self.mapping_rule == other.mapping_rule
            && self.evidence == other.evidence
            && self.locator == other.locator
            && self.statement == other.statement
            && self.known == other.known
    }
}
