(function ($) {
    function isMobileDevice() {
        return (typeof Drivefaze !== "undefined" &&
            !!Drivefaze.Segment &&
            Drivefaze.Segment.DEVICE_MOBILE === Drivefaze.Segment.detectDevice());
    }
    if (isMobileDevice()) {
        _DF_.Ads.stickyMoney();
    }
    window.scrolling = {
        down: false,
        up: false,
        viewport_top: 0,
        update: function () {
            var viewport_top = $(window).scrollTop();
            window.scrolling.up = !(viewport_top > window.scrolling.viewport_top);
            window.scrolling.down = !window.scrolling.up;
            window.scrolling.viewport_top = viewport_top;
            if (window.scrolling.viewport_top <= 0) {
                window.scrolling.up = true;
                window.scrolling.down = false;
            }
        },
    };
    $(window).scroll(window.scrolling.update);
    $(document).ready(function () {
        $(window).scroll(sidebarStalker);
    });
    function sidebarStalker() {
        var $window = $(window);
        var $content = $("#content");
        var $sidebar = $("#side");
        var $sidebarContent = $($sidebar.children()[0]);
        $sidebar.css({
            "min-height": $sidebar.outerHeight(),
            position: "relative",
        });
        var sidebarWidth = $sidebar.outerWidth();
        var sidebarTop = $sidebar.offset().top + parseInt($sidebar.css("padding-top"), 10);
        var sidebarBottom = sidebarTop + $sidebar.outerHeight();
        var sidebarContentTop = $sidebarContent.offset().top;
        var sidebarContentBottom = sidebarContentTop + $sidebarContent.outerHeight();
        var windowScrollTop = $window.scrollTop();
        var windowScrollBottom = windowScrollTop + $window.height();
        $sidebarContent.clear = function () {
            $sidebarContent
                .removeClass("sidebar-frozen")
                .removeClass("sidebar-frozen-bottom")
                .removeClass("sidebar-follow")
                .removeClass("sidebar-sticky")
                .removeClass("sidebar-sticky-bottom")
                .removeClass("sidebar-sticky-top")
                .removeAttr("style");
            $sidebar.css({ "min-height": "", position: "" });
        };
        $sidebarContent.isFollow = function () {
            return $sidebarContent.hasClass("sidebar-follow");
        };
        $sidebarContent.isFrozen = function () {
            return $sidebarContent.hasClass("sidebar-frozen");
        };
        $sidebarContent.isSticky = function () {
            return $sidebarContent.hasClass("sidebar-sticky");
        };
        $sidebarContent.enableFollow = function (position) {
            var positionMap = {
                bottom: sidebarTop + $sidebar.outerHeight() - sidebarContentBottom,
                top: Math.abs(sidebarTop - sidebarContentTop),
            };
            if (!$sidebarContent.isFollow()) {
                var sidebarPadding = parseInt($sidebar.css("padding-left"), 10) +
                    parseInt($sidebar.css("padding-right"), 10);
                $sidebarContent.clear();
                $sidebarContent
                    .addClass("sidebar-follow")
                    .css(position, positionMap[position])
                    .css("width", sidebarWidth - sidebarPadding);
            }
        };
        $sidebarContent.enableFrozen = function () {
            if (!$sidebarContent.isFrozen()) {
                $sidebarContent.clear();
                $sidebarContent
                    .addClass("sidebar-frozen")
                    .addClass("sidebar-frozen-bottom");
            }
        };
        $sidebarContent.enableSticky = function (position) {
            if (!$sidebarContent.isSticky()) {
                var sidebarPadding = {
                    left: parseInt($sidebar.css("padding-left"), 10),
                    right: parseInt($sidebar.css("padding-right"), 10),
                };
                $sidebarContent.clear();
                $sidebarContent
                    .addClass("sidebar-sticky")
                    .addClass("sidebar-sticky-" + position)
                    .css({
                    left: $sidebar.offset().left + sidebarPadding.left,
                    width: sidebarWidth - (sidebarPadding.left + sidebarPadding.right),
                });
            }
            else if (!$sidebarContent.hasClass("sidebar-sticky-" + position)) {
                $sidebarContent
                    .removeClass("sidebar-sticky-top")
                    .removeClass("sidebar-sticky-bottom")
                    .addClass("sidebar-sticky-" + position);
            }
        };
        if (!isMobileDevice() &&
            $(window).width() >= 970 &&
            $sidebar.outerHeight() < $content.outerHeight() + 300) {
            if (windowScrollTop <= sidebarTop) {
                $sidebarContent.clear();
                return void 0;
            }
            if (windowScrollBottom >= sidebarBottom) {
                $sidebarContent.enableFrozen();
                return void 0;
            }
            if (!$sidebarContent.hasClass("sidebar-sticky") &&
                !$sidebarContent.hasClass("sidebar-follow")) {
                if (sidebarContentBottom < windowScrollBottom) {
                    $sidebarContent.enableSticky("bottom");
                }
                else if (sidebarContentTop > windowScrollTop) {
                    $sidebarContent.enableSticky("top");
                }
            }
            else if ($sidebarContent.hasClass("sidebar-sticky") &&
                !$sidebarContent.hasClass("sidebar-follow")) {
                if (window.scrolling.up &&
                    $sidebarContent.hasClass("sidebar-sticky-bottom")) {
                    $sidebarContent.enableFollow("bottom");
                }
                else if (window.scrolling.down &&
                    $sidebarContent.hasClass("sidebar-sticky-top")) {
                    $sidebarContent.enableFollow("top");
                }
            }
            else if ($sidebarContent.hasClass("sidebar-follow")) {
                if (window.scrolling.down &&
                    sidebarContentBottom < windowScrollBottom) {
                    $sidebarContent.enableSticky("bottom");
                }
                else if (window.scrolling.up && sidebarContentTop > windowScrollTop) {
                    $sidebarContent.enableSticky("top");
                }
            }
        }
        else {
            $sidebarContent.clear();
        }
    }
})(jQuery);
//# sourceMappingURL=common-ms05.js.map