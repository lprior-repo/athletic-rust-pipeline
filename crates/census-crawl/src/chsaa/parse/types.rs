pub struct MemberSchool {
    pub school_code: Option<u64>,
    pub slug: Option<String>,
    pub name: Option<String>,
    pub official_name: Option<String>,
    pub city: Option<String>,
    pub street_address: Option<String>,
    pub zip_code: Option<String>,
    pub phone: Option<String>,
    pub district_name: Option<String>,
    pub member_type: Option<String>,
    pub school_type: Option<String>,
    pub setting: Option<String>,
}

pub struct SchoolCoachRow {
    pub school_name: String,
    pub person: String,
    pub activity_name: String,
    pub title: String,
}
