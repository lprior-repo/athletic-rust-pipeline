use super::*;

#[test]
fn live_and_immutable_sources_share_full_verification_ordinals_and_merge() -> TestResult {
    let (_dir, store, retained) = captured(513)?;
    let snapshot = store.snapshot();
    let graph = VerifiedRawGraph::verify(&snapshot, retained.binding(), retained.seal())?;
    let mut expected = Vec::new();
    let expected_counts = snapshot.for_each_physical_observation_bounded::<CanonicalAthlete>(
        Table::Athletes,
        budget(),
        |_, key, raw, bytes| {
            expected.push((key.to_vec(), raw.to_vec(), bytes));
            Ok(())
        },
    )?;
    let mut actual = Vec::new();
    let mut ordinals = Vec::new();
    let counts = graph.for_each_physical::<CanonicalAthlete>(
        AuthorityTableV2::Athletes,
        budget(),
        |_, key, raw, ordinal, bytes| {
            ordinals.push(ordinal);
            actual.push((key.to_vec(), raw.to_vec(), bytes));
            Ok(())
        },
    )?;
    check!(eq; counts, expected_counts);
    check!(eq; actual, expected);
    check!(eq; ordinals, (0..513).collect::<Vec<u64>>());
    let mut expected_merged = Vec::new();
    let merged_counts = snapshot.for_each_merged_bounded(
        Table::Athletes,
        budget(),
        |row: CanonicalAthlete, bytes| {
            expected_merged.push((
                serde_json::to_vec(&row).map_err(|source| StoreError::Decode {
                    key: "fixture".into(),
                    source,
                })?,
                bytes,
            ));
            Ok(())
        },
    )?;
    check!(eq; merged(&graph)?, (merged_counts, expected_merged.clone()));
    let frozen = FrozenFixture::capture(&snapshot, &retained)?;
    let portable = VerifiedRawGraph::verify(&frozen, retained.binding(), retained.seal())?;
    check!(eq; merged(&portable)?, (merged_counts, expected_merged));
    check!(eq; portable.binding(), retained.binding());
    check!(eq; portable.seal(), retained.seal());
    check!(eq; portable.table_inventory(AuthorityTableV2::Athletes)?.rows(), 513);
    Ok(())
}

#[test]
fn immutable_source_needs_no_open_store_and_repeated_traversals_reset_counts() -> TestResult {
    let (frozen, retained) = {
        let (_dir, store, retained) = captured(2)?;
        (
            FrozenFixture::capture(&store.snapshot(), &retained)?,
            retained,
        )
    };
    let graph = VerifiedRawGraph::verify(&frozen, retained.binding(), retained.seal())?;
    let first = merged(&graph)?;
    let second = merged(&graph)?;
    check!(eq; first, second);
    check!(eq; first.0.decoded_rows, 2);
    check!(eq; first.0.merged_rows, 1);
    let empty = graph.for_each_observation::<census_domain::model::SourceObservation>(
        AuthorityTableV2::SourceObservations,
        budget(),
        |_, _| Err(super::super::super::invalid("unexpected empty callback")),
    )?;
    check!(eq; empty, ReadCounts::default());
    Ok(())
}
