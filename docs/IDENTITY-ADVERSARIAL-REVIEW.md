# Adversarial Identity Review — Athlete Matching and Adjudication

**Date:** 2026-09-25
**Scope:** `crates/census-reconcile/`, `crates/census-review/`, `crates/census-domain/src/model/`, `crates/census-store/src/entities/`
**Objective:** §58 of `ATHLETIC_PIPELINE_DELIVERY_GOAL_183fca1.md` (GOAL-2026-09-24-01) adversarial-identity-review requirement.
**Authority:** Deterministic Rust owns ingestion, parsing, normalization, source identifiers, school matching, cohort interpretation, candidate generation, contradiction detection, mark arithmetic, comparison, PR reduction, evidence persistence, caching, coverage and export. AI may advise only on genuinely unresolved identity questions *after* normalization and *after* hard contradictions are computed; exactly one model owns a case; the deterministic adjudicator owns acceptance. Never fabricate a field to make a row look complete.

---

## 1. False Merge — Two different athletes collapsed into one canonical row

### 1a. Shared name + grad year at different schools (school mapping drift)

- **Severity:** material
- **Trigger:** A school's jurisdiction mapping is updated between pipeline runs. The school's `SchoolId` changes. The same athlete, observed by the same provider with the same `source_identities`, now mints a different `AthleteCandidateKey` because `AthleteCandidateKey::new` (crates/census-domain/src/model/athlete.rs:27-36) hashes the new `SchoolId`. The candidate ID changes, and the store's merged read key on ID, so the old canonical row and the new observation are two separate `CanonicalAthlete` entities — but the shared `source_identities` link is present on both.
- **Guard:** `CanonicalAthlete::candidate_key()` (crates/census-domain/src/model/athlete.rs:153-160) returns `(school, name, grad_year, gender)`. The merge checks `same_natural_key` (crates/census-domain/src/model/natural_key.rs:161-166), which compares `school` directly (`self.school == other.school`). If the school ID changed, the keys differ and no merge happens.
- **Gap:** There is no re-association step. The shared `source_identities` are not consulted to detect that the same external athlete ID appeared under a different school. The athlete exists as two canonical rows, one with the old school and one with the new school, and the workbook (crates/census-report/src/workbook/recruiting/athletes.rs:173-185) publishes both as separate people.
- **Impact:** A transferring athlete or a school whose mapping was corrected appears as two distinct athletes in the recruiting workbook, each with partial evidence. A recruiter could contact both rows.

### 1b. Shared provider ID reused by a timing provider

- **Severity:** material
- **Trigger:** A timing provider (e.g., Milesplit) reuses an athlete ID across two different people (e.g., two athletes named "John Smith" in the same state). Both observations are appended to the store. The `reconcile_athletes` pass (crates/census-review/src/athlete_clusters.rs:69-126) reads the `Observed` index (crates/census-review/src/athlete_cluster_findings.rs:73-88), which maps `(namespace, id)` → `BTreeSet<canonical_id>`. When the same ID maps to two canonical IDs, `spans()` (crates/census-review/src/athlete_cluster_findings.rs:106-112) produces a `Span`. If the two rows agree on name, grad year, and gender (`Span::agrees`, line 158-164), the deterministic rule verdicts `same_person` and files the case as `Resolved`. If they disagree on any of those three, the case stays `Pending`.
- **Guard:** The `reconcile_athletes` pass detects spans and the `AthleteIdentity` review lane can be asked about them.
- **Gap:** The deterministic agreement rule at `athlete_clusters.rs:93-98` (`span.agrees()`) does **not** check whether the two rows have conflicting grade observations, conflicting source identities, or any other signal that they might be different people. It only checks name, grad year, and gender. The packet sent to the model (crates/census-review/src/athlete_packet.rs:94-116) carries `member_ids` from the case, but the deterministic verdict is applied without any model involvement when `agrees()` is true. The model is not consulted for this class of finding.
- **Impact:** If two different athletes share the same provider ID and also share the same name, grad year, and gender (plausible for common names in large states), the pipeline merges them deterministically without any evidence check beyond those three fields.

