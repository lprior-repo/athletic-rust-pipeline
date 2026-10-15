use super::in_run_scope;
use census_domain::{JurisdictionBucket, UsJurisdiction};
use std::borrow::Cow;

pub(super) fn exclude_out_of_scope<T: Clone>(
    rows: &mut Cow<'_, [T]>,
    place: impl Fn(&T) -> JurisdictionBucket,
    jurisdictions: &[UsJurisdiction],
) -> Vec<T> {
    if rows
        .iter()
        .all(|row| in_run_scope(place(row), jurisdictions))
    {
        return Vec::new();
    }
    super::exclude_out_of_scope(rows.to_mut(), place, jurisdictions)
}

pub(super) fn retain<T: Clone>(rows: &mut Cow<'_, [T]>, keep: impl Fn(&T) -> bool) {
    match rows {
        Cow::Borrowed(borrowed) => {
            if borrowed.iter().any(|row| !keep(row)) {
                *rows = Cow::Owned(borrowed.iter().filter(|row| keep(row)).cloned().collect());
            }
        }
        Cow::Owned(owned) => owned.retain(keep),
    }
}

#[cfg(test)]
mod tests;
