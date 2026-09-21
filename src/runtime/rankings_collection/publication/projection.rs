use super::*;

/// Project one observation into its candidate entries, source rows and roster
/// observations in the order the page index records them.
pub(super) fn index_parts(
    observation: PageObservation,
) -> anyhow::Result<(
    Vec<RankingCandidateEntry>,
    Vec<RankingSourceRow>,
    Vec<RankingRosterObservation>,
)> {
    let PageObservation {
        grade_11_candidates_list,
        verified_relay_members,
        source_rows,
        ..
    } = observation;
    let individual = grade_11_candidates_list
        .into_iter()
        .enumerate()
        .map(|(index, candidate)| {
            Ok(RankingCandidateEntry {
                name: candidate.name,
                athlete_id: candidate.athlete_id,
                kind: RankingCandidateKind::Individual,
                record_index: u32::try_from(index)?,
                result_id: candidate.id_result,
            })
        });
    let relay = verified_relay_members
        .into_iter()
        .enumerate()
        .map(|(index, member)| {
            Ok(RankingCandidateEntry {
                name: member.name,
                athlete_id: member.athlete_id,
                kind: RankingCandidateKind::RelayMember,
                record_index: u32::try_from(index)?,
                result_id: member.id_result,
            })
        });
    Ok((
        individual
            .chain(relay)
            .collect::<anyhow::Result<Vec<_>>>()?,
        source_rows
            .iter()
            .map(|row| RankingSourceRow {
                result_id: row.result_id,
                row_number: row.row_number,
            })
            .collect(),
        source_rows
            .iter()
            .filter_map(|row| {
                row.roster_present.map(|present| RankingRosterObservation {
                    result_id: row.result_id,
                    present,
                })
            })
            .collect(),
    ))
}
