use super::limits;
use census_domain::school_directory::DirectoryError;
use std::fmt::{self, Write};

#[derive(Default)]
struct Detail {
    text: String,
    failure: Option<DirectoryError>,
}

impl Write for Detail {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        let result = self.append(text);
        match result {
            Ok(()) => Ok(()),
            Err(error) => {
                self.failure = Some(error);
                Err(fmt::Error)
            }
        }
    }
}

impl Detail {
    fn append(&mut self, text: &str) -> Result<(), DirectoryError> {
        let next = limits::add(self.text.len(), text.len())?;
        limits::check(
            "directory issue detail bytes",
            next,
            limits::MAX_DETAIL_BYTES,
        )?;
        self.text
            .try_reserve(text.len())
            .map_err(|_| DirectoryError::Allocation {
                resource: "directory issue detail",
            })?;
        self.text.push_str(text);
        Ok(())
    }
}

pub fn issue_detail(args: fmt::Arguments<'_>) -> Result<String, DirectoryError> {
    let mut detail = Detail::default();
    let result = fmt::write(&mut detail, args);
    if let Some(error) = detail.failure {
        return Err(error);
    }
    result.map_err(|_| DirectoryError::Representation {
        detail: "formatting directory issue failed".into(),
    })?;
    Ok(detail.text)
}
