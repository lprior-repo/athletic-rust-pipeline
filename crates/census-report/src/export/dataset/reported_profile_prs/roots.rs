use super::{
    invalid, ReportedProfilePrs, ReportedProfileRootSchemaV1, ReportedProfileRootV1,
    MAX_PACKET_BYTES, MAX_ROOTS,
};
use crate::export::dataset::selected_proofs::objects;
use crate::report::ReportResult;
use serde::de::{Error, SeqAccess, Visitor};
use std::path::Path;

impl ReportedProfilePrs {
    pub(in crate::export::dataset) fn require_initial_objects(
        &self,
        directory: &Path,
    ) -> ReportResult<()> {
        self.validate_roots()?;
        if self.roots.is_empty() {
            return objects::require_empty(directory);
        }
        let mut expected = self
            .roots
            .iter()
            .map(|root| objects::Reference {
                digest: root.payload.digest.as_str().to_owned(),
                bytes: root.payload.bytes,
            })
            .collect::<Vec<_>>();
        expected.sort_unstable_by(|a, b| a.digest.cmp(&b.digest));
        expected.dedup();
        objects::verify(directory, &expected)
    }

    pub(super) fn validate_roots(&self) -> ReportResult<()> {
        if self.roots.len() > MAX_ROOTS {
            return Err(invalid("reported profile PR root count exceeds its bound"));
        }
        let mut previous: Option<&str> = None;
        let mut bytes = 0_u64;
        for root in &self.roots {
            if (root.schema == ReportedProfileRootSchemaV1::OwnedPublicProfileCaptureV1
                && root.key != root.payload.digest.as_str())
                || root.key.len() != 64
                || !root
                    .key
                    .bytes()
                    .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
                || root.payload.bytes == 0
                || root.payload.bytes > MAX_PACKET_BYTES
                || previous.is_some_and(|key| key >= root.key.as_str())
            {
                return Err(invalid(
                    "reported profile PR roots are invalid or noncanonical",
                ));
            }
            bytes = bytes
                .checked_add(root.payload.bytes)
                .ok_or_else(|| invalid("reported profile PR byte count overflow"))?;
            if bytes > objects::MAX_BYTES {
                return Err(invalid(
                    "reported profile PR source objects exceed the archive limit",
                ));
            }
            previous = Some(&root.key);
        }
        Ok(())
    }
}

pub(super) fn deserialize<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<ReportedProfileRootV1>, D::Error> {
    deserializer.deserialize_seq(Roots)
}

struct Roots;

impl<'de> Visitor<'de> for Roots {
    type Value = Vec<ReportedProfileRootV1>;

    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "at most {MAX_ROOTS} retained profile PR roots")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
        let mut roots = Vec::new();
        for _ in 0..MAX_ROOTS {
            let Some(root) = sequence.next_element()? else {
                return Ok(roots);
            };
            roots.try_reserve(1).map_err(A::Error::custom)?;
            roots.push(root);
        }
        if sequence.next_element::<serde::de::IgnoredAny>()?.is_some() {
            return Err(A::Error::custom(
                "reported profile PR root count exceeds its bound",
            ));
        }
        Ok(roots)
    }
}
