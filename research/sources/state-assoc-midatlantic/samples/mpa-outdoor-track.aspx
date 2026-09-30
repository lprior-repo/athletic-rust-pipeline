

<!DOCTYPE html>

<html lang="en">
<head><meta name='description' content='The Maine Principals' Association is a nonprofit serving Maine schools through high school interscholastic athletics governance and professional support for Kâ€“12 administrators.'/><link href="/_Styles/2010/Config.css?v=20260921c" rel="stylesheet" type="text/css" /><link href="/_Styles/Controls.css?v=20260921c" rel="stylesheet" type="text/css" /><link href="/_Styles/Dashboard.css?v=20260921c" rel="stylesheet" type="text/css" /><link href="/_Styles/TournamentCentral.css?v=20260921c" rel="stylesheet" type="text/css" /><meta charset="utf-8" /><title>
	FusionPoint Sports - Maine Principal's Association
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
<input type="hidden" name="__VIEWSTATE" id="__VIEWSTATE" value="+HnQBYFk1O8WfmjCxEBUoEPtWX7CrGa2Dk+SInufpEHbyqkM+EBI++z/rlUWp71Kz952n+ndX0P71Vkpy1DrBuFCPGzDdbdhTe45YvCqTEF2DjJIK7iWhjDY8Ml87tYmNdJTcmLLqbsyFM5Cdwp5JeAPTp45NUyl1a2iWStPsiLmh3iHMj/lrDe2EZfuWoXI5bDwru5AZMBDySQBu/C9LfLVh0ag1/0l5W+rz3VSlbcq4BThqOmxCBpBV+o8M7P2IiO78nzWRmiKQIbU8Fl+KsUTnTxiIbugqzqLG76HqBl2Jcxa6Z1zp+UtplmJ0xwIBYv4K/PWs+sMe6xEUikBzdLyDQ9fHhAMHmymEBBbY1Azqjo1vz0sowHc6cBRYKce1hcjEb6UcA7ktaY2rjV+5EqjHeeSV3KTK1oLYEFnGHhoBl/i576y7DRQh24JYznDy0eAp/Hprw1ww2D9DK8pXo/LWLMeipWeXNOBWeGI6R9OSFQmsg9LXeD//u9NVOmVju2VusncRrwnrYAP0On6R6FOY1ZG01IeqU3PTdqIYRAYxWCN6JQoVpbJUAA3+iibZrPyRwGYMWM4wMGnhH2getp39fpHfTeunLpPQEmbjutNg0C1hdxKSESgmTYTkg5PqZ9bpXo2JyE4+KZWuU07tGQd9AGKf9QAc9B89m15toWDYSqYGKrFqn3kA9io9Vtuk0v0fQ8eHrZwpmPgFaiiPaTaQ32CtD8sD7clLn3O/sMfuShfmG0Vo8NAxkGlbKRO/LxDzmn1atuMx3BxQTWdX9VSyEhe0YtwxO9FxXVdhT5XXl1d2VwhqbAbfaplV+MOFKFrWlXGpf2vxkabVag5F72ClDkQO3Jts4bcV18AvxPm2jORICbT0F9G9TfzCW+nAt+3RviVWL5qQX8wYNrFMBgbP9+h/+2T4CR/R8GPCJrFRSdnvHilm2kwQZg7s6fFbvx1rmAm1m6WgUEsJQphX4xt5G11kUaPxKNfgxkgboAS6av/hgu5pOM8qFys62cxfy/UWtGcSpfe7mUV/+n96MYki4CL9/FZLHsBqD8n7hzGsw89w36Pg/stdLAN2F2UWnhq2N75pJDQCrtTj+ldBJv0edwcsCKQ7YGRL8MCH15Bxc4cKcvGPB2MYEbsKkyvX0hiAXLaLVeFVEeJX9/hS+Zk01CAquTI/QJ1tEYjpTsk/CodoPRgXvHlFP4geOPKvf0inqmGTMPbOE3FE8O2+6gYyCC1D1VXILTIQVw+eYT4xEXJy2+fMb9bYYpJnSBwn0Xr02t8hTmS3NYKXQoDqsFWh8d07QowA68vsdSZo80AG/TioxqoZPLO4KCE5H4EiJEpoHq7yhbQuV5BeEltjrMiASVzSZLdaDXJ7r3NyrWIsBuUCO4QzkYZnArr31unR+SZgnJ9yGSL4ZR1N5l7ixd/pxwh6Zgh44DPyMGdEpbNoUDFeKKuBpavx+9j3Lbk" />
</div>

<div class="aspNetHidden">

	<input type="hidden" name="__VIEWSTATEGENERATOR" id="__VIEWSTATEGENERATOR" value="A370CA34" />
