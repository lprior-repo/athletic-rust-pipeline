pub const MAX_ANCHORS: usize = 400;
pub const MAX_TABLES: usize = 8;
pub const MAX_TABLE_ROWS: usize = 40;
pub const CONTEXT_RADIUS: usize = 60;
pub const CONTEXT_MAX: usize = 220;
pub const TITLE_MAX: usize = 90;

pub(super) const SPORT: &str = r"(?:Cross[-\s]?Country|XC|Track(?:\s*(?:&|and)\s*Field)?)";
pub(super) const TOKEN: &str = r"[A-Z](?:[a-z]+)?(?:['’\-][A-Z][a-z]+)?\.?";
pub(super) const NAME: &str = r"[A-Z](?:[a-z]+)?(?:['’\-][A-Z][a-z]+)?\.?(?:\s+[A-Z](?:[a-z]+)?(?:['’\-][A-Z][a-z]+)?\.?){1,3}";

pub(super) const TITLE_WORDS: &[&str] = &[
    "coach",
    "coaches",
    "director",
    "directors",
    "athletic",
    "athletics",
    "teacher",
    "teachers",
    "staff",
    "member",
    "members",
    "principal",
    "assistant",
    "district",
    "head",
    "school",
    "high",
    "junior",
    "senior",
    "elementary",
    "middle",
    "email",
    "phone",
    "fax",
    "contact",
    "contacts",
    "form",
    "forms",
    "physical",
    "transportation",
    "special",
    "education",
    "depot",
    "street",
    "box",
    "search",
    "results",
    "home",
    "menu",
    "news",
    "event",
    "events",
    "calendar",
    "about",
    "academic",
    "academics",
    "student",
    "students",
    "parent",
    "parents",
    "community",
    "administration",
    "department",
    "departments",
    "activity",
    "activities",
    "office",
    "address",
    "city",
    "state",
    "links",
    "popular",
    "social",
    "studies",
    "spring",
    "winter",
    "fall",
    "athlete",
    "athletes",
    "booster",
    "boosters",
    "conference",
    "schedule",
    "schedules",
    "roster",
    "spirit",
    "shop",
    "store",
    "ticket",
    "tickets",
    "live",
    "stream",
    "photos",
];

pub(super) const TLD_OK: &[&str] = &[
    "com",
    "org",
    "net",
    "edu",
    "gov",
    "mil",
    "int",
    "info",
    "biz",
    "name",
    "pro",
    "museum",
    "coop",
    "aero",
    "jobs",
    "mobi",
    "travel",
    "xyz",
    "online",
    "site",
    "tech",
    "store",
    "school",
    "academy",
    "education",
    "us",
    "io",
    "co",
    "me",
    "tv",
    "cc",
    "app",
    "dev",
    "cloud",
    "page",
];

pub(super) const BAD_EMAIL_CHARS: &[char] = &[
    '\\', '"', '\'', '<', '>', '(', ')', '[', ']', '{', '}', ',', ';', ':', '!', '?', '&', '*',
    '%', '$', '#', '|', '^', '~', '`', '=', '+', '/', ' ', '\t', '\r', '\n',
];

pub(super) const INVISIBLE: &[char] = &[
    '\u{200b}', '\u{200c}', '\u{200d}', '\u{200e}', '\u{200f}', '\u{2060}', '\u{feff}', '\u{fffc}',
    '\u{00ad}', '\u{00a0}',
];

pub(super) const TRIM_CHARS: &[char] = &[
    '\\', '"', '\'', '<', '>', '(', ')', '[', ']', '{', '}', ',', ';', ':', '.', '!', '?', ' ',
    '\t',
];

pub(super) const OFF_LIMITS: &str = r"(?i)facebook\.com|twitter\.com|x\.com|instagram\.com|youtube\.com|tiktok\.com|linkedin\.com|maxpreps\.com|milesplit\.com|athletic\.net|hudl\.com|gofan\.co|apple\.com|zoom\.us|spotify\.com|vimeo\.com|paypal\.com|eventbrite\.|smugmug\.com|flickr\.com|docs\.google\.com|drive\.google\.com|maps\.google|www\.google\.com|accounts\.google|doubleclick|googletagmanager|gstatic|cloudflare|jsdelivr|bootstrapcdn";

pub(super) const PLATFORM: &str = r"(?i)bigteams\.com|arbiterwebsites\.com|arbitersports\.com|vnn\.com|rschooltoday\.com|rschoolteams\.com|homecampus\.com|fpsports\.org|8to18\.com|digitalimage|sportngin\.com|leagueathletics\.com|sportsengine\.com|teampages\.com|schedulegalaxy\.com|sites\.google\.com|wordpress\.com|wixsite\.com|weebly\.com|schoolwires\.|blackboard\.com|finalsite|sharpschool|edlio|apptegy|thrillshare";

