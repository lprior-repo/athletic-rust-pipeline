use census_domain::model::{
    CoachLaneError, CoachLaneObligation, CoachLaneRefusal, CoachLaneResolution, CoachLaneStage,
    PendingCoachLane, SchoolYear, SourceRef,
};
use census_domain::UsJurisdiction;

use crate::restate_services::plan::RefusalKind;
use crate::restate_services::wire::{RefusedSource, SourcePlan};

pub(super) struct RefusedLane {
    pub(super) jurisdiction: UsJurisdiction,
    arm: &'static str,
    pub(super) slug: &'static str,
    reason: &'static str,
    kind: RefusalKind,
}

pub(super) const LANES: &[RefusedLane] = &[
    RefusedLane {
        jurisdiction: UsJurisdiction::Colorado,
        arm: "chsaa", slug: "co_missing_coach_sources",
        reason: "CHSAA per-school detail pages beyond the 3 samples (`chsaanow.com/schools/*`); Colorado High School Coaches Association (CHSCA); Colorado TF/XC coaches association; school athletics staff pages.",
        kind: RefusalKind::EngineeringGap,
    },
    RefusedLane {
        jurisdiction: UsJurisdiction::Illinois,
        arm: "ihsa", slug: "il_missing_coach_sources",
        reason: "ITCCCA (`itccca.com`); Chicago Catholic League; ISBE nonpublic registration (`isbe.net`); IHSA conference route (`conf.htm` probe); school staff pages.",
        kind: RefusalKind::EngineeringGap,
    },
    RefusedLane {
        jurisdiction: UsJurisdiction::Kentucky,
        arm: "arbiter_orgs", slug: "ky_missing_coach_sources",
        reason: "KHSAA member-directory route (`khsaa.org`); Kentucky Track & Cross Country Coaches Association; TFRRS KY; school sites (emails).",
        kind: RefusalKind::EngineeringGap,
    },
    RefusedLane {
        jurisdiction: UsJurisdiction::Nebraska,
        arm: "plain_names", slug: "ne_missing_coach_sources",
        reason: "NSAA directory export (already wired but email-less); Nebraska Coaches Association (`nebraskacoaches.org`); school coach/staff pages for emails.",
        kind: RefusalKind::EngineeringGap,
    },
    RefusedLane {
        jurisdiction: UsJurisdiction::Ohio,
        arm: "ohsaa", slug: "oh_missing_coach_sources",
        reason: "`gclc.gclsports.com` (blocked, SRC-104) · `education.ohio.gov` nonchartered nonpublic list (SRC-102) · `oeds.education.ohio.gov/dataextract` (SRC-101; `var/tap-101-oh-oeds/` contents uninspected) · OHSAA Central District XC page (SRC-105) · OATCCC leadership/regional-reps route (SRC-103) · live myOHSAA coverage for all 815 `OhsaaSchoolId`s (only 15 AD bodies sampled).",
        kind: RefusalKind::EngineeringGap,
    },
    RefusedLane {
        jurisdiction: UsJurisdiction::Georgia,
        arm: "coach_directories", slug: "ga_missing_coach_sources",
        reason: "`gatfxcca.com` (GA TF/XC coaches association; held name: `probes/nhstfxcca/FINDINGS.md` E1 #4; unheld) · GA private/independent athletics association (no name present in held evidence) · school athletic staff pages for emails (email 636 vs any 954).",
        kind: RefusalKind::EngineeringGap,
    },
    RefusedLane {
        jurisdiction: UsJurisdiction::NorthCarolina,
        arm: "coach_directories", slug: "nc_missing_coach_sources",
        reason: "NCISAA-class independent-school directory (no name present in held evidence) · `nc.milesplit.com` full teams crawl (adapter discovery-only) · school athletic sites for staff/email. NOTE: NC email coverage is already near its any-coverage (455/463); the gap is schools **without any row** (463/2766).",
        kind: RefusalKind::EngineeringGap,
    },
    RefusedLane {
        jurisdiction: UsJurisdiction::Wisconsin,
        arm: "wiaa", slug: "wi_missing_coach_sources",
        reason: "`wistca.org` (WI T&F coaches assoc; nhstfxcca E1 #32) · `wisccca.com` (WI XC assoc; E1 #33) · `www.wiaa.com/schools` (403 per ALL-STATE note) · school athletic sites for emails.",
        kind: RefusalKind::EngineeringGap,
    },
    RefusedLane {
        jurisdiction: UsJurisdiction::Minnesota,
        arm: "mshsl", slug: "mn_missing_coach_sources",
        reason: "`mshsca.org/track` and `mshsca.org/crosscountry` (MN T&F/XC coaches associations; nhstfxcca E1 #13/14; unheld) · school athletic staff pages for sport-level coach emails.",
        kind: RefusalKind::EngineeringGap,
    },
    RefusedLane {
        jurisdiction: UsJurisdiction::Tennessee,
        arm: "tssaa", slug: "tn_missing_coach_sources",
        reason: "`tntccca.com` (TN TF/XC coaches assoc; nhstfxcca E1 #26; unheld) · school athletic staff pages for email (491 email vs 499 any — email coverage is already near its any-coverage) · remaining TN schools outside TSSAA ids.",
        kind: RefusalKind::EngineeringGap,
    },
    RefusedLane {
        jurisdiction: UsJurisdiction::Pennsylvania,
        arm: "pa_piaa", slug: "pa_missing_coach_sources",
        reason: "`ptfca.org` (PA TF/XC coaches assoc; registry SRC-237 DERIVED; proto 4 responses held; nhstfxcca E1 #22) · `interacathletics` (SRC-236) · `aopathletics.org` (Philadelphia Catholic League, SRC-238) · PIAA `details.aspx` coach tabs beyond sample ID=12048 · EdNA **public**-school extract form (SRC-232; only the PNP form was executed).",
        kind: RefusalKind::EngineeringGap,
    },
    RefusedLane {
        jurisdiction: UsJurisdiction::Maryland,
        arm: "coach_directories", slug: "md_missing_coach_sources",
        reason: "Maryland private/independent athletics association (no name present in held evidence) · school athletic staff pages for TF/XC + email (matrix tfxc 45, email 87) · `md.milesplit.com` full teams crawl.",
        kind: RefusalKind::EngineeringGap,
    },
    RefusedLane {
        jurisdiction: UsJurisdiction::SouthCarolina,
        arm: "coach_directories", slug: "sc_missing_coach_sources",
        reason: "`carolinaxc.com` (SC TF/XC assoc; nhstfxcca E1 #24; unheld) · Knack app full-staff crawl (only school519/staff401 proven) · `sc.milesplit.com` full teams crawl · school sites for email (487/495 already — email is high).",
        kind: RefusalKind::EngineeringGap,
    },
    RefusedLane {
        jurisdiction: UsJurisdiction::Alabama,
        arm: "coach_directories", slug: "al_missing_coach_sources",
        reason: "`www.ahsaa.com` school coach/staff pages (blocked; no archived bodies held) · `www.alabamarunners.com` (unreachable) · `www.xpresstiming.com` (results routes 404) · Alabama private/independent athletics association (no name present in held evidence) · school athletic sites for remaining ~837 uncovered schools (any 704/1541).",
        kind: RefusalKind::AccessBlocked,
    },
    RefusedLane {
        jurisdiction: UsJurisdiction::Arkansas,
        arm: "coach_directories", slug: "ar_missing_coach_sources",
        reason: "`artrackcoaches.com` (AR TF/XC coaches assoc; nhstfxcca E1 #1; unheld) · AR private/parochial athletics association (no name present in held evidence) · school athletic sites for email (449/494).",
        kind: RefusalKind::EngineeringGap,
    },
    RefusedLane {
        jurisdiction: UsJurisdiction::Mississippi,
        arm: "coach_directories", slug: "ms_missing_coach_sources",
        reason: "`misshsaa.com` / `www.misshsaa.com` (home 403 **WAF/bot-shield**; Wayback shows sgcaptcha interstitials) · MS school athletic sites for coach/email at scale (matrix email 509/521 is already near any-coverage, but any 521/1050 = half the state uncovered) · a coach-association seed (MS absent from nhstfxcca E1).",
        kind: RefusalKind::AccessBlocked,
    },
    RefusedLane {
        jurisdiction: UsJurisdiction::NorthDakota,
        arm: "plain_names", slug: "nd_missing_coach_sources",
        reason: "- NDHSAA school pages do not publish emails (COACH_CONTACTS_REFUSAL prose: \"ND 0 emails\") → need per-school district/school sites (e.g. West Fargo, Bismarck, Fargo Davies, Grand Forks Central, Minot, Williston) for emails.\n- NDHSAA cooperative-team alignments (schools fielding combined teams) are not in the held index → NDHSAA co-op pages needed.\n- `ndhsaa.finalforms.com` (1 body only) — full FinalForms school/coach rosters not held.",
        kind: RefusalKind::EngineeringGap,
    },
    RefusedLane {
        jurisdiction: UsJurisdiction::Montana,
        arm: "coach_directories", slug: "mt_missing_coach_sources",
        reason: "- MHSA school pages' coach contacts (TF/XC) — corpus pages are directories/enrollment, not staff.\n- School-site staff directories (MT-school-sites only 1.9 KB).\n- mhsa.org live routes beyond captured set (403/404 observed in corpus).",
        kind: RefusalKind::EngineeringGap,
    },
    RefusedLane {
        jurisdiction: UsJurisdiction::Idaho,
        arm: "coach_directories", slug: "id_missing_coach_sources",
        reason: "- Per-sport (TF/XC) coach names/emails — idhsaa coach pages are the association's own, not school staff.\n- School-site staff directories (ID-school-sites 2.4 KB only).\n- Idaho school district sites for email coverage beyond ADs.",
        kind: RefusalKind::EngineeringGap,
    },
    RefusedLane {
        jurisdiction: UsJurisdiction::Wyoming,
        arm: "coach_directories", slug: "wy_missing_coach_sources",
        reason: "- WY school-site staff directories (WY-school-sites 1.2 KB) — the TF/XC coach/email lane.\n- whsaa.org content beyond captured home/track/XC (live 403/404).\n- Wyoming district sites (Laramie, Cheyenne, Casper, Gillette, Sheridan…) for emails.",
        kind: RefusalKind::EngineeringGap,
    },
    RefusedLane {
        jurisdiction: UsJurisdiction::NewMexico,
        arm: "coach_directories", slug: "nm_missing_coach_sources",
        reason: "- NMAA per-school coach pages (for-coaches is association-level; sport pages are results/records).\n- New Mexico school district sites beyond the 13 KB school-site lane (APS, Las Cruces, Rio Rancho…).\n- NM PED school directory (not held in any tier).",
        kind: RefusalKind::EngineeringGap,
    },
    RefusedLane {
        jurisdiction: UsJurisdiction::Kansas,
        arm: "ks", slug: "ks_missing_coach_sources",
        reason: "- KSHSAA directory letters other than `a` (verify whether name/a returns the whole set — 526 rows suggests it may; if per-letter, b–z needed).\n- KCCTFCA membership list (only clinic/awards pages held) — coach-name lane.\n- KCAA (Christian schools) team directory via athleticnet (adapter lane).",
        kind: RefusalKind::EngineeringGap,
    },
    RefusedLane {
        jurisdiction: UsJurisdiction::WestVirginia,
        arm: "arbiter_orgs", slug: "wv_missing_coach_sources",
        reason: "- WV school-site staff directories (single held probe) — wvusd/k12.wv.us district sites for emails.\n- WVSSAC per-school detail pages (directory page + classifications only).\n- WVSSAC coach listings (association publishes none held).",
        kind: RefusalKind::EngineeringGap,
    },
    RefusedLane {
        jurisdiction: UsJurisdiction::Delaware,
        arm: "coach_directories", slug: "de_missing_coach_sources",
        reason: "- DIAA member/school directory live (`diaa.org` Cloudflare-blocked; only AD directory held).\n- School-site staff directories for TF/XC coaches (`DE-school-sites` 4.3 KB).\n- `diaa.finalforms.com` full rosters (6 bodies only).",
        kind: RefusalKind::EngineeringGap,
    },
    RefusedLane {
        jurisdiction: UsJurisdiction::DistrictOfColumbia,
        arm: "coach_directories", slug: "dc_missing_coach_sources",
        reason: "- DCSAA member coaches for private/non-public schools (only 44 coach rows held).\n- DC school sites beyond the 27.9 KB lane (OSSE sitemap 556 KB held only for discovery).\n- `dcsaa.org` (old host) never captured — MISSING in corpus.",
        kind: RefusalKind::EngineeringGap,
    },
    RefusedLane {
        jurisdiction: UsJurisdiction::RhodeIsland,
        arm: "riil", slug: "ri_missing_coach_sources",
        reason: "- RI school-site staff directories — no held lane at all (`extra/` lacks RI).\n- RIIL per-school pages beyond the 128 list (directory has coaches for 55).\n- Any email source for RI coaches (RIIL publishes names only; matrix says 0 emails).",
        kind: RefusalKind::EngineeringGap,
    },
    RefusedLane {
        jurisdiction: UsJurisdiction::NewHampshire,
        arm: "arbiter_orgs", slug: "nh_missing_coach_sources",
        reason: "- NH school-site staff directories for emails (3.5 KB lane; COACH_CONTACTS_REFUSAL notes NH is client-rendered).\n- NHIAA member directory route itself (page held, parser yields 0 — needs a browser/JS route or fresh parser).\n- NHIAA coach emails (association doesn't publish; arbiter has none).",
        kind: RefusalKind::EngineeringGap,
    },
    RefusedLane {
        jurisdiction: UsJurisdiction::Maine,
        arm: "mpa", slug: "me_missing_coach_sources",
        reason: "- ME school-site staff directories for emails (3.9 KB lane; MPA publishes names only).\n- mpa.cc live bodies (corpus assoc-home MISSING — prototype cache 288 KB is the body).\n- Maine private schools outside MPA (association list needed).",
        kind: RefusalKind::EngineeringGap,
    },
];

pub(super) fn bind(
    plan: &mut SourcePlan,
    jurisdiction: UsJurisdiction,
    season: SchoolYear,
    revision: u32,
    observed_on: &str,
) -> Result<Option<CoachLaneObligation>, CoachLaneError> {
    let Some(row) = LANES.iter().find(|row| row.jurisdiction == jurisdiction) else {
        return Ok(None);
    };
    let refusal = CoachLaneRefusal::new(SourceRef::id(row.slug), row.reason, observed_on)?;
    let pending = CoachLaneObligation::pending(PendingCoachLane {
        jurisdiction,
        season,
        revision,
        arm: row.arm.to_string(),
        arm_basis: row.reason.to_string(),
        observed_on: observed_on.to_string(),
    })?;
    let obligation = pending.record(
        CoachLaneStage::Blocked,
        Some(refusal),
        CoachLaneResolution::unresolved(observed_on),
    )?;
    if !plan.refused.iter().any(|entry| entry.slug == row.slug) {
        plan.refused.push(RefusedSource {
            slug: row.slug.to_string(),
            reason: row.reason.to_string(),
            kind: row.kind,
        });
    }
    Ok(Some(obligation))
}
