use super::{objects, TestResult};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, BufWriter, Seek, SeekFrom, Write};
use std::path::Path;

const IO_BUFFER_BYTES: usize = 64 * 1024;

#[derive(Deserialize)]
struct Inventory {
    #[serde(deserialize_with = "objects::deserialize_inventory")]
    objects: Vec<objects::Reference>,
}

#[derive(Serialize)]
struct InventoryWire<'a> {
    objects: &'a [objects::Reference],
}

#[test]
fn population_sized_inventory_admits_exact_limit_and_refuses_one_more() -> TestResult {
    check!(eq; objects::MAX_OBJECTS, 8_388_608);
    let mut file = tempfile::NamedTempFile::new()?;
    {
        let references = full_inventory()?;
        objects::validate(&references)?;
        write_inventory(file.as_file_mut(), &references)?;
    }
    let mut decoded = read_inventory(file.path())?;
    check!(eq; decoded.objects.len(), objects::MAX_OBJECTS);
    objects::validate(&decoded.objects)?;
    decoded.objects.try_reserve_exact(1)?;
    decoded.objects.push(objects::Reference {
        digest: format!("{:064x}", objects::MAX_OBJECTS),
        bytes: 1,
    });
    check!(objects::validate(&decoded.objects).is_err());
    write_inventory(file.as_file_mut(), &decoded.objects)?;
    drop(decoded);
    check!(read_inventory(file.path()).is_err());
    Ok(())
}

fn full_inventory() -> TestResult<Vec<objects::Reference>> {
    let mut references = Vec::new();
    references.try_reserve_exact(objects::MAX_OBJECTS)?;
    for index in 0..objects::MAX_OBJECTS {
        references.push(objects::Reference {
            digest: format!("{index:064x}"),
            bytes: 1,
        });
    }
    Ok(references)
}

fn write_inventory(file: &mut File, references: &[objects::Reference]) -> TestResult {
    file.set_len(0)?;
    file.seek(SeekFrom::Start(0))?;
    let mut writer = BufWriter::with_capacity(IO_BUFFER_BYTES, file);
    serde_json::to_writer(
        &mut writer,
        &InventoryWire {
            objects: references,
        },
    )?;
    writer.flush()?;
    Ok(())
}

fn read_inventory(path: &Path) -> TestResult<Inventory> {
    let reader = BufReader::with_capacity(IO_BUFFER_BYTES, File::open(path)?);
    Ok(serde_json::from_reader(reader)?)
}
