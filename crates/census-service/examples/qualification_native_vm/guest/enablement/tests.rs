use super::*;
use std::os::unix::fs::symlink;
use std::path::PathBuf;

struct EnablementFixture {
    root: tempfile::TempDir,
    unit: PathBuf,
    link: PathBuf,
}

impl EnablementFixture {
    fn new() -> Result<Self> {
        let root = tempfile::tempdir()?;
        let unit = root.path().join("qualification.service");
        fs::write(&unit, b"owned unit")?;
        let wants = root.path().join("multi-user.target.wants");
        fs::create_dir(&wants)?;
        let link = wants.join("qualification.service");
        Ok(Self { root, unit, link })
    }
}

#[test]
fn owned_relative_enablement_is_accepted_without_replacing_the_link() -> Result<()> {
    let fixture = EnablementFixture::new()?;
    let target = Path::new("../qualification.service");
    symlink(target, &fixture.link)?;
    persist(&fixture.link, &fixture.unit)?;
    assert_eq!(fs::read_link(&fixture.link)?, target);
    assert_eq!(
        fs::canonicalize(&fixture.link)?,
        fs::canonicalize(&fixture.unit)?
    );
    Ok(())
}

#[test]
fn missing_enablement_is_rejected_without_creating_a_replacement() -> Result<()> {
    let fixture = EnablementFixture::new()?;
    let error = persist(&fixture.link, &fixture.unit)
        .err()
        .context("missing boot enablement accepted")?;
    assert_eq!(
        error
            .downcast_ref::<std::io::Error>()
            .map(std::io::Error::kind),
        Some(std::io::ErrorKind::NotFound)
    );
    assert_eq!(
        fs::symlink_metadata(&fixture.link)
            .err()
            .map(|error| error.kind()),
        Some(std::io::ErrorKind::NotFound)
    );
    Ok(())
}

#[test]
fn regular_file_enablement_is_rejected_even_when_it_matches_the_owned_unit() -> Result<()> {
    let fixture = EnablementFixture::new()?;
    fs::copy(&fixture.unit, &fixture.link)?;
    persist(&fixture.link, &fixture.unit)
        .err()
        .context("regular file boot enablement accepted")?;
    assert!(fs::symlink_metadata(&fixture.link)?.file_type().is_file());
    assert_eq!(fs::read(&fixture.link)?, fs::read(&fixture.unit)?);
    Ok(())
}

#[test]
fn foreign_enablement_is_rejected_even_when_the_unit_contents_match() -> Result<()> {
    let fixture = EnablementFixture::new()?;
    let foreign = fixture.root.path().join("foreign.service");
    fs::copy(&fixture.unit, &foreign)?;
    symlink(&foreign, &fixture.link)?;
    persist(&fixture.link, &fixture.unit)
        .err()
        .context("foreign boot enablement accepted")?;
    assert_eq!(fs::read_link(&fixture.link)?, foreign);
    assert_ne!(
        fs::canonicalize(&fixture.link)?,
        fs::canonicalize(&fixture.unit)?
    );
    Ok(())
}

#[test]
fn dangling_enablement_is_rejected_without_repairing_the_target() -> Result<()> {
    let fixture = EnablementFixture::new()?;
    let target = fixture.root.path().join("absent.service");
    symlink(&target, &fixture.link)?;
    let error = persist(&fixture.link, &fixture.unit)
        .err()
        .context("dangling boot enablement accepted")?;
    assert_eq!(
        error
            .downcast_ref::<std::io::Error>()
            .map(std::io::Error::kind),
        Some(std::io::ErrorKind::NotFound)
    );
    assert_eq!(fs::read_link(&fixture.link)?, target);
    assert!(!target.exists());
    Ok(())
}
