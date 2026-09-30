"use strict";


/********************************************************************************************************/
/* GLOBAL VARIABLES                                                                                     */
/********************************************************************************************************/

// Geolocation
var GeoLocationAvailable = false;
var Latitude = 0;
var Longitude = 0;



/********************************************************************************************************/
/* LOAD/UNLOAD                                                                                          */
/********************************************************************************************************/

$(document).ready(function () {

    //Bind Tabs
    bindTabFunctions();
    //if (document.location.hash) changeTab(document.location.hash);

    //Bind Menu
    bindMenuFunctions();

    //Decode EMail
    decodeEmail();

    // Initial calculation
    calculateVh();
    window.addEventListener('resize', calculateVh);
    window.addEventListener('orientationchange', calculateVh);
    
    //Hide Spinner
    $("#dsLoader").hide();

})

$(window).on("beforeunload", function () {
    //$("#dsLoader").show();
})

function calculateVh() {
    var vh = window.innerHeight;
    document.documentElement.style.setProperty('--vh', vh + 'px');
}


/********************************************************************************************************/
/* DECODE EMAIL                                                                                         */
/********************************************************************************************************/

function decodeEmail() {
    $('.dsm').each(function () {
        this.href = 'mailto:' + $(this).attr('mm') + '@' + $(this).attr('ed') + '.' + $(this).attr('ec');
        if (this.innerHTML=="") this.innerHTML = this.href.replace('mailto:', '');
    });
};


/********************************************************************************************************/
/* NAVIGATION                                                                                           */
/********************************************************************************************************/

function openNav() {
    $('.dsNavigationOverlay').css({ "display": "block" });
    $('.dsNavigationMenuOpen').css({ "display": "block", "visibility": "visible" });
    $('.dsNavigationMenuClosed').css({ "display": "none", "visibility": "hidden" });
    //$('.fpLogoImage').css({ "height": "100px" });
    $('body').css('overflow', 'hidden');
    //changeToMainMenu();
}

function closeNav() {
    $('.dsNavigationOverlay').css({ "display": "none" });
    $('.dsNavigationMenuClosed').css({ "display": "block", "visibility": "visible" });
    $('.dsNavigationMenuOpen').css({ "display": "none", "visibility": "hidden" });
    //$('.fpLogoImage').css({ "height": "47px" });
    $('body').css('overflow', 'auto');
}

function NavURLWithParamChange(Param, Value) {
    var url = new URL(window.location.href);
    var params = url.searchParams;
    params.set(Param, Value);
    window.location.href = url.toString();
}


/********************************************************************************************************/
/* MENU                                                                                                 */
/********************************************************************************************************/

function bindMenuFunctions() {
     $(document)
         .on("click", "a[href*='#NavigationMenu_']:not('.active')", function (event) {
            changeMenu('#' + this.hash.split('#')[1]);
            event.preventDefault();
        })
    var lastHash = sessionStorage.getItem("ActiveMenu") ?? "";
    if (lastHash != "") changeMenu(lastHash);
}

function changeMenu(hash) {
    $(hash).addClass("active").siblings().removeClass("active");
    sessionStorage.setItem("ActiveMenu", hash);
}

function changeToMainMenu() {
    $('#dsNavigationMenuUL').addClass("active").siblings().removeClass("active");
    sessionStorage.setItem("ActiveMenu", "");
}




/********************************************************************************************************/
/* TOOLBAR MENU                                                                                         */
/********************************************************************************************************/

function bindToolbarMenuFunctions(hashBase) {
    $(document)
        .on("click", "a[href^='#" + hashBase + "']:not('.active')", function (event) {
            this.focus({ preventScroll: true });
            changeToolbarMenu(this.hash);
            return false;
        })
}

function changeToolbarMenu(hash) {
    $(hash).addClass("active").siblings().removeClass("active");
}

function changeToolbarToMainMenu(hashMain) {
    $(hashMain).addClass("active").siblings().removeClass("active");
}


/********************************************************************************************************/
/* TABS                                                                                           */
/********************************************************************************************************/

function bindTabFunctions() {
    $(".dsTabs a").click(function (event) {
        changeTab(this.hash);
        event.preventDefault();
    })
}

function changeTab(hash) {
    $("[href='" + hash + "']").addClass("active").parent().siblings().find("a").removeClass("active");
    $(hash).addClass("active").siblings().removeClass("active");
}



/********************************************************************************************************/
/* Helper Functions                                                                                           */
/********************************************************************************************************/


function updateQueryStringParameter(uri, key, value) {
    var re = new RegExp("([?&])" + key + "=.*?(&|$)", "i");
    var separator = uri.indexOf('?') !== -1 ? "&" : "?";
    if (uri.match(re)) {
        return uri.replace(re, '$1' + key + "=" + value + '$2');
    }
    else {
        return uri + separator + key + "=" + value;
    }
}


/********************************************************************************************************/
/* PASSWORD SHOW / HIDE                                                                                 */
/*                                                                                                      */
/* The eye button next to a password field, emitted by the dsTextbox control (see AddPasswordEye).       */
/* It lives here rather than in a stylesheet-adjacent script because Application.js is the one file all  */
/* three master pages load - Site, OrganizationSite and SchoolSite - and the control is used by all      */
/* three.                                                                                               */
/*                                                                                                      */
/* Toggling input.type is what actually reveals the text. Nothing is stored and nothing is sent: this is */
/* the browser showing you what you already typed.                                                       */
/********************************************************************************************************/

function dsTogglePassword(button) {

    var wrap = button.parentNode;
    if (!wrap) return;

    var input = wrap.querySelector("input");
    if (!input) return;

    var show = input.type === "password";

    input.type = show ? "text" : "password";

    var icon = button.querySelector("i");
    if (icon) icon.className = show ? "fa-solid fa-eye-slash" : "fa-solid fa-eye";

    button.title = show ? "Hide password" : "Show password";
    button.setAttribute("aria-label", button.title);

    /* The tap moves focus to the button, which on a phone closes the keyboard and loses the caret.
       Putting it back leaves the field exactly where it was. */
    try {
        var at = input.value.length;
        input.focus();
        if (input.setSelectionRange) input.setSelectionRange(at, at);
    } catch (e) { }
}
