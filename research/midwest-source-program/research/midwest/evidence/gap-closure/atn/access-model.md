# Athletic.net anonymous access model (verified 2026-09-20)

## Gate = browser-grade request headers; no login/cookies/CAPTCHA
Same URL, different headers:
- default/short curl UA -> 403 (Cloudflare interstitial)
- full desktop Chrome UA (+ Accept, Accept-Language, Referer, sec-fetch-*) -> 200 for the endpoints below

## Endpoints verified anonymously (plain HTTPS client)
| endpoint | method | auth | result |
|---|---|---|---|
| /api/v1/Meet/GetMeetData?meetId=&sport=tf | GET | none | 200; meet object + tfDivisions + xcDivisions + eventDivsWithResults + jwtMeet + LiveID + HasResults |
| /api/v1/Meet/GetEventDivisionData?meetId=&sport=tf | GET | anettokens: jwtMeet | 200; events[] (ID/EventShort/Gender/Measure/Type/FieldMeasureType/isHurdle) + tfDivisions |
| /api/v1/Meet/GetAllResultsData?meetId=&sport=tf&rawResults=&showTips= | GET | anettokens: jwtMeet | 200; whole meet: flatEvents[].results[] + relayLegs[] + eventTypes[] + rounds[] + teams[] |
| /api/v1/Meet/GetResultsData3?meetId=&sport=tf | POST | anettokens: jwtMeet | 200 IF body gender is lowercase (m/f); adds Wind, Heat, HeatPlace, Measure, advancementData |
| /api/v1/AthleteBio/GetAthleteBioData?athleteId=&sport=tf&level=0 | GET | none | 200; full profile payload (resultsTF/resultsXC/allSeasons/meets/allTeams) |
| /api/v1/TeamNav/Team?team=&sport=&season= | GET | none | 200; team object incl. Address/City/State/Zip/Mascot/hasIndoor |
| /api/v1/tfRankings/GetRankings | POST | none | 200; rankings rows (grades filter server-side) |
| /api/v1/public/GetStatesCountries2 | GET | none | 200 |
| /api/v1/tfRankings/GetNavInfo?... | GET | page-bootstrapped session headers | 403 from plain client; 200 inside the SPA session (headers: anettokens (aud=jwtTFTopReport) + anet-site-roles-token + anet-appinfo + cookies, incl. cf_clearance) |
| /api/v1/tfRankings/DownloadRankings | POST | role-entitled | 403 anonymous (role-gated bulk export; not needed) |

## Token model
- jwtMeet: meet-scoped JWT minted by the *public* GetMeetData response; echoed back in `anettokens`. Not a user credential.
- anettokens (rankings): TFTopReport-audience token with userId:0 and userRoles:[] - an anonymous session token minted by the page bootstrap. Values are session-scoped; DO NOT persist them in reports or logs (redacted from all saved evidence).
- anet-site-roles-token: site-wide anonymous roles token (userId:0, roles:[]).

## Gotchas found the hard way
1. GetResultsData3: `gender` must be lowercase ("m"/"f"); "M" returns {"currentEventValid":false} with HTTP 200.
2. GetResultsData3 is POST-only (GET -> 405 "does not support http method 'GET'").
3. Field events use their own shorts (shot/discus/hj/lj/pv/tj) - "SP" invalid.
4. Wind is only present in rows when the meet recorded it (per-event endpoint), absent from the bundled GetAllResultsData payload.
5. Relay legs: relay rows have PersonalEvent=false and AthleteName = "<BR>"-joined leg names; the legs are enumerated in the top-level relayLegs[] array (ResultID + ID + AthleteID + Name + ShortDesc(=grade)).
6. Implement/hurdle spec: eventTypes[] rows {eventId, IDEventType, Description} - Description carries the spec ("39\" / 0.991m", "12lb", "1.6kg"); isHurdle comes from GetEventDivisionData.events[].
