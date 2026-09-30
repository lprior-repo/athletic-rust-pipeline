

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

    

    <link href="../_Styles/GameCalendar.css?3" rel="stylesheet" />

    <style>
        .dsFormPanelFSwTB {
            width: Calc(100% - 20px);
            padding: 10px;
            background-color: white;
            overflow: visible;
            height: fit-content;
        }

        .dsMiddlePanel {
            background-color: white;
        }


        #HeaderWrapper {
            color: black;
            margin-top: 10px;
            margin-bottom: 10px;
        }


        .TeamSelection {
            display: inline-block;
            font-size: 16px;
            margin: 4px 0 4px 0;
        }


        /*  The gap under the toolbar belongs to this container, not to whatever happens to be first
            inside it.

            It used to come from the app note's own margin-top. In app mode that note is hidden -
            display:none, so it is still the first child and any :first-child rule would land on it
            rather than on the carousel - and the gap went with it, putting the carousel hard
            against the toolbar. Here it is the same 20px whatever is or is not showing, on every
            tab. */
        .school-content {
            color: black;
            padding-top: 20px;
            padding-bottom: 60px;
            min-height: 400px;
        }

            .school-content p {
                margin-bottom: 10px;
            }

            .school-content h2 {
                padding: 2px 0px 2px 10px;
                background-color: #0571b8;
                color: white;
                font-size: 17px;
                font-weight: bold;
                display: block;
                width: 100%;
                margin: 30px 0 10px 0;
                box-sizing: border-box;
                text-transform: uppercase;
                line-height: 30px;
            }

        .SchoolList {
            display: block;
            clear: both;
            margin-bottom: 10px;
        }

        .SchoolListWrapper {
            display: block;
            float: left;
            margin: 10px;
        }

        .SchoolSearchResults a, .SchoolSearchResults a:link, .SchoolSearchResults a:visited, .SchoolSearchResults a:hover, .SchoolSearchResults a:active {
            color: #444444;
            text-decoration: none;
        }

        .SchoolListImageWrapper {
            display: block;
            width: 120px;
            margin: 0 auto;
        }

        .SchoolListLogo {
            display: block;
            object-fit: contain;
            height: 60px;
            max-width: 80px;
            margin: auto auto;
        }

        .SchoolListName {
            display: flex;
            align-items: center;
            justify-content: center;
            text-align: center;
            height: 60px;
            max-width: 120px;
            font-size: 12px;
            font-weight: bold;
            white-space: pre-wrap;
        }

        .SchoolLogo {
            max-height: 150px;
            max-width: 20vw;
        }

        .SchoolName {
            font-size: clamp(20px, 5vw, 30px);
            font-weight: bold;
            clear: right;
        }

        /* Shown INSTEAD of the logo/name table, for a school with extended content and a banner
           uploaded to its own resource folder. It replaces a full-width table, so it is full width
           too - the artwork decides its own height rather than being boxed to the logo's 150px. */
        .SchoolBanner img {
            display: block;
            width: 100%;
            height: auto;
        }

        .SportTitle {
            background-color: #003366;
            color: white;
            padding: 16px 0 16px 0;
            font-size: 32px;
            text-align: center;
            font-weight: bold;
            text-transform: uppercase;
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

        .ImportantWrapper {
            display: flex;
            flex-wrap: wrap;
            gap: 10px;
            margin-top: 10px;
        }

        .ImportantBlock {
            min-height: 200px;
            background-color: white;
            max-height: 600px;
        }

            .ImportantBlock h2 {
                margin: 0;
            }

            .ImportantBlock .ImportantBlockContent {
                display: block;
                font-size: 13px;
                line-height: 1.9;
                overflow-y: auto;
                height: Calc(100% - 30px);
            }

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


            .ImportantBlock ul {
                list-style-type: none;
                padding: 4px 8px 20px 8px;
            }

            .ImportantBlock li a {
                color: black;
                font-size: 14px;
                margin-bottom: 12px;
                line-height: 30px;
            }

                .ImportantBlock li a i {
                    font-size: 18px;
                    margin: 0 6px 0 0;
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
            background-color: lightgrey;
            padding: 10px 0 20px 0;
        }

            .GameTickerWrapper a {
                flex: 0 0 45%;
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
            background-color: white;
            margin-top: 10px;
            text-align: center;
        }

            .SponsorLogos img {
                max-height: 100px;
                margin: 20px;
            }




        .StandingTable {
            margin-left: 10px;
        }

        .TeamPhoto {
            display: block;
            margin: 10px auto 20px auto;
            max-width: 100%;
            max-height: 450px;
        }

        .TeamHeaderTextWrapper {
            margin: 10px 0 10px 0;
            line-height: 1.5;
        }

        .StandingTable th {
            font-weight: bold;
            font-size: 10px;
            padding-bottom: 4px;
        }

        .StandingTable td {
            font-weight: normal;
            font-size: 14px;
            padding: 4px 0 4px 0;
        }

            .StandingTable td:nth-child(2) {
                font-size: 12px;
            }


            .StandingTable td:nth-child(3) {
                font-weight: bold;
            }

            .StandingTable td:nth-child(2),
            .StandingTable td:nth-child(3),
            .StandingTable td:nth-child(4),
            .StandingTable td:nth-child(5) {
                text-align: center;
            }

        .TeamNotifications {
            font-weight: normal;
            font-size: 14px;
            padding: 4px 0 4px 0;
            color: #444444;
        }

            .TeamNotifications th {
                font-weight: bold;
                font-size: 10px;
                padding-bottom: 4px;
            }

            .TeamNotifications td {
                font-weight: normal;
                font-size: 13px;
                padding: 2px 10px 2px 0;
            }

                .TeamNotifications td:nth-child(3),
                .TeamNotifications td:nth-child(4) {
                    font-size: 8px;
                }

            .TeamNotifications a,
            .TeamNotifications a:visited {
                color: #444444;
            }


        .DirectoryGroup {
            padding: 2px 0px 2px 10px;
            background-color: #003366;
            color: white;
            font-size: 17px;
            font-weight: bold;
            display: block;
            width: 100%;
            margin: 10px 0 10px 0 !important;
            box-sizing: border-box;
            text-transform: uppercase;
        }

        .DirectoryDetail {
            padding: 10px 10px 10px 10px !important;
            font-size: 12px;
            width: Calc(100% - 20px);
            overflow-x: auto;
        }

        .DirectoryFixedWidth1 {
            display: inline-block;
            min-width: 120px;
        }

        .DirectoryStaffTable {
            margin-top: 20px !important;
            width: max-content;
        }

            .DirectoryStaffTable td {
                padding: 0px 4px;
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
        /* A school's own carousel, published from its resource folder. One slide at a time.

           max-width rather than width: the published posters are 590px wide, and stretching them
           to a full-width column would upscale and blur them. They shrink on a narrow screen and
           sit centred on a wide one. */
        .SchoolCarouselPanel {
            margin-bottom: 30px;
        }

            .SchoolCarouselPanel img {
                display: block;
                max-width: 100%;
                height: auto;
                margin: 0 auto;
            }

            /* Each slide is a <span class='CarouselTracker'> - that is what the publisher emits,
               and it is what Default.aspx's news carousel is built from too. Slick gives them
               their own display, but until the script runs they would all stack and the page
               would jump as it initialises, so everything after the first is hidden. */
            .SchoolCarouselPanel:not(.slick-initialized) > *:not(:first-child) {
                display: none;
            }

        /* NEWS - image on top, then date, title, text, three across. Mirrors the featured news
           strip on the tenant dashboard; the class names are the school's own because the
           tenant's .NewsBlock rules live inline in Default.aspx rather than in a
           shared stylesheet, and copying them under the same names would invite the two to drift
           apart silently. */
        .SchoolNewsPanel .slick-track {
            display: flex;
            align-items: stretch;
        }

        .SchoolNewsPanel .slick-slide {
            height: auto;
        }

            .SchoolNewsPanel .slick-slide > div,
            .SchoolNewsPanel .CarouselTracker {
                display: block;
                height: 100%;
            }

        /* The block IS the anchor - the whole card is the click target. Colour and underline are
           reset so it reads as an article rather than a link, and the title underlines on hover
           AND on keyboard focus, so the two are not told apart. */
        /*  Written as a.SchoolNewsBlock WITH the link pseudo-classes on purpose. Site.css carries
            a global "a, a:link, a:visited, a:hover, a:active { color: blue }", and a:link scores
            0-1-1 against a bare .SchoolNewsBlock's 0-1-0 - so the card's body text inherited blue
            from the anchor. Matching the pseudo-classes takes this to 0-2-1 and it wins. The date
            and title set their own colours and were never affected.                               */
        a.SchoolNewsBlock,
        a.SchoolNewsBlock:link,
        a.SchoolNewsBlock:visited,
        a.SchoolNewsBlock:hover,
        a.SchoolNewsBlock:active {
            display: block;
            height: 100%;
            box-sizing: border-box;
            margin: 0 8px;
            padding: 8px;
            background-color: white;
            color: black;
            text-decoration: none;
        }

            .SchoolNewsBlock:hover .SchoolNewsTitle,
            .SchoolNewsBlock:focus .SchoolNewsTitle {
                text-decoration: underline;
            }

            .SchoolNewsBlock .SchoolNewsImage {
                display: block;
                background-color: black;
                margin-bottom: 8px;
            }

                /* Beats the .SchoolCarouselPanel img rule, which centres and caps at 100% for the
                   poster strip - here the image is the full width of its card. */
                .SchoolNewsBlock .SchoolNewsImage img {
                    display: block;
                    width: 100%;
                    height: auto;
                    margin: 0;
                }

            /* The template's parts are <span> so the whole card can be one anchor - an <a> may
               not contain <div>. They need to be told to behave as blocks. */
            .SchoolNewsBlock .SchoolNewsDate,
            .SchoolNewsBlock .SchoolNewsText {
                display: block;
            }

            .SchoolNewsBlock .SchoolNewsDate {
                font-size: 12px;
                /* #595959 on white is 7:1 - comfortably past the 4.5:1 WCAG 1.4.3 needs for body
                   text, which #999-ish placeholder greys are not. */
                color: #595959;
                margin: 8px 0 8px 0;
            }

            .SchoolNewsBlock .SchoolNewsTitle {
                display: block;
                font-size: 14px;
                font-weight: bold;
                margin: 0 0 10px 0;
                padding: 0;
                background-color: transparent;
                color: #0571b8;
                text-transform: none;
                line-height: 1.3;
                width: auto;
            }

            .SchoolNewsBlock .SchoolNewsText {
                font-size: 12px;
                line-height: 1.5;
                overflow: hidden;
            }

        /* ARROWS INSIDE THE PANEL.

           slick-theme.css parks them at left:-25px / right:-25px, which assumes the carousel has
           25px of empty page either side of it. These panels run the full width of the content
           column, so the arrows landed outside the site. A gutter on the panel gives them
           somewhere to sit, and they are positioned into it. */
        .SchoolCarouselPanel,
        .SchoolNewsPanel {
            padding: 0 34px;
            box-sizing: border-box;
        }

            .SchoolCarouselPanel .slick-prev,
            .SchoolNewsPanel .slick-prev {
                left: 2px;
                z-index: 2;
            }

            .SchoolCarouselPanel .slick-next,
            .SchoolNewsPanel .slick-next {
                right: 2px;
                z-index: 2;
            }

            /* Slick's arrow glyph is white by default and invisible against these cards. */
            .SchoolCarouselPanel .slick-prev:before,
            .SchoolCarouselPanel .slick-next:before,
            .SchoolNewsPanel .slick-prev:before,
            .SchoolNewsPanel .slick-next:before {
                color: #0571b8;
                opacity: 1;
            }

            .SchoolCarouselPanel .slick-prev:focus-visible,
            .SchoolCarouselPanel .slick-next:focus-visible,
            .SchoolNewsPanel .slick-prev:focus-visible,
            .SchoolNewsPanel .slick-next:focus-visible {
                outline: 2px solid #0571b8;
                outline-offset: 2px;
            }
    </style>
    <script>
        $(document).ready(function () {
            if (window.matchMedia('(display-mode: standalone)').matches) $('#installappnote').hide();

            /*  Matches nothing for the 4,048 schools with no carousel, so this is inert there.
                Slick is already loaded by Site.Master - the same copy Default.aspx uses.

                WCAG 2.3.3 / 2.2.2: a reader who has asked the operating system for reduced motion
                gets no autoplay and no slide animation at all. They keep the arrows and dots, so
                nothing becomes unreachable - only the movement they did not ask for stops. */
            var reduceMotion = window.matchMedia
                && window.matchMedia('(prefers-reduced-motion: reduce)').matches;

            $('.SchoolCarouselPanel').slick({
                dots: true,
                arrows: true,
                infinite: true,
                autoplay: !reduceMotion,
                autoplaySpeed: 3000,
                speed: reduceMotion ? 0 : 300,
                slidesToShow: 1,
                slidesToScroll: 1,
                adaptiveHeight: true,
                pauseOnHover: true,
                pauseOnFocus: true
            });

            /*  News: three across like the tenant dashboard's featured strip, down to one on a
                phone. Not autoplayed - three articles are readable side by side and moving text
                someone is part way through reading is worse than no movement at all. */
            $('.SchoolNewsPanel').slick({
                dots: true,
                arrows: true,
                infinite: false,
                autoplay: false,
                speed: reduceMotion ? 0 : 300,
                slidesToShow: 3,
                slidesToScroll: 3,
                responsive: [
                    { breakpoint: 1100, settings: { slidesToShow: 2, slidesToScroll: 2 } },
                    { breakpoint: 800, settings: { slidesToShow: 1, slidesToScroll: 1 } }
                ]
            });

            /*  Name the arrows. Slick's own markup is a bare <button> whose only content is the
                word Previous/Next in a :before, which does not reach the accessibility tree. */
            $('.SchoolCarouselPanel, .SchoolNewsPanel').each(function () {
                $(this).find('.slick-prev').attr('aria-label', 'Previous');
                $(this).find('.slick-next').attr('aria-label', 'Next');
            });

            /*  No visible pause button - Jason's call, 2026-09-18.
             *
             *  WCAG 2.2.2 wants a mechanism to stop content that moves by itself for more than
             *  five seconds. What is left standing in for it: autoplay stops on hover and on
             *  keyboard focus (pauseOnHover / pauseOnFocus above), and never starts at all for a
             *  reader who has asked the system for reduced motion. That covers a reader who
             *  reaches the carousel, but not one who simply wants the movement to stop without
             *  touching it, so this is short of the letter of 2.2.2. The alternative that would
             *  satisfy it without a control is to stop autoplaying entirely. */
        })

        //let deferredPrompt;
        //const installAppButton = document.querySelector("#installAppButton");

        //window.addEventListener("beforeinstallprompt", (e) => {
        //    //alert('here');
        //    // Prevent default prompt and show your button
        //    e.preventDefault();
        //    deferredPrompt = e;
        //    installAppButton.removeAttribute("hidden");
        //});

        //installAppButton.addEventListener("click", async () => {
        //    if (deferredPrompt) {
        //        // Show the prompt and handle user choice
        //        deferredPrompt.prompt();
        //        const { outcome } = await deferredPrompt.userChoice;
        //        console.log(`User response to the install prompt: ${outcome}`);
        //        deferredPrompt = null;
        //        installAppButton.setAttribute("hidden", "");
        //    }
        //});

        //window.addEventListener("appinstalled", () => {
        //    // Hide button and log success
        //    installAppButton.setAttribute("hidden", "");
        //    console.log("PWA was installed");
        //});
    </script>


</head>
<body>
    <iframe id="hidden-iframe" name="hidden-iframe" style="visibility: hidden; position: absolute;"></iframe>
    <form method="post" action="./School.aspx" id="MainForm">
<div class="aspNetHidden">
<input type="hidden" name="__VIEWSTATE" id="__VIEWSTATE" value="SWMxxh3cyeA9MRqjcw05otYXMScPdcpRdOJOKZz9hjLRxdsGtpoKHB/FISBR4DK7SpRLE6QGFw5dt56oW26QO6pY2Ci/3arX4UOkSZtx0TRCdHqrDKVriLWYrbg2fP2jQt+sEYkGvhSCIg6zAVEPS5zIgFiJGxbqbBi158aWNpTUWDYBTLvKPDso8XIFP8SetAJXfJ973tStbUpPlTnYnnn5JCdBPecyUK62sc/68eYbCzJtnbXVdktnissBk+ceStBmoPeJLjId6wRRa+5kWQ2aD601w5UKod0yEFlW9UyedO5osFppZT+Qcm4IKP7/lMpjPCu6Qaqb/AZ9VkhBh+6L8+qU9m3wRNB95A6fmUu3YxNpsxWb2cbUyvYewEKbRpXQrMhY+i5sXaCI8aSeP6Y6FESiQ2PoFjM3l0kMndSd7o6gbIZvBmuLr6nvqWj+iK3a2mH+gRTjA00Wu1LcGVlpOAQxu+aKdUrB2caju2VfYr6cg/UqH5UZl+34WGBWQUNbvzI58TN/m6UAXoiSCN8FwO6/ALc5H339zTLf9uZ8AoeNg9I0iYFAyrNiDC2FFPWpS6BTdNDafuBYFXwX0t4zoaQosG8DmRdkJrUr9Fxfz4wjgQw3yf+eWNWRIfxPHNURdAS+zeSintRSr9EctL9Vm4TwqVaptWreWhQA0LUHn6DEbfLGmK5oc5qum3HmC59pPtd5YYLZsY+BaatZ1dpZRTA4MiHQMqetVeH4y5KRuM3ldGexnyUeqdz8C8AzWK+s5g5IiQF6O2pKnYWHBHDAY9t92yOIvxiDxafIN/bZYysx6mRhtdPsTQmufEyFWrY5zGUNNS8EuLlC8LCNJAHLa6z7FtTqIv0yYOOtyOQ=" />
</div>

<div class="aspNetHidden">

	<input type="hidden" name="__VIEWSTATEGENERATOR" id="__VIEWSTATEGENERATOR" value="B9366D98" />
	<input type="hidden" name="__EVENTVALIDATION" id="__EVENTVALIDATION" value="m1k7ku5nA8AOWzJAwRyKofRpmgzlEbdfwWuNp3qECeLIxWOL9xnx/Qn6Bsj8SSY3Weh1hsaCLCNm/3nbo8NY4BwF6NHbxoh+dr3UF+4iP0mN+WhnZ6WD5swKuMTOlx2ioJZ37FpV+H1OzhNa6lkiQw==" />
</div>
        <div id="dsLoader"></div>

        <div id="dsNavigation" class="dsNavigationOverlay">
	
            <div id="dsNavigationMenu" class="dsNavigationMenu">
                <a href="javascript:void(0)" class="dsNavigationCloseButton" onclick="closeNav()">×</a>
                <div id="dsNavigationMenuContent" class="dsNavigationMenuContent">
		
                    <ul id="dsNavigationMenuUL" class="dsNavigationMenuUL dsNavigationMenuSubPanel active"><li><a id="mnuDashboard" href="\">Dashboard</a></li><li><a id="mnuDirectory" href="\Directory.aspx">School Directory</a></li><li><a id="mnuSportHS" class=" dsNavigationMenuLink " href="https://www.riil.org/SchoolPages/School.aspx#NavigationMenu_mnuSportHS"><div style="float:left;">Sport Info - High School</div><i class="fa fa-chevron-right" style="float:right;"></i><div style="clear:both;"></div></a></li><li><a id="mnuSportMS" class=" dsNavigationMenuLink " href="https://www.riil.org/SchoolPages/School.aspx#NavigationMenu_mnuSportMS"><div style="float:left;">Sport Info - Middle School</div><i class="fa fa-chevron-right" style="float:right;"></i><div style="clear:both;"></div></a></li><li><a id="mnuScheduleHS" href="\DashboardSchedule.aspx">Schedules</a></li><li><a id="mnuMasterSchedule" href="/MasterSchedule.aspx?TeamLevelID=5">Master Schedule</a></li><li><a id="mnuSchoolRoster" href="/DashboardTeamRoster.aspx?SeasonRoster=1">Team Rosters</a></li><li><a id="mnuSchoolPages" href="/SchoolPages/School.aspx">School Pages</a></li><li class="dsNavigationMenuGroupSeperator"></li><li><div class="dsNavigationMenuGroupTitle">Tournament Central</div></li><li><a id="mnuTournamentDashboard" href="\DashboardTournamentCentral.aspx">Dashboard</a></li><li><a id="mnuTournamentRankings" href="\SportPages\SportPageRanking.aspx">Tournament Standings</a></li><li><a id="mnuTournamentRPIDetail" href="\Reports\RPIDetail.aspx">RPI Detail</a></li><li><a id="mnuTournamentBrackets" href="\TournamentCentralBrackets.aspx">Playoff Brackets</a></li><li class="dsNavigationMenuGroupSeperator"></li><li><div class="dsNavigationMenuGroupTitle">Inside the RIIL</div></li><li><a id="mnuSubResources_InsidetheRIIL_AnnualReports" class=" dsNavigationMenuLink " href="https://www.riil.org/SchoolPages/School.aspx#NavigationMenu_mnuSubResources_InsidetheRIIL_AnnualReports"><div style="float:left;">Annual Reports</div><i class="fa fa-chevron-right" style="float:right;"></i><div style="clear:both;"></div></a></li><li><a id="mnuSubResources_69d3a678-ae15-418d-9157-233088cadf97" href="/TenantHTML.aspx?D=Inside the RIIL&amp;F=Contact Us.html">Contact Us</a></li><li><a id="mnuSubResources_fce32c12-570e-436a-beae-d123873036d9" href="/TenantHTML.aspx?D=Inside the RIIL&amp;F=Our Mission.html">Our Mission</a></li><li><a id="mnuSubResources_62cbf554-9088-45f6-be67-99666ecfd07c" href="/TenantHTML.aspx?D=Inside the RIIL&amp;F=Principals Committee On Athletics.html">Principals Committee On Athletics</a></li><li><a id="mnuSubResources_47d6237d-f6d7-4d37-85e7-d6c80475a83c" href="/TenantHTML.aspx?D=Inside the RIIL&amp;F=Office Staff.html">Office Staff</a></li><li><a id="mnuSubResources_e9012d8a-9e0d-4a7f-b573-0bde89836c55" href="https://www.tumblr.com/riilsports" target="_blank">RIIL Blog</a></li><li><a id="mnuSubResources_InsidetheRIIL_HSAthleticHallofFame" class=" dsNavigationMenuLink " href="https://www.riil.org/SchoolPages/School.aspx#NavigationMenu_mnuSubResources_InsidetheRIIL_HSAthleticHallofFame"><div style="float:left;">HS Athletic Hall of Fame</div><i class="fa fa-chevron-right" style="float:right;"></i><div style="clear:both;"></div></a></li><li><a id="mnuSubResources_b9ed8157-6b91-497f-8329-9e0e6b8a309d" href="/resources/Inside%20the%20RIIL/RIIL%20Middle%20School%206-8%20Manual.pdf" target="_blank">RIIL Middle School 6-8 Manual</a></li><li><a id="mnuSubResources_f9249ffe-b64f-4010-b836-397a9afb166f" href="/resources/Inside%20the%20RIIL/RIIL%20Rules%20&amp;%20Regulations%20-%20Articles%201-15.pdf" target="_blank">RIIL Rules & Regulations - Articles 1-15</a></li><li><a id="mnuSubResources_8ac8cffc-5ead-4621-ad3c-c6aa8181d904" href="/CarouselAll.aspx?CarouselID=9" target="_blank">Sponsors</a></li><li class="dsNavigationMenuGroupSeperator"></li><li><div class="dsNavigationMenuGroupTitle">Resources</div></li><li><a id="mnuSubResources_147e04f0-3936-46e8-ad0f-44f91659cc1a" href="/resources/Resources/2026-2027%20Media%20Information.pdf" target="_blank">2026-2027 Media Information</a></li><li><a id="mnuSubResources_96e0a886-fcd2-4dcc-9487-3b3306456e36" href="/TenantHTML.aspx?D=Resources&amp;F=Affiliate Websites.html">Affiliate Websites</a></li><li><a id="mnuSubResources_d5ac0135-83f6-4c19-a17a-fa95fb5c8911" href="/publicdownloads.aspx">Forms & Document Downloads</a></li><li><a id="mnuSubResources_b208954b-5751-4f72-962a-4bcc1cdb5f03" href="https://highschoolofficials.com/" target="_blank">NFHS Become an Official!</a></li><li><a id="mnuSubResources_ae484776-fc2c-4ddd-96c2-1a8633788cbe" href="/TenantHTML.aspx?D=Resources&amp;F=Officials Courses through RefReps.html">Officials Courses through RefReps</a></li><li><a id="mnuSubResources_1580a7fb-ab77-4b28-b633-55c17d815571" href="/TenantHTML.aspx?D=Resources&amp;F=Officials Registration in Arbiter.html">Officials Registration in Arbiter</a></li><li><a id="mnuSubResources_a1a474be-9426-4b4c-b017-155f8a5c8ff6" href="/TenantHTML.aspx?D=Resources&amp;F=Sportsmanship Expectations.html">Sportsmanship Expectations</a></li><li class="dsNavigationMenuGroupSeperator"></li><li><div class="dsNavigationMenuGroupTitle">Students and Parents</div></li><li><a id="mnuSubResources_StudentsandParents_OperationCleanCompetition" class=" dsNavigationMenuLink " href="https://www.riil.org/SchoolPages/School.aspx#NavigationMenu_mnuSubResources_StudentsandParents_OperationCleanCompetition"><div style="float:left;">Operation Clean Competition</div><i class="fa fa-chevron-right" style="float:right;"></i><div style="clear:both;"></div></a></li><li><a id="mnuSubResources_StudentsandParents_StudentInitiatives" class=" dsNavigationMenuLink " href="https://www.riil.org/SchoolPages/School.aspx#NavigationMenu_mnuSubResources_StudentsandParents_StudentInitiatives"><div style="float:left;">Student Initiatives</div><i class="fa fa-chevron-right" style="float:right;"></i><div style="clear:both;"></div></a></li><li><a id="mnuSubResources_StudentsandParents_TrafficSafetyIsATeamSport" class=" dsNavigationMenuLink " href="https://www.riil.org/SchoolPages/School.aspx#NavigationMenu_mnuSubResources_StudentsandParents_TrafficSafetyIsATeamSport"><div style="float:left;">Traffic Safety Is A Team Sport</div><i class="fa fa-chevron-right" style="float:right;"></i><div style="clear:both;"></div></a></li><li><a id="mnuSubResources_481d4698-3ad0-458c-a2ec-b61798b942c1" href="/TenantHTML.aspx?D=Students and Parents&amp;F=Alice Sullivan Memorial Scholarship.html">Alice Sullivan Memorial Scholarship</a></li><li><a id="mnuSubResources_32a1eb80-4496-47ad-866f-289c9c82a81a" href="/resources/Students%20and%20Parents/Local%2051%20Trades%20Scholarship.pdf" target="_blank">Local 51 Trades Scholarship</a></li><li><a id="mnuSubResources_1773b511-fd50-4462-a890-fd82123774d3" href="/resources/Students%20and%20Parents/RI%20Council%20on%20Problem%20Gambling%20Corner.pdf" target="_blank">RI Council on Problem Gambling Corner</a></li><li class="dsNavigationMenuGroupSeperator"></li><li><div class="dsNavigationMenuGroupTitle">Account</div></li><li><a id="mnuLogin" href="../Login.aspx?ReturnUrl=%2fSchoolPages%2fSchool.aspx">Login</a></li><li><a id="mnuSignUp" href="../SignUp.aspx?ReturnUrl=%2fSchoolPages%2fSchool.aspx">Sign-Up</a></li><li><a id="mnuPasswordReset" href="\PasswordReset.aspx">Forgot Password</a></li></ul>
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
                    <a id="Signup" title="Sign-Up" class="dsHeaderButton" href="../SignUp.aspx?ReturnUrl=%2fSchoolPages%2fSchool.aspx">Sign-Up</a>
                    <a id="Login" title="Login" class="dsHeaderButton" href="../Login.aspx?ReturnUrl=%2fSchoolPages%2fSchool.aspx">Login</a>
                </div>
            
	</div>



            <div class="dsMiddlePanel">

                

    <div class="dsSiteContent">

        
        <div id="NoticePanel" class="dsSiteNotice" role="status">
		
            <i class="fa fa-circle-info" aria-hidden="true"></i>
            We are currently applying upgrades to the site. You may experience temporary display issues while the migration completes.
        
	</div>

        
        

        
        

        
        <main class="dsSiteContentBody">
            

    <div id="Panel1" class="dsFormPanelFSwTB ">
		
        <div id="HeaderWrapper">
			
        
		</div>

        <div id="ToolbarPanel" class="dsToolbarPanel">
            
        </div>

        <div id="BodyWrapper" class="school-content">
			
        <h3>All schools that are members of the association have a custom page.<br/>Please search for your school by name.<br/></h3><div id="SearchWrapper" class="t" style="display:block;width:300px;margin:10px;">
				<span id="Search" class="dsInputWrapper"><label for="Search_Text" id="Search_Label" class="dsInputLabel">Enter the name of the school to search for.</label><input name="ctl00$ctl00$MainContent$MainContent$Search$Search_Text" type="text" maxlength="50" id="Search_Text" class="dsInput dsInputText" data-parsley-group="default" data-parsley-errors-container="#ValidationStatus" onkeydown="return (event.keyCode!=13);" /></span><input type="submit" name="ctl00$ctl00$MainContent$MainContent$ctl00" value="Search" class="dsButton" />
			</div><br/><br/><div class='SchoolSearchResults'><h2 style='clear:both;'>High Schools</h2><a href='/SchoolPages/School.aspx?SchoolID=1'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_1.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Achievement First (PVD) High School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=2'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_2.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Barrington High School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=3'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_3.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Bishop Hendricken High School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=4'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_4.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Blackstone Valley Preparatory Mayoral Academy</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=5'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_5.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Block Island High School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=6'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_6.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Burrillville High School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=8'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_8.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Central Falls High School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=7'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_7.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Central High School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=9'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_9.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Chariho High School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=45'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_45.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Charles E. Shea High School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Chesterton Academy of Our Lady of Hope</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=11'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_11.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Classical High School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=12'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_12.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Coventry High School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=13'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_13.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Cranston East High School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=14'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_14.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Cranston West High School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=15'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_15.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Cumberland High School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=16'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_16.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Davies Career & Technical High School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=17'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_17.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>East Greenwich High School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=18'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_18.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>East Providence High School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=1071'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_1071.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Excel Academy Rhode Island High School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=19'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_19.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Exeter-West Greenwich High School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=20'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_20.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Hope High School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=21'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_21.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Johnston High School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=22'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_22.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Juanita Sanchez Educational Complex</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=23'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_23.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>La Salle Academy</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=24'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_24.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Lincoln High School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=25'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_25.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Lincoln School - Providence</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=26'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_26.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Middletown High School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=27'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_27.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Moses Brown School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=28'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_28.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Mt. Hope High School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=29'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_29.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Mt. Pleasant High School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=30'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_30.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Narragansett High School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=31'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_31.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>North Kingstown High School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=32'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_32.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>North Providence High School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=33'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_33.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>North Smithfield High School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=35'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_35.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Pilgrim High School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=36'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_36.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Ponaganset High School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=37'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_37.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Portsmouth High School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=38'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_38.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Prout School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=39'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_39.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Providence Country Day School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10127'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10127.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Rhode Island Interscholastic League</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=40'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_40.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Rogers High School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=43'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_43.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Saint Raphael Academy</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=44'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_44.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Scituate High School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=46'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_46.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Smithfield High School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=47'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_47.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>South Kingstown High School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=41'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_41.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>St. Mary Academy - Bay View</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=42'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_42.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>St. Patrick Academy</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=48'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_48.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Times2 STEM Academy</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=49'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_49.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Tiverton High School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=50'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_50.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Toll Gate High School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=53'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_53.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>West Warwick High School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=52'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_52.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Westerly High School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=51'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_51.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>William E. Tolman High School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=54'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_54.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Woonsocket High School</p></div></div></a><h2 style='clear:both;'>Middle Schools</h2><a href='/SchoolPages/School.aspx?SchoolID=10301'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10301.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Achievement First Illuminar Academy Middle School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10314'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10314.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Achievement First Providence Academy Middle School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10282'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10282.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Alan Shawn Feinstein Middle School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10321'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10321.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Archie Cole Middle School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10296'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10296.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Barrington Middle School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10290'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10290.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Birchwood Middle School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10271'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10271.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Bishop Hendricken 8th Grade Select</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10299'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10299.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Blackstone Valley Prep Junior High School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10310'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10310.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Block Island Middle School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10275'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10275.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Broad Rock Middle School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10277'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10277.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Burrillville Middle School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10274'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10274.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Chariho Middle School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10315'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10315.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Christopher DelSesto Middle School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=1017'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_1017.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Community Preparatory Middle School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=1003'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_1003.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Compass School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10256'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10256.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Davisville Middle School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10265'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10265.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>De La Salle Middle School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10292'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10292.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Dr. E.A. Ricci Middle School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10261'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10261.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Dr. Earl F. Calcutt Middle School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10259'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10259.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Edward R. Martin Middle School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10316'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10316.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Esek Hopkins Middle School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=1025'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_1025.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Excel Academy</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10269'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10269.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Exeter-West Greenwich Junior High School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=1080'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_1080.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Founders Academy</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10298'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10298.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Frank E. Thompson Middle School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=1035'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_1035.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Frank Spaziano Middle School (PPSD)</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=1083'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_1083.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Good Shepherd Catholic School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10304'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10304.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Gordon School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=1036'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_1036.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Harry Kizirian Middle School (PPSD)</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10175'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10175.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Hope Highlands Middle School </p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10295'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10295.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Hugh B. Bain Middle School </p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10302'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10302.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Jamestown Lawn Middle School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10264'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10264.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>John Deering Middle School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10294'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10294.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Joseph H. Gaudet Middle School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10306'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10306.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Joseph Jenks Middle School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10279'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10279.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Joseph L. McCourt Middle School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10276'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10276.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Kickemuit Middle School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10284'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10284.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Lincoln Middle School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=1000'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_1000.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Lincoln School Middle School- Providence</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10305'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10305.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Lyman B. Goff Middle School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=100036'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_100036.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Mary Fogarty Middle School (PPSD)</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10303'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10303.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Monsignor Clarke Middle School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10267'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10267.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Moses Brown Middle School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10300'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10300.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Narragansett Pier Middle School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10317'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10317.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Nathan Bishop Middle School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10318'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10318.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Nathanael Greene Middle School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10258'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10258.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Nicholas A. Ferri Middle School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10278'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10278.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>North Cumberland Middle School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10266'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10266.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>North Smithfield Middle School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10289'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10289.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Our Lady of Mercy Middle School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10131'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10131.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Park View Middle School </p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10262'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10262.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Ponaganset Middle School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10272'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10272.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Portsmouth Middle School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10311'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10311.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Providence Country Day School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=1018'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_1018.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Providence Preparatory Middle School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=1004'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_1004.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Quest Montesorri School </p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10260'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10260.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Riverside Middle School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=100037'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_100037.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Robert F. Kennedy Middle School (PPSD)</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10319'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10319.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Roger Williams Middle School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10307'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10307.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Samuel Slater Middle School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10280'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10280.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Scituate Middle School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10297'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10297.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>St. Mary Academy Bay View Middle School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10293'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10293.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Tiverton Middle School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10291'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10291.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Urban Collaborative Accelerated Program MS</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10287'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10287.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Vincent J. Gallagher Middle School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10323'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10323.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Warwick Veterans Middle School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=1005'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_1005.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>West Bay Christian Academy</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10320'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10320.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>West Broadway Middle School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10263'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10263.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Westerly Middle School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10132'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10132.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Western Hills Middle School </p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10257'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10257.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Wickford Middle School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10270'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10270.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Winman Middle School</p></div></div></a><a href='/SchoolPages/School.aspx?SchoolID=10312'><div class='SchoolListWrapper'><div class='SchoolListImageWrapper'><img class='SchoolListLogo' src='/resources/_media/SchoolLogos/SchoolLogo_10312.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'"  /></div><div class='SchoolListName'><p>Woonsocket Middle School</p></div></div></a></div>
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


    

<script type="text/javascript">
//<![CDATA[
  $('#Search_Text').change(function () {DetectChange();});//]]>
</script>
</form>
</body>
</html>