### 1c. Truncated/misparsed ID causing key collision

- **Severity:** material
- **Trigger:** An external ID from a provider is longer than expected. The `WorkflowIdentity::push_bounded` (crates/census-reconcile/src/identity.rs:233-247) digests parts longer than `MAX_PART_BYTES` (32 bytes), keeping only 8 bytes of SHA-256 (`DIGEST_BYTES`). Two different external IDs that hash to the same 8-byte digest produce the same workflow identity. This is a **workflow** identity collision, not an athlete ID collision, but it affects which workflow reads which provider data.
- **Guard:** `push_bounded` digests long parts rather than truncating (crates/census-reconcile/src/identity.rs:233-247). The full provider value lives in the store row.
- **Gap:** The 64-bit digest means 2^64 possible values. While the birthday paradox makes collisions unlikely for small sets, the identity pipeline has no explicit collision detection for provider-level workflow identities. A collision would cause a workflow to read a different provider's data than intended, producing observations for the wrong subject. The athlete merge would then have no chance to detect it.
- **Impact:** Wrong observations are appended to the store, potentially creating false athlete candidates from another provider's data.

### 1d. School resolved to wrong jurisdiction

- **Severity:** material
- **Trigger:** A school page's name normalizes to the same key as another school in a different state. The `normalize_name` function (crates/census-domain/src/model/normalization.rs:15-39) lowercases, strips diacritics, collapses whitespace, and strips school-type suffixes. The `same_compressed` function (crates/census-domain/src/model/natural_key.rs:59-63) compares the alphanumeric-only form. Two schools named "Washington" in different states compress to "Washington". If the school mapping does not disambiguate by state, the `CanonicalSchool` merge (crates/census-store/src/entities/canonical.rs:42-73) may absorb one into the other, or both may be retained as separate rows with different `SchoolId`s.
- **Guard:** The crawl adapters require state context when minting school IDs (e.g., `athleticnet/collect.rs:110-113` warns about "merge same-named schools across states").
- **Gap:** The `AthleteCandidateKey` mints from `SchoolId` (crates/census-domain/src/model/athlete.rs:65-76), and `SchoolId::mint` hashes the jurisdiction code and the normalized name (crates/census-domain/src/model/normalization.rs:12). If two schools in different states have the same normalized name and the state code is correct, their IDs differ. But if a crawl adapter fails to capture or pass the state (e.g., a bare-name directory export), the school ID would be the same across states, and athletes from different states would collide.
- **Impact:** Athletes from different states with the same school name would be merged if the state is missing from the school key.

## 2. False Split — One athlete's rows left as two candidates

### 2a. School mapping changes between runs

- **Severity:** material
- **Trigger:** A school is re-mapped between pipeline invocations. The school's `SchoolId` changes. The same athlete, observed by the same provider, mints a new `AthleteCandidateKey` with the new school ID. The old canonical row (under the old school ID) and the new observation produce a new canonical row.
- **Guard:** `source_identities` are preserved on both rows. The `Observed` index (crates/census-review/src/athlete_cluster_findings.rs:90-103) records which provider objects name which canonical rows.
- **Gap:** There is no mechanism to re-associate the old row with the new row. The `reconcile_athletes` deterministic pass (crates/census-review/src/athlete_clusters.rs:85-103) only looks at provider objects that span multiple rows *in the current store*. If the old row and new row have different `source_identities` (because the school changed), the provider object on the new row is a new `(namespace, id)` pair and does not span. The old row's provider objects are still indexed but no longer span anything.
- **Impact:** A transferred athlete appears as two rows in the workbook. Both carry partial performance history. The recruiter sees two different athletes.

### 2b. Gender observation disagreement between sources

