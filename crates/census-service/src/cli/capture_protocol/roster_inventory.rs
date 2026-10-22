use anyhow::{Context, Result};
use census_crawl::participation_evidence::provider_school::{
    ProviderSchoolPacketRefV1, OWNED_PROVIDER_SCHOOL_INTAKE_PHASE,
    OWNED_PROVIDER_SCHOOL_INTERPRETATION,
};
use census_domain::model::{
    CanonicalSchool, CanonicalTeam, EvidenceMethod, Gender, SourceNamespace, Sport, StoreIdentity,
};
use census_store::{ReadBudget, ReadCounts, Store, StoreError, StoreResult, Table};
use serde::Serialize;
use std::collections::BTreeMap;
use std::io::Write;

mod output;
mod owners;
use output::emit;
use owners::ProviderOwners;
use std::path::Path;

const SCHOOL_BUDGET: ReadBudget = ReadBudget {
    rows: 65_536,
    bytes: 64 * 1024 * 1024,
    record_bytes: 1024 * 1024,
};
const TEAM_BUDGET: ReadBudget = ReadBudget {
    rows: 131_072,
    bytes: 128 * 1024 * 1024,
    record_bytes: 1024 * 1024,
};
const MAX_PACKET_REFS: u64 = 65_536;
const MAX_NOTE_BYTES: usize = 64 * 1024;
const MAX_OUTPUT_BYTES: usize = 32 * 1024 * 1024;

#[derive(Serialize)]
struct Envelope<'a> {
    schema: &'static str,
    store_identity: StoreIdentity,
    snapshot_sequence: u64,
    school_readcounts: Counts,
    team_readcounts: Counts,
    school_count: u64,
    packet_count: u64,
    program_count: u64,
    schools: &'a BTreeMap<String, SchoolInventory>,
}

#[derive(Serialize)]
struct Counts {
    key_visits: u64,
    decoded_rows: u64,
    encoded_bytes: u64,
    merged_rows: u64,
}

impl From<ReadCounts> for Counts {
    fn from(value: ReadCounts) -> Self {
        Self {
            key_visits: value.key_visits,
            decoded_rows: value.decoded_rows,
            encoded_bytes: value.encoded_bytes,
            merged_rows: value.merged_rows,
        }
    }
}

#[derive(Serialize)]
struct SchoolInventory {
    school_id: String,
    school_name: String,
    jurisdiction: Option<String>,
    provider_namespace: SourceNamespace,
    #[serde(flatten)]
    owner: ProviderOwners,
    retained_conflicts: usize,
    packets: BTreeMap<String, PacketHint>,
    programs: Vec<Program>,
}

#[derive(PartialEq, Eq, Serialize)]
struct PacketHint {
    evidence_url_hint: Option<String>,
    evidence_observed_at_hint: String,
}

#[derive(Serialize)]
struct Program {
    program_id: String,
    sport: Sport,
    category: Gender,
    school_year: i16,
    retained_conflicts: usize,
    provider_namespace: SourceNamespace,
    #[serde(flatten)]
    owner: ProviderOwners,
    provider_join: &'static str,
}

pub(super) fn run_to<W: Write>(root: &Path, output: &mut W) -> Result<()> {
    let (store, _writer) = Store::open_current_qualification_v3(root)
        .context("opening the stopped current-qualification store")?
        .into_parts();
    let view = store.snapshot();
    let mut schools = BTreeMap::new();
    let mut packet_count = 0;
    let school_counts = view
        .for_each_merged_bounded::<CanonicalSchool>(Table::Schools, SCHOOL_BUDGET, |school, _| {
            collect_school(school, &mut schools, &mut packet_count)
        })
        .context("reading bounded unreviewed school packet hints")?;
    let mut program_count = 0;
    let team_counts = view
        .for_each_merged_bounded::<CanonicalTeam>(Table::Teams, TEAM_BUDGET, |team, _| {
            collect_team(team, &mut schools, &mut program_count)
        })
        .context("reading bounded retained program bindings")?;
    let envelope = Envelope {
        schema: "unreviewed_provider_roster_inventory_v1",
        store_identity: view.store_identity()?,
        snapshot_sequence: view.sequence(),
        school_readcounts: school_counts.into(),
        team_readcounts: team_counts.into(),
        school_count: u64::try_from(schools.len()).map_err(|_| StoreError::CounterOverflow)?,
        packet_count,
        program_count,
        schools: &schools,
    };
    emit(output, &envelope, MAX_OUTPUT_BYTES)
}

