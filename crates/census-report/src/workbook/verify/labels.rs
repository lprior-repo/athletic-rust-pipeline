use crate::bests::{SharedSelection, SurfaceClass};

pub(super) const ATHLETES: &str = "Athletes";

pub(super) const PRS: &str = "PRs";

pub(super) const COACHES: &str = "Coaches";

pub(super) const META_SHEETS: [&str; 9] = [
    "Schools",
    "Meets",
    "Sources",
    "Coverage",
    "Conflicts",
    "Review",
    "Run Metrics",
    "School Contacts",
    "Contact Research",
];

pub(super) const PERFORMANCE_HEADERS: [&str; 20] = [
    "Canonical Result ID",
    "Athlete ID",
    "Athlete",
    "School",
    "Graduation Year",
    "Meet ID",
    "Meet",
    "Date",
    "State",
    "Sport",
    "Event",
    "Mark",
    "Normalized Mark",
    "Timing",
    "Wind",
    "Round",
    "Place",
    "Source",
    "Source ResultID",
    "Source URL",
];

pub(super) const ATHLETE_HEADERS: [&str; 64] = [
    "Athlete ID",
    "Name",
    "Gender",
    "Graduation Year",
    "Observed School Year",
    "State",
    "School",
    "School ID",
    "School City",
    "TF",
    "XC",
    "Indoor",
    "Outdoor",
    "Event list",
    "Headline PR summary",
    "100m (s)",
    "200m (s)",
    "400m (s)",
    "800m (s)",
    "1600m (s)",
    "3200m (s)",
    "1 Mile (s)",
    "5000m (s)",
    "100m Hurdles (s)",
    "110m Hurdles (s)",
    "300m Hurdles (s)",
    "High Jump (m)",
    "Long Jump (m)",
    "Triple Jump (m)",
    "Pole Vault (m)",
    "Shot Put (m)",
    "Discus (m)",
    "Javelin (m)",
    "XC (s)",
    "Performance count",
    "Meet count",
    "Head TF Coach",
    "Head TF Coach Email",
    "Head XC Coach",
    "Head XC Coach Email",
    "Coach Professional Email",
    "Athletic Director",
    "AD Email",
    "School Athletics URL",
    "Public Recruiting GPA",
    "GPA Source",
    "All Emails (school)",
    "Preferred Recruiting Contact",
    "Preferred Contact Role",
    "Preferred Contact Email",
    "Contact Coverage State",
    "Preferred Contact Coach ID",
    "Preferred Contact Source URL",
    "Preferred Contact Capture SHA256",
    "Preferred Contact Acquired At",
    "Athletic.net URL",
    "MileSplit URL",
    "Other profile URLs",
    "Sources Count",
    "Identity Status",
    "Coverage State",
    "Conflict Flag",
    "Review Status",
    crate::export::postal::ATHLETE_ADDRESS_HEADER,
];

pub(super) const PR_HEADERS: [&str; 29] = [
    "Athlete ID",
    "Athlete",
    "Gender",
    "School",
    "State",
    "Graduation Year",
    "Sport",
    "Event",
    "Season",
    "Calculated PR",
    "Mark Value",
    "Unit",
    "Wind",
    "PR date",
    "Meet",
    "Place",
    "Result URL",
    "Source count",
    "PR conflict",
    "Surface",
    "Wind Class",
    "Timing Class",
    "Performance ID",
    "Meet ID",
    "Source Key",
    "Event Context",
    "Published mark",
    "Conversion basis",
    "Conversion note",
];

pub(super) const COACH_HEADERS: [&str; 29] = [
    "School ID",
    "School",
    "School City",
    "State",
    "Sport",
    "Coach",
    "Coach ID",
    "Gender",
    "Role",
    "Professional Email",
    "Personal Email",
    "Phone",
    "Athletic Director",
    "AD Professional Email",
    "Official Source URL",
    "Observed Date",
    "Declared Tenure",
    "Assessment School Year",
    "Professional Email Source URL",
    "Professional Email Capture SHA256",
    "Professional Email Acquired At",
    "Personal Email Source URL",
    "Personal Email Capture SHA256",
    "Personal Email Acquired At",
    "Name Capture SHA256",
    "AD Email Source URL",
    "AD Email Capture SHA256",
    "AD Email Acquired At",
    "Contact Admission State",
];