</div>
        <div id="dsLoader"></div>

        <div id="dsNavigation" class="dsNavigationOverlay">
	
            <div id="dsNavigationMenu" class="dsNavigationMenu">
                <a href="javascript:void(0)" class="dsNavigationCloseButton" onclick="closeNav()">×</a>
                <div id="dsNavigationMenuContent" class="dsNavigationMenuContent">
		
                    <ul id="dsNavigationMenuUL" class="dsNavigationMenuUL dsNavigationMenuSubPanel active"><li><a id="mnuDashboard" href="\">Dashboard</a></li><li><a id="mnuSportHS" class=" dsNavigationMenuLink " href="https://www.mpa.cc/SportPages/SportPageInfo.aspx?TournamentID=204#NavigationMenu_mnuSportHS"><div style="float:left;">Sport Info - High School</div><i class="fa fa-chevron-right" style="float:right;"></i><div style="clear:both;"></div></a></li><li><a id="mnuScheduleHS" href="\DashboardSchedule.aspx">Schedules</a></li><li><a id="mnuSchoolPages" href="/SchoolPages/School.aspx">School Pages</a></li><li><a id="mnuSchoolSchedule" href="/DashboardTeamSchedule.aspx">Team Schedule</a></li><li><a id="mnuSchoolRoster" href="/DashboardTeamRoster.aspx?SeasonRoster=1">Team Rosters</a></li><li><a id="mnuMasterSchedule" href="/MasterSchedule.aspx?TeamLevelID=5">Master Schedule</a></li><li><a id="mnuTournamentCentralPastChamps" href="\Reports\PastChampionsBySport.aspx">Past Champions</a></li><li class="dsNavigationMenuGroupSeperator"></li><li><div class="dsNavigationMenuGroupTitle">Tournament Central</div></li><li><a id="mnuTournamentDashboard" href="\DashboardTournamentCentral.aspx">Dashboard</a></li><li><a id="mnuTournamentRankings" href="\TournamentCentralRankings.aspx">Tournament Standings</a></li><li><a id="mnuTournamentRankingDetail" href="\Custom\MPA\MPADetail.aspx">Standings Detail</a></li><li><a id="mnuTournamentBrackets" href="\TournamentCentralBrackets.aspx">Brackets</a></li><li><a id="mnuTournamentTeamStats" href="\DashboardTeamRoster.aspx">Tournament Rosters</a></li><li class="dsNavigationMenuGroupSeperator"></li><li><div class="dsNavigationMenuGroupTitle">MPA Members</div></li><li><a id="mnuDirectory" href="\Directory.aspx">School Directory</a></li><li class="dsNavigationMenuGroupSeperator"></li><li><div class="dsNavigationMenuGroupTitle">Important Links</div></li><li><a id="mnuSubResources_bc151e55-998d-4059-b787-bd7358089710" href="https://gofan.co/app/school/MPA" target="_blank">GoFan Tickets</a></li><li><a id="mnuSubResources_995b8d5e-a92b-42a9-a69d-e25ccb7bd59b" href="https://nwd.ink/s/mpa/" target="_blank">Championship Apparel</a></li><li><a id="mnuSubResources_81b27a2c-17b4-4f87-b76b-4c9ba0d064cd" href="https://www.flipsnack.com/tpg2020/25-26-maine-state-championships" target="_blank">Championship Programs</a></li><li><a id="mnuSubResources_bc19edee-8b9f-493b-8b85-1b522f17f382" href="https://mpaprof.org" target="_blank">Professional Division</a></li><li class="dsNavigationMenuGroupSeperator"></li><li><div class="dsNavigationMenuGroupTitle">Resources</div></li><li><a id="mnuSubResources_4ec81bda-b4dc-4a87-97d2-ed49d96dbeba" href="/TenantHTML.aspx?D=Resources&amp;F=Heart of the Arts Awards.html">Heart of the Arts Awards</a></li><li><a id="mnuSubResources_790ab1ce-9839-4c4b-8c53-0f685ec49f1f" href="/TenantHTML.aspx?D=Resources&amp;F=Larry LaBrie Award.html">Larry LaBrie Award</a></li><li><a id="mnuSubResources_89314846-d9ea-4014-87ae-dba2d3e8bef7" href="/TenantHTML.aspx?D=Resources&amp;F=State Coaches of the Year.html">State Coaches of the Year</a></li><li><a id="mnuSubResources_2e6aacdc-eae3-40a0-9533-4d17c58be8f6" href="/TenantHTML.aspx?D=Resources&amp;F=Hall of Excellence.html">Hall of Excellence</a></li><li><a id="mnuSubResources_97264800-e8ec-4b70-97d9-950945a815b9" href="/resources/Resources/Sport%20Season%20Dates%202026-2027.pdf" target="_blank">Sport Season Dates 2026-2027</a></li><li><a id="mnuSubResources_74386b14-5073-4f49-a180-d2c035123aec" href="/resources/Resources/Sport%20Season%20Dates%202026-2030.pdf" target="_blank">Sport Season Dates 2026-2030</a></li><li class="dsNavigationMenuGroupSeperator"></li><li><div class="dsNavigationMenuGroupTitle">About Us</div></li><li><a id="mnuSubResources_dc6d1e2d-3d01-4121-91a2-4f0506bf6fe6" href="/TenantHTML.aspx?D=About Us&amp;F=Conference and Meeting Center.html">Conference and Meeting Center</a></li><li><a id="mnuSubResources_91872bf4-85d0-4f96-816d-d9cb32c46822" href="/TenantHTML.aspx?D=About Us&amp;F=Directions.html">Directions</a></li><li><a id="mnuSubResources_519c64de-1346-4983-a504-e35a3af37681" href="/resources/About%20Us/Handbook.pdf" target="_blank">Handbook</a></li><li><a id="mnuSubResources_05ca20cc-9f27-4d88-9ca4-b76f8c86498d" href="/TenantHTML.aspx?D=About Us&amp;F=Membership and Dues.html">Membership and Dues</a></li><li><a id="mnuSubResources_c4d62414-d45c-47bb-a31f-d04b54924eb7" href="/TenantHTML.aspx?D=About Us&amp;F=MPA Information.html">MPA Information</a></li><li><a id="mnuSubResources_aec2742a-a6f2-40a8-80e0-5d0453c3e11c" href="/TenantHTML.aspx?D=About Us&amp;F=MPA Media.html">MPA Media</a></li><li><a id="mnuSubResources_3b1946f9-be1c-4f85-abfb-977bf0edebd1" href="/TenantHTML.aspx?D=About Us&amp;F=Officers and Staff.html">Officers and Staff</a></li><li><a id="mnuSubResources_4fa45c6b-367f-47ca-9640-bbaf50b74532" href="/resources/About%20Us/Professional%20Membership%20Form.pdf" target="_blank">Professional Membership Form</a></li><li><a id="mnuSubResources_2ad7c4ce-6a0e-43f8-8d66-6d02561218c6" href="/CarouselAll.aspx?CarouselID=9" target="_blank">Sponsors</a></li><li><a id="mnuSubResources_a43b4aa0-6853-4136-a046-08df6088a406" href="/resources/About%20Us/MPA%20Acceptable%20Use%20Policy.pdf" target="_blank">MPA Acceptable Use Policy</a></li><li><a id="mnuSubResources_0c569e65-67ed-4c2b-9348-f6a4ba00b5ed" href="/resources/About%20Us/MPA%20Third-Party%20For-Profit%20Data%20Access%20Policy.pdf" target="_blank">MPA Third-Party For-Profit Data Access Policy</a></li><li><a id="mnuSubResources_616a142d-fe61-4db7-bfcc-89ad8fe286ad" href="/resources/About%20Us/MPA%20Committees%202026-2027.pdf" target="_blank">MPA Committees 2026-2027</a></li><li class="dsNavigationMenuGroupSeperator"></li><li><div class="dsNavigationMenuGroupTitle">Account</div></li><li><a id="mnuLogin" href="../Login.aspx?ReturnUrl=%2fSportPages%2fSportPageInfo.aspx%3fTournamentID%3d204">Login</a></li><li><a id="mnuSignUp" href="../SignUp.aspx?ReturnUrl=%2fSportPages%2fSportPageInfo.aspx%3fTournamentID%3d204">Sign-Up</a></li><li><a id="mnuPasswordReset" href="\PasswordReset.aspx">Forgot Password</a></li></ul>
                    <div style="display: none">
                        <ul id="dsNavigationMenuHidden" class="dsNavigationMenuUL dsNavigationMenuSubPane"></ul>
                    </div>
                <ul id="NavigationMenu_mnuSportHS" class="dsNavigationMenuUL dsNavigationMenuSubPanel"><li><div class="dsNavigationSubMenuTitle">Sport Info - High School</div></li><li><a id="SUB_MAIN_mnuSportHS" class=" dsNavigationMenuLink " onclick="changeToMainMenu();"><i class="fa fa-arrow-left" style="float:left;"></i><div style="float:left;" class="dsNavigationMenuBackButton">MAIN MENU</div><div style="clear:both;"></div></a></li><li class="dsNavigationMenuGroupSeperator"></li><li><div class="dsNavigationMenuGroupTitle">Fall</div></li><li><a id="mnuSportButtonHSCornhole (Unified)" href="\SportPages\SportPageInfo.aspx?TournamentID=1026">Cornhole (Unified)</a></li><li><a id="mnuSportButtonHSCross Country (Boys)" href="\SportPages\SportPageInfo.aspx?TournamentID=1">Cross Country (Boys)</a></li><li><a id="mnuSportButtonHSCross Country (Girls)" href="\SportPages\SportPageInfo.aspx?TournamentID=9">Cross Country (Girls)</a></li><li><a id="mnuSportButtonHSESports Fall" href="\SportPages\SportPageInfo.aspx?TournamentID=304">ESports Fall</a></li><li><a id="mnuSportButtonHSField Hockey" href="\SportPages\SportPageInfo.aspx?TournamentID=2">Field Hockey</a></li><li><a id="mnuSportButtonHSFootball" href="\SportPages\SportPageInfo.aspx?TournamentID=3">Football</a></li><li><a id="mnuSportButtonHSGolf (Coed)" href="\SportPages\SportPageInfo.aspx?TournamentID=4">Golf (Coed)</a></li><li><a id="mnuSportButtonHSSoccer (Boys)" href="\SportPages\SportPageInfo.aspx?TournamentID=5">Soccer (Boys)</a></li><li><a id="mnuSportButtonHSSoccer (Girls)" href="\SportPages\SportPageInfo.aspx?TournamentID=6">Soccer (Girls)</a></li><li><a id="mnuSportButtonHSSoccer (Unified)" href="\SportPages\SportPageInfo.aspx?TournamentID=1007">Soccer (Unified)</a></li><li><a id="mnuSportButtonHSVolleyball (Girls)" href="\SportPages\SportPageInfo.aspx?TournamentID=8">Volleyball (Girls)</a></li><li><a id="mnuSportButtonHSVolleyball (Unified)" href="\SportPages\SportPageInfo.aspx?TournamentID=303">Volleyball (Unified)</a></li><li class="dsNavigationMenuGroupSeperator"></li><li><div class="dsNavigationMenuGroupTitle">Winter</div></li><li><a id="mnuSportButtonHSAlpine Ski (Boys)" href="\SportPages\SportPageInfo.aspx?TournamentID=403">Alpine Ski (Boys)</a></li><li><a id="mnuSportButtonHSAlpine Ski (Girls)" href="\SportPages\SportPageInfo.aspx?TournamentID=404">Alpine Ski (Girls)</a></li><li><a id="mnuSportButtonHSBasketball (Unified)" href="\SportPages\SportPageInfo.aspx?TournamentID=302">Basketball (Unified)</a></li><li><a id="mnuSportButtonHSBoys Basketball" href="\SportPages\SportPageInfo.aspx?TournamentID=100">Boys Basketball</a></li><li><a id="mnuSportButtonHSGirls Basketball" href="\SportPages\SportPageInfo.aspx?TournamentID=101">Girls Basketball</a></li><li><a id="mnuSportButtonHSDrama (Coed)" href="\SportPages\SportPageInfo.aspx?TournamentID=1010">Drama (Coed)</a></li><li><a id="mnuSportButtonHSIce Hockey (Boys)" href="\SportPages\SportPageInfo.aspx?TournamentID=103">Ice Hockey (Boys)</a></li><li><a id="mnuSportButtonHSIce Hockey (Girls)" href="\SportPages\SportPageInfo.aspx?TournamentID=108">Ice Hockey (Girls)</a></li><li><a id="mnuSportButtonHSIndoor Track (Boys)" href="\SportPages\SportPageInfo.aspx?TournamentID=104">Indoor Track (Boys)</a></li><li><a id="mnuSportButtonHSIndoor Track (Girls)" href="\SportPages\SportPageInfo.aspx?TournamentID=300">Indoor Track (Girls)</a></li><li><a id="mnuSportButtonHSNordic Ski" href="\SportPages\SportPageInfo.aspx?TournamentID=402">Nordic Ski</a></li><li><a id="mnuSportButtonHSSwimming (Boys)" href="\SportPages\SportPageInfo.aspx?TournamentID=105">Swimming (Boys)</a></li><li><a id="mnuSportButtonHSSwimming (Girls)" href="\SportPages\SportPageInfo.aspx?TournamentID=7">Swimming (Girls)</a></li><li><a id="mnuSportButtonHSCheerleading Winter (Coed)" href="\SportPages\SportPageInfo.aspx?TournamentID=1005">Cheerleading Winter (Coed)</a></li><li><a id="mnuSportButtonHSWrestling" href="\SportPages\SportPageInfo.aspx?TournamentID=106">Wrestling</a></li><li class="dsNavigationMenuGroupSeperator"></li><li><div class="dsNavigationMenuGroupTitle">Spring</div></li><li><a id="mnuSportButtonHSBaseball" href="\SportPages\SportPageInfo.aspx?TournamentID=200">Baseball</a></li><li><a id="mnuSportButtonHSBocce (Unified)" href="\SportPages\SportPageInfo.aspx?TournamentID=400">Bocce (Unified)</a></li><li><a id="mnuSportButtonHSESports Spring" href="\SportPages\SportPageInfo.aspx?TournamentID=1003">ESports Spring</a></li><li><a id="mnuSportButtonHSLacrosse (Boys)" href="\SportPages\SportPageInfo.aspx?TournamentID=202">Lacrosse (Boys)</a></li><li><a id="mnuSportButtonHSLacrosse (Girls)" href="\SportPages\SportPageInfo.aspx?TournamentID=203">Lacrosse (Girls)</a></li><li><a id="mnuSportButtonHSOutdoor Track (Boys)" href="\SportPages\SportPageInfo.aspx?TournamentID=204">Outdoor Track (Boys)</a></li><li><a id="mnuSportButtonHSOutdoor Track (Girls)" href="\SportPages\SportPageInfo.aspx?TournamentID=301">Outdoor Track (Girls)</a></li><li><a id="mnuSportButtonHSSoftball" href="\SportPages\SportPageInfo.aspx?TournamentID=205">Softball</a></li><li><a id="mnuSportButtonHSTennis (Boys)" href="\SportPages\SportPageInfo.aspx?TournamentID=206">Tennis (Boys)</a></li><li><a id="mnuSportButtonHSTennis (Girls)" href="\SportPages\SportPageInfo.aspx?TournamentID=207">Tennis (Girls)</a></li></ul>
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
                            <img id="LogoImage" class="fpLogoImage" src="/resources/_LogoSmall.png?t=2010" alt="Home" /></a>
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
                <a id="Sport" title="Sport" href="#"><i class="fa fa-solid fa-caret-down"></i>HS Outdoor Track (Boys)</a>
                <div id="SportPanel" class="dsToolbarDropdown2 dsToolbarDropdownLeft dsToolbarPanelScroll">
			
                <b>HS - Fall</b><br/><a id="mnuHSSport_Cornhole (Unified)" href="/SportPages/SportPageInfo.aspx?TournamentID=1026&amp;L=1">Cornhole (Unified)</a><a id="mnuHSSport_Cross Country (Boys)" href="/SportPages/SportPageInfo.aspx?TournamentID=1&amp;L=1"><i class="fa-solid fa-person-running"></i>Cross Country (Boys)</a><a id="mnuHSSport_Cross Country (Girls)" href="/SportPages/SportPageInfo.aspx?TournamentID=9&amp;L=1"><i class="fa-solid fa-person-running"></i>Cross Country (Girls)</a><a id="mnuHSSport_ESports Fall" href="/SportPages/SportPageInfo.aspx?TournamentID=304&amp;L=1">ESports Fall</a><a id="mnuHSSport_Field Hockey" href="/SportPages/SportPageInfo.aspx?TournamentID=2&amp;L=1"><i class="fa-solid fa-field-hockey-stick-ball"></i>Field Hockey</a><a id="mnuHSSport_Football" href="/SportPages/SportPageInfo.aspx?TournamentID=3&amp;L=1"><i class="fa-duotone fa-football"></i>Football</a><a id="mnuHSSport_Golf (Coed)" href="/SportPages/SportPageInfo.aspx?TournamentID=4&amp;L=1"><i class="fa-duotone fa-golf-flag-hole"></i>Golf (Coed)</a><a id="mnuHSSport_Soccer (Boys)" href="/SportPages/SportPageInfo.aspx?TournamentID=5&amp;L=1"><i class="fa-duotone fa-futbol"></i>Soccer (Boys)</a><a id="mnuHSSport_Soccer (Girls)" href="/SportPages/SportPageInfo.aspx?TournamentID=6&amp;L=1"><i class="fa-duotone fa-futbol"></i>Soccer (Girls)</a><a id="mnuHSSport_Soccer (Unified)" href="/SportPages/SportPageInfo.aspx?TournamentID=1007&amp;L=1"><i class="fa-duotone fa-futbol"></i>Soccer (Unified)</a><a id="mnuHSSport_Volleyball (Girls)" href="/SportPages/SportPageInfo.aspx?TournamentID=8&amp;L=1"><i class="fa-duotone fa-volleyball"></i>Volleyball (Girls)</a><a id="mnuHSSport_Volleyball (Unified)" href="/SportPages/SportPageInfo.aspx?TournamentID=303&amp;L=1"><i class="fa-duotone fa-volleyball"></i>Volleyball (Unified)</a><b>HS - Winter</b><br/><a id="mnuHSSport_Alpine Ski (Boys)" href="/SportPages/SportPageInfo.aspx?TournamentID=403&amp;L=1"><i class="fa-solid fa-person-skiing"></i>Alpine Ski (Boys)</a><a id="mnuHSSport_Alpine Ski (Girls)" href="/SportPages/SportPageInfo.aspx?TournamentID=404&amp;L=1"><i class="fa-solid fa-person-skiing"></i>Alpine Ski (Girls)</a><a id="mnuHSSport_Basketball (Unified)" href="/SportPages/SportPageInfo.aspx?TournamentID=302&amp;L=1"><i class="fa-duotone fa-basketball-hoop"></i>Basketball (Unified)</a><a id="mnuHSSport_Boys Basketball" href="/SportPages/SportPageInfo.aspx?TournamentID=100&amp;L=1"><i class="fa-duotone fa-basketball-hoop"></i>Boys Basketball</a><a id="mnuHSSport_Cheerleading Winter (Coed)" href="/SportPages/SportPageInfo.aspx?TournamentID=1005&amp;L=1"><i class="fa-solid fa-child-reaching"></i>Cheerleading Winter (Coed)</a><a id="mnuHSSport_Drama (Coed)" href="/SportPages/SportPageInfo.aspx?TournamentID=1010&amp;L=1">Drama (Coed)</a><a id="mnuHSSport_Girls Basketball" href="/SportPages/SportPageInfo.aspx?TournamentID=101&amp;L=1"><i class="fa-duotone fa-basketball-hoop"></i>Girls Basketball</a><a id="mnuHSSport_Ice Hockey (Boys)" href="/SportPages/SportPageInfo.aspx?TournamentID=103&amp;L=1"><i class="fa-duotone fa-hockey-sticks"></i>Ice Hockey (Boys)</a><a id="mnuHSSport_Ice Hockey (Girls)" href="/SportPages/SportPageInfo.aspx?TournamentID=108&amp;L=1"><i class="fa-duotone fa-hockey-sticks"></i>Ice Hockey (Girls)</a><a id="mnuHSSport_Indoor Track (Boys)" href="/SportPages/SportPageInfo.aspx?TournamentID=104&amp;L=1"><i class="fa-solid fa-person-running"></i>Indoor Track (Boys)</a><a id="mnuHSSport_Indoor Track (Girls)" href="/SportPages/SportPageInfo.aspx?TournamentID=300&amp;L=1"><i class="fa-solid fa-person-running"></i>Indoor Track (Girls)</a><a id="mnuHSSport_Nordic Ski" href="/SportPages/SportPageInfo.aspx?TournamentID=402&amp;L=1"><i class="fa-solid fa-person-skiing"></i>Nordic Ski</a><a id="mnuHSSport_Swimming (Boys)" href="/SportPages/SportPageInfo.aspx?TournamentID=105&amp;L=1"><i class="fa-duotone fa-person-swimming"></i>Swimming (Boys)</a><a id="mnuHSSport_Swimming (Girls)" href="/SportPages/SportPageInfo.aspx?TournamentID=7&amp;L=1"><i class="fa-duotone fa-person-swimming"></i>Swimming (Girls)</a><a id="mnuHSSport_Wrestling" href="/SportPages/SportPageInfo.aspx?TournamentID=106&amp;L=1"><i class="fa-duotone fa-hands"></i>Wrestling</a><b>HS - Spring</b><br/><a id="mnuHSSport_Baseball" href="/SportPages/SportPageInfo.aspx?TournamentID=200&amp;L=1"><i class="fa-duotone fa-baseball"></i>Baseball</a><a id="mnuHSSport_Bocce (Unified)" href="/SportPages/SportPageInfo.aspx?TournamentID=400&amp;L=1">Bocce (Unified)</a><a id="mnuHSSport_ESports Spring" href="/SportPages/SportPageInfo.aspx?TournamentID=1003&amp;L=1">ESports Spring</a><a id="mnuHSSport_Lacrosse (Boys)" href="/SportPages/SportPageInfo.aspx?TournamentID=202&amp;L=1"><i class="fa-duotone fa-lacrosse-stick-ball"></i>Lacrosse (Boys)</a><a id="mnuHSSport_Lacrosse (Girls)" href="/SportPages/SportPageInfo.aspx?TournamentID=203&amp;L=1"><i class="fa-duotone fa-lacrosse-stick-ball"></i>Lacrosse (Girls)</a><a id="mnuHSSport_Outdoor Track (Boys)" href="/SportPages/SportPageInfo.aspx?TournamentID=204&amp;L=1"><i class="fa-solid fa-person-running"></i>Outdoor Track (Boys)</a><a id="mnuHSSport_Outdoor Track (Girls)" href="/SportPages/SportPageInfo.aspx?TournamentID=301&amp;L=1"><i class="fa-solid fa-person-running"></i>Outdoor Track (Girls)</a><a id="mnuHSSport_Softball" href="/SportPages/SportPageInfo.aspx?TournamentID=205&amp;L=1"><i class="fa-duotone fa-baseball-bat-ball"></i>Softball</a><a id="mnuHSSport_Tennis (Boys)" href="/SportPages/SportPageInfo.aspx?TournamentID=206&amp;L=1"><i class="fa-duotone fa-racquet"></i>Tennis (Boys)</a><a id="mnuHSSport_Tennis (Girls)" href="/SportPages/SportPageInfo.aspx?TournamentID=207&amp;L=1"><i class="fa-duotone fa-racquet"></i>Tennis (Girls)</a>
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
				<h2 class="dsFlexBlockFill SportTitle">HS Outdoor Track (Boys)</h2><div class="ImportantWrapper">
					<div class="dsFlexBlock50 ImportantBlock">
						<h2>Important Dates</h2><div class="ImportantBlockContent">
							<div class="SportInfoDates">
	<h2>2026-2027 Boys Outdoor Track Regular Season</h2>
	<span class="SportInfoTitle">First Practice:</span> March 29, 2027<br />
	<span class="SportInfoTitle"></span>
	<span class="SportInfoTitle">First Season Meet:</span> April 15, 2027<br />
