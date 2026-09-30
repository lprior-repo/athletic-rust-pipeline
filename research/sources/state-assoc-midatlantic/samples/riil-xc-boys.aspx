

<!DOCTYPE html>

<html lang="en">
<head><meta name='description' content='The Rhode Island Interscholastic League is an organization that runs and regulates interscholastic high school activities in the U.S. state of Rhode Island. A total of 54 public and private schools participate in the league and about 20,000 students annually compete in RIIL sanctioned events.'/><link href="/_Styles/2000/Config.css?v=20260921c" rel="stylesheet" type="text/css" /><link href="/_Styles/Controls.css?v=20260921c" rel="stylesheet" type="text/css" /><link href="/_Styles/Dashboard.css?v=20260921c" rel="stylesheet" type="text/css" /><link href="/_Styles/TournamentCentral.css?v=20260921c" rel="stylesheet" type="text/css" /><meta charset="utf-8" /><title>
	FusionPoint Sports - Rhode Island Interscholastic League
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
    <form method="post" action="./SportPageInfo.aspx?TournamentID=1" id="MainForm">
<div class="aspNetHidden">
<input type="hidden" name="__VIEWSTATE" id="__VIEWSTATE" value="QSLM9iWUFZOJG0Z9zRA/gRnTUlQqKXI/XjMxLJi5/8Mp+d2IN7bkuyl9VbhoRzDYtO2VekEuYZGEulwrfmcSTEIrSsZLJ1udaCm/cc01iblqoSf9P9NwmpPfmcDO5WuFFOHyEmd2DlmGCTiroS/iZr/V/x8ieNr4wXQEywxt9B9ShbXIm69k0wb9TVOeVAJRqnu/Yg+XjKkJ77tmAVGQmj72gVWSXTRYJYDC7Zu1/HuciA49nqMinHtT6W1cUpEocWfGjE9VG0dv6dRzXZM1Q/L8skIcZUDfRVYPnQAz9U0XPVA704q8RD1BA6K4OgVQ8Wwi4K5JneUGraBrQBN43lnFWAkP6GVz6Ww4+ZDmU70fPY/30j3jUAEt765wR4LuktEz32yAyzoFMCF3Q+d5+i/6faoUKSHqbrBSEw36Ng1e7Y0G6FeqEAXluMauuHUAdKSF/4Hee32/Y+/6xiz11P4S/RtSIXmh5mfoNfq3vAxunDbMQ+VRg2PQxDFk0ZduxqavFtTU/iUM0wwSAWBvkI90RGYOcWjvM45RNvjXy/yQIB9CAKwYmvurOaJ6NvRpIpWsckbPpMR1dUxpo65IcBUYl0+3di1vxQRqU8ezj8jgeO6Rc9l7/02ngcu+zv75L+bSxDO4ieG8iwqJ46W4xWPFevZQy8Ek2gqQxGJhJqN6h+/lBwfS7tcml4fh3/EeCbbHVOWFV4jt4MjcuyQSx6161zWyPR1WgfXVfi4MJ3ostdgjCcHdNlo/zch9OOOjrX9rlAjaLjUpSmlF5ys2RejVTVMhJNO5235ENLGVyvzkGAj0TXH4K7p2pej5YOUpXrP43ouOH2yvU+4w1YNWboQlBZDktRAcWEQo79xbhhkcfmIkGOXKZiGOzewQGN3XwxUaM3xm7plViaY8qRxNXms3s/eaQPtdDmu8F2vs8y/RqJc5m+ShDD0nbybR0TPcjc3pzti0SlAON9UV4R/EzxWK3t/tepDGK4qTkUm96ubQly3YRX4vedwPK5z1CDr3HW1FzEcbdM2akoL/yHwxbxrgn9jjr4afVkK0oowoPM8vSYaDIFmh4imOnVSrScLd8jIr3pgf3JlWVqZbq3pQJfIU24qm0F201NNOa/Ign9JYxB6rFXOVFFhnlBp4e6RbFR8A9jFT502QNb0OJ4qoJz7XPUybmuKfulouhvTvo3FqGghp/ZcgRwJspCt0GFbKNiwRiL4Omrzgmag+nbfCgyedsfxFa5QmNUc9AlTJjypcpeGTRep9pXgQ+fWNiEohsc4J+Zc2QfEJtAl8sfCKjZzuU7Gdyq2t1CtxGpiGBoiTbE4vHtedNiBhYbAniEN+LNYKQ6tDe0GZWqO146eyuZAnuDMUqt9uUmVMhfjM9slJIrOqFoKQdTvFssyhRdvE20Ox2AonIsOiDJ+m+EkjiD1UdX8Xxb9ypJdmEnyW1As=" />
</div>

<div class="aspNetHidden">

	<input type="hidden" name="__VIEWSTATEGENERATOR" id="__VIEWSTATEGENERATOR" value="A370CA34" />
