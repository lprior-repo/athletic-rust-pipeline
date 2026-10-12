use super::ReportResult;
use std::path::Path;

pub(super) fn reached(position: &str, root: &Path) -> ReportResult<()> {
    #[cfg(test)]
    stop_child(position, root)?;
    #[cfg(not(test))]
    let _position = (position, root);
    Ok(())
}

#[cfg(test)]
fn stop_child(position: &str, root: &Path) -> ReportResult<()> {
    use std::io::{Read, Write};
    if std::env::var("CENSUS_PUBLICATION_CRASH_WINDOW")
        .ok()
        .as_deref()
        != Some(position)
    {
        return Ok(());
    }
    let socket = std::env::var_os("CENSUS_PUBLICATION_CRASH_SOCKET")
        .map(std::path::PathBuf::from)
        .ok_or_else(|| super::invariant(format!("checkpoint {} has no socket", root.display())))?;
    let mut stream = std::os::unix::net::UnixStream::connect(&socket)
        .map_err(|source| super::io_error(&socket, source))?;
    stream
        .write_all(position.as_bytes())
        .map_err(|source| super::io_error(&socket, source))?;
    let mut release = [0_u8; 1];
    stream
        .read_exact(&mut release)
        .map_err(|source| super::io_error(&socket, source))?;
    if std::env::var("CENSUS_PUBLICATION_CRASH_TERMINATION")
        .ok()
        .as_deref()
        == Some("abort")
    {
        std::process::abort();
    }
    Ok(())
}
