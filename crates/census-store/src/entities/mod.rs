//! `Entity` for the canonical types: the id a row is keyed by, and how a duplicate observation is
//! absorbed at read time.
//!
//! `canonical` holds the seven rows the merge materializes; `derived` holds the observations, cases
//! and queues the census keeps about its own findings; `observations` holds what a source said about
//! its own school and athlete objects. `union_vec` is the one merge helper all three share.

mod canonical;
mod derived;
mod observations;

fn union_vec<T: PartialEq + Clone>(left: &mut Vec<T>, right: &[T]) {
    for item in right {
        if !left.contains(item) {
            left.push(item.clone());
        }
    }
}