</div>
        <div id="dsLoader"></div>

        <div id="dsNavigation" class="dsNavigationOverlay">
	
            <div id="dsNavigationMenu" class="dsNavigationMenu">
                <a href="javascript:void(0)" class="dsNavigationCloseButton" onclick="closeNav()">×</a>
                <div id="dsNavigationMenuContent" class="dsNavigationMenuContent">
		
                    <ul id="dsNavigationMenuUL" class="dsNavigationMenuUL dsNavigationMenuSubPanel active"><li><a id="mnuDashboard" href="\">Dashboard</a></li><li><a id="mnuDirectory" href="\Directory.aspx">School Directory</a></li><li><a id="mnuSportHS" class=" dsNavigationMenuLink " href="https://www.riil.org/SportPages/SportPageInfo.aspx?TournamentID=1#NavigationMenu_mnuSportHS"><div style="float:left;">Sport Info - High School</div><i class="fa fa-chevron-right" style="float:right;"></i><div style="clear:both;"></div></a></li><li><a id="mnuSportMS" class=" dsNavigationMenuLink " href="https://www.riil.org/SportPages/SportPageInfo.aspx?TournamentID=1#NavigationMenu_mnuSportMS"><div style="float:left;">Sport Info - Middle School</div><i class="fa fa-chevron-right" style="float:right;"></i><div style="clear:both;"></div></a></li><li><a id="mnuScheduleHS" href="\DashboardSchedule.aspx">Schedules</a></li><li><a id="mnuMasterSchedule" href="/MasterSchedule.aspx?TeamLevelID=5">Master Schedule</a></li><li><a id="mnuSchoolRoster" href="/DashboardTeamRoster.aspx?SeasonRoster=1">Team Rosters</a></li><li><a id="mnuSchoolPages" href="/SchoolPages/School.aspx">School Pages</a></li><li class="dsNavigationMenuGroupSeperator"></li><li><div class="dsNavigationMenuGroupTitle">Tournament Central</div></li><li><a id="mnuTournamentDashboard" href="\DashboardTournamentCentral.aspx">Dashboard</a></li><li><a id="mnuTournamentRankings" href="\SportPages\SportPageRanking.aspx">Tournament Standings</a></li><li><a id="mnuTournamentRPIDetail" href="\Reports\RPIDetail.aspx">RPI Detail</a></li><li><a id="mnuTournamentBrackets" href="\TournamentCentralBrackets.aspx">Playoff Brackets</a></li><li class="dsNavigationMenuGroupSeperator"></li><li><div class="dsNavigationMenuGroupTitle">Inside the RIIL</div></li><li><a id="mnuSubResources_InsidetheRIIL_AnnualReports" class=" dsNavigationMenuLink " href="https://www.riil.org/SportPages/SportPageInfo.aspx?TournamentID=1#NavigationMenu_mnuSubResources_InsidetheRIIL_AnnualReports"><div style="float:left;">Annual Reports</div><i class="fa fa-chevron-right" style="float:right;"></i><div style="clear:both;"></div></a></li><li><a id="mnuSubResources_69d3a678-ae15-418d-9157-233088cadf97" href="/TenantHTML.aspx?D=Inside the RIIL&amp;F=Contact Us.html">Contact Us</a></li><li><a id="mnuSubResources_fce32c12-570e-436a-beae-d123873036d9" href="/TenantHTML.aspx?D=Inside the RIIL&amp;F=Our Mission.html">Our Mission</a></li><li><a id="mnuSubResources_62cbf554-9088-45f6-be67-99666ecfd07c" href="/TenantHTML.aspx?D=Inside the RIIL&amp;F=Principals Committee On Athletics.html">Principals Committee On Athletics</a></li><li><a id="mnuSubResources_47d6237d-f6d7-4d37-85e7-d6c80475a83c" href="/TenantHTML.aspx?D=Inside the RIIL&amp;F=Office Staff.html">Office Staff</a></li><li><a id="mnuSubResources_e9012d8a-9e0d-4a7f-b573-0bde89836c55" href="https://www.tumblr.com/riilsports" target="_blank">RIIL Blog</a></li><li><a id="mnuSubResources_InsidetheRIIL_HSAthleticHallofFame" class=" dsNavigationMenuLink " href="https://www.riil.org/SportPages/SportPageInfo.aspx?TournamentID=1#NavigationMenu_mnuSubResources_InsidetheRIIL_HSAthleticHallofFame"><div style="float:left;">HS Athletic Hall of Fame</div><i class="fa fa-chevron-right" style="float:right;"></i><div style="clear:both;"></div></a></li><li><a id="mnuSubResources_b9ed8157-6b91-497f-8329-9e0e6b8a309d" href="/resources/Inside%20the%20RIIL/RIIL%20Middle%20School%206-8%20Manual.pdf" target="_blank">RIIL Middle School 6-8 Manual</a></li><li><a id="mnuSubResources_f9249ffe-b64f-4010-b836-397a9afb166f" href="/resources/Inside%20the%20RIIL/RIIL%20Rules%20&amp;%20Regulations%20-%20Articles%201-15.pdf" target="_blank">RIIL Rules & Regulations - Articles 1-15</a></li><li><a id="mnuSubResources_8ac8cffc-5ead-4621-ad3c-c6aa8181d904" href="/CarouselAll.aspx?CarouselID=9" target="_blank">Sponsors</a></li><li class="dsNavigationMenuGroupSeperator"></li><li><div class="dsNavigationMenuGroupTitle">Resources</div></li><li><a id="mnuSubResources_147e04f0-3936-46e8-ad0f-44f91659cc1a" href="/resources/Resources/2026-2027%20Media%20Information.pdf" target="_blank">2026-2027 Media Information</a></li><li><a id="mnuSubResources_96e0a886-fcd2-4dcc-9487-3b3306456e36" href="/TenantHTML.aspx?D=Resources&amp;F=Affiliate Websites.html">Affiliate Websites</a></li><li><a id="mnuSubResources_d5ac0135-83f6-4c19-a17a-fa95fb5c8911" href="/publicdownloads.aspx">Forms & Document Downloads</a></li><li><a id="mnuSubResources_b208954b-5751-4f72-962a-4bcc1cdb5f03" href="https://highschoolofficials.com/" target="_blank">NFHS Become an Official!</a></li><li><a id="mnuSubResources_ae484776-fc2c-4ddd-96c2-1a8633788cbe" href="/TenantHTML.aspx?D=Resources&amp;F=Officials Courses through RefReps.html">Officials Courses through RefReps</a></li><li><a id="mnuSubResources_1580a7fb-ab77-4b28-b633-55c17d815571" href="/TenantHTML.aspx?D=Resources&amp;F=Officials Registration in Arbiter.html">Officials Registration in Arbiter</a></li><li><a id="mnuSubResources_a1a474be-9426-4b4c-b017-155f8a5c8ff6" href="/TenantHTML.aspx?D=Resources&amp;F=Sportsmanship Expectations.html">Sportsmanship Expectations</a></li><li class="dsNavigationMenuGroupSeperator"></li><li><div class="dsNavigationMenuGroupTitle">Students and Parents</div></li><li><a id="mnuSubResources_StudentsandParents_OperationCleanCompetition" class=" dsNavigationMenuLink " href="https://www.riil.org/SportPages/SportPageInfo.aspx?TournamentID=1#NavigationMenu_mnuSubResources_StudentsandParents_OperationCleanCompetition"><div style="float:left;">Operation Clean Competition</div><i class="fa fa-chevron-right" style="float:right;"></i><div style="clear:both;"></div></a></li><li><a id="mnuSubResources_StudentsandParents_StudentInitiatives" class=" dsNavigationMenuLink " href="https://www.riil.org/SportPages/SportPageInfo.aspx?TournamentID=1#NavigationMenu_mnuSubResources_StudentsandParents_StudentInitiatives"><div style="float:left;">Student Initiatives</div><i class="fa fa-chevron-right" style="float:right;"></i><div style="clear:both;"></div></a></li><li><a id="mnuSubResources_StudentsandParents_TrafficSafetyIsATeamSport" class=" dsNavigationMenuLink " href="https://www.riil.org/SportPages/SportPageInfo.aspx?TournamentID=1#NavigationMenu_mnuSubResources_StudentsandParents_TrafficSafetyIsATeamSport"><div style="float:left;">Traffic Safety Is A Team Sport</div><i class="fa fa-chevron-right" style="float:right;"></i><div style="clear:both;"></div></a></li><li><a id="mnuSubResources_481d4698-3ad0-458c-a2ec-b61798b942c1" href="/TenantHTML.aspx?D=Students and Parents&amp;F=Alice Sullivan Memorial Scholarship.html">Alice Sullivan Memorial Scholarship</a></li><li><a id="mnuSubResources_32a1eb80-4496-47ad-866f-289c9c82a81a" href="/resources/Students%20and%20Parents/Local%2051%20Trades%20Scholarship.pdf" target="_blank">Local 51 Trades Scholarship</a></li><li><a id="mnuSubResources_1773b511-fd50-4462-a890-fd82123774d3" href="/resources/Students%20and%20Parents/RI%20Council%20on%20Problem%20Gambling%20Corner.pdf" target="_blank">RI Council on Problem Gambling Corner</a></li><li class="dsNavigationMenuGroupSeperator"></li><li><div class="dsNavigationMenuGroupTitle">Account</div></li><li><a id="mnuLogin" href="../Login.aspx?ReturnUrl=%2fSportPages%2fSportPageInfo.aspx%3fTournamentID%3d1">Login</a></li><li><a id="mnuSignUp" href="../SignUp.aspx?ReturnUrl=%2fSportPages%2fSportPageInfo.aspx%3fTournamentID%3d1">Sign-Up</a></li><li><a id="mnuPasswordReset" href="\PasswordReset.aspx">Forgot Password</a></li></ul>
                    <div style="display: none">
                        <ul id="dsNavigationMenuHidden" class="dsNavigationMenuUL dsNavigationMenuSubPane"></ul>
                    </div>
                <ul id="NavigationMenu_mnuSportHS" class="dsNavigationMenuUL dsNavigationMenuSubPanel"><li><div class="dsNavigationSubMenuTitle">Sport Info - High School</div></li><li><a id="SUB_MAIN_mnuSportHS" class=" dsNavigationMenuLink " onclick="changeToMainMenu();"><i class="fa fa-arrow-left" style="float:left;"></i><div style="float:left;" class="dsNavigationMenuBackButton">MAIN MENU</div><div style="clear:both;"></div></a></li><li class="dsNavigationMenuGroupSeperator"></li><li><div class="dsNavigationMenuGroupTitle">Fall</div></li><li><a id="mnuSportButtonHSCross Country - Boys" href="\SportPages\SportPageInfo.aspx?TournamentID=1">Cross Country - Boys</a></li><li><a id="mnuSportButtonHSCross Country - Girls" href="\SportPages\SportPageInfo.aspx?TournamentID=310">Cross Country - Girls</a></li><li><a id="mnuSportButtonHSField Hockey" href="\SportPages\SportPageInfo.aspx?TournamentID=2">Field Hockey</a></li><li><a id="mnuSportButtonHSFootball" href="\SportPages\SportPageInfo.aspx?TournamentID=3">Football</a></li><li><a id="mnuSportButtonHSGame Day Cheerleading" href="\SportPages\SportPageInfo.aspx?TournamentID=1003">Game Day Cheerleading</a></li><li><a id="mnuSportButtonHSSoccer - Boys" href="\SportPages\SportPageInfo.aspx?TournamentID=5">Soccer - Boys</a></li><li><a id="mnuSportButtonHSSoccer - Girls" href="\SportPages\SportPageInfo.aspx?TournamentID=6">Soccer - Girls</a></li><li><a id="mnuSportButtonHSTennis - Girls" href="\SportPages\SportPageInfo.aspx?TournamentID=207">Tennis - Girls</a></li><li><a id="mnuSportButtonHSVolleyball - Girls" href="\SportPages\SportPageInfo.aspx?TournamentID=8">Volleyball - Girls</a></li><li><a id="mnuSportButtonHSVolleyball - Unified" href="\SportPages\SportPageInfo.aspx?TournamentID=303">Volleyball - Unified</a></li><li class="dsNavigationMenuGroupSeperator"></li><li><div class="dsNavigationMenuGroupTitle">Winter</div></li><li><a id="mnuSportButtonHSBasketball - Boys" href="\SportPages\SportPageInfo.aspx?TournamentID=100">Basketball - Boys</a></li><li><a id="mnuSportButtonHSBasketball - Girls" href="\SportPages\SportPageInfo.aspx?TournamentID=101">Basketball - Girls</a></li><li><a id="mnuSportButtonHSCompetition Cheerleading " href="\SportPages\SportPageInfo.aspx?TournamentID=107">Competition Cheerleading </a></li><li><a id="mnuSportButtonHSESports" href="\SportPages\SportPageInfo.aspx?TournamentID=304">ESports</a></li><li><a id="mnuSportButtonHSGymnastics" href="\SportPages\SportPageInfo.aspx?TournamentID=102">Gymnastics</a></li><li><a id="mnuSportButtonHSIce Hockey - Boys" href="\SportPages\SportPageInfo.aspx?TournamentID=103">Ice Hockey - Boys</a></li><li><a id="mnuSportButtonHSIce Hockey - Girls" href="\SportPages\SportPageInfo.aspx?TournamentID=108">Ice Hockey - Girls</a></li><li><a id="mnuSportButtonHSIndoor Track - Boys" href="\SportPages\SportPageInfo.aspx?TournamentID=104">Indoor Track - Boys</a></li><li><a id="mnuSportButtonHSIndoor Track - Girls" href="\SportPages\SportPageInfo.aspx?TournamentID=1024">Indoor Track - Girls</a></li><li><a id="mnuSportButtonHSSwimming - Boys" href="\SportPages\SportPageInfo.aspx?TournamentID=105">Swimming - Boys</a></li><li><a id="mnuSportButtonHSSwimming - Girls" href="\SportPages\SportPageInfo.aspx?TournamentID=7">Swimming - Girls</a></li><li><a id="mnuSportButtonHSWrestling - Coed" href="\SportPages\SportPageInfo.aspx?TournamentID=106">Wrestling - Coed</a></li><li class="dsNavigationMenuGroupSeperator"></li><li><div class="dsNavigationMenuGroupTitle">Spring</div></li><li><a id="mnuSportButtonHSBaseball" href="\SportPages\SportPageInfo.aspx?TournamentID=200">Baseball</a></li><li><a id="mnuSportButtonHSBasketball - Unified" href="\SportPages\SportPageInfo.aspx?TournamentID=302">Basketball - Unified</a></li><li><a id="mnuSportButtonHSGolf - Coed" href="\SportPages\SportPageInfo.aspx?TournamentID=4">Golf - Coed</a></li><li><a id="mnuSportButtonHSLacrosse - Boys" href="\SportPages\SportPageInfo.aspx?TournamentID=202">Lacrosse - Boys</a></li><li><a id="mnuSportButtonHSLacrosse - Girls" href="\SportPages\SportPageInfo.aspx?TournamentID=203">Lacrosse - Girls</a></li><li><a id="mnuSportButtonHSOutdoor Track - Boys" href="\SportPages\SportPageInfo.aspx?TournamentID=204">Outdoor Track - Boys</a></li><li><a id="mnuSportButtonHSOutdoor Track - Girls" href="\SportPages\SportPageInfo.aspx?TournamentID=1021">Outdoor Track - Girls</a></li><li><a id="mnuSportButtonHSSoftball" href="\SportPages\SportPageInfo.aspx?TournamentID=205">Softball</a></li><li><a id="mnuSportButtonHSTennis - Boys" href="\SportPages\SportPageInfo.aspx?TournamentID=206">Tennis - Boys</a></li><li><a id="mnuSportButtonHSVolleyball - Boys" href="\SportPages\SportPageInfo.aspx?TournamentID=208">Volleyball - Boys</a></li></ul><ul id="NavigationMenu_mnuSportMS" class="dsNavigationMenuUL dsNavigationMenuSubPanel"><li><div class="dsNavigationSubMenuTitle">Sport Info - Middle School</div></li><li><a id="SUB_MAIN_mnuSportMS" class=" dsNavigationMenuLink " onclick="changeToMainMenu();"><i class="fa fa-arrow-left" style="float:left;"></i><div style="float:left;" class="dsNavigationMenuBackButton">MAIN MENU</div><div style="clear:both;"></div></a></li><li class="dsNavigationMenuGroupSeperator"></li><li><div class="dsNavigationMenuGroupTitle">Fall</div></li><li><a id="mnuSportButtonMSCross Country - Boys" href="\SportPages\SportPageInfo.aspx?L=2&amp;TournamentID=1001">Cross Country - Boys</a></li><li><a id="mnuSportButtonMSCross Country - Girls" href="\SportPages\SportPageInfo.aspx?L=2&amp;TournamentID=1313">Cross Country - Girls</a></li><li><a id="mnuSportButtonMSCross Country - Unified" href="\SportPages\SportPageInfo.aspx?L=2&amp;TournamentID=1314">Cross Country - Unified</a></li><li><a id="mnuSportButtonMSSoccer - Boys" href="\SportPages\SportPageInfo.aspx?L=2&amp;TournamentID=1005">Soccer - Boys</a></li><li><a id="mnuSportButtonMSSoccer - Girls" href="\SportPages\SportPageInfo.aspx?L=2&amp;TournamentID=1090">Soccer - Girls</a></li><li class="dsNavigationMenuGroupSeperator"></li><li><div class="dsNavigationMenuGroupTitle">Winter</div></li><li><a id="mnuSportButtonMSBoys Basketball" href="\SportPages\SportPageInfo.aspx?L=2&amp;TournamentID=1100">Boys Basketball</a></li><li><a id="mnuSportButtonMSGirls Basketball" href="\SportPages\SportPageInfo.aspx?L=2&amp;TournamentID=1101">Girls Basketball</a></li><li><a id="mnuSportButtonMSCheerleading Winter" href="\SportPages\SportPageInfo.aspx?L=2&amp;TournamentID=1317">Cheerleading Winter</a></li><li><a id="mnuSportButtonMSWrestling" href="\SportPages\SportPageInfo.aspx?L=2&amp;TournamentID=1106">Wrestling</a></li><li class="dsNavigationMenuGroupSeperator"></li><li><div class="dsNavigationMenuGroupTitle">Spring</div></li><li><a id="mnuSportButtonMSBaseball" href="\SportPages\SportPageInfo.aspx?L=2&amp;TournamentID=1200">Baseball</a></li><li><a id="mnuSportButtonMSBasketball - Unified" href="\SportPages\SportPageInfo.aspx?L=2&amp;TournamentID=1302">Basketball - Unified</a></li><li><a id="mnuSportButtonMSOutdoor Track - Boys" href="\SportPages\SportPageInfo.aspx?L=2&amp;TournamentID=1315">Outdoor Track - Boys</a></li><li><a id="mnuSportButtonMSOutdoor Track - Girls" href="\SportPages\SportPageInfo.aspx?L=2&amp;TournamentID=1316">Outdoor Track - Girls</a></li><li><a id="mnuSportButtonMSSoftball" href="\SportPages\SportPageInfo.aspx?L=2&amp;TournamentID=1205">Softball</a></li><li class="dsNavigationMenuGroupSeperator"></li><li><div class="dsNavigationMenuGroupTitle">Club</div></li><li><a id="mnuSportButtonMSClub Cheerleading" href="\SportPages\SportPageInfo.aspx?L=2&amp;TournamentID=1319">Club Cheerleading</a></li><li><a id="mnuSportButtonMSClub Flag Football" href="\SportPages\SportPageInfo.aspx?L=2&amp;TournamentID=1321">Club Flag Football</a></li><li><a id="mnuSportButtonMSClub Girls Volleyball" href="\SportPages\SportPageInfo.aspx?L=2&amp;TournamentID=1320">Club Girls Volleyball</a></li></ul><ul id="NavigationMenu_mnuSubResources_InsidetheRIIL_AnnualReports" class="dsNavigationMenuUL dsNavigationMenuSubPanel"><li><div class="dsNavigationSubMenuTitle">Annual Reports</div></li><li><a id="SUB_MAIN_mnuSubResources_InsidetheRIIL_AnnualReports" class=" dsNavigationMenuLink " onclick="changeToMainMenu();"><i class="fa fa-arrow-left" style="float:left;"></i><div style="float:left;" class="dsNavigationMenuBackButton">MAIN MENU</div><div style="clear:both;"></div></a></li><li><a id="mnuSubResources_4585f1f8-13cf-4b15-aed2-ceb4ea4fb22b" href="/resources/Inside%20the%20RIIL\Annual%20Reports/Annual%20Report%202004-05.pdf" target="_blank">Annual Report 2004-05</a></li><li><a id="mnuSubResources_51729798-a3c5-4d91-baf8-fa6527fa096b" href="/resources/Inside%20the%20RIIL\Annual%20Reports/Annual%20Report%202005-06.pdf" target="_blank">Annual Report 2005-06</a></li><li><a id="mnuSubResources_8eb31712-559c-415d-a1f7-e279d7f52cdc" href="/resources/Inside%20the%20RIIL\Annual%20Reports/Annual%20Report%202006-07.pdf" target="_blank">Annual Report 2006-07</a></li><li><a id="mnuSubResources_fd6734f8-1b20-4376-8d7d-91c87ef58a5a" href="/resources/Inside%20the%20RIIL\Annual%20Reports/Annual%20Report%202007-08.pdf" target="_blank">Annual Report 2007-08</a></li><li><a id="mnuSubResources_f6deca75-0a80-4645-9e36-16286dd1540b" href="/resources/Inside%20the%20RIIL\Annual%20Reports/Annual%20Report%202008-09.pdf" target="_blank">Annual Report 2008-09</a></li><li><a id="mnuSubResources_960d4c29-2621-4bb9-9a8b-36516501475f" href="/resources/Inside%20the%20RIIL\Annual%20Reports/Annual%20Report%202009-10.pdf" target="_blank">Annual Report 2009-10</a></li><li><a id="mnuSubResources_255336ca-ec5e-4c26-92eb-6c15635992e2" href="/resources/Inside%20the%20RIIL\Annual%20Reports/Annual%20Report%202010-11.pdf" target="_blank">Annual Report 2010-11</a></li><li><a id="mnuSubResources_a91c60c4-1d28-47c9-8db4-0d2f6b1083c1" href="/resources/Inside%20the%20RIIL\Annual%20Reports/Annual%20Report%202011-12.pdf" target="_blank">Annual Report 2011-12</a></li><li><a id="mnuSubResources_c3881efa-a056-4ed5-9b6f-73e42f5faf4c" href="/resources/Inside%20the%20RIIL\Annual%20Reports/Annual%20Report%202012-13.pdf" target="_blank">Annual Report 2012-13</a></li><li><a id="mnuSubResources_7fc75210-9a0f-4faf-9648-898896f2df7b" href="/resources/Inside%20the%20RIIL\Annual%20Reports/Annual%20Report%202013-14.pdf" target="_blank">Annual Report 2013-14</a></li><li><a id="mnuSubResources_aa6a2d9b-5385-46b0-a025-d323ccbfaf08" href="/resources/Inside%20the%20RIIL\Annual%20Reports/Annual%20Report%202014-15.pdf" target="_blank">Annual Report 2014-15</a></li><li><a id="mnuSubResources_e678d92b-dc92-4465-8519-1ffa4a5f9fd6" href="/resources/Inside%20the%20RIIL\Annual%20Reports/Annual%20Report%202015-16.pdf" target="_blank">Annual Report 2015-16</a></li><li><a id="mnuSubResources_47c29974-8965-425d-b214-6b7d4e8e0cea" href="/resources/Inside%20the%20RIIL\Annual%20Reports/Annual%20Report%202016-17.pdf" target="_blank">Annual Report 2016-17</a></li><li><a id="mnuSubResources_d3ab3ad5-b2e1-43f5-906c-0f1b13125b7c" href="/resources/Inside%20the%20RIIL\Annual%20Reports/Annual%20Report%202017-18.pdf" target="_blank">Annual Report 2017-18</a></li><li><a id="mnuSubResources_c9e47014-675a-4a5e-89ac-e58247207f89" href="/resources/Inside%20the%20RIIL\Annual%20Reports/Annual%20Report%202018-19.pdf" target="_blank">Annual Report 2018-19</a></li><li><a id="mnuSubResources_a19f1bdb-a0cc-4876-9309-b6e39e706861" href="/resources/Inside%20the%20RIIL\Annual%20Reports/Annual%20Report%202019-20.pdf" target="_blank">Annual Report 2019-20</a></li><li><a id="mnuSubResources_55253171-5e4b-4a9b-b66b-281d5b9e5e51" href="/resources/Inside%20the%20RIIL\Annual%20Reports/Annual%20Report%202020-21.pdf" target="_blank">Annual Report 2020-21</a></li><li><a id="mnuSubResources_ed7ed90f-f6cc-4288-928a-71a4ca29587f" href="/resources/Inside%20the%20RIIL\Annual%20Reports/Annual%20Report%202021-22.pdf" target="_blank">Annual Report 2021-22</a></li><li><a id="mnuSubResources_06de24fa-52d3-4deb-95a1-b62e10796bc7" href="/resources/Inside%20the%20RIIL\Annual%20Reports/Annual%20Report%202022-23.pdf" target="_blank">Annual Report 2022-23</a></li><li><a id="mnuSubResources_e335050d-9cf6-42f9-9039-906beadb0884" href="https://www.flipsnack.com/tpg2020/riil-annual-report-2023-24/full-view.html" target="_blank">Annual Report 2023-24</a></li></ul><ul id="NavigationMenu_mnuSubResources_InsidetheRIIL_HSAthleticHallofFame" class="dsNavigationMenuUL dsNavigationMenuSubPanel"><li><div class="dsNavigationSubMenuTitle">HS Athletic Hall of Fame</div></li><li><a id="SUB_MAIN_mnuSubResources_InsidetheRIIL_HSAthleticHallofFame" class=" dsNavigationMenuLink " onclick="changeToMainMenu();"><i class="fa fa-arrow-left" style="float:left;"></i><div style="float:left;" class="dsNavigationMenuBackButton">MAIN MENU</div><div style="clear:both;"></div></a></li><li><a id="mnuSubResources_becf4220-9271-45eb-a9c1-ef810de7b918" href="/TenantHTML.aspx?D=Inside the RIIL\HS Athletic Hall of Fame&amp;F=Hall of Fame 2003.html">Hall of Fame 2003</a></li><li><a id="mnuSubResources_492e34ac-dc21-418f-91e2-a1bfdbd5bd22" href="/TenantHTML.aspx?D=Inside the RIIL\HS Athletic Hall of Fame&amp;F=Hall of Fame 2004.html">Hall of Fame 2004</a></li><li><a id="mnuSubResources_4c67d621-46be-4ee4-a5f6-43698b8a0e23" href="/TenantHTML.aspx?D=Inside the RIIL\HS Athletic Hall of Fame&amp;F=Hall of Fame 2005.html">Hall of Fame 2005</a></li><li><a id="mnuSubResources_af1ff3ca-5c9a-4d20-bd75-199562ac1ce9" href="/TenantHTML.aspx?D=Inside the RIIL\HS Athletic Hall of Fame&amp;F=Hall of Fame 2006.html">Hall of Fame 2006</a></li><li><a id="mnuSubResources_4048df4e-2928-468a-b90b-6a047a29aa5c" href="/TenantHTML.aspx?D=Inside the RIIL\HS Athletic Hall of Fame&amp;F=Hall of Fame 2007.html">Hall of Fame 2007</a></li><li><a id="mnuSubResources_9961b8e4-3932-44bb-a7f1-5b5976ba2e85" href="/TenantHTML.aspx?D=Inside the RIIL\HS Athletic Hall of Fame&amp;F=Hall of Fame 2008.html">Hall of Fame 2008</a></li><li><a id="mnuSubResources_02aab8d7-be35-4b6e-87bb-2bad464d19f5" href="/TenantHTML.aspx?D=Inside the RIIL\HS Athletic Hall of Fame&amp;F=Hall of Fame 2009.html">Hall of Fame 2009</a></li><li><a id="mnuSubResources_724bfde7-5811-48a1-b79b-204fb3258f7e" href="/TenantHTML.aspx?D=Inside the RIIL\HS Athletic Hall of Fame&amp;F=Hall of Fame 2010.html">Hall of Fame 2010</a></li><li><a id="mnuSubResources_43576316-dab2-4773-b509-684051882c7f" href="/TenantHTML.aspx?D=Inside the RIIL\HS Athletic Hall of Fame&amp;F=Hall of Fame 2011.html">Hall of Fame 2011</a></li><li><a id="mnuSubResources_8c84069e-3a05-4278-bbf4-31ffee4c90e6" href="/TenantHTML.aspx?D=Inside the RIIL\HS Athletic Hall of Fame&amp;F=Hall of Fame 2012.html">Hall of Fame 2012</a></li><li><a id="mnuSubResources_4bf900ad-9f2d-47d7-a94c-a01b8398e68f" href="/TenantHTML.aspx?D=Inside the RIIL\HS Athletic Hall of Fame&amp;F=Hall of Fame 2013.html">Hall of Fame 2013</a></li><li><a id="mnuSubResources_c149a381-b377-41cd-ba0b-ebbf95d0b060" href="/TenantHTML.aspx?D=Inside the RIIL\HS Athletic Hall of Fame&amp;F=Hall of Fame 2014.html">Hall of Fame 2014</a></li><li><a id="mnuSubResources_d2ab472b-05a6-430b-84a3-fab0a0f1c238" href="/TenantHTML.aspx?D=Inside the RIIL\HS Athletic Hall of Fame&amp;F=Hall of Fame 2015.html">Hall of Fame 2015</a></li><li><a id="mnuSubResources_128ebe74-9cfa-445e-9855-a5961b8a15dc" href="/TenantHTML.aspx?D=Inside the RIIL\HS Athletic Hall of Fame&amp;F=Hall of Fame 2016.html">Hall of Fame 2016</a></li><li><a id="mnuSubResources_c47a8740-0324-4c6b-b27e-f0cd900e5196" href="/TenantHTML.aspx?D=Inside the RIIL\HS Athletic Hall of Fame&amp;F=Hall of Fame 2017.html">Hall of Fame 2017</a></li><li><a id="mnuSubResources_346cc7d7-a2ea-484f-bc6f-6f740ff5a8b9" href="/TenantHTML.aspx?D=Inside the RIIL\HS Athletic Hall of Fame&amp;F=Hall of Fame 2019.html">Hall of Fame 2019</a></li><li><a id="mnuSubResources_ff358e00-252c-4192-ad7e-820ba9a9ce9a" href="/TenantHTML.aspx?D=Inside the RIIL\HS Athletic Hall of Fame&amp;F=Hall of Fame 2022.html">Hall of Fame 2022</a></li><li><a id="mnuSubResources_73cc5401-16bd-488c-8767-2d29859f6b1d" href="/TenantHTML.aspx?D=Inside the RIIL\HS Athletic Hall of Fame&amp;F=Hall of Fame Inductee List.html">Hall of Fame Inductee List</a></li><li><a id="mnuSubResources_708f1e95-ed18-4ec1-969e-95dc7e04fb61" href="/resources/Inside%20the%20RIIL\HS%20Athletic%20Hall%20of%20Fame/Hall%20of%20Fame%20Nomination%20Packet%20.pdf" target="_blank">Hall of Fame Nomination Packet </a></li></ul><ul id="NavigationMenu_mnuSubResources_StudentsandParents_OperationCleanCompetition" class="dsNavigationMenuUL dsNavigationMenuSubPanel"><li><div class="dsNavigationSubMenuTitle">Operation Clean Competition</div></li><li><a id="SUB_MAIN_mnuSubResources_StudentsandParents_OperationCleanCompetition" class=" dsNavigationMenuLink " onclick="changeToMainMenu();"><i class="fa fa-arrow-left" style="float:left;"></i><div style="float:left;" class="dsNavigationMenuBackButton">MAIN MENU</div><div style="clear:both;"></div></a></li><li><a id="mnuSubResources_b091a831-ac5d-4115-9aa2-cff840c06b76" href="/resources/Students%20and%20Parents\Operation%20Clean%20Competition/OCC%20Information%20Sheet%202025-26.pdf" target="_blank">OCC Information Sheet 2025-26</a></li><li><a id="mnuSubResources_2ad67f5e-ee32-4b27-93dc-40de37d1b5b1" href="/resources/Students%20and%20Parents\Operation%20Clean%20Competition/Operation%20Clean%20Competition%20Educational%20Program.pdf" target="_blank">Operation Clean Competition Educational Program</a></li><li><a id="mnuSubResources_cfc6baf1-83cd-4b68-ba2a-ee54053ad708" href="/resources/Students%20and%20Parents\Operation%20Clean%20Competition/Operation%20Clean%20Competition%20Program%20Descriptions.pdf" target="_blank">Operation Clean Competition Program Descriptions</a></li><li><a id="mnuSubResources_7ef17fca-ba3e-4a6a-9455-a9dba7ac7d29" href="/resources/Students%20and%20Parents\Operation%20Clean%20Competition/Operation%20Clean%20Competition%20Toolkit%20for%20Schools.pdf" target="_blank">Operation Clean Competition Toolkit for Schools</a></li><li><a id="mnuSubResources_66a10876-b472-4d2b-93e3-05da2cb73495" href="https://operationcleancomp.com/">Operation Clean Competition Website</a></li></ul><ul id="NavigationMenu_mnuSubResources_StudentsandParents_StudentInitiatives" class="dsNavigationMenuUL dsNavigationMenuSubPanel"><li><div class="dsNavigationSubMenuTitle">Student Initiatives</div></li><li><a id="SUB_MAIN_mnuSubResources_StudentsandParents_StudentInitiatives" class=" dsNavigationMenuLink " onclick="changeToMainMenu();"><i class="fa fa-arrow-left" style="float:left;"></i><div style="float:left;" class="dsNavigationMenuBackButton">MAIN MENU</div><div style="clear:both;"></div></a></li><li><a id="mnuSubResources_5c0b2d83-18c8-41f4-9ace-eab4d258ff22" href="/TenantHTML.aspx?D=Students and Parents\Student Initiatives&amp;F=Leadership Conferences.html">Leadership Conferences</a></li><li><a id="mnuSubResources_13707f9d-fd8d-4bdd-b707-84b50d266a96" href="/resources/Students%20and%20Parents\Student%20Initiatives/Peanut%20Butter%20Express%20Challenge.pdf" target="_blank">Peanut Butter Express Challenge</a></li></ul><ul id="NavigationMenu_mnuSubResources_StudentsandParents_TrafficSafetyIsATeamSport" class="dsNavigationMenuUL dsNavigationMenuSubPanel"><li><div class="dsNavigationSubMenuTitle">Traffic Safety Is A Team Sport</div></li><li><a id="SUB_MAIN_mnuSubResources_StudentsandParents_TrafficSafetyIsATeamSport" class=" dsNavigationMenuLink " onclick="changeToMainMenu();"><i class="fa fa-arrow-left" style="float:left;"></i><div style="float:left;" class="dsNavigationMenuBackButton">MAIN MENU</div><div style="clear:both;"></div></a></li><li><a id="mnuSubResources_ea2dd12e-1346-40a3-9fae-0462902d816e" href="/resources/Students%20and%20Parents\Traffic%20Safety%20Is%20A%20Team%20Sport/Traffic%20Safety%20Info%20Page.pdf" target="_blank">Traffic Safety Info Page</a></li><li><a id="mnuSubResources_491fff15-5e17-40de-8163-a51ca4a0c14d" href="/resources/Students%20and%20Parents\Traffic%20Safety%20Is%20A%20Team%20Sport/Traffic%20Safety%20Pledge.pdf" target="_blank">Traffic Safety Pledge</a></li><li><a id="mnuSubResources_76688a5d-a01a-4f12-b666-22ac87949207" href="/resources/Students%20and%20Parents\Traffic%20Safety%20Is%20A%20Team%20Sport/Traffic%20Safety%20Toolkit.pdf" target="_blank">Traffic Safety Toolkit</a></li></ul>
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
                            <img id="LogoImage" class="fpLogoImage" src="/resources/_LogoSmall.png?t=2000" alt="Home" /></a>
                        <!--<div class="dsLogoTextDatable">datable</div>-->
                    </li>

                </ul>
                <div id="WelcomeText" class="dsWelcomeText">
                    <a id="Signup" title="Sign-Up" class="dsHeaderButton" href="../SignUp.aspx?ReturnUrl=%2fSportPages%2fSportPageInfo.aspx%3fTournamentID%3d1">Sign-Up</a>
                    <a id="Login" title="Login" class="dsHeaderButton" href="../Login.aspx?ReturnUrl=%2fSportPages%2fSportPageInfo.aspx%3fTournamentID%3d1">Login</a>
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
                <a id="Sport" title="Sport" href="#"><i class="fa fa-solid fa-caret-down"></i>HS Cross Country - Boys</a>
                <div id="SportPanel" class="dsToolbarDropdown2 dsToolbarDropdownLeft dsToolbarPanelScroll">
			
                <b>HS - Fall</b><br/><a id="mnuHSSport_Cross Country - Boys" href="/SportPages/SportPageInfo.aspx?TournamentID=1&amp;L=1"><i class="fa-solid fa-person-running"></i>Cross Country - Boys</a><a id="mnuHSSport_Cross Country - Girls" href="/SportPages/SportPageInfo.aspx?TournamentID=310&amp;L=1"><i class="fa-solid fa-person-running"></i>Cross Country - Girls</a><a id="mnuHSSport_Field Hockey" href="/SportPages/SportPageInfo.aspx?TournamentID=2&amp;L=1"><i class="fa-solid fa-field-hockey-stick-ball"></i>Field Hockey</a><a id="mnuHSSport_Football" href="/SportPages/SportPageInfo.aspx?TournamentID=3&amp;L=1"><i class="fa-duotone fa-football"></i>Football</a><a id="mnuHSSport_Game Day Cheerleading" href="/SportPages/SportPageInfo.aspx?TournamentID=1003&amp;L=1">Game Day Cheerleading</a><a id="mnuHSSport_Soccer - Boys" href="/SportPages/SportPageInfo.aspx?TournamentID=5&amp;L=1"><i class="fa-duotone fa-futbol"></i>Soccer - Boys</a><a id="mnuHSSport_Soccer - Girls" href="/SportPages/SportPageInfo.aspx?TournamentID=6&amp;L=1"><i class="fa-duotone fa-futbol"></i>Soccer - Girls</a><a id="mnuHSSport_Tennis - Girls" href="/SportPages/SportPageInfo.aspx?TournamentID=207&amp;L=1"><i class="fa-duotone fa-racquet"></i>Tennis - Girls</a><a id="mnuHSSport_Volleyball - Girls" href="/SportPages/SportPageInfo.aspx?TournamentID=8&amp;L=1"><i class="fa-duotone fa-volleyball"></i>Volleyball - Girls</a><a id="mnuHSSport_Volleyball - Unified" href="/SportPages/SportPageInfo.aspx?TournamentID=303&amp;L=1"><i class="fa-duotone fa-volleyball"></i>Volleyball - Unified</a><b>HS - Winter</b><br/><a id="mnuHSSport_Basketball - Boys" href="/SportPages/SportPageInfo.aspx?TournamentID=100&amp;L=1"><i class="fa-duotone fa-basketball-hoop"></i>Basketball - Boys</a><a id="mnuHSSport_Basketball - Girls" href="/SportPages/SportPageInfo.aspx?TournamentID=101&amp;L=1"><i class="fa-duotone fa-basketball-hoop"></i>Basketball - Girls</a><a id="mnuHSSport_Competition Cheerleading " href="/SportPages/SportPageInfo.aspx?TournamentID=107&amp;L=1"><i class="fa-solid fa-child-reaching"></i>Competition Cheerleading </a><a id="mnuHSSport_ESports" href="/SportPages/SportPageInfo.aspx?TournamentID=304&amp;L=1">ESports</a><a id="mnuHSSport_Gymnastics" href="/SportPages/SportPageInfo.aspx?TournamentID=102&amp;L=1"><i class="fa-solid fa-person-falling"></i>Gymnastics</a><a id="mnuHSSport_Ice Hockey - Boys" href="/SportPages/SportPageInfo.aspx?TournamentID=103&amp;L=1"><i class="fa-duotone fa-hockey-sticks"></i>Ice Hockey - Boys</a><a id="mnuHSSport_Ice Hockey - Girls" href="/SportPages/SportPageInfo.aspx?TournamentID=108&amp;L=1"><i class="fa-duotone fa-hockey-sticks"></i>Ice Hockey - Girls</a><a id="mnuHSSport_Indoor Track - Boys" href="/SportPages/SportPageInfo.aspx?TournamentID=104&amp;L=1"><i class="fa-solid fa-person-running"></i>Indoor Track - Boys</a><a id="mnuHSSport_Indoor Track - Girls" href="/SportPages/SportPageInfo.aspx?TournamentID=1024&amp;L=1"><i class="fa-solid fa-person-running"></i>Indoor Track - Girls</a><a id="mnuHSSport_Swimming - Boys" href="/SportPages/SportPageInfo.aspx?TournamentID=105&amp;L=1"><i class="fa-duotone fa-person-swimming"></i>Swimming - Boys</a><a id="mnuHSSport_Swimming - Girls" href="/SportPages/SportPageInfo.aspx?TournamentID=7&amp;L=1"><i class="fa-duotone fa-person-swimming"></i>Swimming - Girls</a><a id="mnuHSSport_Wrestling - Coed" href="/SportPages/SportPageInfo.aspx?TournamentID=106&amp;L=1"><i class="fa-duotone fa-hands"></i>Wrestling - Coed</a><b>HS - Spring</b><br/><a id="mnuHSSport_Baseball" href="/SportPages/SportPageInfo.aspx?TournamentID=200&amp;L=1"><i class="fa-duotone fa-baseball"></i>Baseball</a><a id="mnuHSSport_Basketball - Unified" href="/SportPages/SportPageInfo.aspx?TournamentID=302&amp;L=1"><i class="fa-duotone fa-basketball-hoop"></i>Basketball - Unified</a><a id="mnuHSSport_Golf - Coed" href="/SportPages/SportPageInfo.aspx?TournamentID=4&amp;L=1"><i class="fa-duotone fa-golf-flag-hole"></i>Golf - Coed</a><a id="mnuHSSport_Lacrosse - Boys" href="/SportPages/SportPageInfo.aspx?TournamentID=202&amp;L=1"><i class="fa-duotone fa-lacrosse-stick-ball"></i>Lacrosse - Boys</a><a id="mnuHSSport_Lacrosse - Girls" href="/SportPages/SportPageInfo.aspx?TournamentID=203&amp;L=1"><i class="fa-duotone fa-lacrosse-stick-ball"></i>Lacrosse - Girls</a><a id="mnuHSSport_Outdoor Track - Boys" href="/SportPages/SportPageInfo.aspx?TournamentID=204&amp;L=1"><i class="fa-solid fa-person-running"></i>Outdoor Track - Boys</a><a id="mnuHSSport_Outdoor Track - Girls" href="/SportPages/SportPageInfo.aspx?TournamentID=1021&amp;L=1"><i class="fa-solid fa-person-running"></i>Outdoor Track - Girls</a><a id="mnuHSSport_Softball" href="/SportPages/SportPageInfo.aspx?TournamentID=205&amp;L=1"><i class="fa-duotone fa-baseball-bat-ball"></i>Softball</a><a id="mnuHSSport_Tennis - Boys" href="/SportPages/SportPageInfo.aspx?TournamentID=206&amp;L=1"><i class="fa-duotone fa-racquet"></i>Tennis - Boys</a><a id="mnuHSSport_Volleyball - Boys" href="/SportPages/SportPageInfo.aspx?TournamentID=208&amp;L=1"><i class="fa-duotone fa-volleyball"></i>Volleyball - Boys</a><b>MS - Fall</b><br/><a id="mnuMSSport_Cross Country - Boys" href="/SportPages/SportPageInfo.aspx?TournamentID=1001&amp;L=2"><i class="fa-solid fa-person-running"></i>Cross Country - Boys</a><a id="mnuMSSport_Cross Country - Girls" href="/SportPages/SportPageInfo.aspx?TournamentID=1313&amp;L=2"><i class="fa-solid fa-person-running"></i>Cross Country - Girls</a><a id="mnuMSSport_Cross Country - Unified" href="/SportPages/SportPageInfo.aspx?TournamentID=1314&amp;L=2"><i class="fa-solid fa-person-running"></i>Cross Country - Unified</a><a id="mnuMSSport_Soccer - Boys" href="/SportPages/SportPageInfo.aspx?TournamentID=1005&amp;L=2"><i class="fa-duotone fa-futbol"></i>Soccer - Boys</a><a id="mnuMSSport_Soccer - Girls" href="/SportPages/SportPageInfo.aspx?TournamentID=1090&amp;L=2"><i class="fa-duotone fa-futbol"></i>Soccer - Girls</a><b>MS - Winter</b><br/><a id="mnuMSSport_Boys Basketball" href="/SportPages/SportPageInfo.aspx?TournamentID=1100&amp;L=2"><i class="fa-duotone fa-basketball-hoop"></i>Boys Basketball</a><a id="mnuMSSport_Cheerleading Winter" href="/SportPages/SportPageInfo.aspx?TournamentID=1317&amp;L=2"><i class="fa-solid fa-child-reaching"></i>Cheerleading Winter</a><a id="mnuMSSport_Girls Basketball" href="/SportPages/SportPageInfo.aspx?TournamentID=1101&amp;L=2"><i class="fa-duotone fa-basketball-hoop"></i>Girls Basketball</a><a id="mnuMSSport_Wrestling" href="/SportPages/SportPageInfo.aspx?TournamentID=1106&amp;L=2"><i class="fa-duotone fa-hands"></i>Wrestling</a><b>MS - Spring</b><br/><a id="mnuMSSport_Baseball" href="/SportPages/SportPageInfo.aspx?TournamentID=1200&amp;L=2"><i class="fa-duotone fa-baseball"></i>Baseball</a><a id="mnuMSSport_Basketball - Unified" href="/SportPages/SportPageInfo.aspx?TournamentID=1302&amp;L=2"><i class="fa-duotone fa-basketball-hoop"></i>Basketball - Unified</a><a id="mnuMSSport_Outdoor Track - Boys" href="/SportPages/SportPageInfo.aspx?TournamentID=1315&amp;L=2"><i class="fa-solid fa-person-running"></i>Outdoor Track - Boys</a><a id="mnuMSSport_Outdoor Track - Girls" href="/SportPages/SportPageInfo.aspx?TournamentID=1316&amp;L=2"><i class="fa-solid fa-person-running"></i>Outdoor Track - Girls</a><a id="mnuMSSport_Softball" href="/SportPages/SportPageInfo.aspx?TournamentID=1205&amp;L=2"><i class="fa-duotone fa-baseball-bat-ball"></i>Softball</a><b>MS - Club</b><br/><a id="mnuMSSport_Club Cheerleading" href="/SportPages/SportPageInfo.aspx?TournamentID=1319&amp;L=2">Club Cheerleading</a><a id="mnuMSSport_Club Flag Football" href="/SportPages/SportPageInfo.aspx?TournamentID=1321&amp;L=2">Club Flag Football</a><a id="mnuMSSport_Club Girls Volleyball" href="/SportPages/SportPageInfo.aspx?TournamentID=1320&amp;L=2">Club Girls Volleyball</a>
		</div>
            </li>
            <li class="dsToolbarLeft dsToolbarButton">
                <a id="InfoLink" title="Home" href="/SportPages/SportPageInfo.aspx?TournamentID=1" target="_self">Home</a>
            </li>
            <li class="dsToolbarLeft dsToolbarButton">
                <a id="TournamentLink" title="Tournament" href="/SportPages/SportPageBrackets.aspx?TournamentID=1" target="_self">Tournament</a>
            </li>
            <li class="dsToolbarLeft dsToolbarButton">
                
            </li>
            <li class="dsToolbarLeft dsToolbarButton">
                <a id="ScoresLink" title="Past Scores" href="/SportPages/SportPageScore.aspx?TournamentID=1" target="_self">Scores</a>
            </li>
            <li class="dsToolbarLeft dsToolbarButton">
                <a id="ScheduleLink" title="Upcoming Games" href="/SportPages/SportPageSchedule.aspx?TournamentID=1" target="_self">Upcoming</a>
            </li>
            <li class="dsToolbarLeft dsToolbarButton">
                <a id="RosterLink" title="Rosters" href="/SportPages/SportPageRosters.aspx?TournamentID=1" target="_self">Rosters</a>
            </li>
        </ul>
    
	</div>

    <div id="Panel1" class="dsFormPanelNormal ">
		
        

 

         <div id="HeaderWrapper">
			
        
		</div>

        <div id="BodyWrapper" class="dashboard-content">
			
        <div class="dsFlexBlock80">
				<img style='width:100%' src='/resources/_Media/Sport Headers/Cross Country Boys Header.jpg' border='0'><div class="ImportantWrapper">
					<div class="dsFlexBlock50 ImportantBlock">
						<h2>Important Dates</h2><div class="ImportantBlockContent">
							<div class="SportInfoDates">
	<h2>Fall 2026 Cross Country Regular Season</h2>
	<span class="SportInfoTitle">First Day of Practice:</span> Monday, August 17, 2026<br />
	<span class="SportInfoTitle">First Day to Schedule a Competition :</span> Thursday, August 27, 2026<br />
	<span class="SportInfoTitle">Dual Meet #1:</span> Week of September 14th<br />
	<span class="SportInfoTitle">Dual Meet #2:</span> Week of September 21st<br />
	<span class="SportInfoTitle">Dual Meet #3:</span> Week of September 28th<br />
	<span class="SportInfoTitle">Dual Meet #4:</span> Week of October 5th<br />
	<span class="SportInfoTitle">Last Day of Regular Season:</span> Friday, October 23, 2026<br />
