"use strict";
$(function () {
    var $toggle = $('#myResultsToggle');
    var $meetList = $('#meetList');
    var $empty = $('#myResultsEmpty');
    var mineUrl = '/calendar/mine.json' + window.location.search;
    var pristineListHtml = $meetList.html();
    var mine = null;
    var toggledOn = false;
    var searchTimer = null;
    function esc(value) {
        return String(value == null ? '' : value)
            .replace(/&/g, '&amp;')
            .replace(/</g, '&lt;')
            .replace(/>/g, '&gt;')
            .replace(/"/g, '&quot;');
    }
    var MONTH_LONG = ['January', 'February', 'March', 'April', 'May', 'June', 'July', 'August', 'September', 'October', 'November', 'December'];
    var MONTH_SHORT = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
    function monthKey(dateStr) {
        return String(dateStr || '').substring(0, 7);
    }
    function monthHeading(dateStr) {
        var m = parseInt(String(dateStr).substring(5, 7), 10);
        var y = String(dateStr).substring(0, 4);
        return (MONTH_LONG[m - 1] || '') + ' ' + y;
    }
    function shortDate(dateStr) {
        var m = parseInt(String(dateStr).substring(5, 7), 10);
        var d = parseInt(String(dateStr).substring(8, 10), 10);
        return (MONTH_SHORT[m - 1] || '') + ' ' + d;
    }
    function applyFilters() {
        var q = String($('#txtFilter').val() || '').toLowerCase();
        var lvl = String($('#ddLevel').val() || '');
        $meetList.find('.meet-month').each(function () {
            var $section = $(this);
            var visible = 0;
            $section.find('.meet-row').each(function () {
                var $row = $(this);
                var text = String($row.attr('data-filter-text') || '');
                var levels = String($row.attr('data-level') || '');
                var matchText = !q || text.indexOf(q) !== -1;
                var matchLevel = !lvl || (',' + levels + ',').indexOf(',' + lvl + ',') !== -1;
                var show = matchText && matchLevel;
                $row.toggle(show);
                if (show) {
                    visible++;
                }
            });
            $section.toggle(visible > 0);
        });
    }
    $('#txtFilter').on('input', applyFilters);
    $('#ddLevel').on('change', applyFilters);
    $('#frmFilter').on('change', 'select', function () {
        if (this.id === 'ddLevel') {
            return;
        }
        var form = document.getElementById('frmFilter');
        if (form) {
            form.submit();
        }
    });
    $('#frmFilter').on('keydown', '#txtFilter', function (e) {
        if (e.which === 13 || e.keyCode === 13) {
            e.preventDefault();
        }
    });
    function decorate() {
        if (!mine || !mine.meetIds) {
            return;
        }
        var ids = {};
        for (var i = 0; i < mine.meetIds.length; i++) {
            ids[String(mine.meetIds[i])] = true;
        }
        $meetList.find('.meet-row').each(function () {
            var $row = $(this);
            if (ids[String($row.attr('data-meet-id'))] && !$row.hasClass('is-followed')) {
                $row.addClass('is-followed');
                $row.find('.meet-row__name').first().append(' <span class="is-followed-badge">Following</span>');
            }
        });
    }
    function renderPersonalized(meets) {
        if (!meets || !meets.length) {
            return '';
        }
        var html = '';
        var curKey = '';
        for (var i = 0; i < meets.length; i++) {
            var mt = meets[i];
            var key = monthKey(mt.dateStart);
            if (key !== curKey) {
                if (curKey !== '') {
                    html += '</ul></section>';
                }
                curKey = key;
                html += '<section class="meet-month" data-month="' + esc(key) + '">'
                    + '<h2 class="meet-month-heading">' + esc(monthHeading(mt.dateStart)) + '</h2>'
                    + '<ul class="meet-rows">';
            }
            var dates = '<span class="meet-row__day">' + esc(shortDate(mt.dateStart)) + '</span>';
            if (mt.dateEnd && mt.dateEnd > mt.dateStart) {
                dates += '<span class="meet-row__day-end">&ndash;' + esc(shortDate(mt.dateEnd)) + '</span>';
            }
            var venue = '';
            if (mt.venueCity || mt.venueState) {
                venue = '<span class="meet-row__venue">' + esc(mt.venueCity)
                    + (mt.venueCity && mt.venueState ? ', ' : '') + esc(mt.venueState) + '</span>';
            }
            var reg = mt.registrationUrl
                ? '<a class="meet-row__reg" href="' + esc(mt.registrationUrl) + '"><i class="fa fa-exclamation-circle" aria-hidden="true"></i> Registering Now!</a>'
                : '';
            var links = mt.hasResults
                ? '<a class="meet-row__results" href="' + esc(mt.url) + '/results" aria-label="Results for ' + esc(mt.name) + '"><i class="fa fa-file-text" aria-hidden="true"></i> Results</a>'
                : '';
            var filterText = esc(String((mt.name || '') + ' ' + (mt.venueCity || '') + ' ' + (mt.venueState || '')).toLowerCase());
            html += '<li class="meet-row is-followed" data-meet-id="' + esc(mt.id) + '" data-filter-text="' + filterText + '">'
                + '<span class="meet-row__date">' + dates + '</span>'
                + '<span class="meet-row__body">'
                + '<a class="meet-row__name" href="' + esc(mt.url) + '">' + esc(mt.name) + ' <span class="is-followed-badge">Following</span></a>'
                + venue + reg + '</span>'
                + '<span class="meet-row__links">' + links + '</span>'
                + '</li>';
        }
        html += '</ul></section>';
        return html;
    }
    function updateCount(n) {
        var $count = $('#myResultsCount');
        if (n >= 0) {
            $count.text(String(n)).prop('hidden', false);
        }
        else {
            $count.text('').prop('hidden', true);
        }
    }
    function setToggle(on) {
        toggledOn = on;
        $toggle.attr('aria-pressed', on ? 'true' : 'false');
        if (on) {
            if (!mine || !mine.hasFollows) {
                $meetList.hide();
                $empty.removeClass('hidden').attr('aria-hidden', 'false');
                updateCount(0);
                return;
            }
            $empty.addClass('hidden').attr('aria-hidden', 'true');
            var visible = (mine && mine.meets) ? mine.meets : [];
            if (!visible.length) {
                $meetList.html('<div class="results-empty"><p>No upcoming meets from teams you follow on this calendar.</p></div>').show();
                updateCount(0);
                return;
            }
            $meetList.html(renderPersonalized(visible)).show();
            updateCount(visible.length);
            applyFilters();
        }
        else {
            $empty.addClass('hidden').attr('aria-hidden', 'true');
            $meetList.html(pristineListHtml).show();
            decorate();
            updateCount(-1);
            applyFilters();
        }
    }
    function fetchMine(cb) {
        $.ajax({ url: mineUrl, dataType: 'json' })
            .done(function (data) {
            mine = (data && typeof data === 'object') ? data : null;
            if (mine && mine.loggedIn) {
                decorate();
            }
            if (cb) {
                cb();
            }
        })
            .fail(function () { if (cb) {
            cb();
        } });
    }
    $toggle.on('click', function () {
        if (!mine) {
            fetchMine(function () { setToggle(!toggledOn); });
        }
        else {
            setToggle(!toggledOn);
        }
    });
    var $picker = $('#followPicker');
    function renderCurrentFollows() {
        var $current = $('#followPickerCurrent');
        if (!mine || !mine.follows) {
            $current.empty();
            return;
        }
        var follows = mine.follows;
        var html = '';
        if (follows.teams && follows.teams.length) {
            html += '<div class="follow-group"><strong>Teams You Follow</strong>';
            for (var i = 0; i < follows.teams.length; i++) {
                var t = follows.teams[i];
                html += '<div class="follow-row"><span><a href="' + esc(t.url) + '">' + esc(t.name) + '</a></span></div>';
            }
            html += '</div>';
        }
        if (!html) {
            html = '<p class="follow-empty">You aren’t following any teams yet. Search above to add one.</p>';
        }
        $current.html(html);
    }
    function openPicker() {
        $picker.removeClass('hidden').attr('aria-hidden', 'false');
        $('#followPickerResults').empty();
        $('#followPickerSearch').val('');
        renderCurrentFollows();
        $('#followPickerSearch').trigger('focus');
    }
    function closePicker() {
        $picker.addClass('hidden').attr('aria-hidden', 'true');
    }
    $('#manageFollows, #myResultsEmptyManage').on('click', function () {
        if (!mine) {
            fetchMine(openPicker);
        }
        else {
            openPicker();
        }
    });
    $('#followPickerClose').on('click', closePicker);
    $('#followPickerSearch').on('input', function () {
        var q = String($(this).val() || '').replace(/^\s+|\s+$/g, '');
        if (searchTimer) {
            clearTimeout(searchTimer);
        }
        if (q.length < 2) {
            $('#followPickerResults').empty();
            return;
        }
        searchTimer = setTimeout(function () { runSearch(q); }, 250);
    });
    function collectSayt(data, type, out) {
        if (!data || !data.suggestions) {
            return;
        }
        for (var i = 0; i < data.suggestions.length && out.length < 40; i++) {
            var s = data.suggestions[i];
            if (!s || !s.data || !s.data.id) {
                continue;
            }
            var fields = s.data.fields || {};
            out.push({
                type: type,
                id: s.data.id,
                name: fields.title || s.value || '',
                meta: fields.description || 'Team'
            });
        }
    }
    function runSearch(q) {
        var $results = $('#followPickerResults');
        $results.html('<p class="follow-searching">Searching…</p>');
        $.ajax({ url: '/sayt/teams', data: { q: q, location: 1 }, dataType: 'json' })
            .done(function (data) {
            var items = [];
            collectSayt(data, 'team', items);
            if (!items.length) {
                $results.html('<p class="follow-searching">No matches.</p>');
                return;
            }
            var html = '';
            for (var i = 0; i < items.length; i++) {
                var it = items[i];
                html += '<div class="follow-row" data-type="' + esc(it.type) + '" data-id="' + esc(it.id) + '">'
                    + '<span>' + esc(it.name) + ' <span class="follow-row__meta">' + esc(it.meta) + '</span></span>'
                    + '<button type="button" class="follow-row__btn" data-follow="1">Follow</button></div>';
            }
            $results.html(html);
        })
            .fail(function () { $results.html('<p class="follow-searching">No matches.</p>'); });
    }
    function refreshAfterMutation() {
        setTimeout(function () {
            fetchMine(function () {
                renderCurrentFollows();
                if (toggledOn) {
                    setToggle(true);
                }
            });
        }, 600);
    }
    $('#followPickerResults').on('click', '.follow-row__btn', function () {
        var $btn = $(this);
        if ($btn.hasClass('is-following')) {
            return;
        }
        var $row = $btn.closest('.follow-row');
        var id = String($row.attr('data-id'));
        if (!id) {
            return;
        }
        $.post('/teams/' + id + '/claim', { type: 4 });
        $btn.addClass('is-following').text('Following');
        refreshAfterMutation();
    });
    if ($toggle.length) {
        fetchMine();
    }
});
//# sourceMappingURL=index.js.map