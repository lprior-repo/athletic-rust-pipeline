use super::super::chunks::ChunkWriter;
use super::super::codec::{
    FrozenCodecError, CODEC_REVISION, LINE_BYTES_LIMIT, MAX_CHUNKS_PER_STREAM,
};
use super::super::fixtures::{athlete, bundle, refusal, refusal_what, school, TestResult};
use super::super::inventory::{ChunkDescriptor, StreamInventory};
use super::super::line::{decode_line, encode_line};
use super::super::reader::ChunkReader;
use super::super::stream::StreamKind;
use census_domain::model::CanonicalAthlete;

#[test]
fn frozen_codec_oversize_record_refuses_typed_and_keeps_the_writer_usable() -> TestResult {
    let school = school("Oversize High");
    let mut huge = athlete(&school.id, "Huge Runner", "huge");
    huge.canonical_name = "x".repeat(700 * 1024);
    huge.known_names = vec![huge.canonical_name.clone()];
    let normal = athlete(&school.id, "Normal Runner", "normal");
    let mut writer = ChunkWriter::new(StreamKind::Athletes);
    let error = refusal(writer.push(0, &huge))?;
    check!(matches!(error, FrozenCodecError::RowTooLarge { .. }));
    check!(eq; writer.rows(), 0);
    writer.push(0, &normal)?;
    let (last, inventory) = writer.finish()?;
    let mut objects = Vec::new();
    if let Some(chunk) = last {
        objects.push(chunk.bytes);
    }
    check!(eq; inventory.rows(), 1);
    let decoded: Vec<CanonicalAthlete> =
        super::super::fixtures::read_back(StreamKind::Athletes, &objects, &inventory)?;
    check!(eq; decoded, vec![normal]);
    Ok(())
}

#[test]
fn frozen_codec_deep_nesting_refuses_typed_before_allocation() -> TestResult {
    let mut line = String::from("{\"ordinal\":0,\"entity_id\":\"ath_deep\",\"payload\":");
    line.push_str(&"[".repeat(70));
    line.push_str(&"]".repeat(70));
    line.push('}');
    let error = refusal(decode_line::<CanonicalAthlete>(line.as_bytes()))?;
    check!(matches!(error, FrozenCodecError::BudgetExceeded { .. }));
    check!(eq; refusal_what(&error), Some("frozen json depth"));
    Ok(())
}

#[test]
fn frozen_codec_huge_string_refuses_typed_before_allocation() -> TestResult {
    let line = format!(
        "{{\"ordinal\":0,\"entity_id\":\"ath_string\",\"payload\":{{\"canonical_name\":\"{}\"}}}}",
        "y".repeat(300 * 1024)
    );
    let error = refusal(decode_line::<CanonicalAthlete>(line.as_bytes()))?;
    check!(matches!(error, FrozenCodecError::BudgetExceeded { .. }));
    check!(eq; refusal_what(&error), Some("frozen json string bytes"));
    Ok(())
}

#[test]
fn frozen_codec_wide_object_refuses_typed_before_allocation() -> TestResult {
    let mut payload = String::new();
    for index in 0..70_000 {
        if index > 0 {
            payload.push(',');
        }
        payload.push_str("\"k");
        payload.push_str(&index.to_string());
        payload.push_str("\":0");
    }
    let line = format!("{{\"ordinal\":0,\"entity_id\":\"ath_wide\",\"payload\":{{{payload}}}}}");
    check!(u64::try_from(line.len())? < LINE_BYTES_LIMIT);
    let error = refusal(decode_line::<CanonicalAthlete>(line.as_bytes()))?;
    check!(matches!(error, FrozenCodecError::BudgetExceeded { .. }));
    check!(eq; refusal_what(&error), Some("frozen json nodes"));
    Ok(())
}

#[test]
fn frozen_codec_chunk_reader_refuses_blank_lines_and_missing_newline() -> TestResult {
    let school = school("Reader High");
    let runner = athlete(&school.id, "Ada Runner", "1");
    let line = encode_line(0, &runner)?;
    let mut blank = line.clone();
    blank.push(b'\n');
    blank.push(b'\n');
    let error = refusal(ChunkReader::open(StreamKind::Athletes, &blank, 0))?;
    check!(matches!(error, FrozenCodecError::CorruptArtifact { .. }));
    let error = refusal(ChunkReader::open(StreamKind::Athletes, &line, 0))?;
    check!(matches!(error, FrozenCodecError::CorruptArtifact { .. }));
    let mut closed = line.clone();
    closed.push(b'\n');
    check!(ChunkReader::open(StreamKind::Athletes, &closed, 0).is_ok());
    Ok(())
}