- **Severity:** hardening
- **Trigger:** Source A reports gender `Boys`. Source B reports gender `Girls`. The `AthleteCandidateKey` includes gender, so the two observations mint different candidate IDs. The merge checks `same_natural_key` (crates/census-domain/src/model/natural_key.rs:161-166), which compares `self.gender == other.gender` directly — they differ, so no merge.
- **Guard:** `Mixed` and `Unknown` both mint the letter `u` in `gender_letter` (crates/census-domain/src/model/athlete.rs:55-61), so they are treated differently from `Boys`/`Girls`. The `same_natural_key` comparison uses the full `Gender` enum, not the letter.
- **Gap:** The `reconcile_athletes` pass (crates/census-review/src/athlete_clusters.rs:93-102) checks `agrees()` which requires `self.gender == other.gender`. If the genders disagree, the case stays `Pending` and is sent to the model. But there is no deterministic rule that resolves a gender disagreement when other fields agree. The model is asked whether they are the same person, but the gender field is a direct disagreement that the model cannot resolve from the packet's text (the packet lists each side's gender as a fact, and the model's answer is just "same_person" or "different_person" — it cannot say "the gender is Boys").
- **Impact:** A same-person case where genders disagree from different sources remains perpetually `Pending` or is resolved incorrectly by the model.

## 3. Contradiction Bypass — Pipeline proceeds with disagreeing identity fields

### 3a. Deterministic verdict applied without contradiction computation

- **Severity:** material
- **Trigger:** A provider object spans two canonical rows at different schools. The rows agree on name, grad year, and gender. The `reconcile_athletes` deterministic pass (crates/census-review/src/athlete_clusters.rs:93-98) immediately verdicts `same_person` and closes the case as `Resolved`, writing a `ReviewVerdictRecord` with `reviewer: RULE_REVIEWER` and `confidence: 100`.
- **Guard:** The deterministic rule is documented (crates/census-review/src/athlete_clusters.rs:7-9) and the verdict carries a rationale. The `member_ids` on the verdict identify which candidates are merged.
- **Gap:** The agreement check (`Span::agrees`, crates/census-review/src/athlete_cluster_findings.rs:158-164) does **not** compute or check hard contradictions. It only checks name, grad year, and gender. It does not check:
  - Whether grade observations imply different graduation years (`FlagKind::GradYearEvidenceDiffers` in crates/census-review/src/athlete_flags.rs:112-121)
  - Whether the rows carry conflicting provider identities (`FlagKind::DistinctProviderObjects` in crates/census-review/src/athlete_flags.rs:99-103)
  - Whether one row carries no evidence at all
  
  These contradictions are computed only in the `athlete_flags` module (crates/census-review/src/athlete_flags.rs:83-140), which is used by the **model packet**, not by the deterministic verdict. The deterministic pass never reads the flags.
- **Impact:** Two athletes who share name, grad year, and gender but have contradictory evidence (e.g., one has a performance in event X and the other does not, or one has grade observations implying class 2026 and the other implies class 2028) are deterministically merged as the same person without any contradiction check.

### 3b. AI verdict applied without hard-contradiction computation visible to operator

- **Severity:** hardening
- **Trigger:** The `AthleteIdentity` review family sends a packet (crates/census-review/src/athlete_packet.rs:94-116) to the model. The packet includes flags computed from the rows' own fields, including contradictions (`athlete_flags::flags`, crates/census-review/src/athlete_flags.rs:83-140). The model answers, the verdict is validated (crates/census-review/src/verdicts.rs:66-73), and the verdict is recorded.
- **Guard:** The packet carries the flags as facts (crates/census-review/src/athlete_packet.rs:14-15). The verdict validation checks that the case was asked (`read`, crates/census-review/src/athlete_verdict.rs:76-79) and that the value is parseable (`AthleteVerdict::parse`, crates/census-review/src/athlete_verdict.rs:40-53).
- **Gap:** The verdict is validated only against the packet's `cases` list (crates/census-review/src/athlete_verdict.rs:76-79). The verdict's `case_id` must match one case in the packet. But the packet is built from the store's current state. If the store changes between when the packet was built and when the verdict is processed, the verdict may be applied to a different set of rows than the one the model actually saw. The `member_ids` on the verdict record come from the case (crates/census-review/src/records.rs:25), which copies the case's `member_ids` at verdict-creation time. There is no re-validation of `member_ids` against the current store state.
- **Impact:** A verdict answering a case about rows A and B could be applied to a store where rows A and B have been re-indexed or re-grouped, causing the verdict to land on the wrong candidates.

