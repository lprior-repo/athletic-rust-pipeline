

<!DOCTYPE html>

<html lang="en">
<head><meta name='description' content='The Connecticut Association of Schools and the Connecticut Interscholastic Athletic Conference (CIAC) is the governing body for secondary school athletics and other interscholastic competition in the U.S. state of Connecticut'/><link href="/_Styles/1000/Config.css?v=20260921c" rel="stylesheet" type="text/css" /><link href="/_Styles/Controls.css?v=20260921c" rel="stylesheet" type="text/css" /><link href="/_Styles/Dashboard.css?v=20260921c" rel="stylesheet" type="text/css" /><link href="/_Styles/TournamentCentral.css?v=20260921c" rel="stylesheet" type="text/css" /><meta charset="utf-8" /><title>
	FusionPoint Sports - CIAC
</title><meta http-equiv="Content-Type" content="text/html; charset=utf-8" /><meta id="Viewport" name="viewport" content="width=device-width, initial-scale=1, minimal-ui" /><meta name="mobile-web-app-capable" content="yes" /><meta name="apple-mobile-web-app-capable" content="yes" /><meta http-equiv="Cache-control" content="max-age=300" /><link id="SiteCss" rel="stylesheet" type="text/css" href="/_Styles/Site.css?v=20260921c" /><link rel="stylesheet" type="text/css" href="/_Styles/parsley.css" /><link rel="stylesheet" type="text/css" href="/_Styles/jquery-ui.css" /><link rel="stylesheet" type="text/css" href="/_Fonts/Awesome/css/fontawesome.css" /><link rel="stylesheet" type="text/css" href="/_Fonts/Awesome/css/regular.css" /><link rel="stylesheet" type="text/css" href="/_Fonts/Awesome/css/solid.css" /><link rel="stylesheet" type="text/css" href="../_Fonts/Awesome/css/duotone.css" /><link href="../favicon.ico" rel="shortcut icon" type="image/x-icon" />

    <script src="https://code.jquery.com/jquery-3.7.1.min.js" integrity="sha256-/JqT3SQfawRcv/BIHPThkBvs0OEvtFFmqPF/lYI/Cxo=" crossorigin="anonymous"></script>
    <script src="https://ajax.googleapis.com/ajax/libs/jqueryui/1.10.3/jquery-ui.min.js"></script>


    <script type="text/javascript" src="/_Scripts/Application.js"></script>
    <script type="text/javascript" src="/_Scripts/Controls.js"></script>
    <script type="text/javascript" src="https://cdn.fpsports.org/3rdParty/Parsley/v2.0.7/parsley.js"></script>
    <script type="text/javascript" src="https://cdn.fpsports.org/3rdParty/JLoadImage/js/load-image.all.min.js"></script>
    <script src="https://cdn.fpsports.org/3rdParty/slick/slick.js"></script>
    <link rel="stylesheet" href="https://cdn.fpsports.org/3rdParty/slick/slick.css" /><link rel="stylesheet" href="https://cdn.fpsports.org/3rdParty/slick/slick-theme.css" /><link rel="icon" sizes="196x196" href="/_Images/Logos/196x196.png" /><link rel="manifest" href="/manifest.json" /><link rel="mask-icon" href="/safari-pinned-tab.svg" color="#5bbad5" /><meta name="theme-color" content="#ffffff" />

    <!--jstree References-->
    <script type="text/javascript" src="https://cdn.fpsports.org/3rdParty/jsTree/V3.1/jstree.min.js"></script>
    <link rel="stylesheet" type="text/css" href="https://cdn.fpsports.org/3rdParty/jsTree/jstree.css" /><link href="../_Styles/fancy.css" rel="stylesheet" />
    <script src="https://cdn.fpsports.org/3rdParty/fancygrid/fp/fancy_v7.full.min.js"></script>

    <script type="text/javascript">
        //Prevent Links in Standalone Web Apps Opening Mobile Safari
        if (("standalone" in window.navigator) && window.navigator.standalone) {
            var noddy, remotes = false;
            document.addEventListener('click', function (event) {
                noddy = event.target;
                while (noddy.nodeName !== "A" && noddy.nodeName !== "HTML") {
                    noddy = noddy.parentNode;
                }
                if ('href' in noddy && noddy.href.indexOf('http') !== -1 && (noddy.href.indexOf(document.location.host) !== -1 || remotes)) {
                    event.preventDefault();
                    document.location.href = noddy.href;
                }
            }, false);
        }

        $(document).ready(function () {
            Fancy.MODULESDIR = 'https://cdn.fpsports.org/3rdParty/fancygrid/modules/';
            FancyGrid.LICENSE = ['f3e1cd90dfb49ba1b9d16d7c5abe7260'];

            FixCSS();

            bindToolbarMenuFunctions('SchoolMenu_');


        });

        $(window).on('load', function () { ProcessesLinkTracking(); });

        $(window).on("resize", function () {
            FixCSS();
        });

        function FixCSS() {
            $(":root").css("--toolbarheight", ($("#ToolbarPanel").outerHeight() ?? 0) + 'px');
            $(":root").css("--noticeheight", ($(".dsSiteNotice").outerHeight(true) ?? 0) + 'px');
        }


        function ProcessesLinkTracking() {
            var Items = [];

            $('.CarouselTracker:visible').each(function () {
                var CarouselTracker = $(this);
                Items.push({ CarouselID: $(this).attr('carouselid'), Carousel: $(this).attr('carousel'), CarouselEntryID: $(this).attr('carouselentryid'), CarouselEntry: $(this).attr('carouselentry') });
                $(this).find('a').each(function () { $(this).on('click', function () { ProcessLinkClick(CarouselTracker.attr('carouselid'), CarouselTracker.attr('carousel'), CarouselTracker.attr('carouselentryid'), CarouselTracker.attr('carouselentry')) }); });
            });

            var dto = {
                'Links': Items
            };

            $.ajax({
                type: 'post',
                contentType: 'application/json; charset-utf-8',
                dataType: 'json',
                url: '/Services/CoreServices.asmx/LogLinkImpression',
                data: JSON.stringify(dto),
                success: function (result) {
                }
            });
        }

        function ProcessLinkClick(CarouselID, Carousel, CarouselEntryID, CarouselEntry) {
            var dto = {
                'CarouselID': CarouselID,
                'Carousel': Carousel,
                'CarouselEntryID': CarouselEntryID,
                'CarouselEntry': CarouselEntry,
            };

            $.ajax({
                type: 'post',
                contentType: 'application/json; charset-utf-8',
                dataType: 'json',
                url: '/Services/CoreServices.asmx/LogLinkClick',
                data: JSON.stringify(dto),
                success: function (result) {
                }
            });
        }



    </script>

    <style>
        .LegacyItem {
            font-size: 9px;
            color: red;
        }

        .NewItem {
            font-size: 9px;
            color: red;
        }

        .dsNavigationMenuHidden {
            display: none;
        }

        #mnuTournamentCentralButton {
            background-color: #b12e34;
            color: white;
        }

    </style>



    
    <link id="ShellCss" rel="stylesheet" type="text/css" href="/_Styles/SiteShell.css?v=20260921c"></link>

    

    <style>
        .dsHeaderPanel {
            margin-bottom: 20px;
        }

        #mnuTournament {
            background-color: #b12e34;
            color: white;
        }

        .HeaderText {
            margin: 20px 0px 0px 0;
        }

        .SportTitle {
            text-align: center;
            font-weight: bold;
            text-transform: uppercase;
            background-color: var(--primary-dark-color);
            color: white;
            padding: 16px 0 16px 0 !important;
            font-size: 32px !important;
            display: block;
            width: 100%;
            margin: 0 0 10px 0;
            box-sizing: border-box;    
            line-height: 1;
        }

        
    </style>

    <script>

      

    </script>

    

    <style>
      

        .dashboard-content .dsFlexBlockFill img {
            width: 100%;
        }

        .dashboard-content h2 {
            /* background-color: #003366;*/
            text-align: center;
            font-weight: bold;
            text-transform: uppercase;
        }

       

        .SportTitle {
            /*background-color: #003366;*/
            /*color: white;*/
            padding: 16px 0 16px 0 !important;
            font-size: 32px !important;
        }

        .SportInfoTitle {
            font-weight: bold;
            margin-top: 4px;
        }

        .SportInfoIndent {
            padding-left: 10px;
            font-weight: bold;
        }

        .DashboardAds img {
            width: Calc(100% - 7px) !important;
            margin-bottom: 6px;
        }

        /*  .ImportantWrapper, .ImportantBlock, .ImportantBlockContent and the ul/li rules that were
            here are now in _Styles/Dashboard.css, which this page already loads. The school home
            page renders the same pair of blocks, and one shared definition beat a second copy.
            Only the sport-specific rules stay below.                                              */

            .ImportantBlock .SportInfoDates {
                color: black;
                padding: 8px 8px 20px 8px;
            }

                .ImportantBlock .SportInfoDates h2 {
                    background-color: transparent;
                    margin: 0;
                    color: black;
                    font-size: 14px;
                    text-align: left;
                    padding: 0 0px 4px 0px;
                }


        .SportInfoDetail {
            padding: 10px;
            min-height: 80px;
            background-color: transparent;
            color: white;
            text-shadow: -2px 0 2px #222222, 0 2px 2px #222222, 2px 0 2px #222222, 0 -2px 2px #222222;
            font-size: 14px;
        }

        .SportInfoLinks {
            font-size: 16px;
            margin-top: 10px;
            margin-right: 30px;
            margin-left: 20px;
            float: left;
            color: black;
        }

        .GameTickerWrapper {
            display: flex;
            gap: 25px;
            flex-wrap: wrap;
            justify-content: center;
        }

            .GameTickerWrapper a {
                flex: 0 0 30%;
                height: 170px;
                text-decoration: none;
                color: white;
                padding: 6px;
                background-color: white;
            }

                .GameTickerWrapper a:visited {
                    text-decoration: none;
                    color: white;
                }

        .GameTickerBlock {
            height: 100%;
            width: 100%;
            display: block;
            background-size: cover;
            background-attachment: local;
            background-position: center;
            background-color: white;
            color: black;
            text-align: center;
            padding: 0;
            position: relative;
        }



        .GameTickerDivision {
            font-size: 15px;
            font-weight: bold;
            text-transform: uppercase;
            margin-bottom: 8px;
            background-color: white;
            color: black;
        }

        .GameTickerTeam {
            width: Calc(50% - 15px);
        }

            .GameTickerTeam img {
                display: inline-block;
                vertical-align: middle;
                height: 65px;
                width: 65px;
                margin: 4px 0 0 0;
            }

        .GameSchoolName {
            height: 36px;
            overflow: hidden;
        }

        .GameTickerWinner {
            border: 2px solid black;
            background-color: white;
        }

        .GameTeamLeft {
            display: block;
            float: left;
        }

        .GameTeamRight {
            display: block;
            float: right;
        }

        .GameTickerScore {
            font-size: 18px;
            font-weight: bold;
            margin-top: 2px;
        }

        .GameTickerDetail {
            clear: both;
            text-align: left;
            line-height: 1;
            position: absolute;
            bottom: 8px;
            width: Calc(100% - 20px);
            overflow-x: hidden;
            white-space: nowrap;
            font-size: 11px;
            height: 12px;
        }

        .SponsorLogos {
            clear: both;
            background-color: white;
            margin-top: 10px;
            text-align: center;
        }

            .SponsorLogos img {
                max-height: 100px;
                margin: 20px;
            object-fit: contain;
            }

        .MaxPrepsStats {
            height: 300px;
            margin-bottom: 20px;
        }
              

        @media (max-width: 900px) {
            .DashboardAds {
                display: flex;
                flex-wrap: wrap;
            }

                .DashboardAds a {
                    flex: 0 0 50%;
                }

            .ImportantWrapper .dsFlexBlock50 {
                width: 100%;
            }

            .highlightedad {
                flex: 1 0 100% !important;
            }

            .SponsorLogos img {
                width: 100%;
                margin: 0px;
            }
        }

        @media (max-width: 1000px) {
            .GameTickerWrapper a {
                flex: 0 0 45%;
            }
        }

        @media (max-width: 700px) {
            .GameTickerWrapper {
                display: block;
            }

                .GameTickerWrapper a {
                    display: block;
                    width: 90%;
                    margin: 0 auto 15px auto;
                }
        }


        @page {
            size: auto; /* auto is the initial value */
            margin: 5mm 5mm 5mm 5mm;
        }

        @media print {
            body {
                background-image: none;
                background: none;
            }

            .dashboard-content h2 {
                background-color: white;
                color: black;
            }

            .dsFormPanelFSwTB {
                overflow: visible;
                color: black !important;
            }

            .dsPagePanel {
                padding-top: 0;
                color: black !important;
            }

            h1, h2, .dsTable, .dsTable td, .dsTable th, .dsTable a {
                color: #222222 !important;
            }

            .RankingNotes,
            .dsHeaderPanel,
            .dsToolbarPanel {
                display: none;
            }
        }
    </style>





