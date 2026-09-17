use crate::discovery::allowed_profile_url;
use anyhow::{bail, Context, Result};
use std::{fs::File, io::Read, path::Path};

const MAX_PROFILE_BYTES: usize = 4 * 1024 * 1024;

pub fn load_saved_profile(url: &str, directory: &Path) -> Result<Option<String>> {
    let normalized = allowed_profile_url(url).context("candidate URL is outside the allow-list")?;
    let parsed = url::Url::parse(&normalized)?;
    let identifier = parsed.path_segments().and_then(|segments| {
        let mut after_athlete = segments.skip_while(|segment| *segment != "athlete");
        after_athlete.next()?;
        after_athlete.next()
    });
    let Some(identifier) = identifier else {
        return Ok(None);
    };
    if identifier.is_empty()
        || !identifier
            .chars()
            .all(|character| character.is_ascii_digit())
    {
        return Ok(None);
    }
    let path = directory.join(format!("{identifier}.html"));
    let mut file = match File::open(&path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("opening manually saved profile {}", path.display()));
        }
    };
    let read_limit = u64::try_from(MAX_PROFILE_BYTES)?
        .checked_add(1)
        .context("saved profile size limit overflow")?;
    let mut html = String::new();
    file.by_ref()
        .take(read_limit)
        .read_to_string(&mut html)
        .with_context(|| format!("reading manually saved profile {}", path.display()))?;
    if html.len() > MAX_PROFILE_BYTES {
        bail!("saved profile exceeds 4 MiB evidence limit");
    }
    Ok(Some(html))
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_saved_profile_is_not_an_input_error() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let result = load_saved_profile(
            "https://www.athletic.net/athlete/123/track-and-field",
            directory.path(),
        )?;
        assert_eq!(result, None);
        Ok(())
    }

    #[test]
    fn oversized_saved_profile_is_rejected_before_returning_html() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("123.html");
        std::fs::write(&path, vec![b'x'; MAX_PROFILE_BYTES + 1])?;
        let error = match load_saved_profile(
            "https://www.athletic.net/athlete/123/track-and-field",
            directory.path(),
        ) {
            Ok(_) => anyhow::bail!("oversized saved profile unexpectedly loaded"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("exceeds 4 MiB"));
        Ok(())
    }
}