</div><div class="SportInfoDates">
    <h2>2026-2027 Boys Outdoor Track Championships</h2>
    <span class="SportInfoTitle"></span>

    <span class="SportInfoTitle">State Championship Meets: June 5, 2027<br /></span>Class A @ TBD<br /> Class B @ TBD<br />Class C @ TBD<br />
    <span class="SportInfoTitle">New England Championship Meet:</span> June 12 @ TBD<br />

</div>
						</div>
					</div><div class="dsFlexBlock50 ImportantBlock">
						<h2>Important Links</h2><div class="ImportantBlockContent">
							<ul><li><a id="mnuSubResources_48f3c90b-0e38-4fbc-94f2-2737ac692861" href="/resources/Tournament%20Info\Outdoor%20Track%20Boys/Bulletin%202025-26.pdf" target="_blank"><i class='fa-duotone fa-file-circle-info'></i>Bulletin 2025-26</a></li><li><a id="mnuSubResources_35fdda73-1a75-447e-ae84-90ac00e70a97" href="/resources/Tournament%20Info\Outdoor%20Track%20Boys/Class%20A%20Meet%20Program.pdf" target="_blank"><i class='fa-duotone fa-file-circle-info'></i>Class A Meet Program</a></li><li><a id="mnuSubResources_3e099be6-5229-4a8d-aae4-6d2d7365c8ba" href="/resources/Tournament%20Info\Outdoor%20Track%20Boys/Outdoor%20Track%20New%20England%20Championship%20Packet%202026.pdf" target="_blank"><i class='fa-duotone fa-file-circle-info'></i>Outdoor Track New England Championship Packet 2026</a></li><li><a id="mnuSubResources_34ccf3ef-db63-4a76-83ac-1fb3b5eed364" href="/resources/Tournament%20Info\Outdoor%20Track%20Boys/Outdoor%20Track%20Pole%20Vault%20Form.pdf" target="_blank"><i class='fa-duotone fa-file-circle-info'></i>Outdoor Track Pole Vault Form</a></li><li><a id="mnuSubResources_41b0d9ba-69bb-4140-bc35-0692ae71cd2e" href="/resources/Tournament%20Info\Outdoor%20Track%20Boys/Outdoor%20Track%20State%20Records%20Boys.pdf" target="_blank"><i class='fa-duotone fa-file-circle-info'></i>Outdoor Track State Records Boys</a></li></ul>
						</div>
					</div>
				</div><div class="InnerPageAdWrapper">
					