</head>
<body>
    <iframe id="hidden-iframe" name="hidden-iframe" style="visibility: hidden; position: absolute;"></iframe>
    <form method="post" action="./SportPageInfo.aspx?TournamentID=204" id="MainForm">
<div class="aspNetHidden">
<input type="hidden" name="__VIEWSTATE" id="__VIEWSTATE" value="psM/y0gUZ72gBB0o0ffj8LfA5rEw+jJpN3MHnVIJRmQyBkTP8PkNcH/hp6N2+wwWpaFC37DPcWHx9YsyQcJBVycLyqdAUZrMzAX23u8UNWJLL31+apKCj8bcMsDdwasH1hUTUyVtuhlERVcTfutm8NyG07lfomhwwpQD6MFegUB9K8kCYM4T/hQf5czyDIpLKdJrzfz8wqmtRoJx8IAXRVjspd0Y0aVZonpwQJWjk7sy6Vyl1wU2HxD5Du6gTIViRCIkeEywJOx06mWbp6sOfAX7ZWJ1xtbrc8rybf0ejiz4X2GNmsPirtALgRIlsoZw7REWSAfWM3SpzLgMXDoB6eaJwh+Oq8XQ8g313QH08/alqpv2nY+6Omj5aksmoijz/43tAhz7+9rqus4oTSzeGMryR4RptY2gzzSKaY+YlFFJWJTeKvFTp3yLJWJ5Am4gvMJbr6OamQO1GT2NQGQ9lxCQJktQLEuH6t74jYR58z1zfeyeY3lmjKH9NH8LnBofwQyg3fud3IuuiV9biwZ2Hm0FTKx3FrUbcZR2Hpd9RJ4bL8VpLiREiqFA8VZzSGCQYie+02tR7p/DCMGddX0zU4FFdcESe5NU6wau8/YpC8tRrA/MSCCoe+ZGS3iTUhbfVpaHTHWnzNOE8jWQUb+H/7ptic9NapefXmsbGmGWCfzi8WHPPfyzvWBnsqRMXLOLn+y2ZSVRU+FWJohnwOnh8f7e+LWGm1TwVuHZDExSUgijVlh9N6T8DFXlCcyfS8A2ozW+LSBhr0z5ztMO5dv++dpzksnytPAeUxWClffzTlAmtuvDxyS06ldYcSmZ0166QFJfhEn5uAEZIJmUdfAQJb9O3z+OpjpVQlvmqc+i9SX0hxGGDeEQr7amz1XQwbEP24k4GMqviqS6F7aPp2V1C1moCZNyCag1MRAJXyvLG4tPYxWDtbN/+PCJWhd8nunjRG4Xqa0MPo2fz4cYoNnBsY9P98F9CxhXPUw0Pyzp9RQP6b0ShfqjDHhduS6/thV65bqFnxGynbe1QI5jd2OiXdIYF3aCVfUmfvvjq1EYtMczjjqM4gjOTB7gWlOUCCBQI1eTVMWiNSAF4/9hqvrBu/AFeTmuRs6OF4l0+3Dt1DGFFX7HBhnkTibFlF2S58ij3zpeVoHhO1Bet+4qi2yaaKKrzB5cNXZuACEG/5MxukNchXZc6GtwQ9AEt8axQQ5wZoURH/Vm+UzVDiGHpiuQ1e4zdb4XjcGNGNClNUkzdVD30xv9SdhYSws56cnSxJOI7EILpfgKOTYTcDlaAdr1hn3vuFQOsumRq/eHL9Iqdlyzss7tIpO5vNDl3PUy+hdNsXGWjT8YOSHFLEnt0Gc48AxkfSUrK9CR/gf4ONmoafFi66rNfIy+2egYZp5K8qhGItFS6EQpVEc5/EH4P0MLmU912ZObFGYKndagOwCqhhUFu0NaFgbHzyCl0T44wQ295BFARQlKSdT3BXL8IC0v3Q==" />
</div>

