use crate::bests::SharedSelection;

pub(super) const ATHLETES: &str = "Athletes";

pub(super) const PRS: &str = "PRs";

pub(super) const COACHES: &str = "Coaches";

pub(super) const META_SHEETS: [&str; 7] = [
    "Schools",
    "Meets",
    "Sources",
    "Coverage",
    "Conflicts",
    "Review",
    "Run Metrics",
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

pub(super) const ATHLETE_HEADERS: [&str; 59] = [
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
    "Athletic.net URL",
    "MileSplit URL",
    "Other profile URLs",
    "Sources Count",
    "Identity Status",
    "Coverage State",
    "Conflict Flag",
    "Review Status",
];

pub(super) const PR_HEADERS: [&str; 26] = [
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
];

pub(super) const COACH_HEADERS: [&str; 18] = [
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

pub(super) fn qualified_mark(pr: &SharedSelection) -> String {
    let mut text = String::new();
    text.push_str(pr_event_name(&pr.key.event_kind.stable_key()));
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
    text.push(']');
    text
}
