use super::{CoopMemberCandidate, RelationshipCandidate};
use crate::model::SchoolId;

pub(super) fn school<'a>(relationship: RelationshipCandidate<'a>) -> Option<&'a SchoolId> {
    match relationship {
        RelationshipCandidate::EnrolledAtSchool(school)
        | RelationshipCandidate::RepresentsSchoolProgram { school, .. } => Some(school),
        RelationshipCandidate::RepresentsCoopProgram {
            member: CoopMemberCandidate::Claimed { school, .. },
            ..
        } => Some(school),
        RelationshipCandidate::RepresentsCoopProgram {
            member: CoopMemberCandidate::Unknown,
            ..
        }
        | RelationshipCandidate::NonSchool(_) => None,
    }
}

pub(super) fn same_relationship(
    left: RelationshipCandidate<'_>,
    right: RelationshipCandidate<'_>,
) -> bool {
    match (left, right) {
        (
            RelationshipCandidate::EnrolledAtSchool(a),
            RelationshipCandidate::EnrolledAtSchool(b),
        ) => a == b,
        (
            RelationshipCandidate::RepresentsSchoolProgram {
                school: a,
                program: x,
            },
            RelationshipCandidate::RepresentsSchoolProgram {
                school: b,
                program: y,
            },
        ) => a == b && x == y,
        (
            RelationshipCandidate::RepresentsCoopProgram { program: x, .. },
            RelationshipCandidate::RepresentsCoopProgram { program: y, .. },
        ) => x == y && school(left) == school(right),
        (RelationshipCandidate::NonSchool(a), RelationshipCandidate::NonSchool(b)) => a == b,
        _ => false,
    }
}
