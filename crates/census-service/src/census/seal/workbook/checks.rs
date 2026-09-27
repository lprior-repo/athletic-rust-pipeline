pub(super) fn check_required_sheets(names: &[&str], required: &[&str]) -> Vec<String> {
    required
        .iter()
        .filter(|required| !names.contains(*required))
        .map(|required| format!("the workbook has no {required} sheet"))
        .collect()
}
