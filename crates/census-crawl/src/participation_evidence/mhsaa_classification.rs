use super::{bound, hash, invalid};
use census_store::StoreResult;

mod document;
mod literal;
mod transitions;
mod types;
pub use literal::MhsaaOriginalSpanV1;
pub use types::{
    MhsaaClassificationCategoryV1, MhsaaClassificationTransitionV1, MhsaaTransitionBasisV1,
};

pub const MHSAA_CLASSIFICATION_SOURCE_URL: &str = "https://www.mhsaa.com/media/press-releases/mhsaa-2026-27-school-year-classifications-announced";
const MAX_ANNOUNCEMENT_BYTES: usize = 256 * 1024;

#[derive(Debug)]
pub struct MhsaaClassificationAnnouncementCandidateV1<'a> {
    original: &'a [u8],
    source_url: &'a str,
    source_sha256: String,
    article: MhsaaOriginalSpanV1<'a>,
    heading: MhsaaOriginalSpanV1<'a>,
    published_label: MhsaaOriginalSpanV1<'a>,
    native_path: MhsaaOriginalSpanV1<'a>,
    school_year_label: MhsaaOriginalSpanV1<'a>,
    paragraph: MhsaaOriginalSpanV1<'a>,
    revision_notice: MhsaaOriginalSpanV1<'a>,
    opt_up_notice: MhsaaOriginalSpanV1<'a>,
    transitions: [MhsaaClassificationTransitionV1<'a>; 4],
}

pub fn decode_mhsaa_classification_candidate_v1<'a>(
    source_url: &'a str,
    original: &'a [u8],
) -> StoreResult<MhsaaClassificationAnnouncementCandidateV1<'a>> {
    bound(
        original.len(),
        MAX_ANNOUNCEMENT_BYTES,
        "MHSAA announcement bytes",
    )?;
    if source_url != MHSAA_CLASSIFICATION_SOURCE_URL || original.is_empty() {
        return Err(invalid(
            "MHSAA announcement source is not the closed recipe",
        ));
    }
    let html = std::str::from_utf8(original)
        .map_err(|_| invalid("MHSAA announcement body is not UTF-8"))?;
    let document = document::read(html)?;
    let transitions = transitions::read(&document.paragraph)?;
    Ok(MhsaaClassificationAnnouncementCandidateV1 {
        original,
        source_url,
        source_sha256: hash(original),
        article: document.article,
        heading: document.heading,
        published_label: document.published_label,
        native_path: document.native_path,
        school_year_label: document.school_year_label,
        paragraph: document.paragraph,
        revision_notice: document.revision_notice,
        opt_up_notice: document.opt_up_notice,
        transitions,
    })
}

impl<'a> MhsaaClassificationAnnouncementCandidateV1<'a> {
    pub fn original_bytes(&self) -> &'a [u8] {
        self.original
    }
    pub fn source_url(&self) -> &'a str {
        self.source_url
    }
    pub fn source_sha256(&self) -> &str {
        &self.source_sha256
    }
    pub fn article(&self) -> &MhsaaOriginalSpanV1<'a> {
        &self.article
    }
    pub fn heading(&self) -> &MhsaaOriginalSpanV1<'a> {
        &self.heading
    }
    pub fn published_label(&self) -> &MhsaaOriginalSpanV1<'a> {
        &self.published_label
    }
    pub fn native_article_path(&self) -> &str {
        "node/28899"
    }
    pub fn native_path_locator(&self) -> &MhsaaOriginalSpanV1<'a> {
        &self.native_path
    }
    pub fn school_year_label(&self) -> &MhsaaOriginalSpanV1<'a> {
        &self.school_year_label
    }
    pub fn paragraph(&self) -> &MhsaaOriginalSpanV1<'a> {
        &self.paragraph
    }
    pub fn revision_notice(&self) -> &MhsaaOriginalSpanV1<'a> {
        &self.revision_notice
    }
    pub fn opt_up_notice(&self) -> &MhsaaOriginalSpanV1<'a> {
        &self.opt_up_notice
    }
    pub fn transitions(&self) -> &[MhsaaClassificationTransitionV1<'a>; 4] {
        &self.transitions
    }
}

#[cfg(test)]
mod tests;
