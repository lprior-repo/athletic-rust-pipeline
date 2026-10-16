use super::{exclude_out_of_scope, retain};
use census_domain::{JurisdictionBucket, UsJurisdiction};
use std::borrow::Cow;
use std::cell::Cell;

#[derive(Debug)]
struct Counted<'a> {
    value: usize,
    clones: &'a Cell<usize>,
}

impl Clone for Counted<'_> {
    fn clone(&self) -> Self {
        self.clones.set(self.clones.get().saturating_add(1));
        Self {
            value: self.value,
            clones: self.clones,
        }
    }
}

#[test]
fn unchanged_borrowed_selection_preserves_storage() {
    let source = [1, 2, 3];
    let mut selected = Cow::Borrowed(source.as_slice());
    retain(&mut selected, |value| *value > 0);
    assert!(matches!(selected, Cow::Borrowed(_)));
    assert_eq!(source.as_ptr(), selected.as_ptr());
}

#[test]
fn changed_borrowed_selection_clones_only_retained_members() {
    let clones = Cell::new(0);
    let source = [
        Counted {
            value: 1,
            clones: &clones,
        },
        Counted {
            value: 2,
            clones: &clones,
        },
        Counted {
            value: 3,
            clones: &clones,
        },
    ];
    let mut selected = Cow::Borrowed(source.as_slice());
    retain(&mut selected, |row| row.value != 2);
    assert!(matches!(selected, Cow::Owned(_)));
    assert_eq!(
        selected.iter().map(|row| row.value).collect::<Vec<_>>(),
        vec![1, 3]
    );
    assert_eq!(clones.get(), 2);
    retain(&mut selected, |row| row.value == 3);
    assert_eq!(clones.get(), 2);
    assert_eq!(selected.len(), 1);
}

#[test]
fn geography_without_exclusions_keeps_borrowed_storage() {
    let source = [UsJurisdiction::Michigan];
    let mut selected = Cow::Borrowed(source.as_slice());
    let outside = exclude_out_of_scope(
        &mut selected,
        |state| JurisdictionBucket::from(*state),
        &[UsJurisdiction::Michigan],
    );
    assert!(outside.is_empty());
    assert!(matches!(selected, Cow::Borrowed(_)));
    assert_eq!(source.as_ptr(), selected.as_ptr());
}

#[test]
fn geography_partition_preserves_member_order_and_outside_rows() {
    let source = [
        UsJurisdiction::Michigan,
        UsJurisdiction::Alaska,
        UsJurisdiction::Iowa,
    ];
    let mut selected = Cow::Borrowed(source.as_slice());
    let outside =
        exclude_out_of_scope(&mut selected, |state| JurisdictionBucket::from(*state), &[]);
    assert_eq!(
        selected.as_ref(),
        &[UsJurisdiction::Michigan, UsJurisdiction::Iowa]
    );
    assert_eq!(outside, vec![UsJurisdiction::Alaska]);
}

#[test]
fn selecting_nothing_does_not_clone_rejected_members() {
    let clones = Cell::new(0);
    let source = [Counted {
        value: 1,
        clones: &clones,
    }];
    let mut selected = Cow::Borrowed(source.as_slice());
    retain(&mut selected, |_| false);
    assert!(selected.is_empty());
    assert_eq!(clones.get(), 0);
}

mod population;
mod public_identity;