<div class="aspNetHidden">

	<input type="hidden" name="__VIEWSTATEGENERATOR" id="__VIEWSTATEGENERATOR" value="A370CA34" />
</div>
        <div id="dsLoader"></div>

        <div id="dsNavigation" class="dsNavigationOverlay">
	
            <div id="dsNavigationMenu" class="dsNavigationMenu">
                <a href="javascript:void(0)" class="dsNavigationCloseButton" onclick="closeNav()">×</a>
                <div id="dsNavigationMenuContent" class="dsNavigationMenuContent">
		
                    <ul id="dsNavigationMenuUL" class="dsNavigationMenuUL dsNavigationMenuSubPanel active"><li><a id="mnuDashboard" href="\">Dashboard</a></li><li><a id="mnuSport" class=" dsNavigationMenuLink " href="https://ciac.fpsports.org/SportPages/SportPageInfo.aspx?TournamentID=204#NavigationMenu_mnuSport"><div style="float:left;">Sport Specific Info</div><i class="fa fa-chevron-right" style="float:right;"></i><div style="clear:both;"></div></a></li><li><a id="mnuScheduleHS" href="\DashboardSchedule.aspx">Schedules</a></li><li><a id="mnuSubResources_c527d1d0-9168-430c-b6bc-18068995f94e" href="/TenantHTML.aspx?D=&amp;F=Unified Sports.html">Unified Sports</a></li><li><a id="mnuSchoolSchedule" href="/DashboardTeamSchedule.aspx">Team Schedule</a></li><li><a id="mnuSchoolRoster" href="/DashboardTeamRoster.aspx?SeasonRoster=1">Team Rosters</a></li><li><a id="mnuSchoolPages" href="/SchoolPages/School.aspx">School Pages</a></li><li><a id="mnuMasterSchedule" href="/MasterSchedule.aspx?TeamLevelID=5">Master Schedule</a></li><li><a id="mnuTournamentCentralButton" href="\DashboardTournamentCentral.aspx">Tournament Central</a></li><li><a id="mnuTournamentCentralPastChamps" href="\Reports\PastChampionsBySport.aspx">Past Champions</a></li><li><a id="mnuEligibilityHistoricRecords" href="\Reports\HistoricRecords.aspx">Historic Records</a></li><li><a id="mnuEligibilityHistoricTeamSchedule" href="\Reports\HistoricTeamSchedule.aspx">Historic Schedules</a></li><li class="dsNavigationMenuGroupSeperator"></li><li><div class="dsNavigationMenuGroupTitle">Important Links</div></li><li><a id="mnuSubResources_3d3542a1-5d38-42c5-9ba6-c21dae8e3063" href="http://cas.casciac.org/" target="_blank">CT Association of Schools (CAS)</a></li><li><a id="mnuSubResources_4d67a602-89b4-42a0-afeb-7c9871e175fd" href="/TenantHTML.aspx?D=Important Links&amp;F=FAQs.html">FAQs</a></li><li><a id="mnuSubResources_69cff5cd-46e8-48d8-baf8-0eaf11ec12a2" href="/forms/post.aspx?posttypeid=1">Games Wanted</a></li><li><a id="mnuSubResources_694fd047-090a-49a8-9fda-77c1cd78d192" href="https://www.nfhsnetwork.com/associations/ciac" target="_blank">NFHS Network</a></li><li><a id="mnuSubResources_465281a4-5ab8-4143-998a-e0285e4e6ac4" href="https://gofan.co/app/school/CIAC" target="_blank">Tickets</a></li><li><a id="mnuSubResources_2b1537c6-fde6-429e-ba5b-2016758e70a8" href="/forms/post.aspx?posttypeid=2">Vacancies</a></li><li class="dsNavigationMenuGroupSeperator"></li><li><div class="dsNavigationMenuGroupTitle">Resources</div></li><li><a id="mnuSubResources_Resources_Administrators" class=" dsNavigationMenuLink " href="https://ciac.fpsports.org/SportPages/SportPageInfo.aspx?TournamentID=204#NavigationMenu_mnuSubResources_Resources_Administrators"><div style="float:left;">Administrators</div><i class="fa fa-chevron-right" style="float:right;"></i><div style="clear:both;"></div></a></li><li><a id="mnuSubResources_Resources_Officials" class=" dsNavigationMenuLink " href="https://ciac.fpsports.org/SportPages/SportPageInfo.aspx?TournamentID=204#NavigationMenu_mnuSubResources_Resources_Officials"><div style="float:left;">Officials</div><i class="fa fa-chevron-right" style="float:right;"></i><div style="clear:both;"></div></a></li><li><a id="mnuSubResources_Resources_Students__Parents" class=" dsNavigationMenuLink " href="https://ciac.fpsports.org/SportPages/SportPageInfo.aspx?TournamentID=204#NavigationMenu_mnuSubResources_Resources_Students__Parents"><div style="float:left;">Students/Parents</div><i class="fa fa-chevron-right" style="float:right;"></i><div style="clear:both;"></div></a></li><li><a id="mnuSubResources_ab73feee-af14-4319-9f82-7ef1b8c37fcb" href="/TenantHTML.aspx?D=Resources&amp;F=Class Act Schools.html">Class Act Schools</a></li><li><a id="mnuSubResources_889fee9b-12ed-4ac0-bd73-374ef899d63c" href="/TenantHTML.aspx?D=Resources&amp;F=Coaches Spotlight.html">Coaches Spotlight</a></li><li><a id="mnuSubResources_c226dd5e-d512-4506-9f25-f0468b0a34de" href="http://www.ctcoachinged.org/">Coaching Education</a></li><li><a id="mnuSubResources_6c0012f2-026b-4550-9d20-182c40435de4" href="https://sites.google.com/casciac.org/ciac-education-based-athletics?usp=sharing" target="_blank">Education Based Athletics</a></li><li><a id="mnuSubResources_3314c388-7ee1-41f7-bd1e-6af656de0bc4" href="/TenantHTML.aspx?D=Resources&amp;F=Esports.html">Esports</a></li><li><a id="mnuSubResources_a97e95c1-a395-4f4d-84eb-f39e7f62e66c" href="http://cas.casciac.org/?page_id=961" target="_blank">Event Registration</a></li><li><a id="mnuSubResources_92679bb2-7366-4afa-aca3-26f31ab84408" href="/TenantHTML.aspx?D=Resources&amp;F=Glory Days Podcast.html">Glory Days Podcast</a></li><li><a id="mnuSubResources_8d423f2a-b946-4ce1-aae8-ce04fe8c9097" href="/resources/Resources/Handbook.pdf" target="_blank">Handbook</a></li><li><a id="mnuSubResources_e6d03544-e583-4664-bfc4-02fd274cf7a6" href="/TenantHTML.aspx?D=Resources&amp;F=IKEA Spotlight.html">IKEA Spotlight</a></li><li><a id="mnuSubResources_5f522754-cab7-422a-be75-4ac391351daa" href="/TenantHTML.aspx?D=Resources&amp;F=Leagues.html">Leagues</a></li><li><a id="mnuSubResources_9279bf6a-27a3-4cc3-a0cd-75170d5bf6ab" href="/resources/Resources/Medical%20Handbook%202026-2027.pdf" target="_blank">Medical Handbook 2026-2027</a></li><li><a id="mnuSubResources_ea4f59b8-aaab-4a40-b3c5-a18e8dbc3ce8" href="/TenantHTML.aspx?D=Resources&amp;F=Performing Arts Spotlight.html">Performing Arts Spotlight</a></li><li><a id="mnuSubResources_19511d75-00e1-400c-bbc7-f0a2d6707425" href="/TenantHTML.aspx?D=Resources&amp;F=Sanctioned Events.html">Sanctioned Events</a></li><li><a id="mnuSubResources_81010115-0f98-4eda-ac1c-26cbc3aca041" href="/TenantHTML.aspx?D=Resources&amp;F=SERVPRO Spotlight.html">SERVPRO Spotlight</a></li><li><a id="mnuSubResources_d08255c7-c5cb-41c3-8f14-905535bdd846" href="/resources/Resources/Website%20Privacy%20Policy.pdf" target="_blank">Website Privacy Policy</a></li><li><a id="mnuSubResources_18406ba5-231a-4d25-b1e7-5c5f9fb15191" href="/resources/Resources/Website%20Terms.pdf" target="_blank">Website Terms</a></li><li class="dsNavigationMenuGroupSeperator"></li><li><div class="dsNavigationMenuGroupTitle">About CIAC</div></li><li><a id="mnuSubResources_ca9a5c64-8dcd-4ce0-ad39-bb716130e919" href="/TenantHTML.aspx?D=About CIAC&amp;F=History.html">History</a></li><li><a id="mnuSubResources_085eccd2-0ebd-44e7-b76b-126636fd4b5a" href="/TenantHTML.aspx?D=About CIAC&amp;F=Contact Info.html">Contact Info</a></li><li><a id="mnuSubResources_03f66c8d-12d2-4ed9-ad82-6d46e17b8601" href="/TenantHTML.aspx?D=About CIAC&amp;F=Staff.html">Staff</a></li><li><a id="mnuSubResources_a3e6a03f-45e4-4cd5-bb38-42762ab4ed01" href="/TenantHTML.aspx?D=About CIAC&amp;F=Related Organizations.html">Related Organizations</a></li><li><a id="mnuSubResources_fb728295-4ae1-4f99-b9ad-555fe49f274b" href="/CommitteeDirectory.aspx">Boards and Committees</a></li><li class="dsNavigationMenuGroupSeperator"></li><li><div class="dsNavigationMenuGroupTitle">Account</div></li><li><a id="mnuLogin" href="../Login.aspx?ReturnUrl=%2fSportPages%2fSportPageInfo.aspx%3fTournamentID%3d204">Login</a></li><li><a id="mnuSignUp" href="../SignUp.aspx?ReturnUrl=%2fSportPages%2fSportPageInfo.aspx%3fTournamentID%3d204">Sign-Up</a></li><li><a id="mnuPasswordReset" href="\PasswordReset.aspx">Forgot Password</a></li></ul>
                    <div style="display: none">
                        <ul id="dsNavigationMenuHidden" class="dsNavigationMenuUL dsNavigationMenuSubPane"><li><a id="mnuTournament" class=" dsNavigationMenuLink " href="https://ciac.fpsports.org/SportPages/SportPageInfo.aspx?TournamentID=204#NavigationMenu_mnuTournament"><div style="float:left;">Tournament Central</div><i class="fa fa-chevron-right" style="float:right;"></i><div style="clear:both;"></div></a></li></ul>
                    </div>
                <ul id="NavigationMenu_mnuSport" class="dsNavigationMenuUL dsNavigationMenuSubPanel"><li><div class="dsNavigationSubMenuTitle">Sport Specific Info</div></li><li><a id="SUB_MAIN_mnuSport" class=" dsNavigationMenuLink " onclick="changeToMainMenu();"><i class="fa fa-arrow-left" style="float:left;"></i><div style="float:left;" class="dsNavigationMenuBackButton">MAIN MENU</div><div style="clear:both;"></div></a></li><li class="dsNavigationMenuGroupSeperator"></li><li><div class="dsNavigationMenuGroupTitle">Fall</div></li><li><a id="mnuSportButtonCross Country (Boys)" href="\SportPages\SportPageInfo.aspx?TournamentID=1">Cross Country (Boys)</a></li><li><a id="mnuSportButtonCross Country (Girls)" href="\SportPages\SportPageInfo.aspx?TournamentID=9">Cross Country (Girls)</a></li><li><a id="mnuSportButtonFall Golf (Boys)" href="\SportPages\SportPageInfo.aspx?TournamentID=4">Fall Golf (Boys)</a></li><li><a id="mnuSportButtonField Hockey" href="\SportPages\SportPageInfo.aspx?TournamentID=2">Field Hockey</a></li><li><a id="mnuSportButtonFootball" href="\SportPages\SportPageInfo.aspx?TournamentID=3">Football</a></li><li><a id="mnuSportButtonSoccer (Boys)" href="\SportPages\SportPageInfo.aspx?TournamentID=5">Soccer (Boys)</a></li><li><a id="mnuSportButtonSoccer (Girls)" href="\SportPages\SportPageInfo.aspx?TournamentID=6">Soccer (Girls)</a></li><li><a id="mnuSportButtonSwimming (Girls)" href="\SportPages\SportPageInfo.aspx?TournamentID=7">Swimming (Girls)</a></li><li><a id="mnuSportButtonVolleyball (Girls)" href="\SportPages\SportPageInfo.aspx?TournamentID=8">Volleyball (Girls)</a></li><li class="dsNavigationMenuGroupSeperator"></li><li><div class="dsNavigationMenuGroupTitle">Winter</div></li><li><a id="mnuSportButtonBoys Basketball" href="\SportPages\SportPageInfo.aspx?TournamentID=100">Boys Basketball</a></li><li><a id="mnuSportButtonGirls Basketball" href="\SportPages\SportPageInfo.aspx?TournamentID=101">Girls Basketball</a></li><li><a id="mnuSportButtonDance" href="\SportPages\SportPageInfo.aspx?TournamentID=1008">Dance</a></li><li><a id="mnuSportButtonGymnastics (Girls)" href="\SportPages\SportPageInfo.aspx?TournamentID=102">Gymnastics (Girls)</a></li><li><a id="mnuSportButtonIce Hockey (Boys)" href="\SportPages\SportPageInfo.aspx?TournamentID=103">Ice Hockey (Boys)</a></li><li><a id="mnuSportButtonIce Hockey (Girls)" href="\SportPages\SportPageInfo.aspx?TournamentID=108">Ice Hockey (Girls)</a></li><li><a id="mnuSportButtonIndoor Track (B&G)" href="\SportPages\SportPageInfo.aspx?TournamentID=104">Indoor Track (B&G)</a></li><li><a id="mnuSportButtonSwimming (Boys)" href="\SportPages\SportPageInfo.aspx?TournamentID=105">Swimming (Boys)</a></li><li><a id="mnuSportButtonCheer" href="\SportPages\SportPageInfo.aspx?TournamentID=107">Cheer</a></li><li><a id="mnuSportButtonWrestling" href="\SportPages\SportPageInfo.aspx?TournamentID=106">Wrestling</a></li><li class="dsNavigationMenuGroupSeperator"></li><li><div class="dsNavigationMenuGroupTitle">Spring</div></li><li><a id="mnuSportButtonBaseball" href="\SportPages\SportPageInfo.aspx?TournamentID=200">Baseball</a></li><li><a id="mnuSportButtonFlag Football (Girls)" href="\SportPages\SportPageInfo.aspx?TournamentID=1017">Flag Football (Girls)</a></li><li><a id="mnuSportButtonLacrosse (Boys)" href="\SportPages\SportPageInfo.aspx?TournamentID=202">Lacrosse (Boys)</a></li><li><a id="mnuSportButtonLacrosse (Girls)" href="\SportPages\SportPageInfo.aspx?TournamentID=203">Lacrosse (Girls)</a></li><li><a id="mnuSportButtonOutdoor Track (B&G)" href="\SportPages\SportPageInfo.aspx?TournamentID=204">Outdoor Track (B&G)</a></li><li><a id="mnuSportButtonSoftball" href="\SportPages\SportPageInfo.aspx?TournamentID=205">Softball</a></li><li><a id="mnuSportButtonGolf (Boys)" href="\SportPages\SportPageInfo.aspx?TournamentID=201">Golf (Boys)</a></li><li><a id="mnuSportButtonGolf (Girls)" href="\SportPages\SportPageInfo.aspx?TournamentID=209">Golf (Girls)</a></li><li><a id="mnuSportButtonTennis (Boys)" href="\SportPages\SportPageInfo.aspx?TournamentID=206">Tennis (Boys)</a></li><li><a id="mnuSportButtonTennis (Girls)" href="\SportPages\SportPageInfo.aspx?TournamentID=207">Tennis (Girls)</a></li><li><a id="mnuSportButtonVolleyball (Boys)" href="\SportPages\SportPageInfo.aspx?TournamentID=208">Volleyball (Boys)</a></li></ul><ul id="NavigationMenu_mnuTournament" class="dsNavigationMenuUL dsNavigationMenuSubPanel"><li><div class="dsNavigationSubMenuTitle">Tournament Central</div></li><li><a id="SUB_MAIN_mnuTournament" class=" dsNavigationMenuLink " onclick="changeToMainMenu();"><i class="fa fa-arrow-left" style="float:left;"></i><div style="float:left;" class="dsNavigationMenuBackButton">MAIN MENU</div><div style="clear:both;"></div></a></li><li><a id="mnuTournamentDashboard" href="\DashboardTournamentCentral.aspx">Dashboard</a></li><li><a id="mnuTournamentTodaysGames" href="\DashboardSchedule.aspx?QuickFilter=5">Tournament Games</a></li><li><a id="mnuTournamentRankings" href="\TournamentCentralRankings.aspx">Rankings</a></li><li><a id="mnuTournamentBrackets" href="\TournamentCentralBrackets.aspx">Brackets</a></li><li><a id="mnuTournamentTeamStats" href="\DashboardTeamRoster.aspx">Tournament Rosters</a></li><li><a id="mnuTournamentExpectations" href="\resources\_pdfs\CIAC%20Spectator%20Tournament%20Expectations.pdf">Spectator Expectations</a></li></ul><ul id="NavigationMenu_mnuSubResources_Resources_Administrators" class="dsNavigationMenuUL dsNavigationMenuSubPanel"><li><div class="dsNavigationSubMenuTitle">Administrators</div></li><li><a id="SUB_MAIN_mnuSubResources_Resources_Administrators" class=" dsNavigationMenuLink " onclick="changeToMainMenu();"><i class="fa fa-arrow-left" style="float:left;"></i><div style="float:left;" class="dsNavigationMenuBackButton">MAIN MENU</div><div style="clear:both;"></div></a></li><li><a id="mnuSubResources_debe0e6d-a092-4a8b-96f5-10e74df2b260" href="/TenantHTML.aspx?D=Resources\Administrators&amp;F=Athletic Program Evaluation.html">Athletic Program Evaluation</a></li><li><a id="mnuSubResources_991fb1b9-f4b7-4b6a-ac3f-69760d09f026" href="/TenantHTML.aspx?D=Resources\Administrators&amp;F=Coach__Team Evaluation Tools.html">Coach/Team Evaluation Tools</a></li><li><a id="mnuSubResources_4cc974f5-9326-42ec-a2ad-9b87d19419c5" href="/TenantHTML.aspx?D=Resources\Administrators&amp;F=Ford Athletic Grant.html">Ford Athletic Grant</a></li><li><a id="mnuSubResources_60e8d065-37d7-4ccf-ad87-e3b69bb21800" href="/TenantHTML.aspx?D=Resources\Administrators&amp;F=Link to CIAC.html">Link to CIAC</a></li><li><a id="mnuSubResources_2d096601-f743-47f1-b5a4-44dce752fe64" href="/TenantHTML.aspx?D=Resources\Administrators&amp;F=Login and Access Instructions.html">Login and Access Instructions</a></li><li><a id="mnuSubResources_eb130743-0ec1-474c-8311-ab4419c76e35" href="/TenantHTML.aspx?D=Resources\Administrators&amp;F=Michaels Achievement Cup.html">Michaels Achievement Cup</a></li><li><a id="mnuSubResources_c399785f-735d-47e7-b46a-df46a6a3b605" href="/resources/Resources\Administrators/Middle%20Level%20Guidelines.pdf" target="_blank">Middle Level Guidelines</a></li><li><a id="mnuSubResources_8be5c446-0df5-4f5d-9fcc-4206009b6b3a" href="/TenantHTML.aspx?D=Resources\Administrators&amp;F=Scholar-Athlete Banquet.html">Scholar-Athlete Banquet</a></li><li><a id="mnuSubResources_17735e2f-8386-4837-bf78-3d08786a327c" href="/TenantHTML.aspx?D=Resources\Administrators&amp;F=Spirit of Sport Award.html">Spirit of Sport Award</a></li><li><a id="mnuSubResources_df89f3d5-17d4-4807-80a2-89760708b942" href="/TenantHTML.aspx?D=Resources\Administrators&amp;F=Sportsmanship Conference.html">Sportsmanship Conference</a></li></ul><ul id="NavigationMenu_mnuSubResources_Resources_Officials" class="dsNavigationMenuUL dsNavigationMenuSubPanel"><li><div class="dsNavigationSubMenuTitle">Officials</div></li><li><a id="SUB_MAIN_mnuSubResources_Resources_Officials" class=" dsNavigationMenuLink " onclick="changeToMainMenu();"><i class="fa fa-arrow-left" style="float:left;"></i><div style="float:left;" class="dsNavigationMenuBackButton">MAIN MENU</div><div style="clear:both;"></div></a></li><li><a id="mnuSubResources_a365e4ba-b1c0-4f1e-bd7b-3969d8250170" href="/TenantHTML.aspx?D=Resources\Officials&amp;F=General Information.html">General Information</a></li><li><a id="mnuSubResources_29813acc-f2fa-446e-861c-4bf21d14e75d" href="/TenantHTML.aspx?D=Resources\Officials&amp;F=Cards__Ejections.html">Cards/Ejections</a></li><li><a id="mnuSubResources_715a26b0-3d44-4bc5-bc08-fa43af6d5996" href="/TenantHTML.aspx?D=Resources\Officials&amp;F=Downloads.html">Downloads</a></li><li><a id="mnuSubResources_08218405-8826-49bf-a997-654b0e570b1d" href="/Reports/OfficialsCard.aspx" target="_blank">Game Access Card</a></li><li><a id="mnuSubResources_c1797b75-904b-431f-a13c-52e561013cd2" href="/TenantHTML.aspx?D=Resources\Officials&amp;F=NFHS.html">NFHS</a></li><li><a id="mnuSubResources_c998772a-fef6-44ca-98b1-b0d4fe36f41f" href="/resources/Resources\Officials/NFHS%20Network%20Officials%20Pass.pdf" target="_blank">NFHS Network Officials Pass</a></li><li><a id="mnuSubResources_52f8d071-ce86-4d7a-8a95-ce62d99e0ef7" href="/resources/Resources\Officials/Officials%20Brochure%202026-2027.pdf" target="_blank">Officials Brochure 2026-2027</a></li></ul><ul id="NavigationMenu_mnuSubResources_Resources_Students__Parents" class="dsNavigationMenuUL dsNavigationMenuSubPanel"><li><div class="dsNavigationSubMenuTitle">Students/Parents</div></li><li><a id="SUB_MAIN_mnuSubResources_Resources_Students__Parents" class=" dsNavigationMenuLink " onclick="changeToMainMenu();"><i class="fa fa-arrow-left" style="float:left;"></i><div style="float:left;" class="dsNavigationMenuBackButton">MAIN MENU</div><div style="clear:both;"></div></a></li><li><a id="mnuSubResources_3db086dd-d2f0-4486-bfdf-ed5c25fb8a6c" href="/TenantHTML.aspx?D=Resources\Students__Parents&amp;F=College Bound Athlete Resources.html">College Bound Athlete Resources</a></li><li><a id="mnuSubResources_7341131a-7af9-4936-8d94-f2100928190a" href="/resources/Resources\Students__Parents/Concussion%20Consent%20Form.pdf" target="_blank">Concussion Consent Form</a></li><li><a id="mnuSubResources_c735cd8b-36f7-4874-b791-0d5209d1900e" href="/resources/Resources\Students__Parents/Eligibility%20Rules.pdf" target="_blank">Eligibility Rules</a></li><li><a id="mnuSubResources_4c398e87-f8f9-4e69-ac70-7d7adf2aa712" href="/resources/Resources\Students__Parents/Heat%20Awarenes%20Consent%20Form.pdf" target="_blank">Heat Awarenes Consent Form</a></li><li><a id="mnuSubResources_dc15fa2b-557c-4302-9a7a-1815f0c0ce34" href="/resources/Resources\Students__Parents/Mental%20Health%20Consent%20Form.pdf" target="_blank">Mental Health Consent Form</a></li><li><a id="mnuSubResources_0c0f2bed-f251-4dc6-b9f6-e1ca03fd3f5a" href="http://www.nfhslearn.com/electiveDetail.aspx?courseID=22000" target="_blank">NFHS Sportsmanship Course</a></li><li><a id="mnuSubResources_77e079e8-542a-4b01-b4c2-6c5df9782c9a" href="https://nfhslearn.com/?courseID=18000" target="_blank">Parents in Sports Course</a></li><li><a id="mnuSubResources_7c104ead-e1cc-4e74-b0ed-fe5a382cfa2a" href="/TenantHTML.aspx?D=Resources\Students__Parents&amp;F=Student Athlete Parenting Video.html">Student Athlete Parenting Video</a></li><li><a id="mnuSubResources_9a5d073d-c20d-48f6-9ad8-15d30326b028" href="/TenantHTML.aspx?D=Resources\Students__Parents&amp;F=Students__Parents FAQs.html">Students/Parents FAQs</a></li><li><a id="mnuSubResources_03ac10a5-39be-42fe-bd17-11f48693a7df" href="/resources/Resources\Students__Parents/Sudden%20Cardiac%20Arrest%20Consent%20Form.pdf" target="_blank">Sudden Cardiac Arrest Consent Form</a></li><li><a id="mnuSubResources_f3b4326d-4ad8-42ec-9a7e-abfd307b6d33" href="/TenantHTML.aspx?D=Resources\Students__Parents&amp;F=Transfers and Hardship Appeals.html">Transfers and Hardship Appeals</a></li></ul>
	</div>
            </div>
        
