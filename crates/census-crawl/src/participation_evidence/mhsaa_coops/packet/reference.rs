use super::refused;
use census_store::StoreResult;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct MhsaaCoopPdfPacketRefV1(String);

impl MhsaaCoopPdfPacketRefV1 {
    pub fn parse(value: &str) -> StoreResult<Self> {
        valid_digest(value)?;
        let mut held = String::new();
        held.try_reserve_exact(value.len())
            .map_err(|_| refused("MHSAA PDF reference allocation refused"))?;
        held.push_str(value);
        Ok(Self(held))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub(super) fn valid_digest(value: &str) -> StoreResult<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(refused(
            "MHSAA PDF reference is not a lowercase SHA256 digest",
        ));
    }
    Ok(())
}
