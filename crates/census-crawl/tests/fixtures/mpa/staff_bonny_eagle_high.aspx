

<!DOCTYPE html>

<html lang="en">
<head><meta name='description' content='The Maine Principals' Association is a nonprofit serving Maine schools through high school interscholastic athletics governance and professional support for Kâ€“12 administrators.'/><link href="/_Styles/2010/Config.css?v=20260924a" rel="stylesheet" type="text/css" /><link href="/_Styles/Controls.css?v=20260924a" rel="stylesheet" type="text/css" /><link href="/_Styles/Dashboard.css?v=20260924a" rel="stylesheet" type="text/css" /><link href="/_Styles/TournamentCentral.css?v=20260924a" rel="stylesheet" type="text/css" /><meta charset="utf-8" /><title>
	FusionPoint Sports - Maine Principal's Association
</title><meta http-equiv="Content-Type" content="text/html; charset=utf-8" /><meta id="Viewport" name="viewport" content="width=device-width, initial-scale=1, minimal-ui" /><meta name="mobile-web-app-capable" content="yes" /><meta name="apple-mobile-web-app-capable" content="yes" /><meta http-equiv="Cache-control" content="max-age=300" /><link id="SiteCss" rel="stylesheet" type="text/css" href="/_Styles/Site.css?v=20260924a" /><link href="/_Styles/School/Config.css?v=20260924a" rel="stylesheet" type="text/css" /><link href="/_Styles/School/Site.css?v=20260924a" rel="stylesheet" type="text/css" /><link rel="stylesheet" type="text/css" href="/_Styles/parsley.css" /><link rel="stylesheet" type="text/css" href="/_Styles/jquery-ui.css" /><link rel="stylesheet" type="text/css" href="/_Fonts/Awesome/css/fontawesome.css" /><link rel="stylesheet" type="text/css" href="/_Fonts/Awesome/css/regular.css" /><link rel="stylesheet" type="text/css" href="/_Fonts/Awesome/css/solid.css" /><link rel="stylesheet" type="text/css" href="../_Fonts/Awesome/css/duotone.css" /><link href="../favicon.ico" rel="shortcut icon" type="image/x-icon" />

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

            // The full-height report panel scrolls on its own, so a keyboard user has to be able to
            // focus it to scroll it with the arrow keys (WCAG 2.1.1). Done here rather than on the
            // 100-plus pages that declare one. A page that set its own tabindex keeps it.
            $('.dsFormPanelFSwTB').each(function () {
                if (!this.hasAttribute('tabindex')) this.setAttribute('tabindex', '0');
            });

            bindToolbarMenuFunctions('SchoolMenu_');


        });

        // Slick marks the slides that are off screen aria-hidden, but leaves their links in the tab order,
        // so a keyboard user tabs onto things a screen reader has been told are not there (WCAG 4.1.2).
        // Bound on document, not per slider, so every .slick() on every page is covered - including those
        // a page starts in its own ready handler before this one runs.
        $(document).on('init reInit afterChange setPosition', '.slick-slider', function () {
            $(this).find('.slick-slide[aria-hidden="true"]').find('a, button, input, select, textarea, [tabindex]').attr('tabindex', '-1');
            $(this).find('.slick-slide[aria-hidden="false"]').find('a, button, input, select, textarea').removeAttr('tabindex');
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



    
    <link id="ShellCss" rel="stylesheet" type="text/css" href="/_Styles/SiteShell.css?v=20260924a"></link>

    

    <link href="../_Styles/GameCalendar.css?5" rel="stylesheet" />

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
    <form method="post" action="./School.aspx?SchoolID=3&amp;tab=staff" id="MainForm">
<input type="hidden" name="__VIEWSTATE" id="__VIEWSTATE" value="6sfBvySRB7123lN/0dgk8CvUIGgpVmZI9l16iCSBQ6WCMaDdD8BAmyXtqmk3dlxpg2YTdbif+YJlGmlFMmbhP4YdPn9SVYQWLejOYgUR9alOqgYj283kbfnt4AI5xLaW2nOYOTlJB0dZdQd/J0SdeVdJaHoO+tqUzE5UbXA3suoGtibAsE35BhJ6rWtAPrt66/XFEMIuZwLeoTlvcZDw4OhY6MeXO/Tj9DdTGNUFJbPxtq8i2W47rFW1xX6Sy7vSKezMYMz2n+3F9KB2wVYH8lrNJx/EQfFLNu/Y2fWcNsAH6gUp4/Z7mNBQTkGeRT3m3/ahpNriO/W5vUkk/FgnB5nT0ZIs4r8ssh6EJXFBhBqUawiv58TrX2uizrfjNMXQyo6SVevV8fObATK1ONSqMoASIebrsucMCi19d4o/qCtKT+MLnhb97E/moIiDQLHIqvmsfssWqKKb98upNTRhPucXneaqonXbUAKElJTLhB5gpgpWi7jjDdxlnLxoiwfTkSy5bkFSaQBWg9WEhSyCuOkmtrUZv9uFIswbS+AJxiMKvv5NzpkAfV/tJUVwGenQqjWVaOKTzilI2b2oYm0XORDEInGCptcF5KWKul8NZFAESpPq2Spnll/2azcG0OENq4Z5e5vY06bqMyHn/vljIyIP7p5/G9Fsq9dtDCAzgPgZGCDJocE7xl96PpZB4DXCnHMaVQSdk6AJ0Qux5cfnfwev2oVVrrbfb5ErQ0k4GpbMEzg48L5dchpZr20cGgN5a1f9PGAjZj0yxJdUBJjmbc2eOy8L2fCcno2XEsqQ+1dznOM6fyLL5kN46MzVaY3OrbeV9VtOdz04odBzgwnqUET9KClWx51LixhYELaezODYL0AYhDOEn/WalcLQ/Vy+ZZAk3usK/VtY0iCKZbytHA==" />

<input type="hidden" name="__VIEWSTATEGENERATOR" id="__VIEWSTATEGENERATOR" value="B9366D98" />
        <div id="dsLoader"></div>

        <div id="dsNavigation" class="dsNavigationOverlay">
	
            <div id="dsNavigationMenu" class="dsNavigationMenu">
                <a href="javascript:void(0)" class="dsNavigationCloseButton" onclick="closeNav()">×</a>
                <div id="dsNavigationMenuContent" class="dsNavigationMenuContent">
		
                    <ul id="dsNavigationMenuUL" class="dsNavigationMenuUL dsNavigationMenuSubPanel active"><li><a id="mnuHome" href="\SchoolPages\School.aspx?SchoolID=3">School Home Page</a></li><li><a id="mnuHomeAssoc" href="\">Association Home Page</a></li><li class="dsNavigationMenuGroupSeperator"></li><li><div class="dsNavigationMenuGroupTitle">Account</div></li><li><a id="mnuLogin" href="../Login.aspx?ReturnUrl=%2fSchoolPages%2fSchool.aspx%3fSchoolID%3d3%26tab%3dstaff">Login</a></li><li><a id="mnuSignUp" href="../SignUp.aspx?ReturnUrl=%2fSchoolPages%2fSchool.aspx%3fSchoolID%3d3%26tab%3dstaff">Sign-Up</a></li><li><a id="mnuPasswordReset" href="\PasswordReset.aspx">Forgot Password</a></li></ul>
                    <div style="display: none">
                        <ul id="dsNavigationMenuHidden" class="dsNavigationMenuUL dsNavigationMenuSubPane"></ul>
                    </div>
                
	</div>
            </div>
        
</div>

        <div id="dsPagePanel" class="dsPagePanel">
	

            <div id="dsHeaderPanel" class="dsHeaderPanel">
		
                
                
                <ul id="ToolbarUL" class="dsHeaderMenu" role="none">
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
                        
                        <a href="/SchoolPages/School.aspx?SchoolID=3" id="LogoLink" onclick="changeToMainMenu();">
                            <img id="LogoImage" class="fpLogoImage" onerror="this.onerror=null;this.src=&#39;/resources/_LogoSmall.png?t=2010&#39;;" src="/resources/_media/SchoolLogos/SchoolLogo_3.jpg?t=2010" alt="Home" /></a>
                        <!--<div class="dsLogoTextDatable">datable</div>-->
                    </li>

                </ul>
                <div id="WelcomeText" class="dsWelcomeText">
                    <a id="Signup" title="Sign-Up" class="dsHeaderButton" href="../SignUp.aspx?ReturnUrl=%2fSchoolPages%2fSchool.aspx%3fSchoolID%3d3%26tab%3dstaff">Sign-Up</a>
                    <a id="Login" title="Login" class="dsHeaderButton" href="../Login.aspx?ReturnUrl=%2fSchoolPages%2fSchool.aspx%3fSchoolID%3d3%26tab%3dstaff">Login</a>
                </div>
            
	</div>



            <div class="dsMiddlePanel">

                

    <div class="dsSiteContent">

        
        

        
        

        
        

        
        <main class="dsSiteContentBody">
            

    <div id="Panel1" class="dsFormPanelFSwTB ">
		
        <div id="HeaderWrapper">
			
        
                <table width=100%>
                <tr>
                <td style='width:25%;padding-right:20px;' align='right'>
                    <img class='SchoolLogo' alt='' src='/resources/_media/SchoolLogos/SchoolLogo_3.jpg' onerror="this.onerror=null;this.src='/resources/_LogoSmall.png'" />
                </td>
                <td style='width:75%;' align='left'>
                <div class='SchoolName'>Bonny Eagle High School </div>
                </td>
                </tr>
                </table>
            
		</div>

        <div id="ToolbarPanel" class="dsToolbarPanel">
            <ul id="Toolbar" class="dsToolbar">
                <li class="dsToolbarLeft dsToolbarButton">
                    <a id="Home" title="Home" aria-label="Home" onclick="NavURLWithParamChange(&#39;tab&#39;, &#39;home&#39;); return false;" href="#" target="_blank"><i class="fa-solid fa-house" aria-hidden="true"></i></a>
                </li>
                <li class="dsToolbarLeft dsToolbarButton">
                    <a id="Association" title="Association" aria-label="Association Home" href="/" target="_self" style="margin: 0; padding: 0;"><img Style="height: 40px;" src="/resources/_LogoSmall.png" alt="" /></a>
                </li>
                <li class="dsToolbarLeft dsToolbarButton">
                    
                </li>
                <li class="dsToolbarLeft dsToolbarButton">
                    <a id="Schedules" title="Schedules" onclick="NavURLWithParamChange(&#39;tab&#39;, &#39;schedules&#39;); return false;" href="#" target="_blank">Schedules</a>
                </li>
                <li class="dsToolbarLeft dsToolbarButton">
                    <a id="Rosters" title="Rosters" onclick="NavURLWithParamChange(&#39;tab&#39;, &#39;rosters&#39;); return false;" href="#" target="_blank">Rosters</a>
                </li>
                <li class="dsToolbarLeft dsToolbarButton">
                    <a id="Staff" title="Staff" onclick="NavURLWithParamChange(&#39;tab&#39;, &#39;staff&#39;); return false;" href="#" target="_blank">Staff</a>
                </li>
                <li class="dsToolbarLeft dsToolbarButton">
                    <a id="Registration" title="Registration" onclick="NavURLWithParamChange(&#39;tab&#39;, &#39;registration&#39;); return false;" href="#" target="_blank">Registration</a>
                </li>
                
                <li class="dsToolbarLeft dsToolbarButton">
                    
                </li>
                <li class="dsToolbarLeft dsToolbarButton">
                    
                </li>
            </ul>
        </div>

        <div id="BodyWrapper" class="school-content">
			
        <div class='DirectoryDetail'><b>Bonny Eagle High School</b><br />700 Saco Road, Standish, ME 04084<br /><div class='DirectoryFixedWidth1'>School Phone:</div>207-929-3840<br /><div class='DirectoryFixedWidth1'>School Fax: </div>207-910-2773<br /><table class='DirectoryStaffTable'><tr><th></th><th>Role</th><th>Name</th><th>Phone</th></tr><tr><td></td><td>Principal</td><td>Ted Finn</td><td><a href='tel:'></a></td></tr><tr><td></td><td>Athletic Director</td><td>Eric Curtis, CAA</td><td><a href='tel:207-929-3840'>207-929-3840</a></td></tr><tr><td></td><td>Athletic Trainer</td><td>Jenna McCurdy, MS-ATC</td><td><a href='tel:'></a></td></tr><tr><td></td><td>Assistant Principal</td><td>Alicia Davis</td><td><a href='tel:'></a></td></tr><tr><td></td><td>Assistant Principal</td><td>EJ Kruse</td><td><a href='tel:'></a></td></tr><tr><td></td><td>Admin Assistant</td><td>Shelley Barrows</td><td><a href='tel:207-642-7805'>207-642-7805</a></td></tr><tr><td></td><td>Other</td><td>Tom Landberg</td><td><a href='tel:'></a></td></tr><tr><td>Boys Baseball</td><td>Head Coach</td><td>Adam Begos</td><td><a href='tel:'></a></td></tr><tr><td>Boys Basketball</td><td>Head Coach</td><td>John Trull</td><td><a href='tel:207-929-3833'>207-929-3833</a></td></tr><tr><td>Boys Cross Country</td><td>Head Coach</td><td>Ben Davis</td><td><a href='tel:'></a></td></tr><tr><td>Boys Football</td><td>Head Coach</td><td>Kevin Cooper</td><td><a href='tel:'></a></td></tr><tr><td>Boys Indoor Track</td><td>Head Coach</td><td>Jacob Newcomb</td><td><a href='tel:'></a></td></tr><tr><td>Boys Lacrosse</td><td>Head Coach</td><td>Andrew Slefinger</td><td><a href='tel:'></a></td></tr><tr><td>Boys Outdoor Track</td><td>Head Coach</td><td>Jacob Newcomb</td><td><a href='tel:'></a></td></tr><tr><td>Boys Soccer</td><td>Head Coach</td><td>Haidar Al-Freihy</td><td><a href='tel:'></a></td></tr><tr><td>Boys Swimming</td><td>Head Coach</td><td>Amanda Kry</td><td><a href='tel:'></a></td></tr><tr><td>Boys Tennis</td><td>Head Coach</td><td>Jordan Roche</td><td><a href='tel:'></a></td></tr><tr><td>Coed Fall Cheerleading</td><td>Head Coach</td><td>Shelly Warren</td><td><a href='tel:'></a></td></tr><tr><td>Coed Winter Cheerleading</td><td>Head Coach</td><td>Shelly Warren</td><td><a href='tel:'></a></td></tr><tr><td>Coed Wrestling</td><td>Head Coach</td><td>Caleb Frost</td><td><a href='tel:'></a></td></tr><tr><td>Girls Basketball</td><td>Head Coach</td><td>EJ Regan</td><td><a href='tel:'></a></td></tr><tr><td>Girls Cross Country</td><td>Head Coach</td><td>Tom Noonan</td><td><a href='tel:'></a></td></tr><tr><td>Girls Field Hockey</td><td>Head Coach</td><td>Dawn Staples</td><td><a href='tel:'></a></td></tr><tr><td>Girls Indoor Track</td><td>Head Coach</td><td>Ben Davis</td><td><a href='tel:'></a></td></tr><tr><td>Girls Lacrosse</td><td>Head Coach</td><td>Ashley Dyer</td><td><a href='tel:'></a></td></tr><tr><td>Girls Outdoor Track</td><td>Head Coach</td><td>Ben Davis</td><td><a href='tel:'></a></td></tr><tr><td>Girls Soccer</td><td>Head Coach</td><td>Michelle Riesbeck</td><td><a href='tel:'></a></td></tr><tr><td>Girls Softball</td><td>Head Coach</td><td>Travis Demmons</td><td><a href='tel:'></a></td></tr><tr><td>Girls Swimming</td><td>Head Coach</td><td>Amanda Kry</td><td><a href='tel:'></a></td></tr><tr><td>Girls Tennis</td><td>Head Coach</td><td>John Pelletier</td><td><a href='tel:'></a></td></tr><tr><td>Girls Volleyball</td><td>Head Coach</td><td>Kelley Champagne</td><td><a href='tel:'></a></td></tr></table></div>
                <div style='display:block;float:right;font-size: 12px;margin-top:60px;'>
Powered By: <br/> <img style='margin-top:8px; height:50px;' src='/_Images/FPSportsLogo.png' alt='FusionPoint Sports' />
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
