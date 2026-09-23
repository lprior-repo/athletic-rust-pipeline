# Captures — state-assoc-midatlantic lane

Anonymous HTTP captures. One request per row; per-host spacing held at >=1.1 s by the capture helper.
User-Agent: `Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36 athletic-census-research/1.0`

Exact command template (one invocation per row; all rows for a batch are passed as further `name:url` arguments):

```
python3 /tmp/cap.py samples /tmp/cap.log \
  "<file>:<url>" ...
```

Helper (`/tmp/cap.py`) fetched with urllib, followed redirects, wrote bytes verbatim, and logged `file<TAB>status<TAB>bytes<TAB>utc_ts<TAB>content_type<TAB>url_effective<TAB>url_requested<TAB>error`.

That helper lived in `/tmp` and no longer exists, and the repository carries no Python; the command above
records how these bytes were fetched, not a re-runnable recipe (provenance rule in `research/README.md`).

| file | HTTP status | bytes | UTC timestamp | URL requested | effective URL |
|---|---|---|---|---|---|
| `samples/robots-nysphsaa.txt` | 200 | 5489 | 2026-09-22T03:54:26Z | https://www.nysphsaa.org/robots.txt | https://www.nysphsaa.org/robots.txt |
| `samples/robots-casciac.txt` | 200 | 1343 | 2026-09-22T03:54:27Z | https://www.casciac.org/robots.txt | https://www.casciac.org/robots.txt |
| _(no file — transport error)_ | ERR | 0 | 2026-09-22T03:54:27Z | https://www.ciacsports.com/robots.txt | — |
| `samples/robots-miaa.txt` | 200 | 2027 | 2026-09-22T03:54:27Z | https://miaa.net/robots.txt | https://www.miaa.net/robots.txt |
| `samples/robots-psal.txt` | 200 | 40245 | 2026-09-22T03:54:27Z | https://www.psal.org/robots.txt | https://www.psal.org/robots.txt |
| `samples/robots-riil.txt` | 200 | 247 | 2026-09-22T03:54:28Z | https://www.riil.org/robots.txt | https://www.riil.org/robots.txt |
| _(no file — transport error)_ | ERR | 0 | 2026-09-22T03:54:29Z | https://www.mpa.ccsso.org/robots.txt | — |
| `samples/robots-nhiaa.txt` | 404 | 13 | 2026-09-22T03:54:29Z | https://www.nhiaa.org/robots.txt | https://www.nhiaa.org/robots.txt |
| `samples/robots-vpa.txt` | 200 | 187 | 2026-09-22T03:54:29Z | https://www.vpaonline.org/robots.txt | https://vpaonline.org/robots.txt |
| `samples/robots-athleticnet.txt` | 200 | 1080 | 2026-09-22T03:54:30Z | https://www.athletic.net/robots.txt | https://www.athletic.net/robots.txt |
| `samples/robots-dcsaa.txt` | 404 | 109040 | 2026-09-22T03:54:30Z | https://www.dcsaa.org/robots.txt | https://www.dcsaa.org/robots.txt |
| `samples/robots-diaa.txt` | 404 | 0 | 2026-09-22T03:54:30Z | https://www.diaa.org/robots.txt | https://www.diaa.org/robots.txt |
| `samples/dcsaa-home.html` | 200 | 109844 | 2026-09-22T03:54:56Z | https://www.dcsaa.org/ | https://www.dcsaa.org/ |
| `samples/delaware-edu-diaa.html` | 403 | 5705 | 2026-09-22T03:54:57Z | https://education.delaware.gov/diaa/ | https://education.delaware.gov/diaa/ |
| `samples/diaa-home.html` | 403 | 5702 | 2026-09-22T03:54:57Z | https://www.diaa.org/ | https://education.delaware.gov/diaa |
| `samples/casciac-home.html` | 200 | 1320 | 2026-09-22T03:54:58Z | https://www.casciac.org/ | https://www.casciac.org/ |
| `samples/miaa-home.html` | 200 | 125645 | 2026-09-22T03:54:58Z | https://www.miaa.net/ | https://www.miaa.net/ |
| `samples/nysphsaa-home.html` | 200 | 154949 | 2026-09-22T03:54:58Z | https://www.nysphsaa.org/ | https://nysphsaa.org/ |
| `samples/psal-home.html` | 200 | 289009 | 2026-09-22T03:54:58Z | https://www.psal.org/ | https://www.psal.org/ |
| `samples/nhiaa-home.html` | 200 | 674484 | 2026-09-22T03:54:59Z | https://www.nhiaa.org/ | https://www.nhiaa.org/ |
| `samples/riil-home.html` | 200 | 108860 | 2026-09-22T03:54:59Z | https://www.riil.org/ | https://www.riil.org/ |
| `samples/mpaonline-home.html` | 200 | 626 | 2026-09-22T03:55:00Z | https://www.mpaonline.org/ | https://flmusiced.org/mpaonline/ |
| `samples/vpa-home.html` | 200 | 188000 | 2026-09-22T03:55:00Z | https://vpaonline.org/ | https://vpaonline.org/ |
| `samples/dc-milesplit-teams.html` | 200 | 52995 | 2026-09-22T03:55:01Z | https://dc.milesplit.com/teams | https://dc.milesplit.com/teams |
| `samples/de-milesplit-teams.html` | 200 | 52333 | 2026-09-22T03:55:10Z | https://de.milesplit.com/teams | https://de.milesplit.com/teams |
| `samples/md-milesplit-teams.html` | 200 | 151367 | 2026-09-22T03:55:11Z | https://md.milesplit.com/teams | https://md.milesplit.com/teams |
| `samples/nj-milesplit-teams.html` | 200 | 190108 | 2026-09-22T03:55:11Z | https://nj.milesplit.com/teams | https://nj.milesplit.com/teams |
| `samples/ny-milesplit-teams.html` | 200 | 419779 | 2026-09-22T03:55:12Z | https://ny.milesplit.com/teams | https://ny.milesplit.com/teams |
| `samples/pa-milesplit-teams.html` | 200 | 297673 | 2026-09-22T03:55:12Z | https://pa.milesplit.com/teams | https://pa.milesplit.com/teams |
| `samples/ct-milesplit-teams.html` | 200 | 104902 | 2026-09-22T03:55:13Z | https://ct.milesplit.com/teams | https://ct.milesplit.com/teams |
| `samples/ma-milesplit-teams.html` | 200 | 171854 | 2026-09-22T03:55:13Z | https://ma.milesplit.com/teams | https://ma.milesplit.com/teams |
| `samples/ri-milesplit-teams.html` | 200 | 48289 | 2026-09-22T03:55:13Z | https://ri.milesplit.com/teams | https://ri.milesplit.com/teams |
| `samples/nh-milesplit-teams.html` | 200 | 60332 | 2026-09-22T03:55:14Z | https://nh.milesplit.com/teams | https://nh.milesplit.com/teams |
| `samples/vt-milesplit-teams.html` | 200 | 63242 | 2026-09-22T03:55:14Z | https://vt.milesplit.com/teams | https://vt.milesplit.com/teams |
| `samples/me-milesplit-teams.html` | 200 | 76491 | 2026-09-22T03:55:15Z | https://me.milesplit.com/teams | https://me.milesplit.com/teams |
| `samples/ciac-fpsports-robots.txt` | 200 | 247 | 2026-09-22T03:55:22Z | https://ciac.fpsports.org/robots.txt | https://ciac.fpsports.org/robots.txt |
| `samples/ciac-fpsports-home.html` | 200 | 76502 | 2026-09-22T03:55:23Z | https://ciac.fpsports.org/ | https://ciac.fpsports.org/ |
| `samples/dcsaa-robots-404-check.txt` | 200 | 111957 | 2026-09-22T03:55:24Z | https://www.dcsaa.org/about | https://www.dcsaa.org/about |
| `samples/mpa-robots.txt` | 200 | 247 | 2026-09-22T03:55:31Z | https://www.mpa.cc/robots.txt | https://www.mpa.cc/robots.txt |
| `samples/sitemap-miaa.xml` | 200 | 166278 | 2026-09-22T03:55:31Z | https://www.miaa.net/sitemap.xml | https://www.miaa.net/sitemap.xml |
| `samples/sitemap-mpssaa.xml` | 200 | 1154 | 2026-09-22T03:55:31Z | https://www.mpssaa.org/sitemap_index.xml | https://www.mpssaa.org/sitemap_index.xml |
| `samples/sitemap-nysphsaa.xml` | 200 | 126066 | 2026-09-22T03:55:31Z | https://www.nysphsaa.org/sitemap.xml | https://nysphsaa.org/sitemap.xml |
| `samples/delaware-doe-robots.txt` | 403 | 5647 | 2026-09-22T03:55:32Z | https://www.doe.k12.de.us/robots.txt | https://education.delaware.gov/ |
| `samples/mpa-home.html` | 200 | 54994 | 2026-09-22T03:55:32Z | https://www.mpa.cc/ | https://www.mpa.cc/ |
| `samples/osse-dc-robots.txt` | 200 | 2294 | 2026-09-22T03:55:32Z | https://osse.dc.gov/robots.txt | https://osse.dc.gov/robots.txt |
| `samples/sitemap-njsiaa.xml` | 200 | 431 | 2026-09-22T03:55:32Z | https://www.njsiaa.org/sitemap.xml | https://www.njsiaa.org/sitemap.xml |
| `samples/sitemap-piaa.xml` | 404 | 51670 | 2026-09-22T03:55:32Z | https://www.piaa.org/sitemap.xml | https://www.piaa.org/sitemap.xml |
| `samples/sitemap-vpa.xml` | 200 | 1031 | 2026-09-22T03:55:32Z | https://vpaonline.org/sitemap_index.xml | https://vpaonline.org/sitemap_index.xml |
| `samples/sitemap-mpssaa-school.xml` | 200 | 26600 | 2026-09-22T03:55:43Z | https://www.mpssaa.org/school-sitemap.xml | https://www.mpssaa.org/school-sitemap.xml |
| `samples/sitemap-mpssaa-page.xml` | 200 | 30826 | 2026-09-22T03:55:44Z | https://www.mpssaa.org/page-sitemap.xml | https://www.mpssaa.org/page-sitemap.xml |
| `samples/sitemap-ciac.xml` | 404 | 1245 | 2026-09-22T03:55:45Z | https://ciac.fpsports.org/sitemap.xml | https://ciac.fpsports.org/sitemap.xml |
| `samples/sitemap-mpa.xml` | 404 | 1245 | 2026-09-22T03:55:45Z | https://www.mpa.cc/sitemap.xml | https://www.mpa.cc/sitemap.xml |
| `samples/sitemap-vpa-page.xml` | 200 | 28554 | 2026-09-22T03:55:45Z | https://vpaonline.org/page-sitemap.xml | https://vpaonline.org/page-sitemap.xml |
| `samples/sitemap-riil.xml` | 404 | 1245 | 2026-09-22T03:55:46Z | https://www.riil.org/sitemap.xml | https://www.riil.org/sitemap.xml |
| `samples/sitemap-nhiaa.xml` | 200 | 48205 | 2026-09-22T03:55:48Z | https://www.nhiaa.org/sitemap.xml | https://www.nhiaa.org/sitemap.xml |
| `samples/sitemap-osse-dc.xml` | 200 | 569617 | 2026-09-22T03:55:48Z | https://osse.dc.gov/sitemap.xml | https://osse.dc.gov/sitemap.xml |
| `samples/sitemap-psal.xml` | 200 | 40245 | 2026-09-22T03:55:48Z | https://www.psal.org/sitemap.xml | https://www.psal.org/sitemap.xml |
| `samples/sitemap-casciac.xml` | 404 | 841 | 2026-09-22T03:55:49Z | https://www.casciac.org/sitemap.xml | https://www.casciac.org/sitemap.xml |
| `samples/njsiaa-home.html` | 200 | 189279 | 2026-09-22T03:56:10Z | https://www.njsiaa.org/ | https://www.njsiaa.org/ |
| `samples/piaa-home.html` | 200 | 60903 | 2026-09-22T03:56:10Z | https://www.piaa.org/ | https://www.piaa.org/ |
| `samples/nysphsaa-sitemap.aspx` | 500 | 644 | 2026-09-22T03:56:11Z | https://www.nysphsaa.org/sitemap.aspx | https://nysphsaa.org/sorry.ashx |
| `samples/dcsaasports-com-home.html` | 200 | 744340 | 2026-09-22T03:56:12Z | https://www.dcsaasports.com/ | https://www.dcsaasports.com/ |
| `samples/dcsaasports-home.html` | 200 | 162889 | 2026-09-22T03:56:12Z | https://www.dcsaasports.org/ | https://www.dcsaasports.org/ |
| `samples/mpssaa-school-directory.html` | 200 | 528894 | 2026-09-22T03:56:21Z | https://www.mpssaa.org/school-directory/ | https://www.mpssaa.org/school-directory/ |
| `samples/nhiaa-member-directory.html` | 200 | 373904 | 2026-09-22T03:56:24Z | https://www.nhiaa.org/member-directory/ | https://www.nhiaa.org/member-directory/ |
| `samples/miaa-member-schools.html` | 200 | 68876 | 2026-09-22T03:56:25Z | https://www.miaa.net/about-miaa/miaa-member-schools | https://www.miaa.net/about-miaa/miaa-member-schools |
| `samples/mpa-school-list.aspx` | 200 | 100625 | 2026-09-22T03:56:26Z | https://www.mpa.cc/SchoolPages/School.aspx | https://www.mpa.cc/SchoolPages/School.aspx |
| `samples/riil-school-list.aspx` | 200 | 109578 | 2026-09-22T03:56:26Z | https://www.riil.org/SchoolPages/School.aspx | https://www.riil.org/SchoolPages/School.aspx |
| `samples/ciac-school-list.aspx` | 200 | 208855 | 2026-09-22T03:56:27Z | https://ciac.fpsports.org/SchoolPages/School.aspx | https://ciac.fpsports.org/SchoolPages/School.aspx |
| `samples/vpa-scoreboard.html` | 200 | 128158 | 2026-09-22T03:56:27Z | https://vpaonline.org/athletics/scoreboard/ | https://vpaonline.org/athletics/scoreboard/ |
| `samples/mpssaa-xc-region-schedule.html` | 200 | 230434 | 2026-09-22T03:56:28Z | https://www.mpssaa.org/cross-country-region-meet-schedule/ | https://www.mpssaa.org/cross-country-region-meet-schedule/ |
| `samples/njsiaa-schools.html` | 200 | 166905 | 2026-09-22T03:56:34Z | https://www.njsiaa.org/schools | https://www.njsiaa.org/schools |
| `samples/piaa-sitemap.aspx.html` | 200 | 66969 | 2026-09-22T03:56:34Z | https://www.piaa.org/sitemap.aspx | https://www.piaa.org/sitemap.aspx |
| `samples/njsiaa-sports.html` | 200 | 188142 | 2026-09-22T03:56:35Z | https://www.njsiaa.org/sports | https://www.njsiaa.org/sports |
| `samples/piaa-school-directory-a.html` | 200 | 75791 | 2026-09-22T03:56:48Z | https://www.piaa.org/schools/directory/list.aspx?alpha=A | http://www.piaa.org/schools/directory/list.aspx?alpha=A |
| `samples/njsiaa-member-info.html` | 200 | 238063 | 2026-09-22T03:56:49Z | https://www.njsiaa.org/schools/member-information | https://www.njsiaa.org/schools/member-information |
| `samples/piaa-membership.html` | 200 | 53487 | 2026-09-22T03:56:49Z | https://www.piaa.org/schools/membership/default.aspx | https://www.piaa.org/schools/membership/default.aspx |
| `samples/njsiaa-tf-outdoor.html` | 200 | 179986 | 2026-09-22T03:56:50Z | https://www.njsiaa.org/sports/track-field-outdoor | https://www.njsiaa.org/sports/track-field-outdoor |
| `samples/nhiaa-member-directory-p42.html` | 200 | 362449 | 2026-09-22T03:56:53Z | https://www.nhiaa.org/member-directory/page/42/ | https://www.nhiaa.org/member-directory/page/42/ |
| `samples/nysphsaa-xc.html` | 200 | 159202 | 2026-09-22T03:56:54Z | https://www.nysphsaa.org/index.aspx?path=cross | https://nysphsaa.org/sports/cross |
| `samples/dcsaasports-school-directory.html` | 200 | 301157 | 2026-09-22T03:56:55Z | https://www.dcsaasports.com/school-directory/ | https://www.dcsaasports.com/school-directory/ |
| `samples/vpa-tournament-pairings.html` | 200 | 146204 | 2026-09-22T03:56:55Z | https://vpaonline.org/athletics/tournament-pairings/ | https://vpaonline.org/athletics/tournament-pairings/ |
| `samples/mpssaa-members.html` | 200 | 222246 | 2026-09-22T03:56:56Z | https://www.mpssaa.org/members/ | https://www.mpssaa.org/login/?redirect_to=https%3A%2F%2Fwww.mpssaa.org%2Fmembers%2F |
| `samples/piaa-xc-championships.html` | 200 | 62792 | 2026-09-22T03:57:27Z | https://www.piaa.org/sports/crscountry/championships/default.aspx | https://www.piaa.org/sports/championship_details.aspx?sport=crscountry |
| `samples/mpssaa-fall-championships.html` | 200 | 246138 | 2026-09-22T03:57:28Z | https://www.mpssaa.org/state-championships/fall-championships/ | https://www.mpssaa.org/ |
| `samples/nhiaa-boys-xc.html` | 200 | 309908 | 2026-09-22T03:57:29Z | https://www.nhiaa.org/sports/fall/boys-cross-country/ | https://www.nhiaa.org/sports/fall/boys-cross-country/ |
| `samples/miaa-track-xc.html` | 200 | 112173 | 2026-09-22T03:57:30Z | https://www.miaa.net/track-cross-country | https://www.miaa.net/track-cross-country |
| `samples/de-milesplit-home.html` | 200 | 108823 | 2026-09-22T03:57:31Z | https://de.milesplit.com/ | https://de.milesplit.com/ |
| `samples/delaware-regulations-robots.txt` | 200 | 65540 | 2026-09-22T03:57:31Z | https://regulations.delaware.gov/robots.txt | https://regulations.delaware.gov/robots.txt |
| `samples/njsiaa-xc.html` | 200 | 173783 | 2026-09-22T03:57:31Z | https://www.njsiaa.org/sports/cross-country | https://www.njsiaa.org/sports/cross-country |
| `samples/vpa-tournaments.html` | 200 | 140586 | 2026-09-22T03:57:31Z | https://vpaonline.org/athletics/tournaments/ | https://vpaonline.org/athletics/tournaments/ |
| `samples/ciac-xc-boys.aspx` | 200 | 73655 | 2026-09-22T03:57:43Z | https://ciac.fpsports.org/SportPages/SportPageInfo.aspx?TournamentID=1 | https://ciac.fpsports.org/SportPages/SportPageInfo.aspx?TournamentID=1 |
| `samples/mpa-xc-boys.aspx` | 200 | 68082 | 2026-09-22T03:57:44Z | https://www.mpa.cc/SportPages/SportPageInfo.aspx?TournamentID=1 | https://www.mpa.cc/SportPages/SportPageInfo.aspx?TournamentID=1 |
| `samples/riil-xc-boys.aspx` | 200 | 96285 | 2026-09-22T03:57:44Z | https://www.riil.org/SportPages/SportPageInfo.aspx?TournamentID=1 | https://www.riil.org/SportPages/SportPageInfo.aspx?TournamentID=1 |
| `samples/ciac-outdoor-track.aspx` | 200 | 59199 | 2026-09-22T03:57:47Z | https://ciac.fpsports.org/SportPages/SportPageInfo.aspx?TournamentID=204 | https://ciac.fpsports.org/SportPages/SportPageInfo.aspx?TournamentID=204 |
| `samples/mpa-outdoor-track.aspx` | 200 | 53994 | 2026-09-22T03:57:47Z | https://www.mpa.cc/SportPages/SportPageInfo.aspx?TournamentID=204 | https://www.mpa.cc/SportPages/SportPageInfo.aspx?TournamentID=204 |
| `samples/nysphsaa-otrack.html` | 200 | 154537 | 2026-09-22T03:58:06Z | https://nysphsaa.org/sports/otrack | https://nysphsaa.org/sports/otrack |
| `samples/psal-xc.html` | 200 | 322944 | 2026-09-22T03:58:07Z | https://www.psal.org/sports/sport.aspx?spCode=033&flag=All | https://www.psal.org/sports/sport.aspx?spCode=033&flag=All |
| `samples/psal-outdoor-track.html` | 200 | 390802 | 2026-09-22T03:58:08Z | https://www.psal.org/sports/sport.aspx?spCode=030&flag=All | https://www.psal.org/sports/sport.aspx?spCode=030&flag=All |
| `samples/miaa-scores.html` | 200 | 67366 | 2026-09-22T03:58:09Z | https://www.miaa.net/scores | https://www.miaa.net/scores |
| `samples/dcsaasports-about.html` | 404 | 218245 | 2026-09-22T03:58:10Z | https://www.dcsaasports.com/about-us/ | https://www.dcsaasports.com/about-us/ |
| `samples/nysphsaa-schools.html` | 404 | 166771 | 2026-09-22T03:58:48Z | https://www.nysphsaa.org/schools | https://nysphsaa.org/404-1.aspx?url=/schools |
| `samples/vpa-divisional-alignments.html` | 200 | 123539 | 2026-09-22T03:58:48Z | https://vpaonline.org/athletics/divisional-alignments/ | https://vpaonline.org/athletics/divisional-alignments/ |
| `samples/delaware-admincode-1001.shtml` | 200 | 65540 | 2026-09-22T03:58:49Z | https://regulations.delaware.gov/AdminCode/title14/1000/1001.shtml | https://regulations.delaware.gov/AdminCode/title14/1000/1001.shtml |
| `samples/nhiaa-about-schools.html` | 200 | 294581 | 2026-09-22T03:58:49Z | https://www.nhiaa.org/about-nhiaa/schools/ | https://www.nhiaa.org/about-nhiaa/schools/ |
| `samples/piaa-school-directory.html` | 200 | 55149 | 2026-09-22T03:58:49Z | https://www.piaa.org/schools/directory/default.aspx | https://www.piaa.org/schools/directory/default.aspx |
| `samples/miaa-school-list.pdf` | 200 | 123825 | 2026-09-22T03:58:55Z | https://www.miaa.net/media/824 | https://www.miaa.net/sites/default/files/2024-05/miaa-member-school-list.pdf |
| `samples/delaware-admincode-title14.html` | 200 | 65540 | 2026-09-22T03:59:11Z | https://regulations.delaware.gov/AdminCode/title14/ | https://regulations.delaware.gov/AdminCode/title14/ |
| `samples/piaa-championship-details-xc.html` | 200 | 62779 | 2026-09-22T03:59:19Z | https://www.piaa.org/sports/championship_details.aspx?sport=crscountry | https://www.piaa.org/sports/championship_details.aspx?sport=crscountry |
| `samples/mpssaa-cross-country.html` | 200 | 239184 | 2026-09-22T03:59:20Z | https://www.mpssaa.org/cross-country/ | https://www.mpssaa.org/sports/cross-country/ |
| `samples/nysphsaa-itrack.html` | 200 | 155576 | 2026-09-22T03:59:21Z | https://nysphsaa.org/sports/itrack | https://nysphsaa.org/sports/itrack |
| `samples/psal-profile.html` | 200 | 45175 | 2026-09-22T03:59:21Z | https://www.psal.org/profiles/profile.aspx | https://www.psal.org/profiles/profile.aspx |
| `samples/mpssaa-xc-2025-results.html` | 200 | 228829 | 2026-09-22T03:59:41Z | https://www.mpssaa.org/2026/cross-country/2025-championship-results/ | https://www.mpssaa.org/2026/cross-country/2025-championship-results/ |
| `samples/dcsaasports-boys-xc.html` | 200 | 325955 | 2026-09-22T03:59:43Z | https://www.dcsaasports.com/boys-cross-country/ | https://www.dcsaasports.com/boys-cross-country/ |
| `samples/dcsaasports-otf.html` | 200 | 329074 | 2026-09-22T03:59:46Z | https://www.dcsaasports.com/sports/outdoor-track-field/ | https://www.dcsaasports.com/sports/outdoor-track-field/ |
| `samples/newengland-milesplit-teams.html` | 500 | 0 | 2026-09-22T03:59:46Z | https://newengland.milesplit.com/teams | https://newengland.milesplit.com/teams |
| `samples/dcmdva-milesplit-teams.html` | 500 | 0 | 2026-09-22T03:59:47Z | https://dcmdva.milesplit.com/teams | https://dcmdva.milesplit.com/teams |
| `samples/robots-dcsaasports.txt` | 404 | 13 | 2026-09-22T04:00:14Z | https://www.dcsaasports.com/robots.txt | https://www.dcsaasports.com/robots.txt |
| `samples/mpssaa-ad-directory.html` | 200 | 222316 | 2026-09-22T04:00:16Z | https://www.mpssaa.org/members/athletic-director-email-directory/ | https://www.mpssaa.org/login/?redirect_to=https%3A%2F%2Fwww.mpssaa.org%2Fmembers%2Fathletic-director-email-directory%2F |
| `samples/nysphsaa-schools-path.html` | 200 | 154879 | 2026-09-22T04:00:16Z | https://www.nysphsaa.org/index.aspx?path=schools | https://nysphsaa.org/index.aspx?path=schools |

