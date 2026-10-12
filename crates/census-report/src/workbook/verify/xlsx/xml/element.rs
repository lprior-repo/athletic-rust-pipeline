use crate::report::ReportResult;
use crate::workbook::verify::xlsx::Scope;

pub(in crate::workbook::verify::xlsx) struct Element<'a> {
    pub(in crate::workbook::verify::xlsx) name: &'a str,
    pub(in crate::workbook::verify::xlsx) attributes: &'a [(String, String)],
}

impl Element<'_> {
    pub(in crate::workbook::verify::xlsx) fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    pub(in crate::workbook::verify::xlsx) fn required(
        &self,
        name: &str,
        scope: &Scope<'_>,
    ) -> ReportResult<&str> {
        self.attribute(name).ok_or_else(|| {
            scope.corrupt(format!("element {} lacks its {name} attribute", self.name))
        })
    }
}

pub(in crate::workbook::verify::xlsx) trait Handler {
    fn start(&mut self, element: &Element<'_>) -> ReportResult<()>;
    fn end(&mut self, name: &str) -> ReportResult<()>;
    fn text(&mut self, text: &str) -> ReportResult<()>;
}

pub(super) fn trim(bytes: &[u8]) -> &[u8] {
    let start = bytes
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .map_or(bytes.len(), |index| index);
    let end = bytes
        .iter()
        .rposition(|byte| !byte.is_ascii_whitespace())
        .map_or(start, |index| index.saturating_add(1));
    bytes.get(start..end).map_or(&[], |trimmed| trimmed)
}