</div><div class="SportInfoDates">
    <h2>Fall 2026 Cross Country Championships</h2>
    <span class="SportInfoTitle">Class Championships:</span> Saturday, October 31, 2026 @ TBD<br />
    <span class="SportInfoTitle">RI State Championships:</span> Saturday, November 7, 2026 @ TBD<br />
    <span class="SportInfoTitle">New England Championships:</span> Saturday, November 14, 2026 @ TBD<br />
</div>
						</div>
					</div><div class="dsFlexBlock50 ImportantBlock">
						<h2>Important Links</h2><div class="ImportantBlockContent">
							<ul><li><a id="mnuSubResources_71abd99b-216d-40d9-86f8-5807c9780c09" href="https://nfhs.org/sports/cross-country" target="_blank"><i class='fa-duotone fa-file-circle-info'></i>NFHS Cross Country Homepage</a></li><li><a id="mnuSubResources_2df1558a-801f-48fc-9a31-dc1b67e1c038" href="https://ri.milesplit.com/" target="_blank"><i class='fa-duotone fa-file-circle-info'></i>RIIL Cross Country on MileSplit</a></li><li><a id="mnuSubResources_592ae88a-2a4a-456f-88fd-94b0ebd0d09b" href="/resources/Tournament%20Info\Cross%20Country%20Boys/CNESSPA%20New%20England%20XC%20Championship%20Packet%202026.pdf" target="_blank"><i class='fa-duotone fa-file-circle-info'></i>CNESSPA New England XC Championship Packet 2026</a></li><li><a id="mnuSubResources_0dc4a534-84e6-482a-abfb-7a62c9449385" href="/TenantHTML.aspx?D=Tournament Info\Cross Country Boys&amp;F=NFHS Cross Country Rules"><i class='fa-duotone fa-file-circle-info'></i>NFHS Cross Country Rules</a></li><li><a id="mnuSubResources_e483c38b-95f3-4a15-91ae-b9208d259a12" href="/resources/Tournament%20Info\Cross%20Country%20Boys/RIIL%20Sport%20Handbook%20-%20Fall%202026%20-%20Cross%20Country.pdf" target="_blank"><i class='fa-duotone fa-file-circle-info'></i>RIIL Sport Handbook - Fall 2026 - Cross Country</a></li></ul>
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
    <span class='CarouselTracker' CarouselID='7' Carousel='Homepage Ads' CarouselEntryID='2033' CarouselEntry='Horizon Healthcare Partners | 988'><a  target='_blank' href='https://www.hhpartners.org/988'><img src='\Resources\Carousel\Home Ads\Images\0000002033.png'></a></span>
