use crate::coach_directories::parse::StaffMember;
use census_domain::model::Sport;
use std::collections::hash_map::Entry;
use std::collections::HashMap;

pub fn dedup_staff(staff: &[StaffMember]) -> Vec<&StaffMember> {
    let mut unique: Vec<&StaffMember> = Vec::new();
    let mut index: HashMap<&str, usize> = HashMap::new();
    for member in staff {
        match index.entry(member.id.as_str()) {
            Entry::Occupied(slot) => {
                if let Some(cell) = unique.get_mut(*slot.get()) {
                    *cell = member;
                }
            }
            Entry::Vacant(slot) => {
                slot.insert(unique.len());
                unique.push(member);
            }
        }
    }
    unique
}

pub fn person_name(member: &StaffMember) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for part in [member.first_name.as_deref(), member.last_name.as_deref()] {
        if let Some(value) = part.map(str::trim).filter(|value| !value.is_empty()) {
            parts.push(value);
        }
    }
    parts.join(" ")
}

pub fn sport_family(sport: Option<Sport>) -> &'static str {
    match sport {
        Some(Sport::IndoorTrack | Sport::OutdoorTrack) => "Track",
        Some(Sport::CrossCountry) => "CrossCountry",
        None => "None",
    }
}
