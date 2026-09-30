(function ($) {
    $(document).ready(function () {
        var cache = {};
        var $scheduleDataset = $('#schedule');
        var $scheduleSeason = $('#scheduleSeason');
        var $scheduleSeasonYear = $('#scheduleSeasonYear');
        var $rankedDataset = $('#rankedPerformances');
        var $rankedFilterSeason = $('#rankedPerformancesFilterSeason');
        var $rankedFilterGrade = $('#rankedPerformancesFilterGrade');
        var $rankedFilterGender = $('#rankedPerformancesFilterGender');
        var $rankedFilterType = $('#rankedPerformancesFilterType');
        var updateScheduleUrl = 'v1/teams/' + $scheduleDataset.data('teamId') + '/schedules';
        var updateRosterUrl = 'v1/rosters/teams/' + $rankedDataset.data('teamId') + '/ranked';
        function generateCacheKey(url, options) {
            var key = url;
            if (0 < Object.keys(options).length) {
                var queryString = Object.keys(options)
                    .map(function (key) { return key + '=' + options[key]; })
                    .join('&');
                key = key + '?' + queryString;
            }
            return btoa(key);
        }
        function cacheKeyExists(key) {
            return -1 !== Object.keys(cache).indexOf(key);
        }
        function getScheduleOptions() {
            return {
                season: $scheduleSeason.val(),
                year: $scheduleSeasonYear.val()
            };
        }
        function getRankedPerformancesOptions() {
            var options = {
                formatRank: true
            };
            if ($rankedFilterGender.is(':visible') && $rankedFilterGender.val()) {
                options['gender'] = $rankedFilterGender.val();
            }
            if ($rankedFilterGrade && $rankedFilterGrade.is(':visible') && $rankedFilterGrade.val()) {
                options['grade'] = $rankedFilterGrade.val();
            }
            if ($rankedFilterType.val()) {
                options['ranking'] = $rankedFilterType.val();
            }
            if ($rankedFilterSeason.is(':visible') && $rankedFilterSeason.val()) {
                options['season'] = $rankedFilterSeason.val();
            }
            return options;
        }
        function updateTeamSchedule() {
            var options = getScheduleOptions();
            var key = generateCacheKey(updateScheduleUrl, options);
            if (!cacheKeyExists(key)) {
                $scheduleDataset.html(Handlebars.templates['loading']({
                    text: 'Loading Schedule...'
                }));
                _DF_.API.get(updateScheduleUrl, options).done(function (json) {
                    cache[key] = json;
                    $scheduleDataset.html(Handlebars.templates['team.schedule'](json));
                });
            }
            else {
                $scheduleDataset.html(Handlebars.templates['team.schedule'](cache[key]));
            }
        }
        function updateTeamRankedPerformances() {
            var options = getRankedPerformancesOptions();
            var key = generateCacheKey(updateRosterUrl, options);
            if (-1 === Object.keys(cache).indexOf(key)) {
                $rankedDataset.html(Handlebars.templates['loading']({
                    text: 'Loading Ranked Performances...'
                }));
                _DF_.API.get(updateRosterUrl, getRankedPerformancesOptions()).done(function (json) {
                    if (json.data) {
                        $.each(json.data, function (i, athlete) {
                            if ('0' == athlete.gradYear) {
                                athlete.gradYear = '--';
                            }
                            if ($rankedDataset.parents('.paywall-overlay-container').hasClass('paywall-active')) {
                                athlete.stateRank = _DF_.Helper.obfuscateNumber(athlete.stateRank);
                                athlete.nationalRank = _DF_.Helper.obfuscateNumber(athlete.nationalRank);
                            }
                        });
                    }
                    cache[key] = json;
                    $rankedDataset.html(Handlebars.templates['team.rankings'](json));
                });
            }
            else {
                $rankedDataset.html(Handlebars.templates['team.rankings'](cache[key]));
            }
        }
        $scheduleSeason.change(updateTeamSchedule);
        $scheduleSeasonYear.change(updateTeamSchedule);
        updateTeamSchedule();
        $rankedFilterGender.change(updateTeamRankedPerformances);
        $rankedFilterGrade.change(updateTeamRankedPerformances);
        $rankedFilterSeason.change(updateTeamRankedPerformances);
        $rankedFilterType.change(updateTeamRankedPerformances);
        updateTeamRankedPerformances();
    });
})($);
//# sourceMappingURL=index.js.map