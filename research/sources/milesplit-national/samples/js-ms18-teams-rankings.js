var RegexHelper = Drivefaze.RegexHelper;
(function ($) {
    var cache = {};
    var $leaguesDataset = $("#leaguesDataset");
    var $leagueSeasonFilter = $("#leaguesSeasonFilter");
    var $seasonBestDataset = $("#seasonBestDataset");
    var $seasonBestTypeFilter = $("#seasonBestTypeFilter");
    var $seasonBestGradeFilter = $("#seasonBestGradeFilter");
    var $seasonBestGenderFilter = $("#seasonBestGenderFilter");
    var $teamRecordsDataset = $("#teamRecordsDataset");
    var $teamRecordsEventFilter = $("#teamRecordsEventFilter");
    var $teamRecordsGenderFilter = $("#teamRecordsGenderFilter");
    var $teamRecordsSeasonFilter = $("#teamRecordsSeasonFilter");
    var leaguesUrl = "v1/teams/" + $leaguesDataset.data("teamId") + "/rankings";
    var seasonBestUrl = "v1/teams/" + $seasonBestDataset.data("teamId") + "/best";
    var teamRecordsUrl = "v1/teams/" + $teamRecordsDataset.data("teamId") + "/records";
    function generateCacheKey(url, options) {
        var key = url;
        if (0 < Object.keys(options).length) {
            var queryString = Object.keys(options)
                .map(function (key) { return key + "=" + options[key]; })
                .join("&");
            key = key + "?" + queryString;
        }
        return btoa(key);
    }
    function cacheKeyExists(key) {
        return -1 !== Object.keys(cache).indexOf(key);
    }
    var getLeaguesOptions = function () {
        var options = { season: "all" };
        if ($leagueSeasonFilter) {
            options["season"] = $leagueSeasonFilter.val();
        }
        return options;
    };
    var getSeasonBestOptions = function () {
        var options = { gender: "m", grade: "all" };
        if ($seasonBestGenderFilter) {
            options.gender = $seasonBestGenderFilter.val();
        }
        if ($seasonBestGradeFilter) {
            options.grade = $seasonBestGradeFilter.val();
        }
        if ($seasonBestTypeFilter) {
            options.season = $seasonBestTypeFilter.val();
        }
        return options;
    };
    var getTeamRecordsOptions = function () {
        var options = {};
        if ($teamRecordsSeasonFilter) {
            options["season"] = $teamRecordsSeasonFilter.val();
            if ("cc" === options["season"]) {
                $teamRecordsEventFilter.val("").closest(".select-wrapper").hide();
            }
            else if (!$teamRecordsEventFilter.is(":visible")) {
                $teamRecordsEventFilter.closest(".select-wrapper").show();
            }
        }
        if ($teamRecordsGenderFilter) {
            options["gender"] = $teamRecordsGenderFilter.val();
        }
        if ($teamRecordsEventFilter && "" !== $teamRecordsEventFilter.val()) {
            options["eventType"] = $teamRecordsEventFilter.val();
        }
        return options;
    };
    var updateLeagues = function () {
        var options = getLeaguesOptions();
        var key = generateCacheKey(leaguesUrl, options);
        if (!cacheKeyExists(key)) {
            $leaguesDataset.html(Handlebars.templates["loading"]({
                text: "Loading Leagues...",
            }));
            _DF_.API.get(leaguesUrl, getLeaguesOptions()).done(function (json) {
                cache[key] = json;
                $leaguesDataset.html(Handlebars.templates["team.leagues"](json));
            });
        }
        else {
            $leaguesDataset.html(Handlebars.templates["team.leagues"](cache[key]));
        }
    };
    var updateSeasonBest = function () {
        var options = getSeasonBestOptions();
        var key = generateCacheKey(seasonBestUrl, options);
        if (!cacheKeyExists(key)) {
            $seasonBestDataset.html(Handlebars.templates["loading"]({
                text: "Searching Season's Best...",
            }));
            _DF_.API.get(seasonBestUrl, getSeasonBestOptions()).done(function (json) {
                var performances = {};
                var best = [];
                $.each(Object.keys(json.data.eventGroups), function (key, event) {
                    for (var p = 0; p < json.data.eventGroups[event].performances.length; p++) {
                        var performance_1 = json.data.eventGroups[event].performances[p];
                        if (!performances[performance_1.eventCode]) {
                            performances[performance_1.eventCode] = {
                                athlete: performance_1.firstName + " " + performance_1.lastName,
                                athleteLink: performance_1.athleteLink,
                                eventCode: performance_1.eventCode,
                                eventLink: performance_1.eventLink,
                                gender: performance_1.gender,
                                gradYear: performance_1.hsGraduationYear,
                                mark: performance_1.mark,
                                meet: performance_1.meetName,
                                meetLink: performance_1.meetLink,
                            };
                            if (RegexHelper.EventCodeRelay.test(performance_1.eventCode)) {
                                performances[performance_1.eventCode].athlete =
                                    performance_1.teamName;
                                performances[performance_1.eventCode].athleteLink =
                                    performance_1.teamLink;
                            }
                        }
                    }
                });
                Object.keys(performances)
                    .sort()
                    .forEach(function (key) {
                    best.push(performances[key]);
                });
                cache[key] = best;
                $seasonBestDataset.html(Handlebars.templates["team.season-best"]({ best: best }));
            });
        }
        else {
            $seasonBestDataset.html(Handlebars.templates["team.season-best"]({ best: cache[key] }));
        }
    };
    var updateTeamRecords = function () {
        var options = getTeamRecordsOptions();
        var key = generateCacheKey(teamRecordsUrl, options);
        if (!cacheKeyExists(key)) {
            $teamRecordsDataset.html(Handlebars.templates["loading"]({
                text: "Loading Team Records...",
            }));
            _DF_.API.get(teamRecordsUrl, getTeamRecordsOptions()).done(function (json) {
                $.each(Object.keys(json.data.eventGroups), function (key, event) {
                    for (var p = 0; p < json.data.eventGroups[event].performances.length; p++) {
                        var performance_2 = json.data.eventGroups[event].performances[p];
                        performance_2.athlete =
                            performance_2.firstName + " " + performance_2.lastName;
                        if (RegexHelper.EventCodeRelay.test(performance_2.eventCode)) {
                            performance_2.athlete = performance_2.teamName;
                            performance_2.athleteLink = performance_2.teamLink;
                        }
                        json.data.eventGroups[event].performances[p] = performance_2;
                    }
                });
                cache[key] = json;
                $teamRecordsDataset.html(Handlebars.templates["team.records"](json));
            });
        }
        else {
            $teamRecordsDataset.html(Handlebars.templates["team.records"](cache[key]));
        }
    };
    $(document).ready(function () {
        $leagueSeasonFilter.change(updateLeagues);
        updateLeagues();
        $seasonBestGenderFilter.change(updateSeasonBest);
        $seasonBestGradeFilter.change(updateSeasonBest);
        $seasonBestTypeFilter.change(updateSeasonBest);
        updateSeasonBest();
        $teamRecordsEventFilter.change(updateTeamRecords);
        $teamRecordsGenderFilter.change(updateTeamRecords);
        $teamRecordsSeasonFilter.change(updateTeamRecords);
        updateTeamRecords();
    });
})($);
//# sourceMappingURL=rankings.js.map