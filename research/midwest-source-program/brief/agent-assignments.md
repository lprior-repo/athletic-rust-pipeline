# Midwest High-School Track/XC Recruiting Source Exploration — Canonical Assignments

(Imported verbatim from the user's phase plan, 2026-09-19.)

## Geographic scope

Prioritize:

Wisconsin
Minnesota
Iowa
Illinois
Michigan
Indiana
Ohio
Missouri
Kansas
Nebraska
North Dakota
South Dakota

Target source-verified Class of 2027 / Grade 11 athletes in:

* boys track and field
* girls track and field
* boys cross-country
* girls cross-country
* indoor track where applicable
* outdoor track

This phase is primarily source research. Agents must not make conflicting production-code modifications.

Each agent owns its assigned domain exclusively and writes its findings to:

`research/midwest/<assignment>.md`

## Core research question

Determine how external sources can:

1. discover athletes missing from Athletic.net ranking enumeration;
2. identify Athletic.net profiles without broad Athletic.net searching;
3. identify Athletic.net meet IDs and team IDs;
4. supply results without hitting Athletic.net;
5. independently verify Class of 2027 / Grade 11;
6. independently verify school identity;
7. find authoritative high-school track/XC coach contacts;
8. identify public athlete recruiting/profile pages;
9. identify new meets/results for weekly incremental refresh;
10. measure Athletic.net coverage gaps.

Do not attempt to collect athlete personal email addresses, personal phone numbers, home addresses, or other unrelated personal data.

For athletes, retain public athletic/recruiting profile URLs and athletic evidence.

For coaches and athletic administrators, retain public professional contact information explicitly published for their school/sport role.

If a directory exposes additional personal information, do not ingest it.

## Required report schema

Every agent must return:

### Source

Provider/site name and URLs.

### Coverage

States, sports, school levels, seasons and historical depth.

### Enumeration

Can we enumerate:

* schools;
* teams;
* meets;
* athletes;
* Class of 2027 athletes;
* results?

Document exact navigation/query structure.

### Stable identifiers

Find and document:

* athlete IDs;
* school/team IDs;
* meet IDs;
* result IDs;
* event IDs;
* season IDs.

Do not assume names are identifiers.

### Athletic.net leverage

Determine whether this source directly exposes:

* Athletic.net meet links;
* Athletic.net team links;
* Athletic.net athlete links;
* Athletic.net result links;

or whether its school/name/grade/meet context can deterministically seed an Athletic.net lookup.

Estimate how many Athletic.net source requests could be avoided.

### Athlete evidence

Determine availability of:

* name;
* graduating class/grade;
* school;
* city/state;
* gender/category;
* TF/XC distinction;
* indoor/outdoor;
* performances;
* PRs;
* progression;
* meets;
* athlete profile URL.

### Recruiting information

Determine whether the source exposes:

* head track coach;
* head XC coach;
* assistant coaches;
* athletic director;
* public professional email;
* school athletics website;
* team website.

Do not collect athlete personal contact information.

### Result evidence

Determine availability of:

* ResultID;
* AthleteID;
* MeetID;
* EventID;
* mark;
* normalized mark inputs;
* timing method;
* wind;
* implement/hurdle specification;
* heat/round;
* place;
* date;
* school represented;
* relay membership.

### Incremental use

Determine how a weekly collector can identify only:

* new meets;
* changed meets;
* new results;
* affected athletes.

Avoid designs that require re-fetching the entire historical corpus weekly.

### Access characteristics

Classify as:

* documented API;
* public structured JSON;
* static CSV/XML/XLSX;
* normal HTML;
* browser application;
* downloadable PDF;
* authenticated;
* subscription-restricted;
* unavailable.

Record published limits and observed `Retry-After` behavior where applicable.

Do not bypass CAPTCHAs, authentication barriers or access controls.

### Recommendation

Choose:

* PRIMARY
* ATHLETIC.NET-SEED
* RESULT-SOURCE
* COACH-DIRECTORY
* VALIDATION
* DISCOVERY-ONLY
* REJECT

Explain the expected marginal coverage.

# Agent assignments

1. **Athletic.net Midwest school/team universe**

   Determine how to enumerate Athletic.net high-school teams across the 12 target states. Map TeamID, school name, location, gender/sport relationships and profile/result links. Investigate whether state-association approved-team lists provide Athletic.net TeamIDs directly.

2. **Athletic.net profile acquisition**

   Using existing HAR evidence, qualify the minimum-request method for acquiring athlete Bio/history through the persistent Chromium session. Map TF, XC, school, grade, meet, season and result fields.

3. **Athletic.net meet acquisition**

   Fully qualify `GetMeetData`, `GetAllResultsData`, `GetResultsData3`, event/division endpoints and MeetID discovery. Determine whether whole-meet results can replace most athlete-profile refreshes.

4. **Athletic.net rankings discovery**

   Determine the cheapest complete method for discovering Grade 11 boys and girls across TF/XC. Test single-event versus multi-event enumeration and identify omissions.

5. **Athletic.net identifier graph**

   Catalog all stable Athletic.net IDs and determine how TeamID, AthleteID, MeetID, ResultID, EventID and SeasonID connect.

6. **Wisconsin WIAA**

   Enumerate WIAA member high schools. Map school IDs, track/XC participation, head track coaches, head XC coaches, ADs and public professional emails. Map tournament/result pages and all links into Athletic.net or timing providers.

7. **Wisconsin MileSplit**

   Determine Class-of-2027 athlete enumeration, school/team pages, athlete IDs, profile URLs, PR/history availability and how well WI MileSplit identities join to Athletic.net.

8. **Wisconsin timing-provider ecosystem**

   Starting from WIAA meets and known Athletic.net meets, identify timers/results hosts. Rank providers by number of 2025-2026 meets. Map result formats and stable IDs.

9. **Minnesota MSHSL**

   Enumerate schools and track/XC teams. Qualify the coach directory for head track/XC coach name, school and public professional email only. Explicitly exclude home address, personal telephone and unrelated fields.

10. **Minnesota results/timers**

    Identify major Minnesota track/XC result providers, including state/championship and Wayzata-style systems where relevant. Determine whether bulk meet results are available.

11. **Iowa IHSAA/IGHSAU**

    Build the Iowa school universe from official association sources. Map Bound relationships, classifications, track/XC participation, official results providers and school/coach directories.

12. **Iowa Wayzata/results ecosystem**

    Investigate Wayzata Results and other Iowa timing systems. Determine meet enumeration, result format, athlete/grade/school fields and historical coverage.

13. **Illinois IHSA**

    Enumerate IHSA member schools and official school-directory IDs. Determine availability of track/XC head coach names and professional emails. Map postseason result providers.

14. **Illinois DirectAthletics**

    Map DirectAthletics Illinois state/team indexes, team IDs, results, meets and school aliases. Determine how much of the Illinois school universe it independently covers.

15. **Michigan MHSAA**

    Exploit MHSAA regional and championship pages that link directly to Athletic.net. Build mappings:

    `MHSAA meet -> Athletic.net MeetID`

    and determine whether schools/team pages similarly map directly.

16. **Michigan athlete/result alternatives**

    Research MileSplit Michigan, timing companies and official result archives for athlete/Class-of-2027 discovery independent of Athletic.net.

17. **Indiana IHSAA**

    Enumerate current member schools and school directories. Map official track/XC postseason result sources, coach/AD information and Athletic.net adoption.

18. **Indiana DirectAthletics/MileSplit**

    Map state team indexes, athlete profiles, results and stable IDs. Measure overlap against Athletic.net.

19. **Ohio OHSAA**

    Deeply map Ohio's 2026 Athletic.net integration. Extract approved high-school team pages, tournament MeetIDs, qualifiers, results and any official school/coach directory information.

20. **Ohio independent athlete sources**

    Map Ohio MileSplit, timing providers and other result databases. Determine whether they improve athlete discovery beyond OHSAA/Athletic.net.

21. **Missouri MSHSAA**

    Map official school/coach information and every track/XC postseason result surface. Explicitly map Athletic.net official-result links.

22. **Missouri PrimeTime**

    Map PrimeTime Timing meet enumeration, live/static results, MeetIDs, result structure and connection to corresponding Athletic.net meets.

23. **Kansas KSHSAA + DirectAthletics**

    Build the Kansas school/team universe. Map DirectAthletics team IDs, official association result sources, coaches and timers.

24. **Nebraska NSAA**

    Build the Nebraska track/XC school universe, official championship/result provider map, coach-directory availability and timing-company landscape.

25. **North Dakota NDHSAA**

    Enumerate all member high schools, track/XC participation, official results, coach resources and timing systems. Determine whether coach identities can be obtained from official public sources.

26. **South Dakota SDHSAA**

    Map Athletic.net usage end to end. Enumerate participating track/XC schools, team pages, result pages, coaches and association IDs.

27. **Midwest MileSplit super-index**

    Across all 12 states, determine whether MileSplit can directly enumerate:

    `Class of 2027 + state + sex + sport`

    Measure athlete counts, profile availability and overlap with the existing Athletic.net boys corpus.

    Produce candidate records containing only athletic identity/evidence.

28. **Midwest DirectAthletics super-index**

    Enumerate all target-state high-school team/league pages, team IDs, meets and results. Produce a canonical school alias map and provider coverage matrix.

29. **Official coach-contact graph**

    Research the best authoritative professional-contact source for every target state.

    Preferred hierarchy:

    `state association`
    -> `official school athletics directory`
    -> `school district staff directory`
    -> `other official school source`

    Output only:

    * school;
    * coach name;
    * sport;
    * role;
    * public professional email;
    * athletic director name/email where public;
    * source URL;
    * last observed date.

    Do not collect personal addresses, personal telephone numbers or athlete contacts.

30. **Cross-source coverage/Pareto analysis**

    Consume the other 29 reports.

    Build a source coverage matrix for every target state and answer:

    * Which sources discover athletes Athletic.net misses?
    * Which sources expose Athletic.net URLs/IDs directly?
    * Which sources eliminate Athletic.net profile requests?
    * Which sources supply verified Grade 11 evidence?
    * Which sources supply coach contact information?
    * Which sources provide bulk meet results?
    * What smallest source set reaches the highest Midwest coverage?
    * What are the estimated requests per newly discovered athlete?
    * What source gaps remain?

    Rank implementation work by marginal verified-athlete coverage per engineering effort.

# Main integration target

After exploration, Main should construct:

`CanonicalSchool`
→ `CanonicalCoach`
→ `CanonicalAthlete`
→ `CanonicalMeet`
→ `CanonicalPerformance`

Athletic.net remains the strongest initial athlete-history source, but other sources should be used to avoid unnecessary Athletic.net work.

Preferred acquisition flow:

`official school/state indexes`
→ `candidate schools`
→ `MileSplit/DirectAthletics/result-source athletes and meets`
→ `known Athletic.net TeamID/MeetID/ProfileURL where available`
→ `targeted Athletic.net acquisition`
→ `canonical cross-source reconciliation`

Do not perform broad Athletic.net profile searching when another qualified source has already supplied a stable Athletic.net URL or enough deterministic context to request one specific candidate.

# Recruiting output

For each source-verified Class-of-2027 athlete, the desired recruiting-facing projection is:

* athlete name;
* school;
* state;
* Class of 2027 evidence;
* events;
* verified PRs;
* recent performances;
* TF/XC;
* public Athletic.net URL;
* public MileSplit URL when available;
* other public athletic/recruiting profile URLs;
* head track coach;
* head XC coach where relevant;
* public professional coach email;
* athletic director;
* public professional AD email when useful;
* school athletics website;
* evidence/confidence status;
* source coverage status.

Athlete personal email, personal phone and home address are outside the collection contract.

# Acceptance goal

The research phase succeeds when we can answer quantitatively:

1. how many Midwest Class-of-2027 athletes each source discovers;
2. how many unique athletes remain after deterministic reconciliation;
3. what percentage have Athletic.net profiles;
4. what percentage have an independently corroborating source;
5. what percentage have a school with an identified track/XC coach;
6. what percentage have a public professional coach contact;
7. how many Athletic.net requests are avoided through external discovery;
8. which states/providers account for remaining coverage gaps;
9. which five to ten production adapters give the best coverage/engineering ratio.