</div>

        <div id="dsPagePanel" class="dsPagePanel">
	

            <div id="dsHeaderPanel" class="dsHeaderPanel">
		
                
                <ul id="ToolbarUL" class="dsHeaderMenu">
                    <li class="dsHeaderLeft dsNavigationMenuClosed" role="button" tabindex="0" aria-label="Open menu" onclick="openNav()" onkeydown="if(event.key==='Enter'||event.key===' '){event.preventDefault();openNav();}"><i class="fa fa-bars" aria-hidden="true"></i></li>
                    <li class="dsHeaderLeft dsNavigationMenuOpen" role="button" tabindex="0" aria-label="Close menu" onclick="closeNav()" onkeydown="if(event.key==='Enter'||event.key===' '){event.preventDefault();closeNav();}"><span style="font-size: 32px !important; font-weight: normal;" aria-hidden="true">×</span></li>

                    <li id="NavBack" class="dsHeaderLeft dsHeaderNavButton">
                        <a role="button" tabindex="0" aria-label="Back" onclick="history.back();return false;"
                           onkeydown="if(event.key==='Enter'||event.key===' '){event.preventDefault();history.back();}"><i class="fa fa-arrow-left" aria-hidden="true"></i></a>
                    </li>

                    <li id="NavRefresh" class="dsHeaderLeft dsHeaderNavButton">
                        <a role="button" tabindex="0" aria-label="Refresh" onclick="window.location.reload();return false;"
                           onkeydown="if(event.key==='Enter'||event.key===' '){event.preventDefault();window.location.reload();}"><i class="fa fa-arrow-rotate-right" aria-hidden="true"></i></a>
                    </li>

                    <li id="Logo" class="dsHeaderLeft">
                        
                        <a href="/" id="LogoLink" onclick="changeToMainMenu();">
                            <img id="LogoImage" class="fpLogoImage" src="/resources/_LogoSmall.png?t=1000" alt="Home" /></a>
                        <!--<div class="dsLogoTextDatable">datable</div>-->
                    </li>

                </ul>
                <div id="WelcomeText" class="dsWelcomeText">
                    <a id="Signup" title="Sign-Up" class="dsHeaderButton" href="../SignUp.aspx?ReturnUrl=%2fSportPages%2fSportPageInfo.aspx%3fTournamentID%3d204">Sign-Up</a>
                    <a id="Login" title="Login" class="dsHeaderButton" href="../Login.aspx?ReturnUrl=%2fSportPages%2fSportPageInfo.aspx%3fTournamentID%3d204">Login</a>
                </div>
            
	</div>



            <div class="dsMiddlePanel">

                

    <div class="dsSiteContent">

        
        <div id="NoticePanel" class="dsSiteNotice" role="status">
		
            <i class="fa fa-circle-info" aria-hidden="true"></i>
            We are currently applying upgrades to the site. You may experience temporary display issues while the migration completes.
        
	</div>

        
        

        
        

        
        <main class="dsSiteContentBody">
            

    <div id="ToolbarPanel" class="dsToolbarPanel">
		
        <ul id="Toolbar" class="dsToolbar">
            <li class="dsToolbarLeft">
                <a id="Sport" title="Sport" href="#"><i class="fa fa-solid fa-caret-down"></i>HS Outdoor Track (B&G)</a>
                <div id="SportPanel" class="dsToolbarDropdown2 dsToolbarDropdownLeft dsToolbarPanelScroll">
			
                <b>HS - Fall</b><br/><a id="mnuHSSport_Cross Country (Boys)" href="/SportPages/SportPageInfo.aspx?TournamentID=1&amp;L=1"><i class="fa-solid fa-person-running"></i>Cross Country (Boys)</a><a id="mnuHSSport_Cross Country (Girls)" href="/SportPages/SportPageInfo.aspx?TournamentID=9&amp;L=1"><i class="fa-solid fa-person-running"></i>Cross Country (Girls)</a><a id="mnuHSSport_Fall Golf (Boys)" href="/SportPages/SportPageInfo.aspx?TournamentID=4&amp;L=1"><i class="fa-duotone fa-golf-flag-hole"></i>Fall Golf (Boys)</a><a id="mnuHSSport_Field Hockey" href="/SportPages/SportPageInfo.aspx?TournamentID=2&amp;L=1"><i class="fa-solid fa-field-hockey-stick-ball"></i>Field Hockey</a><a id="mnuHSSport_Football" href="/SportPages/SportPageInfo.aspx?TournamentID=3&amp;L=1"><i class="fa-duotone fa-football"></i>Football</a><a id="mnuHSSport_Soccer (Boys)" href="/SportPages/SportPageInfo.aspx?TournamentID=5&amp;L=1"><i class="fa-duotone fa-futbol"></i>Soccer (Boys)</a><a id="mnuHSSport_Soccer (Girls)" href="/SportPages/SportPageInfo.aspx?TournamentID=6&amp;L=1"><i class="fa-duotone fa-futbol"></i>Soccer (Girls)</a><a id="mnuHSSport_Swimming (Girls)" href="/SportPages/SportPageInfo.aspx?TournamentID=7&amp;L=1"><i class="fa-duotone fa-person-swimming"></i>Swimming (Girls)</a><a id="mnuHSSport_Volleyball (Girls)" href="/SportPages/SportPageInfo.aspx?TournamentID=8&amp;L=1"><i class="fa-duotone fa-volleyball"></i>Volleyball (Girls)</a><b>HS - Winter</b><br/><a id="mnuHSSport_Boys Basketball" href="/SportPages/SportPageInfo.aspx?TournamentID=100&amp;L=1"><i class="fa-duotone fa-basketball-hoop"></i>Boys Basketball</a><a id="mnuHSSport_Cheer" href="/SportPages/SportPageInfo.aspx?TournamentID=107&amp;L=1"><i class="fa-solid fa-child-reaching"></i>Cheer</a><a id="mnuHSSport_Dance" href="/SportPages/SportPageInfo.aspx?TournamentID=1008&amp;L=1"><i class="fa-solid fa-user-music"></i>Dance</a><a id="mnuHSSport_Girls Basketball" href="/SportPages/SportPageInfo.aspx?TournamentID=101&amp;L=1"><i class="fa-duotone fa-basketball-hoop"></i>Girls Basketball</a><a id="mnuHSSport_Gymnastics (Girls)" href="/SportPages/SportPageInfo.aspx?TournamentID=102&amp;L=1"><i class="fa-solid fa-person-falling"></i>Gymnastics (Girls)</a><a id="mnuHSSport_Ice Hockey (Boys)" href="/SportPages/SportPageInfo.aspx?TournamentID=103&amp;L=1"><i class="fa-duotone fa-hockey-sticks"></i>Ice Hockey (Boys)</a><a id="mnuHSSport_Ice Hockey (Girls)" href="/SportPages/SportPageInfo.aspx?TournamentID=108&amp;L=1"><i class="fa-duotone fa-hockey-sticks"></i>Ice Hockey (Girls)</a><a id="mnuHSSport_Indoor Track (B&G)" href="/SportPages/SportPageInfo.aspx?TournamentID=104&amp;L=1"><i class="fa-solid fa-person-running"></i>Indoor Track (B&G)</a><a id="mnuHSSport_Swimming (Boys)" href="/SportPages/SportPageInfo.aspx?TournamentID=105&amp;L=1"><i class="fa-duotone fa-person-swimming"></i>Swimming (Boys)</a><a id="mnuHSSport_Wrestling" href="/SportPages/SportPageInfo.aspx?TournamentID=106&amp;L=1"><i class="fa-duotone fa-hands"></i>Wrestling</a><b>HS - Spring</b><br/><a id="mnuHSSport_Baseball" href="/SportPages/SportPageInfo.aspx?TournamentID=200&amp;L=1"><i class="fa-duotone fa-baseball"></i>Baseball</a><a id="mnuHSSport_Flag Football (Girls)" href="/SportPages/SportPageInfo.aspx?TournamentID=1017&amp;L=1"><i class="fa-duotone fa-football"></i>Flag Football (Girls)</a><a id="mnuHSSport_Golf (Boys)" href="/SportPages/SportPageInfo.aspx?TournamentID=201&amp;L=1"><i class="fa-duotone fa-golf-flag-hole"></i>Golf (Boys)</a><a id="mnuHSSport_Golf (Girls)" href="/SportPages/SportPageInfo.aspx?TournamentID=209&amp;L=1"><i class="fa-duotone fa-golf-flag-hole"></i>Golf (Girls)</a><a id="mnuHSSport_Lacrosse (Boys)" href="/SportPages/SportPageInfo.aspx?TournamentID=202&amp;L=1"><i class="fa-duotone fa-lacrosse-stick-ball"></i>Lacrosse (Boys)</a><a id="mnuHSSport_Lacrosse (Girls)" href="/SportPages/SportPageInfo.aspx?TournamentID=203&amp;L=1"><i class="fa-duotone fa-lacrosse-stick-ball"></i>Lacrosse (Girls)</a><a id="mnuHSSport_Outdoor Track (B&G)" href="/SportPages/SportPageInfo.aspx?TournamentID=204&amp;L=1"><i class="fa-solid fa-person-running"></i>Outdoor Track (B&G)</a><a id="mnuHSSport_Softball" href="/SportPages/SportPageInfo.aspx?TournamentID=205&amp;L=1"><i class="fa-duotone fa-baseball-bat-ball"></i>Softball</a><a id="mnuHSSport_Tennis (Boys)" href="/SportPages/SportPageInfo.aspx?TournamentID=206&amp;L=1"><i class="fa-duotone fa-racquet"></i>Tennis (Boys)</a><a id="mnuHSSport_Tennis (Girls)" href="/SportPages/SportPageInfo.aspx?TournamentID=207&amp;L=1"><i class="fa-duotone fa-racquet"></i>Tennis (Girls)</a><a id="mnuHSSport_Volleyball (Boys)" href="/SportPages/SportPageInfo.aspx?TournamentID=208&amp;L=1"><i class="fa-duotone fa-volleyball"></i>Volleyball (Boys)</a>
		</div>
            </li>
            <li class="dsToolbarLeft dsToolbarButton">
                <a id="InfoLink" title="Home" href="/SportPages/SportPageInfo.aspx?TournamentID=204" target="_self">Home</a>
            </li>
            <li class="dsToolbarLeft dsToolbarButton">
                <a id="TournamentLink" title="Tournament" href="/SportPages/SportPageBrackets.aspx?TournamentID=204" target="_self">Tournament</a>
            </li>
            <li class="dsToolbarLeft dsToolbarButton">
                
            </li>
            <li class="dsToolbarLeft dsToolbarButton">
                <a id="ScoresLink" title="Past Scores" href="/SportPages/SportPageScore.aspx?TournamentID=204" target="_self">Scores</a>
            </li>
            <li class="dsToolbarLeft dsToolbarButton">
                <a id="ScheduleLink" title="Upcoming Games" href="/SportPages/SportPageSchedule.aspx?TournamentID=204" target="_self">Upcoming</a>
            </li>
            <li class="dsToolbarLeft dsToolbarButton">
                <a id="RosterLink" title="Rosters" href="/SportPages/SportPageRosters.aspx?TournamentID=204" target="_self">Rosters</a>
            </li>
        </ul>
    
	</div>

    <div id="Panel1" class="dsFormPanelNormal ">
		
        

 

         <div id="HeaderWrapper">
			
        
		</div>

        <div id="BodyWrapper" class="dashboard-content">
			
        <div class="dsFlexBlock80">
				<img style='width:100%' src='/resources/_Media/Sport Headers/Outdoor Track Header.jpg' border='0'><div class="ImportantWrapper">
					<div class="dsFlexBlock50 ImportantBlock">
						<h2>Important Dates</h2><div class="ImportantBlockContent">
							<div class="SportInfoDates">
	<h2>2025-2026 Boys & Girls Outdoor Track Season</h2>
	<span class='SportInfoTitle'>First Practice Date:</span> March 21<br />
	<span class='SportInfoTitle'>Coaches Meeting:</span> March 24 at 7:30pm via Zoom<br />
	<span class='SportInfoTitle'>First Contest:</span> April 4<br />
	<span class='SportInfoTitle'>Last Date to Count for Tournament:</span> May 27<br />
	<!--<span class='SportInfoTitle'>State Tournament:</span> Class June 1, June 2, June 3  Open June 7; NE June 14; Specials June 16, 17, 18<br />-->