<span class='CarouselTracker' CarouselID='7' Carousel='Homepage Ads' CarouselEntryID='2029' CarouselEntry='Green Goods'><a  target='_blank' href='https://www.greengoodscafe.com/'><img src='\Resources\Carousel\Home Ads\Images\0000002029.png'></a></span>
<span class='CarouselTracker' CarouselID='7' Carousel='Homepage Ads' CarouselEntryID='1423' CarouselEntry='RISLA'><a  target='_blank' href=' https://www.risla.com/home-page'><img src='\Resources\Carousel\Home Ads\Images\0000001423.png'></a></span>
<span class='CarouselTracker' CarouselID='7' Carousel='Homepage Ads' CarouselEntryID='1288' CarouselEntry='OrthoRI'><a  target='_blank' href='https://www.orthopedicsri.com/ortho-ri-express/'><img src='\Resources\Carousel\Home Ads\Images\0000001288.png'></a></span>
<span class='CarouselTracker' CarouselID='7' Carousel='Homepage Ads' CarouselEntryID='1256' CarouselEntry='Rhode Island National Guard'><a  target='_blank' href='https://bit.ly/49Sh4m7'><img src='\Resources\Carousel\Home Ads\Images\0000001256.jpg'></a></span>
<span class='CarouselTracker' CarouselID='7' Carousel='Homepage Ads' CarouselEntryID='1205' CarouselEntry='Wright's Farm'><a  target='_blank' href='https://www.wrightsfarm.com/'><img src='\Resources\Carousel\Home Ads\Images\0000001205.png'></a></span>
<span class='CarouselTracker' CarouselID='7' Carousel='Homepage Ads' CarouselEntryID='1303' CarouselEntry='Restore Hyper Wellness'><a  target='_blank' href='https://www.restore.com/'><img src='\Resources\Carousel\Home Ads\Images\0000001303.png'></a></span>
<span class='CarouselTracker' CarouselID='7' Carousel='Homepage Ads' CarouselEntryID='1199' CarouselEntry='KyleCares'><a  target='_blank' href='https://kylecaresinc.org/'><img src='\Resources\Carousel\Home Ads\Images\0000001199.png'></a></span>
<span class='CarouselTracker' CarouselID='7' Carousel='Homepage Ads' CarouselEntryID='1175' CarouselEntry='Litter Free RI'><a  target='_blank' href='https://litterfree.ri.gov/take-pledge'><img src='\Resources\Carousel\Home Ads\Images\0000001175.png'></a></span>
<span class='CarouselTracker' CarouselID='7' Carousel='Homepage Ads' CarouselEntryID='1173' CarouselEntry='Blue Cross Blue Shield of Rhode Island'><a  target='_blank' href='https://www.bcbsri.com/'><img src='\Resources\Carousel\Home Ads\Images\0000001173.png'></a></span>
<span class='CarouselTracker' CarouselID='7' Carousel='Homepage Ads' CarouselEntryID='1146' CarouselEntry='Drive Sober Get Pulled Over'><a  target='_blank' href=''><img src='\Resources\Carousel\Home Ads\Images\0000001146.png'></a></span>
<span class='CarouselTracker' CarouselID='7' Carousel='Homepage Ads' CarouselEntryID='1130' CarouselEntry='Rhode Island Energy'><a  target='_blank' href='https://www.rienergy.com/'><img src='\Resources\Carousel\Home Ads\Images\0000001130.png'></a></span>
<span class='CarouselTracker' CarouselID='7' Carousel='Homepage Ads' CarouselEntryID='1128' CarouselEntry='Navigant Credit Union'><a  target='_blank' href='https://navigantcu.org/personal/embark-account/#:~:text=With%20our%20Embark%20account%2C%20we%20promise%20to%20never,mobile%20deposits.%20All%20you%20need%20is%20your%20phone.'><img src='\Resources\Carousel\Home Ads\Images\0000001128.png'></a></span>

