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
