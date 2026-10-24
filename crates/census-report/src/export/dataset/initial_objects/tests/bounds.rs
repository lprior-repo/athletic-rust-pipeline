use super::*;

#[test]
fn scalar_reference_validation_accepts_inclusive_object_byte_cap_and_refuses_next_byte(
) -> TestResult {
    let fixture = Fixture::new()?;
    let installed = fixture.install(b"genuine digest for scalar validation")?;
    let mut reference = installed.clone();
    reference.bytes = 0;
    invariant(
        objects::validate(&[reference.clone()]),
        "selected proof inventory is noncanonical or exceeds an object limit",
    )?;
    reference.bytes = proof_objects::MAX_OBJECT_BYTES;
    objects::validate(&[reference.clone()])?;
    reference.bytes = reference.bytes.checked_add(1).ok_or("test byte overflow")?;
    invariant(
        objects::validate(&[reference]),
        "selected proof inventory is noncanonical or exceeds an object limit",
    )?;
    fixture.object(&installed, b"genuine digest for scalar validation")?;
    fixture.sentinel()
}

#[test]
fn scalar_reference_validation_accepts_inclusive_total_byte_cap_and_refuses_next_byte() -> TestResult
{
    let fixture = Fixture::new()?;
    check!(eq; objects::MAX_OBJECTS, 8_388_608);
    check!(eq; proof_objects::MAX_OBJECT_BYTES, 4_294_967_296);
    check!(eq; objects::MAX_BYTES, 17_592_186_044_416);
    let mut references = (0_u64..4096)
        .map(|index| objects::Reference {
            digest: format!("{index:064x}"),
            bytes: proof_objects::MAX_OBJECT_BYTES,
        })
        .collect::<Vec<_>>();
    check!(eq; references.len(), 4096);
    objects::validate(&references)?;
    references.push(objects::Reference {
        digest: format!("{:064x}", 4096),
        bytes: 1,
    });
    invariant(
        objects::validate(&references),
        "selected proof objects exceed the 16 TiB archive limit",
    )?;
    fixture.sentinel()
}
