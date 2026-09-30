$(function() {

    var tld = window.location.host.split('.').pop(),
        subdomain = window.location.host.split('.').shift(),
        $form = $('#rankingsFilters');

    var whatChanged = $(this).attr('name'),
        country     = $('#ddCountry').val() || subdomain,
        state       = $('#ddState').val() || subdomain,
        level       = $('#ddLevel').val(),
        event       = $('#ddEvent').val(),
        season      = $('#ddSeason').val(),
        accuracy    = $('#ddAccuracy').val(),
        year        = $('#ddYear').val(),
        grade       = $('#ddGrade').val(),
        ageGroup    = $('#ddAgeGroup').val(),
        league      = $('#ddLeague').val(),
        conversion  = $('input[name="conversion"]').val(),
        venue       = $('input[name="venue"]').val(),
        team        = $('input[name="team"]').val(),
        meet        = $('input[name="meet"]').val();

    var navigate = function() {
        var url = 'https://',
            queryString = [];
        url += (state === 'usa') ? 'www' : state;
        url += '.milesplit.' + tld + '/rankings/';
        // subjective rankings
        if (season === 'flo50') {
            url += 'flo50?';
            if (year) {
                url += 'year=' + year;
            }
        }
        else {
            url += (event === 'leaders') ? 'leaders' : 'events';
            url += '/' + level + '/' + season;
            url += (event === 'leaders') ? '' : '/' + event;
            if (-1 !== level.indexOf('alumni')) {
                queryString.push('alumni=1');
            }
            if (year) {
                queryString.push('year=' + year);
            }
            if (accuracy) {
                queryString.push('accuracy=' + accuracy);
            }
            if (grade) {
                queryString.push('grade=' + grade);
            }
            if (ageGroup) {
                queryString.push('ageGroup=' + ageGroup);
            }
            if (conversion) {
                queryString.push('conversion=' + conversion);
            }
            if (league && whatChanged !== 'state') {
                queryString.push('league=' + league);
            }
            if (venue) {
                queryString.push('venue=' + venue);
            }
            if (meet) {
                queryString.push('meet=' + meet);
            }
            if (team) {
                queryString.push('team=' + team);
            }
            if (queryString.length > 0) {
                url += '?' + queryString.join('&');
            }
        }
        document.location = url;
    };

    var submitForm = function(e) {
        e.preventDefault();
        whatChanged = $(this).attr('name');
        state       = $('#ddState').val() || subdomain;
        level       = $('#ddLevel').val();
        event       = $('#ddEvent').val();
        season      = $('#ddSeason').val();
        accuracy    = $('#ddAccuracy').val();
        year        = $('#ddYear').val();
        grade       = $('#ddGrade').val();
        ageGroup    = level.substr(0,4) === 'club' ? $('#ddAgeGroup').val() : '';
        league      = $('#ddLeague').val();
        conversion  = $('input[name="conversion"]').val();
        venue       = $('input[name="venue"]').val();
        team        = $('input[name="team"]').val();
        meet        = $('input[name="meet"]').val();
        navigate();
    };

    var toggleLinkFilter = function(e) {
        e.preventDefault();
        var $link = $(this),
            $field = $form.find('input[name="' + $link.attr('data-field') + '"]');
        $field.val($link.attr('data-value'));
        submitForm(e);
    };

    $form
        .on('change', 'select', submitForm)
        .on('click', 'a.filter', toggleLinkFilter);

    // LEAD-439 — segmented accuracy control: click a pill → write to hidden input + submit
    $form.on('click', '.rankings-segmented__btn', function(e) {
        e.preventDefault();
        var $btn = $(this);
        var value = $btn.attr('data-value');
        var $group = $btn.closest('.rankings-segmented');
        var $input = $group.find('input[name="accuracy"]');
        $group.find('.rankings-segmented__btn')
            .removeClass('is-active')
            .attr('aria-pressed', 'false');
        $btn.addClass('is-active').attr('aria-pressed', 'true');
        $input.val(value);
        whatChanged = 'accuracy';
        accuracy = value;
        navigate();
    });

    // Leaders page: clicking an event-name cell link navigates to that event's rankings
    // via navigate() so current filters are preserved.
    $('.rankings-table').on('click', '.col-event a', function(e) {
        e.preventDefault();
        whatChanged = 'event';
        event = $(this).attr('data-event');
        navigate();
    });

    // LEAD-439 — hide right-edge scroll gradient when there's no overflow
    // or when the user has scrolled all the way to the right.
    var $tableOuter = $('.rankings-table-outer');
    if ($tableOuter.length) {
        var $tableWrap = $tableOuter.find('.rankings-table-wrap');
        var updateScrollState = function() {
            var el = $tableWrap[0];
            if (!el) return;
            var hasOverflow = el.scrollWidth > el.clientWidth + 1;
            $tableOuter.toggleClass('is-no-overflow', !hasOverflow);
            var atEnd = el.scrollLeft + el.clientWidth >= el.scrollWidth - 1;
            $tableOuter.toggleClass('is-scroll-end', hasOverflow && atEnd);
        };
        $tableWrap.on('scroll', updateScrollState);
        $(window).on('resize', updateScrollState);
        updateScrollState();

        // Pro funnel: keep the CTA with the reader while they scroll the locked
        // rows, then release it at the end of the table. The card stays anchored
        // to the first masked row until it would scroll past STICKY_PIN, pins
        // there while the masked region passes behind it, and never drops below
        // the last masked row — so it tracks the locked content instead of
        // floating freely over the rest of the page (no fixed-banner nagging).
        var $cta = $tableOuter.find('.pro-cta--ab');
        var $masked = $tableOuter.find('.rankings-row.lk');
        if ($cta.length && $masked.length) {
            var $firstMasked = $masked.first();
            var $lastMasked = $masked.last();
            var STICKY_PIN = 70;  // viewport offset the card pins to while scrolling (clears the site header)
            var positionCta = function() {
                var outerTop = $tableOuter[0].getBoundingClientRect().top;
                var regionTop = $firstMasked[0].getBoundingClientRect().top;
                var regionBottom = $lastMasked[0].getBoundingClientRect().bottom;
                var ctaH = $cta[0].offsetHeight;
                var desired;
                if (regionBottom - regionTop <= ctaH) {
                    // Locked region shorter than the card — nothing to scroll
                    // through, so just sit at the top of the masked rows.
                    desired = regionTop;
                } else {
                    // Pin at STICKY_PIN, clamped to the locked region's bounds.
                    desired = Math.max(regionTop, STICKY_PIN);
                    desired = Math.min(desired, regionBottom - ctaH);
                }
                $cta.css('top', Math.round(desired - outerTop) + 'px');
            };
            var ticking = false;
            var onScroll = function() {
                if (ticking) return;
                ticking = true;
                window.requestAnimationFrame(function() { positionCta(); ticking = false; });
            };
            positionCta();
            $(window).on('scroll', onScroll);
            $(window).on('resize', positionCta);
            $(window).on('load', positionCta);  // re-run after late layout (web fonts / images)
        }
    }
});