use super::{archive::Archive, invalid, note::RowNote};
use census_crawl::milesplit::{self, LocatedRosterAthlete, TeamRef};
use census_domain::model::CanonicalSchool;
use census_store::{StoreError, StoreResult};
use std::collections::BTreeMap;

pub(super) struct CapturedRoster {
    pub members: BTreeMap<u32, LocatedRosterAthlete>,
    pub hashes: BTreeMap<u32, String>,
    pub metadata_ref: String,
}

pub(super) struct Captures {
    archive: Archive,
    retained: BTreeMap<String, Option<CapturedRoster>>,
    rows: usize,
}

impl Captures {
    #[cfg(test)]
    pub(super) fn new(root: &std::path::Path) -> StoreResult<Self> {
        Ok(Self::with_archive(Archive::new(root)?))
    }

    pub(super) fn with_archive(archive: Archive) -> Self {
        Self {
            archive,
            retained: BTreeMap::new(),
            rows: 0,
        }
    }

    pub(super) fn retention(self) -> Option<super::frozen::assets::Sink> {
        self.archive.retention()
    }

    pub(super) fn get(
        &mut self,
        school: &CanonicalSchool,
        note: &RowNote,
    ) -> StoreResult<Option<&CapturedRoster>> {
        let key = census_domain::model::serialized_digest(&(
            &school.id,
            &note.capture_sha256,
            &note.capture_url,
            &note.response_url,
            note.capture_bytes,
            &note.fetched_at,
        ))?;
        if !self.retained.contains_key(&key) {
            if self.retained.len() >= 4096 {
                return Err(invalid(
                    "current affiliation capture inventory exceeds 4096 objects",
                ));
            }
            let parsed = self.load(school, note)?;
            self.retained.insert(key.clone(), parsed);
        }
        Ok(self.retained.get(&key).and_then(Option::as_ref))
    }

    fn load(
        &mut self,
        school: &CanonicalSchool,
        note: &RowNote,
    ) -> StoreResult<Option<CapturedRoster>> {
        let Some((capture, metadata_ref)) = self.archive.capture(note)? else {
            return Ok(None);
        };
        let team = retained_team(school, note)?;
        let body = std::str::from_utf8(&capture.body)
            .map_err(|_| invalid("retained roster body is not UTF-8"))?;
        let parsed = milesplit::parse_captured_roster(body, team, &capture).map_err(|error| {
            invalid(&format!(
                "retained roster ownership or parse refused: {error}"
            ))
        })?;
        let roster = match parsed {
            milesplit::RosterVerdict::Complete { roster }
            | milesplit::RosterVerdict::Partial { roster, .. } => roster,
            milesplit::RosterVerdict::Quarantined { .. } => {
                return Err(invalid("retained roster has no admitted rows"))
            }
        };
        self.rows = self
            .rows
            .checked_add(roster.athletes.len())
            .ok_or(StoreError::CounterOverflow)?;
        if self.rows > 200_000 {
            return Err(invalid(
                "current affiliation parsed roster inventory exceeds 200000 rows",
            ));
        }
        let mut hashes = BTreeMap::new();
        for member in &roster.athletes {
            let row = member.locator();
            let end = row
                .byte_offset
                .checked_add(row.byte_length)
                .ok_or(StoreError::CounterOverflow)?;
            let span = capture
                .body
                .get(row.byte_offset..end)
                .ok_or_else(|| invalid("retained roster row lies outside capture"))?;
            hashes.insert(row.ordinal, super::archive::hash(span));
        }
        let members = roster
            .athletes
            .into_iter()
            .map(|member| (member.locator().ordinal, member))
            .collect();
        Ok(Some(CapturedRoster {
            members,
            hashes,
            metadata_ref,
        }))
    }
}

fn retained_team(school: &CanonicalSchool, note: &RowNote) -> StoreResult<TeamRef> {
    let url = note
        .capture_url
        .strip_suffix("/roster")
        .ok_or_else(|| invalid("retained roster request has no roster path"))?;
    let team = TeamRef {
        id: note.provider_team_id.clone(),
        slug: String::new(),
        url: url.to_string(),
        name: school.name.clone(),
        city_state: school
            .state
            .map_or(String::new(), |state| state.code().to_string()),
    };
    Ok(team)
}
