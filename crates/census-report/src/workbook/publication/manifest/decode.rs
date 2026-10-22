use serde::{Deserialize, Deserializer};

#[derive(Deserialize)]
#[serde(remote = "super::Manifest", deny_unknown_fields)]
struct Fields {
    schema_revision: u32,
    lineage: crate::export::DatasetLineage,
    selection: super::Selection,
    artifacts: std::collections::BTreeMap<String, super::Artifact>,
    #[serde(default, deserialize_with = "super::counts::decode")]
    normalized_csv_rows: Option<super::super::inventory::RowCounts>,
    #[serde(default, deserialize_with = "selected_profile")]
    selected_profile: Option<super::super::selected::SelectedProfile>,
    generation_digest: String,
}

fn selected_profile<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<super::super::selected::SelectedProfile>, D::Error> {
    super::super::selected::SelectedProfile::deserialize(deserializer).map(Some)
}

impl<'de> Deserialize<'de> for super::Manifest {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let manifest = Fields::deserialize(deserializer)?;
        if manifest.schema_revision <= 6
            && manifest
                .normalized_csv_rows
                .as_ref()
                .is_some_and(|rows| rows.len() > 9)
        {
            return Err(serde::de::Error::custom(
                "normalized CSV count inventory exceeds nine entries",
            ));
        }
        if manifest.schema_revision <= 8
            && manifest
                .normalized_csv_rows
                .as_ref()
                .is_some_and(|rows| rows.len() > 11)
        {
            return Err(serde::de::Error::custom(
                "normalized CSV count inventory exceeds eleven entries",
            ));
        }
        if manifest.schema_revision <= 12
            && manifest
                .normalized_csv_rows
                .as_ref()
                .is_some_and(|rows| rows.len() > 12)
        {
            return Err(serde::de::Error::custom(
                "normalized CSV count inventory exceeds twelve entries",
            ));
        }
        super::super::selected::validate(
            manifest.schema_revision,
            manifest.selected_profile.as_ref(),
        )
        .map_err(serde::de::Error::custom)?;
        Ok(manifest)
    }
}
