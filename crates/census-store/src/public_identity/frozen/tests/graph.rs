use super::*;
use crate::public_identity::{
    graph_witness::HeldGraphWitness,
    paged_generation::{capture::verify, ObjectRefV2},
};
use std::collections::BTreeMap;

pub(super) fn inspect(
    bytes: &[u8],
) -> TestResult<(Vec<wire::Operation>, BTreeMap<String, ObjectRefV2>)> {
    let bundle: serde_json::Value = serde_json::from_slice(bytes)?;
    check!(eq; bundle.get("version").and_then(serde_json::Value::as_u64), Some(2));
    check!(bundle.get("contexts").is_none());
    let operations = bundle
        .get("operations")
        .and_then(serde_json::Value::as_array)
        .ok_or("graph operations")?
        .iter()
        .cloned()
        .map(serde_json::from_value::<wire::Operation>)
        .collect::<Result<Vec<_>, _>>()?;
    let mut objects = BTreeMap::new();
    for object in bundle
        .get("objects")
        .and_then(serde_json::Value::as_array)
        .ok_or("graph objects")?
    {
        let reference: ObjectRefV2 =
            serde_json::from_value(object.get("reference").ok_or("graph reference")?.clone())?;
        let root = reference.root().as_str().to_owned();
        check!(objects.insert(root, reference).is_none());
    }
    Ok((operations, objects))
}

pub(super) fn assert_exact_inventory(store: &Store, bytes: &[u8]) -> TestResult {
    let (operations, actual) = inspect(bytes)?;
    let snapshot = store.snapshot();
    let mut expected = BTreeMap::new();
    for operation in operations {
        let held =
            HeldGraphWitness::decode("frozen graph test witness", operation.witness.as_bytes())?;
        verify::verify(
            &snapshot,
            held.graph().capture(),
            held.graph().seal(),
            |_, object| {
                let root = object.root().as_str().to_owned();
                if expected
                    .insert(root, object.clone())
                    .is_some_and(|previous| previous != *object)
                {
                    return Err(PublicIdentityError::Corrupt(
                        "test graph object reference conflict",
                    )
                    .into());
                }
                Ok(())
            },
        )?;
    }
    check!(eq; actual, expected);
    Ok(())
}