<style>

	#homepageads2 {
		margin-bottom: 40px;
	}

        #homepageads1,
	#homepageads2 {
	    float: left;
	    width: 50% !important;        
	    line-height: 1;
        }

        #homepageads1 img,
	#homepageads2 img {
            height: auto;
	    width: 100%;
        }

	/* applied in code */
	.InnerPageAdWrapper {
	    display: flex;
	    flex-wrap: wrap;
    	    padding-top: 20px; 
	}




 @media (max-width: 800px) {
    #homepageads1,    
    #homepageads2 {
        width: 100% !important;
    }

    #homepageads1 img {
        margin-bottom:20px;
    }
 }

    
</style>

<script>
    $(document).ready(function () {

        $('span', $('#homepageads1pool')).sort(function () { return (Math.round(Math.random()) - 0.5) }).appendTo($('#homepageads1pool'));

        //for (let i = 0; i < $('#homepageads1pool').children().length; i += 1) {
        //    $('#homepageads1').append('<div>' + $('#homepageads1pool').children()[i].outerHTML + '</div>');
        //}

	$('#homepageads1').append('<div>' + $('#homepageads1pool').children()[0].outerHTML + '</div>');
	$('#homepageads2').append('<div>' + $('#homepageads1pool').children()[1].outerHTML + '</div>');

       
    });
