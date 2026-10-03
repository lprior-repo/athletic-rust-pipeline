use super::ExampleResult;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

const RESERVE_CHUNK_BYTES: usize = 64 * 1024;
const RESERVE_CHUNKS: usize = 128;

pub(super) struct Reservation {
    file: File,
    path: PathBuf,
    device: u64,
    inode: u64,
}

impl Reservation {
    pub(super) fn create(root: &Path) -> ExampleResult<Self> {
        fs::DirBuilder::new().mode(0o700).create(root)?;
        let path = root.join(".enospc-reserve");
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)?;
        let chunk = [0_u8; RESERVE_CHUNK_BYTES];
        (0..RESERVE_CHUNKS).try_for_each(|_| file.write_all(&chunk))?;
        file.sync_all()?;
        File::open(root)?.sync_all()?;
        let metadata = file.metadata()?;
        Ok(Self {
            file,
            path,
            device: metadata.dev(),
            inode: metadata.ino(),
        })
    }

    pub(super) fn release(self) -> ExampleResult<()> {
        let metadata = fs::symlink_metadata(&self.path)?;
        if !metadata.is_file()
            || metadata.dev() != self.device
            || metadata.ino() != self.inode
            || metadata.nlink() != 1
        {
            return Err(io::Error::other("reserved-space file identity changed").into());
        }
        fs::remove_file(&self.path)?;
        drop(self.file);
        let root = self
            .path
            .parent()
            .ok_or_else(|| io::Error::other("reserved-space file has no parent"))?;
        File::open(root)?.sync_all()?;
        Ok(())
    }
}