</div>





				</div>
			</div><div class="dsFlexBlock20">
				<div id='DashboardSideAds' class='DashboardAds'><a target='_self' href='/resources/Tournament%20Info/Cross Country Boys/RIIL Sport Handbook - Fall 2026 - Cross Country.pdf'><img src='/resources/_media/DashboardAds/Handbook_1_Cross Country Boys.png' /></a><a target='_self' href='/DashboardSchedule.aspx?L=1&SportID=2_1_-1'><img src=' /resources/_media/DashboardAds/Todays Games.png' /></a><a target='_blank' href='https://gofan.co/app/school/Rhode Island Interscholastic League'><img src=' /resources/_media/DashboardAds/Go Fan Tickets.png' ></a></div>
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
    <span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='2031' CarouselEntry='Horizon Healthcare Partners-988'><a target='_blank' href='https://www.hhpartners.org/988'><img src='\Resources\Carousel\Partner\Images\0000002031.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='2025' CarouselEntry='Green Goods'><a target='_blank' href='https://www.greengoodscafe.com/'><img src='\Resources\Carousel\Partner\Images\0000002025.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1976' CarouselEntry='Northwest Design Ink'><a target='_blank' href='https://online-apparel.com/s/riil-store/'><img src='\Resources\Carousel\Partner\Images\0000001976.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1425' CarouselEntry='RISLA'><a target='_blank' href=' https://www.risla.com/home-page'><img src='\Resources\Carousel\Partner\Images\0000001425.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1383' CarouselEntry='Beacon Bank'><a target='_blank' href='https://www.beaconbank.com/'><img src='\Resources\Carousel\Partner\Images\0000001383.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1090' CarouselEntry='Blue Cross Blue Shield'><a target='_blank' href='https://www.bcbsri.com/'><img src='\Resources\Carousel\Partner\Images\0000001090.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1062' CarouselEntry='Ortho Rhode Island'><a target='_blank' href='https://www.orthopedicsri.com/'><img src='\Resources\Carousel\Partner\Images\0000001062.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1068' CarouselEntry='Rhode Island Energy'><a target='_blank' href='https://www.rienergy.com/'><img src='\Resources\Carousel\Partner\Images\0000001068.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1108' CarouselEntry='Navigant'><a target='_blank' href='https://navigantcu.org/'><img src='\Resources\Carousel\Partner\Images\0000001108.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1120' CarouselEntry='Go Fan'><a target='_blank' href='https://gofan.co/app/school/RIIL'><img src='\Resources\Carousel\Partner\Images\0000001120.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1072' CarouselEntry='Credit Unions of RI'><a target='_blank' href='https://www.bankri.com/'><img src='\Resources\Carousel\Partner\Images\0000001072.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1396' CarouselEntry='Local 51'><a target='_blank' href='https://ualocal51.com/'><img src='\Resources\Carousel\Partner\Images\0000001396.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1324' CarouselEntry='Wilson'><a target='_blank' href='https://www.wilson.com/'><img src='\Resources\Carousel\Partner\Images\0000001324.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1316' CarouselEntry='Litter Free RI'><a target='_blank' href='https://litterfree.ri.gov/take-pledge'><img src='\Resources\Carousel\Partner\Images\0000001316.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1092' CarouselEntry='Adrenaline'><a target='_blank' href='https://www.adrenalinefundraisingne.com/'><img src='\Resources\Carousel\Partner\Images\0000001092.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1100' CarouselEntry='Chick-fil-A'><a target='_blank' href='https://www.chick-fil-a.com/locations/ma/seekonk'><img src='\Resources\Carousel\Partner\Images\0000001100.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1096' CarouselEntry='Army National Guard'><a target='_blank' href='https://nationalguard.com/rhode-island?utm_campaign=fy26iheartriildigital&utm_source=76&utm_medium=fy26iheartriildigital-1763046552&utm_content=web'><img src='\Resources\Carousel\Partner\Images\0000001096.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1102' CarouselEntry='MaxPreps'><a target='_blank' href='https://www.maxpreps.com/'><img src='\Resources\Carousel\Partner\Images\0000001102.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1328' CarouselEntry='Mikasa'><a target='_blank' href='https://mikasasports.com/'><img src='\Resources\Carousel\Partner\Images\0000001328.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1098' CarouselEntry='KyleCares'><a target='_blank' href='https://kylecaresinc.org/'><img src='\Resources\Carousel\Partner\Images\0000001098.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1104' CarouselEntry='Mile Split'><a target='_blank' href='https://ri.milesplit.com/'><img src='\Resources\Carousel\Partner\Images\0000001104.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1110' CarouselEntry='Wrights Farm'><a target='_blank' href='https://www.wrightsfarm.com/'><img src='\Resources\Carousel\Partner\Images\0000001110.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1326' CarouselEntry='Select'><a target='_blank' href='https://us.select-sport.com/'><img src='\Resources\Carousel\Partner\Images\0000001326.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1116' CarouselEntry='Jostens'><a target='_blank' href='https://jostens.my.canva.site/athletic-and-championship-recognition'><img src='\Resources\Carousel\Partner\Images\0000001116.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1314' CarouselEntry='Restore Hyper Wellness'><a target='_blank' href='https://www.restore.com/'><img src='\Resources\Carousel\Partner\Images\0000001314.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1094' CarouselEntry='Game Changer'><a target='_blank' href='https://gc.com/?utm_source=RIIL&utm_medium=referral&utm_campaign=2022&utm_content=sponsorlogo&utm_term=partner'><img src='\Resources\Carousel\Partner\Images\0000001094.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1124' CarouselEntry='iWannamaker'><a target='_blank' href='https://iwanamaker.com/league/7344456'><img src='\Resources\Carousel\Partner\Images\0000001124.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1122' CarouselEntry='Dicks Sporting Goods'><a target='_blank' href='https://www.dickssportinggoods.com/'><img src='\Resources\Carousel\Partner\Images\0000001122.png'  /></a></span>
