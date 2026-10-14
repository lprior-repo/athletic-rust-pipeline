# Athletic Recruiting Database End Vision and Implementation Specification

Version 2 · October 11 2026

This specification defines a reusable public-source database for NCAA-relevant high-school Track and Field and Cross Country recruiting, its implementation contract, and release gates. First delivery focuses on Michigan, Minnesota, Iowa, Illinois, and Colorado, with Michigan's Upper Peninsula prioritized; the long-term national target remains the lower 48 states plus Washington DC. The Fjall-backed Rust pipeline handles acquisition and finished-dataset generation. A separate query database remains an open choice; SQLite is recommended for the finished local dataset.

## Contents

- [1 Purpose and product scope](#1-purpose-and-product-scope)
- [2 Architecture and ownership boundaries](#2-architecture-and-ownership-boundaries)
- [3 Public discovery and coverage](#3-public-discovery-and-coverage)
- [4 Query profiles and cohort membership](#4-query-profiles-and-cohort-membership)
- [5 Logical records and reversible identities](#5-logical-records-and-reversible-identities)
- [6 Capture content and interpretation identity](#6-capture-content-and-interpretation-identity)
- [7 Comparable best marks and retained candidates](#7-comparable-best-marks-and-retained-candidates)
- [8 School affiliations addresses and contacts](#8-school-affiliations-addresses-and-contacts)
- [9 Durable commits and replay safety](#9-durable-commits-and-replay-safety)
- [10 Resumable discovery and refresh](#10-resumable-discovery-and-refresh)
- [11 Offline reparse and incremental projections](#11-offline-reparse-and-incremental-projections)
- [12 Storage limits retention and recovery](#12-storage-limits-retention-and-recovery)
- [13 Consistent recruiting publications](#13-consistent-recruiting-publications)
- [14 Annual operation and configuration](#14-annual-operation-and-configuration)
- [15 Finished dataset query database boundary](#15-finished-dataset-query-database-boundary)
- [16 Qualification and performance evidence](#16-qualification-and-performance-evidence)
- [17 Architecture decision transition and sources](#17-architecture-decision-transition-and-sources)
- [18 Implementation handoff and acceptance checklist](#18-implementation-handoff-and-acceptance-checklist)
- [19 Five state rollout appendix](#19-five-state-rollout-appendix)

## 1 Purpose and product scope

Build a persistent, public-source-discovered high-school Track and Field and Cross Country database that supports NCAA college recruiting and gets more useful with every run. Discover athletes across the configured geography and graduating classes, identify them correctly, select their best valid performance for each comparable event, and connect them to their school, school address, and current public professional coaching contacts. Produce a usable recruiting workbook and machine-readable datasets with source evidence and honest gaps.

Preserve acquired knowledge across restarts, corrections, parser upgrades, and annual cohorts. Normal operation incrementally synchronizes a long-lived store. Optimize measured throughput and repeated work on the existing single node while preserving accuracy, bounded resources, and recoverability. Make accepted finished data easy to query; detailed downstream schema selection follows this specification.

This is the proposed end-state contract for implementation. It does not certify that the current code implements or passes these requirements. MUST denotes a release-blocking obligation for the affected feature; SHOULD denotes a preferred approach requiring a documented reason to depart. Suggested API names and storage layouts are design contracts, not claims that existing commands or tables already exist.

### What the user receives

- One discoverable athlete identity with evidence-backed graduating-class membership and school affiliation.
- A single selected best valid mark for each event and compatible comparison class. Different distances, disciplines, conditions, and relay versus individual results are not collapsed into a misleading number.
- School postal address and current, source-backed public professional coach or program contact when available. Missing or disputed information remains visibly unknown.
- Source profile links and sufficient provenance to explain each selected fact and best mark.
- A primary recruiting workbook, a normalized machine-readable dataset, a coverage report, and a review queue derived from one consistent data revision.
- A stable completed-dataset handoff suitable for a query database with useful tables, relationships, and indexes. SQLite is recommended, but the database choice is not yet approved.
- Fast repeat refreshes, durable resumability, and an annual rollover that adds the next class while reusing qualified existing evidence.

Retain captures and useful observations for correction, audit, and candidate fallback. A career-history presentation is not required; retaining only today's winner is insufficient.

### Initial and future scope

The first delivery profile is graduating class 2027, high-school boys' and girls' XC plus indoor and outdoor TF, in Michigan, Minnesota, Iowa, Illinois, and Colorado (`MI,MN,IA,IL,CO`). Prioritize Michigan's Upper Peninsula with an explicitly defined school/program membership and scheduling policy. A separately scoped UP qualification slice precedes broader five-state publication; neither slice nor five-state success means national completion.

The long-term national profile remains the lower 48 states and Washington DC, exactly 49 jurisdictions. Geography, regional membership, priority, graduation years, season windows, event policy, and publication time MUST be configuration, not hardcoded into athlete identity or storage keys. Scope and policy identity must still be carried through durable work, coverage, and publication contracts.

NCAA recruiting relevance does not restrict the database to NCAA championship event names. Include legitimate high-school TF/XC variants, preserve separate comparison classes, and include javelin and hammer where qualified public evidence exists. Exclude competitive walking and wheelchair events from the recruiting output under the explicit event-level policy in Section 7. Dataset inclusion establishes neither NCAA eligibility nor admission, scholarship qualification, or a recruiting performance standard.

Support an explicit 50-state-plus-DC profile without redesign; do not silently include Alaska and Hawaii in a 49-jurisdiction completion claim. Territories require a separate extension. Multiple classes coexist without relabeling athletes or erasing evidence.

School and program discovery includes schools that have not yet produced an accepted athlete. The national target is broad public-source discovery, not enrichment of a private admissions list. An old workbook is not the population denominator.

## 2 Architecture and ownership boundaries

Use these priorities when requirements compete:

1. Correct athlete, cohort, event, mark, affiliation, and contact semantics.
2. Durable evidence and completed effects, with recoverable incomplete work.
3. Honest coverage, failures, freshness, and publication readiness.
4. Minimum repeated acquisition and recomputation.
5. Measured throughput, memory use, storage cost, and operator effort.

Preserve the current Rust single-node direction. Restate remains the proposed durable-work owner and Fjall the evidence and decision owner unless a measured, approved design decision changes that allocation. One process owns a Fjall root. No second maintenance or export process opens a live store to work around its ownership lock. Expose bounded read and export operations through the owner or documented consistent snapshots.

Use deterministic Rust for normalization, comparison, validation, and publication. Local models may advise on genuine identity ambiguity; they must not fabricate source facts, select unsupported cohort membership, silently merge people, or bypass deterministic acceptance rules. Model failure leaves an explicit review case. Advice caches bind their exact evidence, prompt/schema, model identity, and adjudication policy so changed evidence is not answered from an obsolete verdict.

Keep one implementation of domain rules shared by the workbook, CSV, audit output, and any future query layer. Prefer small end-to-end improvements over a new orchestration framework, parallel scraper, or speculative engine replacement. Build persistent indexes only for demonstrated query/update needs and validate their maintenance cost.

Specify and test transaction, isolation, flush, and recovery behavior against the pinned Restate and Fjall versions. Orchestration replay alone does not establish exactly-once database effects, durable raw bytes, or a consistent backup across both systems. Fjall retains acquired evidence and resolution decisions; the Rust pipeline generates finished datasets from that state under Restate orchestration. Any separate query database receives accepted finished generations; it does not participate in acquisition or become a second evidence authority.

## 3 Public discovery and coverage

Maintain a versioned source registry for athletic result providers, state associations, timing companies, meet hosts, school and district athletics pages, and public school directories. Examples to evaluate include MileSplit, Athletic.net, DirectAthletics, TFRRS high-school material, AthleticLIVE, and local official timing or school sources. Listing a candidate provider does not certify access, coverage, correctness, or an implemented adapter.

Each source entry MUST declare:

- Provider namespace, allowed public origins, acquisition method, scope, and qualification status.
- Geography, sports, seasons, event classes, and entity types it can actually supply.
- Discovery entry points, pagination/continuation rules, source-owner identifiers, and completion evidence.
- Refresh policy, retry ownership, request and byte limits, and source-specific freshness semantics.
- Parser/schema version, fixture set, known limitations, and evidence needed for promotion.
- Policy refusal, human-required challenge, temporary outage, exhausted retry, malformed content, valid empty result, and successful partial result as distinct outcomes.

Use permitted public HTTP or browser acquisition. Respect source access policies and rate limits; do not spoof identities, replay authentication cookies, bypass paywalls or challenges, or treat archived access-policy fixtures as current permission. Browser challenges become a durable HumanRequired state. Broad coverage is achieved by qualifying more sources and reusing evidence, not by defeating access controls.

Meet-first result discovery and school/program discovery should reinforce each other. An athlete profile, a meet result, and a school roster are complementary evidence paths. The source graph must retain newly discovered links beyond the first hop within explicit depth, byte, task, and origin budgets. Duplicate URLs and linked resources are deduplicated without collapsing distinct season or ownership context.

### Explainable coverage

For each configured jurisdiction, source family, season, and discovery obligation, report whether it is not attempted, scheduled, active, successful complete, successful partial, valid empty, retry deferred, HumanRequired, policy blocked, exhausted, or structurally invalid. Report known denominators separately from unknown denominators. A timeout, missing page, unsupported adapter, or parser failure is never evidence that no athletes exist.

Publish useful partial datasets when explicitly labeled partial. A five-state manifest names all five jurisdictions; report Michigan UP progress separately from the rest of Michigan. Use school/program × sport/category × season denominators with source-backed co-op membership and explicit unknown coverage. Championship finals alone cannot establish season-long or all-athlete coverage; include qualified regular-season, section/regional, indoor, and relevant club/open evidence paths. Reserve a completed or sealed release for the declared acceptance policy and disclosed terminal gaps. Neither a large row count nor checking all known URLs proves exhaustive coverage of the public Internet. Each release must state which source graph and scope were actually evaluated.

### Privacy and purpose limits

Collect public athletic and school/program evidence necessary for recruiting. Public coach contact means a source-published professional mailbox or program contact tied to its school and role. Do not infer private contact details or add private athlete addresses, private email, birth dates, health data, or unrelated sensitive attributes. School postal addresses are distinct from athlete home addresses. Retention and access policies must reflect that many athletes are minors. Retaining useful source evidence is not permission to accumulate unrelated personal information indefinitely.

## 4 Query profiles and cohort membership

A versioned query profile must include:

- `graduation_years`: an explicit set, initially `[2027]`; switching to `[2028]` must not mutate athlete identities or erase other cohorts.
- `jurisdictions`: the actual school-location scope and a named profile for convenience; first delivery is exactly `MI,MN,IA,IL,CO`, and the distinct national profile contains 49 jurisdictions.
- `regional_membership` and `discovery_priority`: optional versioned school/program sets and their public-source membership evidence, sport/season applicability, stable school IDs, and co-op handling. Distinguish a UP-only membership filter from UP-first scheduling within the full five-state scope; a priority does not shrink the completion denominator.
- `source_registry`, `acquisition_window`, and `completion_fence`: the admitted source obligations, observation period, and exact scope/revision required for this publication. A partial or subset fence cannot satisfy the national fence.
- `sports` and `competition_categories`: the requested TF/XC and source-reported competition divisions. Preserve unknown or conflicting classifications instead of inferring them from names.
- `as_of`: the effective cutoff for the current recruiting view; the selected committed data revision separately determines what evidence was known when the export was built.
- `performance_window`: by default, all retained eligible high-school performances through `as_of`. A season-only or date-bounded product must be an explicit alternative. The output must never imply that unobserved earlier results were searched.
- `cohort_policy`, `event_policy`, `identity_policy`, and `contact_policy`: version identifiers governing evidence resolution, comparable marks, matching, and contact freshness.
- `include_provisional`: default false for the primary recruiting export. Ambiguous records remain available in a separate exception view and count toward the coverage report.

Geographic membership is based on the athlete's resolved school affiliation as of the export cutoff, not the venue of a meet. An athlete at an in-scope school may have an eligible mark from an out-of-scope meet. Known transfer-out, unresolved school, independent or home-school, and club-only cases need explicit inclusion or exclusion reasons. Do not silently assign a club's address as a high-school address. Support these cases in the model even if the initial profile excludes some of them. Apply resolved geographic and event membership again at publication, not only during acquisition: a whole-meet capture can contain athletes from other states, grades, or disciplines. Store-wide export over a reused store must not leak out-of-scope records into the requested recruiting view. Preserve excluded evidence and reason counts under the retention policy. A roster count limit, athlete-name search, or timer's location is not a regional membership definition.

### Evidence and conflict resolution

Graduation year is a resolved claim, not an immutable identity attribute. Preserve each explicit class-year claim and each grade-based derivation, including the academic year used in the derivation. A grade alone without an interpretable date or school year cannot establish a graduating class. Do not increment a stored athlete's graduation year merely because the calendar year changes.

Resolution must use current, specific, attributable evidence under a documented policy. An explicit graduation-year statement can establish a stronger claim than an unsupported grade calculation, but source priority alone must not silently erase contradictory class-year claims. A conflict is resolved only by accepted explicit source correction, defensible dated interpretation showing that the claims concern different periods, or an evidence-backed reviewed decision. Preserve the competing assertions and decision. Unresolved material contradictions produce `conflicted`, never “last writer wins.”

Expose `graduation_year`, `cohort_status` (`confirmed`, `inferred`, `conflicted`, or `unknown`), the evidence date, and a primary provenance reference. A defensible dated grade derivation may be included as `inferred` under the selected profile; conflicted and unknown athletes must not silently enter the confirmed cohort. Preserve them for targeted rechecks and report their counts. Manual resolutions must cite evidence and remain reversible.

Changing the selected cohort creates a new projection over the same store. Discovery and refresh prioritize its gaps while retaining already observed younger and older athletes. Graduation class, competition season, academic year, eligibility interval, and school affiliation remain separate concepts.

## 5 Logical records and reversible identities

These are logical records within the Fjall-backed pipeline, not a mandate for separate services. Their resolved values produce the finished recruiting dataset.

| Record | Required content and role |
| --- | --- |
| Athlete | Opaque permanent `athlete_id`, display name, retained name variants, lifecycle state, source-profile bindings, and resolution status. No inferred birth date, home address, or personal contact fields are required. |
| Source identity binding | Source namespace, source athlete identifier, athlete ID, evidence, effective/revision range, and active or superseded status. A source ID is namespaced and must not be assumed globally unique or immune to reuse. |
| Cohort assertion | Athlete/source identity, claimed graduation year or observed grade, school year or relevant date, source/capture reference, observed time, evidence type, and resolution status. |
| School | Stable `school_id`, names and aliases, external identifiers when available, institution type, operational status, and sourced postal address assertions. Track school campuses distinctly when they are distinct institutions. |
| School affiliation | Athlete/source identity, school ID, sport when relevant, asserted effective interval or dated observation, evidence, and status. Uncertain dates remain uncertain. |
| Source capture | Source URL or stable resource key, fetch time, request/response and completeness status, content reference when present, and outcome. Captures do not own parser versions; interpretations do. |
| Source interpretation | Observation reference; parser artifact/schema/configuration versions; deterministic extracted claims; source locators; and extraction outcome. Reparse creates a new interpretation without altering captures. |
| Performance observation | Stable source result key where available, immutable observation/version ID, athlete binding, meet/date, school at performance, participation level and eligibility evidence, event attributes, raw mark, normalized mark, units/precision, result status, and provenance. |
| Contact and role assertion | School, public professional contact, name if published, role, sport/division coverage, official publication URL, business contact channels, effective dates when stated, first/last observation, last successful verification, and current/stale/ended/unknown state. |
| Resolution decision | Target IDs, decision type, resulting mapping or resolved value, evidence, rationale, actor or rule version, revision, and superseded/reversed decision reference. |
| Best-mark projection | Athlete, comparison class, chosen performance version, tied-result count, candidate count, policy version, and committed data revision. Rebuildable from retained evidence. |

All mutable facts must distinguish when the fact applied (`effective_at` or an interval, when known) from when the database learned it (`observed_at` and committed revision). Unknown effective dates must not be fabricated from a crawl timestamp. A simple append-only assertion history plus current-value views is adequate; a general-purpose event-sourcing system is not required.

### Athletes schools and reversible merges

Use opaque, never-reused IDs for athletes and schools. Stable source IDs are the strongest routine binding signal, but retain source context and detect contradictions. Name, graduation year, school, event mix, location, and contemporaneous results can provide matching evidence. A same-name or name-plus-school match alone is insufficient to silently combine athletes. Cross-source matching must have a documented, conservative automatic threshold; uncertain matches go to a review queue rather than contaminating a best-mark record.

Merging must preserve the original IDs and source-level evidence. Store a versioned equivalence decision and a canonical redirect rather than destructively moving and deleting all records. The canonical ID should be selected deterministically, for example the oldest established ID. Existing IDs must remain resolvable. Do not recursively merge a school or contact merely because two athletes merge.

An incorrect merge must be reversible. A split must restore the original bindings where evidence permits, assign uncertain observations to review, and recompute affected cohort, school, and best-mark projections. Published aliases must expose redirects, revoked redirects, and affected record keys; an exported change log is optional. A previously published merged ID cannot be silently reassigned to a different person. If a split creates ambiguity for consumers, preserve the old ID as a deprecated/ambiguous alias and expose the successor IDs with their relationship.

Schools need the same basic protection against conflating similarly named institutions. A name change can retain a school ID when evidence shows institutional continuity; separate campuses, closures, and consolidations require explicit relationships and review rather than address-based guessing.

## 6 Capture content and interpretation identity

Do not use a run ID, URL, athlete name, or body hash as a universal deduplication key. Use versioned, domain-separated identity encodings and retain the fields needed to explain every key.

| Identity | Meaning and proposed construction | Reuse rule |
| --- | --- | --- |
| Source resource key | Stable source namespace plus normalized resource locator, request method and meaningful parameters, representation variant, and access scope. Canonicalization rules are versioned. Preserve the original locator and redirect chain separately. | Equivalent requests may share a key only when the source contract establishes equivalence. Query parameters or representation variants must not be discarded merely to improve deduplication. |
| Capture ID | One durably allocated acquisition attempt, with scheduled reason, source key, start and end times, request validators, response status and headers, redirect chain, transport outcome, and any content reference. | Replay of a committed attempt returns its receipt. Every genuinely new network attempt receives a new capture ID, including a later fetch of identical bytes. |
| Content ID | A cryptographic digest of a precisely defined retained byte representation, including hash and representation versions. Record length and verify the digest on read or scrub. | Identical retained bytes may share a blob across captures and sources. Content equality is not proof that two sources or two athletes are equivalent. |
| Observation ID | An immutable source document version: source resource key, representation contract, and content ID. A capture-to-observation edge records each successful acquisition or valid revalidation. | Repeated identical bytes for the same resource reuse the observation while adding fresh provenance. Identical bytes at a different source retain distinct source observations. |
| Interpretation ID | Observation ID plus extraction/parser artifact digest, schema version, configuration, and the exact reference-data or normalization versions used by that stage. Extracted claims have deterministic child IDs and source locators. | The same inputs reuse the same interpretation. A parser or rule change creates a new interpretation without overwriting the old one. Separate extraction from entity resolution so a mapping change need not repeat expensive document parsing. |
| Application effect ID | Target namespace and key, operation kind, accepted input revision or desired-state generation, and projection/rule version. | Duplicate delivery of the same effect cannot mutate semantic state twice. A genuine correction, retraction, identity remap, or return to an earlier source version has a new accepted revision and can legitimately change state. |

For HTTP bodies, define the default content representation as the complete entity bytes after transport and content decoding but before text normalization; preserve the decoding recipe, media type, charset declarations, and necessary response metadata. Keep compressed or wire artifacts separately when a source requires them. Stream with both encoded and decoded size limits. Persist an allowlisted request description and access-scope identifier, never reusable credentials or authentication headers.

Preserve competing parser claims, source-native record IDs, and raw document locators. Names and row positions alone cannot establish identity.

The required example is: a resource returns bytes A on Monday and A on Tuesday. There are two captures and two acquisition timestamps, one retained content object, one source observation, and one interpretation per parser dependency set. Tuesday advances freshness and adds provenance without adding duplicate performances. If the source later returns B and then returns A again, the old content and interpretation may be reused, but accepted source revisions A1, B2, and A3 remain distinct. A receipt for A1 must not prevent the legitimate B2-to-A3 transition.

A 304 response is a new capture and a revalidation edge to a previously retained observation, not a new body. Failed or ambiguous attempts remain attempts; they do not manufacture observations or successful-freshness timestamps. Acquisition time, source-reported publication or modification time, and the event's effective date are separate fields. None substitutes for another.

## 7 Comparable best marks and retained candidates

Define an event catalog and versioned comparison classes before choosing best marks. Each class must specify ranking direction, normalized unit, allowed statuses, relevant technical conditions, and a stable class ID. The grouping must be specific enough that a “best” value means the same thing for every candidate.

### Recruiting event inclusion and exclusion

Use a positive versioned event catalog, controlled aliases, and a typed disposition: included comparison class, excluded discipline, or unresolved event requiring review. Retain the original source label and event context. Unknown, mixed, or unsupported labels must never silently map to the nearest distance. Event-family inclusion admits source-verifiable high-school participation under the profile; it does not certify every event in every source as relevant or covered.

The initial recruiting catalog covers these families and separately represented variants:

- Track running: standard sprints, middle distance, and distance races, including 55, 60, 100, 200, 300, 400, 500, 600, 800, 1000, 1500, 1600, 2000, 3000, 3200, 5000, and 10,000 meters, mile, and two miles where legitimately contested in the configured high-school participation scope. Other scholastic track distances require an explicit catalog entry and source evidence.
- Hurdles and steeplechase: 55, 60, 100, 110, 300, and 400-meter hurdle variants, plus steeplechase at its actual distance, including scholastic 2000 and 3000 meters. Retain height, spacing, and barrier configuration. Flat running is a different event.
- Throws: shot put, discus, javelin, hammer, and indoor weight throw. Include javelin and hammer when qualified public marks exist even if the athlete's state school championship does not offer them. Preserve implement mass and material configuration. Unknown implements remain unknown; weight throw is a distinct event from hammer.
- Jumps: high jump, pole vault, long jump, and triple jump, retaining applicable technical and wind conditions.
- Combined events: pentathlon, heptathlon, decathlon, and explicitly cataloged scholastic variants. Retain component lineup, category, scoring-table version, and completion status. Different lineups or scoring rules form separate comparison classes; incomplete totals do not become completed-event bests. Component marks retain their combined-event context.
- Relays: 4×100, 4×200, 4×400, 4×800, distance medley, sprint medley, and documented scholastic variants. Preserve exact leg composition, category, and evidenced membership. Team results and individually attributable splits have separate classes; an athlete never inherits the full team time as an individual mark.
- Cross country: actual published scholastic distances, including 3 miles and 5000 meters. Preserve course, meet, terrain/measurement information when available, and approximate-distance status. Collegiate championship distances are not a reason to discard legitimate high-school XC evidence.

Exclude racewalking, speedwalking, powerwalking and other competitive walking events, wheelchair races, and handcycle events from the recruiting output. Road races, marathons, ultras, non-XC trail/mountain races, triathlon, and unrelated sports are outside this TF/XC profile. Preserve source evidence and explicit event exclusion reasons where retention is justified; do not relabel excluded events as ordinary running. These are event-level decisions: an athlete's other qualifying observations remain eligible. Never infer disability, medical status, or person-level exclusion from an event entry. Generic labels such as “adaptive” or “special” alone do not classify a performance; resolve event evidence or record uncertainty.

This exclusion is the requested product policy, not a statement that NCAA schedules never feature wheelchair events. The [2026 NCAA indoor schedule](https://ncaaorg.s3.amazonaws.com/championships/sports/crosstrack/d1/indoortf/2025-26D1XTI_CompetitionSchedule.pdf) and [2026 outdoor participant manual](https://ncaaorg.s3.amazonaws.com/championships/sports/crosstrack/d1/outdoortf/2025-26D1XTO_ParticipantManual_Oregon.pdf) provide collegiate event context, including throws, multis, and relays; the outdoor manual also includes wheelchair inclusion-project races. They do not replace the high-school event and participation policy.

### Comparison and eligibility rules

At minimum, preserve distinctions for:

- Sport, event distance/type, and competition category.
- Indoor versus outdoor track; hurdle configuration, throwing implement specification, and other material event variants.
- Fully automatic, hand-timed, and unknown timing. Do not silently compare converted marks with observed marks.
- Wind legality where relevant. Legal, wind-aided, and unknown-wind marks must remain distinguishable. A documented policy may define an additional all-conditions view, but the primary output must identify its class explicitly.
- Individual events, relay team results, and relay splits. Team times must never become an individual's personal best. Relay splits require a separate opt-in class and trustworthy individual attribution.
- XC distance and any documented course/measurement distinction. A fastest observed XC time across different courses can be useful, but must be labeled as such and retain course/meet context. It must not imply standardized cross-course equivalence. Course-specific comparison can be a separate configured class.

Keep 1500 m, 1600 m, and the mile separate; keep 3000 m, 3200 m, and two miles separate; keep 300 m and 400 m hurdles separate. Do not treat 3-mile XC as 5000 m. Do not silently normalize an approximate XC distance into an exact measured distance. Exact unit conversion is allowed when the original precision is retained; modeled event/time conversions belong in a separately labeled optional product and must not replace observed best marks.

Performance eligibility must establish that the result belongs to the athlete and the configured high-school participation scope. Club, unattached, or open-meet participation is neither automatic disqualification nor proof of high-school eligibility. Require performance date, independently evidenced high-school status, cohort membership, and an explicit versioned participation window, including the summer boundary. Keep the represented club/team separate from school affiliation. An aggregated profile or USATF age band alone cannot establish high-school participation or graduation year. Collegiate, middle-school, post-graduation, and unresolved observations must not silently enter accepted high-school bests; uncertain cases follow the configured provisional policy. Projected collegiate-implement throws, adjusted hurdle performances, or distance-equivalent times cannot replace observed marks.

Parse raw marks into exact decimals or integer base units with explicit precision; avoid floating-point tie decisions. Preserve the original display mark. DNS, DNF, disqualification, no-mark, withdrawn, malformed, conflicting, or materially incomplete observations must be retained as evidence but excluded from a valid-best calculation with a machine-readable reason. Unknown attributes must never silently inherit the most favorable class. Plausibility flags can trigger review, but must not invent a corrected mark.

The primary best-mark invariant is exactly one row per `(athlete_id, comparison_class_id)` with at least one eligible observed candidate at the selected data revision. Select the minimum normalized value for timed events and maximum for distance/height/points classes as defined by the catalog. The value is the best eligible mark in the retained, stated observation scope, not a guarantee about every result the athlete has ever achieved.

Equal normalized marks at the same comparison precision are ties. Keep all tied candidate references internally, deduplicate duplicate publications of the same performance, and export one deterministic representative. Use documented provenance preference, then earliest known performance date, then stable performance ID to select that representative. Export `tied_performance_count`; an optional evidence file can enumerate ties. Do not manufacture ranking differences through source formatting such as `10.50` versus `10.500`.

### Corrections withdrawals and candidate fallback

A repeated unchanged capture must be idempotent. Re-parsing the same evidence with a new parser must create a traceable interpretation revision without creating duplicate performances. When no stable source result ID exists, derive a conservative source-local key from the available meet/event/participant/result context and retain ambiguity instead of forcing a collision-prone universal key.

Keep every useful candidate mark, not only the current winner. If a source corrects a mark, changes its attribution, disqualifies a result, or explicitly removes it, supersede or withdraw that observation version and recompute affected classes. A deleted or invalidated 10.70 best must fall back to a retained eligible 10.82 without requiring that 10.82 to be rediscovered. If no eligible candidate remains, remove the best row in the next revision and record a deletion in the internal semantic change journal; do not leave the old winner as current.

Page omission is not automatically a deletion. A failed request, incomplete pagination, changed filter, inaccessible season, or partial parser output must not erase prior observations. Treat removal as established only through explicit source status, a confirmed complete replacement of the same resource/scope, or a documented reviewed decision. Preserve unresolved disappearance as a verification issue. When two sources disagree about the same performance, retain both assertions and the resolution; duplicate corroboration must not resurrect a result known to be disqualified or withdrawn. Unresolved material validity conflicts should quarantine the candidate until resolved.

Recompute all affected projections when identity, cohort, event classification, affiliation, result validity, or source corrections change. Reuse unchanged captures and reprocess retained evidence when parsers or policy versions change. Raw-capture retention may be storage-tiered under an explicit retention and access policy, but enough evidence and all relevant candidate observations must remain to reproduce the recruiting view and recover from a bad winner. Do not require unlimited retention of irrelevant pages or prohibited/private material.

Source removal changes evidence availability and confidence under an explicit rule; it does not automatically retract independently supported facts. A legal or privacy-driven purge is a separate, auditable operation with defined derived-data and backup consequences. Ordinary refresh and garbage collection must not imitate a purge.

## 8 School affiliations addresses and contacts

A transfer must not create a new athlete. Preserve the school recorded with each performance and maintain a separate current affiliation as of the export cutoff. A best achieved at School A remains attributable to School A even when the athlete now attends School B. The recruiting directory points to School B and its current coaching contacts. Concurrent or ambiguous affiliations must be marked, not forced into a single school without evidence.

Store school postal addresses as sourced institution data: address lines, locality, region, postal code, and country, plus address type, verification time, and source. Differentiate physical campus, mailing, and district-office addresses. Prefer a verified school campus address for the recruiting view; if only a district or mailing address is available, label it. Do not substitute an athlete residence or infer missing components.

A coaching contact is a time-bound relationship to a school and sport, not a permanent field on an athlete. A school may have multiple current coaches with distinct TF/XC, head/assistant, or division responsibilities. Preserve these relationships and support a published athletics-office fallback without falsely labeling it a coach.

Only collect published professional contact channels relevant to the role, such as a school-listed work email, office phone, or official contact page. Do not generate email addresses from naming patterns or enrich with private personal contacts. A non-school-domain email can be retained only when the school or another approved authoritative source explicitly publishes it for that professional role, with provenance. Contact availability does not authorize outreach or onward sharing of athlete data.

“Current” requires source-backed role, school/program, relevant sport/division, and current-season or otherwise explicit tenure evidence as of `as_of`; successful verification within a configured interval; and no stronger superseding departure evidence. The mailbox or contact channel must be attributable to that published professional role, rather than merely appearing somewhere on the school website. A proposed initial `contact_max_age_days` of 90 is a freshness threshold only, configurable by season and source; it does not establish current tenure. An old page without current-season or explicit active-tenure evidence is `published_unconfirmed` even if fetched today. Expired freshness becomes `stale`; known departed coaches become `ended`; missing contacts remain `unknown` or `not_found` with the search scope. Prefer a blank current contact with a reason over a departed or unconfirmed coach labeled current.

## 9 Durable commits and replay safety

Treat fetch, parse, resolution, and application work as retryable and at least once. Promise exactly-once semantic effects only inside a clearly defined local commit boundary. The network cannot be committed atomically with Fjall, and a crash can leave an HTTP attempt's outcome unknown. An unknown attempt must be recorded honestly and retried as a new acquisition when policy permits.

For each semantic application, the implementation must perform the following under an isolated transaction or a serialized writer that owns all conflicting keys:

1. Check the effect receipt and validate that the input revision and dependency fingerprint are still eligible to apply.
2. Compute deterministic inserts, replacements, withdrawals, and index updates. Never use an unguarded increment for replayable counts.
3. Commit the semantic mutation, effect receipt, provenance links, and durable downstream invalidations or outbox entries atomically in the same Fjall durability domain.
4. Acknowledge the orchestration step only after the selected persistence guarantee has completed. If acknowledgment is lost, replay finds the receipt and returns the committed result.

An atomic write batch alone does not make a preceding check-and-write race safe. Choose and verify the concurrency mechanism explicitly. A receipt in Restate and an effect in Fjall are not an atomic pair. Each receipt records its effect ID, accepted input revision, commit sequence, output digest or version, and outcome. If a dependency changed while work was running, discard or retain the obsolete result as historical evidence and schedule current work; do not publish it over newer state.

For large outputs, use bounded deterministic chunks, each with an atomic chunk effect and receipt. Publish a generation manifest or head pointer only after all required chunks are durable and validated. Readers see an explicitly complete generation, or a view that is visibly rebuilding, rather than an accidentally partial result. Old generations remain readable until reader and backup pins have been released.

If content is held in blob files alongside Fjall, first stream it to a bounded staging file, validate the digest and length, and make it durable through the required file and directory operations. Then commit the metadata reference. This ordering may leave a collectible orphan after a crash; it must never leave a committed reference to bytes that were never made durable. Reference creation, pins, and garbage collection need a coordinated protocol so an otherwise unreferenced blob cannot be deleted while a commit is preparing to adopt it.

A durable internal invalidation journal is required to recover affected pipeline projections. A continuously replicated downstream database is not required. The recommended query-database handoff uses a complete, consistent accepted dataset snapshot, as specified in Section 15. If an ordered downstream change feed is later justified, append its events atomically with semantic changes, define a snapshot fence and gap-free continuation, and commit consumer updates with their checkpoint. Declare retention and lag limits; a consumer outside them must rebuild. This is an optional measured optimization, not a prerequisite to completing the first dataset.

External publication, if later added, requires its own durable outbox and destination idempotency or reconciliation contract. Do not extend the local exactly-once claim to an arbitrary remote service.

## 10 Resumable discovery and refresh

Persist the acquisition frontier, not merely the last page number in memory. A work record needs a deterministic logical work key; source and scope; discovery parent; cursor or range; reason and priority; dependency versions; due time; attempt count; claim or lease token; and terminal disposition. States should distinguish ready, claimed, retry scheduled, completed, explicitly deferred, policy blocked, and failed pending review. Expired claims are reclaimable; a fencing token prevents a late worker from overwriting a newer owner.

Commit the processed-page result, newly discovered child work, and next checkpoint together. Advancing a cursor before discovered children are durable can permanently skip evidence. Checkpoints bind to the source contract and traversal generation; an expired cursor restarts the affected traversal with overlap and deduplication, not the entire store. Offset pagination over a changing source needs overlap or reconciliation sweeps; it must not be described as complete merely because an offset reached the end once.

Use one in-flight refresh per source resource by default. If parallel refreshes are justified, establish an explicit acceptance rule using source versions or serialized generations; response completion time alone cannot decide which response is newer. Retain late captures for audit without allowing them to roll back the accepted resource head.

A run manifest records requested scope, cohort definition, source-plan versions, starting checkpoints, admission limits, and completion fence. Completion means that every required work item inside that fence is durably accounted for, including explicit coverage gaps. Deferred work, retry-budget exhaustion, throttled sources, and parser failures are visible outcomes and survive into the next run. They are never silently reclassified as completed discovery. Restate history and Fjall work receipts must have a defined reconciliation path after process restart or restore.

### Refresh and negative evidence

Refresh policy should be source-specific and bounded: prioritize changed pages, active seasons, upcoming or recent meets, unresolved candidates, correction-prone results, and stale high-value evidence. Older sources can move to slower revalidation schedules while remaining available for reuse. Public acquisition should support permitted HTML, structured endpoints, feeds, sitemaps, result files, PDFs, and archived source resources through adapter contracts. Public availability does not authorize bypassing authentication, access restrictions, or site limits.

Each adapter declares its representation identity, pagination and completeness semantics, validators, expected update pattern, rate limits, maximum body and decompressed sizes, and recognizable failure pages. Adaptors must preserve both successful and unsuccessful acquisition provenance. Source admission remains broad, but operational budgets and evidence quality determine scheduling rather than unrestricted fan-out.

| Response or condition | Required behavior |
| --- | --- |
| 200 with identical bytes | Add capture provenance and freshness; reuse content, observation, and eligible interpretation. Do not duplicate semantic claims. |
| 200 with changed bytes | Create a new observation, parse or reuse its interpretation, and stage a source-version transition. Compare claim sets before publishing changes. |
| 304 | Accept only with a valid conditional request and an available retained representation for the same resource and variant. Link the capture to that observation and update freshness. If the referenced body has been evicted or the validator context is missing, fetch a full representation before claiming reparsability. |
| 429 | Respect applicable Retry-After guidance, persist the next eligible time, and reduce the affected host or source budget. Use bounded jittered backoff and shared host coordination, not independent retry storms. |
| Timeout, transport failure, or retryable server error | Persist the attempt outcome and bounded backoff. After the per-window retry budget is spent, defer or flag the work with a future eligibility condition. The budget controls load; it does not erase the obligation. |
| 401, 403, consent wall, or bot challenge | Classify as access or policy blocked. Do not count it as empty data, change routes to evade the restriction, or retry indefinitely. |
| 404, 410, or source withdrawal | Record negative evidence with resource scope, time, policy version, and confidence. Apply the adapter's confirmation rule before changing source availability. Resource disappearance alone does not prove an athlete or result never existed. |
| Empty, truncated, malformed, or soft-error body | Record the body and validation failure where retention permits. Do not activate an empty interpretation or withdraw previous claims merely because parsing produced zero rows. |

Negative evidence is a versioned observation about a specific scope with an expiry or recheck condition. Keep last attempted, last successfully acquired, last revalidated, and last semantically changed times separate. A disappearing row is meaningful only when a successful, validated, complete replacement snapshot covers that row's scope. Missing rows in partial pages, failed traversals, or incremental feeds without deletion semantics are not deletions.

## 11 Offline reparse and incremental projections

Provide distinct durable jobs for acquisition, offline extraction, normalization or resolution, and projection. An offline reparse operates solely on retained content and captured context, pins those inputs, and emits new interpretations under explicit parser versions. It must not silently access the network. It reports missing, corrupt, or intentionally evicted content as gaps; it cannot claim to reproduce evidence that is no longer retained.

Maintain reverse dependencies from observations and claims to resolved entities and projection keys. A changed parser, entity mapping, source revision, eligibility rule, or cohort definition should invalidate only its affected dependency closure. Persist dirty keys and the change journal in the same commit as the triggering change. Coalesce repeated invalidations by key and desired revision; process them in bounded chunks. Validate the dependency fingerprint again before publishing a result computed concurrently with new evidence.

For best marks, invalidate both old and new athlete or event keys after a reassignment. For identity merges and splits, retain versioned aliases and membership decisions, then invalidate every dependent cohort and performance view. Periodic independent reconciliation should compare incremental outputs against a full recomputation over a bounded frozen evidence set. This is a correctness check, not the normal refresh algorithm.

## 12 Storage limits retention and recovery

Durability means recoverable state under stated retention and failure assumptions. It does not mean infinite retention on a finite disk. Define separate retention classes for source bytes, capture metadata, interpretations, candidate claims, receipts, completed frontier items, published snapshots, and backups. Publish a retention manifest showing what remains replayable, what is cold or archived, and what has been intentionally removed. A content digest by itself does not make an evicted document recoverable.

Set enforceable quotas for on-disk bytes, staging and temporary files, response and decompression limits, in-memory queues, worker concurrency, database caches, batch sizes, and pending invalidations. Account for physical storage, including WAL, compaction headroom, pinned generations, and snapshots, rather than only logical payload bytes. Stream large inputs, use byte-budgeted channels, and spill sortable or join work to bounded disk structures. Per-worker limits do not replace a global RAM budget.

Admission control reserves space before accepting new bodies or large parse outputs. Configured soft watermarks first reduce speculative discovery and concurrency; higher watermarks pause new acquisitions while allowing safe commits, drain, cleanup, and recovery. Keep an emergency reserve sized from measured worst-case commit and compaction behavior. If the engine cannot enforce a particular hard bound, expose that limitation and qualify a safe operating envelope. Do not promise bounded disk solely because a queue length is bounded.

Garbage collection should be restartable mark-and-sweep or an equivalently proven reachability protocol. Its roots include active observations, retained historical snapshots, correction-relevant candidates, in-flight job pins, unapplied outbox entries, and backup manifests. Use a grace period and generation fences to protect readers and newly adopted blobs. Reconcile reference counts if used; do not trust a potentially interrupted counter as the only deletion proof. Candidate evidence necessary to recompute the current view is not collectible merely because it is not the current winner.

Receipt pruning requires a replay-horizon contract. Keep receipts or compact equivalently protective generation fences until no surviving orchestration history, restore point, queued message, or accepted old request can replay the effect. A restore must not resurrect an old message after its deduplication protection has been discarded. Age alone is not a sufficient deletion criterion. The same principle applies to retraction tombstones and completed work keys.

Backups need a versioned manifest covering the consistent Fjall checkpoint, every referenced blob, source and schema contracts, parser artifacts or reproducible artifact references, and the Restate recovery or reconciliation position. Use a supported quiesce or consistency-barrier procedure if a cross-system snapshot is unavailable; copying two live directories independently is not a proven backup. Retain tested recovery points outside the node's failure domain under a declared capacity and retention budget. Specify the target recovery point and recovery time, then measure them rather than inventing guarantees.

Restore qualification must run on an isolated instance before resuming network work. Verify checksums, referenced-byte availability, schema compatibility, receipt and frontier consistency, and snapshot readability. Reconcile uncertain in-flight work, replay it idempotently, and compare semantic digests with the expected checkpoint plus recoverable journal. A backup is qualified only after a demonstrated restore, including documented treatment of any gap between orchestration and evidence-store recovery positions.

## 13 Consistent recruiting publications

The primary user deliverable is a usable recruiting workbook, accompanied by a versioned UTF-8 CSV bundle and machine-readable manifest. These artifacts and any query-database load must share one accepted generation boundary. Consumers retain identifiers, statuses, policy versions, and revision identity; downstream edits cannot implicitly overwrite evidence in Fjall.

| File | Row key and essential fields |
| --- | --- |
| `athletes.csv` | `athlete_id`; display name; graduation year/status; competition category/status; current school ID/status; cohort and affiliation evidence references; record revision. Include eligible known athletes even when no valid performance is available, with an explicit performance-availability status. |
| `best_marks.csv` | `(athlete_id, comparison_class_id)`; performance ID and version; event label; raw/display and normalized mark; unit and precision; timing/wind/technical class attributes; meet/date/course where applicable; school at performance; source URL/capture reference; tied and eligible candidate counts; selection policy version. |
| `schools.csv` | `school_id`; canonical name; aliases where useful; type/status; structured school address/type/status; address source and verification date. |
| `coach_contacts.csv` | `contact_role_id`; school ID; published name and role; sports/divisions; work email/phone/contact URL when published; status; effective dates if known; last verification; source URL; fallback type if applicable. Multiple roles must not multiply best-mark rows. |
| `comparison_classes.csv` | Class ID, event identity, sport/category, ranking direction, units/precision, condition requirements, comparability label, and policy version. |
| `provenance.csv` | Referenced evidence ID, source namespace/URL, capture ID/time, interpretation version, and concise evidence type. Raw captures remain in controlled storage rather than being embedded by default. |
| `coverage.csv` | Scope unit, cohort, sport/category, source/scope searched, discovery and fetch state, last successful refresh, observed athlete/result/contact counts, excluded/uncertain counts, and unresolved gaps. Denominators must identify the actual known registry or source universe. |
| `identity_aliases.csv` | Original ID, resolved canonical ID or ambiguous/deprecated state, successor IDs when applicable, entity type, effective revision, and resolution reference. This makes redirects and splits interpretable even for a consumer loading a full snapshot without prior change history. |
| `changes.csv` | Optional companion for any snapshot, including later snapshots; when supplied, changes since a named prior revision: entity type/key, upsert/delete/redirect/split, affected IDs, revision, and reason. Cohort reclassification or a policy filter change must be distinguishable from source deletion. |

The manifest must include `schema_version`, `export_id`, `data_revision`, `generated_at`, `as_of`, full effective configuration, policy/parser versions or their immutable references, source freshness range, file row counts/checksums, unresolved-exception counts, and coverage limitations. Use ISO dates/timestamps, explicit units, a documented null encoding, and stable column ordering. Empty, not collected, not found, conflicted, and excluded must not all collapse into an unexplained blank.

All files must come from one committed data revision and one configuration. Stage and validate an export before atomically publishing its manifest or completion marker. A reader must never receive best marks from one revision with athlete merges or school contacts from another. Failed refreshes retain the last good snapshot with its original freshness dates; they must not advance a misleading “updated” timestamp. Re-running the same revision and policy should produce the same substantive rows, tie representative, and ordering.

Validate referential integrity, one best row per key, candidate eligibility, alias resolution, cohort/geography/regional and event-policy filtering, contact status, and manifest counts before publication. Verify the same membership and exclusions in workbook, normalized bundle, and any query-database copy; excluded events cannot appear as accepted recruiting marks through a summary, relay sheet, or convenience join. Coverage and review outputs may identify their exclusion reasons without presenting them as eligible performances. Every chosen best must be traceable to retained evidence. No validation may turn missing evidence into a positive coverage claim. Report coverage separately for athlete discovery, result availability, school address resolution, and current coaching contact availability; “all attempted pages succeeded” is not proof that all athletes were found.

### Recruiting workbook

The workbook MUST provide an athlete directory, a best-marks sheet with one row per athlete/comparison class, school and professional-contact sheets, and coverage and review sheets. Include stable IDs and source hyperlinks so the workbook can be reconciled with the normalized bundle. A convenience recruiting view may join current school and eligible contacts, but multiple coaches must not inflate counts or create apparent duplicate best marks. Provide filters, frozen headers, readable event labels, explicit units, and visible freshness/status columns. Preserve missing or conflicted facts as labeled states instead of filling them with guesses.

Protect text fields from spreadsheet formula execution while preserving their original values in the authoritative data. Verify workbook row counts, selected marks, cohort membership, source links, and statuses against the matching normalized snapshot. Workbook generation is a presentation step over the shared domain rules, never a second implementation of eligibility or best-mark selection. A failed workbook build leaves the prior accepted bundle intact and the new generation unready. Raw source documents and unrelated personal information are not embedded in the recruiting workbook.

Revised historical exports receive new publication IDs. Keep previously published snapshots immutable and addressable under their original evidence revision and policy; late corrections must not silently rewrite an artifact that a consumer has already received.

## 14 Annual operation and configuration

The annual workflow MUST be:

1. Open the established service and store through its normal owner, verify schema and integrity, and confirm a restorable backup exists.
2. Add or select graduation year 2028 and its explicit season/as-of/scope profile. Keep 2027 data and immutable prior publications intact.
3. Resolve existing evidence against the new cohort profile. A published 2028 graduation claim remains 2028; a grade observation is interpreted using its source academic year, not the current wall clock.
4. Build refresh obligations for new seasons, new sources/schools, stale resources, prior gaps, and changed policies. Reuse fresh qualified evidence and unchanged retained content.
5. Acquire and interpret due material, reconcile only affected identities, cohorts, affiliations, contacts, and best-mark slots, and resume deferred work within its budgets.
6. Publish a new revision with the requested cohort and current contact policy. Verify the complete artifact bundle against the same frozen input revision.

A new session is distinct from a logical source obligation and a publication. Annual operation must not require year-constant edits, store deletion, replacement athlete IDs, or new run keys that evade replay protection.

### Operator capabilities

Expose these operations through the existing CLI/service conventions; these are capabilities to implement, not runnable command syntax:

- Initialize a new installation with validated storage/resource/source profiles.
- Inspect status and open work without opening the live database independently.
- Preview a synchronization plan showing due, reusable, blocked, and estimated work before acquisition.
- Synchronize selected scopes and sources with a finite work budget or continuous scheduled refresh policy.
- Resume the same logical work after interruption, without duplicating accepted effects.
- Add/select a cohort profile and publish a consistent export.
- Reinterpret retained captures with a named parser version and zero network access. Missing inputs produce a separate reacquisition plan, executable only through an acquisition operation.
- Recompute affected projections after an identity, cohort, comparison, or publication-policy change.
- Inspect provenance, conflicts, freshness, source coverage, and the reason a field is missing.
- Verify, back up, restore, and migrate through documented ownership-safe workflows.
- Stop intake, drain bounded in-flight work, persist unfinished obligations, and report the exact outcome.

Each operation MUST have machine-readable status and a meaningful exit/result contract. A completed command with accepted terminal gaps is distinguishable from full success, retryable failure, human action required, or incomplete work. Errors cannot be hidden behind a zero failed-count while success/empty counts increase.

### Installation configuration and release identity

An installation profile MUST define the following explicitly:

- Stable installation/store identity and schema version.
- Target cohorts and geographic/sport/season profiles.
- Event comparison, identity, cohort, contact-currentness, and publication policy versions.
- Source registry versions and per-source refresh/retry/concurrency rules.
- Maximum response bytes, parse work, queues, browser processes, model requests, and total in-flight bytes.
- Disk quota, low-water stop threshold, staging reserve, retention and backup policy.
- Publication freshness requirements, allowable disclosed terminal gaps, and artifact destination.

A sync attempt records operational timing and outcomes. A source observation records when material was actually acquired. A semantic data revision records committed facts/decisions. A publication records its input revision and policy. These are separate concepts; changing an attempt timestamp alone must not create new facts or make old evidence fresh.

Defaults must be inspectable. Qualify tuning on the actual workstation and representative corpus; report network-bound and offline performance separately.

Storage-schema changes use explicit migrations with compatibility checks, backup prerequisites, and a restartable migration journal. Migration progress is not hidden inside an annual reset. Qualification must include successive annual cohorts in one populated store and an independently rebuilt reference result.

## 15 Finished dataset query database boundary

The Fjall-backed pipeline performs scraping, durable processing, evidence retention, resolution, and finished-dataset generation. The user-facing requirement is easy access to the finished data once collection and generation are ready. A separate relational database is a recommended consumer, not an approved engine replacement or a mandatory live replica. SQLite is the current recommendation for a local, readily queryable dataset; DuckDB is an alternative for analysis-heavy use, and a server database such as PostgreSQL would need a concrete multi-user or service requirement. Final selection and detailed schemas follow this specification.

If a query database is selected, the first implementation SHOULD load a full accepted snapshot per generation. Build a staged database from the manifest-backed normalized bundle or an equivalent owner-produced snapshot. Preserve export ID, Fjall data revision, schema/policy versions, `as_of`, coverage, checksums, and import status. Importing the same generation is idempotent. Validate the staged database, then publish it through an atomic generation switch with a reader-safe replacement protocol. Interrupted imports cannot become current; scraping must not depend on live dual writes.

The relational model should make athletes, current school/cohort membership, best marks, comparison classes, school addresses, professional contact roles, identity aliases, necessary provenance, and coverage easy to join. Use foreign keys, uniqueness constraints, and indexes justified by actual queries. Preserve one best row per athlete/comparison class and model multiple coaches without multiplying performances. Include eligible athletes lacking marks and honest unknown/currentness states. Raw bodies, retry queues, parser scraps, and every intermediate pipeline record need not enter the query database; provenance needed to explain published data does.

Fjall remains the evidence and generation authority. The query copy is rebuildable, with no dual-authoritative writes or implicit reverse synchronization. Future user annotations need separately owned records. Recover the copy by reimporting an accepted generation. Ordered incremental updates are optional only if measured full-load cost justifies them; they must preserve consistent generations and recovery. No database creation, population, migration, or engine choice is claimed as completed here.

## 16 Qualification and performance evidence

Acceptance has three separate decisions: implementation qualification of an exact build; completion of a configured run with all admitted obligations accounted for; and approval of a publication under its coverage and recruiting rules. A completed run may retain explicit terminal gaps and still be unsuitable for a completed publication. Fresh-store qualification and annual production reuse are both required.

### Rigorous Rust engineering requirements

The requested Holzman-inspired and “NASA-level” rigor means measurable safety, correctness, bounded-resource, and recovery obligations. It does not mean NASA certification, aerospace compliance, or flight-software assurance. The repository's published [engineering standards](https://github.com/lprior-repo/athletic-rust-pipeline/blob/90dd05f0a03d3e119c4cae531774df30973bd915/ARCHITECTURE.md#10-engineering-standards-37-43) and [working instructions](https://github.com/lprior-repo/athletic-rust-pipeline/blob/90dd05f0a03d3e119c4cae531774df30973bd915/AGENTS.md) remain the concrete reference.

The named `rust-contract`, `holzman-rust`, `async-rust-reviewer`, and `scott-ddd-refactor` skills were absent from the supplied skill catalog and inspected checkout skill paths. Their exact instructions have not been applied. Restore and read them before claiming skill-conformant implementation; this specification applies the published repository standards without pretending to have invoked missing skills.

Required implementation and qualification discipline:

1. Freeze validated domain types, checked constructors/deserialization, exhaustive states, and explicit transitions before implementation. Separate raw observations, validated claims, identity candidates, and accepted decisions. Keep pure domain rules free from runtime, storage, network, browser, and model dependencies.
2. Enforce project-owned `forbid(unsafe_code)` and `deny(unused_must_use)`, checked arithmetic, complete typed outcomes, no input-triggered panics, no production recursion, and the repository's full prohibited `unwrap`/`expect` family across owned Rust, including tests. Do not replace missing outcomes with fabricated defaults or waive a failing rule. Preserve `thiserror` in production and `anyhow` only at CLI/composition boundaries.
3. Preserve the published zero-project-code-comment policy and source/callable size rules: production files within 300 lines; handwritten production callables at most 60 trusted logical lines, with 25 the preferred advisory threshold. More than five parameters remains a cohesion-review warning, not an excuse for artificial context bags or a new hard failure. Trusted checks must resist statement packing and distinguish source from captured evidence.
4. Declare finite whole-workload budgets, admit before spawning, supervise every task, avoid locks across awaits, and bound blocking work. Prove cancellation, timeout, abort, panic, drain accounting, replay idempotency, stale-writer fencing, atomic effect/receipt behavior, and preservation of the last accepted generation.
5. Run repository build, test, check, lint, formatting, benchmark, and gate work only through `tools/moon-local`. All-target compilation, strict applicable lint/format/architecture checks, dependency and security review, property tests, bounded parser-fuzz campaigns, real CLI/Restate/export readback, and all 17 cataloged native fault scenarios remain required as owned by the release contracts. A passing ordinary CI or release wrapper does not prove separately unexecuted fault/fuzz lanes. Do not restore retired requirements or weaken existing gates.
6. Record exact build, corpus, configuration, commands, exit codes, selected test counts, fault witnesses, resource peaks, and raw evidence. Report passed, failed, blocked, not run, and inconclusive separately. Source inspection, new tests, or an inspiring quality label cannot establish a runtime pass.

### Independent correctness oracle

Prepare an expected-result manifest independently of the candidate reducer, resolver, and incremental planner. Use fully enumerable fixtures plus enumerated reference subsets for larger corpora. Compare complete multisets with stable semantic keys, preserving multiplicities, every output family, statuses, exclusions, and provenance. Comparing two modes that share the same faulty reducer is insufficient. Semantic fingerprints may exclude presentation-only generation time, never decision-relevant values.

Include each event family and every material comparison distinction with independently expected results. Exercise racewalking and wheelchair aliases, ambiguous category labels, athletes with both excluded and included events, javelin/hammer/weight and unknown implement specifications, legitimate scholastic distances, club/open participation windows, and out-of-region meet entrants. Verify event exclusion never becomes person-level inference. Include corrected/withdrawn winners with retained fallback; equal marks and duplicate syndication; same-name athletes/schools; cohort contradictions and reclassification; transfers; merge/split reversal; invalid and incomparable events; former, sub-varsity, stale, and unconfirmed coaches; unknown contacts; and relay teams with missing membership. No relay team becomes an individual mark. Preserve declared unmatched and many-to-many join semantics. Every selected fact must trace to its evidence and exact rule.

Prove the oracle fails on missing/extra/duplicate rows, a missing export family, hidden parser failure, unsupported checkpoint, empty test selection, skipped required lane, and a wrapper that prints PASS after child failure. Compare logical physical-observation identity and content independently of sequence-bearing storage keys and interpretation receipts. Preserve baseline multiplicity and detect forbidden increments even if the baseline already contains duplicates.

### Numeric operating envelope

Before execution, require finite numeric limits for whole-workload memory and headroom; encoded/decoded response and row/key/field sizes; parser work; global and per-stage concurrency and byte queues; batch sizes; sort/join/identity working sets; spill size/fan-in/open files; database/evidence/WAL/staging/compaction/publication/backup disk; physical requests, wire bytes, redirects, host rates, retries and deadlines; shutdown time; and retention. Missing or zero-as-unlimited limits are configuration errors.

Measure the whole process group, including browsers and declared page-cache accounting, rather than only engine heap. Charge variable payload and container overhead. Sort/join skew and giant identity components must spill, use a qualified bounded plan, or stop resumably without changing global semantics. Reserve actual disk overlap atomically and reconcile counters against a bounded physical inventory after recovery. Unknown inventory is not free capacity. Count requests at dispatch, including retries and redirect hops; state the browser-subrequest accounting scope. Recheck host policy and budgets immediately before every transport dispatch. An OOM kill, unhandled full disk, silent truncation, or averaged-away ceiling breach fails qualification.

### Benchmark cases

Each case binds exact binary, corpus, input-store snapshot, configuration, virtual time, expected output, hardware, and cache-state digests. Test enumerable, intended annual, and growth/skew corpora under fixed declared limits. Controlled fixture replay and bounded permitted live-source qualification are separate measurements.

| Case | Required result and work evidence |
| --- | --- |
| Fresh acquisition | Complete oracle result without undeclared prior-store/cache dependence; reconcile capture, commit, projection, and output counts. |
| Offline evidence rebuild | Identical semantic result from retained captures with zero external requests; separately measure interpretation and index construction. |
| Warm query and export | Correct persisted revision; separate query cost from serializing output rows and reuse an already matching artifact. |
| No change and nothing due | Zero acquisition, parsing, appended physical observations, and unnecessary projection recomputation. Demonstrate indexed due/change lookup instead of scanning all history. |
| Sparse and broad changes | New mark, corrected/removed result, changed contact, and high-fan-out capture affect the expected dependency closure. Compare zero, sparse, and broad changes with counters. |
| Time-only expiry | Correct freshness/tenure changes and exact due obligations without restamping cached bytes as freshly acquired. |
| Annual rollover | Correct next cohort, stable identities, preserved prior publications, and acquisition only for new/due evidence. |
| Parser or policy upgrade | Offline expected result, traceable prior interpretation, applicable dependency invalidation, and explicit gaps for missing retained inputs. |
| Restore and resume | Backed-up publication restored, pending obligations reconciled, no duplicate effects or dangling evidence within the declared recovery point. |
| Capacity and skew | Same correct output within the envelope, or truthful resumable refusal while preserving the last valid generation. |

Report elapsed and CPU time by applicable stage; useful-row throughput and denominator; peak whole-workload memory and disk; network requests/bytes/outcomes; reused/new captures and bodies; parser calls; reads/writes/scans; spill and compaction amplification; dependency closure/recomputed rows; coverage and owed work. Distinguish cold store, cold application cache, and controlled OS-cold cache. Predeclare warmup, trial ordering, repetition, aggregation, outlier handling, and regression tolerances; retain all trials and uncertainty. Compare paired workloads with identical retained history. Inconclusive measurements do not pass. Adopt an optimization only for measured improvement without semantic, coverage, recovery, or resource regression.

### Fault and adversarial qualification

Exercise actual native boundaries, retaining the reached operation/source/attempt identity and before/after witness:

1. Kill during body staging/publication, capture commit, interpretation, semantic effect/receipt commit, acknowledgment, frontier advancement, and generation publication. Kill endpoint and coordinator separately. Recover the exact stopped unit; unrelated journal growth cannot prove recovery. Distinguish process-kill from power-loss guarantees and qualified flush assumptions.
2. Replay identical work concurrently from different parent workflows; reject conflicting payloads under one idempotency key. Race cloned handles, generation publishers, and capture-versus-quota reservations. Ambiguous acknowledgment must return the original receipt without duplicate effects or oversubscribed budgets.
3. Poll sibling source futures in one task with both completion/cancellation orders and the real collection path. Fault context remains bound to the intended source/attempt. Race browser start/stop/restart and cancellation during launch; successful stop means no live owner or late stale installation. Release or transfer all memory, disk, and work permits safely.
4. Exercise valid empty, malformed, partial, unsupported schema, missing owner, failed detail, offline miss, timeout, 429, 403, and human-required outcomes. Check CLI status, coverage, and durable obligations agree. Retry-After numeric/date values, skew, overflow, malformed values, and cooldown restart must remain bounded. A queued request must honor a newly recorded policy block.
5. Test identical A, changed B, and return-to-A revisions; stale concurrent responses; 304 with missing retained bytes; soft errors; partial pagination; and explicit withdrawals. Failed or incomplete acquisition cannot erase facts.
6. Inject ENOSPC, short writes, fsync failure, malformed metadata, wrong length/hash, oversized/decompression-bomb input, and unsupported persisted variants. Inventory nested metadata and failed-response evidence. Refuse evidence-output aliases, symlinks, or replaced paths that would write into the source witness; inspection remains bounded and read-only.
7. Interrupt garbage collection, backup, restore, and migration; race cleanup with readers, jobs, and publication. Pinned inputs survive. Missing bodies, altered manifests, unknown-newer schemas, and partial restores cannot become active. Combine receipt pruning, restore, and delayed replay within the declared protection horizon.

### Exact build evidence

Retain commit/tree and dirty status; lockfile/toolchain/features/build configuration; fingerprints for candidate, oracle, fixtures, clients, and fault helpers; corpus/store/configuration manifests; exact commands and exit codes; raw logs; test selections/counts; reached fault witnesses; resource samples and all trials. Refuse stale or unattested executables. Real upgrade tests identify actual old/new schemas and artifacts. Record each required lane as passed, failed, blocked, not run, or inconclusive; skipped lanes cannot aggregate to success.

A local acknowledged-commit durability claim is distinct from restore to an older backup. State the protected recovery point, any validated later replay, and predeclared restore/resume targets; measure them. Changes affecting a lane require rerunning it and dependent aggregate checks on the resulting exact build. No repair is accepted solely from source inspection.

## 17 Architecture decision transition and sources

The original design was based on the supplied source review at commit [`9040510e534e650efea0de1668dc8a725595d924`](https://github.com/lprior-repo/athletic-rust-pipeline/tree/9040510e534e650efea0de1668dc8a725595d924). The five-state readiness amendment uses a later read-only check pinned to [`90dd05f0a03d3e119c4cae531774df30973bd915`](https://github.com/lprior-repo/athletic-rust-pipeline/tree/90dd05f0a03d3e119c4cae531774df30973bd915), observed as HEAD on October 11, 2026 at 23:25 UTC. It inspected relevant scope, cohort, event, source-dispatch, and publication contracts. This narrow readiness check is not a full-history audit or execution qualification; the exact-head review's compilation, archive-recovery, frozen-replay, and other retained release holds remain open.

The later [ARCHITECTURE](https://github.com/lprior-repo/athletic-rust-pipeline/blob/90dd05f0a03d3e119c4cae531774df30973bd915/ARCHITECTURE.md), [AGENTS](https://github.com/lprior-repo/athletic-rust-pipeline/blob/90dd05f0a03d3e119c4cae531774df30973bd915/AGENTS.md), and [ADR 013](https://github.com/lprior-repo/athletic-rust-pipeline/blob/90dd05f0a03d3e119c4cae531774df30973bd915/docs/adr/ADR-013-fresh-national-source-census.md) still require a fresh store for the class-2027 source-discovered census and prohibit importing old observations or corpus. Their fresh-store restriction conflicts with reusable yearly production operation. This specification proposes a formal supersession or clarification of that production-lifecycle restriction during implementation. Update the owning ADR, ARCHITECTURE, AGENTS, and OPERATIONS together before relying on persistent annual reuse. Preserve public-source population discovery and honest isolated fresh-store qualification; never launder imported evidence into a fresh-acquisition claim or use an old workbook as the census denominator.

This document does not change those repository contracts, authorize an unreviewed migration, or claim that any existing release is accepted. It was assembled from source review and bounded public-source URL research; no build, benchmark, live census, athlete dataset collection, database inspection, or release checks were executed for this specification. Release acceptance remains unverified. A researched URL, a registered adapter, and a qualified end-to-end source lane are different evidence states.

The optional query-database recommendation is informed by [SQLite’s intended uses](https://sqlite.org/whentouse.html); the existing embedded-store boundary is described in [Fjall’s documentation](https://docs.rs/fjall/latest/fjall/). Pin exact dependency versions and verify required behavior during implementation rather than treating latest documentation as qualification evidence.

## 18 Implementation handoff and acceptance checklist

Implement in dependency order, with one integrator owning persisted schemas, canonical IDs, receipt semantics, and publication contracts:

1. Reconcile the owning architecture documents; freeze profiles, event/cohort/contact semantics, typed outcomes, identity boundaries, and publication rules. Establish the independent oracle, negative fixtures, exact-build evidence, and benchmark plan.
2. Qualify capture inventory, bounded admission, durable effects/receipts, frontiers, ownership, and backup consistency. Prove a fresh and restored capture-to-commit path before widening coverage.
3. Deliver one UP-prioritized end-to-end qualification slice across boys' and girls' D1–D3 programs, with regular-season and postseason XC plus representative indoor/outdoor TF evidence: schedule/results, identities, school affiliations, professional contacts, best marks, workbook, normalized export, honest gaps, and provenance. Freeze source-backed regional membership and enforce event/publication filters before claiming the slice is ready.
4. Add due/dependency indexes, incremental projections, real bounded spill processing, and streamed publication. Pass no-change, sparse-update, expiry, reparse, and annual-rollover cases against the independent reference.
5. Expand the qualified slice to Michigan, Minnesota, Iowa, Illinois, and Colorado, using the source and acceptance matrix in Section 19. Preserve separate state/UP/sport/event-family gaps and qualify annual and growth/skew behavior. Only then expand qualified source families toward the distinct 49-jurisdiction national profile; five-state delivery cannot close national obligations.
6. Rehearse schema migration and rollback on isolated copies, then run all applicable exact-build release lanes. Publish only a validated generation with a recoverable predecessor. Discuss the separate query-database choice and detailed schema after this spec; if selected, qualify its snapshot import before presenting it as ready.

Before any migration, inventory evidence, receipts, workflow state, publication pointers, and formats; verify a consistent backup and reserve destination space. Use a stopped or snapshot-pinned source and a separate destination. Preserve original bytes, timestamps, claims, contradictions, and historical IDs; mark missing legacy provenance unknown. Journal restartable steps with input/output digests. Shadow-compare outputs and explain differences. Unknown formats refuse before mutation. Rollback uses a verified previous store/publication or backup, not an old binary opening an incompatible new store; post-cutover writes require a tested replay plan or paused intake before reverting.

### Final acceptance checklist

- [ ] Owning architecture contracts explicitly permit reusable production state while retaining public discovery and isolated fresh-store qualification.
- [ ] First delivery is class 2027 in MI, MN, IA, IL, and CO, with explicit UP-first priority and source-backed regional membership; completion never implies the long-term lower-48-plus-DC national target is done. Future 50-state-plus-DC configuration needs no identity/schema redesign; territories are outside the current model.
- [ ] Recruiting event policy includes legitimate high-school TF/XC variants and qualified javelin/hammer evidence, keeps comparison classes distinct, and excludes walking/wheelchair events at event level in every output without inferring disability or excluding otherwise eligible athletes.
- [ ] Geographic/regional membership and event policy are enforced in publication, with whole-meet, reused-store, club/open, transfer, co-op, and unknown-category regressions.
- [ ] Published Rust engineering standards and all applicable release/fault/fuzz obligations pass with exact-build evidence; absent named skills and every unexecuted lane remain disclosed.
- [ ] Athlete/school identities, cohort changes, transfers, merges/splits, and aliases remain evidence-backed and reversible.
- [ ] Every best mark is comparable, valid, deterministic, and supported by retained candidates; corrections can worsen or remove it.
- [ ] School addresses and current professional contacts meet their exact evidence/currentness rules; unknowns remain visible.
- [ ] Capture/content/observation/interpretation/effect identities and atomic replay receipts pass duplicate, concurrency, and crash tests.
- [ ] Refresh, negative evidence, offline reparse, annual rollover, and affected-key recomputation pass independent semantic checks.
- [ ] Numeric resource ceilings, retention, garbage collection, backup/restore, and migration pass their declared failure models.
- [ ] Workbook and normalized bundle share one validated immutable generation with complete provenance, coverage, and checksums.
- [ ] Any selected query database imports finished generations idempotently and remains rebuildable from Fjall-backed publications.
- [ ] Exact-build evidence covers every required lane; unresolved, blocked, skipped, or inconclusive checks cannot become an accepted release.
- [ ] Measured performance, capacity, recovery limits, and operator procedures are documented for the actual deployment.

## 19 Five state rollout appendix

### Scope and implementation readiness

This appendix is an actionable qualification plan for the first five-state delivery, not a completed census or an estimate of elapsed implementation time. The inspected build is `90dd05f0a03d3e119c4cae531774df30973bd915`; source URLs below were researched on October 11, 2026. Website availability and result content can change and must be revalidated during qualification. Current XC is an in-progress 2026 season; spring 2027 marks are future refreshes. A useful class-2027 publication can use existing eligible high-school marks with an honest cutoff rather than wait for graduation.

The existing [national CLI](https://github.com/lprior-repo/athletic-rust-pipeline/blob/90dd05f0a03d3e119c4cae531774df30973bd915/crates/census-service/src/cli/national/mod.rs) accepts `--states MI,MN,IA,IL,CO`; [request construction](https://github.com/lprior-repo/athletic-rust-pipeline/blob/90dd05f0a03d3e119c4cae531774df30973bd915/crates/census-service/src/cli/national/run.rs) carries the jurisdiction set into workflow identity. A corresponding report must use that same set. This is interface evidence, not an instruction to start a run.

No UP region or school-ID allowlist was found in the inspected national/jurisdiction request interfaces, and the inspected [publication interfaces](https://github.com/lprior-repo/athletic-rust-pipeline/blob/90dd05f0a03d3e119c4cae531774df30973bd915/crates/census-service/src/cli/publish.rs) do not expose the required state/school/event filters. Add the regional and publication contracts in Sections 4 and 7, or qualify a rigorously isolated dataset for the narrow pilot. A count limit and adapter-specific school-name argument do not supply a durable UP denominator or publication guarantee.

Reusable implementation is substantial, with specific limits:

- Michigan has MileSplit and Athletic.net paths despite lacking a dedicated MHSAA adapter. Qualify MHSAA's UP PDFs, linked meet results, and local timers as complementary sources. Do not interpret missing MHSAA integration as zero Michigan capability.
- Minnesota has MSHSL school/coach, MileSplit, Athletic.net, and Wayzata meet-discovery paths. A directory collector is not complete athlete/result discovery.
- Iowa has a Bound school/staff collector plus MileSplit, Athletic.net, and Wayzata discovery. The current Bound collector is not the researched Bound athlete roster/results pipeline and relies on known school names/store schools for staff discovery.
- Illinois has IHSA school/coach, MileSplit, and Athletic.net paths. The separate IHSA tournament provider command does not establish a national result-stage arm; state-feed identity attestations remain CandidateOnly until admissible corroboration requirements are met.
- Colorado has CHSAA school/coach-name, MileSplit, and Athletic.net paths. Championship results and timer extensions still require qualification; a directory is not an athlete-history source.

These distinctions follow the current [source applicability](https://github.com/lprior-repo/athletic-rust-pipeline/blob/90dd05f0a03d3e119c4cae531774df30973bd915/crates/census-crawl/src/applicability/table/data.rs), [team dispatch](https://github.com/lprior-repo/athletic-rust-pipeline/blob/90dd05f0a03d3e119c4cae531774df30973bd915/crates/census-service/src/restate_services/teams_arms.rs), [meet dispatch](https://github.com/lprior-repo/athletic-rust-pipeline/blob/90dd05f0a03d3e119c4cae531774df30973bd915/crates/census-service/src/restate_services/meets_arms.rs), and [result dispatch](https://github.com/lprior-repo/athletic-rust-pipeline/blob/90dd05f0a03d3e119c4cae531774df30973bd915/crates/census-service/src/restate_services/results_arms.rs). AthleticLIVE artifact/index collectors list MI, MN, IA, and IL applicability, but the [durable planner](https://github.com/lprior-repo/athletic-rust-pipeline/blob/90dd05f0a03d3e119c4cae531774df30973bd915/crates/census-service/src/restate_services/plan/strategies.rs) refuses the missing artifact/seed discovery strategies; Colorado is absent from those applicability lists. Current generic TFRRS applicability is FL/IN/NH, none of these five states. Neither researched provider availability nor a collector's existence means a complete durable lane. Mirrored Athletic.net/AthleticLIVE/IHSA results do not establish independent authorship or cross-provider corroboration.

### Verified public source entry points

“Verified” means the URL was found in current public results or followed from its primary source, with the observed access state stated below. It does not mean all content was readable, permission to bypass restrictions, a working extraction adapter, or statewide coverage. Follow actual published links rather than guessing PDF filenames or undocumented APIs.

| State | School and program denominator | Results and supplementation | Observed qualification limits |
| --- | --- | --- | --- |
| Michigan with UP priority | [MHSAA schools](https://www.mhsaa.com/schools); [UP XC assignments](https://www.mhsaa.com/sports/boys-cross-country/assignments/final?uplpcode=up&year=2026-27). Establish TF and XC membership separately, including boys/girls and co-ops. | [TF hub](https://www.mhsaa.com/sports/boys-track-field), [2026 regionals](https://www.mhsaa.com/sports/boys-track-field/2026-mhsaa-track-field-regional-results), accessible [UP boys D1 PDF](https://www.mhsaa.com/sites/default/files/Track%20Field-Boys/2026/Finals/2026-UP-Boys-D1-Finals.pdf?time=1780175792370) and [UP girls D1 PDF](https://www.mhsaa.com/sites/default/files/Track%20Field-Boys/2026/Finals/2026-UP-Girls-D1-Finals.pdf?time=1780175792370); [XC archives](https://www.mhsaa.com/sports/boys-cross-country/results-archive); [Superior Timing](https://www.superiortiming.com/category/raceresults/?wpmp_switcher=desktop); [Michigan Indoor Track Series](https://www.mitstrack.org/copy-of-home); [USATF Michigan events](https://michigan.usatf.org/events?pastevents=1). | Directory extraction did not enumerate schools. D1 PDFs opened; D2/D3 must follow the hub's actual links. LP Athletic.net and linked indoor document were thin/JS shells. Regionals also contain middle-school rows. No youth hammer/javelin artifact was qualified in this pass. |
| Minnesota | [MSHSL schools](https://www.mshsl.org/schools) and [TF hub](https://www.mshsl.org/sports-and-activities/track-and-field-boys). | [Wayzata meet index](https://results.wayzatatiming.com/meet-list) and [2026 state meet](https://results.wayzatatiming.com/meets/74652); [Section 1A](https://kometsports.com/section1a-track/); [Hero's 2026 results](https://www.herostiming.com/2026-results.html) and [2025 results](https://www.herostiming.com/2025-results.html); [2025 XC state results](https://www.mshsl.org/2025-cross-country-state-tournament-results); [USATF MN JO/Open/Masters event](https://minnesota.usatf.org/events/2026/2026-usatf-minnesota-junior-olympic-open-and-maste). | MSHSL returned 403 in the research tool; respect the restriction. Wayzata results resolved to AthleticLIVE shells. Hero's readable indexes mix HS, college, youth and neighboring states. USATF page mentions javelin; hammer and implement metadata require actual-result verification. |
| Iowa | [IHSAA member schools](https://www.iahsaa.org/member-schools/), [boys TF classifications/co-ops](https://www.iahsaa.org/classifications/track-field/), [IGHSAU classifications](https://ighsau.org/ighsau-classifications), and [Bound school directory](https://www.gobound.com/ia/schools). | Boys [TF](https://www.iahsaa.org/track-field/) and [XC](https://www.iahsaa.org/cross-country/); girls [TF](https://ighsau.org/sports/track-field) and [XC](https://ighsau.org/sports/cross-country); [2026 co-ed TF PDF](https://ighsau.nyc3.digitaloceanspaces.com/2026-track/results.pdf); [Bound boys scores](https://www.gobound.com/ia/ihsaa/boystrack/2025-26/scores); [IATC results](https://www.iatrackcoaches.org/results/); [USATF Iowa](https://iowa.usatf.org/events.aspx?pastevents=1). | Bound and AthleticLIVE were thin JS shells. IATC's current route works; its old domain did not. Boys and girls authorities/programs need distinct qualification. Club/field-only results do not imply every throwing event is offered. |
| Illinois | [IHSA school directory](https://www.ihsa.org/schools/school-directory) and [2026 boys 1A assignments](https://www.ihsa.org/data/trb/1assign.htm), retaining co-ops and individual-only entries. | Boys [TF](https://www.ihsa.org/sports/boys-track-field) and [XC](https://www.ihsa.org/sports/boys-cross-country); girls [TF](https://www.ihsa.org/sports/girls-track-field) and [XC](https://www.ihsa.org/sports/girls-cross-country); [XC results gateway](https://www.ihsa.org/archive/ccg/2025-26/3winner.htm); [Illinois Top Times](https://illinoistoptimes.runnerspace.com/); [IL Prep Top Timing](https://ilpreptoptiming.com/); [USATF Illinois results](https://www.usatfillinois.org/association-championships-1/all-results/). | Current hubs resolved but returned no extracted body; qualify permitted browser navigation. IHSA is Illinois, not Indiana's IHSAA or elementary IESA. Preserve 3-mile XC. State assignments and indoor championship results are incomplete population denominators on their own. |
| Colorado | [CHSAA member schools](https://chsaanow.com/schools); [Manual school profile](https://chsaanow.com/schools/manual) demonstrates address and 2026–28 TF/XC alignment fields. | [TF qualifying-meet index](https://chsaanow.com/sports/track-and-field/info), [championship hub](https://chsaa.co/events/track), [2026 state PDF](https://s3.amazonaws.com/chsaanow.com/documents/2026/5/17/combined_state_results_2026.pdf), [XC postseason](https://chsaanow.com/sports/cross-country/info/postseason), [Colorado MileSplit](https://co.milesplit.com/), and [USATF Colorado JO](https://colorado.usatf.org/jotf26). | Some CHSAA news routes had robots restrictions; preserve accessible official hubs/artifacts and report gaps. Staff rows were not exposed in text extraction. Regular-season timers and indoor invitationals need their own qualified inventory. |

For school addresses and contacts, follow current school-controlled athletics/staff pages and keep coach, athletics-office, AD, and meet-host roles separate. Positive field examples are [Wayzata boys TF](https://www.wayzataathletics.com/spring-sports/track-and-field-boys/) and [girls TF](https://www.wayzataathletics.com/spring-sports/track-and-field-girls/), which publish coach contact information; [Marquette Senior High School](https://mshs.mapsnet.org/) links its [athletics site](https://marquetteathletics.com/), but no current TF/XC coach email was verified there in this pass. In Iowa, the [IHSAA contact page](https://www.iahsaa.org/contact/) directs school-contact discovery to Bound. These are routes and specimens, not proof of statewide current-contact completeness. Do not generate emails, substitute meet venues for school addresses, or collect athlete/family contacts.

### Implementation acceptance for the first delivery

1. **Clear existing release holds.** Resolve the exact-build compilation, archive recovery/accounting, frozen-replay, and retained release blockers through sanctioned checks before production acceptance. A narrower geography does not waive correctness or durability.
2. **Freeze the five-state and UP contracts.** Record school/program membership, source/date/season evidence, boys/girls categories, co-op constituents, current-affiliation rules, priority versus filter semantics, and scope digests. Carry those contracts through durable work, coverage, projections, and publication. Prove an out-of-state meet can contribute a mark to an in-scope athlete without admitting unrelated meet participants.
3. **Qualify a coherent UP pilot.** Include boys' and girls' D1–D3 result artifacts, one or more ordinary regular-season results, completed XC history, and representative indoor/outdoor TF. Pair them with source-backed school/cohort and current professional-contact evidence. Include small schools/co-ops, sparse and empty rosters, missing grades, excluded event aliases, unknown technical metadata, and athletes with both included and excluded observations. Javelin/hammer enter when evidence supports them; missing live examples remain visible gaps with separate fixture qualification.
4. **Qualify complementary five-state sources.** Test accessible official PDFs, browser-backed results, school directories, ordinary-season timers, indoor sources, and relevant club/open events. Verify actual row/event counts, pagination and ownership, date/grade interpretation, partial/empty/error outcomes, source lineage, and allowed acquisition method. Use class-year evidence or dated grades: normally grade 9 in 2023–24, 10 in 2024–25, 11 in 2025–26, and 12 in 2026–27, subject to conflicts/reclassification. An age band alone is insufficient.
5. **Prove data and output invariants.** Independent fixtures must cover every included family, separated distance/implement/hurdle/timing/wind classes, excluded walking/wheelchair events, identity/cohort conflicts, ties, corrected winners/fallback, missing relay membership, transfers, and reused-store leakage. Read back every workbook and normalized-export family against one immutable expected generation, with coverage and exclusion reasons. If selected later, qualify the query-database snapshot import separately under Section 15.
6. **Account for every obligation and limit.** Qualify a fresh store separately from proposed reusable-store refresh, respecting the unresolved architecture transition in Section 17. Predeclare numeric request/time/memory/storage limits; exhausted budgets preserve deferred work rather than create success. Report school/program discovery, cohort yield, accepted identities, result histories, event-family coverage, missing addresses/contacts, and terminal gaps separately by state and UP scope.
7. **Publish or state the exact blocker.** Acceptance requires the independent oracle, applicable release/fault/resource checks, durable disposition of all admitted obligations, and publication integrity for the declared build and manifest. A completed run may still fail publication approval. Stop at a verified five-state qualification report and separately adjudicated publication, or a specific unresolved blocker needing a decision. Do not seal national completion, invent live coverage from fixtures, or supply elapsed-time/person-day promises unsupported by measured execution.
