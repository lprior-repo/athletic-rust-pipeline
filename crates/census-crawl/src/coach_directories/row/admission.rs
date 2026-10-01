use super::super::map::{CoachCounters, Row};
use super::super::parse::StaffMember;
use crate::row_hygiene;
use census_domain::model::{CoachRole, Gender, Sport};

pub(super) enum Rejection {
    Level,
    Person,
    Vendor,
}

struct RejectedContext<'a> {
    member: &'a StaffMember,
    sport: Option<Sport>,
    gender: Gender,
    role: CoachRole,
    level: String,
}

#[derive(Default)]
pub(super) struct AdmissionLedger<'a> {
    rejected: Vec<RejectedContext<'a>>,
}

impl<'a> AdmissionLedger<'a> {
    pub(super) fn record(
        &mut self,
        counters: &mut CoachCounters,
        row: &Row<'a>,
        level: Option<&str>,
        rejection: Rejection,
    ) {
        let label = row_hygiene::level_label(level);
        let duplicate = self.rejected.iter().any(|existing| {
            std::ptr::eq(existing.member, row.member)
                && existing.sport == row.sport
                && existing.gender == row.gender
                && existing.role == row.role
                && existing.level == label
        });
        if duplicate {
            return;
        }
        match rejection {
            Rejection::Level => {
                let slot = counters.dropped_levels.entry(label.clone()).or_insert(0);
                *slot = slot.saturating_add(1);
            }
            Rejection::Person => {
                counters.dropped_person = counters.dropped_person.saturating_add(1);
            }
            Rejection::Vendor => {
                counters.dropped_vendor = counters.dropped_vendor.saturating_add(1);
            }
        }
        self.rejected.push(RejectedContext {
            member: row.member,
            sport: row.sport,
            gender: row.gender,
            role: row.role,
            level: label,
        });
    }
}