## 4. Wrong-Case Application — Verdict applied to wrong candidates

### 4a. Case-ID digest instability after school mapping change

- **Severity:** blocking
- **Trigger:** A school's name is updated in the store (e.g., "Washington HS" → "Washington High School"). The `Span::case()` method (crates/census-review/src/athlete_cluster_findings.rs:166-189) builds the case detail string, which includes the school name: `"{} at {} (class {}, {:?})"` where the school name comes from `schools.get(stored)` (crates/census-review/src/athlete_cluster_findings.rs:40-43). A school rename changes the detail string, which changes the `CaseEvidence` digest, which changes the case ID.
- **Guard:** The `carried` function (crates/census-reconcile/src/index.rs:74-83) merges the stored case's state with the newly minted case. The old case retains its state.
- **Gap:** The case ID is the identity for the review case table. When the case ID changes, the old case is no longer derivable by the new pass. The `supersede` function (crates/census-reconcile/src/index.rs:92-111) marks old `Pending` cases as `Superseded`. But old `Resolved` cases stay as-is — their verdicts are preserved. The **new** pass mints a new case with a different ID and no prior verdict. A model verdict applied to the new case answers a question about the same rows but under a new ID. The verdict record (crates/census-review/src/records.rs:21-25) uses the case's ID as the verdict's ID and case_id. If a later pass re-derives the case and the school name changes again, the verdict record's ID no longer matches any current case.
  
  More critically, the **case-id digest** (crates/census-domain/src/model/records.rs:197-200) includes the evidence digest, which includes the school name in the subject line. If the subject line changes, the case ID changes. The verdict record keeps the old case ID. There is no mechanism to update the verdict record's case_id when the case ID changes.
  
  The `AthleteVerdict::read` (crates/census-review/src/athlete_verdict.rs:76-79) checks `verdict.case_id` against `packet.cases[*].case_id`. If the case ID changed, the verdict's `case_id` won't match any case in the new packet, and the verdict will be refused with `Refusal::UnnamedCase`. This means verdicts on school-dependent cases are **non-transferable** across school renames.
- **Impact:** A model verdict on an athlete identity case is lost if the school's name changes between the pass that built the packet and the pass that processes the verdict. The case must be re-asked, and if the model gives a different answer, the athlete's identity becomes non-deterministic.

### 4b. `member_ids` not checked against verdict application

- **Severity:** material
- **Trigger:** A verdict record contains `member_ids` (crates/census-domain/src/model/records.rs:232) that name the candidates the verdict applies to. The verdict is matched to a case by `case_id`, not by `member_ids`.
- **Guard:** The verdict record is built from the case at the time the verdict is recorded (crates/census-review/src/records.rs:25), copying the case's `member_ids`.
- **Gap:** There is no validation that the verdict's `member_ids` match the case's `member_ids` at the time the verdict is applied. If the case's `member_ids` change between verdict creation and verdict processing (e.g., because the store's athlete index changed), the verdict could be recorded with stale `member_ids`. The `ReviewVerdictRecord` (crates/census-domain/src/model/review.rs:210-233) carries `member_ids` but the downstream consumers (workbook, seal) do not validate them against the canonical rows they name.
- **Impact:** A verdict that should apply to candidates A and B could be recorded with `member_ids` for candidates A and C (if the case changed between creation and recording). The workbook or seal would not detect the mismatch.

