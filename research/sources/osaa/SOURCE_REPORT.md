# OSAA (Oregon School Activities Association) - Source Report

**Date:** 2026-10-04
**Source ID:** SRC-146
**URL:** https://www.osaa.org/schools/regions
**Disposition:** adapter_or_extract → documented refusal

## Capture Attempts

### 1. Regions Directory Page
- **URL:** https://www.osaa.org/schools/regions
- **HTTP Status:** 403 Forbidden
- **Bytes:** 0
- **Outcome:** Cannot enumerate schools; anonymous access blocked

### 2. Site Root (diagnostic)
- **URL:** https://www.osaa.org/
- **HTTP Status:** 403 Forbidden
- **Bytes:** 0
- **Outcome:** Entire site appears to require authentication or blocks automated access

### 3. Alternative URLs (diagnostic)
- **URL:** https://osaa.org/schools/regions
- **HTTP Status:** 403 Forbidden
- **URL:** http://www.osaa.org/schools/regions
- **HTTP Status:** 403 Forbidden (redirects to https)

## Evidence

All HTTP requests to www.osaa.org (and osaa.org) return 403 Forbidden regardless of path:
- `/` → 403
- `/schools/regions` → 403

This indicates the site implements a blanket access control that prevents anonymous crawling. No raw HTML captures could be obtained.

## Coach Field Inventory

No coach field inventory could be established because no page content was retrievable. The site's structure, whether it publishes school profiles with coach names/emails, and whether such data is available anonymously, cannot be determined without access to the page content.

## Routing Decision

**Documented refusal.** The OSAA website returns HTTP 403 for all anonymous requests, making it impossible to:
1. Enumerate schools via the regions directory
2. Fetch any school profile pages to inspect coach data

Since the contract prohibits bypassing access blocks, OSAA cannot be tapped without a documented change in access policy. An adapter implementation is not justified when no content can be retrieved.

## Recommended Follow-up

- Contact OSAA directly to inquire about public data access or API availability
- Monitor for changes to site access policy
- Consider whether OSAA provides data through alternate channels (annual reports, data files, FOIA-like requests)

## Acceptance Criteria Met

- [x] Raw captures attempted with URL/status/bytes documented
- [ ] Rows.csv or adapter not produced (no content available)
- [ ] Import/readback deferred to Main if adapter is later feasible
- [x] Dated evidence provided in this report with exact URLs and statuses

## 2026-10-12 qualification correction

The blanket-access conclusion above is historical and is not the current coach-lane refusal.
The Oregon bead records the owner's 2026-10-07 observation: `/schools/full-members` returned
HTTP 200 with 300 member-school links and `robots.txt` returned HTTP 200. That observation is
not re-probed here. Retained `state-assoc-westcoast/samples/osaa-school-347.html` contains
Adrian's school staff and activity head-coach tables; its SHA-256, measured with `sha256sum`,
is `b44dac9d956cbdad802464b2002d82d75434d305bb34d56f479b1b71d4502a7d`.
It carries published AD mailbox text and TF/XC coach names. Oregon is therefore not described
as a source that never published contacts or as an authenticated-only site.

The current refusal is an engineering gap: Oregon has no wired durable OSAA coach arm.
`coach_contacts` is a registered candidate but is not dispatched by the durable teams arm
table. The prototype's 298-profile output is held research, not fresh-run execution or an
admissible current-run contact import. The state obligation records
`or_missing_coach_sources` and the work order's remaining-source inventory, stays nonterminal,
and names the missing durable frontier in `arm_basis`. No operator acceptance is manufactured.

The close/reopen scenario writes the exact obligation and jurisdiction decision to an isolated
temporary Fjall store, flushes and reopens it, and checks refusal preservation and idempotent
rebind. No historical run state is changed, no service is restarted and no live crawl occurs.
Main owns this round's exact command/result evidence; this source qualification does not
claim that an unexecuted scenario passed or that a live Restate restart was exercised.