pub(super) const PR_EVENTS: [&str; 19] = [
    "Track100m",
    "Track200m",
    "Track400m",
    "Track800m",
    "Track1600m",
    "Track3200m",
    "Track1Mile",
    "Track5000m",
    "Track100mHurdles",
    "Track110mHurdles",
    "Track300mHurdles",
    "HighJump",
    "LongJump",
    "TripleJump",
    "PoleVault",
    "ShotPut",
    "Discus",
    "Javelin",
    "CrossCountry",
];

pub(super) fn pr_event_name(key: &str) -> &str {
    match key {
        "Track100m" => "100m",
        "Track200m" => "200m",
        "Track400m" => "400m",
        "Track800m" => "800m",
        "Track1600m" => "1600m",
        "Track3200m" => "3200m",
        "Track1Mile" => "1 Mile",
        "Track5000m" => "5000m",
        "Track100mHurdles" => "100m H",
        "Track110mHurdles" => "110m H",
        "Track300mHurdles" => "300m H",
        "HighJump" => "High Jump",
        "LongJump" => "Long Jump",
        "TripleJump" => "Triple Jump",
        "PoleVault" => "Pole Vault",
        "ShotPut" => "Shot Put",
        "Discus" => "Discus",
        "Javelin" => "Javelin",
        "CrossCountry" => "XC",
        other => other,
    }
}

pub(super) fn mark_name<'a>(pr: &SharedSelection, event: &'a str) -> &'a str {
    if pr.key.surface == SurfaceClass::CrossCountry {
        "XC"
    } else {
        pr_event_name(event)
    }
}

pub(super) fn qualified_mark(pr: &SharedSelection) -> String {
    let mut text = String::new();
    text.push_str(mark_name(pr, &pr.key.event_kind.stable_key()));
    text.push(' ');
    text.push_str(&pr.mark_text());
    text.push_str(" [");
    text.push_str(pr.key.surface.label());
    text.push_str(", ");
    text.push_str(pr.key.wind_class.label());
    text.push_str(", ");
    text.push_str(pr.key.timing.label());
    if let Some(context) = &pr.key.context {
        text.push_str(", event ");
        text.push_str(context.as_str());
    }
    if let Some(context) = super::context::cross_country(pr) {
        text.push_str(", ");
        text.push_str(&context);
    }
    append_specification(&mut text, &pr.source.specification);
    append_adjustment(&mut text, pr);
    text.push(']');
    text
}

fn append_specification(
    text: &mut String,
    specification: &census_domain::model::EventSpecification,
) {
    if let Some(implement) = specification.implement {
        text.push_str(&format!(", implement {} ug", implement.micrograms()));
    }
    if let Some(hurdle) = specification.hurdles {
        text.push_str(&format!(", hurdle height {} um", hurdle.height_micrometres));
        if let Some(spacing) = hurdle.spacing_micrometres {
            text.push_str(&format!(", hurdle spacing {spacing} um"));
        }
    }
    if let Some(track) = specification.indoor_track {
        let banking = match track.banking {
            census_domain::model::TrackBanking::Flat => "flat",
            census_domain::model::TrackBanking::Banked => "banked",
            census_domain::model::TrackBanking::Unknown => "unknown",
        };
        text.push_str(&format!(
            ", indoor track length {} um, banking {banking}",
            track.length_micrometres
        ));
    }
}

fn append_adjustment(text: &mut String, pr: &SharedSelection) {
    if pr.result.adjustment_kind.is_none() {
        return;
    }
    text.push_str(", conversion ");
    match pr.result.adjustment_basis {
        Some(basis) => text.push_str(basis.label()),
        None => text.push_str("unknown"),
    }
    if let Some(original) = &pr.result.original_mark {
        text.push_str(" from ");
        text.push_str(&crate::bests::mark_text(original));
    }
    if let Some(note) = &pr.result.adjustment_note {
        text.push_str(": ");
        text.push_str(note);
    }
}
