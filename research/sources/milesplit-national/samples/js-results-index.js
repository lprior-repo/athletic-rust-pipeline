"use strict";
$(function () {
    var $toggle = $('#myResultsToggle');
    var $meetList = $('#meetList');
    var $pager = $('.results-pager').not('#minePager');
    var $empty = $('#myResultsEmpty');
    var mineUrl = '/results/mine.json' + window.location.search;
    var MINE_PER_PAGE = 50;
    var minePage = 1;
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
    function applyTextFilter() {
        var q = String($('#txtFilter').val() || '').toLowerCase();
        $meetList.find('.meet-month').each(function () {
            var $section = $(this);
            var visible = 0;
            $section.find('.meet-row').each(function () {
                var text = String($(this).attr('data-filter-text') || '');
                var show = !q || text.indexOf(q) !== -1;
                $(this).toggle(show);
                if (show) {
                    visible++;
                }
            });
            $section.toggle(visible > 0);
        });
    }
    $('#txtFilter').on('input', applyTextFilter);
    $('#frmFilter').on('change', 'select', function () {
        var form = document.getElementById('frmFilter');
        if (form) {
            form.submit();
        }
    });
    $('#frmFilter').on('keydown', '#txtFilter, #teamSearch', function (e) {
        if (e.which === 13 || e.keyCode === 13) {
            e.preventDefault();
        }
    });
    var teamNameById = {};
    var athleteNameById = {};
    function buildFollowNameMaps() {
        teamNameById = {};
        athleteNameById = {};
        if (!mine || !mine.follows) {
            return;
        }
        var i;
        var teams = mine.follows.teams || [];
        for (i = 0; i < teams.length; i++) {
            teamNameById[String(teams[i].id)] = String(teams[i].name || '');
        }
        var aths = mine.follows.athletes || [];
        for (i = 0; i < aths.length; i++) {
            athleteNameById[String(aths[i].id)] = String(aths[i].name || '');
        }
    }
    function followNamesFor(meetId) {
        if (!mine || !mine.meetFollows) {
            return '';
        }
        var entry = mine.meetFollows[String(meetId)];
        if (!entry) {
            return '';
        }
        var names = [];
        var i;
        var t = entry.t || [];
        for (i = 0; i < t.length; i++) {
            if (teamNameById[String(t[i])]) {
                names.push(teamNameById[String(t[i])]);
            }
        }
        var a = entry.a || [];
        for (i = 0; i < a.length; i++) {
            if (athleteNameById[String(a[i])]) {
                names.push(athleteNameById[String(a[i])]);
            }
        }
        return names.join(', ');
    }
    function followedBadgeHtml(meetId) {
        var label = followNamesFor(meetId);
        var attrs = label ? ' data-follows="' + esc(label) + '"' : '';
        return '<span class="is-followed-badge"' + attrs + '>Following</span>';
    }
    var $followTip = null;
    function showFollowTip(badge) {
        var label = $(badge).attr('data-follows');
        if (!label) {
            return;
        }
        if (!$followTip) {
            $followTip = $('<div class="follow-tooltip" role="tooltip" hidden></div>').appendTo(document.body);
        }
        var rect = badge.getBoundingClientRect();
        var top = rect.bottom + (window.pageYOffset || 0) + 6;
        var left = rect.left + (window.pageXOffset || 0);
        var maxLeft = $(window).width() - 320;
        $followTip.text(label)
            .css({ top: top + 'px', left: Math.max(8, Math.min(left, maxLeft)) + 'px' })
            .prop('hidden', false);
    }
    function hideFollowTip() {
        if ($followTip) {
            $followTip.prop('hidden', true);
        }
    }
    $meetList.on('mouseenter', '.is-followed-badge[data-follows]', function () { showFollowTip(this); });
    $meetList.on('mouseleave', '.is-followed-badge[data-follows]', hideFollowTip);
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
            var meetId = String($row.attr('data-meet-id'));
            if (ids[meetId] && !$row.hasClass('is-followed')) {
                $row.addClass('is-followed');
                $row.find('.meet-row__name').first().append(' ' + followedBadgeHtml(meetId));
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
            var dates = esc(shortDate(mt.dateStart));
            if (mt.dateEnd && mt.dateEnd > mt.dateStart) {
                dates += '<span class="meet-row__day-end">&ndash;' + esc(shortDate(mt.dateEnd)) + '</span>';
            }
            var venue = '';
            if (mt.venueCity || mt.venueState) {
                venue = '<span class="meet-row__venue">' + esc(mt.venueCity)
                    + (mt.venueCity && mt.venueState ? ', ' : '') + esc(mt.venueState) + '</span>';
            }
            var filterText = esc(String((mt.name || '') + ' ' + (mt.venueCity || '') + ' ' + (mt.venueState || '')).toLowerCase());
            html += '<li class="meet-row is-followed" data-meet-id="' + esc(mt.id) + '" data-filter-text="' + filterText + '">'
                + '<span class="meet-row__date"><span class="meet-row__day">' + dates + '</span></span>'
                + '<span class="meet-row__body">'
                + '<a class="meet-row__name" href="' + esc(mt.url) + '">' + esc(mt.name) + ' ' + followedBadgeHtml(mt.id) + '</a>'
                + venue + '</span>'
                + '<span class="meet-row__links"><a class="meet-row__results" href="' + esc(mt.url) + '" aria-label="Results for ' + esc(mt.name) + '">Results</a></span>'
                + '</li>';
        }
        html += '</ul></section>';
        return html;
    }
    function visibleMineMeets() {
        if (!mine || !mine.meets) {
            return [];
        }
        var includeAway = $('#myResultsAwayToggle').is(':checked');
        var out = [];
        for (var i = 0; i < mine.meets.length; i++) {
            if (includeAway || mine.meets[i].inRegion !== false) {
                out.push(mine.meets[i]);
            }
        }
        return out;
    }
    function syncAwayToggle() {
        var away = (mine && mine.awayCount) ? mine.awayCount : 0;
        if (away > 0) {
            $('#myResultsAwayCount').text('(' + away + ')');
            $('#myResultsAway').removeClass('hidden');
        }
        else {
            $('#myResultsAway').addClass('hidden');
        }
    }
    function setToggle(on) {
        toggledOn = on;
        minePage = 1;
        $toggle.attr('aria-pressed', on ? 'true' : 'false');
        if (on) {
            if (!mine || !mine.hasFollows) {
                $meetList.hide();
                $pager.hide();
                $('#minePager').addClass('hidden');
                $('#myResultsNote').addClass('hidden');
                $('#myResultsAway').addClass('hidden');
                $empty.removeClass('hidden').attr('aria-hidden', 'false');
                return;
            }
            $empty.addClass('hidden').attr('aria-hidden', 'true');
            syncAwayToggle();
            var visible = visibleMineMeets();
            $pager.hide();
            if (!visible.length) {
                var msg = 'No results from teams &amp; athletes you follow in this window.';
                if (mine.awayCount && !$('#myResultsAwayToggle').is(':checked')) {
                    msg += ' Tick “Include out-of-state meets” to widen it.';
                }
                $meetList.html('<div class="results-empty"><p>' + msg + '</p></div>').show();
                $('#minePager').addClass('hidden');
                $('#myResultsNote').addClass('hidden');
                return;
            }
            renderMinePage(visible);
        }
        else {
            $empty.addClass('hidden').attr('aria-hidden', 'true');
            $('#myResultsAway').addClass('hidden');
            $('#myResultsNote').addClass('hidden');
            $('#minePager').addClass('hidden');
            $meetList.html(pristineListHtml).show();
            $pager.show();
            decorate();
            applyTextFilter();
        }
    }
    function renderMinePage(visible) {
        var totalPages = Math.max(1, Math.ceil(visible.length / MINE_PER_PAGE));
        if (minePage > totalPages) {
            minePage = totalPages;
        }
        if (minePage < 1) {
            minePage = 1;
        }
        var start = (minePage - 1) * MINE_PER_PAGE;
        $meetList.html(renderPersonalized(visible.slice(start, start + MINE_PER_PAGE))).show();
        if (totalPages > 1) {
            $('#minePagerPrev').toggleClass('hidden', minePage <= 1);
            $('#minePagerNext').toggleClass('hidden', minePage >= totalPages);
            $('#minePager').removeClass('hidden');
        }
        else {
            $('#minePager').addClass('hidden');
        }
        var range = (mine && mine.wide) ? '' : ' for this period';
        var note = totalPages > 1
            ? 'Your followed meets' + range + ' — page ' + minePage + ' of ' + totalPages + '.'
            : 'Showing all your followed meets' + range + '.';
        if (mine && mine.truncated) {
            note += ' Showing your ' + (mine.meets ? mine.meets.length : '') + ' most recent — pick a month or year to reach older ones.';
        }
        $('#myResultsNote').text(note).removeClass('hidden');
        applyTextFilter();
    }
    function mineGoTo(delta) {
        if (!toggledOn || !mine) {
            return;
        }
        minePage += delta;
        renderMinePage(visibleMineMeets());
        var listTop = $meetList.offset();
        if (listTop) {
            window.scrollTo(0, Math.max(0, listTop.top - 70));
        }
    }
    $('#minePagerPrev').on('click', function () { mineGoTo(-1); });
    $('#minePagerNext').on('click', function () { mineGoTo(1); });
    var mineXhr = null;
    var MINE_CACHE_KEY = 'msResultsMine:' + mineUrl;
    var MINE_CACHE_TTL = 120000;
    function readMineCache() {
        try {
            var raw = sessionStorage.getItem(MINE_CACHE_KEY);
            if (!raw) {
                return null;
            }
            var wrapped = JSON.parse(raw);
            if (!wrapped || !wrapped.at || (Date.now() - wrapped.at) > MINE_CACHE_TTL) {
                return null;
            }
            return wrapped.data || null;
        }
        catch (e) {
            return null;
        }
    }
    function writeMineCache(data) {
        try {
            sessionStorage.setItem(MINE_CACHE_KEY, JSON.stringify({ at: Date.now(), data: data }));
        }
        catch (e) { }
    }
    function fetchMine(cb) {
        if (mine) {
            if (cb) {
                cb();
            }
            return;
        }
        if (mineXhr) {
            if (cb) {
                mineXhr.always(function () { cb(); });
            }
            return;
        }
        mineXhr = $.ajax({ url: mineUrl, dataType: 'json' })
            .done(function (data) {
            mine = (data && typeof data === 'object') ? data : null;
            if (mine) {
                writeMineCache(mine);
            }
            buildFollowNameMaps();
            if (mine && mine.loggedIn) {
                decorate();
            }
        })
            .always(function () {
            mineXhr = null;
            if (cb) {
                cb();
            }
        });
    }
    $toggle.on('click', function () {
        if (mine) {
            setToggle(!toggledOn);
            return;
        }
        if ($toggle.attr('aria-busy') === 'true') {
            return;
        }
        $toggle.attr('aria-busy', 'true');
        $toggle.find('.fa').removeClass('fa-star').addClass('fa-spinner fa-spin');
        fetchMine(function () {
            $toggle.attr('aria-busy', 'false');
            $toggle.find('.fa').removeClass('fa-spinner fa-spin').addClass('fa-star');
            setToggle(!toggledOn);
        });
    });
    $('#myResultsAwayToggle').on('change', function () {
        if (toggledOn) {
            setToggle(true);
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
        if (follows.athletes && follows.athletes.length) {
            html += '<div class="follow-group"><strong>Athletes You Follow</strong>';
            for (var j = 0; j < follows.athletes.length; j++) {
                var a = follows.athletes[j];
                html += '<div class="follow-row"><span><a href="' + esc(a.url) + '">' + esc(a.name) + '</a></span>'
                    + '<button type="button" class="follow-row__btn is-following" data-unfollow-athlete="' + esc(a.id) + '">Unfollow</button></div>';
            }
            html += '</div>';
        }
        if (!html) {
            html = '<p class="follow-empty">You aren’t following anyone yet. Search above to add teams or athletes.</p>';
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
    function collectSayt(resp, type, out) {
        var data = (resp && resp.length && resp[0]) ? resp[0] : resp;
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
                meta: fields.description || (type === 'team' ? 'Team' : 'Athlete')
            });
        }
    }
    function runSearch(q) {
        var $results = $('#followPickerResults');
        $results.html('<p class="follow-searching">Searching…</p>');
        function safe(url, data) {
            return $.ajax({ url: url, data: data, dataType: 'json' })
                .then(function (d) { return d; }, function () { return null; });
        }
        $.when(safe('/sayt/teams', { q: q, location: 1 }), safe('/sayt/athletes', { q: q }))
            .done(function (teamsResp, athletesResp) {
            var items = [];
            collectSayt(teamsResp, 'team', items);
            collectSayt(athletesResp, 'athlete', items);
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
        });
    }
    function refreshAfterMutation() {
        setTimeout(function () {
            mine = null;
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
        var type = String($row.attr('data-type'));
        var id = String($row.attr('data-id'));
        if (!id) {
            return;
        }
        if (type === 'team') {
            $.post('/teams/' + id + '/claim', { type: 4 });
        }
        else {
            $.get('/athletes/' + id + '/follow');
        }
        $btn.addClass('is-following').text('Following');
        refreshAfterMutation();
    });
    $('#followPickerCurrent').on('click', '[data-unfollow-athlete]', function () {
        var id = String($(this).attr('data-unfollow-athlete'));
        if (!id) {
            return;
        }
        $.get('/athletes/' + id + '/unfollow');
        $(this).closest('.follow-row').remove();
        refreshAfterMutation();
    });
    var $teamSearch = $('#teamSearch');
    if ($teamSearch.length) {
        var teamSearchTimer = null;
        $teamSearch.on('input', function () {
            var q = String($(this).val() || '').replace(/^\s+|\s+$/g, '');
            var $res = $('#teamSearchResults');
            if (teamSearchTimer) {
                clearTimeout(teamSearchTimer);
            }
            if (q.length < 3) {
                $res.empty();
                return;
            }
            teamSearchTimer = setTimeout(function () {
                $.ajax({ url: '/autocomplete/teams', data: { q: q }, dataType: 'json' })
                    .done(function (data) {
                    var list = (data && data.suggestions) || [];
                    var html = '';
                    for (var i = 0; i < list.length && i < 12; i++) {
                        var s = list[i];
                        if (!s || !s.data || !s.data.id) {
                            continue;
                        }
                        html += '<a class="results-team-search__item" href="/results?team=' + esc(s.data.id) + '">' + esc(s.value) + '</a>';
                    }
                    $res.html(html || '<div class="results-team-search__empty">No teams found.</div>');
                })
                    .fail(function () { $res.empty(); });
            }, 250);
        });
    }
    if ($toggle.length) {
        var cachedMine = readMineCache();
        if (cachedMine && cachedMine.loggedIn) {
            mine = cachedMine;
            buildFollowNameMaps();
            decorate();
            mine = null;
        }
        fetchMine();
    }
});
//# sourceMappingURL=index.js.map