fn collect_school(
    school: CanonicalSchool,
    schools: &mut BTreeMap<String, SchoolInventory>,
    count: &mut u64,
) -> StoreResult<()> {
    let packets = collect_school_packets(&school, count)?;
    if packets.is_empty() {
        return Ok(());
    }
    let owner = ProviderOwners::new(&school.source_identities, &SourceNamespace::MilesplitSchool);
    let id = school.id.as_str().to_owned();
    if schools.contains_key(&id) {
        return Err(refused("duplicate merged school in inventory"));
    }
    schools.insert(
        id.clone(),
        SchoolInventory {
            school_id: id,
            school_name: school.name,
            jurisdiction: school.state.map(|s| s.code().to_owned()),
            provider_namespace: SourceNamespace::MilesplitSchool,
            owner,
            retained_conflicts: school.retained_conflicts.len(),
            packets,
            programs: Vec::new(),
        },
    );
    Ok(())
}

fn collect_school_packets(
    school: &CanonicalSchool,
    count: &mut u64,
) -> StoreResult<BTreeMap<String, PacketHint>> {
    let mut packets: BTreeMap<String, PacketHint> = BTreeMap::new();
    for evidence in &school.evidence {
        if evidence.method != EvidenceMethod::Parsed {
            continue;
        }
        let Some(note) = evidence.note.as_deref() else {
            continue;
        };
        let Some(packet) = parse_known_note(note)? else {
            continue;
        };
        if let Some(previous) = packets.get(&packet) {
            if previous.evidence_url_hint != evidence.source.url
                || previous.evidence_observed_at_hint != evidence.observed_on
            {
                return Err(refused("same school packet has conflicting evidence hints"));
            }
            continue;
        }
        let next = next_count(
            *count,
            MAX_PACKET_REFS,
            "roster inventory packet references",
        )?;
        packets.insert(
            packet,
            PacketHint {
                evidence_url_hint: evidence.source.url.clone(),
                evidence_observed_at_hint: evidence.observed_on.clone(),
            },
        );
        *count = next;
    }
    Ok(packets)
}

fn collect_team(
    team: CanonicalTeam,
    schools: &mut BTreeMap<String, SchoolInventory>,
    count: &mut u64,
) -> StoreResult<()> {
    let Some(school) = schools.get_mut(team.school.as_str()) else {
        return Ok(());
    };
    let owner = ProviderOwners::new(&team.source_identities, &SourceNamespace::MilesplitTeam);
    let provider_join = school.owner.join(&owner);
    let next = next_count(
        *count,
        TEAM_BUDGET.rows,
        "roster inventory retained programs",
    )?;
    school
        .programs
        .try_reserve(1)
        .map_err(|_| refused("program inventory allocation refused"))?;
    school.programs.push(Program {
        program_id: team.id.as_str().to_owned(),
        sport: team.sport,
        category: team.gender,
        school_year: team.school_year.get(),
        retained_conflicts: team.retained_conflicts.len(),
        provider_namespace: SourceNamespace::MilesplitTeam,
        owner,
        provider_join,
    });
    *count = next;
    Ok(())
}

fn parse_known_note(note: &str) -> StoreResult<Option<String>> {
    if !note.contains(OWNED_PROVIDER_SCHOOL_INTAKE_PHASE)
        && !note.contains(OWNED_PROVIDER_SCHOOL_INTERPRETATION)
    {
        return Ok(None);
    }
    if note.len() > MAX_NOTE_BYTES {
        return Err(StoreError::RowTooLarge {
            bytes: note.len(),
            max: MAX_NOTE_BYTES,
        });
    }
    let value: serde_json::Value = serde_json::from_str(note)
        .map_err(|_| refused("recognized owned school evidence note is malformed"))?;
    let field = |name| value.get(name).and_then(serde_json::Value::as_str);
    if field("role") != Some("provider_roster_owner")
        || field("phase") != Some(OWNED_PROVIDER_SCHOOL_INTAKE_PHASE)
        || field("interpretation") != Some(OWNED_PROVIDER_SCHOOL_INTERPRETATION)
    {
        return Ok(None);
    }
    let packet =
        field("packet").ok_or_else(|| refused("owned school note lacks packet reference"))?;
    ProviderSchoolPacketRefV1::parse(packet)
        .map_err(|_| refused("owned school note has invalid packet reference"))?;
    Ok(Some(packet.to_owned()))
}

fn next_count(current: u64, available: u64, what: &'static str) -> StoreResult<u64> {
    let needed = current.checked_add(1).ok_or(StoreError::CounterOverflow)?;
    if needed > available {
        return Err(StoreError::BudgetExceeded {
            what,
            needed,
            available,
        });
    }
    Ok(needed)
}

fn refused(detail: &str) -> StoreError {
    StoreError::Refused {
        detail: detail.to_owned(),
    }
}

#[cfg(test)]
mod tests;