pub(super) const JUNK_CONTEXT: &str = r"(?i)search results|connect directly|cookie|privacy policy|terms of use|memories|hall of fame|induct|obituar|passed away|in memory|former coach|retired|alumni|class of 19|yearbook|in memoriam";

pub(super) const INTERESTING: &str = r"(?i)coach|athletic|staff|directory|contact|faculty";

pub(super) const SPORT_HINT: &str = r"(?i)Cross[-\s]?Country|XC|Track(?:\s*(?:&|and)\s*Field)?";

pub(super) const TABLE_HEADER: &str = r"(?i)coach|sport|staff|name";

pub(super) const HIDDEN_BLOCK: &str =
    r"(?is)<(?:script|style|noscript)\b[^>]*>.*?</(?:script|style|noscript)\s*>";

pub(super) const COMMENT: &str = r"(?s)<!--.*?-->";

pub(super) const TAG: &str = r"(?s)<[^>]*>";

pub(super) const TITLE_TAG: &str = r"(?is)<title\b[^>]*>(.*?)</title\s*>";

pub(super) const ANCHOR: &str = r"(?is)<a\b([^>]*)>(.*?)</a\s*>";

pub(super) const ATTRIBUTE: &str =
    r#"(?i)([A-Za-z_:][A-Za-z0-9_:.-]*)\s*=\s*(?:"([^"]*)"|'([^']*)')"#;

pub(super) const TABLE: &str = r"(?is)<table\b[^>]*>(.*?)</table\s*>";

pub(super) const ROW: &str = r"(?is)<tr\b[^>]*>(.*?)</tr\s*>";

pub(super) const CELL: &str = r"(?is)<t[dh]\b[^>]*>(.*?)</t[dh]\s*>";

pub(super) const LOCAL: &str = r"[A-Za-z0-9][A-Za-z0-9._%+-]*";

pub(super) const DOMAIN: &str =
    r"[A-Za-z0-9]([A-Za-z0-9-]*[A-Za-z0-9])?(\.[A-Za-z0-9]([A-Za-z0-9-]*[A-Za-z0-9])?)+";

pub(super) const GIRLS: &str = r"(?i)\bgirls?\b|\bwomen\b";

pub(super) const BOYS: &str = r"(?i)\bboys?\b|\bmen\b";

pub(super) const CROSS_COUNTRY: &str = r"(?i)cross[-\s]?country|\bxc\b";

pub(super) const ASSISTANT: &str = r"(?i)assistant|asst\b";

pub(super) const COACH_PATTERNS: [&str; 6] = [
    "school_sites coach a",
    "school_sites coach b",
    "school_sites coach c",
    "school_sites coach d",
    "school_sites coach e",
    "school_sites coach f",
];

pub(super) const AD_PATTERNS: [&str; 3] = [
    "school_sites ad a",
    "school_sites ad b",
    "school_sites ad c",
];

pub(super) fn coach_patterns() -> [String; 6] {
    [
        format!(
            r"(?:Head\s+)?(?:Boys|Girls)?\s*{SPORT}\s+(?:Head\s+)?Coach[^A-Za-z]{{0,12}}({NAME})"
        ),
        format!(r"({NAME})\s*[,|–—-]\s*(?:Head\s+)?(?:Boys|Girls)?\s*{SPORT}\s+(?:Head\s+)?Coach"),
        format!(r"({NAME})\s*\|\s*(?:Boys|Girls)?(?:\s*&\s*(?:Boys|Girls))?\s*{SPORT}"),
        format!(r"{SPORT}\s*\|\s*(?:Boys|Girls)?(?:\s*&\s*(?:Boys|Girls))?\s*\|\s*({NAME})"),
        format!(r"(?:Head\s+)?Coach[^A-Za-z]{{0,6}}{SPORT}[^A-Za-z]{{0,12}}({NAME})"),
        format!(r"({NAME})\s+(?:Head\s+)?(?:Boys|Girls)?\s*{SPORT}\s+(?:Head\s+)?Coach"),
    ]
}

pub(super) fn ad_patterns() -> [String; 3] {
    [
        format!(
            r"(?i)(?:District\s+|Assistant\s+)?Athletic\s+Director[^A-Za-z]{{0,6}}(?:Email\s+|E-mail\s+|Phone\s+|Contact\s+)?({NAME})"
        ),
        format!(r"(?i)({NAME})\s*[,|–—-]\s*(?:District\s+|Assistant\s+)?Athletic\s+Director"),
        format!(r"(?i)(?:District\s+|Assistant\s+)?Athletic\s+Director[^.\n]{{0,40}}?({NAME})"),
    ]
}