</div>
<div class="SportInfoDates">
    <h2>2025-2026 Outdoor Track Championships</h2>
    <span class="SportInfoTitle">Divisional Championships @ Willow Brook Park</span> <br />
    <span class="SportInfoTitle">Class LL: </span>Sat. May 30 Field events start at 11:00am, running events start at 11:30am  <br />
	<span class="SportInfoTitle">Class MM: </span>Sun. May 31 Field events start at 10:00am, running events start at 10:30am <br />
	<span class="SportInfoTitle">Class L: </span>Sun. May 31 Field events start at 4:00pm, running events start at 4:30pm <br />
    <span class="SportInfoTitle">Class M: </span>Mon. June 1 Field events start at 10:00am, running events start at 10:30am  <br />
    <span class="SportInfoTitle">Class S: </span>Mon. June 1 Field events start at 4:00pm, running events start at 4:30pm <br /><br />

    <span class="SportInfoTitle">State Open Championship</span> Sat. June 6 @Willow Brook Park Field events start at 9:30am, running events start at 10:00am <br /><br />

    <span class="SportInfoTitle">New England Championship</span> Sat. June 13 @Nobel HS, North Berwick, Maine events start at 10:00am, running events start at 10:30am <br /><br />

    <span class="SportInfoTitle">CIAC Specials @ Willow Brook Park</span> <br />
	<span class="SportInfoTitle">Boys Hammer Throw</span> Mon. June 15 12:00pm <br />
	<span class="SportInfoTitle">Girls Hammer Throw</span> Mon. June 15 3:00pm <br />

	<span class="SportInfoTitle">Girls Heptathlon</span> Mon. June 15 10:30am, Tues. June 16 10:00am <br />
    <span class="SportInfoTitle">Boys Decathlon</span> Mon. June 15 10:00am, Tues. June 16 10:00 am&nbsp;<br />
    <span class="SportInfoTitle">Girls Steeplechase</span> Mon. June 15 3:30pm <br />
    <span class="SportInfoTitle">Boys Steeplechase</span> Tues. June 16&nbsp;3:30pm <br />
