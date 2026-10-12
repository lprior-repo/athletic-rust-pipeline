use super::super::fixtures::{bundle, school, TestResult};
use super::super::stream::StreamKind;
use super::{ChunkReader, EncodedWindowLimit};
use census_domain::model::CanonicalSchool;

#[test]
fn frozen_window_stops_before_its_record_or_encoded_limit_without_skipping_rows() -> TestResult {
    let mut rows = [school("Window A"), school("Window B"), school("Window C")];
    rows.sort_by(|left, right| left.id.cmp(&right.id));
    let encoded = bundle(StreamKind::Schools, &rows)?;
    let bytes = encoded.objects.first().ok_or("missing fixture chunk")?;
    let first_bytes = bytes
        .iter()
        .position(|b| *b == b'\n')
        .ok_or("newline missing")?
        .checked_add(1)
        .ok_or("overflow")?;
    for limit in [
        EncodedWindowLimit::new(1, 64 * 1024 * 1024)?,
        EncodedWindowLimit::new(25_000, u64::try_from(first_bytes)?)?,
    ] {
        let mut reader = ChunkReader::open(StreamKind::Schools, bytes, 0)?;
        let mut actual = Vec::new();
        let window = reader.visit_window::<CanonicalSchool>(limit, |row| {
            actual.push(row.clone());
            Ok(())
        })?;
        check!(eq; window.rows, 1);
        check!(eq; window.encoded_bytes, u64::try_from(first_bytes)?);
        check!(eq; actual, rows.get(..1).ok_or("fixture row")?);
        while let Some(decoded) = reader.next_record::<CanonicalSchool>()? {
            actual.push(decoded.payload);
        }
        check!(eq; actual, rows);
    }
    Ok(())
}

#[test]
fn frozen_window_refuses_a_record_that_cannot_fit_without_advancing() -> TestResult {
    let row = school("No Room High");
    let encoded = bundle(StreamKind::Schools, std::slice::from_ref(&row))?;
    let bytes = encoded.objects.first().ok_or("missing fixture chunk")?;
    let mut reader = ChunkReader::open(StreamKind::Schools, bytes, 0)?;
    let error =
        match reader.visit_window::<CanonicalSchool>(EncodedWindowLimit::new(1, 1)?, |_| Ok(())) {
            Ok(_) => return Err("oversize window record was accepted".into()),
            Err(error) => error,
        };
    check!(matches!(
        error,
        super::super::codec::FrozenCodecError::BudgetExceeded {
            what: "frozen window encoded bytes",
            available: 1,
            ..
        }
    ));
    check!(eq; reader.rows(), 0);
    check!(eq; reader.next_record::<CanonicalSchool>()?.map(|r| r.payload), Some(row));
    Ok(())
}
