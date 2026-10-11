use census_domain::model::CompetitionLevel;
use census_domain::UsJurisdiction;

const VENUE_STATES: &[(&str, UsJurisdiction)] = &[
    ("university of minnesota", UsJurisdiction::Minnesota),
    ("macalester college", UsJurisdiction::Minnesota),
    ("st. olaf college", UsJurisdiction::Minnesota),
    ("st olaf college", UsJurisdiction::Minnesota),
    ("carleton college", UsJurisdiction::Minnesota),
    ("hamline university", UsJurisdiction::Minnesota),
    ("gustavus adolphus college", UsJurisdiction::Minnesota),
    ("minnesota state mankato", UsJurisdiction::Minnesota),
    ("bemidji state university", UsJurisdiction::Minnesota),
    ("concordia college moorhead", UsJurisdiction::Minnesota),
    ("university of iowa", UsJurisdiction::Iowa),
    ("university of northern iowa", UsJurisdiction::Iowa),
    ("iowa state university", UsJurisdiction::Iowa),
    ("wartburg college", UsJurisdiction::Iowa),
    ("drake university", UsJurisdiction::Iowa),
    ("luther college", UsJurisdiction::Iowa),
    ("simpson college", UsJurisdiction::Iowa),
    ("coe college", UsJurisdiction::Iowa),
    ("central college", UsJurisdiction::Iowa),
    ("grinnell college", UsJurisdiction::Iowa),
    ("cornell college", UsJurisdiction::Iowa),
    ("loras college", UsJurisdiction::Iowa),
    ("buena vista university", UsJurisdiction::Iowa),
    ("university of dubuque", UsJurisdiction::Iowa),
    ("mount mercy university", UsJurisdiction::Iowa),
    ("uw-river falls", UsJurisdiction::Wisconsin),
    ("uw-la crosse", UsJurisdiction::Wisconsin),
    ("uw-eau claire", UsJurisdiction::Wisconsin),
    ("uw-oshkosh", UsJurisdiction::Wisconsin),
    ("uw-stevens point", UsJurisdiction::Wisconsin),
    ("uw-whitewater", UsJurisdiction::Wisconsin),
];

pub fn venue_state(location: &str) -> Option<UsJurisdiction> {
    let location = location.trim();
    explicit_state(location).or_else(|| {
        VENUE_STATES
            .iter()
            .find(|(label, _)| location.eq_ignore_ascii_case(label))
            .map(|(_, state)| *state)
    })
}

fn explicit_state(location: &str) -> Option<UsJurisdiction> {
    let (_, state) = location.rsplit_once(',')?;
    let state = state.trim();
    UsJurisdiction::ALL.into_iter().find(|candidate| {
        state.eq_ignore_ascii_case(candidate.code()) || state.eq_ignore_ascii_case(candidate.name())
    })
}

pub fn level_of(name: &str) -> CompetitionLevel {
    let has = |needle: &str| {
        name.as_bytes()
            .windows(needle.len())
            .any(|window| window.eq_ignore_ascii_case(needle.as_bytes()))
    };
    if has("state") {
        CompetitionLevel::State
    } else if has("sectional") || has("section ") {
        CompetitionLevel::Sectional
    } else if has("regional") {
        CompetitionLevel::Regional
    } else if has("conference") || has("conf.") {
        CompetitionLevel::Conference
    } else if has("district") {
        CompetitionLevel::District
    } else if has("national") {
        CompetitionLevel::National
    } else if has("dual") {
        CompetitionLevel::Dual
    } else if has("invitational") || has("invite") || has("relays") || has("classic") || has("meet")
    {
        CompetitionLevel::Invitational
    } else {
        CompetitionLevel::Unknown
    }
}