<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1126' CarouselEntry='Valley CC'><a target='_blank' href='https://www.valleycountryclub.net/'><img src='\Resources\Carousel\Partner\Images\0000001126.png'  /></a></span>

</div>





				</div>
			</div><h2 class="dsFlexBlock90 SportTitle">Today's Meets</h2><div class="dsFlexBlock90">
				<div id="GameTickerWrapper" class="GameTickerWrapper">
					<a href='/dashboardgame.aspx?tournamentid=1&schoolid=6&school2id=44&SeasonRoster=1'><div class='GameTickerBlock'><div class='GameTickerDivision'>VARSITY</div><div class='GameTickerTeam  GameTeamLeft '><div class='GameSchoolName'>Burrillville HS</div><img src='/resources/_media/SchoolLogos/SchoolLogo_6.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"></div><div class='GameTickerTeam  GameTeamRight '><div class='GameSchoolName'>Scituate HS</div><img src='/resources/_media/SchoolLogos/SchoolLogo_44.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"></div><div class='GameTickerDetail'>MON 9/21 4:00 PM Burrillville Middle School</div></div></a><a href='/dashboardgame.aspx?tournamentid=1&schoolid=6&school2id=16&SeasonRoster=1'><div class='GameTickerBlock'><div class='GameTickerDivision'>VARSITY</div><div class='GameTickerTeam  GameTeamLeft '><div class='GameSchoolName'>Burrillville HS</div><img src='/resources/_media/SchoolLogos/SchoolLogo_6.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"></div><div class='GameTickerTeam  GameTeamRight '><div class='GameSchoolName'>Davies HS </div><img src='/resources/_media/SchoolLogos/SchoolLogo_16.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"></div><div class='GameTickerDetail'>MON 9/21 4:00 PM Burrillville Middle School</div></div></a><a href='/dashboardgame.aspx?tournamentid=1&schoolid=6&school2id=36&SeasonRoster=1'><div class='GameTickerBlock'><div class='GameTickerDivision'>VARSITY</div><div class='GameTickerTeam  GameTeamLeft '><div class='GameSchoolName'>Burrillville HS</div><img src='/resources/_media/SchoolLogos/SchoolLogo_6.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"></div><div class='GameTickerTeam  GameTeamRight '><div class='GameSchoolName'>Ponaganset HS</div><img src='/resources/_media/SchoolLogos/SchoolLogo_36.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"></div><div class='GameTickerDetail'>MON 9/21 4:00 PM Burrillville Middle School</div></div></a><a href='/dashboardgame.aspx?tournamentid=1&schoolid=14&school2id=22&SeasonRoster=1'><div class='GameTickerBlock'><div class='GameTickerDivision'>VARSITY</div><div class='GameTickerTeam  GameTeamLeft '><div class='GameSchoolName'>Cranston West HS</div><img src='/resources/_media/SchoolLogos/SchoolLogo_14.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"></div><div class='GameTickerTeam  GameTeamRight '><div class='GameSchoolName'>Juanita Sanchez HS</div><img src='/resources/_media/SchoolLogos/SchoolLogo_22.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"></div><div class='GameTickerDetail'>MON 9/21 5:00 PM John Chaffee Athletic and Recreation Complex</div></div></a><a href='/dashboardgame.aspx?tournamentid=1&schoolid=14&school2id=12&SeasonRoster=1'><div class='GameTickerBlock'><div class='GameTickerDivision'>VARSITY</div><div class='GameTickerTeam  GameTeamLeft '><div class='GameSchoolName'>Cranston West HS</div><img src='/resources/_media/SchoolLogos/SchoolLogo_14.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"></div><div class='GameTickerTeam  GameTeamRight '><div class='GameSchoolName'>Coventry HS</div><img src='/resources/_media/SchoolLogos/SchoolLogo_12.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"></div><div class='GameTickerDetail'>MON 9/21 5:00 PM John Chaffee Athletic and Recreation Complex</div></div></a><a href='/dashboardgame.aspx?tournamentid=1&schoolid=14&school2id=21&SeasonRoster=1'><div class='GameTickerBlock'><div class='GameTickerDivision'>VARSITY</div><div class='GameTickerTeam  GameTeamLeft '><div class='GameSchoolName'>Cranston West HS</div><img src='/resources/_media/SchoolLogos/SchoolLogo_14.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"></div><div class='GameTickerTeam  GameTeamRight '><div class='GameSchoolName'>Johnston HS</div><img src='/resources/_media/SchoolLogos/SchoolLogo_21.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"></div><div class='GameTickerDetail'>MON 9/21 5:00 PM John Chaffee Athletic and Recreation Complex</div></div></a><a href='/dashboardgame.aspx?tournamentid=1&schoolid=14&school2id=13&SeasonRoster=1'><div class='GameTickerBlock'><div class='GameTickerDivision'>VARSITY</div><div class='GameTickerTeam  GameTeamLeft '><div class='GameSchoolName'>Cranston West HS</div><img src='/resources/_media/SchoolLogos/SchoolLogo_14.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"></div><div class='GameTickerTeam  GameTeamRight '><div class='GameSchoolName'>Cranston East HS</div><img src='/resources/_media/SchoolLogos/SchoolLogo_13.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"></div><div class='GameTickerDetail'>MON 9/21 5:00 PM John Chaffee Athletic and Recreation Complex</div></div></a>
				</div>
			</div><h2 class="dsFlexBlock90 SportTitle">Recent Meets</h2><div class="dsFlexBlock90">
				<div id="GameTickerWrapper" class="GameTickerWrapper">
					<a href='/dashboardgame.aspx?tournamentid=1&schoolid=1019&school2id=27&SeasonRoster=1'><div class='GameTickerBlock'><div class='GameTickerDivision'>VARSITY</div><div class='GameTickerTeam  GameTeamLeft '><div class='GameSchoolName'>TBA</div><img src='/resources/_media/SchoolLogos/SchoolLogo_1019.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"></div><div class='GameTickerTeam  GameTeamRight '><div class='GameSchoolName'>Moses Brown School</div><img src='/resources/_media/SchoolLogos/SchoolLogo_27.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"></div><div class='GameTickerDetail'>SAT 9/19 11:00 AM Manchester, NH</div></div></a><a href='/dashboardgame.aspx?tournamentid=1&schoolid=2&school2id=49&SeasonRoster=1'><div class='GameTickerBlock'><div class='GameTickerDivision'>VARSITY</div><div class='GameTickerTeam  GameTeamLeft '><div class='GameSchoolName'>Barrington HS</div><img src='/resources/_media/SchoolLogos/SchoolLogo_2.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"><div class='GameTickerScore'></div></div><div class='GameTickerTeam  GameTeamRight '><div class='GameSchoolName'>Tiverton HS</div><img src='/resources/_media/SchoolLogos/SchoolLogo_49.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"><div class='GameTickerScore'></div></div><div class='GameTickerDetail'></div></div></a><a href='/dashboardgame.aspx?tournamentid=1&schoolid=2&school2id=43&SeasonRoster=1'><div class='GameTickerBlock'><div class='GameTickerDivision'>VARSITY</div><div class='GameTickerTeam  GameTeamLeft '><div class='GameSchoolName'>Barrington HS</div><img src='/resources/_media/SchoolLogos/SchoolLogo_2.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"><div class='GameTickerScore'></div></div><div class='GameTickerTeam  GameTeamRight '><div class='GameSchoolName'>St. Raphael Academy </div><img src='/resources/_media/SchoolLogos/SchoolLogo_43.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"><div class='GameTickerScore'></div></div><div class='GameTickerDetail'></div></div></a><a href='/dashboardgame.aspx?tournamentid=1&schoolid=2&school2id=39&SeasonRoster=1'><div class='GameTickerBlock'><div class='GameTickerDivision'>VARSITY</div><div class='GameTickerTeam  GameTeamLeft '><div class='GameSchoolName'>Barrington HS</div><img src='/resources/_media/SchoolLogos/SchoolLogo_2.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"><div class='GameTickerScore'></div></div><div class='GameTickerTeam  GameTeamRight '><div class='GameSchoolName'>Providence Country Day HS</div><img src='/resources/_media/SchoolLogos/SchoolLogo_39.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"><div class='GameTickerScore'></div></div><div class='GameTickerDetail'></div></div></a><a href='/dashboardgame.aspx?tournamentid=1&schoolid=40&school2id=52&SeasonRoster=1'><div class='GameTickerBlock'><div class='GameTickerDivision'>VARSITY</div><div class='GameTickerTeam  GameTeamLeft '><div class='GameSchoolName'>Rogers HS</div><img src='/resources/_media/SchoolLogos/SchoolLogo_40.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"><div class='GameTickerScore'></div></div><div class='GameTickerTeam  GameTeamRight '><div class='GameSchoolName'>Westerly HS</div><img src='/resources/_media/SchoolLogos/SchoolLogo_52.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"><div class='GameTickerScore'></div></div><div class='GameTickerDetail'></div></div></a><a href='/dashboardgame.aspx?tournamentid=1&schoolid=40&school2id=53&SeasonRoster=1'><div class='GameTickerBlock'><div class='GameTickerDivision'>VARSITY</div><div class='GameTickerTeam  GameTeamLeft '><div class='GameSchoolName'>Rogers HS</div><img src='/resources/_media/SchoolLogos/SchoolLogo_40.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"><div class='GameTickerScore'></div></div><div class='GameTickerTeam  GameTeamRight '><div class='GameSchoolName'>West Warwick HS</div><img src='/resources/_media/SchoolLogos/SchoolLogo_53.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"><div class='GameTickerScore'></div></div><div class='GameTickerDetail'></div></div></a><a href='/dashboardgame.aspx?tournamentid=1&schoolid=15&school2id=24&SeasonRoster=1'><div class='GameTickerBlock'><div class='GameTickerDivision'>VARSITY</div><div class='GameTickerTeam  GameTeamLeft '><div class='GameSchoolName'>Cumberland HS</div><img src='/resources/_media/SchoolLogos/SchoolLogo_15.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"><div class='GameTickerScore'></div></div><div class='GameTickerTeam  GameTeamRight '><div class='GameSchoolName'>Lincoln HS</div><img src='/resources/_media/SchoolLogos/SchoolLogo_24.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"><div class='GameTickerScore'></div></div><div class='GameTickerDetail'></div></div></a><a href='/dashboardgame.aspx?tournamentid=1&schoolid=15&school2id=16&SeasonRoster=1'><div class='GameTickerBlock'><div class='GameTickerDivision'>VARSITY</div><div class='GameTickerTeam  GameTeamLeft '><div class='GameSchoolName'>Cumberland HS</div><img src='/resources/_media/SchoolLogos/SchoolLogo_15.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"><div class='GameTickerScore'></div></div><div class='GameTickerTeam  GameTeamRight '><div class='GameSchoolName'>Davies HS </div><img src='/resources/_media/SchoolLogos/SchoolLogo_16.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"><div class='GameTickerScore'></div></div><div class='GameTickerDetail'></div></div></a><a href='/dashboardgame.aspx?tournamentid=1&schoolid=15&school2id=44&SeasonRoster=1'><div class='GameTickerBlock'><div class='GameTickerDivision'>VARSITY</div><div class='GameTickerTeam  GameTeamLeft '><div class='GameSchoolName'>Cumberland HS</div><img src='/resources/_media/SchoolLogos/SchoolLogo_15.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"><div class='GameTickerScore'></div></div><div class='GameTickerTeam  GameTeamRight '><div class='GameSchoolName'>Scituate HS</div><img src='/resources/_media/SchoolLogos/SchoolLogo_44.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"><div class='GameTickerScore'></div></div><div class='GameTickerDetail'></div></div></a><a href='/dashboardgame.aspx?tournamentid=1&schoolid=47&school2id=9&SeasonRoster=1'><div class='GameTickerBlock'><div class='GameTickerDivision'>VARSITY</div><div class='GameTickerTeam  GameTeamLeft '><div class='GameSchoolName'>South Kingstown HS</div><img src='/resources/_media/SchoolLogos/SchoolLogo_47.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"><div class='GameTickerScore'></div></div><div class='GameTickerTeam  GameTeamRight '><div class='GameSchoolName'>Chariho HS</div><img src='/resources/_media/SchoolLogos/SchoolLogo_9.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"><div class='GameTickerScore'></div></div><div class='GameTickerDetail'></div></div></a><a href='/dashboardgame.aspx?tournamentid=1&schoolid=47&school2id=38&SeasonRoster=1'><div class='GameTickerBlock'><div class='GameTickerDivision'>VARSITY</div><div class='GameTickerTeam  GameTeamLeft '><div class='GameSchoolName'>South Kingstown HS</div><img src='/resources/_media/SchoolLogos/SchoolLogo_47.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"><div class='GameTickerScore'></div></div><div class='GameTickerTeam  GameTeamRight '><div class='GameSchoolName'>Prout School</div><img src='/resources/_media/SchoolLogos/SchoolLogo_38.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"><div class='GameTickerScore'></div></div><div class='GameTickerDetail'></div></div></a><a href='/dashboardgame.aspx?tournamentid=1&schoolid=11&school2id=50&SeasonRoster=1'><div class='GameTickerBlock'><div class='GameTickerDivision'>VARSITY</div><div class='GameTickerTeam  GameTeamLeft '><div class='GameSchoolName'>Classical HS</div><img src='/resources/_media/SchoolLogos/SchoolLogo_11.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"><div class='GameTickerScore'></div></div><div class='GameTickerTeam  GameTeamRight '><div class='GameSchoolName'>Toll Gate HS</div><img src='/resources/_media/SchoolLogos/SchoolLogo_50.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"><div class='GameTickerScore'></div></div><div class='GameTickerDetail'></div></div></a><a href='/dashboardgame.aspx?tournamentid=1&schoolid=11&school2id=13&SeasonRoster=1'><div class='GameTickerBlock'><div class='GameTickerDivision'>VARSITY</div><div class='GameTickerTeam  GameTeamLeft '><div class='GameSchoolName'>Classical HS</div><img src='/resources/_media/SchoolLogos/SchoolLogo_11.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"><div class='GameTickerScore'></div></div><div class='GameTickerTeam  GameTeamRight '><div class='GameSchoolName'>Cranston East HS</div><img src='/resources/_media/SchoolLogos/SchoolLogo_13.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"><div class='GameTickerScore'></div></div><div class='GameTickerDetail'></div></div></a><a href='/dashboardgame.aspx?tournamentid=1&schoolid=11&school2id=21&SeasonRoster=1'><div class='GameTickerBlock'><div class='GameTickerDivision'>VARSITY</div><div class='GameTickerTeam  GameTeamLeft '><div class='GameSchoolName'>Classical HS</div><img src='/resources/_media/SchoolLogos/SchoolLogo_11.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"><div class='GameTickerScore'></div></div><div class='GameTickerTeam  GameTeamRight '><div class='GameSchoolName'>Johnston HS</div><img src='/resources/_media/SchoolLogos/SchoolLogo_21.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"><div class='GameTickerScore'></div></div><div class='GameTickerDetail'></div></div></a><a href='/dashboardgame.aspx?tournamentid=1&schoolid=6&school2id=32&SeasonRoster=1'><div class='GameTickerBlock'><div class='GameTickerDivision'>VARSITY</div><div class='GameTickerTeam  GameTeamLeft '><div class='GameSchoolName'>Burrillville HS</div><img src='/resources/_media/SchoolLogos/SchoolLogo_6.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"><div class='GameTickerScore'></div></div><div class='GameTickerTeam  GameTeamRight '><div class='GameSchoolName'>North Providence HS</div><img src='/resources/_media/SchoolLogos/SchoolLogo_32.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"><div class='GameTickerScore'></div></div><div class='GameTickerDetail'></div></div></a><a href='/dashboardgame.aspx?tournamentid=1&schoolid=6&school2id=8&SeasonRoster=1'><div class='GameTickerBlock'><div class='GameTickerDivision'>VARSITY</div><div class='GameTickerTeam  GameTeamLeft '><div class='GameSchoolName'>Burrillville HS</div><img src='/resources/_media/SchoolLogos/SchoolLogo_6.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"><div class='GameTickerScore'></div></div><div class='GameTickerTeam  GameTeamRight '><div class='GameSchoolName'>Central Falls HS</div><img src='/resources/_media/SchoolLogos/SchoolLogo_8.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"><div class='GameTickerScore'></div></div><div class='GameTickerDetail'></div></div></a><a href='/dashboardgame.aspx?tournamentid=1&schoolid=6&school2id=33&SeasonRoster=1'><div class='GameTickerBlock'><div class='GameTickerDivision'>VARSITY</div><div class='GameTickerTeam  GameTeamLeft '><div class='GameSchoolName'>Burrillville HS</div><img src='/resources/_media/SchoolLogos/SchoolLogo_6.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"><div class='GameTickerScore'></div></div><div class='GameTickerTeam  GameTeamRight '><div class='GameSchoolName'>North Smithfield HS</div><img src='/resources/_media/SchoolLogos/SchoolLogo_33.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"><div class='GameTickerScore'></div></div><div class='GameTickerDetail'></div></div></a><a href='/dashboardgame.aspx?tournamentid=1&schoolid=14&school2id=3&SeasonRoster=1'><div class='GameTickerBlock'><div class='GameTickerDivision'>VARSITY</div><div class='GameTickerTeam  GameTeamLeft '><div class='GameSchoolName'>Cranston West HS</div><img src='/resources/_media/SchoolLogos/SchoolLogo_14.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"><div class='GameTickerScore'></div></div><div class='GameTickerTeam  GameTeamRight '><div class='GameSchoolName'>Bishop Hendricken HS</div><img src='/resources/_media/SchoolLogos/SchoolLogo_3.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"><div class='GameTickerScore'></div></div><div class='GameTickerDetail'></div></div></a><a href='/dashboardgame.aspx?tournamentid=1&schoolid=14&school2id=29&SeasonRoster=1'><div class='GameTickerBlock'><div class='GameTickerDivision'>VARSITY</div><div class='GameTickerTeam  GameTeamLeft '><div class='GameSchoolName'>Cranston West HS</div><img src='/resources/_media/SchoolLogos/SchoolLogo_14.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"><div class='GameTickerScore'></div></div><div class='GameTickerTeam  GameTeamRight '><div class='GameSchoolName'>Mt. Pleasant HS</div><img src='/resources/_media/SchoolLogos/SchoolLogo_29.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"><div class='GameTickerScore'></div></div><div class='GameTickerDetail'></div></div></a><a href='/dashboardgame.aspx?tournamentid=1&schoolid=46&school2id=54&SeasonRoster=1'><div class='GameTickerBlock'><div class='GameTickerDivision'>VARSITY</div><div class='GameTickerTeam  GameTeamLeft '><div class='GameSchoolName'>Smithfield HS</div><img src='/resources/_media/SchoolLogos/SchoolLogo_46.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"><div class='GameTickerScore'></div></div><div class='GameTickerTeam  GameTeamRight '><div class='GameSchoolName'>Woonsocket HS</div><img src='/resources/_media/SchoolLogos/SchoolLogo_54.jpg' onerror="this.src='data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';"><div class='GameTickerScore'></div></div><div class='GameTickerDetail'></div></div></a>
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