</script>

<div id="homepageads1">

</div>

<div id="homepageads2">

</div>

<div id="homepageads1pool" style="display:none">
    <span class='CarouselTracker' CarouselID='7' Carousel='Homepage Ads' CarouselEntryID='1125' CarouselEntry='ANG - Maine'><a  target='_blank' href=' https://www.airforce.com/ways-to-serve/air-national-guard/maine'><img src='\Resources\Carousel\Home Ads\Images\0000001125.png'></a></span>
<span class='CarouselTracker' CarouselID='7' Carousel='Homepage Ads' CarouselEntryID='1121' CarouselEntry='988 - Maine'><a  target='_blank' href='https://988maine.com/'><img src='\Resources\Carousel\Home Ads\Images\0000001121.png'></a></span>
<span class='CarouselTracker' CarouselID='7' Carousel='Homepage Ads' CarouselEntryID='1112' CarouselEntry='Measles'><a  target='_blank' href='https://www.maine.gov/dhhs/measles'><img src='\Resources\Carousel\Home Ads\Images\0000001112.png'></a></span>
<span class='CarouselTracker' CarouselID='7' Carousel='Homepage Ads' CarouselEntryID='1099' CarouselEntry='Army ROTC'><a  target='_blank' href='https://www.goarmy.com/info?iom=CQTG-XX_LAP_Exhibit_2932408_C1B_NA_ROTC'><img src='\Resources\Carousel\Home Ads\Images\0000001099.png'></a></span>
<span class='CarouselTracker' CarouselID='7' Carousel='Homepage Ads' CarouselEntryID='1061' CarouselEntry='Hammond'><a  target='_blank' href='https://www.hammondlumber.com/'><img src='\Resources\Carousel\Home Ads\Images\0000001061.png'></a></span>
<span class='CarouselTracker' CarouselID='7' Carousel='Homepage Ads' CarouselEntryID='1065' CarouselEntry='Maine Savings'><a  target='_blank' href='https://mainesavings.com/'><img src='\Resources\Carousel\Home Ads\Images\0000001065.png'></a></span>
<span class='CarouselTracker' CarouselID='7' Carousel='Homepage Ads' CarouselEntryID='1093' CarouselEntry='Finance Authority of Maine'><a  target='_blank' href='https://www.famemaine.com/levelup/'><img src='\Resources\Carousel\Home Ads\Images\0000001093.png'></a></span>
<span class='CarouselTracker' CarouselID='7' Carousel='Homepage Ads' CarouselEntryID='1063' CarouselEntry='Just Drive'><a  target='_blank' href='https://www.maine.gov/dps/bhs/ '><img src='\Resources\Carousel\Home Ads\Images\0000001063.png'></a></span>
<span class='CarouselTracker' CarouselID='7' Carousel='Homepage Ads' CarouselEntryID='1059' CarouselEntry='ConvenientMD'><a  target='_blank' href='https://convenientmd.com/'><img src='\Resources\Carousel\Home Ads\Images\0000001059.png'></a></span>