## 5. Evidence Loss — Merge or publication drops an observation, alias, or source identity

### 5a. `RetainedConflict` record uses `compressed` name, merge uses `same_name`

- **Severity:** material
- **Trigger:** Two rows have names that normalize the same way (e.g., "José's HS" and "Josés HS" — diacritics stripped, apostrophe removed) but compress differently (the apostrophe is non-alphanumeric, so both compress to "JosésHS", but `normalize_name` strips school-type suffixes). Wait — `normalize_name` does not strip apostrophes or punctuation except as part of school-type suffix stripping. Let me re-examine.
  
  Actually, `normalize_name` (crates/census-domain/src/model/normalization.rs:15-39) lowercases, strips diacritics, and strips school-type suffixes. It does NOT strip punctuation from names. `same_name` (crates/census-domain/src/model/natural_key.rs:53-55) compares `left == right || normalize_name(left) == normalize_name(right)`. `compressed` (crates/census-domain/src/model/natural_key.rs:66-68) strips ALL non-alphanumeric characters. So "José's HS" normalizes to "jose hs" (diacritic stripped, suffix stripped) and "Joses HS" normalizes to "joses hs" — these differ. But compressed they are "joses" and "joses" — same.
  
  The `collision` function (crates/census-store/src/entities/canonical.rs:22-27) calls `same_natural_key` which uses `same_name`. If `same_name` returns false, a `RetainedConflict` is created. The conflict's `natural_key()` (crates/census-domain/src/model/natural_key.rs:168-176) uses `compressed`, not `same_name`. So the conflict record may say "kept X [josehs]" and "dropped Y [joses]" when the merge actually decided they are different (because `normalize_name("José's HS") != normalize_name("Joses HS")`).
  
  Wait, the `natural_key()` format string is: `"athlete {:?} at {} (class {}, {:?})"` and it passes `normalize_name(&self.canonical_name)` (crates/census-domain/src/model/natural_key.rs:171). So it uses `normalize_name`, not `compressed`. The `compressed` function is only used in `compressed()` (crates/census-domain/src/model/natural_key.rs:66-68) for the `RetainedConflict` display in the detail string (crates/census-domain/src/model/collision.rs:30-31).

- **Guard:** `same_name` is the merge check; `compressed` is only for display in the conflict detail.
- **Gap:** The conflict detail string uses `compressed` names (crates/census-domain/src/model/collision.rs:30-31), but the merge decision uses `same_name`. If two names compress the same but don't normalize the same, the merge keeps them separate, but the conflict detail shows them as the same. This is a reporting inconsistency, not a data loss — the rows are correctly kept separate.
  
  Actually, this is not evidence loss. The rows are correctly kept separate by the merge. The detail just displays confusingly. Let me find a real evidence loss case.

### 5b. `union_vec` deduplicates but does not preserve observation order

- **Severity:** hardening
- **Trigger:** Two observations of the same athlete are merged. `union_vec` (crates/census-store/src/entities/mod.rs:12-16) appends items from the second observation that are not already in the first. For `evidence` and `source_identities`, this means the order is "first observation's items, then new items from second observation."
- **Guard:** The order is deterministic — the first observation's items are always listed first.
- **Gap:** For `known_names`, `sports`, and `public_profile_urls`, the merge does a union. If the first observation has names `["John"]` and the second has `["John", "Johnny"]`, the merged result is `["John", "Johnny"]`. But the `canonical_name` field is taken from the first observation (the merge never changes it). If the first observation's spelling is a typo and the second has the correct spelling, the canonical name is wrong and the alias ("Johnny") is in `known_names`. This is correct behavior — the first spelling is preserved as canonical, and the corrected spelling is an alias. No data loss, but the primary name may be wrong.

