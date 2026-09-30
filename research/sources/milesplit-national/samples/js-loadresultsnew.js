var MeetResults = new _DF_.MeetResults(meetResultParams);
$(document).ready(function () {
    var $results = $('#resultsList');
    var params = {
        isMeetPro: isMeetPro,
        resultsId: MeetResults.resultsId,
        fields: 'id,meetId,meetName,teamId,videoId,teamName,athleteId,firstName,lastName,gender,genderName,levelId,levelName,divisionId,divisionName,meetResultsId,meetResultsDivisionId,resultsDivisionId,ageGroupId,ageGroupName,gradYear,eventName,eventCode,eventDistance,eventGenreOrder,round,roundName,heat,units,mark,place,windReading,profileUrl,teamProfileUrl,performanceVideoId,teamLogo,statusCode'
    };
    if (!MeetResults.isTrack) {
        params.teamScores = MeetResults.teamScores;
    }
    var allMode = false;
    var myShowFull = false;
    var myStarsOff = false;
    var followedAthletes = {};
    var followedTeams = {};
    ((meetResultParams.followedAthleteIds || [])).forEach(function (id) { followedAthletes[String(id)] = true; });
    ((meetResultParams.followedTeamIds || [])).forEach(function (id) { followedTeams[String(id)] = true; });
    function isMine(result) {
        return !!(followedAthletes[String(result.athleteId)] || followedTeams[String(result.teamId)]);
    }
    function myOnly() {
        return $('#myResultsOnly').is(':checked');
    }
    function titleCase(s) {
        s = String(s == null ? '' : s);
        return s ? s.charAt(0).toUpperCase() + s.slice(1) : '';
    }
    function escapeHtml(s) {
        return String(s == null ? '' : s)
            .replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
    }
    var meetAthletes = {};
    var meetTeams = {};
    var followListsBuilt = false;
    function buildFollowLists() {
        if (followListsBuilt) {
            return;
        }
        MeetResults.events().forEach(function (e) {
            e.results().forEach(function (r) {
                var aid = parseInt(r.athleteId, 10);
                if (aid > 1 && aid !== 1500 && aid !== 1501 && r.lastName && !meetAthletes[aid]) {
                    meetAthletes[aid] = { id: aid, name: (titleCase(r.firstName) + ' ' + titleCase(r.lastName)).trim(), team: r.teamName || '' };
                }
                var tid = parseInt(r.teamId, 10);
                if (tid > 0 && r.teamName && !meetTeams[tid]) {
                    meetTeams[tid] = { id: tid, name: r.teamName };
                }
            });
        });
        followListsBuilt = true;
    }
    function countFollowedHere() {
        var seen = {};
        var count = 0;
        MeetResults.events().forEach(function (e) {
            e.results().forEach(function (r) {
                if (r.athleteId && followedAthletes[String(r.athleteId)] && !seen['a' + r.athleteId]) {
                    seen['a' + r.athleteId] = true;
                    count++;
                }
                if (r.teamId && followedTeams[String(r.teamId)] && !seen['t' + r.teamId]) {
                    seen['t' + r.teamId] = true;
                    count++;
                }
            });
        });
        return count;
    }
    function refreshFollowUi() {
        var count = countFollowedHere();
        if (count > 0) {
            $('.myFollowToggle__count').text(count);
            $('.myResultsField, .myFollowAddField').removeClass('hidden');
            $('.myFollowCtaField').addClass('hidden');
        }
        else {
            $('.myResultsField, .myFollowAddField').addClass('hidden');
            $('.myFollowCtaField').removeClass('hidden');
        }
    }
    function setupFollowToggle() { refreshFollowUi(); }
    function renderPicker(query) {
        query = String(query || '').toLowerCase();
        var items = [];
        Object.keys(meetTeams).forEach(function (id) {
            var t = meetTeams[id];
            if (!query || t.name.toLowerCase().indexOf(query) !== -1) {
                items.push({ type: 'team', id: t.id, name: t.name, meta: 'Team', following: !!followedTeams[String(t.id)] });
            }
        });
        Object.keys(meetAthletes).forEach(function (id) {
            var a = meetAthletes[id];
            if (!query || a.name.toLowerCase().indexOf(query) !== -1 || (a.team && a.team.toLowerCase().indexOf(query) !== -1)) {
                items.push({ type: 'athlete', id: a.id, name: a.name, meta: a.team, following: !!followedAthletes[String(a.id)] });
            }
        });
        items.sort(function (x, y) {
            if (x.type !== y.type) {
                return x.type === 'team' ? -1 : 1;
            }
            return x.name.localeCompare(y.name);
        });
        var shown = items.slice(0, 80);
        var html = shown.map(function (it) {
            return '<div class="myFollowRow" data-type="' + it.type + '" data-id="' + it.id + '">'
                + '<span class="myFollowRow__info"><span class="myFollowRow__name">' + escapeHtml(it.name) + '</span>'
                + (it.meta ? '<span class="myFollowRow__meta">' + escapeHtml(it.meta) + '</span>' : '') + '</span>'
                + '<button type="button" class="myFollowRow__btn' + (it.following ? ' is-following' : '') + '">'
                + (it.following ? '<i class="fa fa-check"></i> Following' : 'Follow') + '</button>'
                + '</div>';
        }).join('');
        if (!shown.length) {
            html = '<p class="myFollowPicker__empty">No athletes or teams match "' + escapeHtml(query) + '".</p>';
        }
        else if (items.length > shown.length) {
            html += '<p class="myFollowPicker__more">Showing ' + shown.length + ' of ' + items.length + '. Keep typing to narrow it down.</p>';
        }
        $('#myFollowPickerList').html(html);
    }
    function doFollow(type, id, $btn) {
        if (type === 'team') {
            followedTeams[String(id)] = true;
            $.post('/teams/' + id + '/claim', { type: 4 });
        }
        else {
            followedAthletes[String(id)] = true;
            $.get('/athletes/' + id + '/follow');
        }
        $btn.addClass('is-following').html('<i class="fa fa-check"></i> Following');
        refreshFollowUi();
        render();
    }
    var files = (typeof meetResultFiles !== 'undefined' && meetResultFiles && meetResultFiles.length)
        ? meetResultFiles
        : [{ id: MeetResults.resultsId, name: '', isMeetPro: isMeetPro }];
    var multipleFiles = files.length > 1;
    var fileById = {};
    files.forEach(function (file) { fileById[file.id] = file; });
    var allNonMeetPro = true;
    files.forEach(function (file) { if (file.isMeetPro) {
        allNonMeetPro = false;
    } });
    var hasNonMeetPro = false;
    files.forEach(function (file) { if (!file.isMeetPro) {
        hasNonMeetPro = true;
    } });
    var requests;
    if (!multipleFiles) {
        requests = files.map(function (file) { return { resultsId: file.id, isMeetPro: file.isMeetPro }; });
    }
    else if (allNonMeetPro) {
        requests = [{ resultsId: '', isMeetPro: 0 }];
    }
    else {
        requests = [];
        if (hasNonMeetPro) {
            requests.push({ resultsId: '', isMeetPro: 0, excludeMeetPro: true });
        }
        files.forEach(function (file) {
            if (file.isMeetPro) {
                requests.push({ resultsId: file.id, isMeetPro: 1 });
            }
        });
    }
    var pending = requests.length;
    var allRows = [];
    requests.forEach(function (req) {
        var reqParams = { isMeetPro: req.isMeetPro || 0, fields: params.fields };
        if (req.resultsId) {
            reqParams.resultsId = req.resultsId;
        }
        if (!MeetResults.isTrack) {
            reqParams.teamScores = MeetResults.teamScores;
        }
        _DF_.API.get('v1/meets/' + MeetResults.meetId + '/performances', reqParams)
            .done(function (json) {
            (json.data || []).forEach(function (row) {
                if (multipleFiles) {
                    var fileId = req.resultsId || row.meetResultsId;
                    if (req.excludeMeetPro && fileId && fileById[fileId] && fileById[fileId].isMeetPro) {
                        return;
                    }
                    if (fileId) {
                        var file = fileById[fileId];
                        if (!row.divisionName) {
                            row.divisionId = fileId;
                            if (file && file.name) {
                                row.divisionName = divisionFromFile(file.name);
                            }
                        }
                    }
                }
                allRows.push(row);
            });
        })
            .always(function () {
            pending--;
            if (pending === 0) {
                finishLoad();
            }
        });
    });
    function finishLoad() {
        MeetResults.loadResults(allRows);
        $results.removeClass('loading');
        var events = MeetResults.events();
        if (events && events.length) {
            indexRounds();
            setGenderTabs();
            initFromUrl();
            render();
            setupFollowToggle();
        }
        else {
            showEmptyState();
        }
    }
    var roundsByBase = {};
    function baseEventKey(event) {
        return removeSpecialCharacters(String((event.gender || '') + (event.code || '') + (event.division || '') + (event.ageGroup || ''))).toLowerCase();
    }
    function indexRounds() {
        MeetResults.events().forEach(function (event) {
            var base = baseEventKey(event);
            if (!roundsByBase[base]) {
                roundsByBase[base] = {};
            }
            if (event.round) {
                roundsByBase[base][event.round] = true;
            }
        });
    }
    function showRoundFor(event) {
        if (!event.round) {
            return false;
        }
        var rounds = roundsByBase[baseEventKey(event)];
        return !!rounds && Object.keys(rounds).length > 1;
    }
    function eventDisplayName(event) {
        return event.name + (showRoundFor(event) ? ' ' + event.round : '');
    }
    function eventDisplayFullName(event) {
        if (!event.round || showRoundFor(event)) {
            return event.fullEventName;
        }
        var esc = String(event.round).replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
        return String(event.fullEventName).replace(new RegExp('\\s+' + esc + '\\s*$'), '');
    }
    var setGenderTabs = function () {
        var $tabs = $('#genderTabs');
        var genders = [];
        MeetResults.events().forEach(function (event) {
            if (event.gender && genders.indexOf(event.gender) === -1) {
                genders.push(event.gender);
            }
        });
        if (genders.length > 1) {
            genders.sort();
            $tabs.append("<button type=\"button\" class=\"genderTab active\" data-gender=\"\">All</button>");
            genders.forEach(function (gender) {
                $tabs.append("<button type=\"button\" class=\"genderTab\" data-gender=\"".concat(gender, "\">").concat(gender, "</button>"));
            });
            syncTabAria();
            $tabs.closest('.field').removeClass('hidden');
        }
    };
    function eventTypeKey(event) {
        return removeSpecialCharacters(String(event.code || event.name || '')).toLowerCase();
    }
    function eventTypeLabel(event) {
        return event.name;
    }
    function activeGender() {
        return String($('#genderTabs .genderTab.active').data('gender') || '');
    }
    function rebuildSelect(selector, allLabel, options) {
        var $sel = $(selector);
        $sel.removeAttr('disabled');
        var previous = String($sel.val() || '');
        $sel.empty().append('<option value="">' + allLabel + '</option>');
        options.forEach(function (opt) {
            $sel.append('<option value="' + String(opt.value).replace(/"/g, '&quot;') + '">' + opt.label + '</option>');
        });
        var stillValid = false;
        $sel.find('option').each(function () { if (this.value === previous) {
            stillValid = true;
        } });
        $sel.val(stillValid ? previous : '');
        $sel.parents('.field').toggleClass('hidden', options.length <= 1);
    }
    function rebuildTeamList(options) {
        var $input = $('#teamList');
        var $datalist = $('#teamListOptions');
        var previous = String($input.val() || '');
        var valid = {};
        $datalist.empty();
        options.forEach(function (opt) {
            valid[String(opt.value)] = true;
            $datalist.append('<option value="' + String(opt.value).replace(/"/g, '&quot;') + '"></option>');
        });
        if (previous && !valid[previous]) {
            $input.val('');
        }
        $input.parents('.field').toggleClass('hidden', options.length <= 1);
    }
    function distinctDivisions() {
        var gender = activeGender();
        var level = String($('#levelList').val() || '');
        var out = [];
        MeetResults.events().forEach(function (e) {
            if ((!gender || e.gender === gender) && eventHasLevel(e, level) && e.division && out.indexOf(e.division) === -1) {
                out.push(e.division);
            }
        });
        out.sort(naturalCompare);
        return out.map(function (d) { return { value: d, label: d }; });
    }
    function distinctLevels() {
        var gender = activeGender();
        var seen = {};
        var out = [];
        MeetResults.events().forEach(function (e) {
            if (gender && e.gender !== gender) {
                return;
            }
            e.results().forEach(function (result) {
                var id = String(result.levelId || '');
                if (id && result.levelName && !seen[id]) {
                    seen[id] = true;
                    out.push({ value: id, label: result.levelName });
                }
            });
        });
        out.sort(function (a, b) { return naturalCompare(a.label, b.label); });
        return out;
    }
    function distinctAgeGroups() {
        var gender = activeGender();
        var level = String($('#levelList').val() || '');
        var division = String($('#divisionList').val() || '');
        var out = [];
        MeetResults.events().forEach(function (e) {
            if ((!gender || e.gender === gender)
                && eventHasLevel(e, level)
                && (!division || e.division === division)
                && e.ageGroup
                && out.indexOf(e.ageGroup) === -1) {
                out.push(e.ageGroup);
            }
        });
        out.sort(naturalCompare);
        return out.map(function (ageGroup) { return { value: ageGroup, label: ageGroup }; });
    }
    function distinctEventTypes() {
        var gender = activeGender();
        var level = String($('#levelList').val() || '');
        var division = String($('#divisionList').val() || '');
        var ageGroup = String($('#ageGroupList').val() || '');
        var seen = {};
        var out = [];
        sortEvents(MeetResults.events()).forEach(function (e) {
            if ((!gender || e.gender === gender)
                && eventHasLevel(e, level)
                && (!division || e.division === division)
                && (!ageGroup || (e.ageGroup || '') === ageGroup)) {
                var key = eventTypeKey(e);
                if (!seen[key]) {
                    seen[key] = true;
                    out.push({ value: key, label: eventTypeLabel(e) });
                }
            }
        });
        return out;
    }
    function distinctTeams() {
        var scope = currentScope();
        var seen = {};
        var out = [];
        MeetResults.events().forEach(function (e) {
            if (eventMatchesScope(e, scope)) {
                e.results().forEach(function (result) {
                    var name = result.teamName;
                    if (resultMatchesLevel(result, scope.level) && name && !seen[name]) {
                        seen[name] = true;
                        out.push(name);
                    }
                });
            }
        });
        out.sort(naturalCompare);
        return out.map(function (n) { return { value: n, label: n }; });
    }
    function rebuildDependentFilters() {
        rebuildSelect('#levelList', 'All Levels', distinctLevels());
        rebuildSelect('#divisionList', 'All Divisions', distinctDivisions());
        rebuildSelect('#ageGroupList', 'All Age Groups', distinctAgeGroups());
        rebuildSelect('#eventList', 'All Events', distinctEventTypes());
        rebuildTeamList(distinctTeams());
    }
    function removeSpecialCharacters(inputString) {
        return inputString.replace(/[^\w\s]/gi, '');
    }
    function divisionFromFile(fileName) {
        var name = String(fileName)
            .replace(/^z+(?=[A-Za-z])/, '')
            .replace(/^[a-z]-\s*(?=[A-Za-z0-9])/, '')
            .replace(/\b(Boys|Girls|Mens?|Womens?|Mixed|Combined)'?\b/gi, ' ')
            .replace(/\s{2,}/g, ' ')
            .replace(/\s*[-–—]\s*(?=[-–—])/g, '')
            .replace(/^[\s/–—-]+|[\s/–—-]+$/g, '')
            .trim();
        if (/(correction|corrected|revised|amended|re-?upload|do not use|duplicate)/i.test(name)) {
            return '';
        }
        return name;
    }
    function getTeam() {
        return String($('#teamList').val() || '');
    }
    function numVal(v) {
        var n = parseFloat(v);
        return isNaN(n) ? 0 : n;
    }
    function naturalCompare(a, b) {
        return String(a).localeCompare(String(b), undefined, { numeric: true, sensitivity: 'base' });
    }
    function sortEvents(list) {
        return list.slice().sort(function (a, b) {
            return (numVal(a.order) - numVal(b.order))
                || (numVal(a.distance) - numVal(b.distance))
                || naturalCompare(a.name || '', b.name || '')
                || naturalCompare(a.ageGroup || '', b.ageGroup || '')
                || naturalCompare(a.round || '', b.round || '')
                || naturalCompare(a.division || '', b.division || '');
        });
    }
    function eventId(event) {
        return removeSpecialCharacters(event.key);
    }
    function syncTabAria() {
        $('#genderTabs .genderTab').each(function () {
            $(this).attr('aria-pressed', $(this).hasClass('active') ? 'true' : 'false');
        });
    }
    function currentScope() {
        return {
            gender: activeGender(),
            level: String($('#levelList').val() || ''),
            division: String($('#divisionList').val() || ''),
            ageGroup: String($('#ageGroupList').val() || ''),
            eventType: String($('#eventList').val() || '')
        };
    }
    function resultMatchesLevel(result, level) {
        return !level || String(result.levelId || '') === level;
    }
    function eventHasLevel(event, level) {
        return !level || event.results().some(function (result) {
            return resultMatchesLevel(result, level);
        });
    }
    function eventMatchesScope(event, scope) {
        return (!scope.gender || event.gender === scope.gender)
            && eventHasLevel(event, scope.level)
            && (!scope.division || event.division === scope.division)
            && (!scope.ageGroup || (event.ageGroup || '') === scope.ageGroup)
            && (!scope.eventType || eventTypeKey(event) === scope.eventType);
    }
    function scopedEvents() {
        var scope = currentScope();
        return sortEvents(MeetResults.events().filter(function (event) { return eventMatchesScope(event, scope); }));
    }
    function syncUrl() {
        var params = new URLSearchParams(window.location.search);
        params.set('type', params.get('type') || 'formatted');
        var scope = currentScope();
        var event = String($('#eventList').val() || '');
        var team = getTeam();
        event ? params.set('event', event) : params.delete('event');
        allMode ? params.set('all', '1') : params.delete('all');
        scope.gender ? params.set('gender', scope.gender) : params.delete('gender');
        scope.level ? params.set('level', scope.level) : params.delete('level');
        scope.division ? params.set('division', scope.division) : params.delete('division');
        scope.ageGroup ? params.set('ageGroup', scope.ageGroup) : params.delete('ageGroup');
        team ? params.set('team', team) : params.delete('team');
        history.replaceState(null, '', "".concat(window.location.pathname, "?").concat(params.toString()));
    }
    function initFromUrl() {
        var params = new URLSearchParams(window.location.search);
        var gender = params.get('gender');
        var level = params.get('level');
        var division = params.get('division');
        var ageGroup = params.get('ageGroup');
        var event = params.get('event');
        if (gender) {
            $('#genderTabs .genderTab').removeClass('active');
            $("#genderTabs .genderTab[data-gender=\"".concat(gender, "\"]")).addClass('active');
            syncTabAria();
        }
        rebuildSelect('#levelList', 'All Levels', distinctLevels());
        if (level) {
            $('#levelList').val(level);
        }
        rebuildSelect('#divisionList', 'All Divisions', distinctDivisions());
        if (division) {
            $('#divisionList').val(division);
        }
        rebuildSelect('#ageGroupList', 'All Age Groups', distinctAgeGroups());
        if (ageGroup) {
            $('#ageGroupList').val(ageGroup);
        }
        rebuildSelect('#eventList', 'All Events', distinctEventTypes());
        if (event) {
            $('#eventList').val(event);
        }
        rebuildTeamList(distinctTeams());
        if (params.get('team')) {
            $('#teamList').val(params.get('team'));
        }
        allMode = !!params.get('all');
    }
    function showEmptyState() {
        $results.removeClass('loading').html("<p class=\"empty\">No formatted results are available for this section yet. " +
            "<a href=\"".concat(url, "/results/").concat(resultsId, "/raw\">View the Completed results</a> if they have been posted.</p>"));
    }
    function render() {
        rebuildDependentFilters();
        syncUrl();
        var team = getTeam();
        var scope = currentScope();
        var only = myOnly();
        var events = only
            ? sortEvents(MeetResults.events().filter(function (e) {
                return e.results().some(function (result) {
                    return resultMatchesLevel(result, scope.level) && isMine(result);
                });
            }))
            : scopedEvents();
        $results.empty();
        var anyFilter = !!(team || scope.gender || scope.level || scope.division || scope.ageGroup || scope.eventType || allMode || only);
        var showingIndex = !only && !team && !allMode && !scope.eventType && events.length > 1;
        if (showingIndex) {
            renderEventIndex(events);
        }
        else {
            renderEventTables(events);
        }
        $('.showAllField').toggleClass('hidden', !showingIndex);
        $('.clearFilters').toggleClass('hidden', !anyFilter);
    }
    function clearAllFilters() {
        $('#genderTabs .genderTab').removeClass('active');
        $('#genderTabs .genderTab[data-gender=""]').addClass('active');
        syncTabAria();
        $('#levelList, #divisionList, #ageGroupList, #eventList, #teamList').val('');
        $('#myResultsOnly').prop('checked', false);
        allMode = false;
        myShowFull = false;
        render();
    }
    function renderEventIndex(events) {
        var team = getTeam();
        var level = String($('#levelList').val() || '');
        if (team) {
            events = events.filter(function (e) {
                return e.results().some(function (r) { return r.teamName === team; });
            });
        }
        if (!events.length) {
            $results.append('<p class="empty">No events match the selected filters.</p>');
            return;
        }
        var byGender = {};
        var genderOrder = [];
        events.forEach(function (event) {
            var gender = event.gender || 'Results';
            if (!byGender[gender]) {
                byGender[gender] = [];
                genderOrder.push(gender);
            }
            byGender[gender].push(event);
        });
        genderOrder.sort();
        var hasAnyMine = events.some(function (e) {
            return e.results().some(function (result) {
                return resultMatchesLevel(result, level) && isMine(result);
            });
        });
        var html = '';
        if (hasAnyMine) {
            html += '<div class="eventIndexLegend">'
                + '<span class="eventIndexLegend__key"><i class="fa fa-star"></i> = a race your athletes &amp; teams are in</span>'
                + '<a href="#" id="myStarsToggle" class="eventIndexLegend__toggle">' + (myStarsOff ? 'Show markers' : 'Hide markers') + '</a>'
                + '</div>';
        }
        html += '<div class="eventIndex">';
        genderOrder.forEach(function (gender) {
            html += "<div class=\"eventIndexGroup\"><h3 class=\"eventIndexHeading\">".concat(gender, "</h3><ul class=\"eventIndexList\">");
            var indexKey = function (e) {
                return eventTypeKey(e) + '|' + (e.division || '') + '|' + (e.ageGroup || '');
            };
            var mineByKey = {};
            byGender[gender].forEach(function (e) {
                var k = indexKey(e);
                mineByKey[k] = mineByKey[k] || e.results().some(function (result) {
                    return resultMatchesLevel(result, level) && isMine(result);
                });
            });
            var seenKey = {};
            byGender[gender].forEach(function (event) {
                var key = indexKey(event);
                if (seenKey[key]) {
                    return;
                }
                seenKey[key] = true;
                var metaLabels = [];
                if (event.ageGroup) {
                    metaLabels.push(escapeHtml(event.ageGroup));
                }
                if (event.division) {
                    metaLabels.push(escapeHtml(event.division));
                }
                var meta = metaLabels.length ? "<span class=\"eventIndexDivision\">".concat(metaLabels.join(' &middot; '), "</span>") : '';
                var mine = !myStarsOff && !!mineByKey[key];
                var star = mine ? '<i class="fa fa-star eventIndexStar" title="Your athletes & teams are in this race"></i>' : '';
                html += "<li><a href=\"#\" class=\"eventIndexLink".concat(mine ? ' hasMine' : '', "\" data-gender=\"").concat(event.gender || '', "\" data-division=\"").concat((event.division || '').replace(/"/g, '&quot;'), "\" data-age-group=\"").concat((event.ageGroup || '').replace(/"/g, '&quot;'), "\" data-event-type=\"").concat(eventTypeKey(event), "\">")
                    + "<span class=\"eventIndexName\">".concat(star).concat(event.name, "</span> ").concat(meta)
                    + "</a></li>";
            });
            html += '</ul></div>';
        });
        html += '</div>';
        $results.append(html);
    }
    function renderEventTables(events) {
        var team = getTeam();
        var only = myOnly();
        var list = events;
        if (team) {
            list = events.filter(function (e) {
                return e.results().some(function (r) { return r.teamName === team; });
            });
        }
        var teamScoresCount = 0;
        var out = [];
        list.forEach(function (event) {
            var table = buildEventTable(event, team);
            if (!table) {
                return;
            }
            out.push(table);
            if (event.teamScores.length > 0) {
                var myTeams_1 = null;
                if (only && !myShowFull) {
                    myTeams_1 = {};
                    event.results().forEach(function (r) {
                        if (r.teamId && followedTeams[String(r.teamId)] && r.teamName) {
                            myTeams_1[r.teamName] = true;
                        }
                    });
                }
                var ts = buildTeamScoresTable(event, teamScoresCount + 1, team, myTeams_1);
                if (ts) {
                    teamScoresCount++;
                    out.push(ts);
                }
            }
        });
        if (!out.length) {
            $results.append('<p class="empty">' + (only
                ? 'None of the athletes or teams you follow have results at this meet.'
                : 'No results match the selected filters.') + '</p>');
            return;
        }
        if (only) {
            var lead = myShowFull
                ? '<i class="fa fa-star"></i> Full results &mdash; your athletes &amp; teams highlighted'
                : '<i class="fa fa-star"></i> Showing your athletes &amp; teams';
            var toggle = myShowFull
                ? 'Show only my athletes &amp; teams'
                : 'See everyone in these races';
            $results.append('<div class="myFollowIntro">'
                + '<div class="myFollowIntro__lead">' + lead + '</div>'
                + '<a href="#" class="myFollowIntro__toggle" id="myFollowFullToggle">' + toggle + '</a>'
                + '</div>');
        }
        $results.append(out.join(' '));
    }
    function gradeAbbr(grade) {
        switch (parseInt(grade, 10)) {
            case 12: return 'SR';
            case 11: return 'JR';
            case 10: return 'SO';
            case 9: return 'FR';
            default: return String(grade);
        }
    }
    function escAttr(value) {
        return String(value === undefined || value === null ? '' : value)
            .replace(/&/g, '&amp;').replace(/"/g, '&quot;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
    }
    function escHtml(value) {
        return String(value === undefined || value === null ? '' : value)
            .replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
    }
    function teamLogoImage(className, source, accessibleName) {
        if (accessibleName === void 0) { accessibleName = ''; }
        var fallback = String(meetResultParams.teamLogoFallback || '');
        var imageSource = String(source || fallback);
        if (!imageSource) {
            return '';
        }
        return '<img class="' + escAttr(className) + '" src="' + escAttr(imageSource)
            + '" alt="' + escAttr(accessibleName) + '" data-logo-fallback="' + escAttr(fallback)
            + '" onerror="var f=this.getAttribute(\'data-logo-fallback\');if(!f){this.style.display=\'none\';}else if(this.getAttribute(\'src\')!==f){this.src=f;}else{this.style.display=\'none\';}">';
    }
    function openTeamModal(name, logo, list) {
        var rows = (list || []).map(function (a) {
            var nm = a.u ? ('<a href="' + escAttr(a.u) + '">' + escHtml(a.nm) + '</a>') : escHtml(a.nm);
            var grade = a.g ? (' <span class="tmGrade">' + escHtml(a.g) + '</span>') : '';
            return '<tr><td class="tmPlace">' + escHtml(a.pl) + '</td><td class="tmName">' + nm + grade
                + '</td><td class="tmTime">' + escHtml(a.t) + '</td><td class="tmPts">' + escHtml(a.pt) + '</td></tr>';
        }).join('');
        var body = rows
            ? '<table class="tsModalTable"><thead><tr><th class="tmPlace">Pl</th><th class="tmName">Athlete</th><th class="tmTime">Time</th><th class="tmPts">Pts</th></tr></thead><tbody>' + rows + '</tbody></table>'
            : '<p class="tsModalEmpty">No individual results available for this team.</p>';
        var logoImg = teamLogoImage('tsModalLogo', logo);
        $('#tsModal').remove();
        $('body').append('<div class="tsModal" id="tsModal"><div class="tsModalBackdrop"></div>'
            + '<div class="tsModalPanel" role="dialog" aria-modal="true" aria-label="' + escAttr(name) + ' athletes">'
            + '<div class="tsModalHead">' + logoImg + '<div class="tsModalTitle">' + escHtml(name) + '</div>'
            + '<button type="button" class="tsModalClose" aria-label="Close">&times;</button></div>'
            + '<div class="tsModalBody">' + body + '</div></div></div>');
    }
    function buildEventTable(event, team) {
        var eventKey = eventId(event);
        var hasPoints = '', hasHeat = '', hasWind = '';
        var resultsArray = [];
        var track = event.isTrack;
        var relay = event.isRelay;
        var showMineOnly = myOnly() && !myShowFull;
        var level = String($('#levelList').val() || '');
        var rows = event.results().filter(function (result) { return resultMatchesLevel(result, level) && (result.mark != "0.00" || result.statusCode) && (!team || result.teamName === team) && (!showMineOnly || isMine(result)); });
        if (!rows.length) {
            return '';
        }
        var hasVideo = rows.some(function (result) { return !!result.performanceVideoId; });
        var showWind = !relay && track && rows.some(function (r) { return r.windReading && String(r.windReading).trim() !== ''; });
        var heatSet = {};
        rows.forEach(function (r) { if (r.heat) {
            heatSet[String(r.heat)] = true;
        } });
        var showHeat = track && Object.keys(heatSet).length > 1;
        rows.forEach(function (result) {
            var videoLink = '';
            if (result.performanceVideoId) {
                videoLink = "<a class=\"videoLink\" href=\"/videos/".concat(result.performanceVideoId, "\" target=\"_blank\"><i class=\"fa fa-play\"></i></a>");
            }
            var logoName = team && !relay ? result.teamName : '';
            var logoCell = "<td class=\"rowLogo\">".concat(teamLogoImage('teamLogo', result.teamLogo, logoName), "</td>");
            var gradeTag = (!relay && result.grade) ? " <span class=\"rowGradeTag\">".concat(gradeAbbr(result.grade), "</span>") : '';
            var heatTag = showHeat && result.heat ? " <span class=\"cardHeat\">H".concat(result.heat, "</span>") : '';
            var athleteCell = '';
            var teamCell = '';
            if (!relay) {
                var name_1 = "".concat(result.firstName.charAt(0).toUpperCase() + result.firstName.slice(1), " ").concat(result.lastName.charAt(0).toUpperCase() + result.lastName.slice(1));
                var nameLink = result.profileUrl ? "<a href=\"".concat(result.profileUrl, "\">").concat(name_1, "</a>") : name_1;
                athleteCell = "<td class=\"athlete\">".concat(nameLink).concat(gradeTag).concat(heatTag, "</td>");
                var teamLink = result.teamProfileUrl
                    ? "<a href=\"".concat(result.teamProfileUrl, "\" class=\"teamName\">").concat(result.teamName, "</a>")
                    : "<span class=\"teamName\">".concat(result.teamName, "</span>");
                teamCell = "<td class=\"team\">".concat(teamLink, "</td>");
            }
            else {
                var relayLink = result.teamProfileUrl ? "<a href=\"".concat(result.teamProfileUrl, "\">").concat(result.teamName, "</a>") : result.teamName;
                athleteCell = "<td class=\"athlete\">".concat(relayLink).concat(heatTag, "</td>");
            }
            var windMobile = showWind && result.windReading ? " <span class=\"cardWind\">".concat(result.windReading, "</span>") : '';
            var string = "\n                <tr class=\"".concat(isMine(result) ? 'myRow' : '', "\">\n                    <td class=\"place\">").concat(result.place ? result.place : '', "</td>\n                    ").concat(hasVideo ? "<td class=\"video\">".concat(videoLink, "</td>") : '', "\n                    ").concat(logoCell, "\n                    ").concat(athleteCell, "\n                    ").concat(teamCell, "\n                    <td class=\"finish\">").concat(result.statusCode ? result.statusCode : result.mark).concat(windMobile, "</td>");
            if (result.points && !result.statusCode) {
                string += "<td class=\"point\">".concat(result.points, "</td>");
                hasPoints = '<th>Points</th>';
            }
            if (showWind) {
                string += "<td class=\"wind\">".concat(result.windReading || '', "</td>");
                hasWind = '<th>Wind</th>';
            }
            if (showHeat) {
                string += "<td class=\"heat\">".concat(result.heat || '', "</td>");
                hasHeat = '<th>Heat</th>';
            }
            string += "</tr>";
            resultsArray.push(string);
        });
        var colHeaders = relay ? "<th></th><th>Team</th>" : "<th></th><th>Athlete</th><th>Team</th>";
        return "\n            <div class=\"eventResult".concat(team ? ' teamFiltered' : '', "\" data-event-key=\"").concat(eventKey, "\" data-gender=\"").concat(event.gender, "\" data-division-id=\"").concat(event.division, "\" data-age-group-name=\"").concat(event.ageGroup || '', "\">\n                <div class=\"eventNameDiv\">\n                    <p class=\"eventName\"> ").concat(eventDisplayFullName(event), "</p>\n                    <i class=\"fa fa-chevron-up chevron\" data-event=\"").concat(eventKey, "\"></i>\n                </div>\n                <table class=\"eventTable").concat(hasVideo ? ' hasVideo' : '', "\" id=\"").concat(eventKey, "\">\n                    <thead>\n                        <tr class=\"eventHeadRow\">\n                            <th>Place</th>\n                            ").concat(hasVideo ? '<th>Video</th>' : '', "\n                            ").concat(colHeaders, "\n                            <th>Mark</th>\n                            ").concat(hasPoints, "\n                            ").concat(hasWind, "\n                            ").concat(hasHeat, "\n                        </tr>\n                    </thead>\n                    <tbody>\n                        ").concat(resultsArray.join(' '), "\n                    </tbody>\n                </table>\n            </div>");
    }
    function buildTeamScoresTable(event, teamScoresCount, team, myTeams) {
        var teams = event.teamScores.filter(function (score) { return (!team || score.name === team) && (!myTeams || myTeams[score.name]); });
        if (!teams.length) {
            return '';
        }
        var SCORER_COLS = 5;
        var hasDisp = false;
        var hasAvg = false;
        teams.forEach(function (s) {
            if (String(s.displacers || '').replace(/[^0-9]/g, '') !== '') {
                hasDisp = true;
            }
            if (s.avg || s.split) {
                hasAvg = true;
            }
        });
        var resultsByTeam = {};
        var followedTeamNames = {};
        event.results().forEach(function (r) {
            if (r.teamId && followedTeams[String(r.teamId)] && r.teamName) {
                followedTeamNames[r.teamName] = true;
            }
            if ((r.mark == '0.00' && !r.statusCode) || !r.teamName) {
                return;
            }
            (resultsByTeam[r.teamName] = resultsByTeam[r.teamName] || []).push(r);
        });
        function tsTitleCase(s) { s = String(s || ''); return s.charAt(0).toUpperCase() + s.slice(1); }
        function teamDataAttr(score) {
            var list = (resultsByTeam[score.name] || []).slice()
                .sort(function (a, b) { return (parseInt(a.place, 10) || 9999) - (parseInt(b.place, 10) || 9999); })
                .map(function (r) {
                return {
                    pl: r.place || '',
                    nm: (tsTitleCase(r.firstName) + ' ' + tsTitleCase(r.lastName)).trim(),
                    u: r.profileUrl || '',
                    t: r.statusCode ? r.statusCode : r.mark,
                    pt: (r.points && !r.statusCode) ? r.points : '',
                    g: r.grade ? gradeAbbr(r.grade) : ''
                };
            });
            return ' data-team-name="' + escAttr(score.name) + '" data-team-logo="' + escAttr(score.teamLogo || '')
                + '" data-team-athletes="' + escAttr(JSON.stringify(list)) + '"';
        }
        var head = '<th class="tsPlaceHead">Place</th><th class="tsLogoHead"></th><th class="tsTeamHead">Team</th><th class="tsPointsHead">Pts</th>';
        for (var i = 1; i <= SCORER_COLS; i++) {
            head += "<th class=\"tsScorerHead\">".concat(i, "</th>");
        }
        if (hasDisp) {
            head += '<th class="tsScorerHead disp">6</th><th class="tsScorerHead disp">7</th>';
        }
        if (hasAvg) {
            head += '<th class="tsGap"></th><th class="tsAvgHead">Avg</th><th class="tsSpreadHead">Spread</th>';
        }
        var rows = teams.map(function (score) {
            var places = String(score.scorers || '').split(/[+,]/);
            var disp = String(score.displacers || '').replace(/[()\s]/g, '').split(/[+,]/).filter(function (x) { return x !== ''; });
            var cells = "<td class=\"place\">".concat(score.place, "</td>");
            cells += "<td class=\"tsLogo\">".concat(teamLogoImage('teamLogo', score.teamLogo), "</td>");
            cells += "<td class=\"tsTeam\"><a href=\"".concat(score.teamProfileUrl, "\">").concat(score.name, "</a></td>");
            cells += "<td class=\"tsPoints\">".concat(score.points, "</td>");
            for (var i = 0; i < SCORER_COLS; i++) {
                cells += "<td class=\"tsScorer\">".concat(places[i] !== undefined ? places[i].trim() : '', "</td>");
            }
            if (hasDisp) {
                cells += "<td class=\"tsScorer disp\">".concat(disp[0] !== undefined ? '(' + disp[0] + ')' : '', "</td>");
                cells += "<td class=\"tsScorer disp\">".concat(disp[1] !== undefined ? '(' + disp[1] + ')' : '', "</td>");
            }
            if (hasAvg) {
                cells += '<td class="tsGap"></td>';
                cells += "<td class=\"tsAvg\">".concat(score.avg ? '<strong>' + score.avg + '</strong>' : '', "</td>");
                cells += "<td class=\"tsSpread\">".concat(score.split ? '<strong>' + score.split + '</strong>' : '', "</td>");
            }
            var mine = followedTeamNames[score.name] ? ' tsMine' : '';
            return "<tr class=\"tsRow".concat(mine, "\"").concat(teamDataAttr(score), ">").concat(cells, "</tr>");
        });
        var cards = teams.map(function (score) {
            var scorerSpans = String(score.scorers || '').split(/[+,]/)
                .filter(function (x) { return x.trim() !== ''; })
                .map(function (p) { return "<span class=\"s\">".concat(p.trim(), "</span>"); }).join('');
            var meta = [];
            if (score.avg) {
                meta.push("Avg <b>".concat(score.avg, "</b>"));
            }
            if (score.split) {
                meta.push("Spread <b>".concat(score.split, "</b>"));
            }
            if (score.displacers) {
                meta.push("Displacers ".concat(String(score.displacers).replace(/[()]/g, '').replace(/\+/g, ', ')));
            }
            var cardMine = followedTeamNames[score.name] ? ' tsMine' : '';
            return "\n                <div class=\"tsCard".concat(cardMine, "\"").concat(teamDataAttr(score), ">\n                    <span class=\"tscRank\">").concat(score.place, "</span>\n                    ").concat(teamLogoImage('tscLogo', score.teamLogo), "\n                    <a class=\"tscTeam\" href=\"").concat(score.teamProfileUrl, "\">").concat(score.name, "</a>\n                    <div class=\"tscPts\">").concat(score.points, "<span>pts</span></div>\n                    <div class=\"tscScorers\"><span class=\"lbl\">Scorers</span>").concat(scorerSpans, "</div>\n                    <div class=\"tscMeta\">").concat(meta.join('<span class="sep">|</span>'), "</div>\n                </div>");
        });
        return "\n            <div class=\"eventResult\" data-event-key=\"".concat(eventId(event), "\" data-gender=\"").concat(event.gender, "\" data-division-id=\"").concat(event.division, "\" data-age-group-name=\"").concat(event.ageGroup || '', "\">\n                <div class=\"eventNameDiv\">\n                    <p class=\"eventName\"> ").concat(eventDisplayFullName(event), " Team Scores</p>\n                    <i class=\"fa fa-chevron-up chevron\" data-event=\"ts-").concat(teamScoresCount, "\"></i>\n                </div>\n                <div id=\"ts-").concat(teamScoresCount, "\">\n                    <div class=\"tsScroll\">\n                        <table class=\"eventTable teamScoresTable\">\n                            <thead><tr class=\"eventHeadRow\">").concat(head, "</tr></thead>\n                            <tbody>").concat(rows.join(''), "</tbody>\n                        </table>\n                    </div>\n                    <div class=\"tsCards\">").concat(cards.join(''), "</div>\n                </div>\n            </div>");
    }
    $('#genderTabs').on('click', '.genderTab', function () {
        $('#genderTabs .genderTab').removeClass('active');
        $(this).addClass('active');
        syncTabAria();
        render();
    });
    $('#eventList').on('change', function () {
        allMode = false;
        render();
    });
    $('#levelList, #divisionList, #ageGroupList, #teamList').on('change', render);
    $('#showAllEvents').on('click', function (e) {
        e.preventDefault();
        allMode = true;
        render();
    });
    $('#myResultsOnly').on('change', function () {
        myShowFull = false;
        render();
    });
    $('body').on('click', '.myFollowOpen', function (e) {
        e.preventDefault();
        buildFollowLists();
        $('#myFollowPickerSearch').val('');
        renderPicker('');
        $('#myFollowPicker').removeClass('hidden');
        $('#myFollowPickerSearch').trigger('focus');
    });
    $('body').on('click', '#myFollowPickerClose', function () {
        $('#myFollowPicker').addClass('hidden');
    });
    $('body').on('input', '#myFollowPickerSearch', function () {
        renderPicker($(this).val());
    });
    $('body').on('click', '.myFollowRow__btn', function () {
        var $btn = $(this);
        if ($btn.hasClass('is-following')) {
            return;
        }
        var $row = $btn.closest('.myFollowRow');
        doFollow(String($row.attr('data-type')), $row.attr('data-id'), $btn);
    });
    $results.on('click', '#myFollowFullToggle', function (e) {
        e.preventDefault();
        myShowFull = !myShowFull;
        render();
    });
    $results.on('click', '#myStarsToggle', function (e) {
        e.preventDefault();
        myStarsOff = !myStarsOff;
        render();
    });
    $('body').on('click', '.clearFilters', function (e) {
        e.preventDefault();
        clearAllFilters();
    });
    $results.on('click', '.eventIndexLink', function (e) {
        e.preventDefault();
        allMode = false;
        var $l = $(this);
        var g = String($l.data('gender') || '');
        $('#genderTabs .genderTab').removeClass('active');
        $('#genderTabs .genderTab[data-gender="' + g + '"]').addClass('active');
        syncTabAria();
        rebuildSelect('#levelList', 'All Levels', distinctLevels());
        rebuildSelect('#divisionList', 'All Divisions', distinctDivisions());
        $('#divisionList').val(String($l.data('division') || ''));
        rebuildSelect('#ageGroupList', 'All Age Groups', distinctAgeGroups());
        $('#ageGroupList').val(String($l.data('age-group') || ''));
        rebuildSelect('#eventList', 'All Events', distinctEventTypes());
        $('#eventList').val(String($l.data('event-type') || ''));
        render();
    });
    $('body').on('change', 'select.videoHeatDropDown', function () {
        var $link = $(this).closest('tr').find('a.videoLink');
        $link.attr('href', '/videos/' + $(this).val());
    });
    $('body').on('click', '.fa-chevron-down', function (e) {
        e.preventDefault();
        var id = $(e.target).data('event');
        $('#' + id).toggleClass('hidden');
        $(e.target).toggleClass('fa-chevron-down fa-chevron-up');
    });
    $('body').on('click', '.fa-chevron-up', function (e) {
        e.preventDefault();
        var id = $(e.target).data('event');
        $('#' + id).toggleClass('hidden');
        $(e.target).toggleClass('fa-chevron-down fa-chevron-up');
    });
    $('body').on('click', '.tsRow, .tsCard', function (e) {
        if ($(e.target).closest('a').length) {
            return;
        }
        var raw = $(this).attr('data-team-athletes');
        if (raw === undefined) {
            return;
        }
        var list = [];
        try {
            list = JSON.parse(raw);
        }
        catch (err) {
            list = [];
        }
        openTeamModal($(this).attr('data-team-name') || '', $(this).attr('data-team-logo') || '', list);
    });
    $('body').on('click', '.tsModalClose, .tsModalBackdrop', function () { $('#tsModal').remove(); });
    $(document).on('keyup', function (e) { if (e.key === 'Escape' || e.keyCode === 27) {
        $('#tsModal').remove();
    } });
});
//# sourceMappingURL=loadResultsNew.js.map