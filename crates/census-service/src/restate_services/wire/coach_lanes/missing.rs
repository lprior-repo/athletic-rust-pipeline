use census_domain::UsJurisdiction;

use super::policy::{ArmBasis, Lane, LanePolicy};
use crate::restate_services::plan::RefusalKind;

pub(super) const LANES: &[Lane] = &[
    Lane {
        jurisdiction: UsJurisdiction::Kentucky,
        arm: "arbiter_orgs", refusal: ("ky_missing_coach_sources", "KHSAA member-directory route (`khsaa.org`); Kentucky Track & Cross Country Coaches Association; TFRRS KY; school sites (emails)."),
        basis: ArmBasis::MissingReason, policy: LanePolicy::Missing,
        kind: RefusalKind::EngineeringGap,
    },
    Lane {
        jurisdiction: UsJurisdiction::Nebraska,
        arm: "plain_names", refusal: ("ne_missing_coach_sources", "NSAA directory export (already wired but email-less); Nebraska Coaches Association (`nebraskacoaches.org`); school coach/staff pages for emails."),
        basis: ArmBasis::MissingReason, policy: LanePolicy::Missing,
        kind: RefusalKind::EngineeringGap,
    },
    Lane {
        jurisdiction: UsJurisdiction::Ohio,
        arm: "ohsaa", refusal: ("oh_missing_coach_sources", "`gclc.gclsports.com` (blocked, SRC-104) · `education.ohio.gov` nonchartered nonpublic list (SRC-102) · `oeds.education.ohio.gov/dataextract` (SRC-101; `var/tap-101-oh-oeds/` contents uninspected) · OHSAA Central District XC page (SRC-105) · OATCCC leadership/regional-reps route (SRC-103) · live myOHSAA coverage for all 815 `OhsaaSchoolId`s (only 15 AD bodies sampled)."),
        basis: ArmBasis::MissingReason, policy: LanePolicy::Missing,
        kind: RefusalKind::EngineeringGap,
    },
    Lane {
        jurisdiction: UsJurisdiction::Georgia,
        arm: "coach_directories", refusal: ("ga_missing_coach_sources", "`gatfxcca.com` (GA TF/XC coaches association; held name: `probes/nhstfxcca/FINDINGS.md` E1 #4; unheld) · GA private/independent athletics association (no name present in held evidence) · school athletic staff pages for emails (email 636 vs any 954)."),
        basis: ArmBasis::MissingReason, policy: LanePolicy::Missing,
        kind: RefusalKind::EngineeringGap,
    },
    Lane {
        jurisdiction: UsJurisdiction::NorthCarolina,
        arm: "coach_directories", refusal: ("nc_missing_coach_sources", "NCISAA-class independent-school directory (no name present in held evidence) · `nc.milesplit.com` full teams crawl (adapter discovery-only) · school athletic sites for staff/email. NOTE: NC email coverage is already near its any-coverage (455/463); the gap is schools **without any row** (463/2766)."),
        basis: ArmBasis::MissingReason, policy: LanePolicy::Missing,
        kind: RefusalKind::EngineeringGap,
    },
    Lane {
        jurisdiction: UsJurisdiction::Wisconsin,
        arm: "wiaa", refusal: ("wi_missing_coach_sources", "`wistca.org` (WI T&F coaches assoc; nhstfxcca E1 #32) · `wisccca.com` (WI XC assoc; E1 #33) · `www.wiaa.com/schools` (403 per ALL-STATE note) · school athletic sites for emails."),
        basis: ArmBasis::MissingReason, policy: LanePolicy::Missing,
        kind: RefusalKind::EngineeringGap,
    },
    Lane {
        jurisdiction: UsJurisdiction::Minnesota,
        arm: "mshsl", refusal: ("mn_missing_coach_sources", "`mshsca.org/track` and `mshsca.org/crosscountry` (MN T&F/XC coaches associations; nhstfxcca E1 #13/14; unheld) · school athletic staff pages for sport-level coach emails."),
        basis: ArmBasis::MissingReason, policy: LanePolicy::Missing,
        kind: RefusalKind::EngineeringGap,
    },
    Lane {
        jurisdiction: UsJurisdiction::Tennessee,
        arm: "tssaa", refusal: ("tn_missing_coach_sources", "`tntccca.com` (TN TF/XC coaches assoc; nhstfxcca E1 #26; unheld) · school athletic staff pages for email (491 email vs 499 any — email coverage is already near its any-coverage) · remaining TN schools outside TSSAA ids."),
        basis: ArmBasis::MissingReason, policy: LanePolicy::Missing,
        kind: RefusalKind::EngineeringGap,
    },
    Lane {
        jurisdiction: UsJurisdiction::Pennsylvania,
        arm: "pa_piaa", refusal: ("pa_missing_coach_sources", "`ptfca.org` (PA TF/XC coaches assoc; registry SRC-237 DERIVED; proto 4 responses held; nhstfxcca E1 #22) · `interacathletics` (SRC-236) · `aopathletics.org` (Philadelphia Catholic League, SRC-238) · PIAA `details.aspx` coach tabs beyond sample ID=12048 · EdNA **public**-school extract form (SRC-232; only the PNP form was executed)."),
        basis: ArmBasis::MissingReason, policy: LanePolicy::Missing,
        kind: RefusalKind::EngineeringGap,
    },
    Lane {
        jurisdiction: UsJurisdiction::Maryland,
        arm: "coach_directories", refusal: ("md_missing_coach_sources", "Maryland private/independent athletics association (no name present in held evidence) · school athletic staff pages for TF/XC + email (matrix tfxc 45, email 87) · `md.milesplit.com` full teams crawl."),
        basis: ArmBasis::MissingReason, policy: LanePolicy::Missing,
        kind: RefusalKind::EngineeringGap,
    },
    Lane {
        jurisdiction: UsJurisdiction::SouthCarolina,
        arm: "coach_directories", refusal: ("sc_missing_coach_sources", "`carolinaxc.com` (SC TF/XC assoc; nhstfxcca E1 #24; unheld) · Knack app full-staff crawl (only school519/staff401 proven) · `sc.milesplit.com` full teams crawl · school sites for email (487/495 already — email is high)."),
        basis: ArmBasis::MissingReason, policy: LanePolicy::Missing,
        kind: RefusalKind::EngineeringGap,
    },
    Lane {
        jurisdiction: UsJurisdiction::Alabama,
        arm: "coach_directories", refusal: ("al_missing_coach_sources", "`www.ahsaa.com` school coach/staff pages (blocked; no archived bodies held) · `www.alabamarunners.com` (unreachable) · `www.xpresstiming.com` (results routes 404) · Alabama private/independent athletics association (no name present in held evidence) · school athletic sites for remaining ~837 uncovered schools (any 704/1541)."),
        basis: ArmBasis::MissingReason, policy: LanePolicy::Missing,
        kind: RefusalKind::AccessBlocked,
    },
    Lane {
        jurisdiction: UsJurisdiction::Arkansas,
        arm: "coach_directories", refusal: ("ar_missing_coach_sources", "`artrackcoaches.com` (AR TF/XC coaches assoc; nhstfxcca E1 #1; unheld) · AR private/parochial athletics association (no name present in held evidence) · school athletic sites for email (449/494)."),
        basis: ArmBasis::MissingReason, policy: LanePolicy::Missing,
        kind: RefusalKind::EngineeringGap,
    },
    Lane {
        jurisdiction: UsJurisdiction::Mississippi,
        arm: "coach_directories", refusal: ("ms_missing_coach_sources", "`misshsaa.com` / `www.misshsaa.com` (home 403 **WAF/bot-shield**; Wayback shows sgcaptcha interstitials) · MS school athletic sites for coach/email at scale (matrix email 509/521 is already near any-coverage, but any 521/1050 = half the state uncovered) · a coach-association seed (MS absent from nhstfxcca E1)."),
        basis: ArmBasis::MissingReason, policy: LanePolicy::Missing,
        kind: RefusalKind::AccessBlocked,
    },
    Lane {
        jurisdiction: UsJurisdiction::NorthDakota,
        arm: "plain_names", refusal: ("nd_missing_coach_sources", "- NDHSAA school pages do not publish emails (COACH_CONTACTS_REFUSAL prose: \"ND 0 emails\") → need per-school district/school sites (e.g. West Fargo, Bismarck, Fargo Davies, Grand Forks Central, Minot, Williston) for emails.\n- NDHSAA cooperative-team alignments (schools fielding combined teams) are not in the held index → NDHSAA co-op pages needed.\n- `ndhsaa.finalforms.com` (1 body only) — full FinalForms school/coach rosters not held."),
        basis: ArmBasis::MissingReason, policy: LanePolicy::Missing,
        kind: RefusalKind::EngineeringGap,
    },
    Lane {
        jurisdiction: UsJurisdiction::Montana,
        arm: "coach_directories", refusal: ("mt_missing_coach_sources", "- MHSA school pages' coach contacts (TF/XC) — corpus pages are directories/enrollment, not staff.\n- School-site staff directories (MT-school-sites only 1.9 KB).\n- mhsa.org live routes beyond captured set (403/404 observed in corpus)."),
        basis: ArmBasis::MissingReason, policy: LanePolicy::Missing,
        kind: RefusalKind::EngineeringGap,
    },
    Lane {
        jurisdiction: UsJurisdiction::Idaho,
        arm: "coach_directories", refusal: ("id_missing_coach_sources", "- Per-sport (TF/XC) coach names/emails — idhsaa coach pages are the association's own, not school staff.\n- School-site staff directories (ID-school-sites 2.4 KB only).\n- Idaho school district sites for email coverage beyond ADs."),
        basis: ArmBasis::MissingReason, policy: LanePolicy::Missing,
        kind: RefusalKind::EngineeringGap,
    },
    Lane {
        jurisdiction: UsJurisdiction::Wyoming,
        arm: "coach_directories", refusal: ("wy_missing_coach_sources", "- WY school-site staff directories (WY-school-sites 1.2 KB) — the TF/XC coach/email lane.\n- whsaa.org content beyond captured home/track/XC (live 403/404).\n- Wyoming district sites (Laramie, Cheyenne, Casper, Gillette, Sheridan…) for emails."),
        basis: ArmBasis::MissingReason, policy: LanePolicy::Missing,
        kind: RefusalKind::EngineeringGap,
    },
    Lane {
        jurisdiction: UsJurisdiction::NewMexico,
        arm: "coach_directories", refusal: ("nm_missing_coach_sources", "- NMAA per-school coach pages (for-coaches is association-level; sport pages are results/records).\n- New Mexico school district sites beyond the 13 KB school-site lane (APS, Las Cruces, Rio Rancho…).\n- NM PED school directory (not held in any tier)."),
        basis: ArmBasis::MissingReason, policy: LanePolicy::Missing,
        kind: RefusalKind::EngineeringGap,
    },
    Lane {
        jurisdiction: UsJurisdiction::Kansas,
        arm: "ks", refusal: ("ks_missing_coach_sources", "- KSHSAA directory letters other than `a` (verify whether name/a returns the whole set — 526 rows suggests it may; if per-letter, b–z needed).\n- KCCTFCA membership list (only clinic/awards pages held) — coach-name lane.\n- KCAA (Christian schools) team directory via athleticnet (adapter lane)."),
        basis: ArmBasis::MissingReason, policy: LanePolicy::Missing,
        kind: RefusalKind::EngineeringGap,
    },
    Lane {
        jurisdiction: UsJurisdiction::WestVirginia,
        arm: "arbiter_orgs", refusal: ("wv_missing_coach_sources", "- WV school-site staff directories (single held probe) — wvusd/k12.wv.us district sites for emails.\n- WVSSAC per-school detail pages (directory page + classifications only).\n- WVSSAC coach listings (association publishes none held)."),
        basis: ArmBasis::MissingReason, policy: LanePolicy::Missing,
        kind: RefusalKind::EngineeringGap,
    },
    Lane {
        jurisdiction: UsJurisdiction::Delaware,
        arm: "coach_directories", refusal: ("de_missing_coach_sources", "- DIAA member/school directory live (`diaa.org` Cloudflare-blocked; only AD directory held).\n- School-site staff directories for TF/XC coaches (`DE-school-sites` 4.3 KB).\n- `diaa.finalforms.com` full rosters (6 bodies only)."),
        basis: ArmBasis::MissingReason, policy: LanePolicy::Missing,
        kind: RefusalKind::EngineeringGap,
    },
    Lane {
        jurisdiction: UsJurisdiction::DistrictOfColumbia,
        arm: "coach_directories", refusal: ("dc_missing_coach_sources", "- DCSAA member coaches for private/non-public schools (only 44 coach rows held).\n- DC school sites beyond the 27.9 KB lane (OSSE sitemap 556 KB held only for discovery).\n- `dcsaa.org` (old host) never captured — MISSING in corpus."),
        basis: ArmBasis::MissingReason, policy: LanePolicy::Missing,
        kind: RefusalKind::EngineeringGap,
    },
    Lane {
        jurisdiction: UsJurisdiction::RhodeIsland,
        arm: "riil", refusal: ("ri_missing_coach_sources", "- RI school-site staff directories — no held lane at all (`extra/` lacks RI).\n- RIIL per-school pages beyond the 128 list (directory has coaches for 55).\n- Any email source for RI coaches (RIIL publishes names only; matrix says 0 emails)."),
        basis: ArmBasis::MissingReason, policy: LanePolicy::Missing,
        kind: RefusalKind::EngineeringGap,
    },
    Lane {
        jurisdiction: UsJurisdiction::NewHampshire,
        arm: "arbiter_orgs", refusal: ("nh_missing_coach_sources", "- NH school-site staff directories for emails (3.5 KB lane; COACH_CONTACTS_REFUSAL notes NH is client-rendered).\n- NHIAA member directory route itself (page held, parser yields 0 — needs a browser/JS route or fresh parser).\n- NHIAA coach emails (association doesn't publish; arbiter has none)."),
        basis: ArmBasis::MissingReason, policy: LanePolicy::Missing,
        kind: RefusalKind::EngineeringGap,
    },
    Lane {
        jurisdiction: UsJurisdiction::Maine,
        arm: "mpa", refusal: ("me_missing_coach_sources", "- ME school-site staff directories for emails (3.9 KB lane; MPA publishes names only).\n- mpa.cc live bodies (corpus assoc-home MISSING — prototype cache 288 KB is the body).\n- Maine private schools outside MPA (association list needed)."),
        basis: ArmBasis::MissingReason, policy: LanePolicy::Missing,
        kind: RefusalKind::EngineeringGap,
    },
];
