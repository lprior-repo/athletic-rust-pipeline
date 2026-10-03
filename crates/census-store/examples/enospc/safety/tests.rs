use super::{is_enospc, parse_mount, parse_size, reject_host_device, MAX_ERROR_DEPTH};
use census_store::StoreError;
use std::cell::Cell;
use std::error::Error;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

#[test]
fn filesystem_cap_parses_exact_units_and_rejects_invalid_numbers() -> io::Result<()> {
    [
        ("1", 1),
        ("64M", 67_108_864),
        ("65536k", 67_108_864),
        ("256m", 268_435_456),
        ("268435456", 268_435_456),
        ("1G", 1_073_741_824),
    ]
    .into_iter()
    .try_for_each(|(text, bytes)| -> io::Result<()> {
        assert_eq!(parse_size(text)?, bytes);
        Ok(())
    })?;
    [
        "",
        "0",
        "0M",
        "-1",
        "+1",
        "64%",
        "64MB",
        "64 M",
        "64.0M",
        " 64M",
        "18446744073709551616",
        "18446744073709551615G",
    ]
    .into_iter()
    .for_each(|text| {
        assert_eq!(
            parse_size(text).err().map(|error| error.kind()),
            Some(io::ErrorKind::InvalidData)
        );
    });
    Ok(())
}

#[test]
fn filesystem_inspection_requires_one_private_whole_capped_tmpfs() -> io::Result<()> {
    let mount = parse_mount("tmpfs rw,size=65536k private /fault / 0:99\n")?;
    assert_eq!(mount.target, Path::new("/fault"));
    assert_eq!(mount.device, "0:99");
    let cap = parse_mount("tmpfs rw,size=256M private /fault / 0:99")?;
    assert_eq!(cap.target, Path::new("/fault"));
    [
        "",
        "tmpfs rw private /fault / 0:99",
        "tmpfs rw,size= private /fault / 0:99",
        "tmpfs rw,size=0 private /fault / 0:99",
        "tmpfs rw,size=257M private /fault / 0:99",
        "tmpfs rw,size=1G private /fault / 0:99",
        "tmpfs rw,size=50% private /fault / 0:99",
        "tmpfs rw,size=64M,size=32M private /fault / 0:99",
        "ext4 rw,size=64M private /fault / 8:1",
        "tmpfs rw,size=64M shared /fault / 0:99",
        "tmpfs rw,size=64M private / / 0:99",
        "tmpfs rw,size=64M private fault / 0:99",
        "tmpfs rw,size=64M private /fault /subdir 0:99",
        "tmpfs rw,size=64M private /fault / not-a-device",
        "tmpfs rw,size=64M private /fault / 0:99 extra",
        "tmpfs rw,size=64M private /fault / 0:99\ntmpfs rw,size=64M private /other / 0:100",
    ]
    .into_iter()
    .for_each(|text| {
        assert_eq!(
            parse_mount(text).err().map(|error| error.kind()),
            Some(io::ErrorKind::InvalidData)
        );
    });
    Ok(())
}

#[test]
fn fault_medium_rejects_a_device_also_visible_in_the_host_mount_table() -> io::Result<()> {
    let mounts = "36 29 0:42 / /tmp rw - tmpfs tmpfs rw\n37 29 8:1 / / rw - ext4 /dev/sda1 rw\n";
    assert_eq!(reject_host_device(mounts, "0:99")?, ());
    assert_eq!(
        reject_host_device(mounts, "0:42")
            .err()
            .map(|error| error.kind()),
        Some(io::ErrorKind::InvalidData)
    );
    ["", "malformed", "36 29 bad-device"]
        .into_iter()
        .for_each(|text| {
            assert_eq!(
                reject_host_device(text, "0:99")
                    .err()
                    .map(|error| error.kind()),
                Some(io::ErrorKind::InvalidData)
            );
        });
    Ok(())
}

#[derive(Debug)]
struct Nested(Box<dyn Error>);

impl fmt::Display for Nested {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("nested fault")
    }
}

impl Error for Nested {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.0.as_ref())
    }
}

#[test]
fn enospc_requires_the_raw_kernel_errno_in_the_bounded_error_chain() {
    let error = StoreError::Io {
        path: PathBuf::from("owned-fault"),
        source: io::Error::from_raw_os_error(28),
    };
    assert_eq!(is_enospc(&error), true);
    assert_eq!(is_enospc(&io::Error::from_raw_os_error(13)), false);
    assert_eq!(
        is_enospc(&io::Error::other(
            "ENOSPC: No space left on device (os error 28)"
        )),
        false
    );
    let in_bound = (0..MAX_ERROR_DEPTH.saturating_sub(1)).fold(
        Box::new(io::Error::from_raw_os_error(28)) as Box<dyn Error>,
        |error, _| Box::new(Nested(error)) as Box<dyn Error>,
    );
    assert_eq!(is_enospc(in_bound.as_ref()), true);
    assert_eq!(is_enospc(&Nested(in_bound)), false);
}

#[derive(Debug)]
struct Cyclic(Cell<usize>);

impl fmt::Display for Cyclic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("cyclic source chain")
    }
}

impl Error for Cyclic {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.0.set(self.0.get().saturating_add(1));
        Some(self)
    }
}

#[test]
fn cyclic_error_sources_stop_at_the_static_traversal_budget() {
    let error = Cyclic(Cell::new(0));
    assert_eq!(is_enospc(&error), false);
    assert_eq!(error.0.get(), MAX_ERROR_DEPTH);
}