### Transport errors (no file written)

| attempted URL | error | UTC |
|---|---|---|
| https://www.ciacsports.com/robots.txt | URLError: <urlopen error [SSL: CERTIFICATE_VERIFY_FAILED] certificate verify failed: unable to get local issuer certificate (_ssl.c:1082)> | 2026-09-22T03:54:27Z |
| https://www.mpa.ccsso.org/robots.txt | URLError: <urlopen error [Errno -2] Name or service not known> | 2026-09-22T03:54:29Z |

### Pre-existing captures in this directory (not fetched by this lane)

These six files were already present when this lane started; metadata read from each file's own header block (first line: `<url>\tcaptured <date>\trobots.txt verbatim`).

| file | bytes | header URL |
|---|---|---|
| `samples/robots-education-delaware.txt` | 311 | https://education.delaware.gov/robots.txt |
| `samples/robots-mpssaa.txt` | 352 | https://www.mpssaa.org/robots.txt |
| `samples/robots-njsiaa.txt` | 2183 | https://www.njsiaa.org/robots.txt |
| `samples/robots-piaa.txt` | 593 | https://www.piaa.org/robots.txt |
| `samples/robots-va-milesplit.txt` | 333 | https://va.milesplit.com/robots.txt |
| `samples/robots-vhsl.txt` | 252 | https://www.vhsl.org/robots.txt |