</div>
						</div>
					</div><div class="dsFlexBlock50 ImportantBlock">
						<h2>Important Links</h2><div class="ImportantBlockContent">
							<ul><li><a id="mnuSubResources_c1f4579c-47a8-47ac-87bc-3c9f09e53ec2" href="/resources/Tournament%20Info\Outdoor%20Track/Information%20Packet%202025-2026.pdf" target="_blank"><i class='fa-duotone fa-file-circle-info'></i>Information Packet 2025-2026</a></li><li><a id="mnuSubResources_6a667a23-bce2-4018-9f87-a12aa25d9b21" href="https://www.youtube.com/watch?v=RsNm5SVWq_g" target="_blank"><i class='fa-duotone fa-file-circle-info'></i>2026 Coaches Meeting Recording</a></li><li><a id="mnuSubResources_28c7de1b-27b4-4dab-97c7-a25c4aeb4582" href="http://www.nfhs.org/activities-sports/track-fieldcross-country" target="_blank"><i class='fa-duotone fa-file-circle-info'></i>NFHS TrackCross Country Information</a></li><li><a id="mnuSubResources_ee0c257b-781e-4bd0-9598-21671736c94c" href="mailto://grimeseven@aol.com" target="_blank"><i class='fa-duotone fa-file-circle-info'></i>Officials Contact</a></li><li><a id="mnuSubResources_4ec3149f-ec8b-4485-adb1-00f2393936df" href="/resources/Tournament%20Info\Outdoor%20Track/Game%20Limitations%20and%20Practice%20Dates%20Calendars%2025-27.pdf" target="_blank"><i class='fa-duotone fa-file-circle-info'></i>Game Limitations and Practice Dates Calendars 25-27</a></li></ul>
						</div>
					</div>
				</div>
			</div><div class="dsFlexBlock20">
				<div id='DashboardSideAds' class='DashboardAds'><a target='_self' href='/resources/Tournament%20Info/Outdoor Track/Information Packet 2025-2026.pdf'><img src='/resources/_media/DashboardAds/Information Packet.png' /></a><a target='_self' href='/DashboardSchedule.aspx?L=1&SportID=-1_1029_-1'><img src=' /resources/_media/DashboardAds/Todays Games.png' /></a><a target='_blank' href='https://gofan.co/app/school/CIAC'><img src=' /resources/_media/DashboardAds/Go Fan Tickets.png' ></a><a target='_self' href='/Reports/PastChampionsBySport.aspx?TournamentID=204'><img src='/resources/_media/DashboardAds/Past Champions.png' /></a></div>
			</div><div class="dsFlexBlock90">
				<div class="dsFlexBlockFill">
					
