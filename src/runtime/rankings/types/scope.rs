use crate::runtime::rankings::catalog::RequestedFamily;
use crate::runtime::rankings::division::{
    self, expected_revision, normalize_gender, season_list_id, SeasonKind,
};
use serde::{Deserialize, Serialize};

/// Rankings scope: immutable collection parameters derived from scope.
///
/// One scope covers exactly one seasonal division (season kind + gender).
/// `list_id` and `revision` are evidence-backed: see `division`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingsScope {
    pub revision: String,
    /// Absent in scopes written before the division split, which were
    /// outdoor-only; those revisions are retired rather than reinterpreted
    /// (see `division::expected_revision`).
    #[serde(default)]
    pub season_kind: SeasonKind,
    pub list_id: u64,
    pub season: u64,
    pub gender: String,
    pub projection_grade: u8,
    pub country: String,
    pub level: u64,
    pub max_pages_per_event: u32,
    pub requested_families: Vec<RequestedFamily>,
}

impl RankingsScope {
    /// Construct the canonical 2026 USA outdoor boys Grade 11 scope with all 46
    /// requested event families. The caller supplies only the per-event
    /// page cap; everything else is fixed by the specification.
    pub fn requested(max_pages_per_event: u32) -> anyhow::Result<Self> {
        Self::for_division(SeasonKind::Outdoor, "m", max_pages_per_event)
    }

    /// Construct one seasonal division scope for the given source request
    /// gender code (`"m"` or `"f"`). Everything else is fixed by the
    /// specification and validated against the captured source contract.
    pub fn for_division(
        season_kind: SeasonKind,
        gender: &str,
        max_pages_per_event: u32,
    ) -> anyhow::Result<Self> {
        if max_pages_per_event == 0 || max_pages_per_event > 10_000 {
            anyhow::bail!("max_pages_per_event must be 1..=10000");
        }
        let gender = normalize_gender(gender)
            .ok_or_else(|| anyhow::anyhow!("unsupported gender: {gender}"))?;
        let list_id = season_list_id(season_kind, division::SEASON_YEAR)
            .ok_or_else(|| anyhow::anyhow!("unsupported season: {season_kind:?}"))?;
        let revision = expected_revision(season_kind, gender)
            .ok_or_else(|| anyhow::anyhow!("unsupported gender: {gender}"))?;
        let scope = Self {
            revision,
            season_kind,
            list_id,
            season: division::SEASON_YEAR,
            gender: gender.to_owned(),
            projection_grade: 11,
            country: "USA".to_owned(),
            level: 4,
            max_pages_per_event,
            requested_families: Self::default_families(),
        };
        scope.validate()?;
        Ok(scope)
    }

    /// Source season identifier this scope binds to: the kind-specific value
    /// the captured nav keys its `seasons` map by (outdoor `2026`, indoor
    /// `12026`), which is also the `SeasonID` a division page must report.
    pub fn source_season_id(&self) -> u64 {
        self.season_kind.season_id(self.season)
    }

    /// Only an evidence-backed source division scope may enter the durable
    /// collection: list id, season, gender, and revision must agree.
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.country != "USA" {
            anyhow::bail!("unsupported country: {}", self.country);
        }
        if self.level != 4 {
            anyhow::bail!("unsupported level: {}", self.level);
        }
        if self.season != division::SEASON_YEAR {
            anyhow::bail!("unsupported season: {}", self.season);
        }
        let gender = normalize_gender(&self.gender)
            .ok_or_else(|| anyhow::anyhow!("unsupported gender: {}", self.gender))?;
        if self.projection_grade != 11 {
            anyhow::bail!("unsupported projection_grade: {}", self.projection_grade);
        }
        if self.max_pages_per_event == 0 || self.max_pages_per_event > 10_000 {
            anyhow::bail!("max_pages_per_event must be 1..=10000");
        }
        let expected_list = season_list_id(self.season_kind, self.season).ok_or_else(|| {
            anyhow::anyhow!("unsupported season kind or season: {:?}", self.season_kind)
        })?;
        if self.list_id != expected_list {
            anyhow::bail!(
                "{} {} division list must be {expected_list}, got {}",
                self.season_kind.label(),
                self.season,
                self.list_id
            );
        }
        let expected = expected_revision(self.season_kind, gender)
            .ok_or_else(|| anyhow::anyhow!("unsupported gender for revision: {}", self.gender))?;
        if self.revision != expected {
            anyhow::bail!("unsupported ranking scope revision: {}", self.revision);
        }
        let manifest_matches = self.requested_families.len() == REQUESTED_FAMILY_MANIFEST.len()
            && self
                .requested_families
                .iter()
                .zip(REQUESTED_FAMILY_MANIFEST)
                .all(|(actual, (group, family, short))| {
                    actual.group == group && actual.family == family && actual.short == short
                });
        if !manifest_matches {
            anyhow::bail!("rankings scope must include the complete requested family manifest");
        }
        Ok(())
    }

    /// The 46 requested families for the 2026 USA high-school Grade 11 scope.
    /// The catalog expands these dynamically into the observed variants.
    fn default_families() -> Vec<RequestedFamily> {
        REQUESTED_FAMILY_MANIFEST
            .iter()
            .map(|(group, family, short)| RequestedFamily {
                group: (*group).to_owned(),
                family: (*family).to_owned(),
                short: (*short).to_owned(),
            })
            .collect()
    }
}

const REQUESTED_FAMILY_MANIFEST: [(&str, &str, &str); 46] = [
    ("sprint", "55m", "55m"),
    ("sprint", "60m", "60m"),
    ("sprint", "100m", "100m"),
    ("sprint", "200m", "200m"),
    ("sprint", "300m", "300m"),
    ("sprint", "400m", "400m"),
    ("middle", "500m", "500m"),
    ("middle", "600m", "600m"),
    ("middle", "800m", "800m"),
    ("middle", "1000m", "1000m"),
    ("distance", "1500m", "1500m"),
    ("distance", "1600m", "1600m"),
    ("distance", "mile", "1mile"),
    ("distance", "2 mile", "2miles"),
    ("distance", "3000m", "3000m"),
    ("distance", "3200m", "3200m"),
    ("steeplechase", "2000mSC", "2ksteeple"),
    ("steeplechase", "3000mSC", "3ksteeple"),
    ("hurdles", "55mH", "55mh"),
    ("hurdles", "60mH", "60mh"),
    ("hurdles", "100mH", "100mh"),
    ("hurdles", "110mH", "110mh"),
    ("hurdles", "300mH", "300mh"),
    ("hurdles", "400mH", "400mh"),
    ("relay", "4x100", "4x100m"),
    ("relay", "4x200", "4x200m"),
    ("relay", "4x400", "4x400m"),
    ("relay", "4x800", "4x800m"),
    ("relay", "4x1600", "4x1600m"),
    ("relay", "4×mile", "4xmile"),
    ("relay", "sprint medley", ""),
    ("relay", "distance medley", ""),
    ("relay", "swedish relay", ""),
    ("jump", "high jump", "hj"),
    ("relay", "shuttle hurdle relay", ""),
    ("jump", "long jump", "lj"),
    ("jump", "triple jump", "tj"),
    ("jump", "pole vault", "pv"),
    ("throw", "shot put", "shot"),
    ("throw", "discus", "discus"),
    ("throw", "javelin", "javelin"),
    ("throw", "hammer", "hammer"),
    ("throw", "weight throw", "weight"),
    ("combined", "pentathlon", ""),
    ("combined", "heptathlon", "heptathlon"),
    ("combined", "decathlon", "decathlon"),
];