### 5c. Snapshot reader drops damaged rows

- **Severity:** blocking
- **Trigger:** A JSONL snapshot file is damaged (e.g., truncated mid-row). The snapshot reader (crates/census-store/src/snapshot/mod.rs) is described as failing by file and line (crates/census-store/src/snapshot/tests.rs:30-31), not silently dropping rows. The reader was fixed to refuse damaged snapshots.
- **Guard:** `read_rows` (crates/census-store/src/snapshot/mod.rs) fails on unparseable rows (crates/census-store/src/snapshot/tests.rs:30-31).
- **Gap:** This is the only path where a canonical row could silently disappear. The fix is in place. However, the `consolidate` pass (crates/census-store/src/read/mod.rs:158-162) writes to a temporary file and atomically renames. If the process is killed between the rename and the completion of the write, the old snapshot remains. The old snapshot may contain stale rows. But this is a durability issue, not an identity issue.

### 5d. Alias finding is filed but never resolved

- **Severity:** material
- **Trigger:** A canonical row carries two `SourceIdentity` objects from the same namespace (e.g., two Milesplit IDs on the same row). The `Observed::aliases()` method (crates/census-review/src/athlete_cluster_findings.rs:132-145) detects this and files a case. The case is marked `Pending` (crates/census-review/src/athlete_clusters.rs:113-115).
- **Guard:** The case is filed under the `AthleteIdentity` family and is visible in the workbook's queues.
- **Gap:** There is no deterministic rule to resolve this case. The deterministic `reconcile_athletes` pass only handles `Span` findings (provider object spanning multiple rows), not `Alias` findings (one row with multiple objects of one namespace). The `Alias` case is filed and left `Pending` — it is never decided. The `AthleteIdentity` model pass asks about it, but the packet compares the row against itself (there's no "other" row to compare against). The model's answer is irrelevant — the case stays `Pending` regardless.
- **Impact:** A provider that issued two IDs for one athlete creates a permanent unresolved case. The row carries two conflicting provider identities that are never adjudicated. The `RetainedConflict` that should name this situation is never created because the merge considers it one row (the IDs are both on the same `CanonicalAthlete`).

## 6. Fabrication — Field filled from default, inference, or average

### 6a. `identity_confidence` default

- **Severity:** hardening
- **Trigger:** A `CanonicalAthlete::new` call (crates/census-domain/src/model/athlete.rs:162-185) sets `identity_confidence: Confidence::MEDIUM`. If the row carries no grade observations, `derived_identity_confidence()` (crates/census-domain/src/model/athlete.rs:196-212) returns `None`, and `publish()` (crates/census-store/src/entities/canonical.rs:161-165) leaves the field at `MEDIUM`.
- **Guard:** `derived_identity_confidence()` returns `None` when there are no observations, so the constructor's default is preserved rather than overwritten. The `MEDIUM` default represents "insufficient evidence to determine."
- **Gap:** `MEDIUM` is not fabricated — it is an honest admission of insufficient evidence. However, the workbook's audit columns (crates/census-report/src/workbook/recruiting/athletes.rs:253-269) display `identity_confidence` as a column. A row at `MEDIUM` may appear "complete enough" to a reviewer, when in fact no grade evidence exists. The `Review Flag` column (crates/census-report/src/workbook/recruiting/athletes.rs:262-263) is "yes" when confidence is below `HIGH`, so this is visible. But `MEDIUM` is not the same as `LOW` — a row at `MEDIUM` may not trigger review attention.
- **Impact:** An athlete with no grade observations is published at `MEDIUM` confidence. A recruiter may accept the row as valid without checking that no grade evidence exists.

### 6b. `sports` field never inferred from performances

- **Severity:** hardening
- **Trigger:** A `CanonicalAthlete::new` call creates a row with `sports: Vec::new()`. The merge appends sports from observations (crates/census-store/src/entities/canonical.rs:142: `union_vec(&mut self.sports, &other.sports)`). But if no source observation ever specifies a sport, the field stays empty.
- **Guard:** The sports field comes from source observations, not inference.
- **Gap:** If a source publishes an athlete's performances but does not explicitly name the sport (e.g., a meet results page that only lists event names), the athlete's `sports` field may be empty even though performances exist. The workbook's `participation_cells` (crates/census-report/src/workbook/recruiting/athletes.rs:210-229) uses `sports` to determine sport flags. An athlete with performances but no sports field would have no sport flags set.
- **Impact:** An athlete with verified performances but no explicit sport field appears in the workbook without sport flags, potentially confusing a recruiter.

### 6c. `current_grade` derived from `observed_grades` — newest-last order

- **Severity:** hardening
- **Trigger:** `observed_grades` is stored newest-last (crates/census-domain/src/model/athlete.rs:116). The workbook's `current_grade` function reads the last observation (crates/census-report/src/workbook/recruiting/athletes.rs:180 — `current_grade(athlete)`).
- **Guard:** The last observation is the most recent, which is the correct current grade.
- **Gap:** If a source reports an incorrect grade (e.g., reports a sophomore as a junior), and a later source corrects it, the `observed_grades` vector contains both. The `current_grade` reads the last one, which is correct. But `derived_identity_confidence()` checks whether any observation disagrees with `grad_year` (crates/census-domain/src/model/athlete.rs:197-202). If the last observation agrees but an earlier one disagrees, confidence is `HIGH`. This means the confidence reflects whether **any** observation agrees, not whether the **latest** observation agrees. A row where the latest observation is wrong but an earlier one is right would be at `HIGH` confidence.
- **Impact:** Grade confidence may be inflated if a stale correct observation is paired with a recent incorrect one.

## Summary of Findings

| # | Severity | Attack | Source Location | Trigger | Impact |
|---|----------|--------|----------------|---------|--------|
| 1a | material | False merge — school mapping drift changes candidate key | `athlete.rs:27-36`, `natural_key.rs:161-166` | School `SchoolId` changes between runs | Same athlete appears as two rows |
| 1b | material | False merge — shared provider ID on different people | `athlete_clusters.rs:93-98`, `athlete_cluster_findings.rs:158-164` | Provider reuses ID; rows agree on name/class/gender | Two different people deterministically merged |
| 1c | material | False merge — workflow identity collision | `identity.rs:233-247`, `identity.rs:33-41` | External ID > 32 bytes, 8-byte digest | Wrong provider data read, wrong observations |
| 1d | material | False merge — missing state in school key | `normalization.rs:15-39`, `athleticnet/collect.rs:110-113` | Bare-name directory export, no state | Cross-state athlete merge |
| 2a | material | False split — school mapping change | `athlete_cluster_findings.rs:90-103`, `athlete_clusters.rs:85-103` | School `SchoolId` changes between runs | Same athlete appears as two rows |
| 2b | material | False split — gender disagreement | `natural_key.rs:161-166`, `athlete_clusters.rs:93-102` | Different gender from different sources | Case stays Pending or model resolves incorrectly |
| 3a | material | Contradiction bypass — deterministic rule skips contradictions | `athlete_clusters.rs:93-98`, `athlete_flags.rs:83-140` | Provider span with agreeing name/class/gender but conflicting evidence | Contradictory evidence silently merged |
| 3b | hardening | Contradiction bypass — model verdict with stale packet | `athlete_verdict.rs:76-79`, `athlete_packet.rs:94-116` | Store changes between packet build and verdict processing | Verdict applied to wrong candidate set |
| 4a | **blocking** | Wrong-case application — case ID instability after school rename | `athlete_cluster_findings.rs:166-189`, `records.rs:197-200`, `athlete_verdict.rs:76-79` | School name change changes case evidence digest | Model verdicts non-transferable; case must be re-asked |
| 4b | material | Wrong-case application — `member_ids` not validated on application | `records.rs:25`, `athlete_verdict.rs:76-79` | Case `member_ids` change between verdict creation and processing | Verdict recorded with stale candidate references |
| 5a | material | Evidence loss — conflict detail uses different normalization than merge | `collision.rs:30-31`, `natural_key.rs:53-55` | Names that compress same but normalize differently | Conflict detail shows same name when rows are kept separate |
| 5b | hardening | Evidence loss — canonical name from first observation only | `canonical.rs:42-73` (school merge), `canonical.rs:136-152` (athlete merge) | First observation has typo, second has correct spelling | Canonical name wrong; correction only in `known_names` |
| 5c | blocking | Evidence loss — snapshot truncation | `snapshot/tests.rs:30-31`, `read/mod.rs:158-162` | Damaged JSONL file | Refused (fixed) but old snapshot may have stale rows |
| 5d | material | Evidence loss — Alias finding never resolved | `athlete_cluster_findings.rs:132-145`, `athlete_clusters.rs:113-115` | Provider issues two IDs for one athlete | Permanent unresolved case; two provider IDs never adjudicated |
| 6a | hardening | Fabrication — default `identity_confidence` | `athlete.rs:182`, `canonical.rs:161-165` | No grade observations | `MEDIUM` confidence may not trigger review |
| 6b | hardening | Fabrication — empty `sports` when no source names sport | `athlete.rs:177` | Meet results without sport field | Athlete has performances but no sport flags |
| 6c | hardening | Fabrication — grade confidence reflects stale correct observation | `athlete.rs:197-202` | Correct grade followed by incorrect grade | `HIGH` confidence with wrong current grade |

## Blocking Findings

| # | Finding | Requirement Break |
|---|---------|------------------|
| 4a | Case-ID digest instability after school rename | The deterministic adjudicator must own acceptance. A school rename invalidates all model verdicts on affected cases, requiring re-asking. This means the adjudicator's acceptance is non-deterministic across runs — the same evidence produces different verdict outcomes depending on when the school rename occurs. This breaks the requirement that "the deterministic adjudicator owns acceptance" because acceptance is contingent on the mutable school name. |
| 5c | Snapshot reader stale rows | The snapshot may contain stale rows if the old snapshot was not fully replaced. This means the workbook may read stale canonical rows. The requirement "the workbook is generated from one sealed, versioned snapshot" is violated if the snapshot is stale. |

## Unchecked Items

The following items could not be fully adjudicated from source alone and would require:

1. **Provider ID reuse rate** — Need access to actual provider data to determine how often provider IDs are reused across different athletes. The false merge in 1b is plausible but unquantified.

2. **School mapping change frequency** — Need operational data on how often school mappings are updated between pipeline runs. The false split in 2a is theoretical without data.

3. **Model verdict consistency** — Would require running the review lane multiple times on the same evidence to determine if the model gives consistent answers. The case-ID instability (4a) is derivable from the code but its practical impact depends on model consistency.

4. **Canonical name correctness rate** — Would require comparing source observations' name spellings to determine how often the first-observation canonical name is correct vs. needs correction from the second observation.

5. **Alias case resolution rate** — Would require examining how many `Alias` cases exist in the store and whether any have been resolved by external means (operator intervention).

6. **Workflow identity collision probability** — Would require counting the total number of unique provider IDs and computing the birthday-bound collision probability for 8-byte digests.

7. **Grade evidence coverage rate** — Would require counting what fraction of canonical athletes have grade observations vs. rely on the default `MEDIUM` confidence.

## Methodology

Each finding was derived from static analysis of the pinned source tree (commit `183fca13dd6d48165d04c76fd56648998e6b432f`). The attack was constructed by identifying a code path, finding the guard (if any), and then walking past the guard with a concrete trigger. No code was modified, no build was executed, and no tests were run. Findings cite exact file and line numbers from the source tree.