#[test]
fn frozen_codec_chunk_reader_refuses_invalid_utf8() -> TestResult {
    let bytes = [0x80_u8, b'\n'];
    let error = refusal(ChunkReader::open(StreamKind::Athletes, &bytes, 0))?;
    check!(matches!(error, FrozenCodecError::CorruptArtifact { .. }));
    Ok(())
}

#[test]
fn frozen_codec_chunk_reader_refuses_ordinal_gaps_and_identity_regression() -> TestResult {
    let school = school("Gap High");
    let first = athlete(&school.id, "First Runner", "1");
    let second = athlete(&school.id, "Second Runner", "2");
    let (greater, smaller) = if first.id.as_str() > second.id.as_str() {
        (first, second)
    } else {
        (second, first)
    };
    let mut gapped = Vec::new();
    gapped.extend_from_slice(&encode_line(0, &greater)?);
    gapped.push(b'\n');
    gapped.extend_from_slice(&encode_line(2, &smaller)?);
    gapped.push(b'\n');
    let mut reader = ChunkReader::open(StreamKind::Athletes, &gapped, 0)?;
    check!(reader.next_record::<CanonicalAthlete>()?.is_some());
    let gap = refusal(reader.next_record::<CanonicalAthlete>())?;
    check!(matches!(gap, FrozenCodecError::CorruptArtifact { .. }));

    let mut regressed = Vec::new();
    regressed.extend_from_slice(&encode_line(0, &greater)?);
    regressed.push(b'\n');
    regressed.extend_from_slice(&encode_line(1, &smaller)?);
    regressed.push(b'\n');
    let mut reader = ChunkReader::open(StreamKind::Athletes, &regressed, 0)?;
    check!(reader.next_record::<CanonicalAthlete>()?.is_some());
    let regression = refusal(reader.next_record::<CanonicalAthlete>())?;
    check!(matches!(
        regression,
        FrozenCodecError::CorruptArtifact { .. }
    ));
    Ok(())
}

#[test]
fn frozen_codec_inventory_refuses_unsupported_revisions_and_index_gaps() -> TestResult {
    let school = school("Inventory High");
    let runner = athlete(&school.id, "Ada Runner", "1");
    let bundle = bundle(StreamKind::Athletes, std::slice::from_ref(&runner))?;
    let encoded = String::from_utf8(bundle.inventory.encode()?)?;
    let stale = encoded.replacen("\"codec_revision\":3", "\"codec_revision\":4", 1);
    check!(stale != encoded);
    let error = refusal(StreamInventory::decode(stale.as_bytes()))?;
    check!(matches!(error, FrozenCodecError::UnsupportedVersion { .. }));
    check!(eq; refusal_what(&error), Some("stream inventory"));
    let gapped = encoded.replacen("\"index\":0", "\"index\":5", 1);
    check!(gapped != encoded);
    let error = refusal(StreamInventory::decode(gapped.as_bytes()))?;
    check!(matches!(error, FrozenCodecError::CorruptArtifact { .. }));
    let decoded = StreamInventory::decode(encoded.as_bytes())?;
    check!(eq; decoded, bundle.inventory);
    Ok(())
}

#[test]
fn frozen_codec_inventory_refuses_more_chunks_than_the_envelope() -> TestResult {
    let sha256 = "0".repeat(64);
    let chunks: Vec<ChunkDescriptor> = (0..=MAX_CHUNKS_PER_STREAM)
        .map(|index| ChunkDescriptor {
            index,
            first_ordinal: index,
            rows: 1,
            bytes: 1,
            sha256: sha256.clone(),
            first_key: "ath_shared".to_string(),
            last_key: "ath_shared".to_string(),
            codec_revision: CODEC_REVISION,
        })
        .collect();
    let rows = u64::try_from(chunks.len())?;
    let error = refusal(StreamInventory::new(
        StreamKind::Athletes,
        chunks,
        rows,
        rows,
    ))?;
    check!(matches!(error, FrozenCodecError::NeedsPartition { .. }));
    Ok(())
}