<style>
    #partnerbanner {
        background-color: white;
        padding: 10px;
        line-height: 1;
    }

        #partnerbanner a {
            display: block;
            margin: 0px 20px 0px 20px;
        }

        #partnerbanner img {
            height: 60px;
            width: auto;
        }

    
</style>

<script>
    $(document).ready(function () {

        $('span', $('#partnerbannerads')).sort(function () { return (Math.round(Math.random()) - 0.5) }).appendTo($('#partnerbannerads'));

        for (let i = 0; i < $('#partnerbannerads').children().length - 1; i += 1) {
            $('#partnerbanner').append('<div>' + $('#partnerbannerads').children()[i].outerHTML + '</div>');
        }


        $('.bannerp').slick({
            arrows: false,
            autoplay: true,

            speed: 9000, autoplay: true, autoplaySpeed: 0, cssEase: 'linear', slidesToShow: 1, slidesToScroll: 1, variableWidth: true,

            dots: false,
            infinite: true,
            pauseOnHover: true,
            pauseOnDotsHover: true

            //slidesToShow: 3,
            //slidesToScroll: 1,
            //responsive: [
            //    {
            //        breakpoint: 1024,
            //        settings: {
            //            slidesToShow: 3
            //        }
            //    },
            //    {
            //        breakpoint: 600,
            //        settings: {
            //            slidesToShow: 2
            //        }
            //    },
            //    {
            //        breakpoint: 480,
            //        settings: {
            //            slidesToShow: 1
            //        }
            //    } ]

        });
    });