</div>





				</div>
			</div><div class="dsFlexBlock20">
				<div id='DashboardSideAds' class='DashboardAds'><a target='_self' href='/resources/Tournament%20Info/Outdoor Track Boys/Bulletin 2025-26.pdf'><img src='/resources/_media/DashboardAds/Information Packet.png' /></a><a target='_self' href='/DashboardSchedule.aspx?L=1&SportID=2_1029_-1'><img src=' /resources/_media/DashboardAds/Todays Games.png' /></a><a target='_blank' href='https://gofan.co/app/school/Maine Principal's Association'><img src=' /resources/_media/DashboardAds/Go Fan Tickets.png' ></a></div>
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
    <span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1047' CarouselEntry='Hammond Lumber Comapny'><a target='_blank' href='https://www.hammondlumber.com/'><img src='\Resources\Carousel\Partner\Images\0000001047.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1043' CarouselEntry='Maine Savings'><a target='_blank' href='https://www.mainesavings.com/'><img src='\Resources\Carousel\Partner\Images\0000001043.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1130' CarouselEntry='ROTC'><a target='_blank' href='https://www.goarmy.com/info?iom=CQTG-XX_LAP_Exhibit_2932408_C1B_NA_ROTC'><img src='\Resources\Carousel\Partner\Images\0000001130.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1033' CarouselEntry='ConvenientMD'><a target='_blank' href='https://convenientmd.com/'><img src='\Resources\Carousel\Partner\Images\0000001033.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1095' CarouselEntry='FAME'><a target='_blank' href='https://www.famemaine.com/levelup/'><img src='\Resources\Carousel\Partner\Images\0000001095.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1079' CarouselEntry='Just Drive'><a target='_blank' href='https://www.maine.gov/dps/bhs/ '><img src='\Resources\Carousel\Partner\Images\0000001079.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1053' CarouselEntry='Maine Golf'><a target='_blank' href='https://www.mainegolf.org/'><img src='\Resources\Carousel\Partner\Images\0000001053.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1041' CarouselEntry='Gosline Retirement Planning'><a target='_blank' href='https://www.grplans.com/'><img src='\Resources\Carousel\Partner\Images\0000001041.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1049' CarouselEntry='American Heart Association'><a target='_blank' href='https://cpr.heart.org/en/training-programs/cardiac-emergency-response-plan-cerp'><img src='\Resources\Carousel\Partner\Images\0000001049.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1051' CarouselEntry='Diadem'><a target='_blank' href='https://diademsports.com/'><img src='\Resources\Carousel\Partner\Images\0000001051.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1039' CarouselEntry='Presentation Systems'><a target='_blank' href='https://www.presentationsys.com/ecolor-poster-printer-system'><img src='\Resources\Carousel\Partner\Images\0000001039.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1027' CarouselEntry='MaxPreps'><a target='_blank' href='https://www.maxpreps.com/photography/browse-galleries?state=me'><img src='\Resources\Carousel\Partner\Images\0000001027.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1025' CarouselEntry='Mile Split'><a target='_blank' href='https://me.milesplit.com/'><img src='\Resources\Carousel\Partner\Images\0000001025.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1023' CarouselEntry='Neptune'><a target='_blank' href='https://neptunenow.com/'><img src='\Resources\Carousel\Partner\Images\0000001023.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1017' CarouselEntry='Dudley'><a target='_blank' href='https://www.mpa.cc/page/spalding.com/dudley'><img src='\Resources\Carousel\Partner\Images\0000001017.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1015' CarouselEntry='Mikasa'><a target='_blank' href='https://mikasasports.com/product-category/indoor-volleyball/'><img src='\Resources\Carousel\Partner\Images\0000001015.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1013' CarouselEntry='Penn Monto'><a target='_blank' href='https://penn-monto.com'><img src='\Resources\Carousel\Partner\Images\0000001013.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1011' CarouselEntry='Rawlings'><a target='_blank' href='https://rawlings.com'><img src='\Resources\Carousel\Partner\Images\0000001011.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1009' CarouselEntry='Select'><a target='_blank' href='https://www.mpa.cc/page/select-sport.com'><img src='\Resources\Carousel\Partner\Images\0000001009.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1007' CarouselEntry='Spalding'><a target='_blank' href='https://www.mpa.cc/page/spalding.com'><img src='\Resources\Carousel\Partner\Images\0000001007.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1005' CarouselEntry='Wilson'><a target='_blank' href='https://www.mpa.cc/page/wilson.com'><img src='\Resources\Carousel\Partner\Images\0000001005.png'  /></a></span>

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