### Command-only evidence (no captured file)

| command | observed output | UTC |
|---|---|---|
| `curl -sS -o /dev/null -w 'ciacsports.com: code=%{http_code} ssl_verify=%{ssl_verify_result} eff=%{url_effective}\n' --max-time 20 -A '<chrome-UA> athletic-census-research/1.0' https://www.ciacsports.com/` | `curl: (60) SSL certificate OpenSSL verify result: unable to get local issuer certificate (20)`; `ciacsports.com: code=000 ssl_verify=20 eff=https://www.ciacsports.com/` | 2026-09-22 |
| `timeout 20 openssl s_client -connect www.ciacsports.com:443 -servername www.ciacsports.com -showcerts </dev/null` | `verify error:num=20:unable to get local issuer certificate`, `verify error:num=10:certificate has expired`, leaf `s:CN=ciacsports.com i:C=US, O=Let's Encrypt, CN=R3`, chain root `i:O=Digital Signature Trust Co., CN=DST Root CA X3`, `Verify return code: 10 (certificate has expired)` | 2026-09-22 |
| `curl -sS -o /dev/null -w 'code=%{http_code} eff=%{url_effective}\n' --max-time 20 http://www.ciacsports.com/` | `code=302`, effective `http://www.ciacsports.com/` | 2026-09-22 |
| `curl -sS -o /dev/null -D /tmp/diaa-hdr.txt -w 'code=%{http_code} eff=%{url_effective} redirect=%{redirect_url}\n' --max-time 25 https://www.diaa.org/` | `code=302 redirect=https://education.delaware.gov/diaa`; headers include `location: https://education.delaware.gov/diaa`, `server: ip-10-123-124-92.ec2.internal` | 2026-09-22 |
| `curl -sS -o /dev/null -w 'code=%{http_code} bytes=%{size_download} type=%{content_type}\n' --max-time 25 -A '<chrome-UA> athletic-census-research/1.0' https://education.delaware.gov/diaa` (also `/diaa/` and `/`) | `code=403 bytes=5659/5662/5626 type=text/html; charset=UTF-8` — bodies are Cloudflare `Just a moment...` interstitial | 2026-09-22 |
| `getent hosts <candidate>` for 15 association/state candidate hosts | MPA/DIAA/DCSAA host discovery; see `coverage.json` → `host_probes` | 2026-09-22 |

### Non-HTTP extraction commands (local, no requests)

| purpose | command | result |
|---|---|---|
| MPSSAA school count | `grep -oE 'school-directory/([a-z0-9-]+)/' samples/mpssaa-school-directory.html \| sort -u` | 201 slugs, 9 county aggregates, 192 schools |
| MIAA member count | `pdftotext -layout samples/miaa-school-list.pdf -` then `grep -c 'MIAA District:'` | 381 |
| CIAC/MPA/RIIL school count | `grep -oE 'SchoolID=(\d+)' samples/{ciac,mpa,riil}-school-list.aspx \| sort -u` | 440 / 152 / 128 |
| DCSAA school count | `grep -oE 'dcsaasports.com/school-directory/([a-z0-9-]+)/' samples/dcsaasports-school-directory.html \| sort -u` | 45 |
| MileSplit per-state team links | `grep -oE 'https://([a-z]+)\.milesplit\.com/teams/(\d+)-' samples/*-milesplit-teams.html \| sort -u` | see `coverage.json` → `milesplit_state_sites` |
| robots identity | `md5sum samples/mpa-robots.txt samples/robots-riil.txt samples/ciac-fpsports-robots.txt` | all `901da3fd0f5f4be7f447ace46cdab356` |