</script>

<div id="partnerbanner" class="bannerp">

</div>

<div id="partnerbannerads" style="display:none">
    <span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='2037' CarouselEntry='988 Call line'><a target='_blank' href='https://uwc.211ct.org/youthmentalhealth/'><img src='\Resources\Carousel\Partner\Images\0000002037.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='2015' CarouselEntry='Jostens'><a target='_blank' href='https://www.jostens.com/'><img src='\Resources\Carousel\Partner\Images\0000002015.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1952' CarouselEntry='CBYD'><a target='_blank' href='https://www.cbyd.com/'><img src='\Resources\Carousel\Partner\Images\0000001952.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1803' CarouselEntry='Partner Eli's'><a target='_blank' href='https://elisrg.com/'><img src='\Resources\Carousel\Partner\Images\0000001803.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1543' CarouselEntry='IKEA Logo'><a target='_blank' href='https://www.ikea.com/us/en/stores/new-haven/'><img src='\Resources\Carousel\Partner\Images\0000001543.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1531' CarouselEntry='Rebel Athletic'><a target='_blank' href='https://rebelathletic.com/'><img src='\Resources\Carousel\Partner\Images\0000001531.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1350' CarouselEntry='Partner Logo'><a target='_blank' href='https://www.hartford.edu/admission/?utm_source=marketing_materials&utm_medium=digital&utm_campaign=ug-other-awareness-admission-ciac-partnership'><img src='\Resources\Carousel\Partner\Images\0000001350.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1330' CarouselEntry='CT Realtors Logo'><a target='_blank' href='https://www.ctrealtors.com/for-consumers/'><img src='\Resources\Carousel\Partner\Images\0000001330.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1209' CarouselEntry='CT Run Co'><a target='_blank' href='https://ctrunco.com/'><img src='\Resources\Carousel\Partner\Images\0000001209.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1205' CarouselEntry='Middlesex Health'><a target='_blank' href='https://middlesexhealth.org/'><img src='\Resources\Carousel\Partner\Images\0000001205.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1203' CarouselEntry='Chesla'><a target='_blank' href='https://chesla.org/'><img src='\Resources\Carousel\Partner\Images\0000001203.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1197' CarouselEntry='CT Dairy'><a target='_blank' href='http://www.ctdairy.org/'><img src='\Resources\Carousel\Partner\Images\0000001197.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1193' CarouselEntry='ServPro'><a target='_blank' href='https://www.servpro.com/'><img src='\Resources\Carousel\Partner\Images\0000001193.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1191' CarouselEntry='MaxPreps'><a target='_blank' href='https://maxpreps.com/'><img src='\Resources\Carousel\Partner\Images\0000001191.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1189' CarouselEntry='Hartford Healthcare'><a target='_blank' href='https://hartfordhealthcare.org/'><img src='\Resources\Carousel\Partner\Images\0000001189.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1187' CarouselEntry='Hartford Athletics'><a target='_blank' href='https://www.hartfordathletic.com/seasontickets/'><img src='\Resources\Carousel\Partner\Images\0000001187.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1185' CarouselEntry='Spalding'><a target='_blank' href='https://www.spalding.com/'><img src='\Resources\Carousel\Partner\Images\0000001185.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1183' CarouselEntry='Konica Minolta'><a target='_blank' href='https://www.konicaminolta.com/'><img src='\Resources\Carousel\Partner\Images\0000001183.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1181' CarouselEntry='Army National Guard'><a target='_blank' href='http://bit.ly/38WXK3Z'><img src='\Resources\Carousel\Partner\Images\0000001181.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1179' CarouselEntry='Drive Sober'><a target='_blank' href='https://portal.ct.gov/dot'><img src='\Resources\Carousel\Partner\Images\0000001179.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1177' CarouselEntry='CT DOT'><a target='_blank' href='https://portal.ct.gov/dot'><img src='\Resources\Carousel\Partner\Images\0000001177.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1175' CarouselEntry='Northwest Designs'><a target='_blank' href='https://nwd.ink/'><img src='\Resources\Carousel\Partner\Images\0000001175.png'  /></a></span>

</div>





				</div>
			</div><h2 class="dsFlexBlock90 SportTitle">Today's Meets</h2><div class="dsFlexBlock90">
				<div id="GameTickerWrapper" class="GameTickerWrapper">
					No scheduled events
				</div>
			</div><h2 class="dsFlexBlock90 SportTitle">Recent Meets</h2><div class="dsFlexBlock90">
				<div id="GameTickerWrapper" class="GameTickerWrapper">
					No scheduled events
				</div>
			</div>
		</div>


    
	</div>


        </main>

        
        <div id="FooterPanel" class="dsSiteFooter" role="contentinfo">
		
            <div id="FooterLinks" class="dsSiteFooterLinks">

		</div>
            <div class="dsSiteFooterMark">
                Powered by <span class="dsSiteFooterMarkName">FusionPoint Sports</span>
            </div>
        
	</div>

    </div>



            </div>
        
</div>


    </form>
</body>
</html>
