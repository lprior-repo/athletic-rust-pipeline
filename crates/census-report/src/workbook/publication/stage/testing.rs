use super::*;

impl<'s> Stage<'s> {
    pub(in crate::workbook) fn with_before_input_fence(
        mut self,
        callback: &'s dyn Fn() -> ReportResult<()>,
    ) -> Self {
        self.before_input_fence = Some(callback);
        self
    }

    pub(in crate::workbook) fn with_after_input_intent(
        mut self,
        callback: &'s dyn Fn() -> ReportResult<()>,
    ) -> Self {
        self.after_input_intent = Some(callback);
        self
    }
}
