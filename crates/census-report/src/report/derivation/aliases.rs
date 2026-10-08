use census_domain::model::{CanonicalAthlete, Gender};
use std::collections::{BTreeMap, HashMap};

type AliasGroups<'a> = BTreeMap<&'a str, Vec<&'a CanonicalAthlete>>;

pub(in crate::report) fn collapse_athletes(
    athletes: &[CanonicalAthlete],
    aliases: &HashMap<String, String>,
) -> Vec<CanonicalAthlete> {
    if aliases.is_empty() {
        return athletes.to_vec();
    }
    let groups = group_aliases(athletes, aliases);
    let capacity = groups.len();
    groups
        .into_iter()
        .fold(Vec::with_capacity(capacity), append_group)
}

fn append_group(
    mut collapsed: Vec<CanonicalAthlete>,
    (canonical, members): (&str, Vec<&CanonicalAthlete>),
) -> Vec<CanonicalAthlete> {
    match members
        .iter()
        .find(|member| member.id.as_str() == canonical)
    {
        Some(representative) => collapsed.push(union_accepted_members(representative, &members)),
        None => collapsed.extend(members.into_iter().cloned()),
    }
    collapsed
}

fn group_aliases<'a>(
    athletes: &'a [CanonicalAthlete],
    aliases: &'a HashMap<String, String>,
) -> AliasGroups<'a> {
    athletes
        .iter()
        .fold(AliasGroups::new(), |mut groups, athlete| {
            let canonical = aliases
                .get(athlete.id.as_str())
                .map_or(athlete.id.as_str(), String::as_str);
            groups.entry(canonical).or_default().push(athlete);
            groups
        })
}

fn union_accepted_members(
    representative: &CanonicalAthlete,
    members: &[&CanonicalAthlete],
) -> CanonicalAthlete {
    let mut row = representative.clone();
    resolve_gender(&mut row, members);
    members
        .iter()
        .filter(|member| member.id != representative.id)
        .for_each(|member| {
            union_member(&mut row, member);
        });
    row
}

fn resolve_gender(row: &mut CanonicalAthlete, members: &[&CanonicalAthlete]) {
    let known_gender = members
        .iter()
        .map(|member| member.gender)
        .filter(|gender| *gender != Gender::Unknown);
    if row.gender == Gender::Unknown {
        let mut genders = known_gender;
        if let Some(first) = genders.next() {
            if genders.all(|gender| gender == first) {
                row.gender = first;
            }
        }
    }
}

fn union_member(row: &mut CanonicalAthlete, member: &CanonicalAthlete) {
    union_values(&mut row.known_names, &member.known_names);
    union_values(
        &mut row.known_names,
        std::slice::from_ref(&member.canonical_name),
    );
    union_values(&mut row.sports, &member.sports);
    union_values(&mut row.public_profile_urls, &member.public_profile_urls);
    union_values(&mut row.observed_grades, &member.observed_grades);
    union_values(
        &mut row.published_graduations,
        &member.published_graduations,
    );
    union_values(&mut row.evidence, &member.evidence);
    union_values(&mut row.retained_conflicts, &member.retained_conflicts);
    member.identities().for_each(|identity| {
        if let Some(url) = &identity.url {
            union_values(&mut row.public_profile_urls, std::slice::from_ref(url));
        }
        row.add_identity(identity.clone());
    });
}

fn union_values<T: PartialEq + Clone>(target: &mut Vec<T>, values: &[T]) {
    values.iter().for_each(|value| {
        if !target.contains(value) {
            target.push(value.clone());
        }
    });
}
