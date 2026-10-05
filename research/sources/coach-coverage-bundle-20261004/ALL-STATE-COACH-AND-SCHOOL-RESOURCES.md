# All-State Coach and School Resources

Complete research bundle for high-school track & field and cross country.

Research date: 2026-10-04  
Coverage: 25 states; 75 prioritized sources; 278 complete source records.

This file consolidates the entire original shortlist, every manifest field and URL, and all extraction notes. The companion COACH-SCHOOL-RESOURCES-COMPLETE.json retains the original structured manifest, an ordered shortlist_by_state index, and losslessly retained named note sections. Source IDs match across both files.

## Contents

1. [Three prioritized sources per state](#three-prioritized-sources-per-state)
2. [Registry metadata](#registry-metadata)
3. [Complete source registry](#complete-source-registry)
4. [Extraction notes and additional resources](#extraction-notes-and-additional-resources)

## Three prioritized sources per state

75 PRIORITIZED SOURCES: THREE PER STATE
High-school track & field / cross country coaches and schools
Checked: 2026-10-04

25 states: the 22 zero-coach states supplied, plus PA, IL and KS.
Each source is labeled by what it actually provides. Some sources are school inventories, regional subsets, membership lists or dated rosters, rather than complete current coach directories.
A successful page check does not establish complete data coverage or unlimited scraping permission. Read each access and coverage note. The full JSON includes additional sources, observed routes and explicitly blocked/restricted leads.

### TEXAS (TX)
#### 1. UIL sport alignments and school-code inventory
https://www.uiltexas.org/alignments
Provides: School name; conference; region/district; track school codes; organizing-chair contacts where posted
Coverage: UIL members, predominantly public schools and participating charters; not all Texas private schools
Format/access check: HTML indexes with linked PDFs | opened_verified
Use and limitations: Use sport-specific XC and spring alignments, not football districts. Current index has 2026–27 XC and spring files. Spring includes golf/tennis as well as track, so confirm actual sport sponsorship. Alphabetical all-school list and track school codes are useful alias crosswalks. Coach names/emails are not a comprehensive part of these files.
Manifest ID: [SRC-001](#src-001)

#### 2. TEA AskTED school, district and personnel downloads
https://tealprod.tea.state.tx.us/tea.askted.web/Forms/Home.aspx
Provides: School/district identity; addresses; contact information; district/campus personnel reports
Coverage: Texas public schools, districts and ESCs; includes public charters
Format/access check: ASP.NET HTML forms; daily comma-delimited downloads; report exports | opened_verified
Use and limitations: Highest-value public-school entity seed. Homepage says daily updates and offers school/district files with site addresses. Official help documents CSV exports. Follow current download links, preserving cookies/form state if needed; no public REST API verified. Personnel are administrative, not a guaranteed track/XC coach roster. Binary downloads failed in research fetch, but landing and official export documentation are public.
Manifest ID: [SRC-002](#src-002)

#### 3. TAPPS school directory and alignment embeds
https://www.tapps.biz/school-directory-2/
Provides: Published embed field list: schoolName,address,mascot,classification,website,email,telephone
Coverage: Texas private/parochial TAPPS members
Format/access check: WordPress HTML wrapper; TMS JavaScript iframe | opened_verified
Use and limitations: Wrapper and embedded URLs opened. Embedded rows were not rendered by text fetch and shell fetch returned 403, so verify UI row coverage before implementation. This is a public UI embed, not a documented API. Follow pagination rather than assuming limit=12 returns all schools. Coach-specific fields were not verified. Use school websites to reach current coaches.
Manifest ID: [SRC-003](#src-003)

### CALIFORNIA (CA)
#### 1. CIF / Home Campus public school and coaches directory
https://www.cifsshome.org/widget/school/directory
Provides: School identity; school information; athletic faculty; sport; head-coach name; public email; vacancy markers
Coverage: Section selector lists all 10 CIF sections; individual fields verified for Southern Section Arcadia only
Format/access check: Public JavaScript school selector and Coaches and Sports tab | opened_verified
Use and limitations: Highest-priority direct coach source. Cloud browser clicked Arcadia then Coaches and Sports and verified boys/girls XC coach and email, and boys/girls track coach and email. Discover school IDs from rendered school buttons; do not enumerate guessed IDs. Display includes BYE, association placeholders and some middle schools; filter with CDE. Other section presence verified in selector, but their rows/coach completeness not individually audited. No bulk API or hidden endpoint claimed.
Manifest ID: [SRC-017](#src-017)

#### 2. California Department of Education school directory and exports
https://sd.cde.ca.gov/schooldirectory/
Provides: CDS code; school/district; county; type; sector; charter/status; grades; address; administrator and contact fields
Coverage: California public/private/nonpublic schools, districts and county offices
Format/access check: Searchable HTML; Excel/text exports documented | opened_verified
Use and limitations: Use active status and grades including 9–12; retain 14-digit CDS as string. Directory includes K–12 and continuation/alternative high schools, so do not filter name alone. Private-school data page explicitly documents Excel/text export with contacts. Directory is not a coach list. Public bulk-file landing hit CDE WAF during research; prefer verified searchable export route rather than inventing replacement URLs.
Manifest ID: [SRC-018](#src-018)

#### 3. MileSplit California team directory
https://ca.milesplit.com/teams
Provides: Team ID; school/team; city; section abbreviation in many team names; profile URL
Coverage: Statewide HS plus mixed school levels/clubs/inactive entries
Format/access check: Public HTML team index | opened_verified
Use and limitations: Strong independent sport-program/alias crosswalk. Example suffixes SS, CC, NC, SJ, NS, CS encode sections but should be validated. Use CDE and CIF to exclude non-HS/closed schools. Coach coverage not demonstrated, no API verified. Do not traverse athlete rosters.
Manifest ID: [SRC-019](#src-019)

### NEW YORK (NY)
#### 1. NYSPHSAA official section map and section links
https://nysphsaa.org/sports/2021/6/7/section-map.aspx
Provides: Section jurisdiction, official section websites, section office contacts
Coverage: 11 NYSPHSAA sections; separate NYC PSAL/CHSAA/independent coverage needed
Format/access check: HTML | opened_verified
Use and limitations: Use as authoritative source-of-sources. Section staff are not school coaches. Follow all 11 section links and member-school/league/sport pages.
Manifest ID: [SRC-050](#src-050)

#### 2. PSAL Cross Country school/team listing
https://www.psal.org/sports/sport.aspx?flag=All&spCode=033
Provides: School/team names and IDs; published client code renders head, co-, and assistant coach names from team records
Coverage: NYC PSAL programs; campus programs may span schools
Format/access check: ASP.NET HTML; client-routed team profiles | opened_verified
Use and limitations: Observed profile https://www.psal.org/profiles/team-profile.aspx#033/13507; boys/girls XC codes 033/034. Public https://www.psal.org/scripts/js/Team_Profile.js verified with getTeamDetails and getCoachNamesIds calls and head/co/assistant coach rendering. These are observed site-internal endpoints, not a supported public API; actual JSON responses not tested. Harvest real school/sport IDs and current season from page; fragment needs JS. Never call getTeamRosters or athlete/statistics endpoints for this project.
Manifest ID: [SRC-054](#src-054)

#### 3. NYSED School and District Directory / SEDREF public reports
https://www.p12.nysed.gov/irs/schoolDirectory/
Provides: Institution identity; public/nonpublic school universe; school contacts and grades via linked reports
Coverage: Statewide public, charter, nonpublic and BOCES
Format/access check: HTML hub; public query; report exports | opened_verified
Use and limitations: Direct HTTPS GET returned 200 after web-reader failure. Observed links: https://portal.nysed.gov/pls/sedrefpublic/SED.sed_inst_qry_vw$.startup and http://eservices.nysed.gov/sedreports/list?id=1 . Follow public reporting workflow; do not persist transient encrypted obrar.cgi URLs. Sport/coach data requires enrichment.
Manifest ID: [SRC-049](#src-049)

### OHIO (OH)
#### 1. myOHSAA public sport-by-school coach directory
https://officials.myohsaa.org/Outside/Schedule/SportsInformation?ohsaaId=582
Provides: school ID/name/address; sport; boys/girls head coach; public email when supplied; division; season
Coverage: Member schools with supplied sports staff
Format/access check: server-rendered HTML table | opened_verified
Use and limitations: Exact working host is officials.myohsaa.org. Bishop Fenwick sample actually rendered 2026-27 XC and track coaches/emails. Parse N/A as sport not offered and TBA as vacant/unknown. Keep separate gender appointments even when person is shared. Pair with OHSAA enrollment seed.
Manifest ID: [SRC-098](#src-098)

#### 2. OATCCC current coach membership
https://www.oatccc.com/Coaches/Membership/
Provides: first name; last name; school; county
Coverage: Association members, including lifetime members and some missing/unaffiliated schools; not all Ohio coaches
Format/access check: HTML landing plus public Google Sheet | opened_verified
Use and limitations: 2026 sheet opened and rows verified: https://docs.google.com/spreadsheets/d/1i8mbrZolrxSxJN8jOTOWfnjNEMR38RVd/edit?gid=335617153&rtpof=true&sd=true . No emails or head/assistant role columns observed. Membership is a candidate signal; confirm current HS appointment locally.
Manifest ID: [SRC-100](#src-100)

#### 3. Ohio Educational Directory System public extract
https://oeds.education.ohio.gov/dataextract
Provides: organization type; grades served; NCES school ID; selectable organization and role fields
Coverage: Public, charter/community, STEM and chartered nonpublic schools
Format/access check: public report-generation form | opened_verified
Use and limitations: Choose school-level types and high-school grade range. Public extract form observed; export submission was not exercised. No documented REST API verified. Join NCES/local identifiers to school sites; role exports are not automatically coach lists.
Manifest ID: [SRC-101](#src-101)

### NEW JERSEY (NJ)
#### 1. NJSIAA Member Information
https://www.njsiaa.org/schools/member-information
Provides: School name,address,phone,athletic director name/phone
Coverage: Statewide NJSIAA public, nonpublic, charter members
Format/access check: Public HTML table; ?page=0, ?page=1 pagination | opened_verified
Use and limitations: Scrape list pages only and follow real pager links. Opened school detail /schools/abraham-clark-high-school redirects to /user/login?destination=...; do not assume contacts behind detail are public. AD is not track coach.
Manifest ID: [SRC-058](#src-058)

#### 2. NJXCTFCA county association link index
https://njxctfca.org/links/
Provides: Verified outgoing county/coaches association URLs
Coverage: NJ county/regional associations
Format/access check: HTML hyperlinks | opened_verified
Use and limitations: Links include South Jersey, Shore, Bergen, Passaic, Hudson, Union, Essex, Mercer, Middlesex, Morris. Crawl each subject to current access; some old providers such as webs.com may be obsolete.
Manifest ID: [SRC-062](#src-062)

#### 3. MileSplit NJ team directory
https://nj.milesplit.com/teams
Provides: Team/school name,city,team page URL and platform ID
Coverage: State teams including HS,MS,college,clubs and historical/closed records
Format/access check: Public HTML alphabetical directory; level filter | opened_verified
Use and limitations: Opened state team listing without login. Filter to high school; cross-check current official school inventory and season. Team pages/coach contact presence varies and was not assumed. Do not scrape student rosters, athlete profiles or results. No public documented API verified.
Manifest ID: [SRC-087](#src-087)

### FLORIDA (FL)
#### 1. FHSAA public Home Campus school and coach directory
https://www.cifsshome.org/widget/school/directory?section=10&school=
Provides: School ID; sport; head-coach name/email; position-not-filled marker; school/AD data tabs
Coverage: FHSAA public/private/charter members; selector includes middle schools and placeholders
Format/access check: Public JavaScript shared Home Campus directory | opened_verified
Use and limitations: In cloud browser selected FHSAA from public CIF widget, then Bolles, then Coaches and Sports. Verified separate boys/girls XC coach+email, boys track coach+email and girls track Position not filled. Observed FHSAA selector listed 880 entries, including middle schools, so this is not 880 high schools. Use school IDs discovered from UI; no hidden API or bulk endpoint claimed. FHSAA native member-directory wrapper and sport-filtered locations widget are also verified.
Manifest ID: [SRC-034](#src-034)

#### 2. MileSplit Florida / flrunners team directory
https://fl.milesplit.com/teams
Provides: Team ID; school/team; city; profile URL
Coverage: Florida sport programs, with middle school/college/club/closed entries mixed
Format/access check: Public HTML index | opened_verified
Use and limitations: Strong crosswalk and gap seed, not a confirmed complete coaching directory. Preserve team IDs; reconcile school status and level. The index explicitly includes closed-school labels and middle schools. No coach API verified and student roster/result extraction is out of scope.
Manifest ID: [SRC-035](#src-035)

#### 3. Florida DOE private-school contact export
https://web09.fldoe.org/PrivateSchoolDirectory/DownloadSchools
Provides: School name; address; director name; phone; school email
Coverage: Florida private-school annual survey directory; self-reported, not accreditation
Format/access check: HTML download form; Excel export | opened_verified
Use and limitations: Official page states exact exported contact fields and offers Download All Schools or district selection. Main search initially displayed zero pending filters/loading; that is not zero schools. Use export and grade filters/profile checks, then official athletics pages for coaches. Do not infer that director is coach or DOE endorses a listed school. Binary export not downloaded in this research.
Manifest ID: [SRC-036](#src-036)

### VIRGINIA (VA)
#### 1. Virginia Department of Education public-school alphabetical directory
https://www.va-doeapp.com/publicschoolsalphabetical.aspx?w=true
Provides: School name, street address, phone, principal, grade span, school-division website
Coverage: Virginia public schools, all grades
Format/access check: Large public HTML table | opened_verified
Use and limitations: Best clean public-school seed. Opened table includes 9–12 and combined-grade high schools. Filter grade span rather than name alone; deduplicate academies sharing a high-school campus. Principal is not coach. Follow division/school athletics staff pages for current adults.
Manifest ID: [SRC-184](#src-184)

#### 2. VISAA member-school directory
https://www.visaa.org/schools
Provides: School, address, official website, mascot, conference, provisional membership notes
Coverage: Current VISAA private/independent members; excludes nonmembers
Format/access check: Public HTML school cards | opened_verified
Use and limitations: Strong private-school seed with outbound official domains and conference names. Resolve each school then follow athletics/team/coaches pages. Do not infer sport participation from association membership; join XC/track division lists.
Manifest ID: [SRC-185](#src-185)

#### 3. VHSL alignment/classification hub
https://www.vhsl.org/alignment/
Provides: School class, region, district and alignment documents
Coverage: VHSL public and approved nonpublic membership
Format/access check: HTML hub with linked alignment documents | opened_verified
Use and limitations: Third distinct authoritative system. Follow current alignment documents and retain their effective years. Primary member-directory route is also listed below but returned 403. Alignment membership is a seed, not proof of a particular coach.
Manifest ID: [SRC-186](#src-186)

### MICHIGAN (MI)
#### 1. MHSAA public school directory and coach tabs
https://www.mhsaa.com/schools
Provides: school ID; name; city; school level; website/address/phone; sports; coach names by gender and sport
Coverage: Michigan member/participating schools, including private; search also contains junior-high/nonmember-like entries
Format/access check: JavaScript-rendered public directory | opened_verified
Use and limitations: Cloud browser search Novi returned HS/JH rows. Sample https://www.mhsaa.com/schools/novi → Staff → Coaches publicly rendered boys/girls XC and track names; no coach emails visible. Profile School ID 1833 differs from internal calendar SchoolId 3933. Keep source IDs separately. No data API verified.
Manifest ID: [SRC-106](#src-106)

#### 2. Michigan EEM public school data export
https://cepi.state.mi.us/eem/PublicDatasets.aspx
Provides: entity IDs; school/facility names; grades; addresses; contact information
Coverage: Public LEA/PSA/ISD/state schools and nonpublic schools
Format/access check: ASP.NET public data export form | opened_verified
Use and limitations: Choose school entity types, then filter grades. Download-format control and column-description PDF observed. Public export guidance explicitly says login is unnecessary: https://www.michigan.gov/cepi/pk-12/eem/creating-lists-and-mailing-labels . Contact exports include administrator information, not a sport assignment guarantee.
Manifest ID: [SRC-107](#src-107)

#### 3. MITCA track/XC coaches association
https://mitca.org/MITCA/
Provides: association contacts; regional coach award routes; school associations from award material where present
Coverage: Statewide association with public enrichment limited to exposed contacts/awards
Format/access check: WordPress HTML and linked documents | opened_verified
Use and limitations: Correct working path is mitca.org/MITCA/. www.mitca.org root failed in web tool. Site announces a move toward https://runsignup.com/w/MITCAMeets (linked, not opened). Contacts: https://mitca.org/MITCA/about-mitca/mitca-committees-contacts/; awards: https://mitca.org/MITCA/mitca-awards/regional-coach-of-the-year/ both opened. No comprehensive public member directory verified.
Manifest ID: [SRC-108](#src-108)

### MISSOURI (MO)
#### 1. MSHSAA member-school listing and coaching-roster route
https://www.mshsaa.org/Schools/SchoolListing.aspx
Provides: School name, member type, county, city, numeric school ID; linked coach names and head/assistant roles
Coverage: Statewide full members, affiliates and homeschool associations; includes junior high
Format/access check: Public HTML table plus per-school HTML | opened_verified
Use and limitations: Highest-value Missouri system. Crawl actual school links, filter high-school grades, then sport links and Coaches tabs. Verified sample https://www.mshsaa.org/MySchool/Coaches.aspx?alg=11&s=85 shows 2025–26 boys XC head/assistant names; no coach email shown. Preserve displayed season.
Manifest ID: [SRC-195](#src-195)

#### 2. Missouri Track and Cross Country Coaches Association
https://www.mtccca.org/
Provides: Selected coach names, school affiliations, sport/class awards; officer and clinic links
Coverage: Association community; public pages are selective, not all members
Format/access check: Public Wix HTML | opened_verified
Use and limitations: Second distinct coach-bearing system. /xc opened with 2025 class-by-class boys/girls Coach of the Year names and schools. Use as corroboration or discovery only; no complete public membership roster verified. Do not infer an award winner still holds the role.
Manifest ID: [SRC-196](#src-196)

#### 3. Missouri Nonpublic School Accrediting Association member schools
https://www.moqualityschools.com/member-listing1.html
Provides: Accredited school names and school links
Coverage: 265 nonpublic schools stated on page; many are elementary-only
Format/access check: Public HTML directory | opened_verified
Use and limitations: Third distinct scalable source for nonpublic gaps. Filter to high-school grades using official school pages or NCES; join to MSHSAA/private athletics, then staff pages. Accreditation membership does not establish track/XC participation.
Manifest ID: [SRC-197](#src-197)

### MASSACHUSETTS (MA)
#### 1. Massachusetts State Track Coaches Association current members
https://mstca.org/coaches-corner/current-members
Provides: Member ID, first/last name, organization, email and phone in public embedded page data; membership status
Coverage: MSTCA membership; not a complete current coaching census
Format/access check: Next.js HTML plus interactive pagination | opened_verified
Use and limitations: Direct GET verified Next.js self.__next_f.push page-data with members array: 24 rows on first page, totalCount 1417. Fields: _id, firstName, lastName, organization, email, phone, status, lastPaymentDate, image, altText. First page had 24 email values, 11 equal unknown@mstca.org. Parse public payload or render normal pagination; no separate API verified. Reject placeholders; collect only name/org and professional contact fields. Membership or old payment date does not prove current coach employment/sport. Confirm at school site.
Manifest ID: [SRC-068](#src-068)

#### 2. MIAA Schools Directory and school profiles
https://www.miaa.net/schools
Provides: School, district, league, principal, athletic director, mascot; published school details
Coverage: MIAA public and private/parochial member schools
Format/access check: Drupal HTML paginated listing and /group/{id} profiles | opened_verified
Use and limitations: Observed school profile https://www.miaa.net/group/2 lists principal and AD, not track coaches. Enumerate real links/pagination; use athletics school seed to find staff. Membership PDF at https://www.miaa.net/media/824 is 32 pages but landing says updated 2024-07-18.
Manifest ID: [SRC-069](#src-069)

#### 3. Massachusetts DESE public/private directories
https://profiles.doe.mass.edu/search/search.aspx
Provides: School/org name,organization type, profiles and export/search capabilities
Coverage: Statewide public, charter, private, approved special education
Format/access check: ASP.NET search/directories and export workflow | opened_verified
Use and limitations: Observed public-directory link https://profiles.doe.mass.edu/search/search_link.aspx?leftNavId=11238&orgType=6,13&runOrgSearch=Y and private link orgType=11. Link pages opened as short redirect shell; render normal search if needed. No documented JSON API found. Include high-school grade ranges, then school athletics sites.
Manifest ID: [SRC-072](#src-072)

### INDIANA (IN)
#### 1. Indiana DOE downloadable school directory
https://www.in.gov/doe/it/data-center-and-reports/
Provides: school/corporation directory records; workbook columns require implementation inspection
Coverage: Indiana public and accredited nonpublic schools; confirm individual workbook sheet coverage
Format/access check: HTML landing and XLSX | opened_verified
Use and limitations: Exact download href verified in page HTML: https://www.in.gov/doe/files/2025-2026-school-directory-2026-03-23.xlsx . A 447,924-byte XLSX downloaded locally; sheet fields not validated in this pass. Landing labels update 4/1/2026. Use grade fields to restrict HS; no coach role assumed.
Manifest ID: [SRC-113](#src-113)

#### 2. Indiana IHSAA member school directory
https://www.ihsaa.org/schools/ihsaa-school-directory
Provides: member school universe; public/nonpublic membership totals; myIHSAA contact/profile links
Coverage: 413 members shown for 2026-27: 356 public and 57 nonpublic, including provisional schools
Format/access check: HTML hub and JavaScript myIHSAA application | opened_verified
Use and limitations: Hub directs to https://www.myihsaa.net/ . Application opened as JS shell, with no individual directory rows verified. Official /schools page says directory has contacts/directions/profiles. Do not claim coach-email coverage or invent API routes. Public conference list is a fallback seed.
Manifest ID: [SRC-114](#src-114)

#### 3. IATCCC officers and school affiliations
https://iatccc.org/officers-and-council/
Provides: current officer name; school; email; historical officer years
Coverage: Four 2025-26 officers plus historical entries; not statewide membership
Format/access check: HTML tables | opened_verified
Use and limitations: Separate current officers table from historical table. Main https://iatccc.org/ provides recent clinic/coach-award enrichment. Membership signup is not a public membership database; no statewide searchable member list/API verified.
Manifest ID: [SRC-115](#src-115)

### WASHINGTON (WA)
#### 1. WIAA Washington member-school directory
https://wiaa-dna4aga5arc0gyeb.westus2-01.azurewebsites.net/directory.aspx?SecID=654
Provides: school; type; level; classification; league; school district; WIAA district
Coverage: WIAA membership, public and private; filter high-school level
Format/access check: ASP.NET HTML search form | opened_verified
Use and limitations: Search controls opened. Enumerate available high-school/league filters and follow returned school links. No coach export or API verified. Current www.wiaa.com/schools/ landing returned 403 in this session. Do not confuse with Wisconsin schools.wiaawi.org.
Manifest ID: [SRC-136](#src-136)

#### 2. Seattle Public Schools coach-page network, West Seattle example
https://westseattlehs.seattleschools.org/student-life/athletics/coaches/
Provides: school; season; sport; gender; head coach; email
Coverage: Seattle public high schools; directly verified at West Seattle and Ballard, not statewide
Format/access check: HTML; district school selector links | opened_verified
Use and limitations: 2026-27 coach list explicitly labels all four target teams. Enumerate high-school subdomains from Select school menu, then follow actual athletics/contact links. Paths vary; avoid guessing one path for every school. Exclude elementary/middle-school sections.
Manifest ID: [SRC-137](#src-137)

#### 3. WSCCCA leadership and district advisory board
https://www.wsccca.com/leadership
Provides: coach name; school; district representation; some emails; association role
Coverage: Small statewide representative subset, NOT all Washington coaches
Format/access check: HTML | opened_verified
Use and limitations: Includes executive, advisory and service boards with school affiliations. Treat these as partial coach leads; confirm current school coaching job on school website. Do not label this a statewide member directory.
Manifest ID: [SRC-138](#src-138)

### IOWA (IA)
#### 1. Iowa IHSAA member school list and detail pages
https://www.iahsaa.org/member-schools/
Provides: school; conference; school colors/nickname; linked school details
Coverage: Statewide high-school members, public and private
Format/access check: HTML table | opened_verified
Use and limitations: Current table dated Aug 4, 2026. Sample https://www.iahsaa.org/schools/acgc/ opened. Official contact page sends school-contact users to https://www.gobound.com/ia/schools . No current coach list visible on IHSAA sample itself; use linked Bound route with access caveats.
Manifest ID: [SRC-128](#src-128)

#### 2. Iowa DOE public and nonpublic building directories
https://educate.iowa.gov/directories
Provides: public district/building and nonpublic building directory records; workbook fields not inspected
Coverage: Statewide public and nonpublic PK-12 schools
Format/access check: HTML landing with XLSX downloads | opened_verified
Use and limitations: Three 2026-27 Excel downloads dated Sep 30, 2026: public district, public building, nonpublic building. Prefer building files then filter grades. Exact binary download URLs were not resolved in this pass; follow current links. ArcGIS school-building API below provides verified schema.
Manifest ID: [SRC-130](#src-130)

#### 3. Iowa Association of Track Coaches
https://www.iatrackcoaches.org/
Provides: public school membership; coach/advisory contacts; coaching awards
Coverage: Statewide association; public lists are partial
Format/access check: HTML | opened_verified
Use and limitations: Membership list https://www.iatrackcoaches.org/membership-list/ verified 2026-27 school + class rows, not individual coach directory. Contact page https://www.iatrackcoaches.org/contact-us/ opened. Good supplement for schools and named coaches but neither complete nor guaranteed current head-coach assignments.
Manifest ID: [SRC-132](#src-132)

### ARIZONA (AZ)
#### 1. AIA member-school and sport-coach directory
https://aiaonline.org/schools
Provides: school ID; name; district; website; address; sport; head coach; administrative phone
Coverage: 287 member schools claimed by landing; public/private statewide
Format/access check: JSON school search plus HTML school profiles | opened_verified
Use and limitations: GET /schools/search.json?q=chandler returned 10 JSON school records. Parse school profiles /schools/{id}; verified /schools/100 has XC boys/girls and Track boys/girls head coaches. Empty q returned only 20, not complete inventory. Public emails redirected to admin directory; no public coach-email API verified.
Manifest ID: [SRC-155](#src-155)

#### 2. CAA GameSource sport team and coach tables
https://teams.gamesource.io/teams.php?association=CAA&association_sport_id=44&association_sport_level_id=198&division_id=8
Provides: school/team; head coach; assistant coach; coordinator; team profile links
Coverage: Canyon Athletic Association; charter/private/home-school and other small-school programs
Format/access check: HTML tables and team links | opened_verified
Use and limitations: Observed exact parameters select Track & Field Boys Varsity. Preserve team-to-coach row boundaries; several coaches can follow one team. Discover other sport/level IDs from page controls, never guess. Some rows have no coach. Historical/freshness caveat: footer 2025; reconcile 2026-27 against CAA current site/Bound.
Manifest ID: [SRC-156](#src-156)

#### 3. ADE Active LEAs and Schools
https://www.azed.gov/finance/local-education-agencies
Provides: active LEA and school names/identifiers; XLSX schema not verified
Coverage: Fundable active public/charter LEAs and school sites; FY2026
Format/access check: XLSX linked from HTML | opened_verified
Use and limitations: Exact observed file https://www.azed.gov/sites/default/files/2025/09/FY2026%20Active%20LEAs%20and%20Schools.xlsx. Web loader cannot parse XLSX and direct file request returned 403; landing verified only. Use latest link rather than hardcoding fiscal year. Does not imply complete private-school coverage.
Manifest ID: [SRC-157](#src-157)

### OKLAHOMA (OK)
#### 1. OSSAARankings school directory and sport schedules
https://www.ossaarankings.com/default.aspx?sc=1117&sel=ssch&st=OK
Provides: School names, school links/IDs, sport schedule/class links
Coverage: Broad Oklahoma athletics membership including private schools
Format/access check: Public ASP.NET HTML | opened_verified
Use and limitations: Alphabetical school links were fully exposed. Discover exact links for each sport/gender; parse school identities separately from schedules. Query parameters are observed website routes, not a documented public API. Coach/contact data must be checked on school pages.
Manifest ID: [SRC-203](#src-203)

#### 2. Oklahoma State School and District Directory
https://oklahoma.gov/education/resources/state-school-directory.html
Provides: Physical/mailing addresses, phone, email, website and district/site identifiers
Coverage: OSDE-accredited education sites
Format/access check: Public HTML download hub | opened_verified
Use and limitations: Hub opened and explicitly describes school/district downloads and field coverage; download click failed in this research tool, so file schema is not inspected. Use published School Directory link, not guessed API. Data is reported annually; note observation and reporting years.
Manifest ID: [SRC-204](#src-204)

#### 3. NCPSA Oklahoma accredited-private-school directory
https://ncpsa.org/directory/oklahoma/
Provides: School name, city, grade range, accreditor, detail links
Coverage: 55 accredited Oklahoma private schools stated; not all private schools
Format/access check: Public HTML city groups | opened_verified
Use and limitations: Third accessible distinct inventory. Page says updated Sept 25, 2026. Filter high-school grade spans and verify official domain/athletics. Some grade values may be missing or malformed; do not discard without resolving school page.
Manifest ID: [SRC-205](#src-205)

### LOUISIANA (LA)
#### 1. LHSAA 2025–2026 Coaches Directory
https://www.lhsaa.org/siteuploads/editorimg/file/Administration/25-26/LHSAA%20Coaches%20Directory%20-%202025%20-%202026.pdf
Provides: Coach/school directory; exact column schema requires visual/OCR validation because extraction is garbled
Coverage: LHSAA member schools, public and private; 108 pages
Format/access check: Public PDF | opened_verified
Use and limitations: Highest-value Louisiana system, linked as School Administration Directory from current LHSAA navigation. Opened 108-page PDF, but extracted text is badly font-encoded. Render/OCR and verify representative pages before production. Pair with clean XC registered-coach PDF below; no assumption that all emails are present.
Manifest ID: [SRC-212](#src-212)

#### 2. Louisiana BESE-approved nonpublic-school list 2026–2027
https://doe.louisiana.gov/docs/default-source/nonpublic-schools/information---nps-2026-2027-approval-with-brumfield-v-dodd.pdf?sfvrsn=aa8fa3de_2
Provides: School name, site/district codes, parish, grade span, approval classification
Coverage: BESE-approved nonpublic schools; voluntary approval means not all private schools
Format/access check: Public 19-page PDF table | opened_verified
Use and limitations: Second distinct authoritative system. Opened current PDF from LDOE Nonpublic School Resources. Filter high-school grades and preserve parish/code identifiers for joins. Do not ingest student/enrollment-by-race tables elsewhere on hub.
Manifest ID: [SRC-213](#src-213)

#### 3. DirectAthletics Louisiana track team directory
https://www.directathletics.com/leagues/track/89.html
Provides: Team names and separate men's/women's team IDs/links
Coverage: Broad Louisiana teams on platform; historical/JV/club contamination possible
Format/access check: Public HTML link table | opened_verified
Use and limitations: Third distinct directly inspectable sport seed. Enumerate team links, collapse boys/girls identity at school layer while retaining program relation, exclude JV/club-only entries when outside scope. No coach-email API verified; school staff enrichment still needed.
Manifest ID: [SRC-214](#src-214)

### OREGON (OR)
#### 1. OSAA member-school profiles and coach directory
https://www.osaa.org/schools/regions
Provides: school ID; school name; type; website; address; league; sport; head coach; athletic director contacts
Coverage: Full OSAA members, public/private; includes some WA-border schools
Format/access check: Server-rendered HTML lists and profiles | opened_verified
Use and limitations: Enumerate profile hrefs from regional/member list. Verified profile https://www.osaa.org/schools/46 has Sports / Activities rows with all four target sports and coach names. A separate Contact Information table is administrative: do not mislabel AD emails as coach emails.
Manifest ID: [SRC-146](#src-146)

#### 2. ODE Institutions Database and daily extract
https://www.ode.state.or.us/instID/
Provides: institution ID; name; city; county; district; ESD; open/closed status
Coverage: Public schools plus other institutions doing business with ODE; not complete private registry
Format/access check: Search HTML plus documented zipped Excel 8.0 daily extract | opened_verified
Use and limitations: Landing documents daily all-institution ZIP/XLS download. Download link itself failed in web fetch and shell landing returned 502. Current canonical explanation page links this application; do not invent JSON API. Filter school institutions, active status and high-school grades.
Manifest ID: [SRC-147](#src-147)

#### 3. OACA Coach of the Year 2025-26
https://oregoncoach.org/oaca-coach-of-the-year/
Provides: coach; school; sport; classification; award season
Coverage: Selected award winners only
Format/access check: HTML headings and bullet lists | opened_verified
Use and limitations: Useful dated coach-school validation; not current roster or all coaches. Store award year and verify employment independently.
Manifest ID: [SRC-151](#src-151)

### CONNECTICUT (CT)
#### 1. CAS-CIAC MobileDir legacy school list
https://www.casciac.org/mobiledir/
Provides: School names,towns; school detail href identifiers
Coverage: CAS/CIAC school members; multiple grade levels
Format/access check: HTML list; search.cgi?a=a&id=406 example | opened_verified
Use and limitations: Opened CAS-CIAC public school-list entry point. IMPORTANT: this is a legacy list containing old/closed school names; reconcile with current EdSight inventory before use. School detail example opened near-empty text. Stronger sport-specific head-coach route is https://ciac.fpsports.org/Directory.aspx?SchoolLevelID=1 but direct access redirects to WebBotZone.aspx Unauthorized Access/login from this location. Do not bypass. Use verified current official school athletics/staff pages for appointments. MobileDir alone supplies neither a current coach roster nor current membership guarantee.
Manifest ID: [SRC-077](#src-077)

#### 2. Connecticut EdSight school/district directory and exports
https://public-edsight.ct.gov/general-information
Provides: School,district,type,location,export links; contacts separate
Coverage: Connecticut public and nonpublic school reporting universe
Format/access check: HTML hub plus embedded reports/export to Excel | opened_verified
Use and limitations: Opened official hub. Observed https://public-edsight.ct.gov/overview/find-schools/find-schools-export and /overview/find-schools/find-school-district . Report notes distinguish nonpublic secondary schools and APSEPs. Use grades/status to seed athletics staff discovery; no documented JSON API verified. Source HTML of /overview/find-schools/find-school-district embeds https://edsight.ct.gov/SASStoredProcess/do with _program=/CTDOE/EdSight/Release/Reporting/Public/Reports/StoredProcesses/OrgSearchReport_SiteCore, orgtype/orgdistrict/orgname query fields. This is an observed embedded report route, not a documented JSON API; report response not tested.
Manifest ID: [SRC-078](#src-078)

#### 3. NEPSAC independent member schools
https://nepsac.org/about/nepsac-member-schools/
Provides: School name and direct official website
Coverage: NEPSAC member prep schools across New England and some NY; state filter needed
Format/access check: HTML linked school list | opened_verified
Use and limitations: Public list has direct school websites and covers institutions outside state associations. Filter state and upper-school grades; some schools are middle-only. Follow athletics > teams/coaches/staff; members do not all sponsor XC/track. No statewide coach roster implied.
Manifest ID: [SRC-093](#src-093)

### UTAH (UT)
#### 1. UHSAA school profiles and coaches
https://uhsaa.org/school-directory-new/
Provides: schoolID; name; address; district; classification; region; sport; coach; email
Coverage: UHSAA public/charter/private member schools statewide
Format/access check: Enumerable HTML links and profile tables | opened_verified
Use and limitations: Follow actual profile links with Reg/id/schoolID query parameters. Verified Alta schoolID=1 has coach names/emails for all four target teams; some Summit Academy cells empty. Empty coach does not prove absent sport. Parse Coaches separately from administrator roles.
Manifest ID: [SRC-164](#src-164)

#### 2. USBE Utah Schools Directory
https://schools.utah.gov/schoolsdirectory
Provides: school and district/LEA; full schema depends on live data
Coverage: Utah school/LEA directory reported via CACTUS; do not assume exhaustive private coverage
Format/access check: JavaScript table; client-generated CSV; observed JSON endpoint | opened_verified
Use and limitations: Raw official HTML contains dataUrl=https://cactus.schools.utah.gov/api/legacy/schools and Export CSV generated from rendered table. Endpoint requests returned 502 twice, so record endpoint as observed but unavailable, not verified successful JSON. District directory is accessible alternative.
Manifest ID: [SRC-165](#src-165)

#### 3. Grand County High School sports coaches
https://gchs.grandschools.org/apps/pages/index.jsp?uREC_ID=1651964&type=d&pREC_ID=2260652&tota11y=true
Provides: school; sport; coach; email
Coverage: One school; concrete official-site fallback, NOT statewide
Format/access check: Edlio HTML page | opened_verified
Use and limitations: Verified explicit target-sport coaches/email entries. Use a school-site adapter after statewide UHSAA/USBE seeds; discover actual athletics/staff links. Retain sports separately even when the same coach appears multiple times.
Manifest ID: [SRC-166](#src-166)

### NEVADA (NV)
#### 1. Southern Nevada Track & Cross Country Coaches Association directories
https://www.sntccca.org/
Provides: school; boys/girls; classification; head coach; assistant coach; email; phone
Coverage: Southern Nevada region; schools plus non-coach operational contacts
Format/access check: Public Google Sheets; anonymous CSV exports verified HTTP 200 | opened_verified
Use and limitations: Two 2026-27 sheets linked from association. Track ID 1cs60d8y7YkfLlLVENZlgx4-kYOqfAPQeJU4fcQmp4oY; XC ID 1FijPtauqQ9N5v5-EZI0Ts3RirYIN0e2d-RQrFvla-7k. Append /export?format=csv to /spreadsheets/d/{id}. Self-maintained; strip preamble, timers, officials and coordinator rows.
Manifest ID: [SRC-175](#src-175)

#### 2. Nevada Department of Education public and private school inventories
https://doe.nv.gov/school-and-district-information
Provides: school/state/NCES identifiers; name; grades; status; address; website; phone; principal email
Coverage: Statewide public/charter; separate licensed/exempt private directory
Format/access check: XLSX files; actual downloads and shared-string headers verified | opened_verified
Use and limitations: 2026-27 public school workbook is snapshot 2026-09-16. Public exact file: https://webapp-strapi-paas-prod-nde-001.azurewebsites.net/uploads/school_directory_9b69a05740.xlsx. Private route under NDE office student/school supports. Follow current landing links because hashed names can change. No coaches in inventories.
Manifest ID: [SRC-176](#src-176)

#### 3. Washoe County official school-site coach directories
https://reno.washoeschools.net/activities/athletics/welcome
Provides: school; sport; coach name; email where supplied
Coverage: Northern Nevada district schools; Reno/North Valleys examples verified, not statewide
Format/access check: Finalsite HTML and linked coach PDFs | opened_verified
Use and limitations: Reno welcome page publishes Coaches Directory; North Valleys staff page explicitly annotates XC/track roles. Enumerate district high-school websites, then actual athletics/coaches/staff pages. Do not confuse college University of Nevada coaching records.
Manifest ID: [SRC-177](#src-177)

### SOUTH DAKOTA (SD)
#### 1. SDCCTFCA 2024–2025 membership roster
https://cdn1.sportngin.com/attachments/document/cdfc-2949014/_1__SDCCTFCA_Membership_as_of_01-07-2025_.pdf
Provides: Last name, first name, email, member title, school
Coverage: Association members as of Jan 7, 2025; not all statewide coaches; 10 pages
Format/access check: Public text-readable PDF table | opened_verified
Use and limitations: Highest-value direct coach source. Linked from https://www.sdhsca.org/page/show/6314203-membership . Preserve member title and school; membership does not identify exact sport/gender/head role unless explicitly printed. Validate 2026 employment and professional use of listed email before outreach.
Manifest ID: [SRC-222](#src-222)

#### 2. South Dakota Department of Education educational directory
https://doe.sd.gov/ofm/edudir.aspx
Provides: Public/nonpublic/tribal system names, system IDs, directory detail links; principal/school Excel advertised
Coverage: Statewide accredited public, nonpublic and tribal/BIE-related systems plus other entity types
Format/access check: Public HTML index, district query pages, Excel/PDF links | opened_verified
Use and limitations: Second distinct clean seed system. Principal/school Excel link labeled updated Sept 8, 2026, though binary click failed here. Follow actual district links and retain IDs, school grade spans and system type. Exclude community-support and preschool-only entities.
Manifest ID: [SRC-223](#src-223)

#### 3. SDHSAA official member directory on Bound
https://www.gobound.com/sd/associations/sdhsaa/schools
Provides: School identity and school links expected; records not exposed in text-only extraction
Coverage: SDHSAA members including public/private and co-ops
Format/access check: Public JavaScript app | opened_verified
Use and limitations: Third distinct official athletics system. SDHSAA homepage links this exact Member Directory URL. Only application shell opened; rendered rows/contact schema not verified. Use official school URLs when available, and stop at any login wall; no private Bound API documented.
Manifest ID: [SRC-224](#src-224)

### VERMONT (VT)
#### 1. Vermont Principals Association athletics
https://vpaonline.org/athletics/
Provides: Athletics links,rankings,tournament guides/programs; participation evidence
Coverage: Vermont member athletics
Format/access check: WordPress HTML and linked guides | opened_verified
Use and limitations: Official statewide sport source. https://vpaonline.org/athletic-guides-rules/ shows XC/indoor/outdoor titles but those titles were unlinked in extracted current HTML. Do not invent guide PDFs or claim statewide coach directory. Follow tournament/program links to team lists then schools.
Manifest ID: [SRC-082](#src-082)

#### 2. MileSplit VT team directory
https://vt.milesplit.com/teams
Provides: Team/school name,city,team page URL and platform ID
Coverage: State teams including HS,MS,college,clubs and historical/closed records
Format/access check: Public HTML alphabetical directory; level filter | opened_verified
Use and limitations: Opened state team listing without login. Filter to high school; cross-check current official school inventory and season. Team pages/coach contact presence varies and was not assumed. Do not scrape student rosters, athlete profiles or results. No public documented API verified.
Manifest ID: [SRC-090](#src-090)

#### 3. Vermont Annual Snapshot organization directory
https://schoolsnapshot.vermont.gov/Organization/Directory
Provides: School/organization identity and locality; fields in rendered directory
Coverage: Vermont education organizations
Format/access check: HTML/JS rendered directory | opened_verified
Use and limitations: Public page opens as directory shell but text extraction omits school rows; search indexed rows exist. Browser render needed to verify data route and fields. No JSON endpoint verified; do not confuse login UI at footer with evidence entire directory requires login.
Manifest ID: [SRC-085](#src-085)

### PENNSYLVANIA (PA)
#### 1. PIAA school directory
https://www.piaa.org/schools/directory/default.aspx
Provides: School names, type, district, address, AD/admin contacts, sponsored sports from detail pages
Coverage: PIAA member high schools and junior/middle schools; public/private
Format/access check: Public ASP.NET directory hub/list/detail | opened_verified
Use and limitations: First distinct athletics seed. Hub opened and provides district/alphabetical routes. /schools/directory/list.aspx was search-visible but fetch failed; sample details.aspx?ID=11513 search result shows AD email and sports, not coaches. Do not claim coach emails from PIAA. Preserve grade/type filters.
Manifest ID: [SRC-229](#src-229)

#### 2. Pennsylvania EdNA output files
https://www.edna.pa.gov/Screens/Extracts/wfExtracts.aspx
Provides: Education entity identifiers, names, addresses, administrators, public-school grades and entity relationships
Coverage: Recognized public, private and nonpublic educational entities statewide
Format/access check: Public ASP.NET export forms; Excel outputs | opened_verified
Use and limitations: Second distinct broad system. Public Schools and Private/Nonpublic extract forms both opened without login; binary reports were not generated. Select open schools, relevant categories and high-school grades; do not treat a diocese/LEA as a school.
Manifest ID: [SRC-230](#src-230)

#### 3. Track and Field Coaches Association of Greater Philadelphia handbook
https://www.tfcaofgp.org/coaches-handbook/
Provides: Officer, meet-director and league-representative coach names, selected school affiliations, emails/phones
Coverage: Regional leadership only, not statewide membership
Format/access check: Public HTML | opened_verified
Use and limitations: Direct coach/contact-bearing supplement with 2025–26 schedule. Keep roles as association/league representative, not automatically head coach. Member Schools link exists but returned bot-challenge page. Never mistake approximately 100 participating schools statement for 100 directory contacts.
Manifest ID: [SRC-234](#src-234)

### ILLINOIS (IL)
#### 1. IHSA new public school directory
https://www.ihsa.org/schools/school-directory
Provides: school ID; website; address; phone; county; type; conference; entered sports and class
Coverage: Public and private IHSA schools
Format/access check: JavaScript browser directory | opened_verified
Use and limitations: Live browser sample https://www.ihsa.org/schools/details/0235 verified Heritage school and all four XC/track gender entries. Staff panel failed with 'We couldn't load the staff directory' after retry; do not claim current coach contacts verified. Old /data/school/schools/0235.htm returned 404. No data API verified.
Manifest ID: [SRC-120](#src-120)

#### 2. ISBE nightly public/private school directory
https://www.isbe.net/Pages/Data-Analysis-Directories.aspx
Provides: entity name; administrator; address/contact; grades; RCDTS; NCES ID
Coverage: All public/nonpublic K-12 education entities known to ISBE
Format/access check: HTML landing plus Excel | opened_verified
Use and limitations: Exact href verified in live HTML: https://www.isbe.net/_layouts/Download.aspx?SourceUrl=/Documents/dir_ed_entities.xls . Landing says updated nightly. Use Public Sch and Dist and Non Pub Sch tabs, filter HS grades and school entity type. No coach roles assumed. Download body not verified; landing has generic archived-page footer despite current links.
Manifest ID: [SRC-122](#src-122)

#### 3. ITCCCA officers and coaching awards
https://www.itccca.com/itccca-officers
Provides: coach/officer name; school; association responsibility
Coverage: Current association officers/coordinators, not full membership
Format/access check: HTML tables | opened_verified
Use and limitations: Main https://www.itccca.com/ opened and exposes Coach of the Year and Assistant Coach of the Year award sections. Useful named-coach and assistant-coach enrichment, but confirm current appointment and no public full membership API verified.
Manifest ID: [SRC-123](#src-123)

### KANSAS (KS)
#### 1. Kansas Educational Directory Reports
https://uapps.ksde.gov/directory_rpts/default.aspx
Provides: High-school directories, accredited/nonaccredited private directories, active-building and accredited-organization raw data
Coverage: Kansas public/private education entities; current organizational reports 2026–2027
Format/access check: Public ASP.NET reports; PDF and Excel options | opened_verified
Use and limitations: Best clean seed. Opened report selector explicitly offers Active Building Report (Excel) and accredited/nonaccredited nonpublic lists (PDF). Educator report is separately labeled 2023–24 updated Oct 28, 2024; do not treat it as current coach database. Outputs not generated here.
Manifest ID: [SRC-239](#src-239)

#### 2. KSHSAA school leagues directory
https://kshsaa.org/Public/General/Leagues.cfm
Provides: League names, league-school detail routes
Coverage: KSHSAA public/private members and approved-school categories
Format/access check: Public HTML legacy hub; some routes migrating to React | opened_verified
Use and limitations: Second authoritative system. Opened league index has many statewide leagues. School-search homepage redirects to /react/ and approved-schools to /react/approved-schools, both JS shells in this tool. Follow actual current links and verify rendered data; no API documented.
Manifest ID: [SRC-240](#src-240)

#### 3. Kansas Cross Country and Track and Field Coaches Association
https://www.kcctfca.com/
Provides: Selected clinic speaker coaches and schools, award links, association contacts
Coverage: Statewide association; public data selective
Format/access check: Public Wix HTML | opened_verified
Use and limitations: Third distinct coach-bearing system. Winter clinic page opened with 2026 speaker school/college affiliations. Filter to high-school coaches and verify current staff roles. No complete public membership/email roster found; association homepage/contacts are not all coaches.
Manifest ID: [SRC-241](#src-241)

### SHARED NATIONAL SCHOOL INVENTORY
Use these in addition to the three state-specific picks; they do not contain coach assignments.

NCES CCD 2024–25 school directory CSV/SAS ZIP
https://nces.ed.gov/ccd/Data/zip/ccd_sch_029_2425_w_1a_073025.zip
Use any of G_9_OFFERED,G_10_OFFERED,G_11_OFFERED,G_12_OFFERED == Yes. Active school-year statuses are 1,3,4,5,8; exclude 2 Closed,6 Inactive,7 Future. Updated status can supersede school-year status for current outreach; preserve both. Keep combined K–12 and other schools if they offer HS grades. File includes territories; limit to requested states. NCES school ID must be a string with leading zeros.

NCES PSS 2023–24 public-use CSV ZIP
https://nces.ed.gov/surveys/pss/zip/pss2324_pu_csv.zip
Exact HS filter: any of P265,P275,P285,P295 == 1. Values 1=Yes,2=No. HIGR2024 is recoded: 14=9th,15=10th,16=11th,17=12th; never use numeric HIGR>=9 as if it were a grade number. Preserve combined schools. No coach fields or school website field found in this CSV. School search About Data currently says 22,502 responding schools; downloaded CSV/codebook both report 22,510, so do not interchange counts.

NFHS state association directory
https://nfhs.org/about/state-association-directory
Association discovery, not school coach inventory. Page has member and affiliate tabs; private associations may require affiliate coverage. Do not claim NFHS self-service spreadsheet is publicly accessible: https://utilities.nfhs.org/schooldirectory/selfservice requires sign-in.


## Registry metadata

- **title:** High-school track and cross-country coach and school source registry
- **research_date:** 2026-10-04
- **requested_states:**
  - TX
  - CA
  - NY
  - OH
  - NJ
  - FL
  - VA
  - MI
  - MO
  - MA
  - IN
  - WA
  - IA
  - AZ
  - OK
  - LA
  - OR
  - CT
  - UT
  - NV
  - SD
  - VT
  - PA
  - IL
  - KS
- **shortlist_entries:** 75
- **resource_entries:** 278
- **unique_primary_urls:** 275
- **scope:** Three prioritized source systems per requested state, plus additional regional, national, download, profile, API and qualified inaccessible leads. Public/private high-school school inventory and published professional coach information only.
- **verification_note:** opened_verified means the source page or UI was inspected. It does not mean every child page, export, API, current coach assignment or statewide coverage was verified. Read evidence, coverage and implementation_notes. Other verification statuses identify weaker evidence, blocked access, documentation-only checks and explicit restrictions.
- **completion_note:** Source research completed, not a populated coach database or an exhaustive guarantee that every existing source has been discovered.

## Complete source registry

All 278 records are retained below in original registry order. Field names, values, source IDs, evidence links, observed routes, coverage caveats, and access restrictions are preserved. Explicit null and empty-list values are shown.

### SRC-001

UIL sport alignments and school-code inventory

- **state:** TX
- **name:** UIL sport alignments and school-code inventory
- **url:** https://www.uiltexas.org/alignments
- **category:** state_athletic_association
- **sports_scope:** High-school XC and outdoor track, boys/girls
- **school_or_coach_fields:** School name; conference; region/district; track school codes; organizing-chair contacts where posted
- **coverage:** UIL members, predominantly public schools and participating charters; not all Texas private schools
- **access_format:** HTML indexes with linked PDFs
- **verification_status:** opened_verified
- **evidence_url:** https://www.uiltexas.org/alignments
- **implementation_notes:** Use sport-specific XC and spring alignments, not football districts. Current index has 2026–27 XC and spring files. Spring includes golf/tennis as well as track, so confirm actual sport sponsorship. Alphabetical all-school list and track school codes are useful alias crosswalks. Coach names/emails are not a comprehensive part of these files.
- **priority:** 1
- **top_three_rank:** 1
- **verification_date:** 2026-10-04
- **observed_routes:**
  - https://www.uiltexas.org/alignments/category/align-cross-country
  - https://www.uiltexas.org/alignments/category/align-spring-athletics
  - https://www.uiltexas.org/files/alignments/Alpha_26-28.pdf
  - https://www.uiltexas.org/track-field/school-codes
  - https://www.uiltexas.org/files/alignments/TF_School_Codes_2026.pdf
- **checked_date:** 2026-10-04
- **applicable_states:**
  - TX
- **shortlist_rank:** 1
- **source_id:** SRC-001

### SRC-002

TEA AskTED school, district and personnel downloads

- **state:** TX
- **name:** TEA AskTED school, district and personnel downloads
- **url:** https://tealprod.tea.state.tx.us/tea.askted.web/Forms/Home.aspx
- **category:** education_department_directory
- **sports_scope:** All schools; join to XC/track participation
- **school_or_coach_fields:** School/district identity; addresses; contact information; district/campus personnel reports
- **coverage:** Texas public schools, districts and ESCs; includes public charters
- **access_format:** ASP.NET HTML forms; daily comma-delimited downloads; report exports
- **verification_status:** opened_verified
- **evidence_url:** https://tealprod.tea.state.tx.us/tea.askted.web/Forms/Home.aspx
- **implementation_notes:** Highest-value public-school entity seed. Homepage says daily updates and offers school/district files with site addresses. Official help documents CSV exports. Follow current download links, preserving cookies/form state if needed; no public REST API verified. Personnel are administrative, not a guaranteed track/XC coach roster. Binary downloads failed in research fetch, but landing and official export documentation are public.
- **priority:** 1
- **top_three_rank:** 2
- **verification_date:** 2026-10-04
- **observed_routes:**
  - https://tealprod.tea.state.tx.us/Tea.AskTed.Web/help/Downloading_a_Data_File.htm
- **checked_date:** 2026-10-04
- **applicable_states:**
  - TX
- **shortlist_rank:** 2
- **source_id:** SRC-002

### SRC-003

TAPPS school directory and alignment embeds

- **state:** TX
- **name:** TAPPS school directory and alignment embeds
- **url:** https://www.tapps.biz/school-directory-2/
- **category:** private_parochial_athletic_association
- **sports_scope:** TAPPS XC and track; all-school seed
- **school_or_coach_fields:** Published embed field list: schoolName,address,mascot,classification,website,email,telephone
- **coverage:** Texas private/parochial TAPPS members
- **access_format:** WordPress HTML wrapper; TMS JavaScript iframe
- **verification_status:** opened_verified
- **evidence_url:** https://www.tapps.biz/school-directory-2/
- **implementation_notes:** Wrapper and embedded URLs opened. Embedded rows were not rendered by text fetch and shell fetch returned 403, so verify UI row coverage before implementation. This is a public UI embed, not a documented API. Follow pagination rather than assuming limit=12 returns all schools. Coach-specific fields were not verified. Use school websites to reach current coaches.
- **priority:** 1
- **top_three_rank:** 3
- **verification_date:** 2026-10-04
- **observed_routes:**
  - https://tms.tapps.biz/embed-code/SchoolInformation/TVE9PQ==/schoolName,address,mascot,classification,website,email,telephone/1?columns=3&limit=12
  - https://www.tapps.biz/home/tapps-26-28-alignment/
  - https://tms.tapps.biz/embed-code/alignments/TVE9PQ==?academicYear=3
- **checked_date:** 2026-10-04
- **applicable_states:**
  - TX
- **shortlist_rank:** 3
- **source_id:** SRC-003

### SRC-004

AskTED Texas Open Data Portal dataset

- **state:** TX
- **name:** AskTED Texas Open Data Portal dataset
- **url:** https://data.texas.gov/dataset/AskTED-Data-May-12-2026/hzek-udky
- **category:** open_data_dataset
- **sports_scope:** All public schools
- **school_or_coach_fields:** School/contact/location records; schema must be read before coding
- **coverage:** Snapshot of AskTED, not necessarily as fresh as daily TEA files
- **access_format:** Socrata dataset landing and developer portal
- **verification_status:** opened_verified
- **evidence_url:** https://data.texas.gov/dataset/AskTED-Data-May-12-2026/hzek-udky
- **implementation_notes:** Dataset landing resolved to May 12, 2026 snapshot. Developer portal opened but schema did not render. Do not represent the conventional /resource/hzek-udky.json URL as tested: sample GET was inaccessible. Use dataset-provided API/export documentation when available and validate current schema/row count.
- **priority:** 2
- **top_three_rank:** null
- **verification_date:** 2026-10-04
- **observed_routes:**
  - https://dev.socrata.com/foundry/data.texas.gov/hzek-udky
- **checked_date:** 2026-10-04
- **applicable_states:**
  - TX
- **shortlist_rank:** null
- **source_id:** SRC-004

### SRC-005

TEPSAC accredited private-school seed

- **state:** TX
- **name:** TEPSAC accredited private-school seed
- **url:** https://www.tepsac.org/
- **category:** private_school_accreditation_directory
- **sports_scope:** All private school sports after school-site enrichment
- **school_or_coach_fields:** Accredited nonpublic school identity and accreditation; actual current export fields unverified
- **coverage:** Accredited Texas nonpublic elementary/secondary schools, not every private school
- **access_format:** JavaScript landing; older HTML information page
- **verification_status:** opened_verified
- **evidence_url:** https://tealprod.tea.state.tx.us/tea.askted.web/Forms/Home.aspx
- **implementation_notes:** Linked from current AskTED as accredited non-public school resource. Root rendered no text; information page opened. Current directory/export needs UI verification, so keep as secondary seed and filter grades 9–12. Do not substitute the decades-old TEA acnps.pdf.
- **priority:** 2
- **top_three_rank:** null
- **verification_date:** 2026-10-04
- **observed_routes:**
  - https://www.tepsac.org/home/home.html
- **checked_date:** 2026-10-04
- **applicable_states:**
  - TX
- **shortlist_rank:** null
- **source_id:** SRC-005

### SRC-006

Texas Christian Athletic Fellowship school directory

- **state:** TX
- **name:** Texas Christian Athletic Fellowship school directory
- **url:** https://www.tcafellowship.com/our-schools
- **category:** independent_christian_athletic_association
- **sports_scope:** XC and track confirmed by sport pages
- **school_or_coach_fields:** School; address; phone; administrator; athletic director; public AD email
- **coverage:** TCAF independent Christian schools
- **access_format:** HTML directory
- **verification_status:** blocked
- **evidence_url:** https://www.tcafellowship.com/our-schools
- **implementation_notes:** Search indexed full school/AD contact entries, but direct opens failed twice. Mark rows search-only until a successful fetch. Track page opened; XC page indexed. Directory is a valuable missing-private-school seed, not sport-specific coach roster.
- **priority:** 2
- **top_three_rank:** null
- **verification_date:** 2026-10-04
- **observed_routes:**
  - https://www.tcafellowship.com/track-program
  - https://www.tcafellowship.com/cross-country-program
- **checked_date:** 2026-10-04
- **applicable_states:**
  - TX
- **shortlist_rank:** null
- **source_id:** SRC-006

### SRC-007

Southwest Preparatory Conference

- **state:** TX
- **name:** Southwest Preparatory Conference
- **url:** https://spcsports.org/sports/2019/7/23/governance.aspx
- **category:** independent_school_athletic_conference
- **sports_scope:** XC and track; multisport
- **school_or_coach_fields:** Conference schools; school links; selected AD/leadership names; annual manual
- **coverage:** Independent schools in Texas plus out-of-state members; filter state
- **access_format:** SIDEARM HTML; linked handbook PDF
- **verification_status:** opened_verified
- **evidence_url:** https://spcsports.org/sports/2019/7/23/governance.aspx
- **implementation_notes:** Use Conference Members navigation and current annual manual to enumerate schools, then their athletics staff pages. Governance lists 2026–27 handbook and operations committee. Selected AD contacts are not the full coach directory. Exclude Oklahoma members when building Texas-only inventory.
- **priority:** 2
- **top_three_rank:** null
- **verification_date:** 2026-10-04
- **observed_routes:**
  - https://spcsports.org/
- **checked_date:** 2026-10-04
- **applicable_states:**
  - TX
- **shortlist_rank:** null
- **source_id:** SRC-007

### SRC-008

Texas Charter School Academic & Athletic League

- **state:** TX
- **name:** Texas Charter School Academic & Athletic League
- **url:** https://texascharter.org/
- **category:** charter_independent_athletic_league
- **sports_scope:** XC verified; track not visible in current sports navigation
- **school_or_coach_fields:** School/team names; varsity level; region; schedules and standings
- **coverage:** Charter/independent participating schools across Central, East, North, South and West Texas
- **access_format:** HTML sport/region pages and team links
- **verification_status:** opened_verified
- **evidence_url:** https://texascharter.org/
- **implementation_notes:** Important UIL coverage-gap seed. Follow current Cross Country navigation and each region. Keep varsity/JV HS only; page also includes elementary and middle school teams. Do not infer track coverage from XC. School entities can overlap UIL and require deduplication.
- **priority:** 2
- **top_three_rank:** null
- **verification_date:** 2026-10-04
- **observed_routes:**
  - https://texascharter.org/regions/central/
- **checked_date:** 2026-10-04
- **applicable_states:**
  - TX
- **shortlist_rank:** null
- **source_id:** SRC-008

### SRC-009

Cross Country Coaches Association of Texas

- **state:** TX
- **name:** Cross Country Coaches Association of Texas
- **url:** https://www.cccat.org/
- **category:** sport_coaches_association
- **sports_scope:** High-school cross country
- **school_or_coach_fields:** Coach-of-year name/school; board name/school/contact; clinic presenters
- **coverage:** Selected Texas XC coaches, not all member coaches
- **access_format:** Weebly HTML; annual PDFs
- **verification_status:** opened_verified
- **evidence_url:** https://www.cccat.org/
- **implementation_notes:** Public coach-of-year and board pages opened. Use current awards, board and clinic programs as sparse name-to-school enrichment. Membership page does not establish a public all-member directory. Exclude student awards/scholarships from extraction.
- **priority:** 2
- **top_three_rank:** null
- **verification_date:** 2026-10-04
- **observed_routes:**
  - https://www.cccat.org/coach-of-the-year.html
  - https://www.cccat.org/cccat-board.html
  - https://www.cccat.org/uploads/1/2/3/5/123534441/cccat_2026_summer_clinic_schedule.pdf
- **checked_date:** 2026-10-04
- **applicable_states:**
  - TX
- **shortlist_rank:** null
- **source_id:** SRC-009

### SRC-010

Texas Track & Field Coaches Association

- **state:** TX
- **name:** Texas Track & Field Coaches Association
- **url:** https://www.ttfca.org/coaches-of-the-year
- **category:** sport_coaches_association
- **sports_scope:** Track and field
- **school_or_coach_fields:** Award-winning coaches; school/team affiliation; year
- **coverage:** Selected high-school coaches plus other levels; strict HS filtering needed
- **access_format:** Wix HTML; PDFs
- **verification_status:** opened_verified
- **evidence_url:** https://www.ttfca.org/coaches-of-the-year
- **implementation_notes:** Awards page and association home opened. Separate high-school from college coaches and retired hall-of-famers. Useful corroboration, not exhaustive membership/contact list. Preserve award year and verify current employment on school site.
- **priority:** 2
- **top_three_rank:** null
- **verification_date:** 2026-10-04
- **observed_routes:**
  - https://www.ttfca.org/
- **checked_date:** 2026-10-04
- **applicable_states:**
  - TX
- **shortlist_rank:** null
- **source_id:** SRC-010

### SRC-011

Texas Girls Coaches Association

- **state:** TX
- **name:** Texas Girls Coaches Association
- **url:** https://www.austintgca.com/
- **category:** multisport_coaches_association
- **sports_scope:** Girls XC and track
- **school_or_coach_fields:** Sport committees; all-star coaches; awards and school affiliation where posted
- **coverage:** TGCA members/selected honorees only
- **access_format:** HTML; PDFs; member-site login
- **verification_status:** blocked
- **evidence_url:** https://www.austintgca.com/
- **implementation_notes:** Homepage search result verifies XC/track navigation and membership portal, but direct open failed. Sparse public all-star coach/committee materials may help; do not assume membership profiles or emails are publicly exportable.
- **priority:** 3
- **top_three_rank:** null
- **verification_date:** 2026-10-04
- **observed_routes:**
  - (empty list)
- **checked_date:** 2026-10-04
- **applicable_states:**
  - TX
- **shortlist_rank:** null
- **source_id:** SRC-011

### SRC-012

Texas High School Coaches Association

- **state:** TX
- **name:** Texas High School Coaches Association
- **url:** https://www.thsca.com/
- **category:** multisport_coaches_association
- **sports_scope:** All HS sports including XC/track
- **school_or_coach_fields:** Public awards/committees and affiliation leads; no comprehensive directory verified
- **coverage:** Statewide coaches association
- **access_format:** HTML; PDF; member services
- **verification_status:** opened_verified
- **evidence_url:** https://www.thsca.com/
- **implementation_notes:** Homepage opened; no public all-coach directory found in page. Useful association lead and recognition source only. Do not confuse its suppliers Buyers Guide with coaches. Clell Wade is separately listed as a gated directory option.
- **priority:** 3
- **top_three_rank:** null
- **verification_date:** 2026-10-04
- **observed_routes:**
  - (empty list)
- **checked_date:** 2026-10-04
- **applicable_states:**
  - TX
- **shortlist_rank:** null
- **source_id:** SRC-012

### SRC-013

MileSplit Texas team directory

- **state:** TX
- **name:** MileSplit Texas team directory
- **url:** https://tx.milesplit.com/teams
- **category:** sport_team_database
- **sports_scope:** XC and track
- **school_or_coach_fields:** Team ID; team/school name; city; team profile URL
- **coverage:** HS, middle school, college, club and inactive/unattached entries mixed
- **access_format:** Public HTML team index and profiles
- **verification_status:** opened_verified
- **evidence_url:** https://tx.milesplit.com/teams
- **implementation_notes:** Large directly parseable index; preserve numeric team IDs from hrefs. Join school level/status against TEA/association seeds before keeping records. Sample Abilene profile opened and had no coach field in fetched HTML, so coach/email coverage is not guaranteed. Never collect student rosters or results.
- **priority:** 2
- **top_three_rank:** null
- **verification_date:** 2026-10-04
- **observed_routes:**
  - https://tx.milesplit.com/teams/61776-abilene
- **checked_date:** 2026-10-04
- **applicable_states:**
  - TX
- **shortlist_rank:** null
- **source_id:** SRC-013

### SRC-014

Athletic.net Texas XC division

- **state:** TX
- **name:** Athletic.net Texas XC division
- **url:** https://www.athletic.net/cross-country/division/75199
- **category:** sport_team_database
- **sports_scope:** XC; separate track UI
- **school_or_coach_fields:** Team/school identities and team-page links; actual coach fields unverified
- **coverage:** Texas teams represented in Athletic.net
- **access_format:** JavaScript UI
- **verification_status:** opened_verified
- **evidence_url:** https://www.athletic.net/CrossCountry/Texas/
- **implementation_notes:** Legacy Texas XC URL redirected here. Text fetch returned application shell only; current season/team enumeration needs rendered UI. No public API documented or tested. Avoid athlete/roster extraction.
- **priority:** 3
- **top_three_rank:** null
- **verification_date:** 2026-10-04
- **observed_routes:**
  - (empty list)
- **checked_date:** 2026-10-04
- **applicable_states:**
  - TX
- **shortlist_rank:** null
- **source_id:** SRC-014

### SRC-015

Clell Wade Texas Coaches Directory

- **state:** TX
- **name:** Clell Wade Texas Coaches Directory
- **url:** https://www.coachesdirectory.com/coaches/directory-access/
- **category:** licensed_coach_directory
- **sports_scope:** Interscholastic multisport; confirm XC/track fields in license
- **school_or_coach_fields:** Coach/school contact directory advertised; individual fields not inspected
- **coverage:** 2026–27 Texas book; online state HS and junior-high directories
- **access_format:** Account-gated web access; printed book
- **verification_status:** login_required
- **evidence_url:** https://www.coachesdirectory.com/coaches/directory-access/
- **implementation_notes:** Public access page opened and explicitly requires sign-up for free access limited to interscholastic administrators/coaches. No signup attempted. Commercial users need appropriate licensed access and permitted export rights; not a public scrape target.
- **priority:** 3
- **top_three_rank:** null
- **verification_date:** 2026-10-04
- **observed_routes:**
  - (empty list)
- **checked_date:** 2026-10-04
- **applicable_states:**
  - TX
- **shortlist_rank:** null
- **source_id:** SRC-015

### SRC-016

Community ISD coaches directory example

- **state:** TX
- **name:** Community ISD coaches directory example
- **url:** https://www.communityisd.org/athletics/coaches-directory
- **category:** official_school_staff_fallback
- **sports_scope:** XC and track plus other sports
- **school_or_coach_fields:** Name; exact coaching title; public school email
- **coverage:** One Texas district, not statewide
- **access_format:** Finalsite HTML staff cards
- **verification_status:** opened_verified
- **evidence_url:** https://www.communityisd.org/athletics/coaches-directory
- **implementation_notes:** Verified explicit Head Coach - Track & Field and Head Coach - Cross Country cards with school emails. Useful parser fixture for official school-site fallback after AskTED/UIL seed discovery. Match title, school/campus and season; do not infer gender if absent.
- **priority:** 2
- **top_three_rank:** null
- **verification_date:** 2026-10-04
- **observed_routes:**
  - (empty list)
- **checked_date:** 2026-10-04
- **applicable_states:**
  - TX
- **shortlist_rank:** null
- **source_id:** SRC-016

### SRC-017

CIF / Home Campus public school and coaches directory

- **state:** CA
- **name:** CIF / Home Campus public school and coaches directory
- **url:** https://www.cifsshome.org/widget/school/directory
- **category:** state_section_school_coach_directory
- **sports_scope:** Boys/girls XC and track plus other HS sports
- **school_or_coach_fields:** School identity; school information; athletic faculty; sport; head-coach name; public email; vacancy markers
- **coverage:** Section selector lists all 10 CIF sections; individual fields verified for Southern Section Arcadia only
- **access_format:** Public JavaScript school selector and Coaches and Sports tab
- **verification_status:** opened_verified
- **evidence_url:** https://www.cifsshome.org/widget/school/directory?school_id=19&section_id=1
- **implementation_notes:** Highest-priority direct coach source. Cloud browser clicked Arcadia then Coaches and Sports and verified boys/girls XC coach and email, and boys/girls track coach and email. Discover school IDs from rendered school buttons; do not enumerate guessed IDs. Display includes BYE, association placeholders and some middle schools; filter with CDE. Other section presence verified in selector, but their rows/coach completeness not individually audited. No bulk API or hidden endpoint claimed.
- **priority:** 1
- **top_three_rank:** 1
- **verification_date:** 2026-10-04
- **observed_routes:**
  - https://www.cifsshome.org/widget/school/directory?school_id=19&section_id=1
  - https://www.cifnshome.org/widget/school/directory
  - https://www.cifsdshome.org/widget/school/directory
- **checked_date:** 2026-10-04
- **applicable_states:**
  - CA
- **shortlist_rank:** 1
- **source_id:** SRC-017

### SRC-018

California Department of Education school directory and exports

- **state:** CA
- **name:** California Department of Education school directory and exports
- **url:** https://sd.cde.ca.gov/schooldirectory/
- **category:** education_department_directory
- **sports_scope:** All schools; join sports participation separately
- **school_or_coach_fields:** CDS code; school/district; county; type; sector; charter/status; grades; address; administrator and contact fields
- **coverage:** California public/private/nonpublic schools, districts and county offices
- **access_format:** Searchable HTML; Excel/text exports documented
- **verification_status:** opened_verified
- **evidence_url:** https://www.cde.ca.gov/ds/si/ps/
- **implementation_notes:** Use active status and grades including 9–12; retain 14-digit CDS as string. Directory includes K–12 and continuation/alternative high schools, so do not filter name alone. Private-school data page explicitly documents Excel/text export with contacts. Directory is not a coach list. Public bulk-file landing hit CDE WAF during research; prefer verified searchable export route rather than inventing replacement URLs.
- **priority:** 1
- **top_three_rank:** 2
- **verification_date:** 2026-10-04
- **observed_routes:**
  - https://www.cde.ca.gov/schooldirectory/
  - https://www.cde.ca.gov/ds/si/ps/
  - https://www.cde.ca.gov/ds/si/ds/pubschls.asp
- **checked_date:** 2026-10-04
- **applicable_states:**
  - CA
- **shortlist_rank:** 2
- **source_id:** SRC-018

### SRC-019

MileSplit California team directory

- **state:** CA
- **name:** MileSplit California team directory
- **url:** https://ca.milesplit.com/teams
- **category:** sport_team_database
- **sports_scope:** XC and track
- **school_or_coach_fields:** Team ID; school/team; city; section abbreviation in many team names; profile URL
- **coverage:** Statewide HS plus mixed school levels/clubs/inactive entries
- **access_format:** Public HTML team index
- **verification_status:** opened_verified
- **evidence_url:** https://ca.milesplit.com/teams
- **implementation_notes:** Strong independent sport-program/alias crosswalk. Example suffixes SS, CC, NC, SJ, NS, CS encode sections but should be validated. Use CDE and CIF to exclude non-HS/closed schools. Coach coverage not demonstrated, no API verified. Do not traverse athlete rosters.
- **priority:** 1
- **top_three_rank:** 3
- **verification_date:** 2026-10-04
- **observed_routes:**
  - (empty list)
- **checked_date:** 2026-10-04
- **applicable_states:**
  - CA
- **shortlist_rank:** 3
- **source_id:** SRC-019

### SRC-020

CIF official ten-section index

- **state:** CA
- **name:** CIF official ten-section index
- **url:** https://cifss.org/cif-state-sections/
- **category:** association_coverage_map
- **sports_scope:** XC and track statewide
- **school_or_coach_fields:** Official section names and URLs; section office contacts
- **coverage:** All 10 CIF sections
- **access_format:** HTML table
- **verification_status:** opened_verified
- **evidence_url:** https://cifss.org/cif-state-sections/
- **implementation_notes:** Use as authoritative discovery map for Southern, San Diego, Los Angeles City, Central, Central Coast, North Coast, Northern, Sac-Joaquin, Oakland and San Francisco. School counts on overview may lag, so never use as current row-count assertion. School/coaching records belong to sections and public Home Campus selector.
- **priority:** 2
- **top_three_rank:** null
- **verification_date:** 2026-10-04
- **observed_routes:**
  - (empty list)
- **checked_date:** 2026-10-04
- **applicable_states:**
  - CA
- **shortlist_rank:** null
- **source_id:** SRC-020

### SRC-021

CIF San Francisco high-school members

- **state:** CA
- **name:** CIF San Francisco high-school members
- **url:** https://www.cifsf.org/schools/high-schools/
- **category:** regional_school_directory
- **sports_scope:** XC and track offered by section
- **school_or_coach_fields:** High-school members; school and athletics links; AD fields in school information UI
- **coverage:** San Francisco Section; do not confuse separate middle-school list
- **access_format:** Home Campus powered HTML/JavaScript
- **verification_status:** opened_verified
- **evidence_url:** https://www.cifsf.org/schools/high-schools/
- **implementation_notes:** Direct high-school page opened. Use member-school cards and sport pages, and exclude adjacent middle-school navigation. Homepage template exposes principal/AD/address/website fields; populated per-school details not audited.
- **priority:** 2
- **top_three_rank:** null
- **verification_date:** 2026-10-04
- **observed_routes:**
  - (empty list)
- **checked_date:** 2026-10-04
- **applicable_states:**
  - CA
- **shortlist_rank:** null
- **source_id:** SRC-021

### SRC-022

CIF Southern Section directory wrapper

- **state:** CA
- **name:** CIF Southern Section directory wrapper
- **url:** https://cifss.org/directory/
- **category:** regional_school_league_directory
- **sports_scope:** XC and track
- **school_or_coach_fields:** School directory and league directory iframe routes
- **coverage:** Southern Section
- **access_format:** HTML with iframe widgets
- **verification_status:** search_only
- **evidence_url:** https://cifss.org/directory/
- **implementation_notes:** Search result explicitly exposes separate school and league directory sections. School iframe independently opened and coach data verified; league iframe not separately tested. Retain as discovery wrapper rather than a separate source system from Home Campus.
- **priority:** 2
- **top_three_rank:** null
- **verification_date:** 2026-10-04
- **observed_routes:**
  - (empty list)
- **checked_date:** 2026-10-04
- **applicable_states:**
  - CA
- **shortlist_rank:** null
- **source_id:** SRC-022

### SRC-023

CIF Northern Section public Home Campus directory

- **state:** CA
- **name:** CIF Northern Section public Home Campus directory
- **url:** https://www.cifnshome.org/widget/school/directory
- **category:** regional_school_directory
- **sports_scope:** XC and track
- **school_or_coach_fields:** School selector names and school IDs; coach-tab population untested for this section
- **coverage:** Northern Section public/private school seeds plus placeholders
- **access_format:** JavaScript widget; search-indexed school buttons
- **verification_status:** opened_verified
- **evidence_url:** https://www.cifnshome.org/widget/school/directory
- **implementation_notes:** Opened widget shell and search-indexed school list. Same public UI family as verified Southern directory, but do not assume equal coach completeness. Filter BYE/Non CIFNS School and verify school level.
- **priority:** 2
- **top_three_rank:** null
- **verification_date:** 2026-10-04
- **observed_routes:**
  - (empty list)
- **checked_date:** 2026-10-04
- **applicable_states:**
  - CA
- **shortlist_rank:** null
- **source_id:** SRC-023

### SRC-024

CIF San Diego Section public Home Campus directory

- **state:** CA
- **name:** CIF San Diego Section public Home Campus directory
- **url:** https://www.cifsdshome.org/widget/school/directory
- **category:** regional_school_directory
- **sports_scope:** XC and track
- **school_or_coach_fields:** School selector names and IDs; coach-tab population untested for this section
- **coverage:** San Diego/Imperial-area section members including private schools
- **access_format:** JavaScript widget; search-indexed school buttons
- **verification_status:** opened_verified
- **evidence_url:** https://www.cifsdshome.org/widget/school/directory
- **implementation_notes:** Opened widget shell and indexed member names. School/coach tab should be validated against one current school before bulk adapter use. Remove Non CIFSDS School placeholders and any non-HS entries.
- **priority:** 2
- **top_three_rank:** null
- **verification_date:** 2026-10-04
- **observed_routes:**
  - (empty list)
- **checked_date:** 2026-10-04
- **applicable_states:**
  - CA
- **shortlist_rank:** null
- **source_id:** SRC-024

### SRC-025

CIF North Coast Section official source

- **state:** CA
- **name:** CIF North Coast Section official source
- **url:** https://www.cifncs.org/
- **category:** regional_association
- **sports_scope:** XC and track
- **school_or_coach_fields:** Section membership, sport alignments, school links; detailed fields not verified
- **coverage:** North Coast Section
- **access_format:** Section website; shared Home Campus selector alternative
- **verification_status:** blocked
- **evidence_url:** https://cifss.org/cif-state-sections/
- **implementation_notes:** Official identity/URL verified through CIF ten-section index. Direct root fetch failed (403 for NCS/CCS/Central; inaccessible for Oakland). Section name is present in successfully inspected Home Campus public selector. Preserve as coverage checklist; no unverified endpoint or school count asserted.
- **priority:** 3
- **top_three_rank:** null
- **verification_date:** 2026-10-04
- **observed_routes:**
  - (empty list)
- **checked_date:** 2026-10-04
- **applicable_states:**
  - CA
- **shortlist_rank:** null
- **source_id:** SRC-025

### SRC-026

CIF Central Coast Section official source

- **state:** CA
- **name:** CIF Central Coast Section official source
- **url:** https://www.cifccs.org/
- **category:** regional_association
- **sports_scope:** XC and track
- **school_or_coach_fields:** Section membership, sport alignments, school links; detailed fields not verified
- **coverage:** Central Coast Section
- **access_format:** Section website; shared Home Campus selector alternative
- **verification_status:** blocked
- **evidence_url:** https://cifss.org/cif-state-sections/
- **implementation_notes:** Official identity/URL verified through CIF ten-section index. Direct root fetch failed (403 for NCS/CCS/Central; inaccessible for Oakland). Section name is present in successfully inspected Home Campus public selector. Preserve as coverage checklist; no unverified endpoint or school count asserted.
- **priority:** 3
- **top_three_rank:** null
- **verification_date:** 2026-10-04
- **observed_routes:**
  - (empty list)
- **checked_date:** 2026-10-04
- **applicable_states:**
  - CA
- **shortlist_rank:** null
- **source_id:** SRC-026

### SRC-027

CIF Central Section official source

- **state:** CA
- **name:** CIF Central Section official source
- **url:** https://www.cifcs.org/
- **category:** regional_association
- **sports_scope:** XC and track
- **school_or_coach_fields:** Section membership, sport alignments, school links; detailed fields not verified
- **coverage:** Central Section
- **access_format:** Section website; shared Home Campus selector alternative
- **verification_status:** blocked
- **evidence_url:** https://cifss.org/cif-state-sections/
- **implementation_notes:** Official identity/URL verified through CIF ten-section index. Direct root fetch failed (403 for NCS/CCS/Central; inaccessible for Oakland). Section name is present in successfully inspected Home Campus public selector. Preserve as coverage checklist; no unverified endpoint or school count asserted.
- **priority:** 3
- **top_three_rank:** null
- **verification_date:** 2026-10-04
- **observed_routes:**
  - (empty list)
- **checked_date:** 2026-10-04
- **applicable_states:**
  - CA
- **shortlist_rank:** null
- **source_id:** SRC-027

### SRC-028

CIF Oakland Section official source

- **state:** CA
- **name:** CIF Oakland Section official source
- **url:** https://cifoakland.org/
- **category:** regional_association
- **sports_scope:** XC and track
- **school_or_coach_fields:** Section membership, sport alignments, school links; detailed fields not verified
- **coverage:** Oakland Section
- **access_format:** Section website; shared Home Campus selector alternative
- **verification_status:** blocked
- **evidence_url:** https://cifss.org/cif-state-sections/
- **implementation_notes:** Official identity/URL verified through CIF ten-section index. Direct root fetch failed (403 for NCS/CCS/Central; inaccessible for Oakland). Section name is present in successfully inspected Home Campus public selector. Preserve as coverage checklist; no unverified endpoint or school count asserted.
- **priority:** 3
- **top_three_rank:** null
- **verification_date:** 2026-10-04
- **observed_routes:**
  - (empty list)
- **checked_date:** 2026-10-04
- **applicable_states:**
  - CA
- **shortlist_rank:** null
- **source_id:** SRC-028

### SRC-029

CIF Los Angeles City Section

- **state:** CA
- **name:** CIF Los Angeles City Section
- **url:** https://www.cif-la.org/
- **category:** regional_association
- **sports_scope:** XC and track
- **school_or_coach_fields:** School/program discovery; CIF-LA Home link
- **coverage:** Los Angeles City Section
- **access_format:** Edlio HTML; Home Campus link
- **verification_status:** opened_verified
- **evidence_url:** https://www.cif-la.org/
- **implementation_notes:** Official homepage opened and links to www.cif-lahome.org. Public shared Home Campus selector includes Los Angeles City Section. Detailed standalone directory path and coach completeness not audited; use observed navigation, not guessed URL paths.
- **priority:** 2
- **top_three_rank:** null
- **verification_date:** 2026-10-04
- **observed_routes:**
  - (empty list)
- **checked_date:** 2026-10-04
- **applicable_states:**
  - CA
- **shortlist_rank:** null
- **source_id:** SRC-029

### SRC-030

CIF Sac-Joaquin member schools

- **state:** CA
- **name:** CIF Sac-Joaquin member schools
- **url:** https://www.cifsjs.org/schools/
- **category:** regional_school_directory
- **sports_scope:** XC and track
- **school_or_coach_fields:** Search-indexed school names/city; membership/level needs validation
- **coverage:** Sac-Joaquin intended, but indexed content includes statewide elementary schools and colleges
- **access_format:** Dynamic directory
- **verification_status:** blocked
- **evidence_url:** https://www.cifsjs.org/schools/
- **implementation_notes:** Direct open failed. Search index unexpectedly contains statewide elementary schools, universities and duplicate school names. Never treat every indexed row as a current SJS HS member. Prefer official league alignments or the SJS selection in public Home Campus, then crosswalk CDE.
- **priority:** 3
- **top_three_rank:** null
- **verification_date:** 2026-10-04
- **observed_routes:**
  - (empty list)
- **checked_date:** 2026-10-04
- **applicable_states:**
  - CA
- **shortlist_rank:** null
- **source_id:** SRC-030

### SRC-031

Lynbrook / PrepCalTrack SCVAL coach directory

- **state:** CA
- **name:** Lynbrook / PrepCalTrack SCVAL coach directory
- **url:** https://lynbrooksports.prepcaltrack.com/ATHLETICS/TRACK/2024/coaches.htm
- **category:** regional_sport_coach_directory
- **sports_scope:** SCVAL track; current seasonal pages also cover XC
- **school_or_coach_fields:** School; coach; work phone; published email; additional personal-number columns that should be excluded
- **coverage:** Santa Clara Valley Athletic League; 2024 directory explicitly dated
- **access_format:** Simple preformatted HTML; current index HTML/PDF
- **verification_status:** opened_verified
- **evidence_url:** https://lynbrooksports.prepcaltrack.com/ATHLETICS/TRACK/2024/coaches.htm
- **implementation_notes:** Real multi-school coaching list, dated January 23, 2024, therefore historical lead-only until current school-site confirmation. Ignore home/cell fields. 2026 track index opened and has current Lynbrook head/distance coach and links to league meeting minutes and school websites. Follow current seasonal navigation instead of assuming year-substituted directory URL exists.
- **priority:** 2
- **top_three_rank:** null
- **verification_date:** 2026-10-04
- **observed_routes:**
  - https://lynbrooksports.prepcaltrack.com/ATHLETICS/TRACK/2026/2026.htm
- **checked_date:** 2026-10-04
- **applicable_states:**
  - CA
- **shortlist_rank:** null
- **source_id:** SRC-031

### SRC-032

Pacific Association USATF legacy coaches contacts

- **state:** CA
- **name:** Pacific Association USATF legacy coaches contacts
- **url:** https://www.pausatf.org/data/Coaches.html
- **category:** legacy_sport_coach_directory
- **sports_scope:** XC and track, mixed high-school/college/club
- **school_or_coach_fields:** Coach; school/club; published email; region
- **coverage:** Northern/central California and Nevada; explicitly updated November 4, 2005
- **access_format:** HTML tables
- **verification_status:** opened_verified
- **evidence_url:** https://www.pausatf.org/data/Coaches.html
- **implementation_notes:** Archived historical research only. Do not load as current coach directory. Revalidate every affiliation and email on current official school site and exclude colleges/clubs/Nevada. Included to prevent mistaking a large search-visible directory for fresh data.
- **priority:** 3
- **top_three_rank:** null
- **verification_date:** 2026-10-04
- **observed_routes:**
  - (empty list)
- **checked_date:** 2026-10-04
- **applicable_states:**
  - CA
- **shortlist_rank:** null
- **source_id:** SRC-032

### SRC-033

California Coaches Association

- **state:** CA
- **name:** California Coaches Association
- **url:** https://www.calcoachesassociation.net/
- **category:** multisport_coaches_association
- **sports_scope:** All HS sports including XC/track
- **school_or_coach_fields:** Public awards, section representatives and school affiliations; no all-member export established
- **coverage:** Statewide association; sparse public names
- **access_format:** SportsEngine HTML/PDF; membership registration
- **verification_status:** blocked
- **evidence_url:** https://www.calcoachesassociation.net/
- **implementation_notes:** Search results confirm current association, membership options and section-rep page. Direct root and section-rep opens failed. Association representatives are not a comprehensive track/XC directory. Keep as supplemental award/clinic source only.
- **priority:** 3
- **top_three_rank:** null
- **verification_date:** 2026-10-04
- **observed_routes:**
  - https://www.calcoachesassociation.net/section-reps
- **checked_date:** 2026-10-04
- **applicable_states:**
  - CA
- **shortlist_rank:** null
- **source_id:** SRC-033

### SRC-034

FHSAA public Home Campus school and coach directory

- **state:** FL
- **name:** FHSAA public Home Campus school and coach directory
- **url:** https://www.cifsshome.org/widget/school/directory?section=10&school=
- **category:** state_athletic_coach_directory
- **sports_scope:** Boys/girls XC and track, plus other sports
- **school_or_coach_fields:** School ID; sport; head-coach name/email; position-not-filled marker; school/AD data tabs
- **coverage:** FHSAA public/private/charter members; selector includes middle schools and placeholders
- **access_format:** Public JavaScript shared Home Campus directory
- **verification_status:** opened_verified
- **evidence_url:** https://www.cifsshome.org/widget/school/directory?school_id=1872&section_id=10
- **implementation_notes:** In cloud browser selected FHSAA from public CIF widget, then Bolles, then Coaches and Sports. Verified separate boys/girls XC coach+email, boys track coach+email and girls track Position not filled. Observed FHSAA selector listed 880 entries, including middle schools, so this is not 880 high schools. Use school IDs discovered from UI; no hidden API or bulk endpoint claimed. FHSAA native member-directory wrapper and sport-filtered locations widget are also verified.
- **priority:** 1
- **top_three_rank:** 1
- **verification_date:** 2026-10-04
- **observed_routes:**
  - https://www.cifsshome.org/widget/school/directory?school_id=1872&section_id=10
  - https://fhsaa.com/sports/2020/1/28/member_directory.aspx
  - https://www.fhsaahome.org/widget/school-directory-locations
- **checked_date:** 2026-10-04
- **applicable_states:**
  - FL
- **shortlist_rank:** 1
- **source_id:** SRC-034

### SRC-035

MileSplit Florida / flrunners team directory

- **state:** FL
- **name:** MileSplit Florida / flrunners team directory
- **url:** https://fl.milesplit.com/teams
- **category:** sport_team_database
- **sports_scope:** XC and track
- **school_or_coach_fields:** Team ID; school/team; city; profile URL
- **coverage:** Florida sport programs, with middle school/college/club/closed entries mixed
- **access_format:** Public HTML index
- **verification_status:** opened_verified
- **evidence_url:** https://fl.milesplit.com/teams
- **implementation_notes:** Strong crosswalk and gap seed, not a confirmed complete coaching directory. Preserve team IDs; reconcile school status and level. The index explicitly includes closed-school labels and middle schools. No coach API verified and student roster/result extraction is out of scope.
- **priority:** 1
- **top_three_rank:** 2
- **verification_date:** 2026-10-04
- **observed_routes:**
  - (empty list)
- **checked_date:** 2026-10-04
- **applicable_states:**
  - FL
- **shortlist_rank:** 2
- **source_id:** SRC-035

### SRC-036

Florida DOE private-school contact export

- **state:** FL
- **name:** Florida DOE private-school contact export
- **url:** https://web09.fldoe.org/PrivateSchoolDirectory/DownloadSchools
- **category:** education_department_private_directory
- **sports_scope:** All private schools; join to XC/track sponsorship
- **school_or_coach_fields:** School name; address; director name; phone; school email
- **coverage:** Florida private-school annual survey directory; self-reported, not accreditation
- **access_format:** HTML download form; Excel export
- **verification_status:** opened_verified
- **evidence_url:** https://web09.fldoe.org/PrivateSchoolDirectory/DownloadSchools
- **implementation_notes:** Official page states exact exported contact fields and offers Download All Schools or district selection. Main search initially displayed zero pending filters/loading; that is not zero schools. Use export and grade filters/profile checks, then official athletics pages for coaches. Do not infer that director is coach or DOE endorses a listed school. Binary export not downloaded in this research.
- **priority:** 1
- **top_three_rank:** 3
- **verification_date:** 2026-10-04
- **observed_routes:**
  - https://web09.fldoe.org/PrivateSchoolDirectory/
  - https://www.fldoe.org/schools/school-choice/directories.stml
- **checked_date:** 2026-10-04
- **applicable_states:**
  - FL
- **shortlist_rank:** 3
- **source_id:** SRC-036

### SRC-037

Florida DOE Master School ID public-school inventory

- **state:** FL
- **name:** Florida DOE Master School ID public-school inventory
- **url:** https://eds.fldoe.org/EDS/MasterSchoolID/index.cfm
- **category:** education_department_public_directory
- **sports_scope:** All public schools
- **school_or_coach_fields:** District/school numbers; school identity; city/district; school profile and grade/status details
- **coverage:** Statewide public schools; current indexed year 2026–27
- **access_format:** ColdFusion search; statewide/district downloads
- **verification_status:** blocked
- **evidence_url:** https://eds.fldoe.org/EDS/MasterSchoolID/index.cfm
- **implementation_notes:** Search results verify official 2026–27 UI, instructions to submit blank for all schools, and download chooser. Direct opens timed out. Download URL is observed, but response/schema not tested. Pair with FHSAA coach directory and filter active schools serving grades 9–12.
- **priority:** 2
- **top_three_rank:** null
- **verification_date:** 2026-10-04
- **observed_routes:**
  - https://eds.fldoe.org/EDS/MasterSchoolID/Downloads/SelectDistrict.cfm?type=1
- **checked_date:** 2026-10-04
- **applicable_states:**
  - FL
- **shortlist_rank:** null
- **source_id:** SRC-037

### SRC-038

FHSAA current XC classification files

- **state:** FL
- **name:** FHSAA current XC classification files
- **url:** https://fhsaa.com/news/2026/4/8/about-us-fall-sport-classifications-available-for-2026-27-2027-28.aspx
- **category:** state_sport_participation_inventory
- **sports_scope:** Boys/girls XC
- **school_or_coach_fields:** School/team; classification; district/region as file provides
- **coverage:** Final 2026–27/2027–28 FHSAA XC classification cycle
- **access_format:** HTML release; separate XLSX and PDF links
- **verification_status:** opened_verified
- **evidence_url:** https://fhsaa.com/news/2026/4/8/about-us-fall-sport-classifications-available-for-2026-27-2027-28.aspx
- **implementation_notes:** Use separate boys/girls files as definitive sport-specific participation seeds, not all-sports membership alone. Linked XLSX document wrappers opened; workbook contents not extracted. Follow wrapper download rather than assuming an S3 URL. This is same source organization as coach directory, not an independent shortlist source.
- **priority:** 2
- **top_three_rank:** null
- **verification_date:** 2026-10-04
- **observed_routes:**
  - https://fhsaa.com/documents/2026/4/8//2026_27_2027_28_Finalized_BXC_Classification.xlsx?id=7605
  - https://fhsaa.com/documents/2026/4/8//2026_27_2027_28_Finalized_GXC_Classification.xlsx?id=7609
- **checked_date:** 2026-10-04
- **applicable_states:**
  - FL
- **shortlist_rank:** null
- **source_id:** SRC-038

### SRC-039

FHSAA current track classification files

- **state:** FL
- **name:** FHSAA current track classification files
- **url:** https://fhsaa.com/news/2026/7/17/baseball-final-spring-sport-classifications-available-for-2026-27-2027-28.aspx
- **category:** state_sport_participation_inventory
- **sports_scope:** Boys/girls track and field
- **school_or_coach_fields:** School/team; classification; district/region as file provides
- **coverage:** Final 2026–27/2027–28 FHSAA track classification cycle
- **access_format:** HTML release; XLSX/PDF links
- **verification_status:** opened_verified
- **evidence_url:** https://fhsaa.com/news/2026/7/17/baseball-final-spring-sport-classifications-available-for-2026-27-2027-28.aspx
- **implementation_notes:** Separate boys/girls track files are present and opened as document wrappers. Do not reuse XC class cutoffs for track. Parse actual workbook after download validation; no workbook rows audited in this pass.
- **priority:** 2
- **top_three_rank:** null
- **verification_date:** 2026-10-04
- **observed_routes:**
  - https://fhsaa.com/documents/2026/7/17//2026_27_2027_28_Finalized_BTF_Classification.xlsx?id=7960
  - https://fhsaa.com/documents/2026/7/17//2026_27_2027_28_Finalized_GTF_Classification.xlsx?id=7953
- **checked_date:** 2026-10-04
- **applicable_states:**
  - FL
- **shortlist_rank:** null
- **source_id:** SRC-039

### SRC-040

Florida Athletic Coaches Association

- **state:** FL
- **name:** Florida Athletic Coaches Association
- **url:** https://www.floridacoaches.org/state-sports-chairman.html
- **category:** multisport_coaches_association
- **sports_scope:** XC and track chapters
- **school_or_coach_fields:** Sport-chair coach; school; term; selected public clinic/award coach affiliations
- **coverage:** Selected association leaders/honorees, not statewide roster
- **access_format:** Weebly HTML; PDFs
- **verification_status:** opened_verified
- **evidence_url:** https://www.floridacoaches.org/state-sports-chairman.html
- **implementation_notes:** Current 2026–27 chair page opened and identifies XC/track school affiliations. Districts page maps 24 districts to counties but does not itself list all coaches. Clinic speakers/awards can enrich. FACA explicitly says it does not provide clinic attendee lists in exhibitor agreement, so do not promise bulk membership data.
- **priority:** 2
- **top_three_rank:** null
- **verification_date:** 2026-10-04
- **observed_routes:**
  - https://www.floridacoaches.org/
  - https://www.floridacoaches.org/districts-by-county--schools.html
  - https://floridacoaches.powermediallc.org/exhibitor-agreement/
- **checked_date:** 2026-10-04
- **applicable_states:**
  - FL
- **shortlist_rank:** null
- **source_id:** SRC-040

### SRC-041

FHSAA sports advisory committees

- **state:** FL
- **name:** FHSAA sports advisory committees
- **url:** https://fhsaa.com/sports/2020/3/11/Sport_Committees.aspx
- **category:** sport_advisory_coach_directory
- **sports_scope:** XC and track committees among sports
- **school_or_coach_fields:** Committee coach names; school; public contact fields as published
- **coverage:** Small advisory committees only
- **access_format:** HTML expandable sport panels
- **verification_status:** opened_verified
- **evidence_url:** https://fhsaa.com/sports/2020/3/11/Sport_Committees.aspx
- **implementation_notes:** Good sparse coach-school corroboration and current stakeholder discovery. Filter XC/track sections. Not equivalent to all member coaches; administrative FHSAA staff are not school coaches.
- **priority:** 2
- **top_three_rank:** null
- **verification_date:** 2026-10-04
- **observed_routes:**
  - (empty list)
- **checked_date:** 2026-10-04
- **applicable_states:**
  - FL
- **shortlist_rank:** null
- **source_id:** SRC-041

### SRC-042

Sunshine State Athletic Association

- **state:** FL
- **name:** Sunshine State Athletic Association
- **url:** https://www.sunshinestateathletics.com/
- **category:** independent_athletic_association
- **sports_scope:** High-school XC and track explicitly listed
- **school_or_coach_fields:** School/member discovery and sport-event participation leads; direct coach roster not found
- **coverage:** 120+ members advertised, not all Florida schools
- **access_format:** HTML sport pages
- **verification_status:** opened_verified
- **evidence_url:** https://www.sunshinestateathletics.com/
- **implementation_notes:** Important alternate athletic system for gap search. Homepage and sport pages opened. Current track page contains apparent copied beach-volleyball venue text; do not ingest venue or claims blindly. No complete current school/coach directory was visible in inspected pages. Cross-check entrants with current school websites.
- **priority:** 3
- **top_three_rank:** null
- **verification_date:** 2026-10-04
- **observed_routes:**
  - https://www.sunshinestateathletics.com/cross-country.html
  - https://www.sunshinestateathletics.com/track.html
- **checked_date:** 2026-10-04
- **applicable_states:**
  - FL
- **shortlist_rank:** null
- **source_id:** SRC-042

### SRC-043

Florida Independent Christian Athletic Association members

- **state:** FL
- **name:** Florida Independent Christian Athletic Association members
- **url:** https://ficaa.org/members
- **category:** independent_christian_athletic_association
- **sports_scope:** General athletic school seeds; XC/track sponsorship not verified
- **school_or_coach_fields:** School name; city; conference/region
- **coverage:** Christian schools grouped into Coastal, Mid-Florida, Central Florida, Suncoast conferences
- **access_format:** Public HTML
- **verification_status:** opened_verified
- **evidence_url:** https://ficaa.org/members
- **implementation_notes:** Useful private-school coverage gap seed. Do not infer XC/track sponsorship solely from FICAA membership. Verify high-school grade span and sport at each school or meet entry source before keeping as active track/XC program.
- **priority:** 2
- **top_three_rank:** null
- **verification_date:** 2026-10-04
- **observed_routes:**
  - (empty list)
- **checked_date:** 2026-10-04
- **applicable_states:**
  - FL
- **shortlist_rank:** null
- **source_id:** SRC-043

### SRC-044

Florida Christian Conference school list

- **state:** FL
- **name:** Florida Christian Conference school list
- **url:** https://www.fccsports.net/forms/FCC%20School%20List.pdf
- **category:** independent_christian_athletic_conference
- **sports_scope:** General school seed; XC/track not verified
- **school_or_coach_fields:** School/contact fields pending file inspection
- **coverage:** FCC member schools
- **access_format:** PDF
- **verification_status:** blocked
- **evidence_url:** https://www.fccsports.net/forms/FCC%20School%20List.pdf
- **implementation_notes:** Search-indexed official PDF found, but direct download timed out. Retain as secondary candidate, not as verified extracted contact list. Check document season and school sport sponsorship before ingestion.
- **priority:** 3
- **top_three_rank:** null
- **verification_date:** 2026-10-04
- **observed_routes:**
  - (empty list)
- **checked_date:** 2026-10-04
- **applicable_states:**
  - FL
- **shortlist_rank:** null
- **source_id:** SRC-044

### SRC-045

Florida Association of Christian Colleges and Schools directory

- **state:** FL
- **name:** Florida Association of Christian Colleges and Schools directory
- **url:** https://faccs.org/schools
- **category:** private_school_directory
- **sports_scope:** All schools; sport enrichment needed
- **school_or_coach_fields:** School name; city; grade span; member type; linked school detail
- **coverage:** 123 listed members at inspection, including PK-only, colleges and an overseas school
- **access_format:** Public HTML table and detail links
- **verification_status:** opened_verified
- **evidence_url:** https://faccs.org/schools
- **implementation_notes:** Filter Florida location and grades 9–12 before crawling school athletics pages. Strong explicit grade-span filter for independent/private coverage. Membership/accreditation does not prove sport sponsorship. Do not include colleges, PK-only or Bahamas record.
- **priority:** 2
- **top_three_rank:** null
- **verification_date:** 2026-10-04
- **observed_routes:**
  - (empty list)
- **checked_date:** 2026-10-04
- **applicable_states:**
  - FL
- **shortlist_rank:** null
- **source_id:** SRC-045

### SRC-046

Florida Council of Independent Schools membership directory

- **state:** FL
- **name:** Florida Council of Independent Schools membership directory
- **url:** https://www.fcis.org/about/directory
- **category:** private_school_directory
- **sports_scope:** All private school sports after enrichment
- **school_or_coach_fields:** School identity; school contact/profile fields; phone and website links
- **coverage:** FCIS independent member schools; filter secondary grades
- **access_format:** Finalsite HTML directory
- **verification_status:** opened_verified
- **evidence_url:** https://www.fcis.org/about/directory
- **implementation_notes:** Directory opened. Use to recover private-school aliases and official sites, then athletics staff/sport pages for coaches. Directory membership does not establish XC/track program. Avoid treating school office contacts as coaches.
- **priority:** 2
- **top_three_rank:** null
- **verification_date:** 2026-10-04
- **observed_routes:**
  - (empty list)
- **checked_date:** 2026-10-04
- **applicable_states:**
  - FL
- **shortlist_rank:** null
- **source_id:** SRC-046

### SRC-047

Cypress Creek High School official coaches directory example

- **state:** FL
- **name:** Cypress Creek High School official coaches directory example
- **url:** https://cchs.pasco.k12.fl.us/hscoachdirectory/
- **category:** official_school_staff_fallback
- **sports_scope:** Boys/girls XC and track
- **school_or_coach_fields:** Sport; gender; coach name; public email
- **coverage:** One Pasco County high school
- **access_format:** WordPress HTML seasonal coach lists
- **verification_status:** opened_verified
- **evidence_url:** https://cchs.pasco.k12.fl.us/hscoachdirectory/
- **implementation_notes:** Useful official-school parser fixture. Cross-country and track head-coach records are explicit. Pasco school sites have multiple templates and domains; seed each school from official district/DOE directory rather than guessing URLs.
- **priority:** 2
- **top_three_rank:** null
- **verification_date:** 2026-10-04
- **observed_routes:**
  - (empty list)
- **checked_date:** 2026-10-04
- **applicable_states:**
  - FL
- **shortlist_rank:** null
- **source_id:** SRC-047

### SRC-048

Winter Springs High School coaches directory example

- **state:** FL
- **name:** Winter Springs High School coaches directory example
- **url:** https://www.winterspringshs.scps.k12.fl.us/coaches
- **category:** official_school_staff_fallback
- **sports_scope:** XC and track
- **school_or_coach_fields:** Sport; head coach; public email; athletics administration
- **coverage:** One Seminole County high school
- **access_format:** HTML tables
- **verification_status:** opened_verified
- **evidence_url:** https://www.winterspringshs.scps.k12.fl.us/coaches
- **implementation_notes:** Current page opened with seasonal Sport/Head Coach/Email tables. Treat TBA/vacancy as unknown, not a named coach inherited from another sport. School-site freshness wins over older directories.
- **priority:** 2
- **top_three_rank:** null
- **verification_date:** 2026-10-04
- **observed_routes:**
  - (empty list)
- **checked_date:** 2026-10-04
- **applicable_states:**
  - FL
- **shortlist_rank:** null
- **source_id:** SRC-048

### SRC-049

NYSED School and District Directory / SEDREF public reports

- **state:** NY
- **name:** NYSED School and District Directory / SEDREF public reports
- **url:** https://www.p12.nysed.gov/irs/schoolDirectory/
- **category:** state_education_directory
- **sports_scope:** All schools; sport not supplied
- **school_or_coach_fields:** Institution identity; public/nonpublic school universe; school contacts and grades via linked reports
- **coverage:** Statewide public, charter, nonpublic and BOCES
- **access_format:** HTML hub; public query; report exports
- **verification_status:** opened_verified
- **evidence_url:** https://www.p12.nysed.gov/irs/schoolDirectory/
- **implementation_notes:** Direct HTTPS GET returned 200 after web-reader failure. Observed links: https://portal.nysed.gov/pls/sedrefpublic/SED.sed_inst_qry_vw$.startup and http://eservices.nysed.gov/sedreports/list?id=1 . Follow public reporting workflow; do not persist transient encrypted obrar.cgi URLs. Sport/coach data requires enrichment.
- **priority:** P1
- **verified_at:** 2026-10-04
- **source_id:** SRC-049
- **shortlist_rank:** 3
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NY
- **original_source_id:** NE-001
- **observed_routes:**
  - (empty list)

### SRC-050

NYSPHSAA official section map and section links

- **state:** NY
- **name:** NYSPHSAA official section map and section links
- **url:** https://nysphsaa.org/sports/2021/6/7/section-map.aspx
- **category:** state_athletics_index
- **sports_scope:** All interscholastic sports including XC/indoor/outdoor
- **school_or_coach_fields:** Section jurisdiction, official section websites, section office contacts
- **coverage:** 11 NYSPHSAA sections; separate NYC PSAL/CHSAA/independent coverage needed
- **access_format:** HTML
- **verification_status:** opened_verified
- **evidence_url:** https://nysphsaa.org/sports/2021/6/7/section-map.aspx
- **implementation_notes:** Use as authoritative source-of-sources. Section staff are not school coaches. Follow all 11 section links and member-school/league/sport pages.
- **priority:** P1
- **verified_at:** 2026-10-04
- **source_id:** SRC-050
- **shortlist_rank:** 1
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NY
- **original_source_id:** NE-002
- **observed_routes:**
  - (empty list)

### SRC-051

Section V member school directory

- **state:** NY
- **name:** Section V member school directory
- **url:** https://schools.sectionv.org/
- **category:** regional_school_directory
- **sports_scope:** All sports; XC/track enrichment via school sites
- **school_or_coach_fields:** School name; league; athletics URL; district URL; schedule URL
- **coverage:** Genesee Valley / Section V public and private/parochial members; includes some middle-school/co-op records
- **access_format:** HTML cards; interactive search/filter
- **verification_status:** opened_verified
- **evidence_url:** https://schools.sectionv.org/
- **implementation_notes:** Excellent direct athletics-domain seeds. Page shows 127 member schools but 133 displayed records; do not treat either as high-school count. Filter middle schools and retain cooperative-team links. Dated update 2026-09-03.
- **priority:** P1
- **verified_at:** 2026-10-04
- **source_id:** SRC-051
- **shortlist_rank:** null
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NY
- **original_source_id:** NE-003
- **observed_routes:**
  - (empty list)

### SRC-052

Section IV member schools

- **state:** NY
- **name:** Section IV member schools
- **url:** https://www.sectionivathletics.com/page/member-schools
- **category:** regional_school_directory
- **sports_scope:** All sports
- **school_or_coach_fields:** Member school identity and linked school resources
- **coverage:** Southern Tier / Section IV
- **access_format:** HTML (Apptegy)
- **verification_status:** opened_verified
- **evidence_url:** https://www.sectionivathletics.com/page/member-schools
- **implementation_notes:** Seed section member schools then visit athletics staff/coaching pages. Verify list and school closures before assuming current sport participation.
- **priority:** P1
- **verified_at:** 2026-10-04
- **source_id:** SRC-052
- **shortlist_rank:** null
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NY
- **original_source_id:** NE-004
- **observed_routes:**
  - (empty list)

### SRC-053

Section IX OCIAA spring track handbook

- **state:** NY
- **name:** Section IX OCIAA spring track handbook
- **url:** https://www.sectionixathletics.org/springtrack/20232024/OCIAAspringtrackHandbook2024.pdf
- **category:** regional_coach_contacts
- **sports_scope:** Outdoor track
- **school_or_coach_fields:** Committee member school, coach/name, email, phone
- **coverage:** OCIAA / Section IX subset; 2024 handbook
- **access_format:** PDF, 14 pages
- **verification_status:** opened_verified
- **evidence_url:** https://www.sectionixathletics.org/springtrack/20232024/OCIAAspringtrackHandbook2024.pdf
- **implementation_notes:** Contains a Spring Track Coaches Committee table. Useful named-coach leads, NOT every Section IX coach. Source season is 2024 even if search crawl is recent; verify every current appointment on school site.
- **priority:** P2
- **verified_at:** 2026-10-04
- **source_id:** SRC-053
- **shortlist_rank:** null
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NY
- **original_source_id:** NE-005
- **observed_routes:**
  - (empty list)

### SRC-054

PSAL Cross Country school/team listing

- **state:** NY
- **name:** PSAL Cross Country school/team listing
- **url:** https://www.psal.org/sports/sport.aspx?flag=All&spCode=033
- **category:** state_athletics_sport_directory
- **sports_scope:** Boys and girls cross-country; sport menu links indoor/outdoor
- **school_or_coach_fields:** School/team names and IDs; published client code renders head, co-, and assistant coach names from team records
- **coverage:** NYC PSAL programs; campus programs may span schools
- **access_format:** ASP.NET HTML; client-routed team profiles
- **verification_status:** opened_verified
- **evidence_url:** https://www.psal.org/sports/sport.aspx?flag=All&spCode=033
- **implementation_notes:** Observed profile https://www.psal.org/profiles/team-profile.aspx#033/13507; boys/girls XC codes 033/034. Public https://www.psal.org/scripts/js/Team_Profile.js verified with getTeamDetails and getCoachNamesIds calls and head/co/assistant coach rendering. These are observed site-internal endpoints, not a supported public API; actual JSON responses not tested. Harvest real school/sport IDs and current season from page; fragment needs JS. Never call getTeamRosters or athlete/statistics endpoints for this project.
- **priority:** P1
- **verified_at:** 2026-10-04
- **source_id:** SRC-054
- **shortlist_rank:** 2
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NY
- **original_source_id:** NE-006
- **observed_routes:**
  - (empty list)

### SRC-055

NYSAIS member school directory

- **state:** NY
- **name:** NYSAIS member school directory
- **url:** https://www.nysais.org/schools/
- **category:** independent_school_directory
- **sports_scope:** All schools; not all have track/XC
- **school_or_coach_fields:** Member school lookup and school links
- **coverage:** New York independent schools, all grades
- **access_format:** HTML/JS directory
- **verification_status:** opened_verified
- **evidence_url:** https://www.nysais.org/schools/
- **implementation_notes:** Opened public directory shell; school results may require rendering. Restrict to schools serving grades 9-12, then athletics pages. Complements NYSPHSAA and PSAL.
- **priority:** P1
- **verified_at:** 2026-10-04
- **source_id:** SRC-055
- **shortlist_rank:** null
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NY
- **original_source_id:** NE-007
- **observed_routes:**
  - (empty list)

### SRC-056

NYSAIS Athletics

- **state:** NY
- **name:** NYSAIS Athletics
- **url:** https://www.nysais.org/athletics/
- **category:** independent_athletics
- **sports_scope:** Track/XC plus other sports
- **school_or_coach_fields:** Athletic association resources; championship/team leads; coordinator contacts where published
- **coverage:** NYSAIS participating independent schools
- **access_format:** HTML and linked documents
- **verification_status:** opened_verified
- **evidence_url:** https://www.nysais.org/athletics/
- **implementation_notes:** Use track/XC classification/championship resources to establish participation. Association coordinators are not exhaustive coach directory.
- **priority:** P2
- **verified_at:** 2026-10-04
- **source_id:** SRC-056
- **shortlist_rank:** null
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NY
- **original_source_id:** NE-008
- **observed_routes:**
  - (empty list)

### SRC-057

New York State Catholic High School Athletic Association

- **state:** NY
- **name:** New York State Catholic High School Athletic Association
- **url:** https://www.chsaany.org/
- **category:** parochial_athletics
- **sports_scope:** Track/XC plus other sports
- **school_or_coach_fields:** Organization, event and team resources indicated by search
- **coverage:** Catholic member athletics
- **access_format:** SportsEngine-style public site; retrieval failed
- **verification_status:** blocked
- **evidence_url:** https://www.chsaany.org/
- **implementation_notes:** Search indexing confirms official site and current news. Direct web open failed; not verified scrape-ready. Keep separate from Colorado CHSAA at chsaanow.com. Use school staff sites if retrieval remains unavailable.
- **priority:** P1
- **verified_at:** 2026-10-04
- **source_id:** SRC-057
- **shortlist_rank:** null
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NY
- **original_source_id:** NE-009
- **observed_routes:**
  - (empty list)

### SRC-058

NJSIAA Member Information

- **state:** NJ
- **name:** NJSIAA Member Information
- **url:** https://www.njsiaa.org/schools/member-information
- **category:** state_athletics_school_directory
- **sports_scope:** All sports
- **school_or_coach_fields:** School name,address,phone,athletic director name/phone
- **coverage:** Statewide NJSIAA public, nonpublic, charter members
- **access_format:** Public HTML table; ?page=0, ?page=1 pagination
- **verification_status:** opened_verified
- **evidence_url:** https://www.njsiaa.org/schools/member-information
- **implementation_notes:** Scrape list pages only and follow real pager links. Opened school detail /schools/abraham-clark-high-school redirects to /user/login?destination=...; do not assume contacts behind detail are public. AD is not track coach.
- **priority:** P1
- **verified_at:** 2026-10-04
- **source_id:** SRC-058
- **shortlist_rank:** 1
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NJ
- **original_source_id:** NE-010
- **observed_routes:**
  - (empty list)

### SRC-059

NJSIAA individual school details

- **state:** NJ
- **name:** NJSIAA individual school details
- **url:** https://www.njsiaa.org/schools/abraham-clark-high-school
- **category:** restricted_school_detail
- **sports_scope:** All sports
- **school_or_coach_fields:** Unverified behind login; public list has school/AD
- **coverage:** NJSIAA individual profiles
- **access_format:** Login redirect
- **verification_status:** login_required
- **evidence_url:** https://www.njsiaa.org/schools/abraham-clark-high-school
- **implementation_notes:** Specific profile redirects to https://www.njsiaa.org/user/login?destination=/schools/abraham-clark-high-school . Use public member-information listing for seed data. Do not create accounts or bypass.
- **priority:** P3
- **verified_at:** 2026-10-04
- **source_id:** SRC-059
- **shortlist_rank:** null
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NJ
- **original_source_id:** NE-011
- **observed_routes:**
  - (empty list)

### SRC-060

New Jersey DOE School Directory

- **state:** NJ
- **name:** New Jersey DOE School Directory
- **url:** https://homeroom6.doe.nj.gov/directory/
- **category:** state_education_directory
- **sports_scope:** All schools; sport not supplied
- **school_or_coach_fields:** School/district directory; fields not directly inspected
- **coverage:** Statewide public/nonpublic directory workflow
- **access_format:** Government web application
- **verification_status:** blocked
- **evidence_url:** https://www.nj.gov/education/
- **implementation_notes:** Official NJDOE homepage links this current URL. Direct fetch returned HTTP 403; web open failed. The commonly guessed https://www.nj.gov/education/directory/ is 404. Reverify through normal public access, not bypass; national school files can seed meanwhile.
- **priority:** P1
- **verified_at:** 2026-10-04
- **source_id:** SRC-060
- **shortlist_rank:** null
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NJ
- **original_source_id:** NE-012
- **observed_routes:**
  - (empty list)

### SRC-061

New Jersey XC/TF Coaches Association resources

- **state:** NJ
- **name:** New Jersey XC/TF Coaches Association resources
- **url:** https://njxctfca.org/new-jersey-xc-tf-coaches-association/
- **category:** coaches_association
- **sports_scope:** XC, indoor and outdoor track
- **school_or_coach_fields:** Association resources, jobs, links to county associations; no public statewide membership roster verified
- **coverage:** Statewide coaches association
- **access_format:** WordPress HTML
- **verification_status:** opened_verified
- **evidence_url:** https://njxctfca.org/new-jersey-xc-tf-coaches-association/
- **implementation_notes:** Useful hub, not an exhaustive coach database. Some central page content dates to 2020 despite newer sidebar posts. Jobs describe vacancies and must not be interpreted as filled current coaching positions.
- **priority:** P2
- **verified_at:** 2026-10-04
- **source_id:** SRC-061
- **shortlist_rank:** null
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NJ
- **original_source_id:** NE-013
- **observed_routes:**
  - (empty list)

### SRC-062

NJXCTFCA county association link index

- **state:** NJ
- **name:** NJXCTFCA county association link index
- **url:** https://njxctfca.org/links/
- **category:** coaches_association_index
- **sports_scope:** XC/track
- **school_or_coach_fields:** Verified outgoing county/coaches association URLs
- **coverage:** NJ county/regional associations
- **access_format:** HTML hyperlinks
- **verification_status:** opened_verified
- **evidence_url:** https://njxctfca.org/links/
- **implementation_notes:** Links include South Jersey, Shore, Bergen, Passaic, Hudson, Union, Essex, Mercer, Middlesex, Morris. Crawl each subject to current access; some old providers such as webs.com may be obsolete.
- **priority:** P1
- **verified_at:** 2026-10-04
- **source_id:** SRC-062
- **shortlist_rank:** 2
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NJ
- **original_source_id:** NE-014
- **observed_routes:**
  - (empty list)

### SRC-063

Shore Track Coaches Association

- **state:** NJ
- **name:** Shore Track Coaches Association
- **url:** https://shorecoaches.com/
- **category:** regional_coaches_association
- **sports_scope:** XC, indoor/outdoor track
- **school_or_coach_fields:** Meet/school participation resources; association contacts; no full coach directory verified
- **coverage:** Monmouth and Ocean counties; HS and MS
- **access_format:** WordPress HTML and downloads
- **verification_status:** opened_verified
- **evidence_url:** https://shorecoaches.com/
- **implementation_notes:** Opened without www; www version failed. Filter high-school meets. Extract school-level participation or explicitly named staff only, not athlete result rows.
- **priority:** P2
- **verified_at:** 2026-10-04
- **source_id:** SRC-063
- **shortlist_rank:** null
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NJ
- **original_source_id:** NE-015
- **observed_routes:**
  - (empty list)

### SRC-064

South Jersey Track Coaches Association

- **state:** NJ
- **name:** South Jersey Track Coaches Association
- **url:** https://www.sjtrack.org/about-us
- **category:** regional_coaches_association
- **sports_scope:** XC and track
- **school_or_coach_fields:** Association scope, meeting documents, event and school leads
- **coverage:** Atlantic,Burlington,Cape May,Camden,Cumberland,Gloucester,Salem
- **access_format:** Wix HTML and linked meeting documents
- **verification_status:** opened_verified
- **evidence_url:** https://www.sjtrack.org/about-us
- **implementation_notes:** Association explicitly serves high-school track/XC coaches in seven counties. Documents may expose officers/attendees but no comprehensive membership directory verified.
- **priority:** P2
- **verified_at:** 2026-10-04
- **source_id:** SRC-064
- **shortlist_rank:** null
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NJ
- **original_source_id:** NE-016
- **observed_routes:**
  - (empty list)

### SRC-065

Morris County Track Coaches Association

- **state:** NJ
- **name:** Morris County Track Coaches Association
- **url:** https://www.mctrack.org/
- **category:** regional_coaches_association
- **sports_scope:** XC, winter track, spring track
- **school_or_coach_fields:** School/team participation from meet index; named coach service-award leads
- **coverage:** Morris County and NJAC; statewide NJSIAA result archive as secondary route
- **access_format:** Static HTML result indexes
- **verification_status:** opened_verified
- **evidence_url:** https://www.mctrack.org/
- **implementation_notes:** Current homepage updates visible 2026-09-29. Use team-level evidence only. Service awards and historic results cannot establish current employment.
- **priority:** P2
- **verified_at:** 2026-10-04
- **source_id:** SRC-065
- **shortlist_rank:** null
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NJ
- **original_source_id:** NE-017
- **observed_routes:**
  - (empty list)

### SRC-066

Greater Middlesex Conference Track Coaches Association member schools

- **state:** NJ
- **name:** Greater Middlesex Conference Track Coaches Association member schools
- **url:** https://www.gmctrackcoaches.org/member-schools
- **category:** regional_school_directory
- **sports_scope:** XC/indoor/outdoor track
- **school_or_coach_fields:** Member school listing (Google Sites embedded content may need render)
- **coverage:** Greater Middlesex Conference
- **access_format:** Google Sites HTML/embedded files
- **verification_status:** opened_verified
- **evidence_url:** https://www.gmctrackcoaches.org/member-schools
- **implementation_notes:** Public member-schools page opens. Follow site-rendered school list, then each school athletics staff. No coach emails verified in text extraction.
- **priority:** P1
- **verified_at:** 2026-10-04
- **source_id:** SRC-066
- **shortlist_rank:** null
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NJ
- **original_source_id:** NE-018
- **observed_routes:**
  - (empty list)

### SRC-067

NJAIS independent member directory

- **state:** NJ
- **name:** NJAIS independent member directory
- **url:** https://members.njais.org/rolodex/searchOrganizationDirectory
- **category:** independent_school_directory
- **sports_scope:** All schools
- **school_or_coach_fields:** School search/directory, details not rendered in text reader
- **coverage:** New Jersey independent association members
- **access_format:** JS directory; public entry linked by NJAIS
- **verification_status:** opened_verified
- **evidence_url:** https://www.njais.org/home
- **implementation_notes:** Opened zero-text JS shell; verify rendered results before adapter production. Official provenance https://www.njais.org/home . Filter school grade range and sports.
- **priority:** P2
- **verified_at:** 2026-10-04
- **source_id:** SRC-067
- **shortlist_rank:** null
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NJ
- **original_source_id:** NE-019
- **observed_routes:**
  - (empty list)

### SRC-068

Massachusetts State Track Coaches Association current members

- **state:** MA
- **name:** Massachusetts State Track Coaches Association current members
- **url:** https://mstca.org/coaches-corner/current-members
- **category:** coaches_association_member_directory
- **sports_scope:** Track and XC association; individual sports/roles not labeled
- **school_or_coach_fields:** Member ID, first/last name, organization, email and phone in public embedded page data; membership status
- **coverage:** MSTCA membership; not a complete current coaching census
- **access_format:** Next.js HTML plus interactive pagination
- **verification_status:** opened_verified
- **evidence_url:** https://mstca.org/coaches-corner/current-members
- **implementation_notes:** Direct GET verified Next.js self.__next_f.push page-data with members array: 24 rows on first page, totalCount 1417. Fields: _id, firstName, lastName, organization, email, phone, status, lastPaymentDate, image, altText. First page had 24 email values, 11 equal unknown@mstca.org. Parse public payload or render normal pagination; no separate API verified. Reject placeholders; collect only name/org and professional contact fields. Membership or old payment date does not prove current coach employment/sport. Confirm at school site.
- **priority:** P1
- **verified_at:** 2026-10-04
- **source_id:** SRC-068
- **shortlist_rank:** 1
- **checked_date:** 2026-10-04
- **applicable_states:**
  - MA
- **original_source_id:** NE-020
- **observed_routes:**
  - (empty list)

### SRC-069

MIAA Schools Directory and school profiles

- **state:** MA
- **name:** MIAA Schools Directory and school profiles
- **url:** https://www.miaa.net/schools
- **category:** state_athletics_school_directory
- **sports_scope:** All sports; sport page linked separately
- **school_or_coach_fields:** School, district, league, principal, athletic director, mascot; published school details
- **coverage:** MIAA public and private/parochial member schools
- **access_format:** Drupal HTML paginated listing and /group/{id} profiles
- **verification_status:** opened_verified
- **evidence_url:** https://www.miaa.net/schools
- **implementation_notes:** Observed school profile https://www.miaa.net/group/2 lists principal and AD, not track coaches. Enumerate real links/pagination; use athletics school seed to find staff. Membership PDF at https://www.miaa.net/media/824 is 32 pages but landing says updated 2024-07-18.
- **priority:** P1
- **verified_at:** 2026-10-04
- **source_id:** SRC-069
- **shortlist_rank:** 2
- **checked_date:** 2026-10-04
- **applicable_states:**
  - MA
- **original_source_id:** NE-021
- **observed_routes:**
  - (empty list)

### SRC-070

MIAA certified coaches by school

- **state:** MA
- **name:** MIAA certified coaches by school
- **url:** https://www.miaa.net/sites/default/files/2024-09/1-certified-coaches-by-school.pdf
- **category:** coach_certification_roster
- **sports_scope:** All sports, no sport field
- **school_or_coach_fields:** First name,last name,school
- **coverage:** MIAA/MSAA certified coaches
- **access_format:** Text PDF, 383 pages
- **verification_status:** opened_verified
- **evidence_url:** https://www.miaa.net/sites/default/files/2024-09/1-certified-coaches-by-school.pdf
- **implementation_notes:** PDF pages bear 9/18/2026 while landing labels 09-17-26; retain both source dates. No sport, email or proof of current employment. Useful name-school corroboration only; cannot use as current XC/track roster. Discover current PDF from https://www.miaa.net/you-are/coaches rather than filename date.
- **priority:** P2
- **verified_at:** 2026-10-04
- **source_id:** SRC-070
- **shortlist_rank:** null
- **checked_date:** 2026-10-04
- **applicable_states:**
  - MA
- **original_source_id:** NE-022
- **observed_routes:**
  - (empty list)

### SRC-071

MIAA league directory

- **state:** MA
- **name:** MIAA league directory
- **url:** https://www.miaa.net/sites/default/files/2024-06/miaa-league-directory.pdf
- **category:** regional_school_directory
- **sports_scope:** All sports
- **school_or_coach_fields:** League membership and school information
- **coverage:** MIAA leagues across districts
- **access_format:** Text PDF, 22 pages
- **verification_status:** opened_verified
- **evidence_url:** https://www.miaa.net/sites/default/files/2024-06/miaa-league-directory.pdf
- **implementation_notes:** File path date is not edition date. Use document contents and landing update date. Complements school directory for league-based expansion, not a coach list.
- **priority:** P2
- **verified_at:** 2026-10-04
- **source_id:** SRC-071
- **shortlist_rank:** null
- **checked_date:** 2026-10-04
- **applicable_states:**
  - MA
- **original_source_id:** NE-023
- **observed_routes:**
  - (empty list)

### SRC-072

Massachusetts DESE public/private directories

- **state:** MA
- **name:** Massachusetts DESE public/private directories
- **url:** https://profiles.doe.mass.edu/search/search.aspx
- **category:** state_education_directory
- **sports_scope:** All schools; no sport proof
- **school_or_coach_fields:** School/org name,organization type, profiles and export/search capabilities
- **coverage:** Statewide public, charter, private, approved special education
- **access_format:** ASP.NET search/directories and export workflow
- **verification_status:** opened_verified
- **evidence_url:** https://profiles.doe.mass.edu/search/search.aspx
- **implementation_notes:** Observed public-directory link https://profiles.doe.mass.edu/search/search_link.aspx?leftNavId=11238&orgType=6,13&runOrgSearch=Y and private link orgType=11. Link pages opened as short redirect shell; render normal search if needed. No documented JSON API found. Include high-school grade ranges, then school athletics sites.
- **priority:** P1
- **verified_at:** 2026-10-04
- **source_id:** SRC-072
- **shortlist_rank:** 3
- **checked_date:** 2026-10-04
- **applicable_states:**
  - MA
- **original_source_id:** NE-024
- **observed_routes:**
  - (empty list)

### SRC-073

MassGIS Massachusetts Schools dataset metadata

- **state:** MA
- **name:** MassGIS Massachusetts Schools dataset metadata
- **url:** https://www.mass.gov/info-details/massgis-data-massachusetts-schools-pre-k-through-high-school
- **category:** state_education_geodata
- **sports_scope:** All pre-K through HS
- **school_or_coach_fields:** School location,DESE_ID,type,grade/contact attributes described in metadata
- **coverage:** Public,private,charter,vocational and special education
- **access_format:** Official HTML metadata; shapefile ZIP download; data-hub search
- **verification_status:** opened_verified
- **evidence_url:** https://www.mass.gov/info-details/massgis-data-massachusetts-schools-pre-k-through-high-school
- **implementation_notes:** Direct GET returned HTTP 200 after web-reader failure. Official link observed: https://s3.us-east-1.amazonaws.com/download.massgis.digital.mass.gov/shapefiles/state/schools.zip . Dataset is December 2025 using DESE profiles as of 2025-11-05. Exact linked ZIP was not downloaded. Use DESE_ID, school type/grades and official website attributes for high-school seeds. No coaches in this dataset.
- **priority:** P2
- **verified_at:** 2026-10-04
- **source_id:** SRC-073
- **shortlist_rank:** null
- **checked_date:** 2026-10-04
- **applicable_states:**
  - MA
- **original_source_id:** NE-025
- **observed_routes:**
  - (empty list)

### SRC-074

Schools ArcGIS FeatureServer (MassGIS-sourced)

- **state:** MA
- **name:** Schools ArcGIS FeatureServer (MassGIS-sourced)
- **url:** https://services1.arcgis.com/TXaY625xGc0yvAuQ/arcgis/rest/services/Schools/FeatureServer
- **category:** school_geodata_endpoint
- **sports_scope:** Pre-K through HS
- **school_or_coach_fields:** Feature service layer with school geographic attributes; inspect schema
- **coverage:** Massachusetts schools, MassGIS sourced; host owner not yet verified
- **access_format:** ArcGIS REST FeatureServer; JSON query support
- **verification_status:** opened_verified
- **evidence_url:** https://services1.arcgis.com/TXaY625xGc0yvAuQ/arcgis/rest/services/Schools/FeatureServer
- **implementation_notes:** Service directory explicitly says Data Source MassGIS and supports JSON; max record count 1,000. This establishes technical availability, not authoritative publisher or freshness. Inspect layer 0 fields, item owner/update date before use; only read/query, never applyEdits. Prefer official MassGIS data hub download.
- **priority:** P3
- **verified_at:** 2026-10-04
- **source_id:** SRC-074
- **shortlist_rank:** null
- **checked_date:** 2026-10-04
- **applicable_states:**
  - MA
- **original_source_id:** NE-026
- **observed_routes:**
  - (empty list)

### SRC-075

MIAA Track & Cross Country

- **state:** MA
- **name:** MIAA Track & Cross Country
- **url:** https://www.miaa.net/track-cross-country
- **category:** state_athletics_sport_resources
- **sports_scope:** XC,indoor,outdoor track, boys/girls
- **school_or_coach_fields:** Season information,formats,alignments,team/meet links; coordinator resources
- **coverage:** MIAA track/XC programs
- **access_format:** HTML and linked PDFs/Athletic.net
- **verification_status:** opened_verified
- **evidence_url:** https://www.miaa.net/track-cross-country
- **implementation_notes:** Official page states MIAA partners with Athletic.net for XC/indoor/outdoor entries. Use linked alignments/entry participation to verify sport offering, then staff pages. Do not collect student entries/results.
- **priority:** P2
- **verified_at:** 2026-10-04
- **source_id:** SRC-075
- **shortlist_rank:** null
- **checked_date:** 2026-10-04
- **applicable_states:**
  - MA
- **original_source_id:** NE-027
- **observed_routes:**
  - (empty list)

### SRC-076

CIAC high school coach directory

- **state:** CT
- **name:** CIAC high school coach directory
- **url:** https://ciac.fpsports.org/Directory.aspx?SchoolLevelID=1
- **category:** state_athletics_coach_directory
- **sports_scope:** Boys/girls XC,indoor,outdoor plus other sports
- **school_or_coach_fields:** Search-indexed school,address,sport,role,head coach name,phone
- **coverage:** CIAC high schools including public and private members
- **access_format:** ASP.NET full directory; location/login gate encountered
- **verification_status:** blocked
- **evidence_url:** https://ciac.fpsports.org/Directory.aspx?SchoolLevelID=1
- **implementation_notes:** Search results expose extensive sport-specific head-coach rows, but direct open redirects to https://ciac.fpsports.org/WebBotZone.aspx with Unauthorized Access and login request. Strongest apparent statewide coach source; not verified scrape-ready from this environment. Do not bypass. Public school staff websites/EdSight seeds are fallback.
- **priority:** P1
- **verified_at:** 2026-10-04
- **source_id:** SRC-076
- **shortlist_rank:** null
- **checked_date:** 2026-10-04
- **applicable_states:**
  - CT
- **original_source_id:** NE-028
- **observed_routes:**
  - (empty list)

### SRC-077

CAS-CIAC MobileDir legacy school list

- **state:** CT
- **name:** CAS-CIAC MobileDir legacy school list
- **url:** https://www.casciac.org/mobiledir/
- **category:** state_athletics_school_directory
- **sports_scope:** All sports
- **school_or_coach_fields:** School names,towns; school detail href identifiers
- **coverage:** CAS/CIAC school members; multiple grade levels
- **access_format:** HTML list; search.cgi?a=a&id=406 example
- **verification_status:** opened_verified
- **evidence_url:** https://www.casciac.org/mobiledir/
- **implementation_notes:** Opened CAS-CIAC public school-list entry point. IMPORTANT: this is a legacy list containing old/closed school names; reconcile with current EdSight inventory before use. School detail example opened near-empty text. Stronger sport-specific head-coach route is https://ciac.fpsports.org/Directory.aspx?SchoolLevelID=1 but direct access redirects to WebBotZone.aspx Unauthorized Access/login from this location. Do not bypass. Use verified current official school athletics/staff pages for appointments. MobileDir alone supplies neither a current coach roster nor current membership guarantee.
- **priority:** P1
- **verified_at:** 2026-10-04
- **source_id:** SRC-077
- **shortlist_rank:** 1
- **checked_date:** 2026-10-04
- **applicable_states:**
  - CT
- **original_source_id:** NE-029
- **observed_routes:**
  - (empty list)

### SRC-078

Connecticut EdSight school/district directory and exports

- **state:** CT
- **name:** Connecticut EdSight school/district directory and exports
- **url:** https://public-edsight.ct.gov/general-information
- **category:** state_education_directory
- **sports_scope:** All schools; sports not supplied
- **school_or_coach_fields:** School,district,type,location,export links; contacts separate
- **coverage:** Connecticut public and nonpublic school reporting universe
- **access_format:** HTML hub plus embedded reports/export to Excel
- **verification_status:** opened_verified
- **evidence_url:** https://public-edsight.ct.gov/general-information
- **implementation_notes:** Opened official hub. Observed https://public-edsight.ct.gov/overview/find-schools/find-schools-export and /overview/find-schools/find-school-district . Report notes distinguish nonpublic secondary schools and APSEPs. Use grades/status to seed athletics staff discovery; no documented JSON API verified. Source HTML of /overview/find-schools/find-school-district embeds https://edsight.ct.gov/SASStoredProcess/do with _program=/CTDOE/EdSight/Release/Reporting/Public/Reports/StoredProcesses/OrgSearchReport_SiteCore, orgtype/orgdistrict/orgname query fields. This is an observed embedded report route, not a documented JSON API; report response not tested.
- **priority:** P1
- **verified_at:** 2026-10-04
- **source_id:** SRC-078
- **shortlist_rank:** 2
- **checked_date:** 2026-10-04
- **applicable_states:**
  - CT
- **original_source_id:** NE-030
- **observed_routes:**
  - (empty list)

### SRC-079

Connecticut EdSight administrative contact search

- **state:** CT
- **name:** Connecticut EdSight administrative contact search
- **url:** https://public-edsight.ct.gov/overview/find-contacts
- **category:** state_education_contacts
- **sports_scope:** All schools
- **school_or_coach_fields:** Administrator/data/support contacts; email/export workflow
- **coverage:** Statewide education contacts
- **access_format:** Embedded report with export
- **verification_status:** opened_verified
- **evidence_url:** https://public-edsight.ct.gov/overview/find-contacts
- **implementation_notes:** Official hub says contacts update nightly. These are school administrative contacts, not a coach directory. Use to verify school domain and designated athletic contact only where explicit; do not substitute admin email as coach email.
- **priority:** P2
- **verified_at:** 2026-10-04
- **source_id:** SRC-079
- **shortlist_rank:** null
- **checked_date:** 2026-10-04
- **applicable_states:**
  - CT
- **original_source_id:** NE-031
- **observed_routes:**
  - (empty list)

### SRC-080

Connecticut High School Coaches Association committees

- **state:** CT
- **name:** Connecticut High School Coaches Association committees
- **url:** https://www.chsca.org/officers.html
- **category:** coaches_association
- **sports_scope:** XC,indoor,outdoor track plus other sports
- **school_or_coach_fields:** Sport committee representative names and schools
- **coverage:** 2026-27 association officers/sport representatives, small subset
- **access_format:** Weebly HTML
- **verification_status:** opened_verified
- **evidence_url:** https://www.chsca.org/officers.html
- **implementation_notes:** Explicit current XC/track representatives and school associations, not statewide member/coach roster. Keep association_role separate from coaching_role; confirm school employment before promoting to coach record.
- **priority:** P2
- **verified_at:** 2026-10-04
- **source_id:** SRC-080
- **shortlist_rank:** null
- **checked_date:** 2026-10-04
- **applicable_states:**
  - CT
- **original_source_id:** NE-032
- **observed_routes:**
  - (empty list)

### SRC-081

FCIAC 2021-22 coach directory

- **state:** CT
- **name:** FCIAC 2021-22 coach directory
- **url:** https://www.fciac.net/wp-content/uploads/sites/85/2022/09/FCIAC-directory-2021-22.pdf
- **category:** regional_coach_directory
- **sports_scope:** All sports including XC/track
- **school_or_coach_fields:** School,coach name,sport/role,email,phone
- **coverage:** Fairfield County Interscholastic Athletic Conference; historical 2021-22 edition
- **access_format:** Text PDF, 49 pages
- **verification_status:** opened_verified
- **evidence_url:** https://www.fciac.net/wp-content/uploads/sites/85/2022/09/FCIAC-directory-2021-22.pdf
- **implementation_notes:** Actually coach-bearing and contact-rich, but too old to assert current positions. Use historical names/school links only and revalidate on official school staff pages. Do not ingest unrelated personal/home fields.
- **priority:** P2
- **verified_at:** 2026-10-04
- **source_id:** SRC-081
- **shortlist_rank:** null
- **checked_date:** 2026-10-04
- **applicable_states:**
  - CT
- **original_source_id:** NE-033
- **observed_routes:**
  - (empty list)

### SRC-082

Vermont Principals Association athletics

- **state:** VT
- **name:** Vermont Principals Association athletics
- **url:** https://vpaonline.org/athletics/
- **category:** state_athletics
- **sports_scope:** XC,indoor,outdoor track and other sports
- **school_or_coach_fields:** Athletics links,rankings,tournament guides/programs; participation evidence
- **coverage:** Vermont member athletics
- **access_format:** WordPress HTML and linked guides
- **verification_status:** opened_verified
- **evidence_url:** https://vpaonline.org/athletics/
- **implementation_notes:** Official statewide sport source. https://vpaonline.org/athletic-guides-rules/ shows XC/indoor/outdoor titles but those titles were unlinked in extracted current HTML. Do not invent guide PDFs or claim statewide coach directory. Follow tournament/program links to team lists then schools.
- **priority:** P1
- **verified_at:** 2026-10-04
- **source_id:** SRC-082
- **shortlist_rank:** 1
- **checked_date:** 2026-10-04
- **applicable_states:**
  - VT
- **original_source_id:** NE-034
- **observed_routes:**
  - (empty list)

### SRC-083

Vermont AOE public school directory hub

- **state:** VT
- **name:** Vermont AOE public school directory hub
- **url:** https://education.vermont.gov/schools/school-operations/public-schools
- **category:** state_education_directory
- **sports_scope:** All public schools
- **school_or_coach_fields:** Directory of principals by school; superintendent directory; school/district map
- **coverage:** Vermont public schools
- **access_format:** HTML links to public directory resources
- **verification_status:** blocked
- **evidence_url:** https://education.vermont.gov/schools/school-operations/public-schools
- **implementation_notes:** Official indexed page identifies Directory of Principals by School; direct request returned 403. Reverify normal access later, or use Annual Snapshot organization directory. Principal is not track/XC coach.
- **priority:** P1
- **verified_at:** 2026-10-04
- **source_id:** SRC-083
- **shortlist_rank:** null
- **checked_date:** 2026-10-04
- **applicable_states:**
  - VT
- **original_source_id:** NE-035
- **observed_routes:**
  - (empty list)

### SRC-084

Vermont AOE independent school directory

- **state:** VT
- **name:** Vermont AOE independent school directory
- **url:** https://education.vermont.gov/documents/edu-independent-schools-directory
- **category:** state_education_private_directory
- **sports_scope:** All independent schools
- **school_or_coach_fields:** School/address/contact/phone/grades from directory
- **coverage:** Approved and recognized independent schools; includes non-HS/tutorial/program entries
- **access_format:** PDF download landing page
- **verification_status:** blocked
- **evidence_url:** https://education.vermont.gov/documents/edu-independent-schools-directory
- **implementation_notes:** Search index shows publication 2026-09-22 and filename edu-sy27-independent-school-directory.pdf. Landing direct fetch returned 403; exact current PDF href not verified, so do not guess. Filter grades 9-12 and school vs program; add public-school hub to cover both sectors.
- **priority:** P1
- **verified_at:** 2026-10-04
- **source_id:** SRC-084
- **shortlist_rank:** null
- **checked_date:** 2026-10-04
- **applicable_states:**
  - VT
- **original_source_id:** NE-036
- **observed_routes:**
  - (empty list)

### SRC-085

Vermont Annual Snapshot organization directory

- **state:** VT
- **name:** Vermont Annual Snapshot organization directory
- **url:** https://schoolsnapshot.vermont.gov/Organization/Directory
- **category:** state_education_directory
- **sports_scope:** All school organizations
- **school_or_coach_fields:** School/organization identity and locality; fields in rendered directory
- **coverage:** Vermont education organizations
- **access_format:** HTML/JS rendered directory
- **verification_status:** opened_verified
- **evidence_url:** https://schoolsnapshot.vermont.gov/Organization/Directory
- **implementation_notes:** Public page opens as directory shell but text extraction omits school rows; search indexed rows exist. Browser render needed to verify data route and fields. No JSON endpoint verified; do not confuse login UI at footer with evidence entire directory requires login.
- **priority:** P1
- **verified_at:** 2026-10-04
- **source_id:** SRC-085
- **shortlist_rank:** 3
- **checked_date:** 2026-10-04
- **applicable_states:**
  - VT
- **original_source_id:** NE-037
- **observed_routes:**
  - (empty list)

### SRC-086

MileSplit NY team directory

- **state:** NY
- **name:** MileSplit NY team directory
- **url:** https://ny.milesplit.com/teams
- **category:** sport_team_directory
- **sports_scope:** XC,indoor,outdoor track
- **school_or_coach_fields:** Team/school name,city,team page URL and platform ID
- **coverage:** State teams including HS,MS,college,clubs and historical/closed records
- **access_format:** Public HTML alphabetical directory; level filter
- **verification_status:** opened_verified
- **evidence_url:** https://ny.milesplit.com/teams
- **implementation_notes:** Opened state team listing without login. Filter to high school; cross-check current official school inventory and season. Team pages/coach contact presence varies and was not assumed. Do not scrape student rosters, athlete profiles or results. No public documented API verified.
- **priority:** P1
- **verified_at:** 2026-10-04
- **source_id:** SRC-086
- **shortlist_rank:** null
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NY
- **original_source_id:** NE-038
- **observed_routes:**
  - (empty list)

### SRC-087

MileSplit NJ team directory

- **state:** NJ
- **name:** MileSplit NJ team directory
- **url:** https://nj.milesplit.com/teams
- **category:** sport_team_directory
- **sports_scope:** XC,indoor,outdoor track
- **school_or_coach_fields:** Team/school name,city,team page URL and platform ID
- **coverage:** State teams including HS,MS,college,clubs and historical/closed records
- **access_format:** Public HTML alphabetical directory; level filter
- **verification_status:** opened_verified
- **evidence_url:** https://nj.milesplit.com/teams
- **implementation_notes:** Opened state team listing without login. Filter to high school; cross-check current official school inventory and season. Team pages/coach contact presence varies and was not assumed. Do not scrape student rosters, athlete profiles or results. No public documented API verified.
- **priority:** P1
- **verified_at:** 2026-10-04
- **source_id:** SRC-087
- **shortlist_rank:** 3
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NJ
- **original_source_id:** NE-039
- **observed_routes:**
  - (empty list)

### SRC-088

MileSplit MA team directory

- **state:** MA
- **name:** MileSplit MA team directory
- **url:** https://ma.milesplit.com/teams
- **category:** sport_team_directory
- **sports_scope:** XC,indoor,outdoor track
- **school_or_coach_fields:** Team/school name,city,team page URL and platform ID
- **coverage:** State teams including HS,MS,college,clubs and historical/closed records
- **access_format:** Public HTML alphabetical directory; level filter
- **verification_status:** opened_verified
- **evidence_url:** https://ma.milesplit.com/teams
- **implementation_notes:** Opened state team listing without login. Filter to high school; cross-check current official school inventory and season. Team pages/coach contact presence varies and was not assumed. Do not scrape student rosters, athlete profiles or results. No public documented API verified.
- **priority:** P1
- **verified_at:** 2026-10-04
- **source_id:** SRC-088
- **shortlist_rank:** null
- **checked_date:** 2026-10-04
- **applicable_states:**
  - MA
- **original_source_id:** NE-040
- **observed_routes:**
  - (empty list)

### SRC-089

MileSplit CT team directory

- **state:** CT
- **name:** MileSplit CT team directory
- **url:** https://ct.milesplit.com/teams
- **category:** sport_team_directory
- **sports_scope:** XC,indoor,outdoor track
- **school_or_coach_fields:** Team/school name,city,team page URL and platform ID
- **coverage:** State teams including HS,MS,college,clubs and historical/closed records
- **access_format:** Public HTML alphabetical directory; level filter
- **verification_status:** opened_verified
- **evidence_url:** https://ct.milesplit.com/teams
- **implementation_notes:** Opened state team listing without login. Filter to high school; cross-check current official school inventory and season. Team pages/coach contact presence varies and was not assumed. Do not scrape student rosters, athlete profiles or results. No public documented API verified.
- **priority:** P1
- **verified_at:** 2026-10-04
- **source_id:** SRC-089
- **shortlist_rank:** null
- **checked_date:** 2026-10-04
- **applicable_states:**
  - CT
- **original_source_id:** NE-041
- **observed_routes:**
  - (empty list)

### SRC-090

MileSplit VT team directory

- **state:** VT
- **name:** MileSplit VT team directory
- **url:** https://vt.milesplit.com/teams
- **category:** sport_team_directory
- **sports_scope:** XC,indoor,outdoor track
- **school_or_coach_fields:** Team/school name,city,team page URL and platform ID
- **coverage:** State teams including HS,MS,college,clubs and historical/closed records
- **access_format:** Public HTML alphabetical directory; level filter
- **verification_status:** opened_verified
- **evidence_url:** https://vt.milesplit.com/teams
- **implementation_notes:** Opened state team listing without login. Filter to high school; cross-check current official school inventory and season. Team pages/coach contact presence varies and was not assumed. Do not scrape student rosters, athlete profiles or results. No public documented API verified.
- **priority:** P1
- **verified_at:** 2026-10-04
- **source_id:** SRC-090
- **shortlist_rank:** 2
- **checked_date:** 2026-10-04
- **applicable_states:**
  - VT
- **original_source_id:** NE-042
- **observed_routes:**
  - (empty list)

### SRC-091

NEPSAC independent member schools

- **state:** NY
- **name:** NEPSAC independent member schools
- **url:** https://nepsac.org/about/nepsac-member-schools/
- **category:** independent_athletics_school_directory
- **sports_scope:** All school sports; XC/track through NEPSTA
- **school_or_coach_fields:** School name and direct official website
- **coverage:** NEPSAC member prep schools across New England and some NY; state filter needed
- **access_format:** HTML linked school list
- **verification_status:** opened_verified
- **evidence_url:** https://nepsac.org/about/nepsac-member-schools/
- **implementation_notes:** Shared NEPSAC school-directory system; see CT record for extraction. Restrict to this state and high-school grades.
- **priority:** P1
- **verified_at:** 2026-10-04
- **source_id:** SRC-091
- **shortlist_rank:** null
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NY
- **original_source_id:** NE-043
- **observed_routes:**
  - (empty list)

### SRC-092

NEPSAC independent member schools

- **state:** MA
- **name:** NEPSAC independent member schools
- **url:** https://nepsac.org/about/nepsac-member-schools/
- **category:** independent_athletics_school_directory
- **sports_scope:** All school sports; XC/track through NEPSTA
- **school_or_coach_fields:** School name and direct official website
- **coverage:** NEPSAC member prep schools across New England and some NY; state filter needed
- **access_format:** HTML linked school list
- **verification_status:** opened_verified
- **evidence_url:** https://nepsac.org/about/nepsac-member-schools/
- **implementation_notes:** Shared NEPSAC school-directory system; see CT record for extraction. Restrict to this state and high-school grades.
- **priority:** P1
- **verified_at:** 2026-10-04
- **source_id:** SRC-092
- **shortlist_rank:** null
- **checked_date:** 2026-10-04
- **applicable_states:**
  - MA
- **original_source_id:** NE-044
- **observed_routes:**
  - (empty list)

### SRC-093

NEPSAC independent member schools

- **state:** CT
- **name:** NEPSAC independent member schools
- **url:** https://nepsac.org/about/nepsac-member-schools/
- **category:** independent_athletics_school_directory
- **sports_scope:** All school sports; XC/track through NEPSTA
- **school_or_coach_fields:** School name and direct official website
- **coverage:** NEPSAC member prep schools across New England and some NY; state filter needed
- **access_format:** HTML linked school list
- **verification_status:** opened_verified
- **evidence_url:** https://nepsac.org/about/nepsac-member-schools/
- **implementation_notes:** Public list has direct school websites and covers institutions outside state associations. Filter state and upper-school grades; some schools are middle-only. Follow athletics > teams/coaches/staff; members do not all sponsor XC/track. No statewide coach roster implied.
- **priority:** P1
- **verified_at:** 2026-10-04
- **source_id:** SRC-093
- **shortlist_rank:** 3
- **checked_date:** 2026-10-04
- **applicable_states:**
  - CT
- **original_source_id:** NE-045
- **observed_routes:**
  - (empty list)

### SRC-094

NEPSAC independent member schools

- **state:** VT
- **name:** NEPSAC independent member schools
- **url:** https://nepsac.org/about/nepsac-member-schools/
- **category:** independent_athletics_school_directory
- **sports_scope:** All school sports; XC/track through NEPSTA
- **school_or_coach_fields:** School name and direct official website
- **coverage:** NEPSAC member prep schools across New England and some NY; state filter needed
- **access_format:** HTML linked school list
- **verification_status:** opened_verified
- **evidence_url:** https://nepsac.org/about/nepsac-member-schools/
- **implementation_notes:** Shared NEPSAC school-directory system; see CT record for extraction. Restrict to this state and high-school grades.
- **priority:** P1
- **verified_at:** 2026-10-04
- **source_id:** SRC-094
- **shortlist_rank:** null
- **checked_date:** 2026-10-04
- **applicable_states:**
  - VT
- **original_source_id:** NE-046
- **observed_routes:**
  - (empty list)

### SRC-095

NEPSTA cross country/track coaches association

- **state:** MA,CT,VT,NY
- **name:** NEPSTA cross country/track coaches association
- **url:** https://nepsac.org/coaches-associations/boys-girls-sports-2/boys-girls-cross-country-track-nepsta/
- **category:** independent_coaches_association
- **sports_scope:** Boys/girls XC and track
- **school_or_coach_fields:** 2026-27 executive-board names/emails; divisions and meeting resources
- **coverage:** New England preparatory track association
- **access_format:** HTML plus linked PDF/minutes
- **verification_status:** opened_verified
- **evidence_url:** https://nepsac.org/coaches-associations/boys-girls-sports-2/boys-girls-cross-country-track-nepsta/
- **implementation_notes:** Verified current president/VP/secretary and meeting documents. Only association officers are exposed; no full coaching roster. Current division headings were plain text in reader, so verify actual link availability before implementing downloads.
- **priority:** P2
- **verified_at:** 2026-10-04
- **source_id:** SRC-095
- **shortlist_rank:** null
- **checked_date:** 2026-10-04
- **applicable_states:**
  - MA
  - CT
  - VT
  - NY
- **original_source_id:** NE-047
- **observed_routes:**
  - (empty list)

### SRC-096

Home Campus New Jersey selector (untested NJ data lead)

- **source_id:** SRC-096
- **state:** NJ
- **name:** Home Campus New Jersey selector (untested NJ data lead)
- **url:** https://www.cifsshome.org/widget/school/directory?school_id=19&section_id=1
- **category:** supplemental_untested_lead
- **sports_scope:** School sports including XC/track; NJ coverage not tested
- **school_or_coach_fields:** Shared page code supports coach names, sport, role, email; NJ rows not verified
- **coverage:** New Jersey selector exists; no NJ school sample validated
- **access_format:** Public HTML widget with AJAX JSON routes
- **verification_status:** opened_verified
- **evidence_url:** https://www.cifsshome.org/widget/school/directory?school_id=19&section_id=1
- **implementation_notes:** Direct GET 200 verified dropdown New Jersey value 12. Do not treat CA sample as NJ data. Form field is section; merely appending section_id=12 without a valid school did not select NJ. Observed read routes /widget/schools/get with school,section_id,status=active,hide_from_directory=0 and /widget/get-school-details/{observed_id}/details. Not tested for NJ. Honor hide_from_directory and never enumerate hidden IDs. This is observed site code, not documented supported API.
- **priority:** P3
- **verified_at:** 2026-10-04
- **shortlist_rank:** null
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NJ
- **original_source_id:** NE-048
- **observed_routes:**
  - (empty list)

### SRC-097

OHSAA current enrollment seed table

- **state:** OH
- **name:** OHSAA current enrollment seed table
- **url:** https://www.ohsaa.org/school-resources/school-enrollment
- **category:** state_athletics_school_seed
- **sports_scope:** high school track & field and cross country
- **school_or_coach_fields:** OhsaaSchoolId; school name; city; athletic district; sex-specific classification
- **coverage:** OHSAA member high schools, public and nonpublic
- **access_format:** HTML table
- **verification_status:** opened_verified
- **evidence_url:** https://www.ohsaa.org/school-resources/school-enrollment
- **implementation_notes:** 2026-27/2027-28 table. Use its published IDs to feed public myOHSAA SportsInformation pages; do not brute-force IDs. Enrollment is aggregate school metadata, not student records.
- **priority:** P0
- **source_system:** OHSAA
- **shortlist_rank:** null
- **verification_date:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - OH
- **source_id:** SRC-097
- **observed_routes:**
  - (empty list)

### SRC-098

myOHSAA public sport-by-school coach directory

- **state:** OH
- **name:** myOHSAA public sport-by-school coach directory
- **url:** https://officials.myohsaa.org/Outside/Schedule/SportsInformation?ohsaaId=582
- **category:** state_athletics_coach_directory
- **sports_scope:** high school track & field and cross country
- **school_or_coach_fields:** school ID/name/address; sport; boys/girls head coach; public email when supplied; division; season
- **coverage:** Member schools with supplied sports staff
- **access_format:** server-rendered HTML table
- **verification_status:** opened_verified
- **evidence_url:** https://officials.myohsaa.org/Outside/Schedule/SportsInformation?ohsaaId=582
- **implementation_notes:** Exact working host is officials.myohsaa.org. Bishop Fenwick sample actually rendered 2026-27 XC and track coaches/emails. Parse N/A as sport not offered and TBA as vacant/unknown. Keep separate gender appointments even when person is shared. Pair with OHSAA enrollment seed.
- **priority:** P0
- **source_system:** OHSAA
- **shortlist_rank:** 1
- **verification_date:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - OH
- **source_id:** SRC-098
- **observed_routes:**
  - (empty list)

### SRC-099

myOHSAA public school identity detail

- **state:** OH
- **name:** myOHSAA public school identity detail
- **url:** https://officials.myohsaa.org/Outside/Schedule?ohsaaId=582
- **category:** state_athletics_school_detail
- **sports_scope:** high school track & field and cross country
- **school_or_coach_fields:** school name/ID; address; school/athletic department navigation
- **coverage:** Same public myOHSAA school universe
- **access_format:** HTML
- **verification_status:** opened_verified
- **evidence_url:** https://officials.myohsaa.org/Outside/Schedule?ohsaaId=582
- **implementation_notes:** Discovered by clicking School Information on the actual coach sample. Follow actual athletic-department links for adult administration fallback; preserve OHSAA ID.
- **priority:** P1
- **source_system:** OHSAA
- **shortlist_rank:** null
- **verification_date:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - OH
- **source_id:** SRC-099
- **observed_routes:**
  - (empty list)

### SRC-100

OATCCC current coach membership

- **state:** OH
- **name:** OATCCC current coach membership
- **url:** https://www.oatccc.com/Coaches/Membership/
- **category:** coaches_association_membership
- **sports_scope:** high school track & field and cross country
- **school_or_coach_fields:** first name; last name; school; county
- **coverage:** Association members, including lifetime members and some missing/unaffiliated schools; not all Ohio coaches
- **access_format:** HTML landing plus public Google Sheet
- **verification_status:** opened_verified
- **evidence_url:** https://www.oatccc.com/Coaches/Membership/
- **implementation_notes:** 2026 sheet opened and rows verified: https://docs.google.com/spreadsheets/d/1i8mbrZolrxSxJN8jOTOWfnjNEMR38RVd/edit?gid=335617153&rtpof=true&sd=true . No emails or head/assistant role columns observed. Membership is a candidate signal; confirm current HS appointment locally.
- **priority:** P0
- **source_system:** OATCCC
- **shortlist_rank:** 2
- **verification_date:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - OH
- **source_id:** SRC-100
- **observed_routes:**
  - (empty list)

### SRC-101

Ohio Educational Directory System public extract

- **state:** OH
- **name:** Ohio Educational Directory System public extract
- **url:** https://oeds.education.ohio.gov/dataextract
- **category:** education_department_school_seed
- **sports_scope:** high school track & field and cross country
- **school_or_coach_fields:** organization type; grades served; NCES school ID; selectable organization and role fields
- **coverage:** Public, charter/community, STEM and chartered nonpublic schools
- **access_format:** public report-generation form
- **verification_status:** opened_verified
- **evidence_url:** https://oeds.education.ohio.gov/dataextract
- **implementation_notes:** Choose school-level types and high-school grade range. Public extract form observed; export submission was not exercised. No documented REST API verified. Join NCES/local identifiers to school sites; role exports are not automatically coach lists.
- **priority:** P0
- **source_system:** Ohio DEW OEDS
- **shortlist_rank:** 3
- **verification_date:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - OH
- **source_id:** SRC-101
- **observed_routes:**
  - (empty list)

### SRC-102

Ohio nonchartered nonpublic school list

- **state:** OH
- **name:** Ohio nonchartered nonpublic school list
- **url:** https://education.ohio.gov/Topics/Ohio-Education-Options/Private-Schools/Non-Chartered-Non-Tax-School-Information
- **category:** education_department_private_school_seed
- **sports_scope:** high school track & field and cross country
- **school_or_coach_fields:** nonchartered school identity from annual list; file columns not inspected
- **coverage:** Religious nonchartered schools missing from chartered OEDS coverage
- **access_format:** HTML with annual download links
- **verification_status:** opened_verified
- **evidence_url:** https://education.ohio.gov/Topics/Ohio-Education-Options/Private-Schools/Non-Chartered-Non-Tax-School-Information
- **implementation_notes:** Landing page offers 2025-26 list and 1984-2025 archive. Latest notification window is 2026-27, but current posted list remains 2025-26. Filter actual grades before adding; confirm whether they offer HS sports.
- **priority:** P1
- **source_system:** Ohio DEW NCNP
- **shortlist_rank:** null
- **verification_date:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - OH
- **source_id:** SRC-102
- **observed_routes:**
  - (empty list)

### SRC-103

OATCCC leadership and regional representatives

- **state:** OH
- **name:** OATCCC leadership and regional representatives
- **url:** https://www.oatccc.com/About-Us/Our-Leadership/
- **category:** coaches_association_partial_contacts
- **sports_scope:** high school track & field and cross country
- **school_or_coach_fields:** officer/representative names and association roles; validate school/current appointment
- **coverage:** Association leadership subset only
- **access_format:** HTML
- **verification_status:** opened_verified
- **evidence_url:** https://www.oatccc.com/About-Us/Our-Leadership/
- **implementation_notes:** Useful targeted enrichment, not a statewide member directory. Prefer 2026 membership sheet for broader coach candidates; avoid marking every association officer as a currently employed coach.
- **priority:** P2
- **source_system:** OATCCC
- **shortlist_rank:** null
- **verification_date:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - OH
- **source_id:** SRC-103
- **observed_routes:**
  - (empty list)

### SRC-104

Greater Catholic League Coed

- **state:** OH
- **name:** Greater Catholic League Coed
- **url:** https://gclc.gclsports.com/
- **category:** private_parochial_league
- **sports_scope:** high school track & field and cross country
- **school_or_coach_fields:** school names and athletics links in indexed page; detailed coach fields unverified
- **coverage:** Six Cincinnati-area coed Catholic member high schools
- **access_format:** HTML, currently fetch blocked
- **verification_status:** blocked
- **evidence_url:** https://gclc.gclsports.com/
- **implementation_notes:** Search index explicitly lists Alter, Badin, Carroll, Chaminade-Julienne, Fenwick, McNicholas. Direct open returned 403. Sibling boys route https://gcls.gclsports.com/index.aspx and girls https://ggcl.gclsports.com/ also 403; treat as leads, not verified extraction endpoints.
- **priority:** P2
- **source_system:** Greater Catholic Leagues
- **shortlist_rank:** null
- **verification_date:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - OH
- **source_id:** SRC-104
- **observed_routes:**
  - (empty list)

### SRC-105

OHSAA Central District cross-country school assignments

- **state:** OH
- **name:** OHSAA Central District cross-country school assignments
- **url:** https://www.ohsaa.org/Central-Sports-Tournaments/Cross-Country
- **category:** regional_sport_school_seed
- **sports_scope:** high school track & field and cross country
- **school_or_coach_fields:** participating school names by tournament/division; organizer contacts are not team coaches
- **coverage:** Central Ohio XC teams
- **access_format:** HTML
- **verification_status:** search_only
- **evidence_url:** https://www.ohsaa.org/Central-Sports-Tournaments/Cross-Country
- **implementation_notes:** Indexed school lists include public and private teams. Search result verified only in this pass; direct open not completed. Useful sport-participation corroboration, not staff directory.
- **priority:** P2
- **source_system:** OHSAA
- **shortlist_rank:** null
- **verification_date:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - OH
- **source_id:** SRC-105
- **observed_routes:**
  - (empty list)

### SRC-106

MHSAA public school directory and coach tabs

- **state:** MI
- **name:** MHSAA public school directory and coach tabs
- **url:** https://www.mhsaa.com/schools
- **category:** state_athletics_coach_directory
- **sports_scope:** high school track & field and cross country
- **school_or_coach_fields:** school ID; name; city; school level; website/address/phone; sports; coach names by gender and sport
- **coverage:** Michigan member/participating schools, including private; search also contains junior-high/nonmember-like entries
- **access_format:** JavaScript-rendered public directory
- **verification_status:** opened_verified
- **evidence_url:** https://www.mhsaa.com/schools/novi
- **implementation_notes:** Cloud browser search Novi returned HS/JH rows. Sample https://www.mhsaa.com/schools/novi → Staff → Coaches publicly rendered boys/girls XC and track names; no coach emails visible. Profile School ID 1833 differs from internal calendar SchoolId 3933. Keep source IDs separately. No data API verified.
- **priority:** P0
- **source_system:** MHSAA
- **shortlist_rank:** 1
- **verification_date:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - MI
- **source_id:** SRC-106
- **observed_routes:**
  - (empty list)

### SRC-107

Michigan EEM public school data export

- **state:** MI
- **name:** Michigan EEM public school data export
- **url:** https://cepi.state.mi.us/eem/PublicDatasets.aspx
- **category:** education_department_school_seed
- **sports_scope:** high school track & field and cross country
- **school_or_coach_fields:** entity IDs; school/facility names; grades; addresses; contact information
- **coverage:** Public LEA/PSA/ISD/state schools and nonpublic schools
- **access_format:** ASP.NET public data export form
- **verification_status:** opened_verified
- **evidence_url:** https://cepi.state.mi.us/eem/PublicDatasets.aspx
- **implementation_notes:** Choose school entity types, then filter grades. Download-format control and column-description PDF observed. Public export guidance explicitly says login is unnecessary: https://www.michigan.gov/cepi/pk-12/eem/creating-lists-and-mailing-labels . Contact exports include administrator information, not a sport assignment guarantee.
- **priority:** P0
- **source_system:** Michigan CEPI EEM
- **shortlist_rank:** 2
- **verification_date:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - MI
- **source_id:** SRC-107
- **observed_routes:**
  - (empty list)

### SRC-108

MITCA track/XC coaches association

- **state:** MI
- **name:** MITCA track/XC coaches association
- **url:** https://mitca.org/MITCA/
- **category:** coaches_association
- **sports_scope:** high school track & field and cross country
- **school_or_coach_fields:** association contacts; regional coach award routes; school associations from award material where present
- **coverage:** Statewide association with public enrichment limited to exposed contacts/awards
- **access_format:** WordPress HTML and linked documents
- **verification_status:** opened_verified
- **evidence_url:** https://mitca.org/MITCA/
- **implementation_notes:** Correct working path is mitca.org/MITCA/. www.mitca.org root failed in web tool. Site announces a move toward https://runsignup.com/w/MITCAMeets (linked, not opened). Contacts: https://mitca.org/MITCA/about-mitca/mitca-committees-contacts/; awards: https://mitca.org/MITCA/mitca-awards/regional-coach-of-the-year/ both opened. No comprehensive public member directory verified.
- **priority:** P1
- **source_system:** MITCA
- **shortlist_rank:** 3
- **verification_date:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - MI
- **source_id:** SRC-108
- **observed_routes:**
  - (empty list)

### SRC-109

Michigan approved nonpublic schools

- **state:** MI
- **name:** Michigan approved nonpublic schools
- **url:** https://www.michigan.gov/mde/Services/flexible-learning/options/nonpub-home
- **category:** education_department_private_school_seed
- **sports_scope:** high school track & field and cross country
- **school_or_coach_fields:** approved private-school identity; school IDs; annual list metadata
- **coverage:** Approved nonpublic Michigan schools, all grades
- **access_format:** HTML plus annual lists
- **verification_status:** opened_verified
- **evidence_url:** https://www.michigan.gov/mde/Services/flexible-learning/options/nonpub-home
- **implementation_notes:** 2025-26 final A-Z and ISD listings are exposed. Restrict to secondary-grade institutions; use to audit private-school gaps in athletics directories. Do not enter personnel/student reporting systems.
- **priority:** P1
- **source_system:** Michigan MDE nonpublic
- **shortlist_rank:** null
- **verification_date:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - MI
- **source_id:** SRC-109
- **observed_routes:**
  - (empty list)

### SRC-110

Catholic High School League member schools

- **state:** MI
- **name:** Catholic High School League member schools
- **url:** https://www.chsl.com/about/current-member-schools
- **category:** private_parochial_league
- **sports_scope:** high school track & field and cross country
- **school_or_coach_fields:** member school names; school detail links
- **coverage:** Southeast Michigan private schools plus listed Ohio members
- **access_format:** HTML
- **verification_status:** opened_verified
- **evidence_url:** https://www.chsl.com/about/current-member-schools
- **implementation_notes:** Current list includes Toledo schools: infer state from school address, not league headquarters. Sport routes https://www.chsl.com/sports/cross-country and https://www.chsl.com/sports/track opened. Follow school detail/official athletics site for coaches; league leadership is not school coaching staff.
- **priority:** P1
- **source_system:** CHSL
- **shortlist_rank:** null
- **verification_date:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - MI
- **source_id:** SRC-110
- **observed_routes:**
  - (empty list)

### SRC-111

MHSAA leagues and conferences search

- **state:** MI
- **name:** MHSAA leagues and conferences search
- **url:** https://my.mhsaa.com/About-the-MHSAA/Leagues-Conferences
- **category:** regional_league_index
- **sports_scope:** high school track & field and cross country
- **school_or_coach_fields:** league name; contact-name/email/phone/address template; membership-detail link
- **coverage:** Michigan leagues/conferences
- **access_format:** JavaScript search form
- **verification_status:** opened_verified
- **evidence_url:** https://my.mhsaa.com/About-the-MHSAA/Leagues-Conferences
- **implementation_notes:** HTML exposed raw template placeholders, not populated league records. Query by league, school/city or zone using rendered UI. No JSON endpoint or complete league extract verified.
- **priority:** P2
- **source_system:** MHSAA
- **shortlist_rank:** null
- **verification_date:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - MI
- **source_id:** SRC-111
- **observed_routes:**
  - (empty list)

### SRC-112

Michigan High School Coaches Association awards

- **state:** MI
- **name:** Michigan High School Coaches Association awards
- **url:** https://www.mhsca.org/
- **category:** coaches_association_partial_contacts
- **sports_scope:** high school track & field and cross country
- **school_or_coach_fields:** coach name; sport; school from annual honors
- **coverage:** Awarded coaches only, multiple sports
- **access_format:** HTML
- **verification_status:** search_only
- **evidence_url:** https://www.mhsca.org/
- **implementation_notes:** Search index shows 2026 boys/girls XC coach-of-year entries. Direct page not opened in this pass. Filter sport/year and confirm current school staff before importing.
- **priority:** P2
- **source_system:** MHSCA
- **shortlist_rank:** null
- **verification_date:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - MI
- **source_id:** SRC-112
- **observed_routes:**
  - (empty list)

### SRC-113

Indiana DOE downloadable school directory

- **state:** IN
- **name:** Indiana DOE downloadable school directory
- **url:** https://www.in.gov/doe/it/data-center-and-reports/
- **category:** education_department_school_seed
- **sports_scope:** high school track & field and cross country
- **school_or_coach_fields:** school/corporation directory records; workbook columns require implementation inspection
- **coverage:** Indiana public and accredited nonpublic schools; confirm individual workbook sheet coverage
- **access_format:** HTML landing and XLSX
- **verification_status:** opened_verified
- **evidence_url:** https://www.in.gov/doe/it/data-center-and-reports/
- **implementation_notes:** Exact download href verified in page HTML: https://www.in.gov/doe/files/2025-2026-school-directory-2026-03-23.xlsx . A 447,924-byte XLSX downloaded locally; sheet fields not validated in this pass. Landing labels update 4/1/2026. Use grade fields to restrict HS; no coach role assumed.
- **priority:** P0
- **source_system:** Indiana DOE
- **shortlist_rank:** 1
- **verification_date:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - IN
- **source_id:** SRC-113
- **observed_routes:**
  - (empty list)

### SRC-114

Indiana IHSAA member school directory

- **state:** IN
- **name:** Indiana IHSAA member school directory
- **url:** https://www.ihsaa.org/schools/ihsaa-school-directory
- **category:** state_athletics_school_seed
- **sports_scope:** high school track & field and cross country
- **school_or_coach_fields:** member school universe; public/nonpublic membership totals; myIHSAA contact/profile links
- **coverage:** 413 members shown for 2026-27: 356 public and 57 nonpublic, including provisional schools
- **access_format:** HTML hub and JavaScript myIHSAA application
- **verification_status:** opened_verified
- **evidence_url:** https://www.ihsaa.org/schools/ihsaa-school-directory
- **implementation_notes:** Hub directs to https://www.myihsaa.net/ . Application opened as JS shell, with no individual directory rows verified. Official /schools page says directory has contacts/directions/profiles. Do not claim coach-email coverage or invent API routes. Public conference list is a fallback seed.
- **priority:** P0
- **source_system:** Indiana IHSAA
- **shortlist_rank:** 2
- **verification_date:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - IN
- **source_id:** SRC-114
- **observed_routes:**
  - (empty list)

### SRC-115

IATCCC officers and school affiliations

- **state:** IN
- **name:** IATCCC officers and school affiliations
- **url:** https://iatccc.org/officers-and-council/
- **category:** coaches_association_partial_contacts
- **sports_scope:** high school track & field and cross country
- **school_or_coach_fields:** current officer name; school; email; historical officer years
- **coverage:** Four 2025-26 officers plus historical entries; not statewide membership
- **access_format:** HTML tables
- **verification_status:** opened_verified
- **evidence_url:** https://iatccc.org/officers-and-council/
- **implementation_notes:** Separate current officers table from historical table. Main https://iatccc.org/ provides recent clinic/coach-award enrichment. Membership signup is not a public membership database; no statewide searchable member list/API verified.
- **priority:** P1
- **source_system:** IATCCC
- **shortlist_rank:** 3
- **verification_date:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - IN
- **source_id:** SRC-115
- **observed_routes:**
  - (empty list)

### SRC-116

Indiana IHSAA current athletic conferences

- **state:** IN
- **name:** Indiana IHSAA current athletic conferences
- **url:** https://www.ihsaa.org/schools/athletic-conferences
- **category:** regional_league_index
- **sports_scope:** high school track & field and cross country
- **school_or_coach_fields:** conference names; member schools; independents; upcoming membership changes
- **coverage:** Statewide IHSAA conference and independent school coverage
- **access_format:** HTML
- **verification_status:** opened_verified
- **evidence_url:** https://www.ihsaa.org/schools/athletic-conferences
- **implementation_notes:** Strong fallback seed when myIHSAA JS directory is inaccessible. Distinguish football-only memberships and future moves from current XC/track affiliations; don't assign sports solely from league membership.
- **priority:** P1
- **source_system:** Indiana IHSAA
- **shortlist_rank:** null
- **verification_date:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - IN
- **source_id:** SRC-116
- **observed_routes:**
  - (empty list)

### SRC-117

Indiana Eventlink XC school search

- **state:** IN
- **name:** Indiana Eventlink XC school search
- **url:** https://ihsaa.eventlink.com/SchoolSchedules/Schools/b7630efd-2350-455a-b82b-6ebbac635204
- **category:** sport_participation_school_seed
- **sports_scope:** high school track & field and cross country
- **school_or_coach_fields:** school; city; gender; varsity/JV level after public search
- **coverage:** IHSAA XC teams in Eventlink
- **access_format:** HTML search form/JavaScript
- **verification_status:** opened_verified
- **evidence_url:** https://ihsaa.eventlink.com/SchoolSchedules/Schools/b7630efd-2350-455a-b82b-6ebbac635204
- **implementation_notes:** Observed public XC search UI and column names, but results not submitted. Discover other sports through https://ihsaa.eventlink.com navigation rather than assuming GUIDs. Not a coach directory.
- **priority:** P1
- **source_system:** Indiana IHSAA Eventlink
- **shortlist_rank:** null
- **verification_date:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - IN
- **source_id:** SRC-117
- **observed_routes:**
  - (empty list)

### SRC-118

Indiana Association of Christian Schools members

- **state:** IN
- **name:** Indiana Association of Christian Schools members
- **url:** https://www.indianaacs.org/member-schools.html
- **category:** private_parochial_school_seed
- **sports_scope:** high school track & field and cross country
- **school_or_coach_fields:** school names; location/contact blocks; official website links
- **coverage:** Christian association member schools, mixed grade levels
- **access_format:** HTML
- **verification_status:** opened_verified
- **evidence_url:** https://www.indianaacs.org/member-schools.html
- **implementation_notes:** Useful coverage of smaller religious schools. Filter school grade range and actual XC/track offering; association membership itself does not establish an athletic program.
- **priority:** P1
- **source_system:** IACS
- **shortlist_rank:** null
- **verification_date:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - IN
- **source_id:** SRC-118
- **observed_routes:**
  - (empty list)

### SRC-119

Hoosier Crossroads Conference member contacts

- **state:** IN
- **name:** Hoosier Crossroads Conference member contacts
- **url:** https://hoosiercrossroadsconference.org/member-school-contacts
- **category:** regional_league_school_contacts
- **sports_scope:** high school track & field and cross country
- **school_or_coach_fields:** school; address; athletic director/assistant AD; phone; athletics-site links
- **coverage:** Eight central-Indiana public high schools
- **access_format:** HTML
- **verification_status:** opened_verified
- **evidence_url:** https://hoosiercrossroadsconference.org/member-school-contacts
- **implementation_notes:** Ready school-to-athletics-domain discovery. ADs are contact fallbacks, not sport coaches. Some repeated heading/AD values appear in page; normalize cards carefully and confirm current staff locally.
- **priority:** P1
- **source_system:** Hoosier Crossroads Conference
- **shortlist_rank:** null
- **verification_date:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - IN
- **source_id:** SRC-119
- **observed_routes:**
  - (empty list)

### SRC-120

IHSA new public school directory

- **state:** IL
- **name:** IHSA new public school directory
- **url:** https://www.ihsa.org/schools/school-directory
- **category:** state_athletics_school_seed
- **sports_scope:** high school track & field and cross country
- **school_or_coach_fields:** school ID; website; address; phone; county; type; conference; entered sports and class
- **coverage:** Public and private IHSA schools
- **access_format:** JavaScript browser directory
- **verification_status:** opened_verified
- **evidence_url:** https://www.ihsa.org/schools/details/0235
- **implementation_notes:** Live browser sample https://www.ihsa.org/schools/details/0235 verified Heritage school and all four XC/track gender entries. Staff panel failed with 'We couldn't load the staff directory' after retry; do not claim current coach contacts verified. Old /data/school/schools/0235.htm returned 404. No data API verified.
- **priority:** P0
- **source_system:** IHSA
- **shortlist_rank:** 1
- **verification_date:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - IL
- **source_id:** SRC-120
- **observed_routes:**
  - (empty list)

### SRC-121

IHSA head-coach season-summary database

- **state:** IL
- **name:** IHSA head-coach season-summary database
- **url:** https://www.ihsa.org/data/ccb/records/index.htm
- **category:** state_athletics_coach_history
- **sports_scope:** high school track & field and cross country
- **school_or_coach_fields:** school; season; head-coach name; sport/gender from record collection
- **coverage:** Submitted school seasons and tournament records, incomplete statewide current-year coverage
- **access_format:** server-rendered HTML index and tables
- **verification_status:** opened_verified
- **evidence_url:** https://www.ihsa.org/data/ccb/records/index.htm
- **implementation_notes:** Sample https://www.ihsa.org/data/ccb/records/sum-a.htm opened. Also opened girls XC https://www.ihsa.org/data/ccg/records/index.htm, boys track https://www.ihsa.org/data/trb/records/index.htm, girls track https://www.ihsa.org/data/trg/records/index.htm. Follow actual alphabetic links. Preserve season; updated page timestamp does not make every row current. No emails.
- **priority:** P0
- **source_system:** IHSA
- **shortlist_rank:** null
- **verification_date:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - IL
- **source_id:** SRC-121
- **observed_routes:**
  - (empty list)

### SRC-122

ISBE nightly public/private school directory

- **state:** IL
- **name:** ISBE nightly public/private school directory
- **url:** https://www.isbe.net/Pages/Data-Analysis-Directories.aspx
- **category:** education_department_school_seed
- **sports_scope:** high school track & field and cross country
- **school_or_coach_fields:** entity name; administrator; address/contact; grades; RCDTS; NCES ID
- **coverage:** All public/nonpublic K-12 education entities known to ISBE
- **access_format:** HTML landing plus Excel
- **verification_status:** opened_verified
- **evidence_url:** https://www.isbe.net/Pages/Data-Analysis-Directories.aspx
- **implementation_notes:** Exact href verified in live HTML: https://www.isbe.net/_layouts/Download.aspx?SourceUrl=/Documents/dir_ed_entities.xls . Landing says updated nightly. Use Public Sch and Dist and Non Pub Sch tabs, filter HS grades and school entity type. No coach roles assumed. Download body not verified; landing has generic archived-page footer despite current links.
- **priority:** P0
- **source_system:** Illinois ISBE
- **shortlist_rank:** 2
- **verification_date:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - IL
- **source_id:** SRC-122
- **observed_routes:**
  - (empty list)

### SRC-123

ITCCCA officers and coaching awards

- **state:** IL
- **name:** ITCCCA officers and coaching awards
- **url:** https://www.itccca.com/itccca-officers
- **category:** coaches_association_partial_contacts
- **sports_scope:** high school track & field and cross country
- **school_or_coach_fields:** coach/officer name; school; association responsibility
- **coverage:** Current association officers/coordinators, not full membership
- **access_format:** HTML tables
- **verification_status:** opened_verified
- **evidence_url:** https://www.itccca.com/itccca-officers
- **implementation_notes:** Main https://www.itccca.com/ opened and exposes Coach of the Year and Assistant Coach of the Year award sections. Useful named-coach and assistant-coach enrichment, but confirm current appointment and no public full membership API verified.
- **priority:** P1
- **source_system:** ITCCCA
- **shortlist_rank:** 3
- **verification_date:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - IL
- **source_id:** SRC-123
- **observed_routes:**
  - (empty list)

### SRC-124

IHSA public mobile school search

- **state:** IL
- **name:** IHSA public mobile school search
- **url:** https://center.ihsa.org/go/mobile/app/sch-1-dir.asp
- **category:** state_athletics_school_directory
- **sports_scope:** high school track & field and cross country
- **school_or_coach_fields:** public school-name search; result/detail fields unverified
- **coverage:** IHSA schools
- **access_format:** HTML form
- **verification_status:** opened_verified
- **evidence_url:** https://center.ihsa.org/go/mobile/app/sch-1-dir.asp
- **implementation_notes:** Search form opened without login; visible Login link alone is not proof results require login. Search submission not tested. Alternate entry point if new directory remains fragile, not a claimed documented API.
- **priority:** P2
- **source_system:** IHSA
- **shortlist_rank:** null
- **verification_date:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - IL
- **source_id:** SRC-124
- **observed_routes:**
  - (empty list)

### SRC-125

IHSA conference directory

- **state:** IL
- **name:** IHSA conference directory
- **url:** https://www.ihsa.org/data/school/conf.htm
- **category:** regional_league_index
- **sports_scope:** high school track & field and cross country
- **school_or_coach_fields:** conference; member schools; conference contact; conference website
- **coverage:** Statewide conference associations
- **access_format:** HTML
- **verification_status:** opened_verified
- **evidence_url:** https://www.ihsa.org/data/school/conf.htm
- **implementation_notes:** Opened table/list with school membership and external league URLs. Useful athletics-domain and conference discovery. Conference contacts are not automatically coaches; membership may be sport-specific.
- **priority:** P1
- **source_system:** IHSA
- **shortlist_rank:** null
- **verification_date:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - IL
- **source_id:** SRC-125
- **observed_routes:**
  - (empty list)

### SRC-126

Chicago Catholic League member schools

- **state:** IL
- **name:** Chicago Catholic League member schools
- **url:** https://www.chicagocatholicleague.com/
- **category:** private_parochial_league
- **sports_scope:** high school track & field and cross country
- **school_or_coach_fields:** member school names and school links; sport sections
- **coverage:** Chicago-area Catholic league members
- **access_format:** HTML/image-linked member carousel
- **verification_status:** opened_verified
- **evidence_url:** https://www.chicagocatholicleague.com/
- **implementation_notes:** Member names frequently live in image alt text rather than ordinary anchor text. Extract alt + href together. Follow actual XC/track or school athletics links. Check current membership against IHSA directory because alignments change.
- **priority:** P1
- **source_system:** Chicago Catholic League
- **shortlist_rank:** null
- **verification_date:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - IL
- **source_id:** SRC-126
- **observed_routes:**
  - (empty list)

### SRC-127

ISBE nonpublic registration/recognition

- **state:** IL
- **name:** ISBE nonpublic registration/recognition
- **url:** https://www.isbe.net/nonpublicprograms
- **category:** education_department_private_school_seed
- **sports_scope:** high school track & field and cross country
- **school_or_coach_fields:** registered/recognized nonpublic school annual lists
- **coverage:** Private schools registered or recognized by ISBE
- **access_format:** HTML plus year-specific spreadsheets
- **verification_status:** opened_verified
- **evidence_url:** https://www.isbe.net/nonpublicprograms
- **implementation_notes:** 2025-26 registration and recognition downloads linked; annual renewal means missing schools are not necessarily closed. Master Directory of Educational Entities remains preferred broad seed. This is coverage validation, not separate coach data.
- **priority:** P2
- **source_system:** Illinois ISBE
- **shortlist_rank:** null
- **verification_date:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - IL
- **source_id:** SRC-127
- **observed_routes:**
  - (empty list)

### SRC-128

Iowa IHSAA member school list and detail pages

- **state:** IA
- **name:** Iowa IHSAA member school list and detail pages
- **url:** https://www.iahsaa.org/member-schools/
- **category:** state_athletics_school_seed
- **sports_scope:** high school track & field and cross country
- **school_or_coach_fields:** school; conference; school colors/nickname; linked school details
- **coverage:** Statewide high-school members, public and private
- **access_format:** HTML table
- **verification_status:** opened_verified
- **evidence_url:** https://www.iahsaa.org/member-schools/
- **implementation_notes:** Current table dated Aug 4, 2026. Sample https://www.iahsaa.org/schools/acgc/ opened. Official contact page sends school-contact users to https://www.gobound.com/ia/schools . No current coach list visible on IHSAA sample itself; use linked Bound route with access caveats.
- **priority:** P0
- **source_system:** Iowa IHSAA / Bound
- **shortlist_rank:** 1
- **verification_date:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - IA
- **source_id:** SRC-128
- **observed_routes:**
  - (empty list)

### SRC-129

Bound Iowa school staff directory

- **state:** IA
- **name:** Bound Iowa school staff directory
- **url:** https://www.gobound.com/ia/schools
- **category:** athletics_platform_coach_directory
- **sports_scope:** high school track & field and cross country
- **school_or_coach_fields:** staff role; name; optional school-work email/phone; XC/track gender roles where supplied
- **coverage:** Iowa schools using Bound; official IHSAA referral
- **access_format:** JavaScript platform; indexed public detail pages
- **verification_status:** blocked
- **evidence_url:** https://www.iahsaa.org/contact/
- **implementation_notes:** Root opened only minimal shell. Sample https://www.gobound.com/ia/schools/northbutler/Directory search index shows varsity girls XC coach and administrator contacts; direct open returned 403. Current detail fields therefore search-index-only, not opened-verified. No public API verified; do not bypass access blocks.
- **priority:** P0
- **source_system:** Iowa IHSAA / Bound
- **shortlist_rank:** null
- **verification_date:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - IA
- **source_id:** SRC-129
- **observed_routes:**
  - (empty list)

### SRC-130

Iowa DOE public and nonpublic building directories

- **state:** IA
- **name:** Iowa DOE public and nonpublic building directories
- **url:** https://educate.iowa.gov/directories
- **category:** education_department_school_seed
- **sports_scope:** high school track & field and cross country
- **school_or_coach_fields:** public district/building and nonpublic building directory records; workbook fields not inspected
- **coverage:** Statewide public and nonpublic PK-12 schools
- **access_format:** HTML landing with XLSX downloads
- **verification_status:** opened_verified
- **evidence_url:** https://educate.iowa.gov/directories
- **implementation_notes:** Three 2026-27 Excel downloads dated Sep 30, 2026: public district, public building, nonpublic building. Prefer building files then filter grades. Exact binary download URLs were not resolved in this pass; follow current links. ArcGIS school-building API below provides verified schema.
- **priority:** P0
- **source_system:** Iowa Department of Education
- **shortlist_rank:** 2
- **verification_date:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - IA
- **source_id:** SRC-130
- **observed_routes:**
  - (empty list)

### SRC-131

Iowa school buildings ArcGIS REST service

- **state:** IA
- **name:** Iowa school buildings ArcGIS REST service
- **url:** https://services.arcgis.com/vPD5PVLI6sfkZ5E4/arcgis/rest/services/IowaSchoolBldgs/FeatureServer
- **category:** documented_public_api_school_seed
- **sports_scope:** high school track & field and cross country
- **school_or_coach_fields:** SchoolName; SchoolType; District/School IDs; grades; address; administrator; phone; email; geometry
- **coverage:** Public layer 0 and private layer 1; 2025-26 buildings updated July 29, 2026
- **access_format:** ArcGIS REST JSON/GeoJSON/PBF
- **verification_status:** opened_verified
- **evidence_url:** https://services.arcgis.com/vPD5PVLI6sfkZ5E4/arcgis/rest/services/IowaSchoolBldgs/FeatureServer
- **implementation_notes:** Layer schema and Query UI opened. Public layer /0 uses EmailAddress, MailingStreetAddress, ZipCode, OBJECTID; private /1 uses Email, MailingStreet, Zip, OBJECTID_1. Query endpoints are /0/query and /1/query. MaxRecordCount 1000. SchoolType codes 1 public HS, 6 nonpublic HS, 8 nonpublic K-12; also examine alternative/charter grades. Schema/query UI verified; one-record JSON query attempts were inaccessible via research web tool, so response payload unverified.
- **priority:** P0
- **source_system:** Iowa Department of Education
- **shortlist_rank:** null
- **verification_date:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - IA
- **source_id:** SRC-131
- **observed_routes:**
  - (empty list)

### SRC-132

Iowa Association of Track Coaches

- **state:** IA
- **name:** Iowa Association of Track Coaches
- **url:** https://www.iatrackcoaches.org/
- **category:** coaches_association
- **sports_scope:** high school track & field and cross country
- **school_or_coach_fields:** public school membership; coach/advisory contacts; coaching awards
- **coverage:** Statewide association; public lists are partial
- **access_format:** HTML
- **verification_status:** opened_verified
- **evidence_url:** https://www.iatrackcoaches.org/
- **implementation_notes:** Membership list https://www.iatrackcoaches.org/membership-list/ verified 2026-27 school + class rows, not individual coach directory. Contact page https://www.iatrackcoaches.org/contact-us/ opened. Good supplement for schools and named coaches but neither complete nor guaranteed current head-coach assignments.
- **priority:** P1
- **source_system:** IATC
- **shortlist_rank:** 3
- **verification_date:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - IA
- **source_id:** SRC-132
- **observed_routes:**
  - (empty list)

### SRC-133

IATC track-and-field advisory board

- **state:** IA
- **name:** IATC track-and-field advisory board
- **url:** https://www.iatrackcoaches.org/track-and-field-advisory-board/
- **category:** coaches_association_partial_contacts
- **sports_scope:** high school track & field and cross country
- **school_or_coach_fields:** advisory coach/member identities and affiliations
- **coverage:** Advisory-board subset
- **access_format:** HTML
- **verification_status:** opened_verified
- **evidence_url:** https://www.iatrackcoaches.org/track-and-field-advisory-board/
- **implementation_notes:** A specific contact-enrichment page, not full membership. Cross-country sibling https://www.iatrackcoaches.org/cross-country-advisory-board/ returned only a tiny body in this pass; prefer contact-us and official IHSAA committees if needed.
- **priority:** P2
- **source_system:** IATC
- **shortlist_rank:** null
- **verification_date:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - IA
- **source_id:** SRC-133
- **observed_routes:**
  - (empty list)

### SRC-134

IHSAA joint XC and track committees

- **state:** IA
- **name:** IHSAA joint XC and track committees
- **url:** https://www.iahsaa.org/about/committees/
- **category:** state_athletics_partial_coach_contacts
- **sports_scope:** high school track & field and cross country
- **school_or_coach_fields:** head coach names; school affiliations; committee terms
- **coverage:** Joint IHSAA/IGHSAU sport committee members only
- **access_format:** HTML
- **verification_status:** search_only
- **evidence_url:** https://www.iahsaa.org/about/committees/
- **implementation_notes:** Search index explicitly lists XC and track head coaches/schools; direct page not opened in this pass. Useful specific enrichment but not comprehensive, and committee terms can be stale.
- **priority:** P2
- **source_system:** Iowa IHSAA / Bound
- **shortlist_rank:** null
- **verification_date:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - IA
- **source_id:** SRC-134
- **observed_routes:**
  - (empty list)

### SRC-135

Iowa Girls High School Athletic Union

- **state:** IA
- **name:** Iowa Girls High School Athletic Union
- **url:** https://www.ighsau.org/
- **category:** state_athletics_girls
- **sports_scope:** high school track & field and cross country
- **school_or_coach_fields:** girls XC/track participation, classification, and coach-resource leads; actual fields unverified here
- **coverage:** Girls sports counterpart to Iowa IHSAA
- **access_format:** website; current fetch blocked
- **verification_status:** blocked
- **evidence_url:** https://www.ighsau.org/
- **implementation_notes:** Direct homepage open returned 403. Do not substitute boys-only coverage for girls. Joint IHSAA classification/committee resources and school staff pages can corroborate female-team coverage without claiming this site was opened successfully.
- **priority:** P1
- **source_system:** IGHSAU
- **shortlist_rank:** null
- **verification_date:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - IA
- **source_id:** SRC-135
- **observed_routes:**
  - (empty list)

### SRC-136

WIAA Washington member-school directory

- **state:** WA
- **name:** WIAA Washington member-school directory
- **url:** https://wiaa-dna4aga5arc0gyeb.westus2-01.azurewebsites.net/directory.aspx?SecID=654
- **category:** state_athletics
- **sports_scope:** All school sports; use to seed TF/XC school crawl
- **school_or_coach_fields:** school; type; level; classification; league; school district; WIAA district
- **coverage:** WIAA membership, public and private; filter high-school level
- **access_format:** ASP.NET HTML search form
- **verification_status:** opened_verified
- **evidence_url:** https://wiaa-dna4aga5arc0gyeb.westus2-01.azurewebsites.net/directory.aspx?SecID=654
- **implementation_notes:** Search controls opened. Enumerate available high-school/league filters and follow returned school links. No coach export or API verified. Current www.wiaa.com/schools/ landing returned 403 in this session. Do not confuse with Wisconsin schools.wiaawi.org.
- **priority:** P1
- **verification_date:** 2026-10-04
- **source_system:** WIAA Washington
- **recommended_rank:** 1
- **checked_date:** 2026-10-04
- **applicable_states:**
  - WA
- **shortlist_rank:** 1
- **source_id:** SRC-136
- **observed_routes:**
  - (empty list)

### SRC-137

Seattle Public Schools coach-page network, West Seattle example

- **state:** WA
- **name:** Seattle Public Schools coach-page network, West Seattle example
- **url:** https://westseattlehs.seattleschools.org/student-life/athletics/coaches/
- **category:** school_staff_directory
- **sports_scope:** Boys/girls XC and TF
- **school_or_coach_fields:** school; season; sport; gender; head coach; email
- **coverage:** Seattle public high schools; directly verified at West Seattle and Ballard, not statewide
- **access_format:** HTML; district school selector links
- **verification_status:** opened_verified
- **evidence_url:** https://westseattlehs.seattleschools.org/student-life/athletics/coaches/
- **implementation_notes:** 2026-27 coach list explicitly labels all four target teams. Enumerate high-school subdomains from Select school menu, then follow actual athletics/contact links. Paths vary; avoid guessing one path for every school. Exclude elementary/middle-school sections.
- **priority:** P1
- **verification_date:** 2026-10-04
- **source_system:** Seattle Public Schools
- **recommended_rank:** 2
- **checked_date:** 2026-10-04
- **applicable_states:**
  - WA
- **shortlist_rank:** 2
- **source_id:** SRC-137
- **observed_routes:**
  - (empty list)

### SRC-138

WSCCCA leadership and district advisory board

- **state:** WA
- **name:** WSCCCA leadership and district advisory board
- **url:** https://www.wsccca.com/leadership
- **category:** coaches_association
- **sports_scope:** Cross country
- **school_or_coach_fields:** coach name; school; district representation; some emails; association role
- **coverage:** Small statewide representative subset, NOT all Washington coaches
- **access_format:** HTML
- **verification_status:** opened_verified
- **evidence_url:** https://www.wsccca.com/leadership
- **implementation_notes:** Includes executive, advisory and service boards with school affiliations. Treat these as partial coach leads; confirm current school coaching job on school website. Do not label this a statewide member directory.
- **priority:** P2
- **verification_date:** 2026-10-04
- **source_system:** WSCCCA
- **recommended_rank:** 3
- **checked_date:** 2026-10-04
- **applicable_states:**
  - WA
- **shortlist_rank:** 3
- **source_id:** SRC-138
- **observed_routes:**
  - (empty list)

### SRC-139

Ballard High School teams and coaches

- **state:** WA
- **name:** Ballard High School teams and coaches
- **url:** https://ballardhs.seattleschools.org/student-life/athletics/athletics-contacts/
- **category:** school_staff_directory
- **sports_scope:** XC and TF
- **school_or_coach_fields:** coach; email; team; gender; school
- **coverage:** One Seattle school; additional template fixture
- **access_format:** HTML
- **verification_status:** opened_verified
- **evidence_url:** https://ballardhs.seattleschools.org/student-life/athletics/athletics-contacts/
- **implementation_notes:** Second verified Seattle school template. Parse by target sport section; page also includes workouts and biographies, which are outside collection scope.
- **priority:** P1
- **verification_date:** 2026-10-04
- **source_system:** Seattle Public Schools
- **checked_date:** 2026-10-04
- **applicable_states:**
  - WA
- **shortlist_rank:** null
- **source_id:** SRC-139
- **observed_routes:**
  - (empty list)

### SRC-140

WSTFCA convention presentations

- **state:** WA
- **name:** WSTFCA convention presentations
- **url:** https://www.wstfca.com/2026-presentations
- **category:** coaches_association
- **sports_scope:** Track and field; distance/XC speakers
- **school_or_coach_fields:** speaker coach; school; presentation title
- **coverage:** 2026 convention speakers only
- **access_format:** HTML with presentation links
- **verification_status:** opened_verified
- **evidence_url:** https://www.wstfca.com/2026-presentations
- **implementation_notes:** Coach-school pairs include Washington and Oregon schools. Filter physical state; speaker status is not proof of current employment. Root https://www.wstfca.com/ and executive-board page https://www.wstfca.com/about-1-1 are additional association discovery routes.
- **priority:** P3
- **verification_date:** 2026-10-04
- **source_system:** WSTFCA
- **checked_date:** 2026-10-04
- **applicable_states:**
  - WA
- **shortlist_rank:** null
- **source_id:** SRC-140
- **observed_routes:**
  - (empty list)

### SRC-141

Washington Federation of Independent Schools directory

- **state:** WA
- **name:** Washington Federation of Independent Schools directory
- **url:** https://wfis.org/washington-private-schools-directory/
- **category:** private_school_directory
- **sports_scope:** School discovery only
- **school_or_coach_fields:** school listing; school website/details where rendered
- **coverage:** WFIS membership subset of private schools; all grades
- **access_format:** Dynamic HTML directory
- **verification_status:** opened_verified
- **evidence_url:** https://wfis.org/washington-private-schools-directory/
- **implementation_notes:** Directory landing opened; visible page shell does not expose all records to text extraction. Inspect rendered directory/filter controls. Check grades before coaching crawl. WFIS homepage reports 260+ members; not all approved private schools.
- **priority:** P2
- **verification_date:** 2026-10-04
- **source_system:** WFIS
- **checked_date:** 2026-10-04
- **applicable_states:**
  - WA
- **shortlist_rank:** null
- **source_id:** SRC-141
- **observed_routes:**
  - (empty list)

### SRC-142

Washington SBE approved private schools

- **state:** WA
- **name:** Washington SBE approved private schools
- **url:** https://sbe.wa.gov/our-work/private-schools
- **category:** state_education_private_directory
- **sports_scope:** School discovery only
- **school_or_coach_fields:** school; approval date; address; district; online program indicator
- **coverage:** State-approved private schools, all grades
- **access_format:** Landing plus downloadable annual list
- **verification_status:** blocked
- **evidence_url:** https://sbe.wa.gov/our-work/private-schools
- **implementation_notes:** Search index reports 2026-27 approved list updated 2026-09-15; direct open returned 403. Do not claim file schema/download tested. When accessible follow current annual list and filter high-school grades.
- **priority:** P2
- **verification_date:** 2026-10-04
- **source_system:** Washington SBE
- **checked_date:** 2026-10-04
- **applicable_states:**
  - WA
- **shortlist_rank:** null
- **source_id:** SRC-142
- **observed_routes:**
  - (empty list)

### SRC-143

OSPI Education Directory

- **state:** WA
- **name:** OSPI Education Directory
- **url:** https://eds.ospi.k12.wa.us/DirectoryEDS.aspx
- **category:** state_education_directory_restricted
- **sports_scope:** School inventory only
- **school_or_coach_fields:** school and LEA codes; grade span; address; administrative contacts
- **coverage:** Statewide public/charter/tribal directory
- **access_format:** ASP.NET report selector; Excel export documented
- **verification_status:** opened_verified
- **evidence_url:** https://eds.ospi.k12.wa.us/DirectoryEDS.aspx
- **implementation_notes:** Directory opened and visibly states it may not be used for commercial purposes. Default report is LEA, not school. Do not ingest into a commercial acquisition database absent permission/legal clearance.
- **priority:** P3
- **verification_date:** 2026-10-04
- **source_system:** OSPI
- **restriction_flag:** EXPLICIT NONCOMMERCIAL-USE NOTICE; NOT RECOMMENDED FOR COMMERCIAL INGESTION
- **checked_date:** 2026-10-04
- **applicable_states:**
  - WA
- **shortlist_rank:** null
- **source_id:** SRC-143
- **observed_routes:**
  - (empty list)

### SRC-144

OSPI public-school ArcGIS service

- **state:** WA
- **name:** OSPI public-school ArcGIS service
- **url:** https://services1.arcgis.com/Ezk9fcjSUkeadg6u/ArcGIS/rest/services/Washington_State_Public_Schools/FeatureServer
- **category:** geospatial_inventory_restricted
- **sports_scope:** School inventory only
- **school_or_coach_fields:** school location; school attributes; layer schema not inspected
- **coverage:** Public schools including charter, tribal compact and skills centers; service description says 2021-22
- **access_format:** ArcGIS REST FeatureServer; JSON supported
- **verification_status:** opened_verified
- **evidence_url:** https://services1.arcgis.com/Ezk9fcjSUkeadg6u/ArcGIS/rest/services/Washington_State_Public_Schools/FeatureServer
- **implementation_notes:** Service metadata opened; maxRecordCount 2000. No query result tested. This is OSPI-derived data; do not use it as a workaround for the Education Directory commercial-use notice. Confirm current vintage and usage rights first.
- **priority:** P3
- **verification_date:** 2026-10-04
- **source_system:** OSPI
- **restriction_flag:** OSPI-derived; commercial-use restriction review required
- **checked_date:** 2026-10-04
- **applicable_states:**
  - WA
- **shortlist_rank:** null
- **source_id:** SRC-144
- **observed_routes:**
  - (empty list)

### SRC-145

KingCo league school-search directory

- **state:** WA
- **name:** KingCo league school-search directory
- **url:** https://www.kingcoathletics.com/school-search/
- **category:** regional_league
- **sports_scope:** TF/XC school and league discovery
- **school_or_coach_fields:** school; classification; league; contacts if rendered
- **coverage:** KingCo and shared WIAA districts/leagues widget
- **access_format:** rSchoolToday dynamic HTML
- **verification_status:** opened_verified
- **evidence_url:** https://www.kingcoathletics.com/school-search/
- **implementation_notes:** Landing opened but school widget absent from text extraction; search index contains league-school lists. Needs rendered-page extraction; no endpoint verified. Metro https://www.metroleaguewa.org/ and Olympic https://www.olympicleague.com/ use same system; avoid duplicate counting.
- **priority:** P2
- **verification_date:** 2026-10-04
- **source_system:** Washington rSchoolToday leagues
- **checked_date:** 2026-10-04
- **applicable_states:**
  - WA
- **shortlist_rank:** null
- **source_id:** SRC-145
- **observed_routes:**
  - (empty list)

### SRC-146

OSAA member-school profiles and coach directory

- **state:** OR
- **name:** OSAA member-school profiles and coach directory
- **url:** https://www.osaa.org/schools/regions
- **category:** state_athletics_coach_directory
- **sports_scope:** Boys/girls XC and TF
- **school_or_coach_fields:** school ID; school name; type; website; address; league; sport; head coach; athletic director contacts
- **coverage:** Full OSAA members, public/private; includes some WA-border schools
- **access_format:** Server-rendered HTML lists and profiles
- **verification_status:** opened_verified
- **evidence_url:** https://www.osaa.org/schools/46
- **implementation_notes:** Enumerate profile hrefs from regional/member list. Verified profile https://www.osaa.org/schools/46 has Sports / Activities rows with all four target sports and coach names. A separate Contact Information table is administrative: do not mislabel AD emails as coach emails.
- **priority:** P1
- **verification_date:** 2026-10-04
- **source_system:** OSAA
- **recommended_rank:** 1
- **checked_date:** 2026-10-04
- **applicable_states:**
  - OR
- **shortlist_rank:** 1
- **source_id:** SRC-146
- **observed_routes:**
  - (empty list)

### SRC-147

ODE Institutions Database and daily extract

- **state:** OR
- **name:** ODE Institutions Database and daily extract
- **url:** https://www.ode.state.or.us/instID/
- **category:** state_education_directory
- **sports_scope:** School discovery and identity reconciliation
- **school_or_coach_fields:** institution ID; name; city; county; district; ESD; open/closed status
- **coverage:** Public schools plus other institutions doing business with ODE; not complete private registry
- **access_format:** Search HTML plus documented zipped Excel 8.0 daily extract
- **verification_status:** opened_verified
- **evidence_url:** https://www.oregon.gov/ode/schools-and-districts/Pages/Institution-Identification-School-Names.aspx
- **implementation_notes:** Landing documents daily all-institution ZIP/XLS download. Download link itself failed in web fetch and shell landing returned 502. Current canonical explanation page links this application; do not invent JSON API. Filter school institutions, active status and high-school grades.
- **priority:** P1
- **verification_date:** 2026-10-04
- **source_system:** Oregon Department of Education
- **recommended_rank:** 2
- **checked_date:** 2026-10-04
- **applicable_states:**
  - OR
- **shortlist_rank:** 2
- **source_id:** SRC-147
- **observed_routes:**
  - (empty list)

### SRC-148

Oregon Federation of Independent Schools members

- **state:** OR
- **name:** Oregon Federation of Independent Schools members
- **url:** https://ofisweb.org/members/
- **category:** private_school_association
- **sports_scope:** School discovery only
- **school_or_coach_fields:** membership year/page identity; actual member rows not verified
- **coverage:** 2026-27 OFIS private-school membership subset
- **access_format:** HTML; embedded/downloaded material may need link follow
- **verification_status:** opened_verified
- **evidence_url:** https://ofisweb.org/members/
- **implementation_notes:** Membership-page title July 2026-June 2027 verified, but no actual member list exposed in text extraction beyond the executive director school. Treat as discovery candidate requiring rendered-page verification, not a ready corpus. No TF/XC roster verified.
- **priority:** P3
- **verification_date:** 2026-10-04
- **source_system:** OFIS
- **checked_date:** 2026-10-04
- **applicable_states:**
  - OR
- **shortlist_rank:** null
- **source_id:** SRC-148
- **observed_routes:**
  - (empty list)

### SRC-149

OSAA statewide compact coaches directory

- **state:** OR
- **name:** OSAA statewide compact coaches directory
- **url:** https://www.osaa.org/coaches-directory
- **category:** state_athletics_coach_directory
- **sports_scope:** BXC/GXC/BTF/GTF
- **school_or_coach_fields:** school; league; school phone; AD contact; sport code; head coach
- **coverage:** Statewide OSAA membership
- **access_format:** Single long HTML directory
- **verification_status:** blocked
- **evidence_url:** https://oregoncoach.org/directory/
- **implementation_notes:** Search index shows last update 2026-09-28 and sport codes BXC/GXC/BTF/GTF. Direct web and shell GET both returned 403. Prioritize accessible OSAA school profiles; association OACA confirms this public directory exists.
- **priority:** P1
- **verification_date:** 2026-10-04
- **source_system:** OSAA
- **checked_date:** 2026-10-04
- **applicable_states:**
  - OR
- **shortlist_rank:** null
- **source_id:** SRC-149
- **observed_routes:**
  - (empty list)

### SRC-150

Oregon Athletic Coaches Association directory gateway

- **state:** OR
- **name:** Oregon Athletic Coaches Association directory gateway
- **url:** https://oregoncoach.org/directory/
- **category:** coaches_association
- **sports_scope:** All sports including XC/TF
- **school_or_coach_fields:** links to OSAA directory; description of available fields
- **coverage:** Oregon coaches; gateway, not independent data corpus
- **access_format:** HTML
- **verification_status:** opened_verified
- **evidence_url:** https://oregoncoach.org/directory/
- **implementation_notes:** Explicitly links OSAA statewide coaches directory. Do not count OACA gateway and OSAA as separate coverage. Useful maintained source-discovery link.
- **priority:** P2
- **verification_date:** 2026-10-04
- **source_system:** OACA
- **checked_date:** 2026-10-04
- **applicable_states:**
  - OR
- **shortlist_rank:** null
- **source_id:** SRC-150
- **observed_routes:**
  - (empty list)

### SRC-151

OACA Coach of the Year 2025-26

- **state:** OR
- **name:** OACA Coach of the Year 2025-26
- **url:** https://oregoncoach.org/oaca-coach-of-the-year/
- **category:** coaches_association_awards
- **sports_scope:** Boys/girls XC and TF
- **school_or_coach_fields:** coach; school; sport; classification; award season
- **coverage:** Selected award winners only
- **access_format:** HTML headings and bullet lists
- **verification_status:** opened_verified
- **evidence_url:** https://oregoncoach.org/oaca-coach-of-the-year/
- **implementation_notes:** Useful dated coach-school validation; not current roster or all coaches. Store award year and verify employment independently.
- **priority:** P2
- **verification_date:** 2026-10-04
- **source_system:** OACA
- **recommended_rank:** 3
- **checked_date:** 2026-10-04
- **applicable_states:**
  - OR
- **shortlist_rank:** 3
- **source_id:** SRC-151
- **observed_routes:**
  - (empty list)

### SRC-152

ODE Oregon School Directory PDF

- **state:** OR
- **name:** ODE Oregon School Directory PDF
- **url:** https://www.oregon.gov/ode/about-us/pages/school-directory.aspx
- **category:** state_education_directory
- **sports_scope:** School discovery only
- **school_or_coach_fields:** school; phone; address; key staff; website
- **coverage:** Oregon public schools/districts with private-school information
- **access_format:** 105-page PDF; annual/current landing
- **verification_status:** opened_verified
- **evidence_url:** https://www.oregon.gov/ode/about-us/pages/school-directory.aspx
- **implementation_notes:** Current observed PDF https://www.oregon.gov/ode/about-us/Documents/CombinedDirectory_20260922_034305.pdf opened. Prefer daily structured institution extract when available; PDF link rotates, so discover from landing.
- **priority:** P2
- **verification_date:** 2026-10-04
- **source_system:** Oregon Department of Education
- **checked_date:** 2026-10-04
- **applicable_states:**
  - OR
- **shortlist_rank:** null
- **source_id:** SRC-152
- **observed_routes:**
  - (empty list)

### SRC-153

ODE private-school scope notice

- **state:** OR
- **name:** ODE private-school scope notice
- **url:** https://www.oregon.gov/ode/learning-options/schooltypes/private/Pages/default.aspx
- **category:** coverage_caveat
- **sports_scope:** Private school coverage interpretation
- **school_or_coach_fields:** no coach directory
- **coverage:** Oregon private education
- **access_format:** HTML
- **verification_status:** opened_verified
- **evidence_url:** https://www.oregon.gov/ode/learning-options/schooltypes/private/Pages/default.aspx
- **implementation_notes:** ODE states it does not accredit, approve or register private schools. Therefore its institution database is not an exhaustive approved-private-school inventory; use OFIS, OSAA, and school sites to reconcile.
- **priority:** P3
- **verification_date:** 2026-10-04
- **source_system:** Oregon Department of Education
- **checked_date:** 2026-10-04
- **applicable_states:**
  - OR
- **shortlist_rank:** null
- **source_id:** SRC-153
- **observed_routes:**
  - (empty list)

### SRC-154

OSAA cross-country district meet director list

- **state:** OR
- **name:** OSAA cross-country district meet director list
- **url:** https://osaa.org/docs/bxc/xcdmdinfo.pdf
- **category:** meet_director_contacts
- **sports_scope:** Cross country
- **school_or_coach_fields:** district; league; host school; meet director; work phone; email
- **coverage:** District-meet hosts only; 2025 document indexed
- **access_format:** PDF
- **verification_status:** search_only
- **evidence_url:** https://osaa.org/docs/bxc/xcdmdinfo.pdf
- **implementation_notes:** Search result verifies director fields. Not opened in this pass. Contact can be an athletic director or non-coach; never promote role automatically.
- **priority:** P3
- **verification_date:** 2026-10-04
- **source_system:** OSAA
- **checked_date:** 2026-10-04
- **applicable_states:**
  - OR
- **shortlist_rank:** null
- **source_id:** SRC-154
- **observed_routes:**
  - (empty list)

### SRC-155

AIA member-school and sport-coach directory

- **state:** AZ
- **name:** AIA member-school and sport-coach directory
- **url:** https://aiaonline.org/schools
- **category:** state_athletics_coach_directory
- **sports_scope:** Boys/girls XC and TF
- **school_or_coach_fields:** school ID; name; district; website; address; sport; head coach; administrative phone
- **coverage:** 287 member schools claimed by landing; public/private statewide
- **access_format:** JSON school search plus HTML school profiles
- **verification_status:** opened_verified
- **evidence_url:** https://aiaonline.org/schools/100
- **implementation_notes:** GET /schools/search.json?q=chandler returned 10 JSON school records. Parse school profiles /schools/{id}; verified /schools/100 has XC boys/girls and Track boys/girls head coaches. Empty q returned only 20, not complete inventory. Public emails redirected to admin directory; no public coach-email API verified.
- **priority:** P1
- **verification_date:** 2026-10-04
- **source_system:** AIA
- **recommended_rank:** 1
- **checked_date:** 2026-10-04
- **applicable_states:**
  - AZ
- **shortlist_rank:** 1
- **source_id:** SRC-155
- **observed_routes:**
  - (empty list)

### SRC-156

CAA GameSource sport team and coach tables

- **state:** AZ
- **name:** CAA GameSource sport team and coach tables
- **url:** https://teams.gamesource.io/teams.php?association=CAA&association_sport_id=44&association_sport_level_id=198&division_id=8
- **category:** alternative_athletics_coach_directory
- **sports_scope:** Verified boys varsity track; sport/level controls for other CAA teams
- **school_or_coach_fields:** school/team; head coach; assistant coach; coordinator; team profile links
- **coverage:** Canyon Athletic Association; charter/private/home-school and other small-school programs
- **access_format:** HTML tables and team links
- **verification_status:** opened_verified
- **evidence_url:** https://teams.gamesource.io/teams.php?association=CAA&association_sport_id=44&association_sport_level_id=198&division_id=8
- **implementation_notes:** Observed exact parameters select Track & Field Boys Varsity. Preserve team-to-coach row boundaries; several coaches can follow one team. Discover other sport/level IDs from page controls, never guess. Some rows have no coach. Historical/freshness caveat: footer 2025; reconcile 2026-27 against CAA current site/Bound.
- **priority:** P1
- **verification_date:** 2026-10-04
- **source_system:** Canyon Athletic Association
- **recommended_rank:** 2
- **checked_date:** 2026-10-04
- **applicable_states:**
  - AZ
- **shortlist_rank:** 2
- **source_id:** SRC-156
- **observed_routes:**
  - (empty list)

### SRC-157

ADE Active LEAs and Schools

- **state:** AZ
- **name:** ADE Active LEAs and Schools
- **url:** https://www.azed.gov/finance/local-education-agencies
- **category:** state_education_directory
- **sports_scope:** School inventory only
- **school_or_coach_fields:** active LEA and school names/identifiers; XLSX schema not verified
- **coverage:** Fundable active public/charter LEAs and school sites; FY2026
- **access_format:** XLSX linked from HTML
- **verification_status:** opened_verified
- **evidence_url:** https://www.azed.gov/finance/local-education-agencies
- **implementation_notes:** Exact observed file https://www.azed.gov/sites/default/files/2025/09/FY2026%20Active%20LEAs%20and%20Schools.xlsx. Web loader cannot parse XLSX and direct file request returned 403; landing verified only. Use latest link rather than hardcoding fiscal year. Does not imply complete private-school coverage.
- **priority:** P1
- **verification_date:** 2026-10-04
- **source_system:** Arizona Department of Education
- **recommended_rank:** 3
- **checked_date:** 2026-10-04
- **applicable_states:**
  - AZ
- **shortlist_rank:** 3
- **source_id:** SRC-157
- **observed_routes:**
  - (empty list)

### SRC-158

AIA school-search JSON endpoint

- **state:** AZ
- **name:** AIA school-search JSON endpoint
- **url:** https://aiaonline.org/schools/search.json?q=chandler
- **category:** public_website_json_endpoint
- **sports_scope:** School identity only
- **school_or_coach_fields:** id; name; full_name; mascot; logo.thumbnail; alignment.conference; alignment.region; address.line1/city/state/zip
- **coverage:** Search results, capped response observed; not a bulk export
- **access_format:** JSON GET
- **verification_status:** opened_verified
- **evidence_url:** https://aiaonline.org/schools
- **implementation_notes:** Endpoint observed in official bundled /js/app.js?id=088e302cc3da966e445100a31e2810f5, not guessed. Actual 200 JSON parsed. q= returned 20; q=chandler returned 10. No pagination or supported bulk enumeration established; seed names from alignment lists then query exact names and deduplicate id.
- **priority:** P1
- **verification_date:** 2026-10-04
- **source_system:** AIA
- **checked_date:** 2026-10-04
- **applicable_states:**
  - AZ
- **shortlist_rank:** null
- **source_id:** SRC-158
- **observed_routes:**
  - (empty list)

### SRC-159

AIA sport alignments

- **state:** AZ
- **name:** AIA sport alignments
- **url:** https://aiaonline.org/alignments/2025/activities/crosscountry-girls
- **category:** state_sport_program_inventory
- **sports_scope:** Girls XC; sibling sport links via By Sport
- **school_or_coach_fields:** school names and sport divisions/sections
- **coverage:** AIA 2025-26 block, marked previous scheduling block
- **access_format:** HTML tables
- **verification_status:** search_only
- **evidence_url:** https://aiaonline.org/alignments/2025/activities/crosscountry-girls
- **implementation_notes:** Do not assume root /alignments is current season: opened root currently leads to 2027-28 not-released block. Pin season intentionally and follow actual By Sport links. This dated page is indexed but not individually opened.
- **priority:** P2
- **verification_date:** 2026-10-04
- **source_system:** AIA
- **checked_date:** 2026-10-04
- **applicable_states:**
  - AZ
- **shortlist_rank:** null
- **source_id:** SRC-159
- **observed_routes:**
  - (empty list)

### SRC-160

CAA official Track and Field hub

- **state:** AZ
- **name:** CAA official Track and Field hub
- **url:** https://azcaa.com/sports/track-field
- **category:** alternative_athletics_association
- **sports_scope:** Track and field
- **school_or_coach_fields:** meet host school; division/HS/JH; association sport director; current links
- **coverage:** CAA current TF operations; selective hosts, not coach directory
- **access_format:** HTML schedule and documents
- **verification_status:** opened_verified
- **evidence_url:** https://azcaa.com/sports/track-field
- **implementation_notes:** 2026 meet schedule distinguishes HS from JH. Use as current program/host corroboration and to discover current management platform. Director is association staff, not all school coaches.
- **priority:** P2
- **verification_date:** 2026-10-04
- **source_system:** Canyon Athletic Association
- **checked_date:** 2026-10-04
- **applicable_states:**
  - AZ
- **shortlist_rank:** null
- **source_id:** SRC-160
- **observed_routes:**
  - (empty list)

### SRC-161

CAA Bound high-school enrollment/team lists

- **state:** AZ
- **name:** CAA Bound high-school enrollment/team lists
- **url:** https://www.gobound.com/az/associations/azcaa/enrollments?idSeason=h20250209053859497652e74a5d5194f&schoolType=High+School
- **category:** alternative_athletics_team_inventory
- **sports_scope:** School teams including XC
- **school_or_coach_fields:** school, mascot, division, sport team listing
- **coverage:** CAA high-school teams by season
- **access_format:** Dynamic HTML
- **verification_status:** blocked
- **evidence_url:** https://www.gobound.com/az/associations/azcaa/enrollments?idSeason=h20250209053859497652e74a5d5194f&schoolType=High+School
- **implementation_notes:** Search index showed high-school boys XC teams. Open returned JavaScript anti-robot interstitial, so actual records not verified. Do not bypass challenge. Keep as later user/browser verification candidate.
- **priority:** P2
- **verification_date:** 2026-10-04
- **source_system:** Canyon Athletic Association
- **checked_date:** 2026-10-04
- **applicable_states:**
  - AZ
- **shortlist_rank:** null
- **source_id:** SRC-161
- **observed_routes:**
  - (empty list)

### SRC-162

Arizona Christian Education Coalition member schools

- **state:** AZ
- **name:** Arizona Christian Education Coalition member schools
- **url:** https://azchristianschools.org/members/
- **category:** private_school_association
- **sports_scope:** School discovery only
- **school_or_coach_fields:** school name; official school website
- **coverage:** 31 member schools claimed; Christian-school subset and mixed grades
- **access_format:** HTML links
- **verification_status:** opened_verified
- **evidence_url:** https://azchristianschools.org/members/
- **implementation_notes:** Enumerate member school links, filter grade span, then school athletics staff. Some repeated navigation/body lists must be deduplicated. No coaches or API on directory itself.
- **priority:** P2
- **verification_date:** 2026-10-04
- **source_system:** AZCEC
- **checked_date:** 2026-10-04
- **applicable_states:**
  - AZ
- **shortlist_rank:** null
- **source_id:** SRC-162
- **observed_routes:**
  - (empty list)

### SRC-163

AZPreps365 team pages

- **state:** AZ
- **name:** AZPreps365 team pages
- **url:** https://appext.azpreps365.com/teams/crosscountry-boys/3983-az-prep/226130-varsity
- **category:** state_sport_coach_profile
- **sports_scope:** Boys cross country example
- **school_or_coach_fields:** school, season, division, head coach
- **coverage:** AIA partner/team pages; season-specific
- **access_format:** HTML team pages
- **verification_status:** blocked
- **evidence_url:** https://appext.azpreps365.com/teams/crosscountry-boys/3983-az-prep/226130-varsity
- **implementation_notes:** Search index confirmed 2026-27 AZ College Prep coach display. Direct open cache miss. Prefer AIA school profiles already verified; this is an additional season validation route, not an independent statewide database.
- **priority:** P3
- **verification_date:** 2026-10-04
- **source_system:** AIA/AZPreps365
- **checked_date:** 2026-10-04
- **applicable_states:**
  - AZ
- **shortlist_rank:** null
- **source_id:** SRC-163
- **observed_routes:**
  - (empty list)

### SRC-164

UHSAA school profiles and coaches

- **state:** UT
- **name:** UHSAA school profiles and coaches
- **url:** https://uhsaa.org/school-directory-new/
- **category:** state_athletics_coach_directory
- **sports_scope:** Boys/girls XC and TF
- **school_or_coach_fields:** schoolID; name; address; district; classification; region; sport; coach; email
- **coverage:** UHSAA public/charter/private member schools statewide
- **access_format:** Enumerable HTML links and profile tables
- **verification_status:** opened_verified
- **evidence_url:** https://uhsaa.org/school-directory/?Reg=6&id=Alta&schoolID=1
- **implementation_notes:** Follow actual profile links with Reg/id/schoolID query parameters. Verified Alta schoolID=1 has coach names/emails for all four target teams; some Summit Academy cells empty. Empty coach does not prove absent sport. Parse Coaches separately from administrator roles.
- **priority:** P1
- **verification_date:** 2026-10-04
- **source_system:** UHSAA
- **recommended_rank:** 1
- **checked_date:** 2026-10-04
- **applicable_states:**
  - UT
- **shortlist_rank:** 1
- **source_id:** SRC-164
- **observed_routes:**
  - (empty list)

### SRC-165

USBE Utah Schools Directory

- **state:** UT
- **name:** USBE Utah Schools Directory
- **url:** https://schools.utah.gov/schoolsdirectory
- **category:** state_education_directory
- **sports_scope:** School inventory only
- **school_or_coach_fields:** school and district/LEA; full schema depends on live data
- **coverage:** Utah school/LEA directory reported via CACTUS; do not assume exhaustive private coverage
- **access_format:** JavaScript table; client-generated CSV; observed JSON endpoint
- **verification_status:** opened_verified
- **evidence_url:** https://schools.utah.gov/schoolsdirectory
- **implementation_notes:** Raw official HTML contains dataUrl=https://cactus.schools.utah.gov/api/legacy/schools and Export CSV generated from rendered table. Endpoint requests returned 502 twice, so record endpoint as observed but unavailable, not verified successful JSON. District directory is accessible alternative.
- **priority:** P1
- **verification_date:** 2026-10-04
- **source_system:** Utah State Board of Education
- **recommended_rank:** 2
- **checked_date:** 2026-10-04
- **applicable_states:**
  - UT
- **shortlist_rank:** 2
- **source_id:** SRC-165
- **observed_routes:**
  - (empty list)

### SRC-166

Grand County High School sports coaches

- **state:** UT
- **name:** Grand County High School sports coaches
- **url:** https://gchs.grandschools.org/apps/pages/index.jsp?uREC_ID=1651964&type=d&pREC_ID=2260652&tota11y=true
- **category:** school_staff_directory
- **sports_scope:** XC and TF boys/girls
- **school_or_coach_fields:** school; sport; coach; email
- **coverage:** One school; concrete official-site fallback, NOT statewide
- **access_format:** Edlio HTML page
- **verification_status:** opened_verified
- **evidence_url:** https://gchs.grandschools.org/apps/pages/index.jsp?uREC_ID=1651964&type=d&pREC_ID=2260652&tota11y=true
- **implementation_notes:** Verified explicit target-sport coaches/email entries. Use a school-site adapter after statewide UHSAA/USBE seeds; discover actual athletics/staff links. Retain sports separately even when the same coach appears multiple times.
- **priority:** P2
- **verification_date:** 2026-10-04
- **source_system:** Grand County School District
- **recommended_rank:** 3
- **checked_date:** 2026-10-04
- **applicable_states:**
  - UT
- **shortlist_rank:** 3
- **source_id:** SRC-166
- **observed_routes:**
  - (empty list)

### SRC-167

USBE observed CACTUS school JSON endpoint

- **state:** UT
- **name:** USBE observed CACTUS school JSON endpoint
- **url:** https://cactus.schools.utah.gov/api/legacy/schools
- **category:** public_website_json_endpoint
- **sports_scope:** School inventory only
- **school_or_coach_fields:** live schema not retrieved
- **coverage:** Feeds USBE public school-directory table
- **access_format:** JSON endpoint referenced by official page JavaScript
- **verification_status:** blocked
- **evidence_url:** https://schools.utah.gov/schoolsdirectory
- **implementation_notes:** Official source code establishes exact endpoint, but direct GET returned HTTP 502 twice; no success, count or schema claimed. Recheck availability rather than invent fallback parameters.
- **priority:** P2
- **verification_date:** 2026-10-04
- **source_system:** Utah State Board of Education
- **checked_date:** 2026-10-04
- **applicable_states:**
  - UT
- **shortlist_rank:** null
- **source_id:** SRC-167
- **observed_routes:**
  - (empty list)

### SRC-168

USBE School District Directory

- **state:** UT
- **name:** USBE School District Directory
- **url:** https://www.schools.utah.gov/schooldistricts
- **category:** state_education_directory
- **sports_scope:** School-district seed discovery
- **school_or_coach_fields:** district; website; phone; address; superintendent email
- **coverage:** Utah public school districts
- **access_format:** HTML table and CSV export control
- **verification_status:** opened_verified
- **evidence_url:** https://www.schools.utah.gov/schooldistricts
- **implementation_notes:** Useful district website fallback while school JSON endpoint unavailable. Administrator email is not coach email. Follow district school listings and retain official domain provenance.
- **priority:** P2
- **verification_date:** 2026-10-04
- **source_system:** Utah State Board of Education
- **checked_date:** 2026-10-04
- **applicable_states:**
  - UT
- **shortlist_rank:** null
- **source_id:** SRC-168
- **observed_routes:**
  - (empty list)

### SRC-169

UHSAA schools by district and private/charter category

- **state:** UT
- **name:** UHSAA schools by district and private/charter category
- **url:** https://www.uhsaa.org/districts/
- **category:** state_athletics_school_inventory
- **sports_scope:** School seed discovery
- **school_or_coach_fields:** school; district; public charter/private category
- **coverage:** UHSAA membership statewide, explicit private-school subsection
- **access_format:** HTML lists
- **verification_status:** opened_verified
- **evidence_url:** https://www.uhsaa.org/districts/
- **implementation_notes:** Use to validate school category and catch private/charter membership; same source system as main school directory, not independent coverage.
- **priority:** P2
- **verification_date:** 2026-10-04
- **source_system:** UHSAA
- **checked_date:** 2026-10-04
- **applicable_states:**
  - UT
- **shortlist_rank:** null
- **source_id:** SRC-169
- **observed_routes:**
  - (empty list)

### SRC-170

UHSAA sanctioned-event contacts

- **state:** UT
- **name:** UHSAA sanctioned-event contacts
- **url:** https://www.uhsaa.org/sanctionslist/
- **category:** meet_director_contacts
- **sports_scope:** Track and XC; filter Sport column
- **school_or_coach_fields:** event; sport; date; host; contact name; email; approval statuses
- **coverage:** Meet hosts only; can include colleges/outside groups
- **access_format:** HTML table
- **verification_status:** opened_verified
- **evidence_url:** https://www.uhsaa.org/sanctionslist/
- **implementation_notes:** Filter Sport=XC or Track, then join host school. Contact might be meet director or collegiate organizer rather than school coach. Multiple events repeat contacts. Exclude denied/pending sanctions from program assertions and exclude student records.
- **priority:** P2
- **verification_date:** 2026-10-04
- **source_system:** UHSAA
- **checked_date:** 2026-10-04
- **applicable_states:**
  - UT
- **shortlist_rank:** null
- **source_id:** SRC-170
- **observed_routes:**
  - (empty list)

### SRC-171

UHSAA school participation matrix

- **state:** UT
- **name:** UHSAA school participation matrix
- **url:** https://uhsaa.org/ParticipationNumbers.pdf
- **category:** state_sport_program_inventory
- **sports_scope:** Boys/girls XC/track, plus unified track
- **school_or_coach_fields:** school and sport participation flags
- **coverage:** 2026-27; only schools submitting annual dues/fees form
- **access_format:** 3-page PDF matrix
- **verification_status:** opened_verified
- **evidence_url:** https://uhsaa.org/ParticipationNumbers.pdf
- **implementation_notes:** Useful positive program evidence only. Explicit incomplete-coverage footnote means missing school/flag is not proof of no team. Text extraction collapses X columns; use table coordinates or rendered PDF verification.
- **priority:** P2
- **verification_date:** 2026-10-04
- **source_system:** UHSAA
- **checked_date:** 2026-10-04
- **applicable_states:**
  - UT
- **shortlist_rank:** null
- **source_id:** SRC-171
- **observed_routes:**
  - (empty list)

### SRC-172

UCCTCA association and bylaws

- **state:** UT
- **name:** UCCTCA association and bylaws
- **url:** https://ucctca.org/files/ucctca-bylaws.pdf
- **category:** coaches_association
- **sports_scope:** Cross country and track
- **school_or_coach_fields:** association identity/purpose; no statewide coach roster verified
- **coverage:** Utah secondary-school coaches association
- **access_format:** 4-page PDF; root unavailable
- **verification_status:** opened_verified
- **evidence_url:** https://ucctca.org/files/ucctca-bylaws.pdf
- **implementation_notes:** Association existence and state focus verified via bylaws. Root https://ucctca.org/ timed out in web fetch and returned 403 via shell. No public member directory found; do not treat bylaws as coach data. Current contact subset also listed by UHSAA representatives PDF.
- **priority:** P3
- **verification_date:** 2026-10-04
- **source_system:** UCCTCA
- **checked_date:** 2026-10-04
- **applicable_states:**
  - UT
- **shortlist_rank:** null
- **source_id:** SRC-172
- **observed_routes:**
  - (empty list)

### SRC-173

UHSAA coaches association representatives

- **state:** UT
- **name:** UHSAA coaches association representatives
- **url:** https://uhsaa.org/coachassoc/CoachesAssocReps.pdf
- **category:** coaches_association_contacts
- **sports_scope:** XC and TF
- **school_or_coach_fields:** sport representative, school, email
- **coverage:** 2026-27 association presidents/representatives only
- **access_format:** 1-page PDF
- **verification_status:** opened_verified
- **evidence_url:** https://uhsaa.org/coachassoc/CoachesAssocReps.pdf
- **implementation_notes:** Both target sports list representative Garth Rushforth at Copper Hills. This is one representative, not all Utah coaches; useful validation only.
- **priority:** P3
- **verification_date:** 2026-10-04
- **source_system:** UHSAA
- **checked_date:** 2026-10-04
- **applicable_states:**
  - UT
- **shortlist_rank:** null
- **source_id:** SRC-173
- **observed_routes:**
  - (empty list)

### SRC-174

Pine View High Cross Country / Track

- **state:** UT
- **name:** Pine View High Cross Country / Track
- **url:** https://www.pineview.org/cross-country-track/
- **category:** school_staff_directory
- **sports_scope:** XC and TF
- **school_or_coach_fields:** coach; email; phone; team website
- **coverage:** One Utah high school
- **access_format:** HTML with obfuscated email text
- **verification_status:** opened_verified
- **evidence_url:** https://www.pineview.org/cross-country-track/
- **implementation_notes:** Official page shows coach Dave Holt and email visually reversed in text extraction. Decode only site-published presentation mechanism; never infer email patterns. Team blog links are supplemental sources and may be stale.
- **priority:** P2
- **verification_date:** 2026-10-04
- **source_system:** Washington County School District Utah
- **checked_date:** 2026-10-04
- **applicable_states:**
  - UT
- **shortlist_rank:** null
- **source_id:** SRC-174
- **observed_routes:**
  - (empty list)

### SRC-175

Southern Nevada Track & Cross Country Coaches Association directories

- **state:** NV
- **name:** Southern Nevada Track & Cross Country Coaches Association directories
- **url:** https://www.sntccca.org/
- **category:** coaches_association_directory
- **sports_scope:** Track and field and cross country
- **school_or_coach_fields:** school; boys/girls; classification; head coach; assistant coach; email; phone
- **coverage:** Southern Nevada region; schools plus non-coach operational contacts
- **access_format:** Public Google Sheets; anonymous CSV exports verified HTTP 200
- **verification_status:** opened_verified
- **evidence_url:** https://www.sntccca.org/
- **implementation_notes:** Two 2026-27 sheets linked from association. Track ID 1cs60d8y7YkfLlLVENZlgx4-kYOqfAPQeJU4fcQmp4oY; XC ID 1FijPtauqQ9N5v5-EZI0Ts3RirYIN0e2d-RQrFvla-7k. Append /export?format=csv to /spreadsheets/d/{id}. Self-maintained; strip preamble, timers, officials and coordinator rows.
- **priority:** P1
- **verification_date:** 2026-10-04
- **source_system:** SNTCCCA
- **recommended_rank:** 1
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NV
- **shortlist_rank:** 1
- **source_id:** SRC-175
- **observed_routes:**
  - (empty list)

### SRC-176

Nevada Department of Education public and private school inventories

- **state:** NV
- **name:** Nevada Department of Education public and private school inventories
- **url:** https://doe.nv.gov/school-and-district-information
- **category:** state_education_directory
- **sports_scope:** School inventory and coverage reconciliation
- **school_or_coach_fields:** school/state/NCES identifiers; name; grades; status; address; website; phone; principal email
- **coverage:** Statewide public/charter; separate licensed/exempt private directory
- **access_format:** XLSX files; actual downloads and shared-string headers verified
- **verification_status:** opened_verified
- **evidence_url:** https://doe.nv.gov/school-and-district-information
- **implementation_notes:** 2026-27 public school workbook is snapshot 2026-09-16. Public exact file: https://webapp-strapi-paas-prod-nde-001.azurewebsites.net/uploads/school_directory_9b69a05740.xlsx. Private route under NDE office student/school supports. Follow current landing links because hashed names can change. No coaches in inventories.
- **priority:** P1
- **verification_date:** 2026-10-04
- **source_system:** Nevada Department of Education
- **recommended_rank:** 2
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NV
- **shortlist_rank:** 2
- **source_id:** SRC-176
- **observed_routes:**
  - (empty list)

### SRC-177

Washoe County official school-site coach directories

- **state:** NV
- **name:** Washoe County official school-site coach directories
- **url:** https://reno.washoeschools.net/activities/athletics/welcome
- **category:** school_staff_directory
- **sports_scope:** XC and TF
- **school_or_coach_fields:** school; sport; coach name; email where supplied
- **coverage:** Northern Nevada district schools; Reno/North Valleys examples verified, not statewide
- **access_format:** Finalsite HTML and linked coach PDFs
- **verification_status:** opened_verified
- **evidence_url:** https://reno.washoeschools.net/activities/athletics/welcome
- **implementation_notes:** Reno welcome page publishes Coaches Directory; North Valleys staff page explicitly annotates XC/track roles. Enumerate district high-school websites, then actual athletics/coaches/staff pages. Do not confuse college University of Nevada coaching records.
- **priority:** P1
- **verification_date:** 2026-10-04
- **source_system:** Washoe County School District
- **recommended_rank:** 3
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NV
- **shortlist_rank:** 3
- **source_id:** SRC-177
- **observed_routes:**
  - (empty list)

### SRC-178

Southern Nevada Track coaches CSV

- **state:** NV
- **name:** Southern Nevada Track coaches CSV
- **url:** https://docs.google.com/spreadsheets/d/1cs60d8y7YkfLlLVENZlgx4-kYOqfAPQeJU4fcQmp4oY/export?format=csv
- **category:** public_coach_csv
- **sports_scope:** Track and field
- **school_or_coach_fields:** school, class, coach, email, phone
- **coverage:** Southern Nevada; includes operational non-school contacts
- **access_format:** CSV, 12719 decoded characters retrieved
- **verification_status:** opened_verified
- **evidence_url:** https://www.sntccca.org/
- **implementation_notes:** Anonymous GET succeeded with text/csv. Row headers are inconsistent: nominal SCHOOL,Name,,EMAIL,PHONE but some school rows use second cell for classification and third for coach. Parse semantically, retain raw row and validate school join. Do not write to editable source.
- **priority:** P1
- **verification_date:** 2026-10-04
- **source_system:** SNTCCCA
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NV
- **shortlist_rank:** null
- **source_id:** SRC-178
- **observed_routes:**
  - (empty list)

### SRC-179

Southern Nevada XC coaches CSV

- **state:** NV
- **name:** Southern Nevada XC coaches CSV
- **url:** https://docs.google.com/spreadsheets/d/1FijPtauqQ9N5v5-EZI0Ts3RirYIN0e2d-RQrFvla-7k/export?format=csv
- **category:** public_coach_csv
- **sports_scope:** Cross country
- **school_or_coach_fields:** school, class, coach, email, phone
- **coverage:** Southern Nevada; includes head and optional assistant contacts
- **access_format:** CSV, 10863 decoded characters retrieved
- **verification_status:** opened_verified
- **evidence_url:** https://www.sntccca.org/
- **implementation_notes:** Anonymous GET succeeded. Skip title/instruction rows and association/timing/official service contacts before school rows. Some rows contain AD or stale/contradictory contact data; preserve attribution and reconfirm against school.
- **priority:** P1
- **verification_date:** 2026-10-04
- **source_system:** SNTCCCA
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NV
- **shortlist_rank:** null
- **source_id:** SRC-179
- **observed_routes:**
  - (empty list)

### SRC-180

Nevada authorized private-school directory

- **state:** NV
- **name:** Nevada authorized private-school directory
- **url:** https://doe.nv.gov/offices/office-of-student-and-school-supports/private-schools/
- **category:** state_education_private_directory
- **sports_scope:** School discovery only
- **school_or_coach_fields:** school; address; city; county; license period; grades; school leader; email; phone
- **coverage:** Licensed and exempt private elementary/secondary schools
- **access_format:** XLSX; actual download verified
- **verification_status:** opened_verified
- **evidence_url:** https://doe.nv.gov/offices/office-of-student-and-school-supports/private-schools/
- **implementation_notes:** Exact file https://webapp-strapi-paas-prod-nde-001.azurewebsites.net/uploads/private_school_directory_b5a0270d46.xlsx fetched 200. Filter Grade Levels (Licensed), then school staff sites. Do not collect enrollment or personnel qualifications merely because workbook contains them.
- **priority:** P1
- **verification_date:** 2026-10-04
- **source_system:** Nevada Department of Education
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NV
- **shortlist_rank:** null
- **source_id:** SRC-180
- **observed_routes:**
  - (empty list)

### SRC-181

North Valleys High official staff directory

- **state:** NV
- **name:** North Valleys High official staff directory
- **url:** https://northvalleys.washoeschools.net/our-school/staff-directory
- **category:** school_staff_directory
- **sports_scope:** Track and field and XC
- **school_or_coach_fields:** staff name, role, school
- **coverage:** One Washoe high school; tested alternate template
- **access_format:** HTML staff directory
- **verification_status:** opened_verified
- **evidence_url:** https://northvalleys.washoeschools.net/our-school/staff-directory
- **implementation_notes:** Role strings explicitly include Cross Country Coach and Track Head Coach. Collect adults in target roles only; ignore unrelated staff biographies.
- **priority:** P2
- **verification_date:** 2026-10-04
- **source_system:** Washoe County School District
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NV
- **shortlist_rank:** null
- **source_id:** SRC-181
- **observed_routes:**
  - (empty list)

### SRC-182

NIAA publications and member-school directory route

- **state:** NV
- **name:** NIAA publications and member-school directory route
- **url:** https://niaa.prestosports.com/publications/index
- **category:** state_athletics_directory
- **sports_scope:** All sports; school seed
- **school_or_coach_fields:** member school directory link
- **coverage:** State association; current directory content not verified
- **access_format:** HTML landing/PDF link
- **verification_status:** blocked
- **evidence_url:** https://www.washoeschools.net/directory/student-activities-and-athletics/coaches/coach-documents-and-resources
- **implementation_notes:** Search index confirms NIAA Member School Directory label; direct landing and niaa.com requests returned 403. Washoe links https://www.niaa.com/publications/School_Directory_2019-2020.pdf which is stale and failed. Do not promote 2019-20 directory as current.
- **priority:** P3
- **verification_date:** 2026-10-04
- **source_system:** NIAA
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NV
- **shortlist_rank:** null
- **source_id:** SRC-182
- **observed_routes:**
  - (empty list)

### SRC-183

AIA authenticated administrative email directory

- **state:** AZ
- **name:** AIA authenticated administrative email directory
- **url:** https://admin.aiaonline.org/directory
- **category:** restricted_directory
- **sports_scope:** Administrative and sport contacts; content not inspected
- **school_or_coach_fields:** email directory content not accessible
- **coverage:** AIA account-based directory
- **access_format:** Authenticated HTML
- **verification_status:** login_required
- **evidence_url:** https://aiaonline.org/schools/100
- **implementation_notes:** Followed public school profile email-directory link; redirected to https://admin.aiaonline.org/login. No sign-in attempted. Public AIA coach names are separate from access to emails. Do not bypass authentication.
- **priority:** P3
- **verification_date:** 2026-10-04
- **source_system:** AIA
- **checked_date:** 2026-10-04
- **applicable_states:**
  - AZ
- **shortlist_rank:** null
- **source_id:** SRC-183
- **observed_routes:**
  - (empty list)

### SRC-184

Virginia Department of Education public-school alphabetical directory

- **state:** VA
- **name:** Virginia Department of Education public-school alphabetical directory
- **url:** https://www.va-doeapp.com/publicschoolsalphabetical.aspx?w=true
- **category:** state_education_directory
- **sports_scope:** School inventory; verify XC/track at school
- **school_or_coach_fields:** School name, street address, phone, principal, grade span, school-division website
- **coverage:** Virginia public schools, all grades
- **access_format:** Large public HTML table
- **verification_status:** opened_verified
- **evidence_url:** https://www.va-doeapp.com/publicschoolsalphabetical.aspx?w=true
- **implementation_notes:** Best clean public-school seed. Opened table includes 9–12 and combined-grade high schools. Filter grade span rather than name alone; deduplicate academies sharing a high-school campus. Principal is not coach. Follow division/school athletics staff pages for current adults.
- **priority:** P0
- **shortlist_rank:** 1
- **source_system:** VDOE
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - VA
- **source_id:** SRC-184
- **observed_routes:**
  - (empty list)

### SRC-185

VISAA member-school directory

- **state:** VA
- **name:** VISAA member-school directory
- **url:** https://www.visaa.org/schools
- **category:** private_athletics_directory
- **sports_scope:** All athletics; join sport-specific participation lists
- **school_or_coach_fields:** School, address, official website, mascot, conference, provisional membership notes
- **coverage:** Current VISAA private/independent members; excludes nonmembers
- **access_format:** Public HTML school cards
- **verification_status:** opened_verified
- **evidence_url:** https://www.visaa.org/schools
- **implementation_notes:** Strong private-school seed with outbound official domains and conference names. Resolve each school then follow athletics/team/coaches pages. Do not infer sport participation from association membership; join XC/track division lists.
- **priority:** P0
- **shortlist_rank:** 2
- **source_system:** VISAA
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - VA
- **source_id:** SRC-185
- **observed_routes:**
  - (empty list)

### SRC-186

VHSL alignment/classification hub

- **state:** VA
- **name:** VHSL alignment/classification hub
- **url:** https://www.vhsl.org/alignment/
- **category:** state_athletics_inventory
- **sports_scope:** All VHSL athletics, including XC/indoor/outdoor track
- **school_or_coach_fields:** School class, region, district and alignment documents
- **coverage:** VHSL public and approved nonpublic membership
- **access_format:** HTML hub with linked alignment documents
- **verification_status:** opened_verified
- **evidence_url:** https://www.vhsl.org/alignment/
- **implementation_notes:** Third distinct authoritative system. Follow current alignment documents and retain their effective years. Primary member-directory route is also listed below but returned 403. Alignment membership is a seed, not proof of a particular coach.
- **priority:** P0
- **shortlist_rank:** 3
- **source_system:** VHSL
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - VA
- **source_id:** SRC-186
- **observed_routes:**
  - (empty list)

### SRC-187

VHSL member-directory entry point

- **state:** VA
- **name:** VHSL member-directory entry point
- **url:** https://www.vhsl.org/about-vhsl/
- **category:** state_athletics_directory
- **sports_scope:** All school athletics
- **school_or_coach_fields:** Member-school address, phone, email as described by VHSL
- **coverage:** VHSL membership; page describes 315 schools
- **access_format:** HTML hub linking public directory
- **verification_status:** opened_verified
- **evidence_url:** https://www.vhsl.org/about-vhsl/
- **implementation_notes:** Directory link resolves to https://vhslreports.com/school_directory/?letter=C&schoolName=&state=VA and returned 403 in this session. The hub is verified; directory contents are not. Do not hard-code its stated school count as current completeness.
- **priority:** P1
- **shortlist_rank:** null
- **source_system:** VHSL
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - VA
- **source_id:** SRC-187
- **observed_routes:**
  - (empty list)

### SRC-188

VHSL Reports school directory

- **state:** VA
- **name:** VHSL Reports school directory
- **url:** https://vhslreports.com/school_directory/?letter=C&schoolName=&state=VA
- **category:** state_athletics_directory
- **sports_scope:** All school athletics
- **school_or_coach_fields:** School/contact fields anticipated from official directory description; not directly inspected
- **coverage:** VHSL members
- **access_format:** Query-parameter HTML directory
- **verification_status:** blocked
- **evidence_url:** https://www.vhsl.org/about-vhsl/
- **implementation_notes:** Official link clicked from VHSL About page; HTTP 403. Parameter names observed in official link, not a documented API. Do not brute-force letters/IDs or claim fields have been tested. Reassess lawful public access during implementation.
- **priority:** P2
- **shortlist_rank:** null
- **source_system:** VHSL
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - VA
- **source_id:** SRC-188
- **observed_routes:**
  - (empty list)

### SRC-189

VISAA cross-country participation documents

- **state:** VA
- **name:** VISAA cross-country participation documents
- **url:** https://www.visaa.org/sports/cross-country
- **category:** sport_participation
- **sports_scope:** Boys/girls cross-country
- **school_or_coach_fields:** School names and division by gender and effective cycle
- **coverage:** VISAA XC participants
- **access_format:** HTML hub with PDF documents
- **verification_status:** opened_verified
- **evidence_url:** https://www.visaa.org/sports/cross-country
- **implementation_notes:** Opened hub lists boys/girls 2026–2028 divisions and older versions. Extract school-only rows from latest divisions to filter private-school seeds; avoid collecting individual race results/student records.
- **priority:** P1
- **shortlist_rank:** null
- **source_system:** VISAA
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - VA
- **source_id:** SRC-189
- **observed_routes:**
  - (empty list)

### SRC-190

VISAA outdoor-track participation documents

- **state:** VA
- **name:** VISAA outdoor-track participation documents
- **url:** https://www.visaa.org/sports/track-and-field
- **category:** sport_participation
- **sports_scope:** Boys/girls outdoor track and field
- **school_or_coach_fields:** School names, divisions, effective cycle
- **coverage:** VISAA outdoor-track participants
- **access_format:** HTML hub with PDF documents
- **verification_status:** opened_verified
- **evidence_url:** https://www.visaa.org/sports/track-and-field
- **implementation_notes:** Opened page identifies 2026–2028 boys/girls divisions updated for 2027. Keep future-cycle participation separate from current season. Ignore heat sheets and individual all-state athlete lists.
- **priority:** P1
- **shortlist_rank:** null
- **source_system:** VISAA
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - VA
- **source_id:** SRC-190
- **observed_routes:**
  - (empty list)

### SRC-191

VDOE school-directories download hub

- **state:** VA
- **name:** VDOE school-directories download hub
- **url:** https://www.doe.virginia.gov/about-vdoe/virginia-school-directories
- **category:** state_education_exports
- **sports_scope:** School inventory
- **school_or_coach_fields:** School/principal contact CSV; division directories
- **coverage:** Virginia public schools; link to private accreditation inventory
- **access_format:** HTML hub; CSV/Excel links described in search result
- **verification_status:** blocked
- **evidence_url:** https://www.doe.virginia.gov/about-vdoe/virginia-school-directories
- **implementation_notes:** Search result advertises principal-contact CSV updated Aug 31, 2026, but repeated open returned 403. Prefer opened va-doeapp HTML source now; do not invent or report a tested CSV download URL.
- **priority:** P2
- **shortlist_rank:** null
- **source_system:** VDOE
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - VA
- **source_id:** SRC-191
- **observed_routes:**
  - (empty list)

### SRC-192

VCPE accreditation verification

- **state:** VA
- **name:** VCPE accreditation verification
- **url:** https://www.vcpe.org/Verify-School-Accreditation
- **category:** private_school_inventory
- **sports_scope:** School inventory; sport unknown
- **school_or_coach_fields:** Private-school identity, address, accreditation status; embedded list may require rendering
- **coverage:** VCPE-recognized PK–12 private-school locations
- **access_format:** HTML page; embedded/linked school list
- **verification_status:** opened_verified
- **evidence_url:** https://www.vcpe.org/Verify-School-Accreditation
- **implementation_notes:** Landing content verified. It instructs matching accreditation by school address. List records were not exposed in text extraction, so inspect rendered public list before coding. Accreditation coverage is narrower than every operating private school.
- **priority:** P1
- **shortlist_rank:** null
- **source_system:** VCPE accreditation verification
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - VA
- **source_id:** SRC-192
- **observed_routes:**
  - (empty list)

### SRC-193

Virginia Metro Athletic Conference member schools

- **state:** VA
- **name:** Virginia Metro Athletic Conference member schools
- **url:** https://virginiametroathletics.org/member-schools/
- **category:** regional_league_directory
- **sports_scope:** All conference sports; qualify XC/track separately
- **school_or_coach_fields:** School website, athletic website, AD name/email/phone
- **coverage:** Small Virginia private-school conference
- **access_format:** Public HTML directory
- **verification_status:** opened_verified
- **evidence_url:** https://virginiametroathletics.org/member-schools/
- **implementation_notes:** Verified school and athletic-department points of contact. Useful official-domain discovery and AD fallback, not statewide coach inventory. Keep AD records separately from coaching assignments.
- **priority:** P1
- **shortlist_rank:** null
- **source_system:** Virginia Metro Athletic Conference member schools
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - VA
- **source_id:** SRC-193
- **observed_routes:**
  - (empty list)

### SRC-194

Virginia Track Coaches Association

- **state:** VA
- **name:** Virginia Track Coaches Association
- **url:** https://runsignup.com/MemberOrg/VTCA
- **category:** coaches_association
- **sports_scope:** XC, indoor and outdoor track
- **school_or_coach_fields:** Association description and event/membership route; no public statewide roster verified
- **coverage:** Mixed middle-school, high-school, collegiate and club coaches
- **access_format:** Public RunSignup association page
- **verification_status:** opened_verified
- **evidence_url:** https://runsignup.com/MemberOrg/VTCA
- **implementation_notes:** Organization scope verified. Membership page is not a downloadable coach directory. Do not join or send contact forms for this task; any later data request needs separate authorization.
- **priority:** P2
- **shortlist_rank:** null
- **source_system:** Virginia Track Coaches Association
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - VA
- **source_id:** SRC-194
- **observed_routes:**
  - (empty list)

### SRC-195

MSHSAA member-school listing and coaching-roster route

- **state:** MO
- **name:** MSHSAA member-school listing and coaching-roster route
- **url:** https://www.mshsaa.org/Schools/SchoolListing.aspx
- **category:** state_athletics_directory
- **sports_scope:** Boys/girls XC and track via school sport links
- **school_or_coach_fields:** School name, member type, county, city, numeric school ID; linked coach names and head/assistant roles
- **coverage:** Statewide full members, affiliates and homeschool associations; includes junior high
- **access_format:** Public HTML table plus per-school HTML
- **verification_status:** opened_verified
- **evidence_url:** https://www.mshsaa.org/Schools/SchoolListing.aspx
- **implementation_notes:** Highest-value Missouri system. Crawl actual school links, filter high-school grades, then sport links and Coaches tabs. Verified sample https://www.mshsaa.org/MySchool/Coaches.aspx?alg=11&s=85 shows 2025–26 boys XC head/assistant names; no coach email shown. Preserve displayed season.
- **priority:** P0
- **shortlist_rank:** 1
- **source_system:** MSHSAA
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - MO
- **source_id:** SRC-195
- **observed_routes:**
  - (empty list)

### SRC-196

Missouri Track and Cross Country Coaches Association

- **state:** MO
- **name:** Missouri Track and Cross Country Coaches Association
- **url:** https://www.mtccca.org/
- **category:** coaches_association
- **sports_scope:** XC and track and field
- **school_or_coach_fields:** Selected coach names, school affiliations, sport/class awards; officer and clinic links
- **coverage:** Association community; public pages are selective, not all members
- **access_format:** Public Wix HTML
- **verification_status:** opened_verified
- **evidence_url:** https://www.mtccca.org/
- **implementation_notes:** Second distinct coach-bearing system. /xc opened with 2025 class-by-class boys/girls Coach of the Year names and schools. Use as corroboration or discovery only; no complete public membership roster verified. Do not infer an award winner still holds the role.
- **priority:** P1
- **shortlist_rank:** 2
- **source_system:** MTCCCA
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - MO
- **source_id:** SRC-196
- **observed_routes:**
  - (empty list)

### SRC-197

Missouri Nonpublic School Accrediting Association member schools

- **state:** MO
- **name:** Missouri Nonpublic School Accrediting Association member schools
- **url:** https://www.moqualityschools.com/member-listing1.html
- **category:** private_school_inventory
- **sports_scope:** School inventory; verify sports later
- **school_or_coach_fields:** Accredited school names and school links
- **coverage:** 265 nonpublic schools stated on page; many are elementary-only
- **access_format:** Public HTML directory
- **verification_status:** opened_verified
- **evidence_url:** https://www.moqualityschools.com/member-listing1.html
- **implementation_notes:** Third distinct scalable source for nonpublic gaps. Filter to high-school grades using official school pages or NCES; join to MSHSAA/private athletics, then staff pages. Accreditation membership does not establish track/XC participation.
- **priority:** P1
- **shortlist_rank:** 3
- **source_system:** MNSAA
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - MO
- **source_id:** SRC-197
- **observed_routes:**
  - (empty list)

### SRC-198

MSHSAA boys cross-country coaches sample

- **state:** MO
- **name:** MSHSAA boys cross-country coaches sample
- **url:** https://www.mshsaa.org/MySchool/Coaches.aspx?alg=11&s=85
- **category:** actual_coach_directory
- **sports_scope:** Boys cross-country
- **school_or_coach_fields:** School, displayed school year, head coach, assistant coaches and roles
- **coverage:** Hickman sample; same observed route family across schools
- **access_format:** Public server-rendered HTML
- **verification_status:** opened_verified
- **evidence_url:** https://www.mshsaa.org/MySchool/Coaches.aspx?alg=11&s=85
- **implementation_notes:** Displayed 2025–2026 roster; alg=11 and s=85 are observed values. Enumerate IDs from school links, not numeric guessing. Nearby Schedule pages inconsistently said no longer a member while home says full member, so quarantine conflicting status and validate against current official school page.
- **priority:** P0
- **shortlist_rank:** null
- **source_system:** MSHSAA
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - MO
- **source_id:** SRC-198
- **observed_routes:**
  - (empty list)

### SRC-199

MSHSAA boys track-and-field coaches sample

- **state:** MO
- **name:** MSHSAA boys track-and-field coaches sample
- **url:** https://www.mshsaa.org/MySchool/Coaches.aspx?alg=52&s=85
- **category:** actual_coach_directory
- **sports_scope:** Boys track and field
- **school_or_coach_fields:** Coach names/roles and school/year headings
- **coverage:** Hickman sample; route discovered via sport Schedule → Coaches
- **access_format:** Public HTML
- **verification_status:** opened_verified
- **evidence_url:** https://www.mshsaa.org/MySchool/Coaches.aspx?alg=52&s=85
- **implementation_notes:** Observed boys track alg=52. Girls XC schedule observed alg=12; its Coaches click failed, so that coach endpoint remains unverified. Discover girls track from actual links rather than assuming codes. Preserve source-year and conflicting-membership warning.
- **priority:** P1
- **shortlist_rank:** null
- **source_system:** MSHSAA
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - MO
- **source_id:** SRC-199
- **observed_routes:**
  - (empty list)

### SRC-200

Missouri DESE school directory

- **state:** MO
- **name:** Missouri DESE school directory
- **url:** https://dese.mo.gov/data-system-management/directory
- **category:** state_education_directory
- **sports_scope:** School inventory
- **school_or_coach_fields:** School/district IDs, names, addresses and administrative directory data
- **coverage:** Missouri public education inventory
- **access_format:** HTML hub, ArcGIS map and MCDS report links
- **verification_status:** opened_verified
- **evidence_url:** https://dese.mo.gov/data-system-management/directory
- **implementation_notes:** Hub says much data refreshes weekly. Both tested Data Download and All Districts PDF routes redirected to WebLogin; do not treat those as anonymous bulk exports. Public ArcGIS app opened as JS shell, with no feature layer verified.
- **priority:** P1
- **shortlist_rank:** null
- **source_system:** DESE
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - MO
- **source_id:** SRC-200
- **observed_routes:**
  - (empty list)

### SRC-201

Missouri DESE MCDS data-download report

- **state:** MO
- **name:** Missouri DESE MCDS data-download report
- **url:** https://apps.dese.mo.gov/MCDS/Reports/SSRS_Print.aspx?Reportid=ee8cf509-bf32-455e-b49e-c366a23b37db
- **category:** state_education_exports
- **sports_scope:** School inventory
- **school_or_coach_fields:** Unverified download schema
- **coverage:** Missouri school/district directory
- **access_format:** SSRS report endpoint
- **verification_status:** login_required
- **evidence_url:** https://dese.mo.gov/data-system-management/directory
- **implementation_notes:** Exact report URL obtained from DESE Data Download link. Redirected to WebLogin. No access attempt past login; do not send credentials or infer report parameter schema.
- **priority:** P2
- **shortlist_rank:** null
- **source_system:** DESE
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - MO
- **source_id:** SRC-201
- **observed_routes:**
  - (empty list)

### SRC-202

Missouri Christian School Athletic Association member schools

- **state:** MO
- **name:** Missouri Christian School Athletic Association member schools
- **url:** https://mocsaa.com/member-schools/
- **category:** private_athletics_directory
- **sports_scope:** Multiple school sports; validate XC/track
- **school_or_coach_fields:** School name, mascot, city/state, school detail links
- **coverage:** Association says 38 schools, but opened directory exposed only about ten named cards plus a placeholder
- **access_format:** Public HTML
- **verification_status:** opened_verified
- **evidence_url:** https://mocsaa.com/member-schools/
- **implementation_notes:** Useful private-Christian supplement but apparently incomplete/template-contaminated. Reject literal 'School Name/School Mascot' placeholder. Do not report extracted row count as complete membership. Validate each school and sport.
- **priority:** P2
- **shortlist_rank:** null
- **source_system:** MOCSAA
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - MO
- **source_id:** SRC-202
- **observed_routes:**
  - (empty list)

### SRC-203

OSSAARankings school directory and sport schedules

- **state:** OK
- **name:** OSSAARankings school directory and sport schedules
- **url:** https://www.ossaarankings.com/default.aspx?sc=1117&sel=ssch&st=OK
- **category:** state_athletics_directory
- **sports_scope:** Boys/girls XC and track menus
- **school_or_coach_fields:** School names, school links/IDs, sport schedule/class links
- **coverage:** Broad Oklahoma athletics membership including private schools
- **access_format:** Public ASP.NET HTML
- **verification_status:** opened_verified
- **evidence_url:** https://www.ossaarankings.com/default.aspx?sc=1117&sel=ssch&st=OK
- **implementation_notes:** Alphabetical school links were fully exposed. Discover exact links for each sport/gender; parse school identities separately from schedules. Query parameters are observed website routes, not a documented public API. Coach/contact data must be checked on school pages.
- **priority:** P0
- **shortlist_rank:** 1
- **source_system:** OSSAA
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - OK
- **source_id:** SRC-203
- **observed_routes:**
  - (empty list)

### SRC-204

Oklahoma State School and District Directory

- **state:** OK
- **name:** Oklahoma State School and District Directory
- **url:** https://oklahoma.gov/education/resources/state-school-directory.html
- **category:** state_education_directory
- **sports_scope:** School inventory
- **school_or_coach_fields:** Physical/mailing addresses, phone, email, website and district/site identifiers
- **coverage:** OSDE-accredited education sites
- **access_format:** Public HTML download hub
- **verification_status:** opened_verified
- **evidence_url:** https://oklahoma.gov/education/resources/state-school-directory.html
- **implementation_notes:** Hub opened and explicitly describes school/district downloads and field coverage; download click failed in this research tool, so file schema is not inspected. Use published School Directory link, not guessed API. Data is reported annually; note observation and reporting years.
- **priority:** P0
- **shortlist_rank:** 2
- **source_system:** OSDE
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - OK
- **source_id:** SRC-204
- **observed_routes:**
  - (empty list)

### SRC-205

NCPSA Oklahoma accredited-private-school directory

- **state:** OK
- **name:** NCPSA Oklahoma accredited-private-school directory
- **url:** https://ncpsa.org/directory/oklahoma/
- **category:** private_school_inventory
- **sports_scope:** School inventory; sport unknown
- **school_or_coach_fields:** School name, city, grade range, accreditor, detail links
- **coverage:** 55 accredited Oklahoma private schools stated; not all private schools
- **access_format:** Public HTML city groups
- **verification_status:** opened_verified
- **evidence_url:** https://ncpsa.org/directory/oklahoma/
- **implementation_notes:** Third accessible distinct inventory. Page says updated Sept 25, 2026. Filter high-school grade spans and verify official domain/athletics. Some grade values may be missing or malformed; do not discard without resolving school page.
- **priority:** P1
- **shortlist_rank:** 3
- **source_system:** NCPSA
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - OK
- **source_id:** SRC-205
- **observed_routes:**
  - (empty list)

### SRC-206

Oklahoma Cross Country and Track Coaches Association

- **state:** OK
- **name:** Oklahoma Cross Country and Track Coaches Association
- **url:** https://www.ohstrack.com/
- **category:** coaches_association
- **sports_scope:** XC and outdoor track
- **school_or_coach_fields:** Association resources, meet contacts, coach awards, advisory-board links
- **coverage:** State sport association; public material is selective
- **access_format:** HTML and linked PDFs
- **verification_status:** blocked
- **evidence_url:** https://www.ohstrack.com/
- **implementation_notes:** Search index shows 2026–27 advisory boards and current 2026 material; direct open timed out. Keep discovery lead but no claim of directly verified roster or working scrape. Member sign-up form is not a directory.
- **priority:** P1
- **shortlist_rank:** null
- **source_system:** OCCTCA
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - OK
- **source_id:** SRC-206
- **observed_routes:**
  - (empty list)

### SRC-207

OCCTCA advisory-board contact PDF

- **state:** OK
- **name:** OCCTCA advisory-board contact PDF
- **url:** https://www.ohstrack.com/occtcaofficeradvisorycontactinfo.pdf
- **category:** coaches_association_contacts
- **sports_scope:** Boys/girls XC and track
- **school_or_coach_fields:** Selected class representatives: name, school, email
- **coverage:** Board/advisory representatives only; not statewide coaches
- **access_format:** PDF
- **verification_status:** blocked
- **evidence_url:** https://www.ohstrack.com/occtcaofficeradvisorycontactinfo.pdf
- **implementation_notes:** Search result exposes representative coach-school-email rows, but PDF retrieval failed. Not a member directory and not verified current season. Preserve as optional lead requiring open/recency validation before ingest.
- **priority:** P2
- **shortlist_rank:** null
- **source_system:** OCCTCA
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - OK
- **source_id:** SRC-207
- **observed_routes:**
  - (empty list)

### SRC-208

OSSAA cross-country classifications and regional assignments

- **state:** OK
- **name:** OSSAA cross-country classifications and regional assignments
- **url:** https://ossaaillustrated.com/cross-country/
- **category:** sport_participation
- **sports_scope:** Boys/girls XC
- **school_or_coach_fields:** Classifications, regional assignments and participating school lists
- **coverage:** OSSAA XC programs
- **access_format:** WordPress HTML with linked PDFs
- **verification_status:** blocked
- **evidence_url:** https://ossaaillustrated.com/cross-country/
- **implementation_notes:** Search content lists 2025 boys/girls classifications and regional assignments. Direct fetch blocked/failed; do not equate search snapshot with opened documents. Follow current live links at implementation and keep season dates.
- **priority:** P2
- **shortlist_rank:** null
- **source_system:** OSSAA
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - OK
- **source_id:** SRC-208
- **observed_routes:**
  - (empty list)

### SRC-209

Heartland Christian Athletic Association directory

- **state:** OK
- **name:** Heartland Christian Athletic Association directory
- **url:** https://www.heartlandathletics.com/
- **category:** private_athletics_directory
- **sports_scope:** XC and track plus other sports
- **school_or_coach_fields:** HCAA_Directory.pdf advertised; fields not inspected
- **coverage:** Regional Christian athletics in OK/AR/KS
- **access_format:** TeamPages website and PDF document library
- **verification_status:** blocked
- **evidence_url:** https://www.heartlandathletics.com/
- **implementation_notes:** Search snapshot advertises HCAA_Directory.pdf updated two months before verification; site repeatedly timed out. No PDF download URL or roster schema verified. Filter state and high-school level when accessible; do not substitute Canadian HCAA.
- **priority:** P2
- **shortlist_rank:** null
- **source_system:** HCAA
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - OK
- **source_id:** SRC-209
- **observed_routes:**
  - (empty list)

### SRC-210

Oklahoma Christian Academy XC/track coaches

- **state:** OK
- **name:** Oklahoma Christian Academy XC/track coaches
- **url:** https://www.ocacademy.org/cc-track
- **category:** school_staff_fallback
- **sports_scope:** XC and track
- **school_or_coach_fields:** Coach name, head/assistant role, professional email; middle-school coach separately labeled
- **coverage:** One Edmond high-school program, not a statewide directory
- **access_format:** Public Wix HTML
- **verification_status:** opened_verified
- **evidence_url:** https://www.ocacademy.org/cc-track
- **implementation_notes:** Actual coach-bearing fallback example. Separate HS head/assistant records from explicitly MS-only role; same page contains both track and XC assistants. Use district/athletic directories to discover similar pages rather than crawling unrelated student content.
- **priority:** P1
- **shortlist_rank:** null
- **source_system:** Oklahoma Christian Academy XC/track coaches
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - OK
- **source_id:** SRC-210
- **observed_routes:**
  - (empty list)

### SRC-211

OPSAC member schools (quality warning)

- **state:** OK
- **name:** OPSAC member schools (quality warning)
- **url:** https://opsac.org/member-schools/
- **category:** private_school_inventory
- **sports_scope:** School inventory
- **school_or_coach_fields:** Directory heading only in extracted page
- **coverage:** Official OSDE-linked accrediting commission
- **access_format:** Public WordPress HTML
- **verification_status:** opened_verified
- **evidence_url:** https://oklahoma.gov/education/resources/state-school-directory.html
- **implementation_notes:** OSDE links to OPSAC, but member page exposed no school rows; homepage contained placeholder testimonials and unrelated gambling text. Do not use as clean source until integrity and actual directory records are checked. Prefer NCPSA plus OSDE now.
- **priority:** P3
- **shortlist_rank:** null
- **source_system:** OPSAC
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - OK
- **source_id:** SRC-211
- **observed_routes:**
  - (empty list)

### SRC-212

LHSAA 2025–2026 Coaches Directory

- **state:** LA
- **name:** LHSAA 2025–2026 Coaches Directory
- **url:** https://www.lhsaa.org/siteuploads/editorimg/file/Administration/25-26/LHSAA%20Coaches%20Directory%20-%202025%20-%202026.pdf
- **category:** actual_coach_directory
- **sports_scope:** All school sports, including XC/track; verify sport abbreviations
- **school_or_coach_fields:** Coach/school directory; exact column schema requires visual/OCR validation because extraction is garbled
- **coverage:** LHSAA member schools, public and private; 108 pages
- **access_format:** Public PDF
- **verification_status:** opened_verified
- **evidence_url:** https://www.lhsaa.org/handbook/cross-country
- **implementation_notes:** Highest-value Louisiana system, linked as School Administration Directory from current LHSAA navigation. Opened 108-page PDF, but extracted text is badly font-encoded. Render/OCR and verify representative pages before production. Pair with clean XC registered-coach PDF below; no assumption that all emails are present.
- **priority:** P0
- **shortlist_rank:** 1
- **source_system:** LHSAA
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - LA
- **source_id:** SRC-212
- **observed_routes:**
  - (empty list)

### SRC-213

Louisiana BESE-approved nonpublic-school list 2026–2027

- **state:** LA
- **name:** Louisiana BESE-approved nonpublic-school list 2026–2027
- **url:** https://doe.louisiana.gov/docs/default-source/nonpublic-schools/information---nps-2026-2027-approval-with-brumfield-v-dodd.pdf?sfvrsn=aa8fa3de_2
- **category:** state_private_school_inventory
- **sports_scope:** School inventory; sport unknown
- **school_or_coach_fields:** School name, site/district codes, parish, grade span, approval classification
- **coverage:** BESE-approved nonpublic schools; voluntary approval means not all private schools
- **access_format:** Public 19-page PDF table
- **verification_status:** opened_verified
- **evidence_url:** https://doe.louisiana.gov/topic-pages/louisiana-school-choice/nonpublic-schools/nonpublic-schools-resources
- **implementation_notes:** Second distinct authoritative system. Opened current PDF from LDOE Nonpublic School Resources. Filter high-school grades and preserve parish/code identifiers for joins. Do not ingest student/enrollment-by-race tables elsewhere on hub.
- **priority:** P0
- **shortlist_rank:** 2
- **source_system:** LDOE
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - LA
- **source_id:** SRC-213
- **observed_routes:**
  - (empty list)

### SRC-214

DirectAthletics Louisiana track team directory

- **state:** LA
- **name:** DirectAthletics Louisiana track team directory
- **url:** https://www.directathletics.com/leagues/track/89.html
- **category:** sport_team_directory
- **sports_scope:** Boys/girls track and field
- **school_or_coach_fields:** Team names and separate men's/women's team IDs/links
- **coverage:** Broad Louisiana teams on platform; historical/JV/club contamination possible
- **access_format:** Public HTML link table
- **verification_status:** opened_verified
- **evidence_url:** https://www.directathletics.com/leagues/track/89.html
- **implementation_notes:** Third distinct directly inspectable sport seed. Enumerate team links, collapse boys/girls identity at school layer while retaining program relation, exclude JV/club-only entries when outside scope. No coach-email API verified; school staff enrichment still needed.
- **priority:** P1
- **shortlist_rank:** 3
- **source_system:** DirectAthletics
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - LA
- **source_id:** SRC-214
- **observed_routes:**
  - (empty list)

### SRC-215

LHSAA registered XC coaches, September 22 2025

- **state:** LA
- **name:** LHSAA registered XC coaches, September 22 2025
- **url:** https://www.lhsaa.org/siteuploads/editorimg/file/Cross%20Country/2025%20XC/Registered%20coaches%20list%20as%20of%209-22-25.pdf
- **category:** actual_coach_directory
- **sports_scope:** Cross-country
- **school_or_coach_fields:** School, last name, first name
- **coverage:** Registered LHSAA XC coaches in dated 2025 snapshot; 14 pages
- **access_format:** Public text-readable PDF table
- **verification_status:** opened_verified
- **evidence_url:** https://www.lhsaa.org/handbook/cross-country
- **implementation_notes:** Clean bulk coach-school relation source. Current 2026 XC hub still points to this 2025 file. Treat as historical lead requiring current school verification, not current full staff or head-coach designation. No email/phone/role fields shown.
- **priority:** P0
- **shortlist_rank:** null
- **source_system:** LHSAA
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - LA
- **source_id:** SRC-215
- **observed_routes:**
  - (empty list)

### SRC-216

LHSAA public school-directory search

- **state:** LA
- **name:** LHSAA public school-directory search
- **url:** https://www.lhsaa.org/school-directory
- **category:** state_athletics_directory
- **sports_scope:** All LHSAA sports
- **school_or_coach_fields:** Searchable schools; form fields and results require interaction
- **coverage:** LHSAA members
- **access_format:** HTML search form
- **verification_status:** opened_verified
- **evidence_url:** https://www.lhsaa.org/school-directory
- **implementation_notes:** Correct current path obtained by clicking LHSAA nav. Earlier /schools/school-directory path failed; do not use the indexed www.www2 subdomain. Public form opened but result schema not inspected. Reuse current navigation to find renamed directory files.
- **priority:** P1
- **shortlist_rank:** null
- **source_system:** LHSAA
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - LA
- **source_id:** SRC-216
- **observed_routes:**
  - (empty list)

### SRC-217

LHSAA XC classifications and alignments hub

- **state:** LA
- **name:** LHSAA XC classifications and alignments hub
- **url:** https://www.lhsaa.org/handbook/cross-country
- **category:** sport_participation
- **sports_scope:** Boys/girls XC
- **school_or_coach_fields:** Regular-season alignment, division classification, school/registered-coach links
- **coverage:** LHSAA XC schools
- **access_format:** HTML and linked PDFs
- **verification_status:** opened_verified
- **evidence_url:** https://www.lhsaa.org/handbook/cross-country
- **implementation_notes:** Opened hub lists 2026–2027 six-division classification and boys/girls regular-season alignments. Use school-only lists to measure coach coverage; registered-coach PDF remains older. Avoid athlete result archives.
- **priority:** P1
- **shortlist_rank:** null
- **source_system:** LHSAA
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - LA
- **source_id:** SRC-217
- **observed_routes:**
  - (empty list)

### SRC-218

LHSAA outdoor-track hub

- **state:** LA
- **name:** LHSAA outdoor-track hub
- **url:** https://www.lhsaa.org/outdoor-track-and-field
- **category:** sport_participation
- **sports_scope:** Boys/girls outdoor track and field
- **school_or_coach_fields:** Sport alignment/classification and championship information links
- **coverage:** LHSAA outdoor-track programs
- **access_format:** HTML/PDF hub
- **verification_status:** opened_verified
- **evidence_url:** https://www.lhsaa.org/outdoor-track-and-field
- **implementation_notes:** Fetch only school/sport classification and coach administrative documents discovered from current hub. Keep indoor and outdoor assignments separate; a shared school does not prove same coach. No public JSON API verified.
- **priority:** P1
- **shortlist_rank:** null
- **source_system:** LHSAA
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - LA
- **source_id:** SRC-218
- **observed_routes:**
  - (empty list)

### SRC-219

Louisiana High School Coaches Association

- **state:** LA
- **name:** Louisiana High School Coaches Association
- **url:** https://www.lhsaa.org/lhsca
- **category:** coaches_association
- **sports_scope:** All sports; XC/LTFCA and track representatives
- **school_or_coach_fields:** Executive-council names, sport reps, convention/clinic resources
- **coverage:** Selective statewide association contacts
- **access_format:** Public HTML
- **verification_status:** opened_verified
- **evidence_url:** https://www.lhsaa.org/lhsca
- **implementation_notes:** Verified association page lists XC/LTFCA and track reps, not statewide membership. Useful corroboration or later authorized data-request target, not bulk coach extraction. No forms submitted or membership purchased.
- **priority:** P2
- **shortlist_rank:** null
- **source_system:** LHSAA
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - LA
- **source_id:** SRC-219
- **observed_routes:**
  - (empty list)

### SRC-220

ACEL 2026–2027 cross-country

- **state:** LA
- **name:** ACEL 2026–2027 cross-country
- **url:** https://www.theacel.com/26-27-cross-country.html
- **category:** private_athletics
- **sports_scope:** Christian-school XC; track link in navigation
- **school_or_coach_fields:** Commissioner and participation/season resources
- **coverage:** Louisiana Christian-school/homeschool association outside or overlapping LHSAA
- **access_format:** Public Weebly HTML; members area password-protected
- **verification_status:** opened_verified
- **evidence_url:** https://www.theacel.com/26-27-cross-country.html
- **implementation_notes:** Current season page opened. /members.html redirected to a 401 password form, so no member-directory data obtained. Public participation section may be incomplete; supplement with school official athletics pages rather than bypassing protected members area.
- **priority:** P2
- **shortlist_rank:** null
- **source_system:** ACEL
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - LA
- **source_id:** SRC-220
- **observed_routes:**
  - (empty list)

### SRC-221

Louisiana School Finder

- **state:** LA
- **name:** Louisiana School Finder
- **url:** https://louisianaschools.com/
- **category:** state_education_directory
- **sports_scope:** School inventory
- **school_or_coach_fields:** Official source describes school address, website and contacts; schema not inspected
- **coverage:** Louisiana schools and centers, all levels
- **access_format:** JavaScript app
- **verification_status:** blocked
- **evidence_url:** https://doe.louisiana.gov/docs/default-source/principal-support/louisiana-school-finder-legal-poster.pdf?sfvrsn=c0d5911f_4
- **implementation_notes:** Direct open failed; official LDOE poster identifies this as School Finder and describes basic school data. JS app/APIs not inspected, so no endpoint claimed. Filter out early-childhood centers if later accessed.
- **priority:** P2
- **shortlist_rank:** null
- **source_system:** LDOE
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - LA
- **source_id:** SRC-221
- **observed_routes:**
  - (empty list)

### SRC-222

SDCCTFCA 2024–2025 membership roster

- **state:** SD
- **name:** SDCCTFCA 2024–2025 membership roster
- **url:** https://cdn1.sportngin.com/attachments/document/cdfc-2949014/_1__SDCCTFCA_Membership_as_of_01-07-2025_.pdf
- **category:** actual_coach_directory
- **sports_scope:** Cross-country and track and field association membership
- **school_or_coach_fields:** Last name, first name, email, member title, school
- **coverage:** Association members as of Jan 7, 2025; not all statewide coaches; 10 pages
- **access_format:** Public text-readable PDF table
- **verification_status:** opened_verified
- **evidence_url:** https://www.sdhsca.org/page/show/6314203-membership
- **implementation_notes:** Highest-value direct coach source. Linked from https://www.sdhsca.org/page/show/6314203-membership . Preserve member title and school; membership does not identify exact sport/gender/head role unless explicitly printed. Validate 2026 employment and professional use of listed email before outreach.
- **priority:** P0
- **shortlist_rank:** 1
- **source_system:** SDCCTFCA
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - SD
- **source_id:** SRC-222
- **observed_routes:**
  - (empty list)

### SRC-223

South Dakota Department of Education educational directory

- **state:** SD
- **name:** South Dakota Department of Education educational directory
- **url:** https://doe.sd.gov/ofm/edudir.aspx
- **category:** state_education_directory
- **sports_scope:** School inventory
- **school_or_coach_fields:** Public/nonpublic/tribal system names, system IDs, directory detail links; principal/school Excel advertised
- **coverage:** Statewide accredited public, nonpublic and tribal/BIE-related systems plus other entity types
- **access_format:** Public HTML index, district query pages, Excel/PDF links
- **verification_status:** opened_verified
- **evidence_url:** https://doe.sd.gov/ofm/edudir.aspx
- **implementation_notes:** Second distinct clean seed system. Principal/school Excel link labeled updated Sept 8, 2026, though binary click failed here. Follow actual district links and retain IDs, school grade spans and system type. Exclude community-support and preschool-only entities.
- **priority:** P0
- **shortlist_rank:** 2
- **source_system:** SDDOE
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - SD
- **source_id:** SRC-223
- **observed_routes:**
  - (empty list)

### SRC-224

SDHSAA official member directory on Bound

- **state:** SD
- **name:** SDHSAA official member directory on Bound
- **url:** https://www.gobound.com/sd/associations/sdhsaa/schools
- **category:** state_athletics_directory
- **sports_scope:** All sports; join XC/track alignment
- **school_or_coach_fields:** School identity and school links expected; records not exposed in text-only extraction
- **coverage:** SDHSAA members including public/private and co-ops
- **access_format:** Public JavaScript app
- **verification_status:** opened_verified
- **evidence_url:** https://sdhsaa.com/
- **implementation_notes:** Third distinct official athletics system. SDHSAA homepage links this exact Member Directory URL. Only application shell opened; rendered rows/contact schema not verified. Use official school URLs when available, and stop at any login wall; no private Bound API documented.
- **priority:** P1
- **shortlist_rank:** 3
- **source_system:** SDHSAA/Bound
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - SD
- **source_id:** SRC-224
- **observed_routes:**
  - (empty list)

### SRC-225

SDHSAA cross-country participation hub

- **state:** SD
- **name:** SDHSAA cross-country participation hub
- **url:** https://sdhsaa.com/activity/cross-country/
- **category:** sport_participation
- **sports_scope:** Boys/girls cross-country
- **school_or_coach_fields:** XC school alignment and regional-meet links
- **coverage:** SDHSAA XC programs
- **access_format:** HTML hub; AthleticNET alignment directory
- **verification_status:** opened_verified
- **evidence_url:** https://sdhsaa.com/activity/cross-country/
- **implementation_notes:** Official alignment link resolves to https://www.athletic.net/cross-country/usa/high-school/south-dakota . AthleticNET opened only shell. Use school/team identity links, avoid athlete roster/results collection. Hub confirms 2026 season.
- **priority:** P1
- **shortlist_rank:** null
- **source_system:** SDHSAA/Bound
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - SD
- **source_id:** SRC-225
- **observed_routes:**
  - (empty list)

### SRC-226

SDHSAA track-and-field hub

- **state:** SD
- **name:** SDHSAA track-and-field hub
- **url:** https://sdhsaa.com/activity/track-field/
- **category:** sport_participation
- **sports_scope:** Boys/girls track and field
- **school_or_coach_fields:** Track alignment, sanctioned-meet and coach-resource links
- **coverage:** SDHSAA track programs
- **access_format:** HTML hub with Bound/AthleticNET links
- **verification_status:** opened_verified
- **evidence_url:** https://sdhsaa.com/activity/track-field/
- **implementation_notes:** Track alignments now link to Bound, while XC links to AthleticNET. Do not assume same extraction provider across sports. Target current program membership; no need to ingest individual top-performance lists.
- **priority:** P1
- **shortlist_rank:** null
- **source_system:** SDHSAA/Bound
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - SD
- **source_id:** SRC-226
- **observed_routes:**
  - (empty list)

### SRC-227

SDHSAA athletics cooperatives

- **state:** SD
- **name:** SDHSAA athletics cooperatives
- **url:** https://sdhsaa.com/athletics-cooperatives/
- **category:** cooperative_crosswalk
- **sports_scope:** BXC/GXC/BTF/GTF among other sports
- **school_or_coach_fields:** Cooperative program identity and member-school relationship when rendered
- **coverage:** State athletic co-ops
- **access_format:** HTML page with dynamic cooperative content
- **verification_status:** opened_verified
- **evidence_url:** https://sdhsaa.com/athletics-cooperatives/
- **implementation_notes:** Page explicitly defines BXC, GXC, BTF, GTF. Actual cooperative table did not appear in text extraction, so browser validation required. Model a cooperative as program linked to constituent schools; avoid inventing a different coach for every constituent.
- **priority:** P1
- **shortlist_rank:** null
- **source_system:** SDHSAA/Bound
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - SD
- **source_id:** SRC-227
- **observed_routes:**
  - (empty list)

### SRC-228

SDCCTFCA membership/area-alignment hub

- **state:** SD
- **name:** SDCCTFCA membership/area-alignment hub
- **url:** https://www.sdhsca.org/page/show/6314203-membership
- **category:** coaches_association
- **sports_scope:** XC and track
- **school_or_coach_fields:** Dated membership and school-area alignment PDFs
- **coverage:** SDCCTFCA member/award areas
- **access_format:** Public HTML with SportsEngine CDN PDFs
- **verification_status:** opened_verified
- **evidence_url:** https://www.sdhsca.org/page/show/6314203-membership
- **implementation_notes:** Use hub to rediscover current filenames rather than repeatedly relying on 2025 roster. Opened page advertises 2025–26 A/B and AA area alignments; root /sdcctfca failed in this tool even though indexed newer. Associations and official school lists are separate coverage sets.
- **priority:** P1
- **shortlist_rank:** null
- **source_system:** SDCCTFCA
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - SD
- **source_id:** SRC-228
- **observed_routes:**
  - (empty list)

### SRC-229

PIAA school directory

- **state:** PA
- **name:** PIAA school directory
- **url:** https://www.piaa.org/schools/directory/default.aspx
- **category:** state_athletics_directory
- **sports_scope:** Boys/girls XC and track
- **school_or_coach_fields:** School names, type, district, address, AD/admin contacts, sponsored sports from detail pages
- **coverage:** PIAA member high schools and junior/middle schools; public/private
- **access_format:** Public ASP.NET directory hub/list/detail
- **verification_status:** opened_verified
- **evidence_url:** https://www.piaa.org/schools/directory/default.aspx
- **implementation_notes:** First distinct athletics seed. Hub opened and provides district/alphabetical routes. /schools/directory/list.aspx was search-visible but fetch failed; sample details.aspx?ID=11513 search result shows AD email and sports, not coaches. Do not claim coach emails from PIAA. Preserve grade/type filters.
- **priority:** P0
- **shortlist_rank:** 1
- **source_system:** PIAA
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - PA
- **source_id:** SRC-229
- **observed_routes:**
  - (empty list)

### SRC-230

Pennsylvania EdNA output files

- **state:** PA
- **name:** Pennsylvania EdNA output files
- **url:** https://www.edna.pa.gov/Screens/Extracts/wfExtracts.aspx
- **category:** state_education_exports
- **sports_scope:** School inventory
- **school_or_coach_fields:** Education entity identifiers, names, addresses, administrators, public-school grades and entity relationships
- **coverage:** Recognized public, private and nonpublic educational entities statewide
- **access_format:** Public ASP.NET export forms; Excel outputs
- **verification_status:** opened_verified
- **evidence_url:** https://www.edna.pa.gov/Screens/Extracts/wfExtracts.aspx
- **implementation_notes:** Second distinct broad system. Public Schools and Private/Nonpublic extract forms both opened without login; binary reports were not generated. Select open schools, relevant categories and high-school grades; do not treat a diocese/LEA as a school.
- **priority:** P0
- **shortlist_rank:** 2
- **source_system:** EdNA
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - PA
- **source_id:** SRC-230
- **observed_routes:**
  - (empty list)

### SRC-231

PAISAA independent-school member directory

- **state:** PA
- **name:** PAISAA independent-school member directory
- **url:** https://www.paisaasports.org/page/show/925102-members
- **category:** private_athletics_directory
- **sports_scope:** Independent-school sports including XC/track
- **school_or_coach_fields:** School name/address/phone; member details with league and website
- **coverage:** Pennsylvania independent-school athletics outside/overlapping PIAA
- **access_format:** SportsEngine HTML directory and school detail pages
- **verification_status:** blocked
- **evidence_url:** https://www.paisaasports.org/page/show/5033416-friends-select-school
- **implementation_notes:** Third distinct coverage-critical system. Directory is search-indexed but direct open failed. Successfully opened member detail https://www.paisaasports.org/page/show/5033416-friends-select-school showing actual school/league record. Root directory retrieval must be validated before production; do not claim entire roster was downloaded.
- **priority:** P1
- **shortlist_rank:** null
- **source_system:** PAISAA
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - PA
- **source_id:** SRC-231
- **observed_routes:**
  - (empty list)

### SRC-232

EdNA public-school export form

- **state:** PA
- **name:** EdNA public-school export form
- **url:** https://www.edna.pa.gov/Screens/Extracts/wfExtractPublicSchools.aspx
- **category:** state_education_exports
- **sports_scope:** Public school inventory
- **school_or_coach_fields:** Open/closed, LEA/school categories, county and intermediate-unit filters; Excel report
- **coverage:** Public entities/schools
- **access_format:** Public ASP.NET form
- **verification_status:** opened_verified
- **evidence_url:** https://www.edna.pa.gov/Screens/Extracts/wfExtractPublicSchools.aspx
- **implementation_notes:** Observed form supports public categories and status. Keep viewstate/eventvalidation/cookies if automating server form; derive actual controls from HTML rather than guessing field names. Export output schema not tested. Join school-grade extract to target HS.
- **priority:** P1
- **shortlist_rank:** null
- **source_system:** EdNA
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - PA
- **source_id:** SRC-232
- **observed_routes:**
  - (empty list)

### SRC-233

EdNA private and nonpublic export form

- **state:** PA
- **name:** EdNA private and nonpublic export form
- **url:** https://www.edna.pa.gov/Screens/Extracts/wfPNP.aspx
- **category:** state_private_school_inventory
- **sports_scope:** Private/nonpublic school inventory
- **school_or_coach_fields:** Open/closed, approved private, licensed academic, nonpublic/nonlicensed, county/IU categories
- **coverage:** Recognized private/nonpublic entities, including some nonschool categories
- **access_format:** Public ASP.NET form
- **verification_status:** opened_verified
- **evidence_url:** https://www.edna.pa.gov/Screens/Extracts/wfPNP.aspx
- **implementation_notes:** Observed categories include dioceses and other entities, so filter carefully. Separate school records from umbrella organizations. Directory records seed official domains/contact pages; they do not prove sport or coaching employment.
- **priority:** P1
- **shortlist_rank:** null
- **source_system:** EdNA
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - PA
- **source_id:** SRC-233
- **observed_routes:**
  - (empty list)

### SRC-234

Track and Field Coaches Association of Greater Philadelphia handbook

- **state:** PA
- **name:** Track and Field Coaches Association of Greater Philadelphia handbook
- **url:** https://www.tfcaofgp.org/coaches-handbook/
- **category:** coaches_association_contacts
- **sports_scope:** Primarily indoor track; regional XC/outdoor connections
- **school_or_coach_fields:** Officer, meet-director and league-representative coach names, selected school affiliations, emails/phones
- **coverage:** Regional leadership only, not statewide membership
- **access_format:** Public HTML
- **verification_status:** opened_verified
- **evidence_url:** https://www.tfcaofgp.org/coaches-handbook/
- **implementation_notes:** Direct coach/contact-bearing supplement with 2025–26 schedule. Keep roles as association/league representative, not automatically head coach. Member Schools link exists but returned bot-challenge page. Never mistake approximately 100 participating schools statement for 100 directory contacts.
- **priority:** P1
- **shortlist_rank:** 3
- **source_system:** TFCAofGP
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - PA
- **source_id:** SRC-234
- **observed_routes:**
  - (empty list)

### SRC-235

Friends Schools League

- **state:** PA
- **name:** Friends Schools League
- **url:** https://www.fslathletics.org/
- **category:** regional_private_league
- **sports_scope:** Boys/girls XC and track among league sports
- **school_or_coach_fields:** Member-school names, descriptions, links and sport schedules
- **coverage:** Nine Philadelphia-area independent schools; includes out-of-state member possibility
- **access_format:** Public HTML
- **verification_status:** opened_verified
- **evidence_url:** https://www.fslathletics.org/
- **implementation_notes:** Accessible private-school fallback if PAISAA directory is unavailable. Filter actual state, retain member-school official domains; use team/staff pages for coach names. League schedule opponents are not all league members.
- **priority:** P1
- **shortlist_rank:** null
- **source_system:** FSL
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - PA
- **source_id:** SRC-235
- **observed_routes:**
  - (empty list)

### SRC-236

Inter-Academic League membership/history

- **state:** PA
- **name:** Inter-Academic League membership/history
- **url:** https://interacathletics.com/sports/2020/2/21/GEN_0221202650.aspx
- **category:** regional_private_league
- **sports_scope:** XC and track among league sports
- **school_or_coach_fields:** Current member-school list in history article; official school navigation
- **coverage:** Philadelphia-area independent league
- **access_format:** Public SIDEARM HTML; limited extracted page
- **verification_status:** opened_verified
- **evidence_url:** https://interacathletics.com/sports/2020/2/21/GEN_0221202650.aspx
- **implementation_notes:** Source names current league members but also many historic schools: do not extract every school mentioned as current. Use explicit current-members paragraph and verify outgoing athletics domains. No coach roster/API inspected.
- **priority:** P2
- **shortlist_rank:** null
- **source_system:** Inter-Ac
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - PA
- **source_id:** SRC-236
- **observed_routes:**
  - (empty list)

### SRC-237

Pennsylvania Track and Field Coaches Association

- **state:** PA
- **name:** Pennsylvania Track and Field Coaches Association
- **url:** https://www.ptfca.org/
- **category:** coaches_association
- **sports_scope:** State indoor/outdoor track; scope of XC roster unverified
- **school_or_coach_fields:** No directory fields directly verified
- **coverage:** Statewide association lead
- **access_format:** Website
- **verification_status:** blocked
- **evidence_url:** https://www.tfcaofgp.org/coaches-handbook/
- **implementation_notes:** Site inaccessible via research tool. Organization/URL corroborated by primary USATF association directory and regional coaching handbook; no complete public coach database or current member list confirmed. Keep as lead only.
- **priority:** P2
- **shortlist_rank:** null
- **source_system:** PTFCA
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - PA
- **source_id:** SRC-237
- **observed_routes:**
  - (empty list)

### SRC-238

Philadelphia Catholic League

- **state:** PA
- **name:** Philadelphia Catholic League
- **url:** https://aopathletics.org/
- **category:** regional_parochial_league
- **sports_scope:** XC and track among school athletics
- **school_or_coach_fields:** School navigation/athletics links; sport menus need rendered inspection
- **coverage:** Archdiocese of Philadelphia Catholic athletics
- **access_format:** Public website, limited text extraction
- **verification_status:** opened_verified
- **evidence_url:** https://aopathletics.org/
- **implementation_notes:** Landing opened but limited content means school directory and contact fields not validated. Use only observed school navigation during implementation and cross-check with PIAA/EdNA. Not a statewide coach directory.
- **priority:** P2
- **shortlist_rank:** null
- **source_system:** PCL
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - PA
- **source_id:** SRC-238
- **observed_routes:**
  - (empty list)

### SRC-239

Kansas Educational Directory Reports

- **state:** KS
- **name:** Kansas Educational Directory Reports
- **url:** https://uapps.ksde.gov/directory_rpts/default.aspx
- **category:** state_education_exports
- **sports_scope:** School inventory
- **school_or_coach_fields:** High-school directories, accredited/nonaccredited private directories, active-building and accredited-organization raw data
- **coverage:** Kansas public/private education entities; current organizational reports 2026–2027
- **access_format:** Public ASP.NET reports; PDF and Excel options
- **verification_status:** opened_verified
- **evidence_url:** https://uapps.ksde.gov/directory_rpts/default.aspx
- **implementation_notes:** Best clean seed. Opened report selector explicitly offers Active Building Report (Excel) and accredited/nonaccredited nonpublic lists (PDF). Educator report is separately labeled 2023–24 updated Oct 28, 2024; do not treat it as current coach database. Outputs not generated here.
- **priority:** P0
- **shortlist_rank:** 1
- **source_system:** KSDE
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - KS
- **source_id:** SRC-239
- **observed_routes:**
  - (empty list)

### SRC-240

KSHSAA school leagues directory

- **state:** KS
- **name:** KSHSAA school leagues directory
- **url:** https://kshsaa.org/Public/General/Leagues.cfm
- **category:** state_athletics_directory
- **sports_scope:** All athletics, including XC/track
- **school_or_coach_fields:** League names, league-school detail routes
- **coverage:** KSHSAA public/private members and approved-school categories
- **access_format:** Public HTML legacy hub; some routes migrating to React
- **verification_status:** opened_verified
- **evidence_url:** https://kshsaa.org/Public/General/Leagues.cfm
- **implementation_notes:** Second authoritative system. Opened league index has many statewide leagues. School-search homepage redirects to /react/ and approved-schools to /react/approved-schools, both JS shells in this tool. Follow actual current links and verify rendered data; no API documented.
- **priority:** P0
- **shortlist_rank:** 2
- **source_system:** KSHSAA
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - KS
- **source_id:** SRC-240
- **observed_routes:**
  - (empty list)

### SRC-241

Kansas Cross Country and Track and Field Coaches Association

- **state:** KS
- **name:** Kansas Cross Country and Track and Field Coaches Association
- **url:** https://www.kcctfca.com/
- **category:** coaches_association
- **sports_scope:** XC and track and field
- **school_or_coach_fields:** Selected clinic speaker coaches and schools, award links, association contacts
- **coverage:** Statewide association; public data selective
- **access_format:** Public Wix HTML
- **verification_status:** opened_verified
- **evidence_url:** https://www.kcctfca.com/
- **implementation_notes:** Third distinct coach-bearing system. Winter clinic page opened with 2026 speaker school/college affiliations. Filter to high-school coaches and verify current staff roles. No complete public membership/email roster found; association homepage/contacts are not all coaches.
- **priority:** P1
- **shortlist_rank:** 3
- **source_system:** KCCTFCA
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - KS
- **source_id:** SRC-241
- **observed_routes:**
  - (empty list)

### SRC-242

KSHSAA approved schools

- **state:** KS
- **name:** KSHSAA approved schools
- **url:** https://kshsaa.org/Public/General/ApprovedSchools.cfm
- **category:** nonmember_athletics_inventory
- **sports_scope:** Approved schools allowed to play members; verify XC/track
- **school_or_coach_fields:** School names, official websites, school-detail contacts in indexed legacy content
- **coverage:** Approved nonmember private and homeschool schools
- **access_format:** Legacy HTML URL redirects to React
- **verification_status:** opened_verified
- **evidence_url:** https://kshsaa.org/Public/General/ApprovedSchools.cfm
- **implementation_notes:** Observed redirect to https://www.kshsaa.org/react/approved-schools; only JS shell opened. Search snapshot lists 2025–26 schools but must not become a current verified roster. Approved status is distinct from full KSHSAA membership.
- **priority:** P1
- **shortlist_rank:** null
- **source_system:** KSHSAA
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - KS
- **source_id:** SRC-242
- **observed_routes:**
  - (empty list)

### SRC-243

KSDE annual education directory PDFs

- **state:** KS
- **name:** KSDE annual education directory PDFs
- **url:** https://ksde.gov/data-and-reporting/directories
- **category:** state_education_directory
- **sports_scope:** School inventory
- **school_or_coach_fields:** Contact/enrollment information for public/private schools/districts
- **coverage:** Kansas educational community
- **access_format:** Public HTML with annual PDF links
- **verification_status:** opened_verified
- **evidence_url:** https://ksde.gov/data-and-reporting/directories
- **implementation_notes:** Opened hub lists 2025–26 and older directory PDFs. Prefer live 2026–27 report selector for fresh organizational data; annual PDF is fallback snapshot. Principal/superintendent contacts are not coaching assignments.
- **priority:** P1
- **shortlist_rank:** null
- **source_system:** KSDE
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - KS
- **source_id:** SRC-243
- **observed_routes:**
  - (empty list)

### SRC-244

KCCTFCA 2026 winter clinic

- **state:** KS
- **name:** KCCTFCA 2026 winter clinic
- **url:** https://www.kcctfca.com/winter-coaching-clinic
- **category:** coaches_association_contacts
- **sports_scope:** XC and track coaching disciplines
- **school_or_coach_fields:** Speaker names, school affiliation, coaching topic
- **coverage:** Small selective speaker panel mixing HS/college/independent coaches
- **access_format:** Public HTML
- **verification_status:** opened_verified
- **evidence_url:** https://www.kcctfca.com/winter-coaching-clinic
- **implementation_notes:** Useful coach-school leads only; do not collect registration forms or pay for membership. Exclude out-of-state and college coaches from target output. Retain event date as provenance rather than pretending current staff verification.
- **priority:** P2
- **shortlist_rank:** null
- **source_system:** KCCTFCA
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - KS
- **source_id:** SRC-244
- **observed_routes:**
  - (empty list)

### SRC-245

Flint Hills Christian school sports/coaches

- **state:** KS
- **name:** Flint Hills Christian school sports/coaches
- **url:** https://flinthillschristianschool.com/sports-home/
- **category:** school_staff_fallback
- **sports_scope:** XC and track
- **school_or_coach_fields:** Explicit high-school head/assistant coach names; separate junior-high coach
- **coverage:** One independent Christian school
- **access_format:** Public HTML
- **verification_status:** opened_verified
- **evidence_url:** https://flinthillschristianschool.com/sports-home/
- **implementation_notes:** Actual named-coach fallback for private programs. Page distinguishes high school from junior high, and identifies track/XC roles. School-level extraction template only; not evidence every Kansas private school has similar fields.
- **priority:** P1
- **shortlist_rank:** null
- **source_system:** Flint Hills Christian school sports/coaches
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - KS
- **source_id:** SRC-245
- **observed_routes:**
  - (empty list)

### SRC-246

KCAA track team directory on AthleticNET

- **state:** KS
- **name:** KCAA track team directory on AthleticNET
- **url:** https://www.athletic.net/track-and-field-outdoor/division/80932
- **category:** private_sport_team_directory
- **sports_scope:** Kansas Christian Athletic Association track
- **school_or_coach_fields:** Team names and team links from search snapshot
- **coverage:** Subset of private/homeschool track programs
- **access_format:** JavaScript directory
- **verification_status:** search_only
- **evidence_url:** https://www.athletic.net/track-and-field-outdoor/division/80932
- **implementation_notes:** Search snapshot lists KCAA teams including Christ Preparatory, Flint Hills Christian, Sunrise and Veritas. This exact division URL was not opened; season/division currency unverified. Use official KSHSAA approved inventory first and validate public page before scraping.
- **priority:** P2
- **shortlist_rank:** null
- **source_system:** AthleticNET
- **verified_on:** 2026-10-04
- **checked_date:** 2026-10-04
- **applicable_states:**
  - KS
- **source_id:** SRC-246
- **observed_routes:**
  - (empty list)

### SRC-247

NCES CCD downloadable files selector

- **state:** NATIONAL
- **name:** NCES CCD downloadable files selector
- **url:** https://nces.ed.gov/ccd/files.asp#Fiscal:2,SchoolYearId:39,Page:1
- **category:** school_inventory
- **sports_scope:** School inventory; no sport participation or coach contacts
- **school_or_coach_fields:** NCESSCH; state school ID; school/district name; grade flags; address; phone; WEBSITE; operational status
- **coverage:** Public elementary/secondary schools nationwide; filter requested states and high-school grades
- **access_format:** Official HTML selector linking ZIP CSV/SAS and XLSX companion files
- **verification_status:** verified_live_html_and_selector_data_2026-10-04
- **evidence_url:** https://nces.ed.gov/ccd/files.asp#Fiscal:2,SchoolYearId:39,Page:1
- **implementation_notes:** Preferred discovery page. Legacy pubschuniv.asp direct HTTPS returned an outdated table in this environment; use current files selector. Selector currently labels collection v.2a while directory download filename remains 1a. Save actual source filename, checksum, download timestamp, survey year, schema and per-component version.
- **priority:** 1
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NATIONAL
- **shortlist_rank:** null
- **source_id:** SRC-247
- **observed_routes:**
  - (empty list)

### SRC-248

NCES CCD 2024–25 school directory CSV/SAS ZIP

- **state:** NATIONAL
- **name:** NCES CCD 2024–25 school directory CSV/SAS ZIP
- **url:** https://nces.ed.gov/ccd/Data/zip/ccd_sch_029_2425_w_1a_073025.zip
- **category:** school_inventory_bulk
- **sports_scope:** School inventory; no sport participation or coach contacts
- **school_or_coach_fields:** NCESSCH; SCH_NAME; LEA_NAME; ST_SCHID; ST/LSTATE; address; PHONE; WEBSITE; SY_STATUS/UPDATED_STATUS; G_9_OFFERED through G_12_OFFERED; GSLO/GSHI; LEVEL
- **coverage:** 102,178 all-grade rows in downloaded file; 29,065 offer any grade 9–12; 29,061 after school-year active-status filter, before state/school-type/athletics checks
- **access_format:** ZIP containing CSV and SAS7BDAT
- **verification_status:** verified_downloaded_parsed_2026-10-04
- **evidence_url:** https://nces.ed.gov/ccd/files.asp#Fiscal:2,SchoolYearId:39,Page:1
- **implementation_notes:** Use any of G_9_OFFERED,G_10_OFFERED,G_11_OFFERED,G_12_OFFERED == Yes. Active school-year statuses are 1,3,4,5,8; exclude 2 Closed,6 Inactive,7 Future. Updated status can supersede school-year status for current outreach; preserve both. Keep combined K–12 and other schools if they offer HS grades. File includes territories; limit to requested states. NCES school ID must be a string with leading zeros.
- **priority:** 1
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NATIONAL
- **shortlist_rank:** null
- **source_id:** SRC-248
- **observed_routes:**
  - (empty list)

### SRC-249

NCES CCD 2024–25 school directory companion

- **state:** NATIONAL
- **name:** NCES CCD 2024–25 school directory companion
- **url:** https://nces.ed.gov/ccd/xls/SY_2024-25_SCH_Directory_Companion_2026-005d.xlsx
- **category:** data_dictionary
- **sports_scope:** School inventory; no sport participation or coach contacts
- **school_or_coach_fields:** Field definitions; grade flags; status codes; notes
- **coverage:** Companion for CCD school directory
- **access_format:** XLSX
- **verification_status:** verified_downloaded_inspected_2026-10-04
- **evidence_url:** https://nces.ed.gov/ccd/xls/SY_2024-25_SCH_Directory_Companion_2026-005d.xlsx
- **implementation_notes:** This is the schema authority. Status codes verified: 1 Open,2 Closed,3 New,4 Added,5 Changed Boundary/Agency,6 Inactive,7 Future,8 Reopened. Check updated status separately and do not infer athletic participation.
- **priority:** 1
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NATIONAL
- **shortlist_rank:** null
- **source_id:** SRC-249
- **observed_routes:**
  - (empty list)

### SRC-250

NCES EDGE public-school administrative REST layer

- **state:** NATIONAL
- **name:** NCES EDGE public-school administrative REST layer
- **url:** https://nces.ed.gov/opengis/rest/services/K12_School_Locations/EDGE_ADMINDATA_PUBLICSCH_2425/MapServer/1
- **category:** school_inventory_api
- **sports_scope:** School inventory; no sport participation or coach contacts
- **school_or_coach_fields:** NCESSCH; SCH_NAME; LEA_NAME; address; PHONE; GSLO/GSHI; SCHOOL_LEVEL; STATUS/SY_STATUS_TEXT; SCHOOL_TYPE_TEXT; LATCOD/LONCOD
- **coverage:** Nationwide public-school administrative and geographic layer; 101,110 rows in live count
- **access_format:** Documented ArcGIS REST; JSON/GeoJSON/PBF; query endpoint
- **verification_status:** verified_schema_and_live_query_2026-10-04
- **evidence_url:** https://nces.ed.gov/opengis/rest/services/K12_School_Locations/EDGE_ADMINDATA_PUBLICSCH_2425/MapServer/1
- **implementation_notes:** Layer is /1, not /0. /0 returned Layer not found. MaxRecordCount 2000, pagination/orderBy supported. Query only needed fields, returnGeometry=false. Iterate resultOffset/resultRecordCount with orderByFields=OBJECTID; verify count and exceededTransferLimit, or retrieve IDs then batch. LSTATE may contain trailing spaces. This layer omits WEBSITE; CCD ZIP is more useful as canonical school inventory. GSHI in 09/10/11/12 yielded 28,843 preliminary rows, but exact CCD grade flags are preferable and include atypical grade spans.
- **priority:** 1
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NATIONAL
- **shortlist_rank:** null
- **source_id:** SRC-250
- **observed_routes:**
  - (empty list)

### SRC-251

NCES EDGE public-school geocode REST layer

- **state:** NATIONAL
- **name:** NCES EDGE public-school geocode REST layer
- **url:** https://nces.ed.gov/opengis/rest/services/K12_School_Locations/EDGE_GEOCODE_PUBLICSCH_2425/MapServer/0
- **category:** school_geography_api
- **sports_scope:** School inventory; no sport participation or coach contacts
- **school_or_coach_fields:** NCESSCH; NAME; address; STATE; county; LAT/LON; locale; SCHOOLYEAR
- **coverage:** Public elementary and secondary school locations, all grades
- **access_format:** ArcGIS REST JSON/GeoJSON/PBF
- **verification_status:** verified_schema_2026-10-04
- **evidence_url:** https://catalog.data.gov/dataset/public-school-locations-2024-25
- **implementation_notes:** NO grade span or school-level fields in this layer. Do not count all rows as high schools. Join CCD on NCESSCH. MaxRecordCount 2000. Public-domain metadata verified.
- **priority:** 1
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NATIONAL
- **shortlist_rank:** null
- **source_id:** SRC-251
- **observed_routes:**
  - (empty list)

### SRC-252

NCES EDGE public-school geocode bulk ZIP

- **state:** NATIONAL
- **name:** NCES EDGE public-school geocode bulk ZIP
- **url:** https://nces.ed.gov/programs/edge/data/EDGE_GEOCODE_PUBLICSCH_2425.zip
- **category:** school_geography_bulk
- **sports_scope:** School inventory; no sport participation or coach contacts
- **school_or_coach_fields:** School identity and location/geographic fields
- **coverage:** Nationwide public-school geocodes, 2024–25
- **access_format:** ZIP
- **verification_status:** verified_official_catalog_link_not_downloaded_2026-10-04
- **evidence_url:** https://catalog.data.gov/dataset/public-school-locations-2024-25
- **implementation_notes:** Download link published in Data.gov NCES metadata. Join CCD directory for website, grades, type and status.
- **priority:** 2
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NATIONAL
- **shortlist_rank:** null
- **source_id:** SRC-252
- **observed_routes:**
  - (empty list)

### SRC-253

NCES PSS data and documentation index

- **state:** NATIONAL
- **name:** NCES PSS data and documentation index
- **url:** https://nces.ed.gov/surveys/pss/pssdata.asp
- **category:** private_school_inventory
- **sports_scope:** School inventory; no sport participation or coach contacts
- **school_or_coach_fields:** Private-school bulk data, record layout, codebook, questionnaire, methodological frame files
- **coverage:** Biennial nationwide private school survey; 2023–24 currently downloadable
- **access_format:** HTML download index; ZIP CSV/SAS/SPSS; PDF; frame CSV
- **verification_status:** verified_live_html_2026-10-04
- **evidence_url:** https://nces.ed.gov/surveys/pss/pssdata.asp
- **implementation_notes:** Private bulk source exists. Page says 2025–26 results expected spring 2027. Permanent PPIN is cross-year join key. Survey weights are for statistical estimates, not duplicate school generation. Nonresponding/frame entities may require supplemental state/private-association sources.
- **priority:** 1
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NATIONAL
- **shortlist_rank:** null
- **source_id:** SRC-253
- **observed_routes:**
  - (empty list)

### SRC-254

NCES PSS 2023–24 public-use CSV ZIP

- **state:** NATIONAL
- **name:** NCES PSS 2023–24 public-use CSV ZIP
- **url:** https://nces.ed.gov/surveys/pss/zip/pss2324_pu_csv.zip
- **category:** private_school_inventory_bulk
- **sports_scope:** School inventory; no sport participation or coach contacts
- **school_or_coach_fields:** PPIN; PINST; PL_ADD/PL_CIT/PL_STABB/PL_ZIP; mailing address; PPHONE; grade flags P265/P275/P285/P295; latitude/longitude; recoded grade span
- **coverage:** 22,510 distinct school records in downloaded CSV; 8,750 offer one or more grades 9–12
- **access_format:** ZIP containing pss2324_pu.csv
- **verification_status:** verified_downloaded_parsed_2026-10-04
- **evidence_url:** https://nces.ed.gov/surveys/pss/pssdata.asp
- **implementation_notes:** Exact HS filter: any of P265,P275,P285,P295 == 1. Values 1=Yes,2=No. HIGR2024 is recoded: 14=9th,15=10th,16=11th,17=12th; never use numeric HIGR>=9 as if it were a grade number. Preserve combined schools. No coach fields or school website field found in this CSV. School search About Data currently says 22,502 responding schools; downloaded CSV/codebook both report 22,510, so do not interchange counts.
- **priority:** 1
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NATIONAL
- **shortlist_rank:** null
- **source_id:** SRC-254
- **observed_routes:**
  - (empty list)

### SRC-255

NCES PSS 2023–24 record layout

- **state:** NATIONAL
- **name:** NCES PSS 2023–24 record layout
- **url:** https://nces.ed.gov/surveys/pss/pdf/layout2023_24.pdf
- **category:** data_dictionary
- **sports_scope:** School inventory; no sport participation or coach contacts
- **school_or_coach_fields:** Public-use column names/types/descriptions
- **coverage:** 2023–24 PSS CSV
- **access_format:** PDF
- **verification_status:** verified_downloaded_text_inspected_2026-10-04
- **evidence_url:** https://nces.ed.gov/surveys/pss/pdf/layout2023_24.pdf
- **implementation_notes:** Defines P265/P275/P285/P295 as ninth/tenth/eleventh/twelfth grade offered. Companion codebook contains value encodings.
- **priority:** 1
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NATIONAL
- **shortlist_rank:** null
- **source_id:** SRC-255
- **observed_routes:**
  - (empty list)

### SRC-256

NCES PSS 2023–24 codebook

- **state:** NATIONAL
- **name:** NCES PSS 2023–24 codebook
- **url:** https://nces.ed.gov/surveys/pss/pdf/codebook2023_24.pdf
- **category:** data_dictionary
- **sports_scope:** School inventory; no sport participation or coach contacts
- **school_or_coach_fields:** Question labels; values; recode meanings; imputation flags
- **coverage:** 2023–24 PSS
- **access_format:** PDF
- **verification_status:** verified_downloaded_text_inspected_2026-10-04
- **evidence_url:** https://nces.ed.gov/surveys/pss/pdf/codebook2023_24.pdf
- **implementation_notes:** Use grade-offered flags rather than LEVEL alone: combined schools also serve high-school students. Can retain imputation flags for confidence/provenance.
- **priority:** 1
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NATIONAL
- **shortlist_rank:** null
- **source_id:** SRC-256
- **observed_routes:**
  - (empty list)

### SRC-257

NCES PSS 2023–24 methodological frame file

- **state:** NATIONAL
- **name:** NCES PSS 2023–24 methodological frame file
- **url:** https://nces.ed.gov/surveys/pss/xls/2023-24_PSS_Frame_Data.csv
- **category:** private_school_coverage_reconciliation
- **sports_scope:** School inventory; no sport participation or coach contacts
- **school_or_coach_fields:** PSS frame and response/eligibility tracking variables; consult dictionary before use
- **coverage:** Survey frame, including entities omitted from responding-school data
- **access_format:** CSV
- **verification_status:** verified_link_in_official_live_index_not_downloaded_2026-10-04
- **evidence_url:** https://nces.ed.gov/surveys/pss/pssdata.asp
- **implementation_notes:** Coverage reconciliation source, not automatically a valid list of active high schools. Dictionary: https://nces.ed.gov/surveys/pss/pdf/2023-24_PSS_Frame_File_Data_Dictionary.pdf
- **priority:** 2
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NATIONAL
- **shortlist_rank:** null
- **source_id:** SRC-257
- **observed_routes:**
  - (empty list)

### SRC-258

NCES EDGE private-school geocode REST layer

- **state:** NATIONAL
- **name:** NCES EDGE private-school geocode REST layer
- **url:** https://nces.ed.gov/opengis/rest/services/K12_School_Locations/EDGE_GEOCODE_PRIVATESCH_2324/MapServer/0
- **category:** private_school_geography_api
- **sports_scope:** School inventory; no sport participation or coach contacts
- **school_or_coach_fields:** PPIN; NAME; address; STATE; county; LAT/LON; locale; SCHOOLYEAR
- **coverage:** Private-school locations, 2023–24
- **access_format:** ArcGIS REST JSON/GeoJSON/PBF
- **verification_status:** verified_schema_2026-10-04
- **evidence_url:** https://catalog.data.gov/dataset/private-school-locations-2023-24
- **implementation_notes:** No grade span fields. Join PSS CSV on PPIN. Public-domain metadata verified. Bulk ZIP https://nces.ed.gov/programs/edge/data/EDGE_GEOCODE_PRIVATESCH_2324.zip is officially catalogued, not downloaded in this pass.
- **priority:** 2
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NATIONAL
- **shortlist_rank:** null
- **source_id:** SRC-258
- **observed_routes:**
  - (empty list)

### SRC-259

NCES public-school search

- **state:** NATIONAL
- **name:** NCES public-school search
- **url:** https://nces.ed.gov/ccd/schoolsearch/index.asp
- **category:** school_identity_verification
- **sports_scope:** School inventory; no sport participation or coach contacts
- **school_or_coach_fields:** School name/NCES ID; address; phone; grade span; type
- **coverage:** Public schools; source banner 2024–25 and 2025–26
- **access_format:** HTML search/detail pages
- **verification_status:** verified_search_index_content_2026-10-04
- **evidence_url:** https://nces.ed.gov/ccd/schoolsearch/index.asp
- **implementation_notes:** Useful for manual identity checks; prefer CCD/EDGE bulk for 25-state ingestion. Not a coaches directory.
- **priority:** 2
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NATIONAL
- **shortlist_rank:** null
- **source_id:** SRC-259
- **observed_routes:**
  - (empty list)

### SRC-260

NFHS state association directory

- **state:** NATIONAL
- **name:** NFHS state association directory
- **url:** https://nfhs.org/about/state-association-directory
- **category:** association_discovery
- **sports_scope:** All interscholastic sports including track/XC
- **school_or_coach_fields:** Member and affiliate state associations; links/contact details
- **coverage:** National interscholastic association discovery
- **access_format:** Dynamic HTML
- **verification_status:** indexed_content_live_web_fetch_403_2026-10-04
- **evidence_url:** https://nfhs.org/about/state-association-directory
- **implementation_notes:** Association discovery, not school coach inventory. Page has member and affiliate tabs; private associations may require affiliate coverage. Do not claim NFHS self-service spreadsheet is publicly accessible: https://utilities.nfhs.org/schooldirectory/selfservice requires sign-in.
- **priority:** 2
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NATIONAL
- **shortlist_rank:** null
- **source_id:** SRC-260
- **observed_routes:**
  - (empty list)

### SRC-261

NHSTFXCCA state track/XC associations

- **state:** NATIONAL
- **name:** NHSTFXCCA state track/XC associations
- **url:** https://www.nhstfxcca.org/state-associations/
- **category:** track_xc_association_discovery
- **sports_scope:** High-school track & field and cross country
- **school_or_coach_fields:** State coaching-association links
- **coverage:** National state-association hub; uneven state availability
- **access_format:** HTML
- **verification_status:** verified_live_html_2026-10-04
- **evidence_url:** https://www.nhstfxcca.org/state-associations/
- **implementation_notes:** Especially useful seed list for sport-specific state directories. Live site has migrated from indexed Google Sites content to WordPress; retain final URLs and recheck individual links.
- **priority:** 1
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NATIONAL
- **shortlist_rank:** null
- **source_id:** SRC-261
- **observed_routes:**
  - (empty list)

### SRC-262

NIAAA state athletic-administrator association directory

- **state:** NATIONAL
- **name:** NIAAA state athletic-administrator association directory
- **url:** https://niaaa.org/about-the-niaaa/state-association-directory
- **category:** association_discovery
- **sports_scope:** All high-school sports
- **school_or_coach_fields:** State athletic-director association links
- **coverage:** 50 states and DC links
- **access_format:** HTML
- **verification_status:** verified_opened_2026-10-04
- **evidence_url:** https://niaaa.org/about-the-niaaa/state-association-directory
- **implementation_notes:** Useful AD fallback and association discovery; no national school coach export exposed here.
- **priority:** 2
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NATIONAL
- **shortlist_rank:** null
- **source_id:** SRC-262
- **observed_routes:**
  - (empty list)

### SRC-263

NASO state resource guide

- **state:** NATIONAL
- **name:** NASO state resource guide
- **url:** https://www.naso.org/resources/state-resource-guide/
- **category:** association_discovery
- **sports_scope:** All high-school sports
- **school_or_coach_fields:** State high-school athletic/activity association links
- **coverage:** National association links
- **access_format:** HTML
- **verification_status:** verified_opened_2026-10-04
- **evidence_url:** https://www.naso.org/resources/state-resource-guide/
- **implementation_notes:** Fallback to NFHS dynamic page; some outbound links are older and must be reverified. Not individual coach data.
- **priority:** 3
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NATIONAL
- **shortlist_rank:** null
- **source_id:** SRC-263
- **observed_routes:**
  - (empty list)

### SRC-264

NHSACA national coaches association

- **state:** NATIONAL
- **name:** NHSACA national coaches association
- **url:** https://nhsaca.org/
- **category:** association_discovery
- **sports_scope:** All high-school sports including track/XC
- **school_or_coach_fields:** State links; coach awards; school/coach news
- **coverage:** National and partner-state coverage
- **access_format:** HTML/PDF links
- **verification_status:** verified_opened_2026-10-04
- **evidence_url:** https://nhsaca.org/
- **implementation_notes:** Useful discovery and corroboration; awards are sparse and may be historical. Not a complete current coach directory.
- **priority:** 3
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NATIONAL
- **shortlist_rank:** null
- **source_id:** SRC-264
- **observed_routes:**
  - (empty list)

### SRC-265

NFHS coaches awards

- **state:** NATIONAL
- **name:** NFHS coaches awards
- **url:** https://nfhs.org/stories/awards/coaches-association-awards
- **category:** coach_corroboration
- **sports_scope:** Track/XC and other sports
- **school_or_coach_fields:** Coach name; sport; state; award year/level
- **coverage:** Sparse state/section/national award recipients
- **access_format:** HTML search
- **verification_status:** verified_indexed_2026-10-04
- **evidence_url:** https://nfhs.org/stories/awards/coaches-association-awards
- **implementation_notes:** Do not infer current appointment from an old award; follow school staff page. Legacy searchable database https://tools.nfhs.org/CoachAwards/CoachAwards/Search is indexed.
- **priority:** 3
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NATIONAL
- **shortlist_rank:** null
- **source_id:** SRC-265
- **observed_routes:**
  - (empty list)

### SRC-266

SIDEARM public staff-directory pattern: Miramonte

- **state:** NATIONAL
- **name:** SIDEARM public staff-directory pattern: Miramonte
- **url:** https://gomats.org/staff-directory
- **category:** platform_adapter_example
- **sports_scope:** Track & field and cross country plus all school sports
- **school_or_coach_fields:** Coach name; sport; level; title; linked staff bio; published email where available
- **coverage:** One verified high-school example of a reusable platform pattern, not national coverage
- **access_format:** HTML table and linked staff pages
- **verification_status:** verified_opened_2026-10-04
- **evidence_url:** https://gomats.org/staff-directory
- **implementation_notes:** Verified XC and track head-coach rows. Footer identifies SIDEARM. Discover actual directory links from each school; /staff-directory is a pattern, not a guaranteed endpoint. No public bulk coaches API documented in this pass.
- **priority:** 1
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NATIONAL
- **shortlist_rank:** null
- **source_id:** SRC-266
- **observed_routes:**
  - (empty list)

### SRC-267

Mascot Media public staff-directory pattern: Richland Northeast

- **state:** NATIONAL
- **name:** Mascot Media public staff-directory pattern: Richland Northeast
- **url:** https://www.rneathletics.com/directory
- **category:** platform_adapter_example
- **sports_scope:** Track & field and cross country plus other school sports
- **school_or_coach_fields:** Coach name; role; public work/team email; optional phone/ext
- **coverage:** One verified school example, reusable Mascot Media family
- **access_format:** HTML cards
- **verification_status:** verified_opened_2026-10-04
- **evidence_url:** https://www.rneathletics.com/directory
- **implementation_notes:** Footer identifies Mascot Media; page contains XC, girls track and boys track coaches with public email. Follow observed links; do not assume a private JSON API.
- **priority:** 1
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NATIONAL
- **shortlist_rank:** null
- **source_id:** SRC-267
- **observed_routes:**
  - (empty list)

### SRC-268

Finalsite public school athletics staff pattern: Starr’s Mill

- **state:** NATIONAL
- **name:** Finalsite public school athletics staff pattern: Starr’s Mill
- **url:** https://smhs.fcboe.org/athletics/coaching-staff
- **category:** platform_adapter_example
- **sports_scope:** Track & field and cross country plus other school sports
- **school_or_coach_fields:** Sport; coach name; varsity/JV level; public email; page season
- **coverage:** One verified school example, reusable school CMS pattern
- **access_format:** HTML text and mailto links
- **verification_status:** verified_opened_2026-10-04
- **evidence_url:** https://smhs.fcboe.org/athletics/coaching-staff
- **implementation_notes:** Page explicitly says 2025–26 and includes XC/track. Current download date is not proof of 2026–27 appointment. Record season and confidence; use latest school page when conflicting.
- **priority:** 1
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NATIONAL
- **shortlist_rank:** null
- **source_id:** SRC-268
- **observed_routes:**
  - (empty list)

### SRC-269

Edlio public athletics staff pattern: Del Norte

- **state:** NATIONAL
- **name:** Edlio public athletics staff pattern: Del Norte
- **url:** https://delnorte.powayusd.com/apps/pages/coaching-staff
- **category:** platform_adapter_example
- **sports_scope:** Track & field and cross country plus other school sports
- **school_or_coach_fields:** Sport; coach name; published email links; season categories
- **coverage:** One verified school example, reusable school CMS pattern
- **access_format:** HTML table and email links
- **verification_status:** verified_opened_2026-10-04
- **evidence_url:** https://delnorte.powayusd.com/apps/pages/coaching-staff
- **implementation_notes:** Page lists XC and track. Text renderer masks email labels; inspect ordinary linked email representation when accessible, never guess email. No login is needed for viewed directory.
- **priority:** 1
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NATIONAL
- **shortlist_rank:** null
- **source_id:** SRC-269
- **observed_routes:**
  - (empty list)

### SRC-270

School-owned directory pattern: Wellesley

- **state:** NATIONAL
- **name:** School-owned directory pattern: Wellesley
- **url:** https://wellesleyps.org/athletics/staff-directory/
- **category:** platform_adapter_example
- **sports_scope:** Indoor/outdoor track and cross country plus other sports
- **school_or_coach_fields:** Coach name; varsity/JV/assistant role; indoor/outdoor/XC; public email
- **coverage:** One verified school-owned searchable directory
- **access_format:** HTML alphabetical directory
- **verification_status:** verified_opened_2026-10-04
- **evidence_url:** https://wellesleyps.org/athletics/staff-directory/
- **implementation_notes:** Example has separate indoor and outdoor rows and assistant coaches. Keep person and assignment tables separate; school link to BigTeams does not mean the directory itself is BigTeams.
- **priority:** 1
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NATIONAL
- **shortlist_rank:** null
- **source_id:** SRC-270
- **observed_routes:**
  - (empty list)

### SRC-271

School-owned Google Sites coaching directory: Hopatcong

- **state:** NATIONAL
- **name:** School-owned Google Sites coaching directory: Hopatcong
- **url:** https://sites.google.com/hopatcongschools.org/hopatcongathletics/important-info/coaches-directory
- **category:** platform_adapter_example
- **sports_scope:** Track & field and cross country plus other sports
- **school_or_coach_fields:** Coach name; sport; boys/girls; public school email
- **coverage:** One indexed school-owned directory pattern
- **access_format:** Google Sites HTML
- **verification_status:** verified_indexed_2026-10-04
- **evidence_url:** https://sites.google.com/hopatcongschools.org/hopatcongathletics/important-info/coaches-directory
- **implementation_notes:** Use only school-controlled/officially linked sites; public contact listing is not consent for automated outreach. No coach contact data copied into this catalog.
- **priority:** 2
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NATIONAL
- **shortlist_rank:** null
- **source_id:** SRC-271
- **observed_routes:**
  - (empty list)

### SRC-272

PlayOn Sites / VNN platform

- **state:** NATIONAL
- **name:** PlayOn Sites / VNN platform
- **url:** https://www.playonsports.com/products/vnn
- **category:** platform_discovery
- **sports_scope:** All school sports
- **school_or_coach_fields:** School/team website content; coach contacts only if published by school
- **coverage:** Participating school sites nationwide
- **access_format:** School-hosted HTML/JS; product documentation
- **verification_status:** verified_opened_redirect_2026-10-04
- **evidence_url:** https://www.playonsports.com/products/vnn
- **implementation_notes:** Old https://www.vnnsports.net/vnn-partner-schools/ is still indexed as a school list but LIVE redirects here; do not treat old index as current enumeration. Discover official athletics sites from school websites. No documented public nationwide coaches API verified.
- **priority:** 2
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NATIONAL
- **shortlist_rank:** null
- **source_id:** SRC-272
- **observed_routes:**
  - (empty list)

### SRC-273

DragonFly Public Directory documentation

- **state:** NATIONAL
- **name:** DragonFly Public Directory documentation
- **url:** https://help.dragonflymax.com/knowledge-base/what-is-the-public-directory-dragonfly-athletics-help-center
- **category:** platform_access_limited
- **sports_scope:** All school sports including track/XC
- **school_or_coach_fields:** Staff/team names; positions; email; phone, per official documentation
- **coverage:** Participating schools and associations
- **access_format:** Product documentation; directory access varies
- **verification_status:** verified_documentation_opened_access_not_tested_2026-10-04
- **evidence_url:** https://help.dragonflymax.com/knowledge-base/what-is-the-public-directory-dragonfly-athletics-help-center
- **implementation_notes:** Documentation says directory is for other schools/associations and setup requires login. Public in product name does not establish unrestricted anonymous bulk access. Use state-association published directory links; do not register as a coach or request school staff access.
- **priority:** 3
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NATIONAL
- **shortlist_rank:** null
- **source_id:** SRC-273
- **observed_routes:**
  - (empty list)

### SRC-274

MaxPreps track & field school/team discovery

- **state:** NATIONAL
- **name:** MaxPreps track & field school/team discovery
- **url:** https://www.maxpreps.com/track-field/
- **category:** sport_team_discovery
- **sports_scope:** Track & field
- **school_or_coach_fields:** School/team names; state; city; team links; possible coach name on school pages
- **coverage:** Sport-specific participating/listed high schools; not exhaustive NCES inventory
- **access_format:** HTML state/team pages
- **verification_status:** verified_opened_2026-10-04
- **evidence_url:** https://www.maxpreps.com/track-field/
- **implementation_notes:** Verified example https://www.maxpreps.com/nc/track-field/schools/. Follow actual state links rather than synthesizing coverage claims. Team presence useful for program validation. No documented public bulk coach-contact API verified; do not collect student roster records.
- **priority:** 2
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NATIONAL
- **shortlist_rank:** null
- **source_id:** SRC-274
- **observed_routes:**
  - (empty list)

### SRC-275

MileSplit team search

- **state:** NATIONAL
- **name:** MileSplit team search
- **url:** https://www.milesplit.com/teams
- **category:** sport_team_discovery
- **sports_scope:** Track & field and cross country
- **school_or_coach_fields:** Team/school identity and team links
- **coverage:** Track/XC teams listed by platform; includes non-HS entities
- **access_format:** HTML search; JS/login/paywall behavior varies
- **verification_status:** verified_opened_search_shell_2026-10-04
- **evidence_url:** https://www.milesplit.com/teams
- **implementation_notes:** Discovery/corroboration only. Full underlying coach fields not verified. Avoid student profiles/results and do not assume access through undocumented APIs or paid content.
- **priority:** 3
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NATIONAL
- **shortlist_rank:** null
- **source_id:** SRC-275
- **observed_routes:**
  - (empty list)

### SRC-276

SchoolDigger documented school API

- **state:** NATIONAL
- **name:** SchoolDigger documented school API
- **url:** https://developer.schooldigger.com/docs
- **category:** licensed_school_inventory_api
- **sports_scope:** School inventory; no sport participation or coach contacts
- **school_or_coach_fields:** School identity; grades; address; school type; NCES IDs including private school identifier; inspect current response schema
- **coverage:** National public/private school directory; API/vendor licensed access
- **access_format:** Documented REST JSON with appID/appKey; licensed bulk option
- **verification_status:** verified_documentation_opened_no_credentials_used_2026-10-04
- **evidence_url:** https://developer.schooldigger.com/docs
- **implementation_notes:** Documented school list endpoint /v2.4/schools; version 3.0 also exists in current change log. Confirm selected version, license, retention and plan limits. Not a coach API. Primary NCES bulk is preferable before paying for redundant identity data.
- **priority:** 3
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NATIONAL
- **shortlist_rank:** null
- **source_id:** SRC-276
- **observed_routes:**
  - (empty list)

### SRC-277

GreatSchools NearbySchools documented API

- **state:** NATIONAL
- **name:** GreatSchools NearbySchools documented API
- **url:** https://www.greatschools.org/gk/about/api-developer-resources/
- **category:** licensed_school_inventory_api
- **sports_scope:** School inventory; no sport participation or coach contacts
- **school_or_coach_fields:** School name; address; grades offered; directory data
- **coverage:** U.S. K–12 directory subset per API request; licensed bulk via enterprise
- **access_format:** Documented API; enterprise bulk feed
- **verification_status:** verified_documentation_opened_no_credentials_used_2026-10-04
- **evidence_url:** https://www.greatschools.org/gk/about/api-developer-resources/
- **implementation_notes:** Developer page links current technical docs. NearbySchools does NOT provide bulk files; enterprise license offers bulk feed. Attribution/branding and subscription requirements apply. No coaches directory documented.
- **priority:** 3
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NATIONAL
- **shortlist_rank:** null
- **source_id:** SRC-277
- **observed_routes:**
  - (empty list)

### SRC-278

Clell Wade Coaches Directory: exclusion/restriction record

- **state:** NATIONAL
- **name:** Clell Wade Coaches Directory: exclusion/restriction record
- **url:** https://www.coachesdirectory.com/online-directory
- **category:** restricted_do_not_crawl
- **sports_scope:** All sports including track/XC
- **school_or_coach_fields:** Education-based coaches and administrators, per vendor description
- **coverage:** Nationwide licensed directory
- **access_format:** Account/subscription; manual or explicitly negotiated access only
- **verification_status:** verified_opened_explicit_restriction_2026-10-04
- **evidence_url:** https://www.coachesdirectory.com/online-directory
- **implementation_notes:** Official page prohibits virtual assistants and third-party data collectors and caps viewing at 300 schools/day. Exclude from automated collection unless vendor grants specific written licensed permission. Do not use free-trial/coach-registration route as a workaround. No access purchased or account created.
- **priority:** 99
- **checked_date:** 2026-10-04
- **applicable_states:**
  - NATIONAL
- **shortlist_rank:** null
- **source_id:** SRC-278
- **observed_routes:**
  - (empty list)

## Extraction notes and additional resources

HIGH SCHOOL TRACK / CROSS COUNTRY SOURCE ACQUISITION NOTES
Research date: 2026-10-04

Requested geography: TX, CA, NY, OH, NJ, FL, VA, MI, MO, MA, IN, WA, IA, AZ, OK, LA, OR, CT, UT, NV, SD, VT; additional thin-coverage states PA, IL, KS.

#### PURPOSE
This is a source-discovery and implementation input pack, not a collected coach database. Use each source's evidence and access status. A verified page is not a guarantee of complete coverage, current coach assignments, public bulk access, or permission for unlimited automated requests. Blocked and search-only leads are segregated by status in the manifest. Never represent this research as proof that every relevant website in existence was found.

#### WHAT TO INGEST
1. Build a public AND private high-school inventory from official school records. Preserve NCES public ID, private-school PIN/ID, state school ID and athletics-association IDs separately. Verify grade span; a K-12 school may include high school even if its name lacks 'High'. Do not mistake an all-K-12 directory or point-location layer for a high-school-only inventory.
2. Match state athletic membership and sport participation to that school inventory. Preserve independent, private, parochial, charter and cooperative teams. A cooperative team may represent multiple schools; keep a team-to-school bridge. School existence does not prove track/XC participation.
3. Prefer association sport-specific school profiles, coach directory downloads and public maintained coach sheets. Then follow official school athletics and staff links for gaps and current confirmation. Association membership, officer lists, clinic speakers, awards and historical meet documents are candidate evidence with limited coverage, not universal active-coach lists.
4. Keep outdoor track, indoor track and cross country separate, with boys/girls/coed and head/assistant/event-coach roles. Do not silently label an athletic director, principal, meet director, college coach or club coach as a high-school track coach. Preserve both assignments if one coach leads track and XC.
5. Collect only published professional contact information. Store an email only when explicitly tied to the coach's professional role on the source. Do not infer email addresses or turn a school switchboard into the coach's direct phone. This task does not call for collecting individual student records.

#### RECOMMENDED RECORDS
School: internal ID; public NCES ID or private PIN; state ID; association IDs; official name; aliases; address; state; grades served; public/private; official website; source URL; source date; fetched_at.
Coach assignment: school/team ID; coach display name; role; sport; team gender; season/year; published email; published phone and phone type; evidence URL; evidence text/field; source record ID; source publication date; fetched_at; currentness status.
Acquisition result: source ID; jurisdiction; discovered count; eligible-school count; extracted assignments; rejected roles; missing fields; status; error category; next permitted action.

#### IDENTITY AND CURRENTNESS
Use deterministic IDs from source identifiers when possible. Match schools by exact IDs, otherwise corroborate normalized name with city/address/official domain. Avoid merging same-name schools across cities or states. Preserve source-native records alongside normalized records. Resolve conflicting assignments using season/year and official current school evidence, without deleting historical evidence. Deduplicate people separately from coach assignments; one person can hold several roles or serve multiple teams. Retired, TBA, vacant and N/A are not current coach names.

#### ADAPTER RULES
HTML: follow source-published pagination and profile links; do not guess unbounded numeric IDs.
PDF: verify publication year, repeated headers, multi-column reading order, and school-to-coach association; text extraction alone can join the wrong email to a coach.
Google Sheets: use an actually public published/export route; preserve gid/tab choice and column headers. An export URL must be tested, not merely constructed.
CSV/XLSX: preserve IDs as strings, leading zeros and source-specific null values; use documented grade/status codebooks.
ArcGIS: inspect the layer schema and supported query capabilities. Respect maximum record counts; use deterministic pagination or documented object-ID batching; verify extracted count against count query where supported.
Other APIs: use only documented public interfaces or publicly observable endpoints identified in the source. An endpoint embedded in HTML but returning an error is an unverified extraction lead, not a working API.
Access: do not bypass login, membership, CAPTCHA or access restrictions. Keep such sources as explicit blocked leads. Honor site policies, reasonable rate limits and backoff. No outreach, account setup, payment or repository modification is authorized by this pack.

#### COVERAGE CHECKS THAT PREVENT FALSE ZEROS
Keep distinct: source not attempted, fetch failed, access blocked, parser failed, no sports offered, no coach published, coach found without contact, stale assignment, verified current assignment.
Report per state: eligible high schools; association member schools; schools sponsoring track; schools sponsoring XC; schools with current coach assignments; schools with public coach email; unresolved/blocked schools. Do not use total athlete or performance rows as the denominator for school/coach completeness.
An HTTP 200 page with zero extracted coach rows is not necessarily success. Sample positive schools per adapter and assert expected field structure. Preserve evidence for negative results.
Compare public/private coverage separately. Treat a source disappearance or sudden zero count as an anomaly instead of deleting all existing assignments.

#### INPUT-SUMMARY WARNINGS
The pasted counts were not independently verified against the user's CSV. Kansas's 526 coaches does not meet the pasted '<100' heading. The Illinois figure of 2,145,989 athletes needs a check of whether it counts unique athletes, performances, source rows or repeated records. These issues do not change the requested source-search scope.



### TX / CA / FL HIGH-SCHOOL TRACK & FIELD + CROSS-COUNTRY SOURCE GUIDE
Verification date: 2026-10-04

Scope and verification
This is a source-acquisition guide, not a harvested list of coaches. It covers public school/program identity and adults' publicly published professional coaching affiliations/contact fields. Student rosters, individual results, recruiting profiles, personal home contact fields and private/member-only records are outside the extraction scope.

The companion JSON contains 48 retained records: TX 16, CA 17, FL 15. Nine records have top_three_rank=1/2/3, reflecting the user's revised request for three sources per state. Other records preserve already researched alternate associations, regional sources, candidate routes and blockers. Related wrappers, downloads and profiles are grouped as observed_routes instead of being counted as independent shortlist sources.

opened_verified means the named page or documented public UI was opened and its useful contents/route inspected. It does NOT imply all members were downloaded, every field was populated, or automated scraping is licensed. Dynamic shells, uninspected binary files and sample-only checks are stated in each implementation note. blocked means the research fetch did not complete; it does not prove the site is down or require login. search_only means evidence came from search index without successful direct-page inspection. login_required marks access expressly advertised as account-gated.

THREE-SOURCE SHORTLIST

TEXAS
1. UIL alignments: https://www.uiltexas.org/alignments
   Best authoritative sport-program seed. Follow the current XC and spring-athletics pages and their PDFs; use school codes and alphabetical school list to resolve abbreviated names. Not a comprehensive coach list.
2. TEA AskTED: https://tealprod.tea.state.tx.us/tea.askted.web/Forms/Home.aspx
   Best statewide public-school/district identity and website/contact seed, including charter schools. Daily CSV downloads are officially documented. Use school websites reached from this inventory for actual current coaching staff. Personnel downloads should not be assumed to include every coach.
3. TAPPS: https://www.tapps.biz/school-directory-2/
   Strong private/parochial complement. The public iframe advertises school name, address, mascot, classification, website, email and telephone fields. Dynamic rows need browser verification. TAPPS alignment is a related route, not a fourth independent system.

Texas shortlist tradeoff: No publicly open, statewide, comprehensive track/XC coach-email database was verified. These three source systems maximize school/program coverage and authoritative school identity. For direct coach-school enrichment, CCCAT and TTFCA are the strongest verified association extras, but their public awards/board pages cover a small subset. Clell Wade advertises a real Texas coaches directory but requires eligible account access or licensing. Do not manufacture coach emails from school-domain patterns.

CALIFORNIA
1. CIF / Home Campus: https://www.cifsshome.org/widget/school/directory
   Strongest direct coach source discovered. In the cloud browser, selected Arcadia then Coaches and Sports; the public page exposed separate boys/girls XC head-coach name/email and boys/girls track head-coach name/email.
   Exact inspected record: https://www.cifsshome.org/widget/school/directory?school_id=19&section_id=1
2. CDE school directory: https://sd.cde.ca.gov/schooldirectory/
   Best public/private school entity, active-status, grade-span and CDS-code seed. Official private-school data documentation confirms downloadable Excel/text results with optional school contact information: https://www.cde.ca.gov/ds/si/ps/
3. MileSplit California: https://ca.milesplit.com/teams
   Broad XC/track school-program and alias crosswalk, with city and numeric team IDs. Mixed levels/inactive entries require CDE/CIF filtering. Coach fields were not verified, so use it as the independent program seed rather than a coach-email directory.

All ten CIF sections were verified as available labels in the shared public Home Campus section selector: Southern, San Diego, Los Angeles City, Central, Central Coast, North Coast, Northern, Sac-Joaquin, Oakland and San Francisco. Only Southern's per-school coach contents were directly sampled. Do not claim every section has identical coach completeness. Standalone Northern and San Diego public widget pages also opened. San Francisco's high-school member page opened. North Coast, Central Coast, Central and Oakland direct websites were inaccessible in research fetch; Los Angeles City opened. Sac-Joaquin's search-indexed member directory is contaminated by statewide elementary schools and colleges, so its raw search-index row set is not a safe SJS membership census.

The SCVAL 2024 preformatted coaching list is a real multi-school coach/email source but is historical. Its 2026 track index has current local coach details; no 2026 league-wide directory URL was verified. PAUSATF's large coaches list is explicitly from 2005 and should only be used as an archival lead, not fresh contact data. California Coaches Association is genuine but no comprehensive public member export was established.

FLORIDA
1. FHSAA / shared public Home Campus coach directory:
   https://www.cifsshome.org/widget/school/directory?section=10&school=
   Selecting FHSAA in the public section widget loaded Florida schools. Bolles' Coaches and Sports tab exposed different boys/girls XC coaches with emails, boys track coach/email, and an explicit girls-track vacancy marker.
   Exact inspected record: https://www.cifsshome.org/widget/school/directory?school_id=1872&section_id=10
   Native official discovery wrappers: https://fhsaa.com/sports/2020/1/28/member_directory.aspx and https://www.fhsaahome.org/widget/school-directory-locations
2. MileSplit Florida / flrunners: https://fl.milesplit.com/teams
   Broad sport-program seed with team IDs, names, locations and profile links. Verify high-school and active status; directory includes middle schools, clubs and closed schools. Coach contact availability not demonstrated.
3. Florida DOE private-school contact export:
   https://web09.fldoe.org/PrivateSchoolDirectory/DownloadSchools
   Official public Excel export for school name, address, director name, phone and school email. Covers private-school gaps and supplies official-school discovery leads. Director is not synonymous with coach; sports participation needs separate evidence.

Florida public-school MSID is an important additional inventory, but both direct opens timed out. Search verified its 2026–27 public search and the statewide-download chooser. A production implementation should verify and then use it alongside FHSAA. FACA's current sports chairs are legitimate sparse coach-school leads; FACA does not offer public comprehensive coach coverage, and expressly says it does not supply clinic attendee lists. SSAA, FICAA, FCC, FACCS and FCIS provide alternate/private coverage leads with varied strength. FICAA/FCC/FACCS/FCIS membership alone does not prove XC or track sponsorship.

PUBLIC HOME CAMPUS IMPLEMENTATION NOTES
- This public directory is an unusually valuable source because it exposes actual sport-specific staff and emails without an observed login requirement.
- The initial school list uses buttons with IDs such as school-button-19 (Arcadia) and school-button-1872 (Bolles). These IDs were observed in visible UI, not guessed. Discover IDs from public rendered school lists.
- Clicking a school changes the URL to school_id and section_id parameters. Selecting FHSAA changed the list URL to section=10&school=. These are UI route parameters, not evidence of a documented API.
- Relevant visible tabs are School Information, Athletic Faculty, and Coaches and Sports. Scrape the latter for sport headings followed by role/name/email. Do not attribute a coaching role to general office/AD contacts.
- One school can have the same coach for boys and girls or multiple coach rows for a sport. Model person and coaching assignment separately. Preserve vacant/unfilled positions as null, not an inherited neighboring name.
- Filter exact sport headings: Cross Country, Boys; Cross Country, Girls; Track & Field, Boys; Track & Field, Girls, and documented local equivalents. Preserve role, gender/category, level and source date.
- The selected Southern list showed 572 items and the FHSAA list 880 items in browser UI, with a 500-item accessibility display segment. These include placeholders and non-HS schools and are not verified membership totals. Ensure the real scraper enumerates all rendered/paginated entries rather than stopping at a tool snapshot limit.
- The section selector also listed New Jersey and North Carolina, useful for the broader project, but their school/coach contents were not inspected in this research.
- No hidden application state, unpublished network endpoint, authenticated export or bulk coach API is supplied. The public DOM is a verified extraction surface; investigate permitted normal-page requests only during implementation and do not bypass access restrictions.

RUST INGESTION PLAN
1. Keep a source registry keyed by source system, state and effective season, with final URL, fetch timestamp, evidence URL, MIME type, ETag/Last-Modified if supplied, SHA-256, parser version and verification/access status.
2. Build a school table first. Keep state identifiers (CA CDS; TX district/campus identifiers; FL district/school identifiers) as strings, preserving leading zeros. Preserve separate association and third-party IDs. Normalize names conservatively; resolve using city, address, school website and district, never school name alone.
3. Join sport participation before treating a school as a target program. UIL has separate XC/spring files; Florida has separate boys/girls XC and track XLSX/PDF links for 2026–28. All-school membership is not proof that every sport is offered.
4. HTML indexes: reqwest + scraper can parse ordinary server-rendered tables and hrefs. Use csv for documented CSV/text exports and calamine for real Excel workbooks, checking file signatures because an .xlsx route may initially return a document-viewer HTML wrapper. Dynamic Home Campus/TAPPS pages may require a permitted browser-rendering adapter; don't mistake a 200 response containing an empty shell for successful extraction.
5. PDFs: extract text with layout retained, remove repeat headers, preserve conference/district context across pages and validate against visible sample rows. Keep PDF URL, page and season as provenance. Do not scrape individual athlete results as an alternative to school/coach data.
6. Parse official school athletics pages last, following verified school-site links. Use coaching title plus nearby sport section and campus identity. Example fixtures: Community ISD (Finalsite cards), Cypress Creek (WordPress seasonal lists), Winter Springs (HTML tables). A school staff email is evidence only when paired with a relevant adult coaching assignment.
7. Build explicit quality flags: current_official, current_association, historical, undated, dynamic_uninspected, unknown_school_level, vacancy, ambiguous_school_match and conflicts. An award or clinic appearance can corroborate identity but should not overwrite a current school staff listing.
8. Apply per-domain low concurrency, caching and retry/backoff. Check current robots/terms and licensing before production extraction. Never use account-gated/member data or paid exports without authorized access and use rights. Do not guess professional emails; do not collect home addresses/personal phone fields.

COVERAGE GAPS / STOPPING POINT
- No exhaustive current coach census or documented bulk coach API was verified for any of these states. Direct publicly visible coach fields were verified for one CA and one FL school in Home Campus and several official-school fixtures.
- TX private/independent alternatives investigated: TAPPS, SPC, TCAF, TCAL, TCSAAL, TEPSAC; TCAL redirected to a new site with generic/template remnants and no verified current XC/track roster, so it was excluded from the useful-source manifest rather than padded in.
- CA: all ten CIF sections and statewide public/private CDE seeds accounted for; no separate comprehensive statewide high-school track/XC coaching association roster found. CIF includes private/parochial member schools, but independent/non-CIF programs still need school-site and third-party program checks.
- FL: FHSAA plus SSAA, FICAA, FCC, FACCS, FCIS and DOE private/public sources searched. SSAA track page appeared to contain copied venue text, so it is a low-confidence gap-discovery lead, not an authoritative event-data feed.
- State association websites can be incomplete or lag staff turnover. Opened_verified refers to source inspection as of 2026-10-04, not independent confirmation that each named coach remains employed.

FULL RETAINED SOURCE RECORDS

TX

1. UIL sport alignments and school-code inventory [opened_verified; priority 1]
https://www.uiltexas.org/alignments
Category: state_athletic_association. Sports: High-school XC and outdoor track, boys/girls
Fields: School name; conference; region/district; track school codes; organizing-chair contacts where posted
Coverage: UIL members, predominantly public schools and participating charters; not all Texas private schools
Format: HTML indexes with linked PDFs
Implementation: Use sport-specific XC and spring alignments, not football districts. Current index has 2026–27 XC and spring files. Spring includes golf/tennis as well as track, so confirm actual sport sponsorship. Alphabetical all-school list and track school codes are useful alias crosswalks. Coach names/emails are not a comprehensive part of these files.
Evidence: https://www.uiltexas.org/alignments
Related verified/discovered routes (see verification limits above):
- https://www.uiltexas.org/alignments/category/align-cross-country
- https://www.uiltexas.org/alignments/category/align-spring-athletics
- https://www.uiltexas.org/files/alignments/Alpha_26-28.pdf
- https://www.uiltexas.org/track-field/school-codes
- https://www.uiltexas.org/files/alignments/TF_School_Codes_2026.pdf

2. TEA AskTED school, district and personnel downloads [opened_verified; priority 1]
https://tealprod.tea.state.tx.us/tea.askted.web/Forms/Home.aspx
Category: education_department_directory. Sports: All schools; join to XC/track participation
Fields: School/district identity; addresses; contact information; district/campus personnel reports
Coverage: Texas public schools, districts and ESCs; includes public charters
Format: ASP.NET HTML forms; daily comma-delimited downloads; report exports
Implementation: Highest-value public-school entity seed. Homepage says daily updates and offers school/district files with site addresses. Official help documents CSV exports. Follow current download links, preserving cookies/form state if needed; no public REST API verified. Personnel are administrative, not a guaranteed track/XC coach roster. Binary downloads failed in research fetch, but landing and official export documentation are public.
Evidence: https://tealprod.tea.state.tx.us/tea.askted.web/Forms/Home.aspx
Related verified/discovered routes (see verification limits above):
- https://tealprod.tea.state.tx.us/Tea.AskTed.Web/help/Downloading_a_Data_File.htm

3. TAPPS school directory and alignment embeds [opened_verified; priority 1]
https://www.tapps.biz/school-directory-2/
Category: private_parochial_athletic_association. Sports: TAPPS XC and track; all-school seed
Fields: Published embed field list: schoolName,address,mascot,classification,website,email,telephone
Coverage: Texas private/parochial TAPPS members
Format: WordPress HTML wrapper; TMS JavaScript iframe
Implementation: Wrapper and embedded URLs opened. Embedded rows were not rendered by text fetch and shell fetch returned 403, so verify UI row coverage before implementation. This is a public UI embed, not a documented API. Follow pagination rather than assuming limit=12 returns all schools. Coach-specific fields were not verified. Use school websites to reach current coaches.
Evidence: https://www.tapps.biz/school-directory-2/
Related verified/discovered routes (see verification limits above):
- https://tms.tapps.biz/embed-code/SchoolInformation/TVE9PQ==/schoolName,address,mascot,classification,website,email,telephone/1?columns=3&limit=12
- https://www.tapps.biz/home/tapps-26-28-alignment/
- https://tms.tapps.biz/embed-code/alignments/TVE9PQ==?academicYear=3

4. AskTED Texas Open Data Portal dataset [opened_verified; priority 2]
https://data.texas.gov/dataset/AskTED-Data-May-12-2026/hzek-udky
Category: open_data_dataset. Sports: All public schools
Fields: School/contact/location records; schema must be read before coding
Coverage: Snapshot of AskTED, not necessarily as fresh as daily TEA files
Format: Socrata dataset landing and developer portal
Implementation: Dataset landing resolved to May 12, 2026 snapshot. Developer portal opened but schema did not render. Do not represent the conventional /resource/hzek-udky.json URL as tested: sample GET was inaccessible. Use dataset-provided API/export documentation when available and validate current schema/row count.
Evidence: https://data.texas.gov/dataset/AskTED-Data-May-12-2026/hzek-udky
Related verified/discovered routes (see verification limits above):
- https://dev.socrata.com/foundry/data.texas.gov/hzek-udky

5. TEPSAC accredited private-school seed [opened_verified; priority 2]
https://www.tepsac.org/
Category: private_school_accreditation_directory. Sports: All private school sports after school-site enrichment
Fields: Accredited nonpublic school identity and accreditation; actual current export fields unverified
Coverage: Accredited Texas nonpublic elementary/secondary schools, not every private school
Format: JavaScript landing; older HTML information page
Implementation: Linked from current AskTED as accredited non-public school resource. Root rendered no text; information page opened. Current directory/export needs UI verification, so keep as secondary seed and filter grades 9–12. Do not substitute the decades-old TEA acnps.pdf.
Evidence: https://tealprod.tea.state.tx.us/tea.askted.web/Forms/Home.aspx
Related verified/discovered routes (see verification limits above):
- https://www.tepsac.org/home/home.html

6. Texas Christian Athletic Fellowship school directory [blocked; priority 2]
https://www.tcafellowship.com/our-schools
Category: independent_christian_athletic_association. Sports: XC and track confirmed by sport pages
Fields: School; address; phone; administrator; athletic director; public AD email
Coverage: TCAF independent Christian schools
Format: HTML directory
Implementation: Search indexed full school/AD contact entries, but direct opens failed twice. Mark rows search-only until a successful fetch. Track page opened; XC page indexed. Directory is a valuable missing-private-school seed, not sport-specific coach roster.
Evidence: https://www.tcafellowship.com/our-schools
Related verified/discovered routes (see verification limits above):
- https://www.tcafellowship.com/track-program
- https://www.tcafellowship.com/cross-country-program

7. Southwest Preparatory Conference [opened_verified; priority 2]
https://spcsports.org/sports/2019/7/23/governance.aspx
Category: independent_school_athletic_conference. Sports: XC and track; multisport
Fields: Conference schools; school links; selected AD/leadership names; annual manual
Coverage: Independent schools in Texas plus out-of-state members; filter state
Format: SIDEARM HTML; linked handbook PDF
Implementation: Use Conference Members navigation and current annual manual to enumerate schools, then their athletics staff pages. Governance lists 2026–27 handbook and operations committee. Selected AD contacts are not the full coach directory. Exclude Oklahoma members when building Texas-only inventory.
Evidence: https://spcsports.org/sports/2019/7/23/governance.aspx
Related verified/discovered routes (see verification limits above):
- https://spcsports.org/

8. Texas Charter School Academic & Athletic League [opened_verified; priority 2]
https://texascharter.org/
Category: charter_independent_athletic_league. Sports: XC verified; track not visible in current sports navigation
Fields: School/team names; varsity level; region; schedules and standings
Coverage: Charter/independent participating schools across Central, East, North, South and West Texas
Format: HTML sport/region pages and team links
Implementation: Important UIL coverage-gap seed. Follow current Cross Country navigation and each region. Keep varsity/JV HS only; page also includes elementary and middle school teams. Do not infer track coverage from XC. School entities can overlap UIL and require deduplication.
Evidence: https://texascharter.org/
Related verified/discovered routes (see verification limits above):
- https://texascharter.org/regions/central/

9. Cross Country Coaches Association of Texas [opened_verified; priority 2]
https://www.cccat.org/
Category: sport_coaches_association. Sports: High-school cross country
Fields: Coach-of-year name/school; board name/school/contact; clinic presenters
Coverage: Selected Texas XC coaches, not all member coaches
Format: Weebly HTML; annual PDFs
Implementation: Public coach-of-year and board pages opened. Use current awards, board and clinic programs as sparse name-to-school enrichment. Membership page does not establish a public all-member directory. Exclude student awards/scholarships from extraction.
Evidence: https://www.cccat.org/
Related verified/discovered routes (see verification limits above):
- https://www.cccat.org/coach-of-the-year.html
- https://www.cccat.org/cccat-board.html
- https://www.cccat.org/uploads/1/2/3/5/123534441/cccat_2026_summer_clinic_schedule.pdf

10. Texas Track & Field Coaches Association [opened_verified; priority 2]
https://www.ttfca.org/coaches-of-the-year
Category: sport_coaches_association. Sports: Track and field
Fields: Award-winning coaches; school/team affiliation; year
Coverage: Selected high-school coaches plus other levels; strict HS filtering needed
Format: Wix HTML; PDFs
Implementation: Awards page and association home opened. Separate high-school from college coaches and retired hall-of-famers. Useful corroboration, not exhaustive membership/contact list. Preserve award year and verify current employment on school site.
Evidence: https://www.ttfca.org/coaches-of-the-year
Related verified/discovered routes (see verification limits above):
- https://www.ttfca.org/

11. Texas Girls Coaches Association [blocked; priority 3]
https://www.austintgca.com/
Category: multisport_coaches_association. Sports: Girls XC and track
Fields: Sport committees; all-star coaches; awards and school affiliation where posted
Coverage: TGCA members/selected honorees only
Format: HTML; PDFs; member-site login
Implementation: Homepage search result verifies XC/track navigation and membership portal, but direct open failed. Sparse public all-star coach/committee materials may help; do not assume membership profiles or emails are publicly exportable.
Evidence: https://www.austintgca.com/

12. Texas High School Coaches Association [opened_verified; priority 3]
https://www.thsca.com/
Category: multisport_coaches_association. Sports: All HS sports including XC/track
Fields: Public awards/committees and affiliation leads; no comprehensive directory verified
Coverage: Statewide coaches association
Format: HTML; PDF; member services
Implementation: Homepage opened; no public all-coach directory found in page. Useful association lead and recognition source only. Do not confuse its suppliers Buyers Guide with coaches. Clell Wade is separately listed as a gated directory option.
Evidence: https://www.thsca.com/

13. MileSplit Texas team directory [opened_verified; priority 2]
https://tx.milesplit.com/teams
Category: sport_team_database. Sports: XC and track
Fields: Team ID; team/school name; city; team profile URL
Coverage: HS, middle school, college, club and inactive/unattached entries mixed
Format: Public HTML team index and profiles
Implementation: Large directly parseable index; preserve numeric team IDs from hrefs. Join school level/status against TEA/association seeds before keeping records. Sample Abilene profile opened and had no coach field in fetched HTML, so coach/email coverage is not guaranteed. Never collect student rosters or results.
Evidence: https://tx.milesplit.com/teams
Related verified/discovered routes (see verification limits above):
- https://tx.milesplit.com/teams/61776-abilene

14. Athletic.net Texas XC division [opened_verified; priority 3]
https://www.athletic.net/cross-country/division/75199
Category: sport_team_database. Sports: XC; separate track UI
Fields: Team/school identities and team-page links; actual coach fields unverified
Coverage: Texas teams represented in Athletic.net
Format: JavaScript UI
Implementation: Legacy Texas XC URL redirected here. Text fetch returned application shell only; current season/team enumeration needs rendered UI. No public API documented or tested. Avoid athlete/roster extraction.
Evidence: https://www.athletic.net/CrossCountry/Texas/

15. Clell Wade Texas Coaches Directory [login_required; priority 3]
https://www.coachesdirectory.com/coaches/directory-access/
Category: licensed_coach_directory. Sports: Interscholastic multisport; confirm XC/track fields in license
Fields: Coach/school contact directory advertised; individual fields not inspected
Coverage: 2026–27 Texas book; online state HS and junior-high directories
Format: Account-gated web access; printed book
Implementation: Public access page opened and explicitly requires sign-up for free access limited to interscholastic administrators/coaches. No signup attempted. Commercial users need appropriate licensed access and permitted export rights; not a public scrape target.
Evidence: https://www.coachesdirectory.com/coaches/directory-access/

16. Community ISD coaches directory example [opened_verified; priority 2]
https://www.communityisd.org/athletics/coaches-directory
Category: official_school_staff_fallback. Sports: XC and track plus other sports
Fields: Name; exact coaching title; public school email
Coverage: One Texas district, not statewide
Format: Finalsite HTML staff cards
Implementation: Verified explicit Head Coach - Track & Field and Head Coach - Cross Country cards with school emails. Useful parser fixture for official school-site fallback after AskTED/UIL seed discovery. Match title, school/campus and season; do not infer gender if absent.
Evidence: https://www.communityisd.org/athletics/coaches-directory

CA

1. CIF / Home Campus public school and coaches directory [opened_verified; priority 1]
https://www.cifsshome.org/widget/school/directory
Category: state_section_school_coach_directory. Sports: Boys/girls XC and track plus other HS sports
Fields: School identity; school information; athletic faculty; sport; head-coach name; public email; vacancy markers
Coverage: Section selector lists all 10 CIF sections; individual fields verified for Southern Section Arcadia only
Format: Public JavaScript school selector and Coaches and Sports tab
Implementation: Highest-priority direct coach source. Cloud browser clicked Arcadia then Coaches and Sports and verified boys/girls XC coach and email, and boys/girls track coach and email. Discover school IDs from rendered school buttons; do not enumerate guessed IDs. Display includes BYE, association placeholders and some middle schools; filter with CDE. Other section presence verified in selector, but their rows/coach completeness not individually audited. No bulk API or hidden endpoint claimed.
Evidence: https://www.cifsshome.org/widget/school/directory?school_id=19&section_id=1
Related verified/discovered routes (see verification limits above):
- https://www.cifsshome.org/widget/school/directory?school_id=19&section_id=1
- https://www.cifnshome.org/widget/school/directory
- https://www.cifsdshome.org/widget/school/directory

2. California Department of Education school directory and exports [opened_verified; priority 1]
https://sd.cde.ca.gov/schooldirectory/
Category: education_department_directory. Sports: All schools; join sports participation separately
Fields: CDS code; school/district; county; type; sector; charter/status; grades; address; administrator and contact fields
Coverage: California public/private/nonpublic schools, districts and county offices
Format: Searchable HTML; Excel/text exports documented
Implementation: Use active status and grades including 9–12; retain 14-digit CDS as string. Directory includes K–12 and continuation/alternative high schools, so do not filter name alone. Private-school data page explicitly documents Excel/text export with contacts. Directory is not a coach list. Public bulk-file landing hit CDE WAF during research; prefer verified searchable export route rather than inventing replacement URLs.
Evidence: https://www.cde.ca.gov/ds/si/ps/
Related verified/discovered routes (see verification limits above):
- https://www.cde.ca.gov/schooldirectory/
- https://www.cde.ca.gov/ds/si/ps/
- https://www.cde.ca.gov/ds/si/ds/pubschls.asp

3. MileSplit California team directory [opened_verified; priority 1]
https://ca.milesplit.com/teams
Category: sport_team_database. Sports: XC and track
Fields: Team ID; school/team; city; section abbreviation in many team names; profile URL
Coverage: Statewide HS plus mixed school levels/clubs/inactive entries
Format: Public HTML team index
Implementation: Strong independent sport-program/alias crosswalk. Example suffixes SS, CC, NC, SJ, NS, CS encode sections but should be validated. Use CDE and CIF to exclude non-HS/closed schools. Coach coverage not demonstrated, no API verified. Do not traverse athlete rosters.
Evidence: https://ca.milesplit.com/teams

4. CIF official ten-section index [opened_verified; priority 2]
https://cifss.org/cif-state-sections/
Category: association_coverage_map. Sports: XC and track statewide
Fields: Official section names and URLs; section office contacts
Coverage: All 10 CIF sections
Format: HTML table
Implementation: Use as authoritative discovery map for Southern, San Diego, Los Angeles City, Central, Central Coast, North Coast, Northern, Sac-Joaquin, Oakland and San Francisco. School counts on overview may lag, so never use as current row-count assertion. School/coaching records belong to sections and public Home Campus selector.
Evidence: https://cifss.org/cif-state-sections/

5. CIF San Francisco high-school members [opened_verified; priority 2]
https://www.cifsf.org/schools/high-schools/
Category: regional_school_directory. Sports: XC and track offered by section
Fields: High-school members; school and athletics links; AD fields in school information UI
Coverage: San Francisco Section; do not confuse separate middle-school list
Format: Home Campus powered HTML/JavaScript
Implementation: Direct high-school page opened. Use member-school cards and sport pages, and exclude adjacent middle-school navigation. Homepage template exposes principal/AD/address/website fields; populated per-school details not audited.
Evidence: https://www.cifsf.org/schools/high-schools/

6. CIF Southern Section directory wrapper [search_only; priority 2]
https://cifss.org/directory/
Category: regional_school_league_directory. Sports: XC and track
Fields: School directory and league directory iframe routes
Coverage: Southern Section
Format: HTML with iframe widgets
Implementation: Search result explicitly exposes separate school and league directory sections. School iframe independently opened and coach data verified; league iframe not separately tested. Retain as discovery wrapper rather than a separate source system from Home Campus.
Evidence: https://cifss.org/directory/

7. CIF Northern Section public Home Campus directory [opened_verified; priority 2]
https://www.cifnshome.org/widget/school/directory
Category: regional_school_directory. Sports: XC and track
Fields: School selector names and school IDs; coach-tab population untested for this section
Coverage: Northern Section public/private school seeds plus placeholders
Format: JavaScript widget; search-indexed school buttons
Implementation: Opened widget shell and search-indexed school list. Same public UI family as verified Southern directory, but do not assume equal coach completeness. Filter BYE/Non CIFNS School and verify school level.
Evidence: https://www.cifnshome.org/widget/school/directory

8. CIF San Diego Section public Home Campus directory [opened_verified; priority 2]
https://www.cifsdshome.org/widget/school/directory
Category: regional_school_directory. Sports: XC and track
Fields: School selector names and IDs; coach-tab population untested for this section
Coverage: San Diego/Imperial-area section members including private schools
Format: JavaScript widget; search-indexed school buttons
Implementation: Opened widget shell and indexed member names. School/coach tab should be validated against one current school before bulk adapter use. Remove Non CIFSDS School placeholders and any non-HS entries.
Evidence: https://www.cifsdshome.org/widget/school/directory

9. CIF North Coast Section official source [blocked; priority 3]
https://www.cifncs.org/
Category: regional_association. Sports: XC and track
Fields: Section membership, sport alignments, school links; detailed fields not verified
Coverage: North Coast Section
Format: Section website; shared Home Campus selector alternative
Implementation: Official identity/URL verified through CIF ten-section index. Direct root fetch failed (403 for NCS/CCS/Central; inaccessible for Oakland). Section name is present in successfully inspected Home Campus public selector. Preserve as coverage checklist; no unverified endpoint or school count asserted.
Evidence: https://cifss.org/cif-state-sections/

10. CIF Central Coast Section official source [blocked; priority 3]
https://www.cifccs.org/
Category: regional_association. Sports: XC and track
Fields: Section membership, sport alignments, school links; detailed fields not verified
Coverage: Central Coast Section
Format: Section website; shared Home Campus selector alternative
Implementation: Official identity/URL verified through CIF ten-section index. Direct root fetch failed (403 for NCS/CCS/Central; inaccessible for Oakland). Section name is present in successfully inspected Home Campus public selector. Preserve as coverage checklist; no unverified endpoint or school count asserted.
Evidence: https://cifss.org/cif-state-sections/

11. CIF Central Section official source [blocked; priority 3]
https://www.cifcs.org/
Category: regional_association. Sports: XC and track
Fields: Section membership, sport alignments, school links; detailed fields not verified
Coverage: Central Section
Format: Section website; shared Home Campus selector alternative
Implementation: Official identity/URL verified through CIF ten-section index. Direct root fetch failed (403 for NCS/CCS/Central; inaccessible for Oakland). Section name is present in successfully inspected Home Campus public selector. Preserve as coverage checklist; no unverified endpoint or school count asserted.
Evidence: https://cifss.org/cif-state-sections/

12. CIF Oakland Section official source [blocked; priority 3]
https://cifoakland.org/
Category: regional_association. Sports: XC and track
Fields: Section membership, sport alignments, school links; detailed fields not verified
Coverage: Oakland Section
Format: Section website; shared Home Campus selector alternative
Implementation: Official identity/URL verified through CIF ten-section index. Direct root fetch failed (403 for NCS/CCS/Central; inaccessible for Oakland). Section name is present in successfully inspected Home Campus public selector. Preserve as coverage checklist; no unverified endpoint or school count asserted.
Evidence: https://cifss.org/cif-state-sections/

13. CIF Los Angeles City Section [opened_verified; priority 2]
https://www.cif-la.org/
Category: regional_association. Sports: XC and track
Fields: School/program discovery; CIF-LA Home link
Coverage: Los Angeles City Section
Format: Edlio HTML; Home Campus link
Implementation: Official homepage opened and links to www.cif-lahome.org. Public shared Home Campus selector includes Los Angeles City Section. Detailed standalone directory path and coach completeness not audited; use observed navigation, not guessed URL paths.
Evidence: https://www.cif-la.org/

14. CIF Sac-Joaquin member schools [blocked; priority 3]
https://www.cifsjs.org/schools/
Category: regional_school_directory. Sports: XC and track
Fields: Search-indexed school names/city; membership/level needs validation
Coverage: Sac-Joaquin intended, but indexed content includes statewide elementary schools and colleges
Format: Dynamic directory
Implementation: Direct open failed. Search index unexpectedly contains statewide elementary schools, universities and duplicate school names. Never treat every indexed row as a current SJS HS member. Prefer official league alignments or the SJS selection in public Home Campus, then crosswalk CDE.
Evidence: https://www.cifsjs.org/schools/

15. Lynbrook / PrepCalTrack SCVAL coach directory [opened_verified; priority 2]
https://lynbrooksports.prepcaltrack.com/ATHLETICS/TRACK/2024/coaches.htm
Category: regional_sport_coach_directory. Sports: SCVAL track; current seasonal pages also cover XC
Fields: School; coach; work phone; published email; additional personal-number columns that should be excluded
Coverage: Santa Clara Valley Athletic League; 2024 directory explicitly dated
Format: Simple preformatted HTML; current index HTML/PDF
Implementation: Real multi-school coaching list, dated January 23, 2024, therefore historical lead-only until current school-site confirmation. Ignore home/cell fields. 2026 track index opened and has current Lynbrook head/distance coach and links to league meeting minutes and school websites. Follow current seasonal navigation instead of assuming year-substituted directory URL exists.
Evidence: https://lynbrooksports.prepcaltrack.com/ATHLETICS/TRACK/2024/coaches.htm
Related verified/discovered routes (see verification limits above):
- https://lynbrooksports.prepcaltrack.com/ATHLETICS/TRACK/2026/2026.htm

16. Pacific Association USATF legacy coaches contacts [opened_verified; priority 3]
https://www.pausatf.org/data/Coaches.html
Category: legacy_sport_coach_directory. Sports: XC and track, mixed high-school/college/club
Fields: Coach; school/club; published email; region
Coverage: Northern/central California and Nevada; explicitly updated November 4, 2005
Format: HTML tables
Implementation: Archived historical research only. Do not load as current coach directory. Revalidate every affiliation and email on current official school site and exclude colleges/clubs/Nevada. Included to prevent mistaking a large search-visible directory for fresh data.
Evidence: https://www.pausatf.org/data/Coaches.html

17. California Coaches Association [blocked; priority 3]
https://www.calcoachesassociation.net/
Category: multisport_coaches_association. Sports: All HS sports including XC/track
Fields: Public awards, section representatives and school affiliations; no all-member export established
Coverage: Statewide association; sparse public names
Format: SportsEngine HTML/PDF; membership registration
Implementation: Search results confirm current association, membership options and section-rep page. Direct root and section-rep opens failed. Association representatives are not a comprehensive track/XC directory. Keep as supplemental award/clinic source only.
Evidence: https://www.calcoachesassociation.net/
Related verified/discovered routes (see verification limits above):
- https://www.calcoachesassociation.net/section-reps

FL

1. FHSAA public Home Campus school and coach directory [opened_verified; priority 1]
https://www.cifsshome.org/widget/school/directory?section=10&school=
Category: state_athletic_coach_directory. Sports: Boys/girls XC and track, plus other sports
Fields: School ID; sport; head-coach name/email; position-not-filled marker; school/AD data tabs
Coverage: FHSAA public/private/charter members; selector includes middle schools and placeholders
Format: Public JavaScript shared Home Campus directory
Implementation: In cloud browser selected FHSAA from public CIF widget, then Bolles, then Coaches and Sports. Verified separate boys/girls XC coach+email, boys track coach+email and girls track Position not filled. Observed FHSAA selector listed 880 entries, including middle schools, so this is not 880 high schools. Use school IDs discovered from UI; no hidden API or bulk endpoint claimed. FHSAA native member-directory wrapper and sport-filtered locations widget are also verified.
Evidence: https://www.cifsshome.org/widget/school/directory?school_id=1872&section_id=10
Related verified/discovered routes (see verification limits above):
- https://www.cifsshome.org/widget/school/directory?school_id=1872&section_id=10
- https://fhsaa.com/sports/2020/1/28/member_directory.aspx
- https://www.fhsaahome.org/widget/school-directory-locations

2. MileSplit Florida / flrunners team directory [opened_verified; priority 1]
https://fl.milesplit.com/teams
Category: sport_team_database. Sports: XC and track
Fields: Team ID; school/team; city; profile URL
Coverage: Florida sport programs, with middle school/college/club/closed entries mixed
Format: Public HTML index
Implementation: Strong crosswalk and gap seed, not a confirmed complete coaching directory. Preserve team IDs; reconcile school status and level. The index explicitly includes closed-school labels and middle schools. No coach API verified and student roster/result extraction is out of scope.
Evidence: https://fl.milesplit.com/teams

3. Florida DOE private-school contact export [opened_verified; priority 1]
https://web09.fldoe.org/PrivateSchoolDirectory/DownloadSchools
Category: education_department_private_directory. Sports: All private schools; join to XC/track sponsorship
Fields: School name; address; director name; phone; school email
Coverage: Florida private-school annual survey directory; self-reported, not accreditation
Format: HTML download form; Excel export
Implementation: Official page states exact exported contact fields and offers Download All Schools or district selection. Main search initially displayed zero pending filters/loading; that is not zero schools. Use export and grade filters/profile checks, then official athletics pages for coaches. Do not infer that director is coach or DOE endorses a listed school. Binary export not downloaded in this research.
Evidence: https://web09.fldoe.org/PrivateSchoolDirectory/DownloadSchools
Related verified/discovered routes (see verification limits above):
- https://web09.fldoe.org/PrivateSchoolDirectory/
- https://www.fldoe.org/schools/school-choice/directories.stml

4. Florida DOE Master School ID public-school inventory [blocked; priority 2]
https://eds.fldoe.org/EDS/MasterSchoolID/index.cfm
Category: education_department_public_directory. Sports: All public schools
Fields: District/school numbers; school identity; city/district; school profile and grade/status details
Coverage: Statewide public schools; current indexed year 2026–27
Format: ColdFusion search; statewide/district downloads
Implementation: Search results verify official 2026–27 UI, instructions to submit blank for all schools, and download chooser. Direct opens timed out. Download URL is observed, but response/schema not tested. Pair with FHSAA coach directory and filter active schools serving grades 9–12.
Evidence: https://eds.fldoe.org/EDS/MasterSchoolID/index.cfm
Related verified/discovered routes (see verification limits above):
- https://eds.fldoe.org/EDS/MasterSchoolID/Downloads/SelectDistrict.cfm?type=1

5. FHSAA current XC classification files [opened_verified; priority 2]
https://fhsaa.com/news/2026/4/8/about-us-fall-sport-classifications-available-for-2026-27-2027-28.aspx
Category: state_sport_participation_inventory. Sports: Boys/girls XC
Fields: School/team; classification; district/region as file provides
Coverage: Final 2026–27/2027–28 FHSAA XC classification cycle
Format: HTML release; separate XLSX and PDF links
Implementation: Use separate boys/girls files as definitive sport-specific participation seeds, not all-sports membership alone. Linked XLSX document wrappers opened; workbook contents not extracted. Follow wrapper download rather than assuming an S3 URL. This is same source organization as coach directory, not an independent shortlist source.
Evidence: https://fhsaa.com/news/2026/4/8/about-us-fall-sport-classifications-available-for-2026-27-2027-28.aspx
Related verified/discovered routes (see verification limits above):
- https://fhsaa.com/documents/2026/4/8//2026_27_2027_28_Finalized_BXC_Classification.xlsx?id=7605
- https://fhsaa.com/documents/2026/4/8//2026_27_2027_28_Finalized_GXC_Classification.xlsx?id=7609

6. FHSAA current track classification files [opened_verified; priority 2]
https://fhsaa.com/news/2026/7/17/baseball-final-spring-sport-classifications-available-for-2026-27-2027-28.aspx
Category: state_sport_participation_inventory. Sports: Boys/girls track and field
Fields: School/team; classification; district/region as file provides
Coverage: Final 2026–27/2027–28 FHSAA track classification cycle
Format: HTML release; XLSX/PDF links
Implementation: Separate boys/girls track files are present and opened as document wrappers. Do not reuse XC class cutoffs for track. Parse actual workbook after download validation; no workbook rows audited in this pass.
Evidence: https://fhsaa.com/news/2026/7/17/baseball-final-spring-sport-classifications-available-for-2026-27-2027-28.aspx
Related verified/discovered routes (see verification limits above):
- https://fhsaa.com/documents/2026/7/17//2026_27_2027_28_Finalized_BTF_Classification.xlsx?id=7960
- https://fhsaa.com/documents/2026/7/17//2026_27_2027_28_Finalized_GTF_Classification.xlsx?id=7953

7. Florida Athletic Coaches Association [opened_verified; priority 2]
https://www.floridacoaches.org/state-sports-chairman.html
Category: multisport_coaches_association. Sports: XC and track chapters
Fields: Sport-chair coach; school; term; selected public clinic/award coach affiliations
Coverage: Selected association leaders/honorees, not statewide roster
Format: Weebly HTML; PDFs
Implementation: Current 2026–27 chair page opened and identifies XC/track school affiliations. Districts page maps 24 districts to counties but does not itself list all coaches. Clinic speakers/awards can enrich. FACA explicitly says it does not provide clinic attendee lists in exhibitor agreement, so do not promise bulk membership data.
Evidence: https://www.floridacoaches.org/state-sports-chairman.html
Related verified/discovered routes (see verification limits above):
- https://www.floridacoaches.org/
- https://www.floridacoaches.org/districts-by-county--schools.html
- https://floridacoaches.powermediallc.org/exhibitor-agreement/

8. FHSAA sports advisory committees [opened_verified; priority 2]
https://fhsaa.com/sports/2020/3/11/Sport_Committees.aspx
Category: sport_advisory_coach_directory. Sports: XC and track committees among sports
Fields: Committee coach names; school; public contact fields as published
Coverage: Small advisory committees only
Format: HTML expandable sport panels
Implementation: Good sparse coach-school corroboration and current stakeholder discovery. Filter XC/track sections. Not equivalent to all member coaches; administrative FHSAA staff are not school coaches.
Evidence: https://fhsaa.com/sports/2020/3/11/Sport_Committees.aspx

9. Sunshine State Athletic Association [opened_verified; priority 3]
https://www.sunshinestateathletics.com/
Category: independent_athletic_association. Sports: High-school XC and track explicitly listed
Fields: School/member discovery and sport-event participation leads; direct coach roster not found
Coverage: 120+ members advertised, not all Florida schools
Format: HTML sport pages
Implementation: Important alternate athletic system for gap search. Homepage and sport pages opened. Current track page contains apparent copied beach-volleyball venue text; do not ingest venue or claims blindly. No complete current school/coach directory was visible in inspected pages. Cross-check entrants with current school websites.
Evidence: https://www.sunshinestateathletics.com/
Related verified/discovered routes (see verification limits above):
- https://www.sunshinestateathletics.com/cross-country.html
- https://www.sunshinestateathletics.com/track.html

10. Florida Independent Christian Athletic Association members [opened_verified; priority 2]
https://ficaa.org/members
Category: independent_christian_athletic_association. Sports: General athletic school seeds; XC/track sponsorship not verified
Fields: School name; city; conference/region
Coverage: Christian schools grouped into Coastal, Mid-Florida, Central Florida, Suncoast conferences
Format: Public HTML
Implementation: Useful private-school coverage gap seed. Do not infer XC/track sponsorship solely from FICAA membership. Verify high-school grade span and sport at each school or meet entry source before keeping as active track/XC program.
Evidence: https://ficaa.org/members

11. Florida Christian Conference school list [blocked; priority 3]
https://www.fccsports.net/forms/FCC%20School%20List.pdf
Category: independent_christian_athletic_conference. Sports: General school seed; XC/track not verified
Fields: School/contact fields pending file inspection
Coverage: FCC member schools
Format: PDF
Implementation: Search-indexed official PDF found, but direct download timed out. Retain as secondary candidate, not as verified extracted contact list. Check document season and school sport sponsorship before ingestion.
Evidence: https://www.fccsports.net/forms/FCC%20School%20List.pdf

12. Florida Association of Christian Colleges and Schools directory [opened_verified; priority 2]
https://faccs.org/schools
Category: private_school_directory. Sports: All schools; sport enrichment needed
Fields: School name; city; grade span; member type; linked school detail
Coverage: 123 listed members at inspection, including PK-only, colleges and an overseas school
Format: Public HTML table and detail links
Implementation: Filter Florida location and grades 9–12 before crawling school athletics pages. Strong explicit grade-span filter for independent/private coverage. Membership/accreditation does not prove sport sponsorship. Do not include colleges, PK-only or Bahamas record.
Evidence: https://faccs.org/schools

13. Florida Council of Independent Schools membership directory [opened_verified; priority 2]
https://www.fcis.org/about/directory
Category: private_school_directory. Sports: All private school sports after enrichment
Fields: School identity; school contact/profile fields; phone and website links
Coverage: FCIS independent member schools; filter secondary grades
Format: Finalsite HTML directory
Implementation: Directory opened. Use to recover private-school aliases and official sites, then athletics staff/sport pages for coaches. Directory membership does not establish XC/track program. Avoid treating school office contacts as coaches.
Evidence: https://www.fcis.org/about/directory

14. Cypress Creek High School official coaches directory example [opened_verified; priority 2]
https://cchs.pasco.k12.fl.us/hscoachdirectory/
Category: official_school_staff_fallback. Sports: Boys/girls XC and track
Fields: Sport; gender; coach name; public email
Coverage: One Pasco County high school
Format: WordPress HTML seasonal coach lists
Implementation: Useful official-school parser fixture. Cross-country and track head-coach records are explicit. Pasco school sites have multiple templates and domains; seed each school from official district/DOE directory rather than guessing URLs.
Evidence: https://cchs.pasco.k12.fl.us/hscoachdirectory/

15. Winter Springs High School coaches directory example [opened_verified; priority 2]
https://www.winterspringshs.scps.k12.fl.us/coaches
Category: official_school_staff_fallback. Sports: XC and track
Fields: Sport; head coach; public email; athletics administration
Coverage: One Seminole County high school
Format: HTML tables
Implementation: Current page opened with seasonal Sport/Head Coach/Email tables. Treat TBA/vacancy as unknown, not a named coach inherited from another sport. School-site freshness wins over older directories.
Evidence: https://www.winterspringshs.scps.k12.fl.us/coaches



### NORTHEAST HIGH-SCHOOL TRACK/XC SOURCE GUIDE
Verified 2026-10-04 | NY, NJ, MA, CT, VT
Read-only public research; no accounts, outreach or student-data collection.

RESULT
Three ranked source systems per state, fifteen total, plus verified or access-qualified extras. The full JSON has 48 records; shortlist JSON has 15. Page access does not establish complete coverage or current employment.

TOP THREE PER STATE

NY
1. NYSPHSAA section network: https://nysphsaa.org/sports/2021/6/7/section-map.aspx
Official eleven-section gateway. Section V https://schools.sectionv.org/ is an excellent school/league/athletics-domain seed. Section IV has an opened member-school list. Section IX's 2024 spring-track handbook has a small coaches committee table, not all regional coaches. Visit actual school athletics staff/team pages after seeding. NYC PSAL, independent and Catholic programs require complementary systems.
2. PSAL: https://www.psal.org/sports/sport.aspx?flag=All&spCode=033
Sport-specific school/team listing with real school/sport IDs. Inspected client script renders head, co- and assistant coach names. JSON responses were not directly fetched; see extraction notes. Preserve campus/co-op identities.
3. NYSED: https://www.p12.nysed.gov/irs/schoolDirectory/
Direct GET verified 200. Public/nonpublic school inventory and SEDREF report/query tools close association coverage gaps. No coach fields assumed. NYSAIS/CHSAA sources are extras.

NJ
1. NJSIAA: https://www.njsiaa.org/schools/member-information
Public paginated school/address/AD table; ?page=0 and ?page=1 observed. Individual school details redirect to login. Use public list for school seeds, official school websites for current coaches.
2. NJXC/TFCA network: https://njxctfca.org/links/
Verified county/regional association index. Opened South Jersey, Shore, Morris, Greater Middlesex resources establish concrete school/team participation routes and occasional named-coach leads. No full statewide member roster verified.
3. MileSplit: https://nj.milesplit.com/teams
Public track/XC team index. Filter high school, active school and season; includes clubs, closed and historical records. A team page's coach email is not guaranteed.
Extra: official NJDOE directory is https://homeroom6.doe.nj.gov/directory/ and is 403-blocked here. Old /education/directory/ is 404. Home Campus NJ is an untested supplemental lead only.

MA
1. MSTCA: https://mstca.org/coaches-corner/current-members
Best concrete coach-related source in this region. Public HTML has Next.js members JSON with ID/name/organization/email/phone/status. First page has 24 rows and totalCount 1417; 60 rendered page buttons. Eleven of first 24 email fields were unknown@mstca.org placeholders. Membership does not establish current employment, role or sport. Confirm each school appointment.
2. MIAA: https://www.miaa.net/schools
School/league/district/AD seeds and public profile pages, e.g. /group/2. Certified-coaches PDF has 383 pages of first/last/school but no sport/contact fields or proof of active appointment. Use it only for corroboration.
3. DESE: https://profiles.doe.mass.edu/search/search.aspx
Authoritative public/private completeness and grade filters. Observed search-link routes and exports. MassGIS offers complementary bulk school geodata from DESE.

CT
1. CAS-CIAC public entry point: https://www.casciac.org/mobiledir/
Opened school list, but LEGACY: old/closed school names occur. Reconcile schools with current EdSight before use; this does not provide current coaches by itself. The stronger https://ciac.fpsports.org/Directory.aspx?SchoolLevelID=1 coach directory is search-indexed with school/sport/head-coach/phone rows, but direct open redirects to WebBotZone.aspx Unauthorized Access/login. It remains a blocked dependent route, not a tested feed. Do not bypass; use current official school athletics/staff pages.
2. EdSight: https://public-edsight.ct.gov/general-information
Official public/nonpublic inventory and export workflow. Separate administrative contact report is not a coach directory.
3. NEPSAC: https://nepsac.org/about/nepsac-member-schools/
Public prep-school list with direct school websites. Filter Connecticut and high-school grades; follow each athletics/team/coaches directory. NEPSTA exposes a few officers, not every coach.
Extras: FCIAC 2021-22 directory is an opened 49-page coach/email/phone PDF but historical; CAS MobileDir also contains legacy names.

VT
1. VPA: https://vpaonline.org/athletics/
Official participation/tournament gateway. Current guides page shows XC/indoor/outdoor labels, but those labels were not linked in extracted HTML. Do not invent guide PDFs or claim a coach roster.
2. MileSplit: https://vt.milesplit.com/teams
Opened track/XC team directory; filter high-school and current active programs, then official school staff sites.
3. Vermont AOE Annual Snapshot: https://schoolsnapshot.vermont.gov/Organization/Directory
Opened public government directory shell, with school organization rows visible to search indexing. Browser rendering is still needed to verify all fields and extraction route. No JSON API verified.
Extras: AOE public-school hub and independent-school directory are 403-blocked here. Independent landing is indexed with 2026-09-22 publication but exact current PDF href unverified.
No statewide public Vermont track/XC coach roster or state-specific track coaches membership roster was verified. The honest route is school inventory plus current school staff/team pages.

VERIFICATION LABELS
opened_verified = requested source retrieved; notes explain JS-only shells and downstream resources not tested.
blocked = failed fetch or explicit access/location gate; indexed facts are leads, not tested feed data.
login_required = explicit login redirect observed.
search_only = discovered in search without successful retrieval. No final manifest record needs this label; inaccessible entries are classified blocked.

CONCRETE ROUTES FOR IMPLEMENTATION

MSTCA
Page data in self.__next_f.push chunks contains members array and totalCount. Decode JSON strings, then array. First page is 24 members, not the full 1417. Verified fields: _id,firstName,lastName,organization,email,phone,status,lastPaymentDate,image,altText. Whitelist only coach-relevant professional fields; ignore payment/image fields. Reject placeholders and verify current role at school. Render normal pagination or inspect its observed public navigation. No separate supported API verified; do not invent Sanity endpoints or credentials.

PSAL
Observed team route: https://www.psal.org/profiles/team-profile.aspx#033/13507
Observed public code: https://www.psal.org/scripts/js/Team_Profile.js
This code uses getTeamDetails, getCoachNamesIds, GetTeamsBySport and vw_School_Profile. It renders team.coachid,cocoachid,assist1id,assist2id into head/co/assistant roles. The service base URL and actual responses still require verification from live public page execution. This is site-internal code, not a documented supported API. Never call roster/athlete/inactive-student/statistics endpoints for this task.

NYSED
Observed official directory links:
https://portal.nysed.gov/pls/sedrefpublic/SED.sed_inst_qry_vw$.startup
http://eservices.nysed.gov/sedreports/list?id=1
Follow current public reports; avoid hardcoding long obrar.cgi session/encreply URLs. Report exports were not downloaded in this pass.

MA DESE/MassGIS
Public link:
https://profiles.doe.mass.edu/search/search_link.aspx?leftNavId=11238&orgType=6,13&runOrgSearch=Y
Private link:
https://profiles.doe.mass.edu/search/search_link.aspx?leftNavId=11238&orgType=11&runOrgSearch=Y
Official metadata:
https://www.mass.gov/info-details/massgis-data-massachusetts-schools-pre-k-through-high-school
Exact official ZIP href observed:
https://s3.us-east-1.amazonaws.com/download.massgis.digital.mass.gov/shapefiles/state/schools.zip
Metadata GET verified 200; ZIP was not downloaded. December 2025 data derive from DESE profiles as of 2025-11-05.
Supplementary FeatureServer opened:
https://services1.arcgis.com/TXaY625xGc0yvAuQ/arcgis/rest/services/Schools/FeatureServer
Advertises JSON, layer 0 and 1000 max records. It cites MassGIS as source, but owner/freshness were not established; prefer official download.

EdSight
Export page: https://public-edsight.ct.gov/overview/find-schools/find-schools-export
Table page: https://public-edsight.ct.gov/overview/find-schools/find-school-district
HTML embeds a SASStoredProcess/do report with _program=/CTDOE/EdSight/Release/Reporting/Public/Reports/StoredProcesses/OrgSearchReport_SiteCore and orgtype,orgdistrict,orgname. Follow exact iframe href at runtime. This is a report route, not tested JSON API.

Home Campus NJ: supplemental UNTESTED data lead
CA-hosted widget opened:
https://www.cifsshome.org/widget/school/directory?school_id=19&section_id=1
Dropdown has New Jersey value 12. Form field is section; adding only section_id=12 without a valid school still produced default section 1, so NJ records are not verified.
Observed read routes from public code: /widget/schools/get with school,section_id,status=active,hide_from_directory=0 and /widget/get-school-details/{observed_id}/details.
Shared response renderer supports sport/role/name/email. Do not infer NJ completeness from CA, fabricate IDs, or access hidden-from-directory records. Not a documented supported API.

RUST HANDOFF CHECKLIST
- Build source-specific read adapters with reqwest, scraper and serde_json; render JS only when needed. Respect robots/terms, caching, conditional GET, backoff and small per-origin concurrency.
- Stop adapters on 401/403/login/CAPTCHA/access restrictions. No proxies or identity rotation to bypass. Continue through unrelated permitted sources.
- Extract PDF locally with source URL/hash/page number. Date the document itself, not filename folder or search crawl. Use CSV/XLSX/shapefile readers preserving leading-zero IDs.
- Separate School, Person and CoachingAppointment. One coach may cover several genders/sports/seasons or co-op schools.
- School keys: state, official name, state/NCES ID where published, locality, domain, sector, grade range, active status, association aliases.
- Appointment keys: school_id,person_id,sport,team_gender,season,level,role,school_year,explicit professional email/work phone.
- Every fact keeps source_url, retrieved_at, source_effective_date, field label, extraction method and confidence/currentness. Preserve conflicting evidence rather than silently overwrite.
- Do not infer emails or turn AD/principal/association officer into coach. Filter unknown@mstca.org,TBA,No Team,Position not filled and lifetime/historical-only members.
- Follow verified official school athletics > team/coaches/staff/contact pages with bounded crawl budget. No student rosters, athlete profiles, minors' personal data or unrelated personnel details.
- Deduplicate school aliases, campus programs and cooperative teams without merging distinct campuses. Exclude middle-only,college,club,closed and inactive entities.
- Track coverage separately: school universe, schools with target sport, verified current coaches, explicit public professional emails. Revalidate current appointments each season.

FULL SOURCE REGISTER

NE-001 | NY | NYSED School and District Directory / SEDREF public reports
URL: https://www.p12.nysed.gov/irs/schoolDirectory/
Category: state_education_directory; Priority: P1; Rank: 3
Sports: All schools; sport not supplied
Fields: Institution identity; public/nonpublic school universe; school contacts and grades via linked reports
Coverage: Statewide public, charter, nonpublic and BOCES
Format: HTML hub; public query; report exports
Status: opened_verified (2026-10-04)
Evidence: https://www.p12.nysed.gov/irs/schoolDirectory/
Notes: Direct HTTPS GET returned 200 after web-reader failure. Observed links: https://portal.nysed.gov/pls/sedrefpublic/SED.sed_inst_qry_vw$.startup and http://eservices.nysed.gov/sedreports/list?id=1 . Follow public reporting workflow; do not persist transient encrypted obrar.cgi URLs. Sport/coach data requires enrichment.

NE-002 | NY | NYSPHSAA official section map and section links
URL: https://nysphsaa.org/sports/2021/6/7/section-map.aspx
Category: state_athletics_index; Priority: P1; Rank: 1
Sports: All interscholastic sports including XC/indoor/outdoor
Fields: Section jurisdiction, official section websites, section office contacts
Coverage: 11 NYSPHSAA sections; separate NYC PSAL/CHSAA/independent coverage needed
Format: HTML
Status: opened_verified (2026-10-04)
Evidence: https://nysphsaa.org/sports/2021/6/7/section-map.aspx
Notes: Use as authoritative source-of-sources. Section staff are not school coaches. Follow all 11 section links and member-school/league/sport pages.

NE-003 | NY | Section V member school directory
URL: https://schools.sectionv.org/
Category: regional_school_directory; Priority: P1; Rank: extra
Sports: All sports; XC/track enrichment via school sites
Fields: School name; league; athletics URL; district URL; schedule URL
Coverage: Genesee Valley / Section V public and private/parochial members; includes some middle-school/co-op records
Format: HTML cards; interactive search/filter
Status: opened_verified (2026-10-04)
Evidence: https://schools.sectionv.org/
Notes: Excellent direct athletics-domain seeds. Page shows 127 member schools but 133 displayed records; do not treat either as high-school count. Filter middle schools and retain cooperative-team links. Dated update 2026-09-03.

NE-004 | NY | Section IV member schools
URL: https://www.sectionivathletics.com/page/member-schools
Category: regional_school_directory; Priority: P1; Rank: extra
Sports: All sports
Fields: Member school identity and linked school resources
Coverage: Southern Tier / Section IV
Format: HTML (Apptegy)
Status: opened_verified (2026-10-04)
Evidence: https://www.sectionivathletics.com/page/member-schools
Notes: Seed section member schools then visit athletics staff/coaching pages. Verify list and school closures before assuming current sport participation.

NE-005 | NY | Section IX OCIAA spring track handbook
URL: https://www.sectionixathletics.org/springtrack/20232024/OCIAAspringtrackHandbook2024.pdf
Category: regional_coach_contacts; Priority: P2; Rank: extra
Sports: Outdoor track
Fields: Committee member school, coach/name, email, phone
Coverage: OCIAA / Section IX subset; 2024 handbook
Format: PDF, 14 pages
Status: opened_verified (2026-10-04)
Evidence: https://www.sectionixathletics.org/springtrack/20232024/OCIAAspringtrackHandbook2024.pdf
Notes: Contains a Spring Track Coaches Committee table. Useful named-coach leads, NOT every Section IX coach. Source season is 2024 even if search crawl is recent; verify every current appointment on school site.

NE-006 | NY | PSAL Cross Country school/team listing
URL: https://www.psal.org/sports/sport.aspx?flag=All&spCode=033
Category: state_athletics_sport_directory; Priority: P1; Rank: 2
Sports: Boys and girls cross-country; sport menu links indoor/outdoor
Fields: School/team names and IDs; published client code renders head, co-, and assistant coach names from team records
Coverage: NYC PSAL programs; campus programs may span schools
Format: ASP.NET HTML; client-routed team profiles
Status: opened_verified (2026-10-04)
Evidence: https://www.psal.org/sports/sport.aspx?flag=All&spCode=033
Notes: Observed profile https://www.psal.org/profiles/team-profile.aspx#033/13507; boys/girls XC codes 033/034. Public https://www.psal.org/scripts/js/Team_Profile.js verified with getTeamDetails and getCoachNamesIds calls and head/co/assistant coach rendering. These are observed site-internal endpoints, not a supported public API; actual JSON responses not tested. Harvest real school/sport IDs and current season from page; fragment needs JS. Never call getTeamRosters or athlete/statistics endpoints for this project.

NE-007 | NY | NYSAIS member school directory
URL: https://www.nysais.org/schools/
Category: independent_school_directory; Priority: P1; Rank: extra
Sports: All schools; not all have track/XC
Fields: Member school lookup and school links
Coverage: New York independent schools, all grades
Format: HTML/JS directory
Status: opened_verified (2026-10-04)
Evidence: https://www.nysais.org/schools/
Notes: Opened public directory shell; school results may require rendering. Restrict to schools serving grades 9-12, then athletics pages. Complements NYSPHSAA and PSAL.

NE-008 | NY | NYSAIS Athletics
URL: https://www.nysais.org/athletics/
Category: independent_athletics; Priority: P2; Rank: extra
Sports: Track/XC plus other sports
Fields: Athletic association resources; championship/team leads; coordinator contacts where published
Coverage: NYSAIS participating independent schools
Format: HTML and linked documents
Status: opened_verified (2026-10-04)
Evidence: https://www.nysais.org/athletics/
Notes: Use track/XC classification/championship resources to establish participation. Association coordinators are not exhaustive coach directory.

NE-009 | NY | New York State Catholic High School Athletic Association
URL: https://www.chsaany.org/
Category: parochial_athletics; Priority: P1; Rank: extra
Sports: Track/XC plus other sports
Fields: Organization, event and team resources indicated by search
Coverage: Catholic member athletics
Format: SportsEngine-style public site; retrieval failed
Status: blocked (2026-10-04)
Evidence: https://www.chsaany.org/
Notes: Search indexing confirms official site and current news. Direct web open failed; not verified scrape-ready. Keep separate from Colorado CHSAA at chsaanow.com. Use school staff sites if retrieval remains unavailable.

NE-010 | NJ | NJSIAA Member Information
URL: https://www.njsiaa.org/schools/member-information
Category: state_athletics_school_directory; Priority: P1; Rank: 1
Sports: All sports
Fields: School name,address,phone,athletic director name/phone
Coverage: Statewide NJSIAA public, nonpublic, charter members
Format: Public HTML table; ?page=0, ?page=1 pagination
Status: opened_verified (2026-10-04)
Evidence: https://www.njsiaa.org/schools/member-information
Notes: Scrape list pages only and follow real pager links. Opened school detail /schools/abraham-clark-high-school redirects to /user/login?destination=...; do not assume contacts behind detail are public. AD is not track coach.

NE-011 | NJ | NJSIAA individual school details
URL: https://www.njsiaa.org/schools/abraham-clark-high-school
Category: restricted_school_detail; Priority: P3; Rank: extra
Sports: All sports
Fields: Unverified behind login; public list has school/AD
Coverage: NJSIAA individual profiles
Format: Login redirect
Status: login_required (2026-10-04)
Evidence: https://www.njsiaa.org/schools/abraham-clark-high-school
Notes: Specific profile redirects to https://www.njsiaa.org/user/login?destination=/schools/abraham-clark-high-school . Use public member-information listing for seed data. Do not create accounts or bypass.

NE-012 | NJ | New Jersey DOE School Directory
URL: https://homeroom6.doe.nj.gov/directory/
Category: state_education_directory; Priority: P1; Rank: extra
Sports: All schools; sport not supplied
Fields: School/district directory; fields not directly inspected
Coverage: Statewide public/nonpublic directory workflow
Format: Government web application
Status: blocked (2026-10-04)
Evidence: https://www.nj.gov/education/
Notes: Official NJDOE homepage links this current URL. Direct fetch returned HTTP 403; web open failed. The commonly guessed https://www.nj.gov/education/directory/ is 404. Reverify through normal public access, not bypass; national school files can seed meanwhile.

NE-013 | NJ | New Jersey XC/TF Coaches Association resources
URL: https://njxctfca.org/new-jersey-xc-tf-coaches-association/
Category: coaches_association; Priority: P2; Rank: extra
Sports: XC, indoor and outdoor track
Fields: Association resources, jobs, links to county associations; no public statewide membership roster verified
Coverage: Statewide coaches association
Format: WordPress HTML
Status: opened_verified (2026-10-04)
Evidence: https://njxctfca.org/new-jersey-xc-tf-coaches-association/
Notes: Useful hub, not an exhaustive coach database. Some central page content dates to 2020 despite newer sidebar posts. Jobs describe vacancies and must not be interpreted as filled current coaching positions.

NE-014 | NJ | NJXCTFCA county association link index
URL: https://njxctfca.org/links/
Category: coaches_association_index; Priority: P1; Rank: 2
Sports: XC/track
Fields: Verified outgoing county/coaches association URLs
Coverage: NJ county/regional associations
Format: HTML hyperlinks
Status: opened_verified (2026-10-04)
Evidence: https://njxctfca.org/links/
Notes: Links include South Jersey, Shore, Bergen, Passaic, Hudson, Union, Essex, Mercer, Middlesex, Morris. Crawl each subject to current access; some old providers such as webs.com may be obsolete.

NE-015 | NJ | Shore Track Coaches Association
URL: https://shorecoaches.com/
Category: regional_coaches_association; Priority: P2; Rank: extra
Sports: XC, indoor/outdoor track
Fields: Meet/school participation resources; association contacts; no full coach directory verified
Coverage: Monmouth and Ocean counties; HS and MS
Format: WordPress HTML and downloads
Status: opened_verified (2026-10-04)
Evidence: https://shorecoaches.com/
Notes: Opened without www; www version failed. Filter high-school meets. Extract school-level participation or explicitly named staff only, not athlete result rows.

NE-016 | NJ | South Jersey Track Coaches Association
URL: https://www.sjtrack.org/about-us
Category: regional_coaches_association; Priority: P2; Rank: extra
Sports: XC and track
Fields: Association scope, meeting documents, event and school leads
Coverage: Atlantic,Burlington,Cape May,Camden,Cumberland,Gloucester,Salem
Format: Wix HTML and linked meeting documents
Status: opened_verified (2026-10-04)
Evidence: https://www.sjtrack.org/about-us
Notes: Association explicitly serves high-school track/XC coaches in seven counties. Documents may expose officers/attendees but no comprehensive membership directory verified.

NE-017 | NJ | Morris County Track Coaches Association
URL: https://www.mctrack.org/
Category: regional_coaches_association; Priority: P2; Rank: extra
Sports: XC, winter track, spring track
Fields: School/team participation from meet index; named coach service-award leads
Coverage: Morris County and NJAC; statewide NJSIAA result archive as secondary route
Format: Static HTML result indexes
Status: opened_verified (2026-10-04)
Evidence: https://www.mctrack.org/
Notes: Current homepage updates visible 2026-09-29. Use team-level evidence only. Service awards and historic results cannot establish current employment.

NE-018 | NJ | Greater Middlesex Conference Track Coaches Association member schools
URL: https://www.gmctrackcoaches.org/member-schools
Category: regional_school_directory; Priority: P1; Rank: extra
Sports: XC/indoor/outdoor track
Fields: Member school listing (Google Sites embedded content may need render)
Coverage: Greater Middlesex Conference
Format: Google Sites HTML/embedded files
Status: opened_verified (2026-10-04)
Evidence: https://www.gmctrackcoaches.org/member-schools
Notes: Public member-schools page opens. Follow site-rendered school list, then each school athletics staff. No coach emails verified in text extraction.

NE-019 | NJ | NJAIS independent member directory
URL: https://members.njais.org/rolodex/searchOrganizationDirectory
Category: independent_school_directory; Priority: P2; Rank: extra
Sports: All schools
Fields: School search/directory, details not rendered in text reader
Coverage: New Jersey independent association members
Format: JS directory; public entry linked by NJAIS
Status: opened_verified (2026-10-04)
Evidence: https://www.njais.org/home
Notes: Opened zero-text JS shell; verify rendered results before adapter production. Official provenance https://www.njais.org/home . Filter school grade range and sports.

NE-020 | MA | Massachusetts State Track Coaches Association current members
URL: https://mstca.org/coaches-corner/current-members
Category: coaches_association_member_directory; Priority: P1; Rank: 1
Sports: Track and XC association; individual sports/roles not labeled
Fields: Member ID, first/last name, organization, email and phone in public embedded page data; membership status
Coverage: MSTCA membership; not a complete current coaching census
Format: Next.js HTML plus interactive pagination
Status: opened_verified (2026-10-04)
Evidence: https://mstca.org/coaches-corner/current-members
Notes: Direct GET verified Next.js self.__next_f.push page-data with members array: 24 rows on first page, totalCount 1417. Fields: _id, firstName, lastName, organization, email, phone, status, lastPaymentDate, image, altText. First page had 24 email values, 11 equal unknown@mstca.org. Parse public payload or render normal pagination; no separate API verified. Reject placeholders; collect only name/org and professional contact fields. Membership or old payment date does not prove current coach employment/sport. Confirm at school site.

NE-021 | MA | MIAA Schools Directory and school profiles
URL: https://www.miaa.net/schools
Category: state_athletics_school_directory; Priority: P1; Rank: 2
Sports: All sports; sport page linked separately
Fields: School, district, league, principal, athletic director, mascot; published school details
Coverage: MIAA public and private/parochial member schools
Format: Drupal HTML paginated listing and /group/{id} profiles
Status: opened_verified (2026-10-04)
Evidence: https://www.miaa.net/schools
Notes: Observed school profile https://www.miaa.net/group/2 lists principal and AD, not track coaches. Enumerate real links/pagination; use athletics school seed to find staff. Membership PDF at https://www.miaa.net/media/824 is 32 pages but landing says updated 2024-07-18.

NE-022 | MA | MIAA certified coaches by school
URL: https://www.miaa.net/sites/default/files/2024-09/1-certified-coaches-by-school.pdf
Category: coach_certification_roster; Priority: P2; Rank: extra
Sports: All sports, no sport field
Fields: First name,last name,school
Coverage: MIAA/MSAA certified coaches
Format: Text PDF, 383 pages
Status: opened_verified (2026-10-04)
Evidence: https://www.miaa.net/sites/default/files/2024-09/1-certified-coaches-by-school.pdf
Notes: PDF pages bear 9/18/2026 while landing labels 09-17-26; retain both source dates. No sport, email or proof of current employment. Useful name-school corroboration only; cannot use as current XC/track roster. Discover current PDF from https://www.miaa.net/you-are/coaches rather than filename date.

NE-023 | MA | MIAA league directory
URL: https://www.miaa.net/sites/default/files/2024-06/miaa-league-directory.pdf
Category: regional_school_directory; Priority: P2; Rank: extra
Sports: All sports
Fields: League membership and school information
Coverage: MIAA leagues across districts
Format: Text PDF, 22 pages
Status: opened_verified (2026-10-04)
Evidence: https://www.miaa.net/sites/default/files/2024-06/miaa-league-directory.pdf
Notes: File path date is not edition date. Use document contents and landing update date. Complements school directory for league-based expansion, not a coach list.

NE-024 | MA | Massachusetts DESE public/private directories
URL: https://profiles.doe.mass.edu/search/search.aspx
Category: state_education_directory; Priority: P1; Rank: 3
Sports: All schools; no sport proof
Fields: School/org name,organization type, profiles and export/search capabilities
Coverage: Statewide public, charter, private, approved special education
Format: ASP.NET search/directories and export workflow
Status: opened_verified (2026-10-04)
Evidence: https://profiles.doe.mass.edu/search/search.aspx
Notes: Observed public-directory link https://profiles.doe.mass.edu/search/search_link.aspx?leftNavId=11238&orgType=6,13&runOrgSearch=Y and private link orgType=11. Link pages opened as short redirect shell; render normal search if needed. No documented JSON API found. Include high-school grade ranges, then school athletics sites.

NE-025 | MA | MassGIS Massachusetts Schools dataset metadata
URL: https://www.mass.gov/info-details/massgis-data-massachusetts-schools-pre-k-through-high-school
Category: state_education_geodata; Priority: P2; Rank: extra
Sports: All pre-K through HS
Fields: School location,DESE_ID,type,grade/contact attributes described in metadata
Coverage: Public,private,charter,vocational and special education
Format: Official HTML metadata; shapefile ZIP download; data-hub search
Status: opened_verified (2026-10-04)
Evidence: https://www.mass.gov/info-details/massgis-data-massachusetts-schools-pre-k-through-high-school
Notes: Direct GET returned HTTP 200 after web-reader failure. Official link observed: https://s3.us-east-1.amazonaws.com/download.massgis.digital.mass.gov/shapefiles/state/schools.zip . Dataset is December 2025 using DESE profiles as of 2025-11-05. Exact linked ZIP was not downloaded. Use DESE_ID, school type/grades and official website attributes for high-school seeds. No coaches in this dataset.

NE-026 | MA | Schools ArcGIS FeatureServer (MassGIS-sourced)
URL: https://services1.arcgis.com/TXaY625xGc0yvAuQ/arcgis/rest/services/Schools/FeatureServer
Category: school_geodata_endpoint; Priority: P3; Rank: extra
Sports: Pre-K through HS
Fields: Feature service layer with school geographic attributes; inspect schema
Coverage: Massachusetts schools, MassGIS sourced; host owner not yet verified
Format: ArcGIS REST FeatureServer; JSON query support
Status: opened_verified (2026-10-04)
Evidence: https://services1.arcgis.com/TXaY625xGc0yvAuQ/arcgis/rest/services/Schools/FeatureServer
Notes: Service directory explicitly says Data Source MassGIS and supports JSON; max record count 1,000. This establishes technical availability, not authoritative publisher or freshness. Inspect layer 0 fields, item owner/update date before use; only read/query, never applyEdits. Prefer official MassGIS data hub download.

NE-027 | MA | MIAA Track & Cross Country
URL: https://www.miaa.net/track-cross-country
Category: state_athletics_sport_resources; Priority: P2; Rank: extra
Sports: XC,indoor,outdoor track, boys/girls
Fields: Season information,formats,alignments,team/meet links; coordinator resources
Coverage: MIAA track/XC programs
Format: HTML and linked PDFs/Athletic.net
Status: opened_verified (2026-10-04)
Evidence: https://www.miaa.net/track-cross-country
Notes: Official page states MIAA partners with Athletic.net for XC/indoor/outdoor entries. Use linked alignments/entry participation to verify sport offering, then staff pages. Do not collect student entries/results.

NE-028 | CT | CIAC high school coach directory
URL: https://ciac.fpsports.org/Directory.aspx?SchoolLevelID=1
Category: state_athletics_coach_directory; Priority: P1; Rank: extra
Sports: Boys/girls XC,indoor,outdoor plus other sports
Fields: Search-indexed school,address,sport,role,head coach name,phone
Coverage: CIAC high schools including public and private members
Format: ASP.NET full directory; location/login gate encountered
Status: blocked (2026-10-04)
Evidence: https://ciac.fpsports.org/Directory.aspx?SchoolLevelID=1
Notes: Search results expose extensive sport-specific head-coach rows, but direct open redirects to https://ciac.fpsports.org/WebBotZone.aspx with Unauthorized Access and login request. Strongest apparent statewide coach source; not verified scrape-ready from this environment. Do not bypass. Public school staff websites/EdSight seeds are fallback.

NE-029 | CT | CAS-CIAC MobileDir legacy school list
URL: https://www.casciac.org/mobiledir/
Category: state_athletics_school_directory; Priority: P1; Rank: 1
Sports: All sports
Fields: School names,towns; school detail href identifiers
Coverage: CAS/CIAC school members; multiple grade levels
Format: HTML list; search.cgi?a=a&id=406 example
Status: opened_verified (2026-10-04)
Evidence: https://www.casciac.org/mobiledir/
Notes: Opened CAS-CIAC public school-list entry point. IMPORTANT: this is a legacy list containing old/closed school names; reconcile with current EdSight inventory before use. School detail example opened near-empty text. Stronger sport-specific head-coach route is https://ciac.fpsports.org/Directory.aspx?SchoolLevelID=1 but direct access redirects to WebBotZone.aspx Unauthorized Access/login from this location. Do not bypass. Use verified current official school athletics/staff pages for appointments. MobileDir alone supplies neither a current coach roster nor current membership guarantee.

NE-030 | CT | Connecticut EdSight school/district directory and exports
URL: https://public-edsight.ct.gov/general-information
Category: state_education_directory; Priority: P1; Rank: 2
Sports: All schools; sports not supplied
Fields: School,district,type,location,export links; contacts separate
Coverage: Connecticut public and nonpublic school reporting universe
Format: HTML hub plus embedded reports/export to Excel
Status: opened_verified (2026-10-04)
Evidence: https://public-edsight.ct.gov/general-information
Notes: Opened official hub. Observed https://public-edsight.ct.gov/overview/find-schools/find-schools-export and /overview/find-schools/find-school-district . Report notes distinguish nonpublic secondary schools and APSEPs. Use grades/status to seed athletics staff discovery; no documented JSON API verified. Source HTML of /overview/find-schools/find-school-district embeds https://edsight.ct.gov/SASStoredProcess/do with _program=/CTDOE/EdSight/Release/Reporting/Public/Reports/StoredProcesses/OrgSearchReport_SiteCore, orgtype/orgdistrict/orgname query fields. This is an observed embedded report route, not a documented JSON API; report response not tested.

NE-031 | CT | Connecticut EdSight administrative contact search
URL: https://public-edsight.ct.gov/overview/find-contacts
Category: state_education_contacts; Priority: P2; Rank: extra
Sports: All schools
Fields: Administrator/data/support contacts; email/export workflow
Coverage: Statewide education contacts
Format: Embedded report with export
Status: opened_verified (2026-10-04)
Evidence: https://public-edsight.ct.gov/overview/find-contacts
Notes: Official hub says contacts update nightly. These are school administrative contacts, not a coach directory. Use to verify school domain and designated athletic contact only where explicit; do not substitute admin email as coach email.

NE-032 | CT | Connecticut High School Coaches Association committees
URL: https://www.chsca.org/officers.html
Category: coaches_association; Priority: P2; Rank: extra
Sports: XC,indoor,outdoor track plus other sports
Fields: Sport committee representative names and schools
Coverage: 2026-27 association officers/sport representatives, small subset
Format: Weebly HTML
Status: opened_verified (2026-10-04)
Evidence: https://www.chsca.org/officers.html
Notes: Explicit current XC/track representatives and school associations, not statewide member/coach roster. Keep association_role separate from coaching_role; confirm school employment before promoting to coach record.

NE-033 | CT | FCIAC 2021-22 coach directory
URL: https://www.fciac.net/wp-content/uploads/sites/85/2022/09/FCIAC-directory-2021-22.pdf
Category: regional_coach_directory; Priority: P2; Rank: extra
Sports: All sports including XC/track
Fields: School,coach name,sport/role,email,phone
Coverage: Fairfield County Interscholastic Athletic Conference; historical 2021-22 edition
Format: Text PDF, 49 pages
Status: opened_verified (2026-10-04)
Evidence: https://www.fciac.net/wp-content/uploads/sites/85/2022/09/FCIAC-directory-2021-22.pdf
Notes: Actually coach-bearing and contact-rich, but too old to assert current positions. Use historical names/school links only and revalidate on official school staff pages. Do not ingest unrelated personal/home fields.

NE-034 | VT | Vermont Principals Association athletics
URL: https://vpaonline.org/athletics/
Category: state_athletics; Priority: P1; Rank: 1
Sports: XC,indoor,outdoor track and other sports
Fields: Athletics links,rankings,tournament guides/programs; participation evidence
Coverage: Vermont member athletics
Format: WordPress HTML and linked guides
Status: opened_verified (2026-10-04)
Evidence: https://vpaonline.org/athletics/
Notes: Official statewide sport source. https://vpaonline.org/athletic-guides-rules/ shows XC/indoor/outdoor titles but those titles were unlinked in extracted current HTML. Do not invent guide PDFs or claim statewide coach directory. Follow tournament/program links to team lists then schools.

NE-035 | VT | Vermont AOE public school directory hub
URL: https://education.vermont.gov/schools/school-operations/public-schools
Category: state_education_directory; Priority: P1; Rank: extra
Sports: All public schools
Fields: Directory of principals by school; superintendent directory; school/district map
Coverage: Vermont public schools
Format: HTML links to public directory resources
Status: blocked (2026-10-04)
Evidence: https://education.vermont.gov/schools/school-operations/public-schools
Notes: Official indexed page identifies Directory of Principals by School; direct request returned 403. Reverify normal access later, or use Annual Snapshot organization directory. Principal is not track/XC coach.

NE-036 | VT | Vermont AOE independent school directory
URL: https://education.vermont.gov/documents/edu-independent-schools-directory
Category: state_education_private_directory; Priority: P1; Rank: extra
Sports: All independent schools
Fields: School/address/contact/phone/grades from directory
Coverage: Approved and recognized independent schools; includes non-HS/tutorial/program entries
Format: PDF download landing page
Status: blocked (2026-10-04)
Evidence: https://education.vermont.gov/documents/edu-independent-schools-directory
Notes: Search index shows publication 2026-09-22 and filename edu-sy27-independent-school-directory.pdf. Landing direct fetch returned 403; exact current PDF href not verified, so do not guess. Filter grades 9-12 and school vs program; add public-school hub to cover both sectors.

NE-037 | VT | Vermont Annual Snapshot organization directory
URL: https://schoolsnapshot.vermont.gov/Organization/Directory
Category: state_education_directory; Priority: P1; Rank: 3
Sports: All school organizations
Fields: School/organization identity and locality; fields in rendered directory
Coverage: Vermont education organizations
Format: HTML/JS rendered directory
Status: opened_verified (2026-10-04)
Evidence: https://schoolsnapshot.vermont.gov/Organization/Directory
Notes: Public page opens as directory shell but text extraction omits school rows; search indexed rows exist. Browser render needed to verify data route and fields. No JSON endpoint verified; do not confuse login UI at footer with evidence entire directory requires login.

NE-038 | NY | MileSplit NY team directory
URL: https://ny.milesplit.com/teams
Category: sport_team_directory; Priority: P1; Rank: extra
Sports: XC,indoor,outdoor track
Fields: Team/school name,city,team page URL and platform ID
Coverage: State teams including HS,MS,college,clubs and historical/closed records
Format: Public HTML alphabetical directory; level filter
Status: opened_verified (2026-10-04)
Evidence: https://ny.milesplit.com/teams
Notes: Opened state team listing without login. Filter to high school; cross-check current official school inventory and season. Team pages/coach contact presence varies and was not assumed. Do not scrape student rosters, athlete profiles or results. No public documented API verified.

NE-039 | NJ | MileSplit NJ team directory
URL: https://nj.milesplit.com/teams
Category: sport_team_directory; Priority: P1; Rank: 3
Sports: XC,indoor,outdoor track
Fields: Team/school name,city,team page URL and platform ID
Coverage: State teams including HS,MS,college,clubs and historical/closed records
Format: Public HTML alphabetical directory; level filter
Status: opened_verified (2026-10-04)
Evidence: https://nj.milesplit.com/teams
Notes: Opened state team listing without login. Filter to high school; cross-check current official school inventory and season. Team pages/coach contact presence varies and was not assumed. Do not scrape student rosters, athlete profiles or results. No public documented API verified.

NE-040 | MA | MileSplit MA team directory
URL: https://ma.milesplit.com/teams
Category: sport_team_directory; Priority: P1; Rank: extra
Sports: XC,indoor,outdoor track
Fields: Team/school name,city,team page URL and platform ID
Coverage: State teams including HS,MS,college,clubs and historical/closed records
Format: Public HTML alphabetical directory; level filter
Status: opened_verified (2026-10-04)
Evidence: https://ma.milesplit.com/teams
Notes: Opened state team listing without login. Filter to high school; cross-check current official school inventory and season. Team pages/coach contact presence varies and was not assumed. Do not scrape student rosters, athlete profiles or results. No public documented API verified.

NE-041 | CT | MileSplit CT team directory
URL: https://ct.milesplit.com/teams
Category: sport_team_directory; Priority: P1; Rank: extra
Sports: XC,indoor,outdoor track
Fields: Team/school name,city,team page URL and platform ID
Coverage: State teams including HS,MS,college,clubs and historical/closed records
Format: Public HTML alphabetical directory; level filter
Status: opened_verified (2026-10-04)
Evidence: https://ct.milesplit.com/teams
Notes: Opened state team listing without login. Filter to high school; cross-check current official school inventory and season. Team pages/coach contact presence varies and was not assumed. Do not scrape student rosters, athlete profiles or results. No public documented API verified.

NE-042 | VT | MileSplit VT team directory
URL: https://vt.milesplit.com/teams
Category: sport_team_directory; Priority: P1; Rank: 2
Sports: XC,indoor,outdoor track
Fields: Team/school name,city,team page URL and platform ID
Coverage: State teams including HS,MS,college,clubs and historical/closed records
Format: Public HTML alphabetical directory; level filter
Status: opened_verified (2026-10-04)
Evidence: https://vt.milesplit.com/teams
Notes: Opened state team listing without login. Filter to high school; cross-check current official school inventory and season. Team pages/coach contact presence varies and was not assumed. Do not scrape student rosters, athlete profiles or results. No public documented API verified.

NE-043 | NY | NEPSAC independent member schools
URL: https://nepsac.org/about/nepsac-member-schools/
Category: independent_athletics_school_directory; Priority: P1; Rank: extra
Sports: All school sports; XC/track through NEPSTA
Fields: School name and direct official website
Coverage: NEPSAC member prep schools across New England and some NY; state filter needed
Format: HTML linked school list
Status: opened_verified (2026-10-04)
Evidence: https://nepsac.org/about/nepsac-member-schools/
Notes: Shared NEPSAC school-directory system; see CT record for extraction. Restrict to this state and high-school grades.

NE-044 | MA | NEPSAC independent member schools
URL: https://nepsac.org/about/nepsac-member-schools/
Category: independent_athletics_school_directory; Priority: P1; Rank: extra
Sports: All school sports; XC/track through NEPSTA
Fields: School name and direct official website
Coverage: NEPSAC member prep schools across New England and some NY; state filter needed
Format: HTML linked school list
Status: opened_verified (2026-10-04)
Evidence: https://nepsac.org/about/nepsac-member-schools/
Notes: Shared NEPSAC school-directory system; see CT record for extraction. Restrict to this state and high-school grades.

NE-045 | CT | NEPSAC independent member schools
URL: https://nepsac.org/about/nepsac-member-schools/
Category: independent_athletics_school_directory; Priority: P1; Rank: 3
Sports: All school sports; XC/track through NEPSTA
Fields: School name and direct official website
Coverage: NEPSAC member prep schools across New England and some NY; state filter needed
Format: HTML linked school list
Status: opened_verified (2026-10-04)
Evidence: https://nepsac.org/about/nepsac-member-schools/
Notes: Public list has direct school websites and covers institutions outside state associations. Filter state and upper-school grades; some schools are middle-only. Follow athletics > teams/coaches/staff; members do not all sponsor XC/track. No statewide coach roster implied.

NE-046 | VT | NEPSAC independent member schools
URL: https://nepsac.org/about/nepsac-member-schools/
Category: independent_athletics_school_directory; Priority: P1; Rank: extra
Sports: All school sports; XC/track through NEPSTA
Fields: School name and direct official website
Coverage: NEPSAC member prep schools across New England and some NY; state filter needed
Format: HTML linked school list
Status: opened_verified (2026-10-04)
Evidence: https://nepsac.org/about/nepsac-member-schools/
Notes: Shared NEPSAC school-directory system; see CT record for extraction. Restrict to this state and high-school grades.

NE-047 | MA,CT,VT,NY | NEPSTA cross country/track coaches association
URL: https://nepsac.org/coaches-associations/boys-girls-sports-2/boys-girls-cross-country-track-nepsta/
Category: independent_coaches_association; Priority: P2; Rank: extra
Sports: Boys/girls XC and track
Fields: 2026-27 executive-board names/emails; divisions and meeting resources
Coverage: New England preparatory track association
Format: HTML plus linked PDF/minutes
Status: opened_verified (2026-10-04)
Evidence: https://nepsac.org/coaches-associations/boys-girls-sports-2/boys-girls-cross-country-track-nepsta/
Notes: Verified current president/VP/secretary and meeting documents. Only association officers are exposed; no full coaching roster. Current division headings were plain text in reader, so verify actual link availability before implementing downloads.

NE-048 | NJ | Home Campus New Jersey selector (untested NJ data lead)
URL: https://www.cifsshome.org/widget/school/directory?school_id=19&section_id=1
Category: supplemental_untested_lead; Priority: P3; Rank: extra
Sports: School sports including XC/track; NJ coverage not tested
Fields: Shared page code supports coach names, sport, role, email; NJ rows not verified
Coverage: New Jersey selector exists; no NJ school sample validated
Format: Public HTML widget with AJAX JSON routes
Status: opened_verified (2026-10-04)
Evidence: https://www.cifsshome.org/widget/school/directory?school_id=19&section_id=1
Notes: Direct GET 200 verified dropdown New Jersey value 12. Do not treat CA sample as NJ data. Form field is section; merely appending section_id=12 without a valid school did not select NJ. Observed read routes /widget/schools/get with school,section_id,status=active,hide_from_directory=0 and /widget/get-school-details/{observed_id}/details. Not tested for NJ. Honor hide_from_directory and never enumerate hidden IDs. This is observed site code, not documented supported API.



### MIDWEST HIGH-SCHOOL COACH AND SCHOOL SOURCE GUIDE
Verified 2026-10-04 | States: OH, MI, IN, IL, IA

Scope
This catalog targets adult high-school track-and-field and cross-country staff and school identities. It does not authorize harvesting student/athlete data, joining associations, logging in, contacting coaches, or sending outreach. Three distinct source systems per state are ranked below. The JSON preserves useful extra routes without pretending they are independent databases.

Verification key
opened_verified: actual source page, document landing, or rendered cloud-browser page inspected. A landing page being verified does not mean its download, staff tab, or API payload succeeded; see each note.
search_only: indexed evidence only, not a successful open.
blocked: direct source fetch was inaccessible or returned a failure. This is not proof the site blocks every client or requires login.
No full crawl or statewide record-count reconciliation was run.

OHIO: THREE SYSTEMS
1. OHSAA / public myOHSAA
Seed: https://www.ohsaa.org/school-resources/school-enrollment
Coach sample: https://officials.myohsaa.org/Outside/Schedule/SportsInformation?ohsaaId=582
This is the strongest immediately implementable route in these five states. Collect published OhsaaSchoolId values, request the coach page for those IDs, and extract only Cross Country and Track & Field. Observed 2026-27 sample has boys/girls head-coach columns, divisions and public email text. Use the exact officials.myohsaa.org host.
2. OATCCC
https://www.oatccc.com/Coaches/Membership/
Follow the current Members sheet. The opened 2026 sheet has person name, school and county, useful for additional candidates beyond head coaches. Membership alone does not establish a current job or coaching specialty.
3. Ohio DEW / OEDS
https://oeds.education.ohio.gov/dataextract
Broad school identity and coverage audit. Select public, nonpublic, charter/community and STEM school types, then secondary grades. The separate NCNP annual lists address a private-school coverage gap.
Extra: Catholic leagues, OATCCC leadership, and Central District XC assignments are supplemental, with individual verification limitations recorded in JSON.

MICHIGAN: THREE SYSTEMS
1. MHSAA
https://www.mhsaa.com/schools
Verified example: https://www.mhsaa.com/schools/novi
In the cloud browser, search by school name, select a High School result, then Staff → Coaches. That tab rendered named boys/girls XC and track staff without login. This is a JS adapter, not evidence of a documented JSON API. No coach email appeared in the sample.
2. Michigan CEPI EEM
https://cepi.state.mi.us/eem/PublicDatasets.aspx
Use school-level public and nonpublic entities, preserving EEM identifiers. Public export instructions explicitly support logged-out downloads and contact spreadsheets. Do not confuse administrator contacts with sport-coach contacts.
3. MITCA
https://mitca.org/MITCA/
Current association site, contacts and regional coach-award pages. Treat it as partial enrichment. The site mentions migration; discover future links from its live navigation. A comprehensive public member list was not verified.
Extra: Michigan MDE nonpublic lists and CHSL member-school pages help locate private schools. CHSL spans Michigan and Ohio, so filter physical school state.

INDIANA: THREE SYSTEMS
1. Indiana DOE directory
https://www.in.gov/doe/it/data-center-and-reports/
Exact discovered XLSX: https://www.in.gov/doe/files/2025-2026-school-directory-2026-03-23.xlsx
The file downloaded; workbook sheet/column coverage was not fully inspected. Read headers before mapping and filter schools by secondary grades.
2. IHSAA
https://www.ihsaa.org/schools/ihsaa-school-directory
The official hub points to myIHSAA for contact profiles. Only the JS application shell was opened, so a full profile parser remains unverified. Use the opened statewide conference list at https://www.ihsaa.org/schools/athletic-conferences as a dependable fallback school seed.
3. IATCCC
https://iatccc.org/officers-and-council/
Current officers supply named adult contacts and school affiliations. Import only the current-year table as current candidates; historical tables stay historical. A paid membership form is not an extractable public member database.
Extra: Eventlink's public XC school search can establish participation, IACS covers smaller Christian schools, and Hoosier Crossroads supplies official athletic-domain links. These are not substitutes for validating each school's coaches.

ILLINOIS: THREE SYSTEMS
1. IHSA
Directory: https://www.ihsa.org/schools/school-directory
New sample: https://www.ihsa.org/schools/details/0235
School identity, website, phone, conference and offered XC/track entries were verified in the cloud browser. The staff panel failed after one retry. Legacy /data/school/schools/0235.htm returned 404 despite old indexed coach snippets. Do not build new production code around those snippets.
Verified coach-history fallback:
https://www.ihsa.org/data/ccb/records/index.htm
https://www.ihsa.org/data/ccg/records/index.htm
https://www.ihsa.org/data/trb/records/index.htm
https://www.ihsa.org/data/trg/records/index.htm
Follow the actual By School alphabetical links and retain season, gender and sport. Historical rows must not silently become current staff.
2. Illinois ISBE
https://www.isbe.net/Pages/Data-Analysis-Directories.aspx
Nightly workbook link: https://www.isbe.net/_layouts/Download.aspx?SourceUrl=/Documents/dir_ed_entities.xls
Use RCDTS and NCES IDs, relevant public/nonpublic tabs, school type and grade range. Binary download was not verified here. The page has a generic archived footer despite current links; keep the retrieval date and actual workbook vintage.
3. ITCCCA
https://www.itccca.com/itccca-officers
Officers/coordinators and linked coach/assistant-coach awards provide a small enrichment set. No full publicly accessible statewide membership database was verified.
Extra: IHSA's conference index and Chicago Catholic League expose school/league websites; use these to drive school-athletics fallbacks.

IOWA: THREE SYSTEMS
1. IHSAA / Bound
https://www.iahsaa.org/member-schools/
The public member table supplies statewide school names and conferences; https://www.iahsaa.org/schools/acgc/ is an opened detail example. IHSAA's official contact page refers users to https://www.gobound.com/ia/schools for contacts.
Bound example https://www.gobound.com/ia/schools/northbutler/Directory had indexed sport-coach roles, but direct open returned 403. Mark Bound extraction blocked/unverified in this environment; do not pretend it is a working API.
2. Iowa Department of Education
https://educate.iowa.gov/directories
Current public and nonpublic building XLSX links are available. A separately verified machine-readable route is:
https://services.arcgis.com/vPD5PVLI6sfkZ5E4/arcgis/rest/services/IowaSchoolBldgs/FeatureServer
Both layer schemas and the public query UI opened. Layer 0 is public, layer 1 private. The REST service has older school-year coverage than the current spreadsheet, so reconcile them.
3. IATC
https://www.iatrackcoaches.org/
The opened membership list has high-school names/classes, not a roster of individual coaches. Contacts, advisory boards and coach awards enrich adult staff candidates. The girls association's own homepage returned 403; retain a girls-coverage check rather than assuming boys resources are complete.

RUST IMPLEMENTATION CONTRACT
1. Separate acquisition into school seed adapters, sport-participation adapters and adult-staff adapters. Never require an email to preserve a valid coach appointment.
2. Model School, Person, SportProgram and CoachingAppointment separately. A person can coach both genders and both sports at one school; a school can share a cooperative team or have coaches employed by another district.
3. Retain source_system, source_url, retrieved_at, source_season, effective_school_year, source_school_id and field-level evidence. Treat IDs as strings, keeping leading zeros. Do not merge internal service IDs with display IDs.
4. Use reqwest with a descriptive user agent, per-host throttles, caching, conditional requests and bounded retries. Parse ordinary tables with scraper; Excel with calamine; JSON with serde. Use a browser-rendering acquisition stage only for public JS views when permitted. No bypassing sign-in, CAPTCHAs or site access restrictions.
5. Do not infer full APIs from frontend bundle names, arbitrary JSON routes or path guesses. For ArcGIS, schema/Query pages document actual service layers. Test a small read first and record the returned response before promising a working data feed.
6. ArcGIS query contract: send a URL-encoded GET to the observed layer /query with where, outFields, returnGeometry=false and f=json. Cap the first test to one record, then use documented pagination/object-ID batches. Query payload attempts through the research web tool were inaccessible; schema verification alone is the current evidence. Do not claim response-payload verification.
7. Schools: exact source ID first; otherwise normalized name + city/state + address/domain, with explicit alias/co-op handling. Head/assistant/event-coach roles must come from source wording. Preserve unknown, vacant and not-offered states separately.
8. Follow verified official school/athletics URLs for remaining gaps. Inspect adult staff directories, coaching-staff pages and sport contact sections; exclude athlete rosters, results, biographies and student contact data. School-provided professional contact text is preferable to guessed email patterns.
9. For staff currency, explicit current-season school pages outrank old awards and historical season tables. A page updated today may still describe a retired coach. Keep historical appointments rather than overwriting dates.
10. Coverage report per state: all secondary schools, athletic-association members, verified XC/track programs by gender, programs with at least one verified current coach, programs with published coach email, unresolved/vacant/co-op programs, and excluded non-HS entries. Coach-count targets cannot be inferred from athlete counts.

DELIVERY
39 catalog records: OH 9, MI 7, IN 7, IL 8, IA 8.
33 opened_verified; 3 blocked; 3 search_only.
15 ranked primary entries represent three distinct systems for each state. Extra routes from the same organization do not inflate the count of distinct systems.




### WESTERN HIGH-SCHOOL TRACK & FIELD / CROSS-COUNTRY SOURCE GUIDE
Washington, Oregon, Arizona, Utah, Nevada
Verified 2026-10-04. Research scope: public, read-only school and adult-coach acquisition; no outreach, signup, or student data collection.

DELIVERABLES
coach_sources_west.json: 48 evidence-tagged source records, including alternatives, restrictions, and failures.
coach_sources_west_shortlist.json: exactly three recommended distinct source systems per state (15 records).
The shortlist ranks practical implementation starting points. It is not a claim that every selected source independently contains all coaches statewide. Local school-page routes and association subsets are explicitly limited.

RECOMMENDED THREE PER STATE

WASHINGTON
1. WIAA Washington member-school directory
https://wiaa-dna4aga5arc0gyeb.westus2-01.azurewebsites.net/directory.aspx?SecID=654
Statewide athletics school seeds; search by school, type, level, classification, league, district. Directory form verified; statewide coach export not established. Current landing https://www.wiaa.com/schools/ was blocked to direct fetch.
2. Seattle Public Schools official coach-page network
https://westseattlehs.seattleschools.org/student-life/athletics/coaches/
https://ballardhs.seattleschools.org/student-life/athletics/athletics-contacts/
These are verified examples of adult coach and email pages. The school-selector navigation yields high-school domains for a district-wide follow-through. Local coverage, not all Washington. Sports and gender sections must remain attached to the right coach.
3. Washington State Cross Country Coaches Association
https://www.wsccca.com/leadership
Concrete coach-school pairs and some email contacts across district representatives. This is a small representative list, not the membership directory. Supplement with WSTFCA and school-site verification.
Important restriction: https://eds.ospi.k12.wa.us/DirectoryEDS.aspx displays an express noncommercial-use notice. It is excluded from the recommended commercial-ingestion shortlist. Its derived ArcGIS service is not a workaround.

OREGON
1. OSAA school directory/profile system
https://www.osaa.org/schools/regions
https://www.osaa.org/schools/46 (verified profile fixture)
Enumerate actual school links. Extract target rows from Sports / Activities; preserve a separate administrator table. The compact https://www.osaa.org/coaches-directory was search-indexed but direct 403. OSAA includes some Washington-border members: physical state controls assignment.
2. Oregon Department of Education Institutions Database
https://www.ode.state.or.us/instID/
Documentation: https://www.oregon.gov/ode/schools-and-districts/Pages/Institution-Identification-School-Names.aspx
Authoritative IDs and active/open institution reconciliation, with a documented daily zipped XLS extract. Download itself was not successfully retrieved. Current school-directory PDF is a fallback. ODE does not register/approve all private schools, so it is not exhaustive private coverage.
3. Oregon Athletic Coaches Association awards/directories
https://oregoncoach.org/oaca-coach-of-the-year/
https://oregoncoach.org/directory/
The dated awards page supplies target-sport coach-school-classification pairs; coverage is selected winners only. The directory gateway points to OSAA and is not an independent duplicate statewide database.

ARIZONA
1. AIA school-search JSON plus official school profiles
https://aiaonline.org/schools
https://aiaonline.org/schools/search.json?q=chandler
https://aiaonline.org/schools/100
JSON endpoint and /schools/{id} routing were observed in official bundled JavaScript and then tested. Public profiles supply target head-coach names. Administrator/coach email directory redirects to login; do not claim public emails.
2. Canyon Athletic Association GameSource
https://teams.gamesource.io/teams.php?association=CAA&association_sport_id=44&association_sport_level_id=198&division_id=8
Verified boys varsity track school/team list with head and assistant coaches. This catches a different program population from AIA. Discover actual sport/level IDs using controls. Reconcile dated GameSource rows with https://azcaa.com/sports/track-field and current CAA destinations; Bound currently showed an anti-robot interstitial.
3. Arizona Department of Education active LEA/school inventory
https://www.azed.gov/finance/local-education-agencies
Linked FY2026 XLSX is documented; direct binary fetch failed. Inventory is a public/charter reconciliation layer, not a coach list or full private-school registry. Add the Christian Education Coalition directory for private-school discovery.

UTAH
1. UHSAA school directory
https://uhsaa.org/school-directory-new/
https://uhsaa.org/school-directory/?Reg=6&id=Alta&schoolID=1
Best direct statewide route here: enumerable school links and separate coach rows for boys/girls XC/TF with published emails. Some schools have blank coach fields. UHSAA event contacts and participation PDF provide additional, non-independent validation.
2. Utah State Board of Education directory system
https://schools.utah.gov/schoolsdirectory
https://www.schools.utah.gov/schooldistricts
The school directory's raw HTML references https://cactus.schools.utah.gov/api/legacy/schools, but the endpoint returned 502 twice. District HTML table is verified accessible and can seed school-site fallback. Do not claim a successfully tested statewide JSON response.
3. Grand County High School official coach directory
https://gchs.grandschools.org/apps/pages/index.jsp?uREC_ID=1651964&type=d&pREC_ID=2260652&tota11y=true
A concrete, verified Edlio school-page adapter fixture with target-sport coaches. It is one school, not a statewide association. Extend school-site crawling from UHSAA/USBE seeds. Pine View is a second official template, with published email obfuscation that must be handled faithfully.

NEVADA
1. Southern Nevada Track & Cross Country Coaches Association
https://www.sntccca.org/
Two linked 2026-27 Google Sheets have anonymous CSV exports that returned HTTP 200. This is regional coverage only. Some rows are officials/timers/association staff; exclude those from the school-coach table.
2. Nevada Department of Education inventories
https://doe.nv.gov/school-and-district-information
https://doe.nv.gov/offices/office-of-student-and-school-supports/private-schools/
Both public and private XLSX files downloaded successfully and headers were inspected. Use the files to resolve school identity, grades, active/licensed status and official website, then collect coaches from athletics pages.
3. Washoe County School District official school-site network
https://reno.washoeschools.net/activities/athletics/welcome
https://northvalleys.washoeschools.net/our-school/staff-directory
Concrete northern-Nevada coach-bearing HTML templates. Enumerate the district's high schools, follow athletics/coaching/staff links, and handle linked PDFs when necessary. NIAA directory routes were blocked or stale in this pass, so they are retained as extras rather than presented as current verified coach data.

CONCRETE MACHINE-READABLE ROUTES

AIA school search:
GET https://aiaonline.org/schools/search.json?q=chandler
Observed JSON keys: id, name, full_name, mascot, logo.thumbnail, alignment.conference, alignment.region, address.line1, address.city, address.state, address.zip.
The empty query returned 20 rows. This is not a complete 287-school dump. No verified pagination or unrestricted bulk API exists in this research. Obtain school names from the season-pinned sport alignment directory, query those names, then deduplicate numeric IDs and fetch the matching profile. The current root alignment page pointed to a future unreleased block, so the scraper must explicitly choose season.

Nevada Southern Track CSV:
https://docs.google.com/spreadsheets/d/1cs60d8y7YkfLlLVENZlgx4-kYOqfAPQeJU4fcQmp4oY/export?format=csv
Nevada Southern XC CSV:
https://docs.google.com/spreadsheets/d/1FijPtauqQ9N5v5-EZI0Ts3RirYIN0e2d-RQrFvla-7k/export?format=csv
These stable document URLs redirect to expiring Googleusercontent downloads. Store the stable docs URL, not the redirect. Preamble/header rows do not correctly describe every school-row cell; class often occupies the second column and coach the third. Preserve the original row for review. Do not edit these live community-maintained spreadsheets.

Nevada public schools workbook:
https://webapp-strapi-paas-prod-nde-001.azurewebsites.net/uploads/school_directory_9b69a05740.xlsx
Nevada private schools workbook:
https://webapp-strapi-paas-prod-nde-001.azurewebsites.net/uploads/private_school_directory_b5a0270d46.xlsx
Both were actually retrieved. Public snapshot date was 2026-09-16. Discover hashed file URLs from official landing pages on each refresh rather than freezing them forever.

Utah observed endpoint, presently unavailable:
https://cactus.schools.utah.gov/api/legacy/schools
Evidence is the official school-directory page's dataUrl JavaScript. This establishes existence and purpose only; two 502 responses mean neither schema nor live completeness was verified.

No other JSON API is asserted. ASP.NET, rSchoolToday, Edlio, Finalsite, Google Sheets, and association HTML each need separate evidence-backed adapters. Do not invent /api/coaches endpoints.

IMPLEMENTATION HANDOFF

1. Separate identity from employment. A school record carries state, source ID, official domain, name, physical address, public/private/charter classification, grade span, status and association memberships. A coaching assignment carries person name, role, sport, gender/team, season, school ID, and optional published professional contact. One person can have multiple assignments.
2. Store provenance at field/assignment level: canonical source URL, retrieval UTC timestamp, observed season, verification status, page hash and exact local evidence block or row. Keep null distinct from no team/no coach. Treat contact as published, not deliverability-verified.
3. High-school filtering must use grade span and athletics level, not the substring High School alone. Combined K-12, junior/senior schools and independent schools can qualify; youth clubs, middle schools and colleges do not. Do not collect student rosters, results or biographies.
4. Normalize target labels: BXC/GXC, Boys/Girls Cross Country, Cross-Country, Track & Field, Track and Field and boys/girls Track. Keep coed and unspecified gender explicit. Never turn a meet contact, athletic director, clinic speaker, official or association officer into a head coach without direct role evidence.
5. Favor season-labelled school/association profiles. Keep dated award/clinic material as supporting evidence. Record conflicts instead of silently selecting the first email/name. Recheck assignments with recent staff pages when sources disagree.
6. Adapters: ordinary GET and HTML parsing for OSAA/UHSAA/AIA school pages; CSV parsing for SNTCCCA; XLSX parsing for NDE; structured browser-assisted extraction only when genuinely needed for dynamic widgets. For PDFs use positional extraction when columns collapse, not whitespace guessing. Build fixture tests with one school containing all four sports, one combined coach, one blank coach field, and one private school.
7. Crawl politely: honor published restrictions and robots, identify the client, bound concurrency per host, cache responses, apply backoff and conditional requests, and stop on authorization barriers. A public webpage is not permission to bypass login, CAPTCHA, rate limits or contractual restrictions. AIA authenticated directory and blocked Bound are not targets for bypass.
8. School-site enrichment is necessary for coach email completeness. Follow verified institution domains and actual athletics/staff navigation; avoid guessed email patterns or inferred private contact details. Grand County, Pine View, Seattle and Washoe are verified examples of source variation.
9. Evaluate coverage per physical state, school, sport, team gender and season. Compare the seeded school population against observed program flags. A contact directory may omit nonmembers; participation matrix missingness is not a reliable negative. The same source organization with three URLs is still one source system.

VERIFICATION LEGEND
opened_verified: actual relevant landing/profile/content opened, or successful HTTP binary/CSV retrieval. Notes explicitly distinguish content verified from a landing whose linked data remained unavailable.
search_only: seen in search results but actual content not opened in this pass.
blocked: opening was attempted but returned 403/502/timeout/cache failure/anti-robot barrier. Exact reason is in notes.
login_required: official route led to authentication; no login attempted.

PRIORITIES
P1: implementation first or core authoritative inventory.
P2: useful enrichment, reconciliation, local template, or partial coverage.
P3: low-yield association evidence, dated/blocked candidates, or restricted-use material.

All contact fields are adult professional-role data. No student data is required or recommended.



### HIGH-SCHOOL XC / TRACK COACH AND SCHOOL SOURCE GUIDE
States: Virginia, Missouri, Oklahoma, Louisiana, South Dakota, Pennsylvania, Kansas
Verification date: 2026-10-04
Scope: Public read-only source research. No signups, outreach, authentication, student-data collection, or production scraping performed.

DELIVERABLE
The companion JSON contains 63 records. shortlist_rank identifies exactly three distinct preferred source systems per state. Additional URLs are preserved because they offer concrete extraction examples, sport participation filters, private-school gaps, or explicit access limitations.
A source being opened is NOT proof its whole directory can be exported, and it is NOT confirmation a 2025 coach is still employed in 2026. See each record's implementation_notes.

STATUS LEGEND
opened_verified: The exact page/file was opened. For a JS shell or hub, only that shell/hub is verified unless notes explicitly state actual rows were inspected.
search_only: Discovered in search; target not opened.
blocked: Opening failed or returned an access/error/challenge response. This does not prove the website is universally inaccessible.
login_required: Tested route redirected to login. No restricted material was accessed.
P0: First-pass seed or high-value bulk source.
P1: Major enrichment or coverage source.
P2: Selective lead, historical source, or access-limited optional route.
P3: Quarantine pending source-integrity validation.

WHAT THE TOP THREE ACTUALLY GET YOU
VA: VDOE provides a clean statewide public-school HTML inventory. VISAA gives private-school domains/conferences. VHSL alignment is an additional athletics coverage check. None of these proves current coaching assignments; crawl public official school staff/team pages next. VHSL Reports directory itself returned 403.
MO: MSHSAA is the strongest school-to-coach pipeline. MTCCCA provides selected coach-school corroboration. MNSAA fills private-school identity gaps, with high-school filtering required. DESE remains valuable but tested download/report routes redirected to login, so it is not falsely advertised as an anonymous export.
OK: OSSAARankings yields a large school list and sport menus; OSDE supplies the official school-directory hub; NCPSA adds a directly readable accredited-private inventory. OCCTCA is relevant but its site/PDF retrieval failed here, so it is retained as an optional source rather than displacing a usable top-three inventory.
LA: LHSAA has both a full coaches directory and a clean registered-XC list. BESE nonpublic approval adds school-code/grade coverage. DirectAthletics adds sport team IDs. The full coaches PDF has corrupted text encoding and needs visual/OCR QA; the clean XC roster is from September 2025, not a fresh 2026 staff confirmation.
SD: SDCCTFCA offers real names/emails/titles/schools in a public membership PDF. SDDOE seeds all school types. SDHSAA's official directory is now on Bound, whose JS shell opened but rows were not exposed. The association roster is dated January 2025 and does not enumerate every statewide coach.
PA: PIAA is the athletics seed; EdNA is the statewide public/private education inventory/export system; TFCAofGP provides selected actual regional coach contacts through its handbook. TFCAofGP is leadership/representative coverage, not a statewide roster. PAISAA remains an important extra for independent-school gaps, but its main members page failed to fetch although a member detail page opened. Friends Schools League is another accessible private-school subset.
KS: KSDE reports provide current organizational inventories. KSHSAA league/approved-school pages cover athletics and independent exceptions, with legacy-to-React migration. KCCTFCA is the sport-coach association, but public pages are selective awards/clinic contacts, not a complete membership directory.

HIGHEST-VALUE CONCRETE EXTRACTION ROUTES
1. Missouri MSHSAA
   Seed https://www.mshsaa.org/Schools/SchoolListing.aspx
   Follow actual school links such as https://www.mshsaa.org/MySchool/?s=85
   Then sport and Coaches links, with verified samples:
   https://www.mshsaa.org/MySchool/Coaches.aspx?alg=11&s=85
   https://www.mshsaa.org/MySchool/Coaches.aspx?alg=52&s=85
   The first sample printed a 2025–2026 boys XC roster with head and varsity-assistant roles. The observed boys track code is 52; girls XC schedule code is 12, but its coach page did not open. Do not fabricate remaining codes or infer availability from sequential IDs.
   Some schedule pages displayed a contradictory membership warning while school home showed full membership. Preserve each source's status/year and flag conflicts for current school-page review.

2. Louisiana LHSAA
   Re-discover current files from https://www.lhsaa.org/handbook/cross-country
   Current navigation's School Administration Directory points to the 2025–26 full coaches PDF.
   Its text extraction was garbled. Reject gibberish, render pages and OCR, then confirm school boundaries and sport abbreviations before exporting data.
   The separate Registered Coaches file has clean school/last-name/first-name columns, 14 pages, and an explicit 2025-09-22 date. It does not label head versus assistant or publish email in the inspected schema.
   Correct current school-directory URL is https://www.lhsaa.org/school-directory ; the older /schools/school-directory and www.www2 variants should not be hard-coded.
   Track source year and season separately from retrieval date.

3. South Dakota SDCCTFCA
   Start at https://www.sdhsca.org/page/show/6314203-membership
   Follow the membership PDF currently published, rather than guessing SportsEngine CDN paths.
   The inspected PDF has 10 pages with last name, first name, email, member title, school. Public professional-contact listings may use personal-looking domains; do not infer that every email is institution-owned.
   Association membership can include assistants, retirees or coaches whose precise sport/gender is not stated. Keep those qualifiers and reconcile against official school staff.

4. Pennsylvania EdNA
   Start at https://www.edna.pa.gov/Screens/Extracts/wfExtracts.aspx
   Public forms:
   https://www.edna.pa.gov/Screens/Extracts/wfExtractPublicSchools.aspx
   https://www.edna.pa.gov/Screens/Extracts/wfPNP.aspx
   Both forms opened without authentication; report files were not generated.
   For a Rust implementation, request the page first, inspect named controls and preserve session/ASP.NET hidden state; submit only the actual public report form. Do not guess endpoint parameters.
   Select Open records, separate LEAs/dioceses from school sites, and join the public-grade extract to identify schools offering high-school grades.

5. Kansas KSDE
   https://uapps.ksde.gov/directory_rpts/default.aspx
   The opened selector offers organizational reports for 2026–27, including Active Building Report (Excel), accredited organization raw data, public HS lists, and accredited/nonaccredited private school lists.
   The educator report is explicitly much older (2023–24; updated 2024-10-28). It is not a verified coach roster.
   Respect requested output formats: some reports are PDF only; active-building raw data is Excel only.

6. Virginia public/private seeds
   Public: https://www.va-doeapp.com/publicschoolsalphabetical.aspx?w=true
   Private: https://www.visaa.org/schools
   Both exposed actual school records. Public table provides grade span and school-division URLs; VISAA cards provide official websites and athletic conference labels.
   Use latest VISAA boys/girls XC and outdoor-track division files as participation filters, keeping future 2026–28 lists distinct from present-season evidence.
   VDOE principal-contact CSV is advertised in search, but its hub returned 403 and no exact downloadable CSV was tested.

7. Oklahoma inventories
   OSSAARankings school IDs and sport links are public HTML. OSDE's download hub describes useful contact fields, but its binary link failed in this session. Treat the hub as verified and the file schema as untested.
   NCPSA's private-school directory exposed actual school/city/accreditor/grade entries.
   OSDE also links OPSAC, but OPSAC's opened member page lacked records and its homepage included placeholder material and unrelated gambling text. Quarantine this route instead of blindly trusting a historically official link.

DATA MODEL AND MERGING
Use separate entities for School, AthleticProgram, Coach, CoachingAssignment, and SourceObservation.
School identity: internal ID, source-system ID, state, name, city, county/parish, district/system, address, grade range, public/private status, official website.
AthleticProgram: school or cooperative ID, sport (XC, indoor track, outdoor track), gender, varsity/junior-high level, association/class/region, season.
Coach: adult professional name and only explicitly published professional contact data.
CoachingAssignment: coach ID, program ID, head/assistant/event role exactly as supported, season, valid-from/last-seen, confidence.
SourceObservation: source URL, displayed publication/season date, fetched-at time, verification status, raw-page/PDF hash, page/row locator, extraction quality, source role (primary school, association, education inventory, secondary corroboration).
Do not collapse every school name match: Sacred Heart, Central, Trinity, and Christian Academy recur. Match state+city/address+official domain and preserve source IDs. For co-ops, associate one program with multiple constituent schools; do not duplicate the same coach as separate employment evidence for each school.
Store administrative contacts as AD/principal, never silently relabel as coach.

SCRAPER BOUNDARIES AND QUALITY GATES
- Public school and adult professional staff information only. Do not follow athlete rosters, results tables, DOBs, student emails, eligibility, or scholarship applications.
- School inventory ≠ participation inventory ≠ coach inventory ≠ email directory.
- An association officer, event director, award winner or clinic speaker is a selective lead, not all coaches or automatically a current school head coach.
- Prefer official school pages for current employment; dated association PDFs provide discovery/corroboration.
- No documented public statewide coach JSON API was verified for these states. ASPX/CFM page routes and public report forms are website endpoints, not promised supported APIs.
- Dynamic Bound, AthleticNET, KSHSAA React and School Finder pages need a public rendered-page validation step. Do not claim their hidden APIs, export permissions or auth status have been tested.
- Respect robots/terms, cache responses, limit per-host concurrency, honor Retry-After, and stop at login/CAPTCHA/access controls. No bypass, credential entry, bulk email guessing or outreach is part of this task.
- For PDFs, inspect extraction quality before row parsing. Reject corrupted text, capture page provenance and verify OCR against a representative sample. Preserve empty fields instead of inventing emails.
- Completeness metrics should report coverage of target schools/programs with verified current coaches, not just rows downloaded. Explicitly count missing sport evidence, stale role, inaccessible source and ambiguous school matches.
- Re-fetch current season hubs and compare hashes/dates before relying on saved files. Publication dates in search can be wrong; use displayed document dates when available.

RANKED THREE-SOURCE SHORTLIST

VA
1. Virginia Department of Education public-school alphabetical directory
   https://www.va-doeapp.com/publicschoolsalphabetical.aspx?w=true
   Status: opened_verified. Format: Large public HTML table.
2. VISAA member-school directory
   https://www.visaa.org/schools
   Status: opened_verified. Format: Public HTML school cards.
3. VHSL alignment/classification hub
   https://www.vhsl.org/alignment/
   Status: opened_verified. Format: HTML hub with linked alignment documents.

MO
1. MSHSAA member-school listing and coaching-roster route
   https://www.mshsaa.org/Schools/SchoolListing.aspx
   Status: opened_verified. Format: Public HTML table plus per-school HTML.
2. Missouri Track and Cross Country Coaches Association
   https://www.mtccca.org/
   Status: opened_verified. Format: Public Wix HTML.
3. Missouri Nonpublic School Accrediting Association member schools
   https://www.moqualityschools.com/member-listing1.html
   Status: opened_verified. Format: Public HTML directory.

OK
1. OSSAARankings school directory and sport schedules
   https://www.ossaarankings.com/default.aspx?sc=1117&sel=ssch&st=OK
   Status: opened_verified. Format: Public ASP.NET HTML.
2. Oklahoma State School and District Directory
   https://oklahoma.gov/education/resources/state-school-directory.html
   Status: opened_verified. Format: Public HTML download hub.
3. NCPSA Oklahoma accredited-private-school directory
   https://ncpsa.org/directory/oklahoma/
   Status: opened_verified. Format: Public HTML city groups.

LA
1. LHSAA 2025–2026 Coaches Directory
   https://www.lhsaa.org/siteuploads/editorimg/file/Administration/25-26/LHSAA%20Coaches%20Directory%20-%202025%20-%202026.pdf
   Status: opened_verified. Format: Public PDF.
2. Louisiana BESE-approved nonpublic-school list 2026–2027
   https://doe.louisiana.gov/docs/default-source/nonpublic-schools/information---nps-2026-2027-approval-with-brumfield-v-dodd.pdf?sfvrsn=aa8fa3de_2
   Status: opened_verified. Format: Public 19-page PDF table.
3. DirectAthletics Louisiana track team directory
   https://www.directathletics.com/leagues/track/89.html
   Status: opened_verified. Format: Public HTML link table.

SD
1. SDCCTFCA 2024–2025 membership roster
   https://cdn1.sportngin.com/attachments/document/cdfc-2949014/_1__SDCCTFCA_Membership_as_of_01-07-2025_.pdf
   Status: opened_verified. Format: Public text-readable PDF table.
2. South Dakota Department of Education educational directory
   https://doe.sd.gov/ofm/edudir.aspx
   Status: opened_verified. Format: Public HTML index, district query pages, Excel/PDF links.
3. SDHSAA official member directory on Bound
   https://www.gobound.com/sd/associations/sdhsaa/schools
   Status: opened_verified. Format: Public JavaScript app.

PA
1. PIAA school directory
   https://www.piaa.org/schools/directory/default.aspx
   Status: opened_verified. Format: Public ASP.NET directory hub/list/detail.
2. Pennsylvania EdNA output files
   https://www.edna.pa.gov/Screens/Extracts/wfExtracts.aspx
   Status: opened_verified. Format: Public ASP.NET export forms; Excel outputs.
3. Track and Field Coaches Association of Greater Philadelphia handbook
   https://www.tfcaofgp.org/coaches-handbook/
   Status: opened_verified. Format: Public HTML. Coverage: regional leaders, meet directors and league representatives only.

KS
1. Kansas Educational Directory Reports
   https://uapps.ksde.gov/directory_rpts/default.aspx
   Status: opened_verified. Format: Public ASP.NET reports; PDF and Excel options.
2. KSHSAA school leagues directory
   https://kshsaa.org/Public/General/Leagues.cfm
   Status: opened_verified. Format: Public HTML legacy hub; some routes migrating to React.
3. Kansas Cross Country and Track and Field Coaches Association
   https://www.kcctfca.com/
   Status: opened_verified. Format: Public Wix HTML.

FULL VERIFIED/QUALIFIED SOURCE INDEX
The JSON contains field-level coverage, source-system grouping and implementation caveats for every record. Entries below are additional links, not automatic endorsements of fresh coach data.

VA
- Virginia Department of Education public-school alphabetical directory
  https://www.va-doeapp.com/publicschoolsalphabetical.aspx?w=true
  opened_verified; P0; state_education_directory
- VISAA member-school directory
  https://www.visaa.org/schools
  opened_verified; P0; private_athletics_directory
- VHSL alignment/classification hub
  https://www.vhsl.org/alignment/
  opened_verified; P0; state_athletics_inventory
- VHSL member-directory entry point
  https://www.vhsl.org/about-vhsl/
  opened_verified; P1; state_athletics_directory
- VHSL Reports school directory
  https://vhslreports.com/school_directory/?letter=C&schoolName=&state=VA
  blocked; P2; state_athletics_directory
- VISAA cross-country participation documents
  https://www.visaa.org/sports/cross-country
  opened_verified; P1; sport_participation
- VISAA outdoor-track participation documents
  https://www.visaa.org/sports/track-and-field
  opened_verified; P1; sport_participation
- VDOE school-directories download hub
  https://www.doe.virginia.gov/about-vdoe/virginia-school-directories
  blocked; P2; state_education_exports
- VCPE accreditation verification
  https://www.vcpe.org/Verify-School-Accreditation
  opened_verified; P1; private_school_inventory
- Virginia Metro Athletic Conference member schools
  https://virginiametroathletics.org/member-schools/
  opened_verified; P1; regional_league_directory
- Virginia Track Coaches Association
  https://runsignup.com/MemberOrg/VTCA
  opened_verified; P2; coaches_association

MO
- MSHSAA member-school listing and coaching-roster route
  https://www.mshsaa.org/Schools/SchoolListing.aspx
  opened_verified; P0; state_athletics_directory
- Missouri Track and Cross Country Coaches Association
  https://www.mtccca.org/
  opened_verified; P1; coaches_association
- Missouri Nonpublic School Accrediting Association member schools
  https://www.moqualityschools.com/member-listing1.html
  opened_verified; P1; private_school_inventory
- MSHSAA boys cross-country coaches sample
  https://www.mshsaa.org/MySchool/Coaches.aspx?alg=11&s=85
  opened_verified; P0; actual_coach_directory
- MSHSAA boys track-and-field coaches sample
  https://www.mshsaa.org/MySchool/Coaches.aspx?alg=52&s=85
  opened_verified; P1; actual_coach_directory
- Missouri DESE school directory
  https://dese.mo.gov/data-system-management/directory
  opened_verified; P1; state_education_directory
- Missouri DESE MCDS data-download report
  https://apps.dese.mo.gov/MCDS/Reports/SSRS_Print.aspx?Reportid=ee8cf509-bf32-455e-b49e-c366a23b37db
  login_required; P2; state_education_exports
- Missouri Christian School Athletic Association member schools
  https://mocsaa.com/member-schools/
  opened_verified; P2; private_athletics_directory

OK
- OSSAARankings school directory and sport schedules
  https://www.ossaarankings.com/default.aspx?sc=1117&sel=ssch&st=OK
  opened_verified; P0; state_athletics_directory
- Oklahoma State School and District Directory
  https://oklahoma.gov/education/resources/state-school-directory.html
  opened_verified; P0; state_education_directory
- NCPSA Oklahoma accredited-private-school directory
  https://ncpsa.org/directory/oklahoma/
  opened_verified; P1; private_school_inventory
- Oklahoma Cross Country and Track Coaches Association
  https://www.ohstrack.com/
  blocked; P1; coaches_association
- OCCTCA advisory-board contact PDF
  https://www.ohstrack.com/occtcaofficeradvisorycontactinfo.pdf
  blocked; P2; coaches_association_contacts
- OSSAA cross-country classifications and regional assignments
  https://ossaaillustrated.com/cross-country/
  blocked; P2; sport_participation
- Heartland Christian Athletic Association directory
  https://www.heartlandathletics.com/
  blocked; P2; private_athletics_directory
- Oklahoma Christian Academy XC/track coaches
  https://www.ocacademy.org/cc-track
  opened_verified; P1; school_staff_fallback
- OPSAC member schools (quality warning)
  https://opsac.org/member-schools/
  opened_verified; P3; private_school_inventory

LA
- LHSAA 2025–2026 Coaches Directory
  https://www.lhsaa.org/siteuploads/editorimg/file/Administration/25-26/LHSAA%20Coaches%20Directory%20-%202025%20-%202026.pdf
  opened_verified; P0; actual_coach_directory
- Louisiana BESE-approved nonpublic-school list 2026–2027
  https://doe.louisiana.gov/docs/default-source/nonpublic-schools/information---nps-2026-2027-approval-with-brumfield-v-dodd.pdf?sfvrsn=aa8fa3de_2
  opened_verified; P0; state_private_school_inventory
- DirectAthletics Louisiana track team directory
  https://www.directathletics.com/leagues/track/89.html
  opened_verified; P1; sport_team_directory
- LHSAA registered XC coaches, September 22 2025
  https://www.lhsaa.org/siteuploads/editorimg/file/Cross%20Country/2025%20XC/Registered%20coaches%20list%20as%20of%209-22-25.pdf
  opened_verified; P0; actual_coach_directory
- LHSAA public school-directory search
  https://www.lhsaa.org/school-directory
  opened_verified; P1; state_athletics_directory
- LHSAA XC classifications and alignments hub
  https://www.lhsaa.org/handbook/cross-country
  opened_verified; P1; sport_participation
- LHSAA outdoor-track hub
  https://www.lhsaa.org/outdoor-track-and-field
  opened_verified; P1; sport_participation
- Louisiana High School Coaches Association
  https://www.lhsaa.org/lhsca
  opened_verified; P2; coaches_association
- ACEL 2026–2027 cross-country
  https://www.theacel.com/26-27-cross-country.html
  opened_verified; P2; private_athletics
- Louisiana School Finder
  https://louisianaschools.com/
  blocked; P2; state_education_directory

SD
- SDCCTFCA 2024–2025 membership roster
  https://cdn1.sportngin.com/attachments/document/cdfc-2949014/_1__SDCCTFCA_Membership_as_of_01-07-2025_.pdf
  opened_verified; P0; actual_coach_directory
- South Dakota Department of Education educational directory
  https://doe.sd.gov/ofm/edudir.aspx
  opened_verified; P0; state_education_directory
- SDHSAA official member directory on Bound
  https://www.gobound.com/sd/associations/sdhsaa/schools
  opened_verified; P1; state_athletics_directory
- SDHSAA cross-country participation hub
  https://sdhsaa.com/activity/cross-country/
  opened_verified; P1; sport_participation
- SDHSAA track-and-field hub
  https://sdhsaa.com/activity/track-field/
  opened_verified; P1; sport_participation
- SDHSAA athletics cooperatives
  https://sdhsaa.com/athletics-cooperatives/
  opened_verified; P1; cooperative_crosswalk
- SDCCTFCA membership/area-alignment hub
  https://www.sdhsca.org/page/show/6314203-membership
  opened_verified; P1; coaches_association

PA
- PIAA school directory
  https://www.piaa.org/schools/directory/default.aspx
  opened_verified; P0; state_athletics_directory
- Pennsylvania EdNA output files
  https://www.edna.pa.gov/Screens/Extracts/wfExtracts.aspx
  opened_verified; P0; state_education_exports
- PAISAA independent-school member directory
  https://www.paisaasports.org/page/show/925102-members
  blocked; P1; private_athletics_directory
- EdNA public-school export form
  https://www.edna.pa.gov/Screens/Extracts/wfExtractPublicSchools.aspx
  opened_verified; P1; state_education_exports
- EdNA private and nonpublic export form
  https://www.edna.pa.gov/Screens/Extracts/wfPNP.aspx
  opened_verified; P1; state_private_school_inventory
- Track and Field Coaches Association of Greater Philadelphia handbook
  https://www.tfcaofgp.org/coaches-handbook/
  opened_verified; P1; coaches_association_contacts
- Friends Schools League
  https://www.fslathletics.org/
  opened_verified; P1; regional_private_league
- Inter-Academic League membership/history
  https://interacathletics.com/sports/2020/2/21/GEN_0221202650.aspx
  opened_verified; P2; regional_private_league
- Pennsylvania Track and Field Coaches Association
  https://www.ptfca.org/
  blocked; P2; coaches_association
- Philadelphia Catholic League
  https://aopathletics.org/
  opened_verified; P2; regional_parochial_league

KS
- Kansas Educational Directory Reports
  https://uapps.ksde.gov/directory_rpts/default.aspx
  opened_verified; P0; state_education_exports
- KSHSAA school leagues directory
  https://kshsaa.org/Public/General/Leagues.cfm
  opened_verified; P0; state_athletics_directory
- Kansas Cross Country and Track and Field Coaches Association
  https://www.kcctfca.com/
  opened_verified; P1; coaches_association
- KSHSAA approved schools
  https://kshsaa.org/Public/General/ApprovedSchools.cfm
  opened_verified; P1; nonmember_athletics_inventory
- KSDE annual education directory PDFs
  https://ksde.gov/data-and-reporting/directories
  opened_verified; P1; state_education_directory
- KCCTFCA 2026 winter clinic
  https://www.kcctfca.com/winter-coaching-clinic
  opened_verified; P2; coaches_association_contacts
- Flint Hills Christian school sports/coaches
  https://flinthillschristianschool.com/sports-home/
  opened_verified; P1; school_staff_fallback
- KCAA track team directory on AthleticNET
  https://www.athletic.net/track-and-field-outdoor/division/80932
  search_only; P2; private_sport_team_directory




### NATIONAL SCHOOL / TRACK-XC COACH SOURCE CATALOG
Verified on 2026-10-04. This is a shared-source adjunct to the requested three-per-state list.

MOST IMPORTANT CORRECTIONS
1. A count of about 102,000 in a public all-school file is not a high-school count. The exact downloaded 2024–25 CCD directory has 102,178 all-grade rows. Applying any of G_9_OFFERED/G_10_OFFERED/G_11_OFFERED/G_12_OFFERED == Yes gives 29,065 records; using school-year active statuses 1,3,4,5,8 gives 29,061. These counts include territories, alternative/combined/special schools and schools with no sports. They are not counts of track/XC programs. Filter requested states, reconcile current status, and confirm program participation separately.
2. NCES publishes private-school bulk data. Downloaded PSS 2023–24 CSV contains 22,510 distinct PPIN records; 8,750 have any grade 9–12 offered. Test P265/P275/P285/P295 == 1. HIGR2024 is a recode: 14=9th,15=10th,16=11th,17=12th. Do not treat HIGR2024 as a raw grade number. Combined K–12 schools must remain eligible. PSS response coverage is not a perfect census of every currently operating private institution; supplement from state agencies and associations.
3. EDGE geocode-only layers do not contain grade spans. Join CCD/PSS. Public EDGE administrative layer is /1 and has grade span/status/phone but no WEBSITE. The CCD ZIP has WEBSITE. Live EDGE admin count is 101,110, different from the downloaded CCD directory; do not assume identical universes or silently blend release counts.
4. Clell Wade Coaches Directory explicitly prohibits virtual assistants and third-party data collectors. It is an exclusion/licensing record, not a crawler recommendation. DragonFly's Public Directory documentation does not establish anonymous bulk access. Athletic.net permission and undocumented private API access were not assumed.

INGESTION RULES
- Use CCD NCESSCH and PSS PPIN as string IDs; retain leading zeros. Separate source school ID from canonical ID and athletics-platform ID.
- Fetch complete official bulk files once; store survey year, checksum, source URL, retrieval date, grade predicate, status predicate and row counts. Count only after your target-state filter.
- Preserve source type and school level. Do not restrict to a name containing High School or LEVEL=High; that loses combined/private schools and other schools serving high-school grades.
- Prefer exact CCD grade-offered flags. Grade-span numeric intersection with 9–12 is a fallback; normalize PK/KG/UG/AE, handle grade 13 and unknowns explicitly, and avoid string range comparisons.
- Separate institution inventory, sport-program participation and current coach assignment. No federal directory verifies school track/XC participation or coaches.
- For coach records capture school ID, sport, gender, season, level (varsity/JV/MS), role, public professional name/contact, exact evidence URL, as-of date and source confidence. Keep conflicting and stale assignments reviewable.
- Do not collect student rosters, athlete profiles, medical/eligibility information or personal contact details not published for professional contact. Do not infer email addresses or automate outreach from this research authorization.
- Follow official published links, site terms and rate limits; stop at login, paywall or access restriction. A platform pattern is not a verified universal endpoint or documented bulk API.
- ArcGIS layer JSON documents query fields and limits. Current NCES services advertise maxRecordCount=2000 and pagination/orderBy support. Select only school fields, returnGeometry=false where unnecessary, order deterministically, inspect exceededTransferLimit, and reconcile downloaded counts. Trim padded state codes.

VERIFICATION LEGEND
verified_downloaded_parsed / inspected: fetched actual official bytes and inspected local data/schema.
verified_schema_and_live_query: obtained schema and representative response/count from official REST.
verified_opened / live_html: inspected current page; does not establish permission to bulk scrape.
verified_indexed: search-index content only; no claim of a fresh endpoint success.
verified_official_catalog_link_not_downloaded: direct link published by official source; bytes not fetched.
platform_adapter_example: a real school page showing reusable markup family; NOT nationwide coverage.
restricted_do_not_crawl: explicit restriction or access boundary.

1. NCES CCD downloadable files selector
URL: https://nces.ed.gov/ccd/files.asp#Fiscal:2,SchoolYearId:39,Page:1
Category: school_inventory
Scope: School inventory; no sport participation or coach contacts
Fields: NCESSCH; state school ID; school/district name; grade flags; address; phone; WEBSITE; operational status
Coverage: Public elementary/secondary schools nationwide; filter requested states and high-school grades
Format: Official HTML selector linking ZIP CSV/SAS and XLSX companion files
Verification: verified_live_html_and_selector_data_2026-10-04
Evidence: https://nces.ed.gov/ccd/files.asp#Fiscal:2,SchoolYearId:39,Page:1
Priority: 1
Notes: Preferred discovery page. Legacy pubschuniv.asp direct HTTPS returned an outdated table in this environment; use current files selector. Selector currently labels collection v.2a while directory download filename remains 1a. Save actual source filename, checksum, download timestamp, survey year, schema and per-component version.

2. NCES CCD 2024–25 school directory CSV/SAS ZIP
URL: https://nces.ed.gov/ccd/Data/zip/ccd_sch_029_2425_w_1a_073025.zip
Category: school_inventory_bulk
Scope: School inventory; no sport participation or coach contacts
Fields: NCESSCH; SCH_NAME; LEA_NAME; ST_SCHID; ST/LSTATE; address; PHONE; WEBSITE; SY_STATUS/UPDATED_STATUS; G_9_OFFERED through G_12_OFFERED; GSLO/GSHI; LEVEL
Coverage: 102,178 all-grade rows in downloaded file; 29,065 offer any grade 9–12; 29,061 after school-year active-status filter, before state/school-type/athletics checks
Format: ZIP containing CSV and SAS7BDAT
Verification: verified_downloaded_parsed_2026-10-04
Evidence: https://nces.ed.gov/ccd/files.asp#Fiscal:2,SchoolYearId:39,Page:1
Priority: 1
Notes: Use any of G_9_OFFERED,G_10_OFFERED,G_11_OFFERED,G_12_OFFERED == Yes. Active school-year statuses are 1,3,4,5,8; exclude 2 Closed,6 Inactive,7 Future. Updated status can supersede school-year status for current outreach; preserve both. Keep combined K–12 and other schools if they offer HS grades. File includes territories; limit to requested states. NCES school ID must be a string with leading zeros.

3. NCES CCD 2024–25 school directory companion
URL: https://nces.ed.gov/ccd/xls/SY_2024-25_SCH_Directory_Companion_2026-005d.xlsx
Category: data_dictionary
Scope: School inventory; no sport participation or coach contacts
Fields: Field definitions; grade flags; status codes; notes
Coverage: Companion for CCD school directory
Format: XLSX
Verification: verified_downloaded_inspected_2026-10-04
Evidence: https://nces.ed.gov/ccd/xls/SY_2024-25_SCH_Directory_Companion_2026-005d.xlsx
Priority: 1
Notes: This is the schema authority. Status codes verified: 1 Open,2 Closed,3 New,4 Added,5 Changed Boundary/Agency,6 Inactive,7 Future,8 Reopened. Check updated status separately and do not infer athletic participation.

4. NCES EDGE public-school administrative REST layer
URL: https://nces.ed.gov/opengis/rest/services/K12_School_Locations/EDGE_ADMINDATA_PUBLICSCH_2425/MapServer/1
Category: school_inventory_api
Scope: School inventory; no sport participation or coach contacts
Fields: NCESSCH; SCH_NAME; LEA_NAME; address; PHONE; GSLO/GSHI; SCHOOL_LEVEL; STATUS/SY_STATUS_TEXT; SCHOOL_TYPE_TEXT; LATCOD/LONCOD
Coverage: Nationwide public-school administrative and geographic layer; 101,110 rows in live count
Format: Documented ArcGIS REST; JSON/GeoJSON/PBF; query endpoint
Verification: verified_schema_and_live_query_2026-10-04
Evidence: https://nces.ed.gov/opengis/rest/services/K12_School_Locations/EDGE_ADMINDATA_PUBLICSCH_2425/MapServer/1
Priority: 1
Notes: Layer is /1, not /0. /0 returned Layer not found. MaxRecordCount 2000, pagination/orderBy supported. Query only needed fields, returnGeometry=false. Iterate resultOffset/resultRecordCount with orderByFields=OBJECTID; verify count and exceededTransferLimit, or retrieve IDs then batch. LSTATE may contain trailing spaces. This layer omits WEBSITE; CCD ZIP is more useful as canonical school inventory. GSHI in 09/10/11/12 yielded 28,843 preliminary rows, but exact CCD grade flags are preferable and include atypical grade spans.

5. NCES EDGE public-school geocode REST layer
URL: https://nces.ed.gov/opengis/rest/services/K12_School_Locations/EDGE_GEOCODE_PUBLICSCH_2425/MapServer/0
Category: school_geography_api
Scope: School inventory; no sport participation or coach contacts
Fields: NCESSCH; NAME; address; STATE; county; LAT/LON; locale; SCHOOLYEAR
Coverage: Public elementary and secondary school locations, all grades
Format: ArcGIS REST JSON/GeoJSON/PBF
Verification: verified_schema_2026-10-04
Evidence: https://catalog.data.gov/dataset/public-school-locations-2024-25
Priority: 1
Notes: NO grade span or school-level fields in this layer. Do not count all rows as high schools. Join CCD on NCESSCH. MaxRecordCount 2000. Public-domain metadata verified.

6. NCES EDGE public-school geocode bulk ZIP
URL: https://nces.ed.gov/programs/edge/data/EDGE_GEOCODE_PUBLICSCH_2425.zip
Category: school_geography_bulk
Scope: School inventory; no sport participation or coach contacts
Fields: School identity and location/geographic fields
Coverage: Nationwide public-school geocodes, 2024–25
Format: ZIP
Verification: verified_official_catalog_link_not_downloaded_2026-10-04
Evidence: https://catalog.data.gov/dataset/public-school-locations-2024-25
Priority: 2
Notes: Download link published in Data.gov NCES metadata. Join CCD directory for website, grades, type and status.

7. NCES PSS data and documentation index
URL: https://nces.ed.gov/surveys/pss/pssdata.asp
Category: private_school_inventory
Scope: School inventory; no sport participation or coach contacts
Fields: Private-school bulk data, record layout, codebook, questionnaire, methodological frame files
Coverage: Biennial nationwide private school survey; 2023–24 currently downloadable
Format: HTML download index; ZIP CSV/SAS/SPSS; PDF; frame CSV
Verification: verified_live_html_2026-10-04
Evidence: https://nces.ed.gov/surveys/pss/pssdata.asp
Priority: 1
Notes: Private bulk source exists. Page says 2025–26 results expected spring 2027. Permanent PPIN is cross-year join key. Survey weights are for statistical estimates, not duplicate school generation. Nonresponding/frame entities may require supplemental state/private-association sources.

8. NCES PSS 2023–24 public-use CSV ZIP
URL: https://nces.ed.gov/surveys/pss/zip/pss2324_pu_csv.zip
Category: private_school_inventory_bulk
Scope: School inventory; no sport participation or coach contacts
Fields: PPIN; PINST; PL_ADD/PL_CIT/PL_STABB/PL_ZIP; mailing address; PPHONE; grade flags P265/P275/P285/P295; latitude/longitude; recoded grade span
Coverage: 22,510 distinct school records in downloaded CSV; 8,750 offer one or more grades 9–12
Format: ZIP containing pss2324_pu.csv
Verification: verified_downloaded_parsed_2026-10-04
Evidence: https://nces.ed.gov/surveys/pss/pssdata.asp
Priority: 1
Notes: Exact HS filter: any of P265,P275,P285,P295 == 1. Values 1=Yes,2=No. HIGR2024 is recoded: 14=9th,15=10th,16=11th,17=12th; never use numeric HIGR>=9 as if it were a grade number. Preserve combined schools. No coach fields or school website field found in this CSV. School search About Data currently says 22,502 responding schools; downloaded CSV/codebook both report 22,510, so do not interchange counts.

9. NCES PSS 2023–24 record layout
URL: https://nces.ed.gov/surveys/pss/pdf/layout2023_24.pdf
Category: data_dictionary
Scope: School inventory; no sport participation or coach contacts
Fields: Public-use column names/types/descriptions
Coverage: 2023–24 PSS CSV
Format: PDF
Verification: verified_downloaded_text_inspected_2026-10-04
Evidence: https://nces.ed.gov/surveys/pss/pdf/layout2023_24.pdf
Priority: 1
Notes: Defines P265/P275/P285/P295 as ninth/tenth/eleventh/twelfth grade offered. Companion codebook contains value encodings.

10. NCES PSS 2023–24 codebook
URL: https://nces.ed.gov/surveys/pss/pdf/codebook2023_24.pdf
Category: data_dictionary
Scope: School inventory; no sport participation or coach contacts
Fields: Question labels; values; recode meanings; imputation flags
Coverage: 2023–24 PSS
Format: PDF
Verification: verified_downloaded_text_inspected_2026-10-04
Evidence: https://nces.ed.gov/surveys/pss/pdf/codebook2023_24.pdf
Priority: 1
Notes: Use grade-offered flags rather than LEVEL alone: combined schools also serve high-school students. Can retain imputation flags for confidence/provenance.

11. NCES PSS 2023–24 methodological frame file
URL: https://nces.ed.gov/surveys/pss/xls/2023-24_PSS_Frame_Data.csv
Category: private_school_coverage_reconciliation
Scope: School inventory; no sport participation or coach contacts
Fields: PSS frame and response/eligibility tracking variables; consult dictionary before use
Coverage: Survey frame, including entities omitted from responding-school data
Format: CSV
Verification: verified_link_in_official_live_index_not_downloaded_2026-10-04
Evidence: https://nces.ed.gov/surveys/pss/pssdata.asp
Priority: 2
Notes: Coverage reconciliation source, not automatically a valid list of active high schools. Dictionary: https://nces.ed.gov/surveys/pss/pdf/2023-24_PSS_Frame_File_Data_Dictionary.pdf

12. NCES EDGE private-school geocode REST layer
URL: https://nces.ed.gov/opengis/rest/services/K12_School_Locations/EDGE_GEOCODE_PRIVATESCH_2324/MapServer/0
Category: private_school_geography_api
Scope: School inventory; no sport participation or coach contacts
Fields: PPIN; NAME; address; STATE; county; LAT/LON; locale; SCHOOLYEAR
Coverage: Private-school locations, 2023–24
Format: ArcGIS REST JSON/GeoJSON/PBF
Verification: verified_schema_2026-10-04
Evidence: https://catalog.data.gov/dataset/private-school-locations-2023-24
Priority: 2
Notes: No grade span fields. Join PSS CSV on PPIN. Public-domain metadata verified. Bulk ZIP https://nces.ed.gov/programs/edge/data/EDGE_GEOCODE_PRIVATESCH_2324.zip is officially catalogued, not downloaded in this pass.

13. NCES public-school search
URL: https://nces.ed.gov/ccd/schoolsearch/index.asp
Category: school_identity_verification
Scope: School inventory; no sport participation or coach contacts
Fields: School name/NCES ID; address; phone; grade span; type
Coverage: Public schools; source banner 2024–25 and 2025–26
Format: HTML search/detail pages
Verification: verified_search_index_content_2026-10-04
Evidence: https://nces.ed.gov/ccd/schoolsearch/index.asp
Priority: 2
Notes: Useful for manual identity checks; prefer CCD/EDGE bulk for 25-state ingestion. Not a coaches directory.

14. NFHS state association directory
URL: https://nfhs.org/about/state-association-directory
Category: association_discovery
Scope: All interscholastic sports including track/XC
Fields: Member and affiliate state associations; links/contact details
Coverage: National interscholastic association discovery
Format: Dynamic HTML
Verification: indexed_content_live_web_fetch_403_2026-10-04
Evidence: https://nfhs.org/about/state-association-directory
Priority: 2
Notes: Association discovery, not school coach inventory. Page has member and affiliate tabs; private associations may require affiliate coverage. Do not claim NFHS self-service spreadsheet is publicly accessible: https://utilities.nfhs.org/schooldirectory/selfservice requires sign-in.

15. NHSTFXCCA state track/XC associations
URL: https://www.nhstfxcca.org/state-associations/
Category: track_xc_association_discovery
Scope: High-school track & field and cross country
Fields: State coaching-association links
Coverage: National state-association hub; uneven state availability
Format: HTML
Verification: verified_live_html_2026-10-04
Evidence: https://www.nhstfxcca.org/state-associations/
Priority: 1
Notes: Especially useful seed list for sport-specific state directories. Live site has migrated from indexed Google Sites content to WordPress; retain final URLs and recheck individual links.

16. NIAAA state athletic-administrator association directory
URL: https://niaaa.org/about-the-niaaa/state-association-directory
Category: association_discovery
Scope: All high-school sports
Fields: State athletic-director association links
Coverage: 50 states and DC links
Format: HTML
Verification: verified_opened_2026-10-04
Evidence: https://niaaa.org/about-the-niaaa/state-association-directory
Priority: 2
Notes: Useful AD fallback and association discovery; no national school coach export exposed here.

17. NASO state resource guide
URL: https://www.naso.org/resources/state-resource-guide/
Category: association_discovery
Scope: All high-school sports
Fields: State high-school athletic/activity association links
Coverage: National association links
Format: HTML
Verification: verified_opened_2026-10-04
Evidence: https://www.naso.org/resources/state-resource-guide/
Priority: 3
Notes: Fallback to NFHS dynamic page; some outbound links are older and must be reverified. Not individual coach data.

18. NHSACA national coaches association
URL: https://nhsaca.org/
Category: association_discovery
Scope: All high-school sports including track/XC
Fields: State links; coach awards; school/coach news
Coverage: National and partner-state coverage
Format: HTML/PDF links
Verification: verified_opened_2026-10-04
Evidence: https://nhsaca.org/
Priority: 3
Notes: Useful discovery and corroboration; awards are sparse and may be historical. Not a complete current coach directory.

19. NFHS coaches awards
URL: https://nfhs.org/stories/awards/coaches-association-awards
Category: coach_corroboration
Scope: Track/XC and other sports
Fields: Coach name; sport; state; award year/level
Coverage: Sparse state/section/national award recipients
Format: HTML search
Verification: verified_indexed_2026-10-04
Evidence: https://nfhs.org/stories/awards/coaches-association-awards
Priority: 3
Notes: Do not infer current appointment from an old award; follow school staff page. Legacy searchable database https://tools.nfhs.org/CoachAwards/CoachAwards/Search is indexed.

20. SIDEARM public staff-directory pattern: Miramonte
URL: https://gomats.org/staff-directory
Category: platform_adapter_example
Scope: Track & field and cross country plus all school sports
Fields: Coach name; sport; level; title; linked staff bio; published email where available
Coverage: One verified high-school example of a reusable platform pattern, not national coverage
Format: HTML table and linked staff pages
Verification: verified_opened_2026-10-04
Evidence: https://gomats.org/staff-directory
Priority: 1
Notes: Verified XC and track head-coach rows. Footer identifies SIDEARM. Discover actual directory links from each school; /staff-directory is a pattern, not a guaranteed endpoint. No public bulk coaches API documented in this pass.

21. Mascot Media public staff-directory pattern: Richland Northeast
URL: https://www.rneathletics.com/directory
Category: platform_adapter_example
Scope: Track & field and cross country plus other school sports
Fields: Coach name; role; public work/team email; optional phone/ext
Coverage: One verified school example, reusable Mascot Media family
Format: HTML cards
Verification: verified_opened_2026-10-04
Evidence: https://www.rneathletics.com/directory
Priority: 1
Notes: Footer identifies Mascot Media; page contains XC, girls track and boys track coaches with public email. Follow observed links; do not assume a private JSON API.

22. Finalsite public school athletics staff pattern: Starr’s Mill
URL: https://smhs.fcboe.org/athletics/coaching-staff
Category: platform_adapter_example
Scope: Track & field and cross country plus other school sports
Fields: Sport; coach name; varsity/JV level; public email; page season
Coverage: One verified school example, reusable school CMS pattern
Format: HTML text and mailto links
Verification: verified_opened_2026-10-04
Evidence: https://smhs.fcboe.org/athletics/coaching-staff
Priority: 1
Notes: Page explicitly says 2025–26 and includes XC/track. Current download date is not proof of 2026–27 appointment. Record season and confidence; use latest school page when conflicting.

23. Edlio public athletics staff pattern: Del Norte
URL: https://delnorte.powayusd.com/apps/pages/coaching-staff
Category: platform_adapter_example
Scope: Track & field and cross country plus other school sports
Fields: Sport; coach name; published email links; season categories
Coverage: One verified school example, reusable school CMS pattern
Format: HTML table and email links
Verification: verified_opened_2026-10-04
Evidence: https://delnorte.powayusd.com/apps/pages/coaching-staff
Priority: 1
Notes: Page lists XC and track. Text renderer masks email labels; inspect ordinary linked email representation when accessible, never guess email. No login is needed for viewed directory.

24. School-owned directory pattern: Wellesley
URL: https://wellesleyps.org/athletics/staff-directory/
Category: platform_adapter_example
Scope: Indoor/outdoor track and cross country plus other sports
Fields: Coach name; varsity/JV/assistant role; indoor/outdoor/XC; public email
Coverage: One verified school-owned searchable directory
Format: HTML alphabetical directory
Verification: verified_opened_2026-10-04
Evidence: https://wellesleyps.org/athletics/staff-directory/
Priority: 1
Notes: Example has separate indoor and outdoor rows and assistant coaches. Keep person and assignment tables separate; school link to BigTeams does not mean the directory itself is BigTeams.

25. School-owned Google Sites coaching directory: Hopatcong
URL: https://sites.google.com/hopatcongschools.org/hopatcongathletics/important-info/coaches-directory
Category: platform_adapter_example
Scope: Track & field and cross country plus other sports
Fields: Coach name; sport; boys/girls; public school email
Coverage: One indexed school-owned directory pattern
Format: Google Sites HTML
Verification: verified_indexed_2026-10-04
Evidence: https://sites.google.com/hopatcongschools.org/hopatcongathletics/important-info/coaches-directory
Priority: 2
Notes: Use only school-controlled/officially linked sites; public contact listing is not consent for automated outreach. No coach contact data copied into this catalog.

26. PlayOn Sites / VNN platform
URL: https://www.playonsports.com/products/vnn
Category: platform_discovery
Scope: All school sports
Fields: School/team website content; coach contacts only if published by school
Coverage: Participating school sites nationwide
Format: School-hosted HTML/JS; product documentation
Verification: verified_opened_redirect_2026-10-04
Evidence: https://www.playonsports.com/products/vnn
Priority: 2
Notes: Old https://www.vnnsports.net/vnn-partner-schools/ is still indexed as a school list but LIVE redirects here; do not treat old index as current enumeration. Discover official athletics sites from school websites. No documented public nationwide coaches API verified.

27. DragonFly Public Directory documentation
URL: https://help.dragonflymax.com/knowledge-base/what-is-the-public-directory-dragonfly-athletics-help-center
Category: platform_access_limited
Scope: All school sports including track/XC
Fields: Staff/team names; positions; email; phone, per official documentation
Coverage: Participating schools and associations
Format: Product documentation; directory access varies
Verification: verified_documentation_opened_access_not_tested_2026-10-04
Evidence: https://help.dragonflymax.com/knowledge-base/what-is-the-public-directory-dragonfly-athletics-help-center
Priority: 3
Notes: Documentation says directory is for other schools/associations and setup requires login. Public in product name does not establish unrestricted anonymous bulk access. Use state-association published directory links; do not register as a coach or request school staff access.

28. MaxPreps track & field school/team discovery
URL: https://www.maxpreps.com/track-field/
Category: sport_team_discovery
Scope: Track & field
Fields: School/team names; state; city; team links; possible coach name on school pages
Coverage: Sport-specific participating/listed high schools; not exhaustive NCES inventory
Format: HTML state/team pages
Verification: verified_opened_2026-10-04
Evidence: https://www.maxpreps.com/track-field/
Priority: 2
Notes: Verified example https://www.maxpreps.com/nc/track-field/schools/. Follow actual state links rather than synthesizing coverage claims. Team presence useful for program validation. No documented public bulk coach-contact API verified; do not collect student roster records.

29. MileSplit team search
URL: https://www.milesplit.com/teams
Category: sport_team_discovery
Scope: Track & field and cross country
Fields: Team/school identity and team links
Coverage: Track/XC teams listed by platform; includes non-HS entities
Format: HTML search; JS/login/paywall behavior varies
Verification: verified_opened_search_shell_2026-10-04
Evidence: https://www.milesplit.com/teams
Priority: 3
Notes: Discovery/corroboration only. Full underlying coach fields not verified. Avoid student profiles/results and do not assume access through undocumented APIs or paid content.

30. SchoolDigger documented school API
URL: https://developer.schooldigger.com/docs
Category: licensed_school_inventory_api
Scope: School inventory; no sport participation or coach contacts
Fields: School identity; grades; address; school type; NCES IDs including private school identifier; inspect current response schema
Coverage: National public/private school directory; API/vendor licensed access
Format: Documented REST JSON with appID/appKey; licensed bulk option
Verification: verified_documentation_opened_no_credentials_used_2026-10-04
Evidence: https://developer.schooldigger.com/docs
Priority: 3
Notes: Documented school list endpoint /v2.4/schools; version 3.0 also exists in current change log. Confirm selected version, license, retention and plan limits. Not a coach API. Primary NCES bulk is preferable before paying for redundant identity data.

31. GreatSchools NearbySchools documented API
URL: https://www.greatschools.org/gk/about/api-developer-resources/
Category: licensed_school_inventory_api
Scope: School inventory; no sport participation or coach contacts
Fields: School name; address; grades offered; directory data
Coverage: U.S. K–12 directory subset per API request; licensed bulk via enterprise
Format: Documented API; enterprise bulk feed
Verification: verified_documentation_opened_no_credentials_used_2026-10-04
Evidence: https://www.greatschools.org/gk/about/api-developer-resources/
Priority: 3
Notes: Developer page links current technical docs. NearbySchools does NOT provide bulk files; enterprise license offers bulk feed. Attribution/branding and subscription requirements apply. No coaches directory documented.

32. Clell Wade Coaches Directory: exclusion/restriction record
URL: https://www.coachesdirectory.com/online-directory
Category: restricted_do_not_crawl
Scope: All sports including track/XC
Fields: Education-based coaches and administrators, per vendor description
Coverage: Nationwide licensed directory
Format: Account/subscription; manual or explicitly negotiated access only
Verification: verified_opened_explicit_restriction_2026-10-04
Evidence: https://www.coachesdirectory.com/online-directory
Priority: 99
Notes: Official page prohibits virtual assistants and third-party data collectors and caps viewing at 300 schools/day. Exclude from automated collection unless vendor grants specific written licensed permission. Do not use free-trial/coach-registration route as a workaround. No access purchased or account created.
