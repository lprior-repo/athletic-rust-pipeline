_DF_.MeetResults = function(opts) {

    // Private
    var me = this,
        resultsData = [],
        teamData = new _DF_.HashTable(),
        eventData = new _DF_.HashTable(),
        gradeData = new _DF_.HashTable(),
        phantomRunnerRule = opts.phantomRunnerRule || false,
        teamScores = opts.teamScores || 'team',
        fieldTeamId = 'teamId',
        fieldTeamName = 'teamName',
        fieldTeamUrl  = 'teamProfileUrl',
        fieldTeamLogo = 'teamLogo';

    var gradeName = {
        6: '6th Graders',
        7: '7th Graders',
        8: '8th Graders',
        9: 'Freshmen',
        10: 'Sophomores',
        11: 'Juniors',
        12: 'Seniors'
    };

    // Scoring params
    if (teamScores != 'team') {
        fieldTeamId = 'teamRegion';
        fieldTeamName = 'teamRegion';
    }

    me.meetId = opts.meetId || 0;
    me.resultsId = opts.resultsId || 0;
    me.isTrack = opts.isTrack || false;
    me.isIndoor = me.isTrack && opts.season.toLowerCase() == 'indoor';
    me.isOutdoor = me.isTrack && opts.season.toLowerCase() == 'outdoor';
    me.teamScores = teamScores != 'none';

    // Private methods
    var getEventType = function(eventName) {
        if (/(relay)|(medley)/i.test(eventName)) {
            return 'T';
        }
        else if (/(jump)|(vault)|(throw)|(discus)|(toss)|(javelin)|(shot)/i.test(eventName)) {
            return 'F';
        }
        else if (/(athlon)/i.test(eventName)) {
            return 'M';
        }
        else {
            return 'R';
        }
    };
    var getGradeName = function(grade) {
        return gradeName[parseInt(grade)] || grade;
    };
    var getResults = function() {
        var resultRows = [];
        $.each(
            this.resultsIndex, function(i, resultsIndex) {
                resultRows.push(me.rows(resultsIndex));
            }
        );
        return resultRows;
    };
    var prepareTemplateVideoParams = function(event) {
        if (typeof event == 'undefined') {
            $.each(me.events(), function() {
               prepareTemplateVideoParams(this);
            });
        }
        else {
            // set an array
            event.videoIds = event.videoIdsByHeat.asc(['heat numeric']).get();
            // set defaults
            event.videoId = '';
            event.videoLinkClass = 'hide';
            event.videoHeatDropDownClass = 'hide';
            // if we have at least one video
            if (event.videoIds.length > 0) {
                // set the first videoId
                event.videoId = event.videoIds[0].id;
                // display the single link
                event.videoLinkClass = '';
                // if we have more than one video
                if (event.videoIds.length > 1) {
                    // display the dropdown & hide the single link
                    event.videoLinkClass = 'hide';
                    event.videoHeatDropDownClass = '';
                }
            }
        }
    };
    // Public Methods
    me.loadResults = function(data) {
        // Loop through each
        $.each(data, function(i){
            // Save this results row (as well as event team etc..)
            if (this.mark != "0.00") me.addResult(this); // TODO handle flagged results
        });
        // Get team scores for each event - XC
        if (me.teamScores && !me.isTrack) {
            $.each(me.events(), function(i, event) {
                prepareTemplateVideoParams(event);
                var place = 1,
                    scores = me.scoreXC(event.key);
                $.each(scores.asc(['points numeric', 'sixthRunner numeric']).get(), function() {
                    var avg = this.scorerSum == 0 ? "--:--" : me.unitsToTime(this.scorerSum / 5),
                        split = this.scorerSum == 0 ? "-- - --" : me.unitsToTime(this.lastScorer - this.firstScorer);
                    if (this.points > 0) {
                        event.teamScores.push({
                            place:place,
                            name:this.name,
                            teamProfileUrl:this.teamProfileUrl,
                            teamLogo:this.teamLogo,
                            points:this.points,
                            scorers:this.scorers.join('+'),
                            displacers:((this.displacers.length > 0) ? ' (' + this.displacers.join('+') + ')' : ''),
                            split:split,
                            avg:avg
                        });
                        place++;
                    }
                });
            })
        }
        else {
            prepareTemplateVideoParams();
        }
    };

    me.gradYearToGrade = function(row) {
        var eventDate = opts.startDate,
            eventDateArray = eventDate.split("-"),
            eventYear = eventDateArray[0],
            eventMonth = eventDateArray[1],
            difference,
            grade;
        if (opts.season != 'outdoor' && eventMonth > 6) {
            eventYear++;
        }
        difference = row.gradYear - eventYear;
        grade = 12 - difference;
        return (grade <= 12 && grade > 0) ? grade : '';
    };

    me.addEvent = function(event) {
        if (event) {
            // Generate event key and grade
            var eventKey = (
                event.gender+event.eventCode+event.roundName
            ).toLowerCase();
            // Generate event name
            event.fullEventName = event.eventName;
            if (event.genderName) {
                event.fullEventName = event.genderName+' '+event.fullEventName;
            }
            if (event.roundName) {
                eventKey += event.roundName;
                event.fullEventName += ' '+event.roundName;
            }
            if (event.ageGroupName) {
                eventKey += event.ageGroupName;
                event.fullEventName = event.ageGroupName+' '+event.fullEventName;
            }
            // Level is rankings metadata on each result row, not a race or XC scoring boundary.
            // Keep it out of the event bucket so the named division remains the scored field.
            // Group by the displayed division when one is available. MeetPro can assign different
            // internal division ids to rows that belong in the same named race (notably unmatched
            // athletes). Keep the id as the fallback for unnamed files so unrelated correction
            // files do not collapse into one table.
            var divisionIdentity = event.divisionName || event.divisionId;
            if (divisionIdentity) {
                eventKey += divisionIdentity;
            }
            if (event.divisionName) {
                event.fullEventName = event.divisionName+' '+event.fullEventName;
            }
            if (event.windReading === null) {
                event.windReading = '';
            }
            let eventType = getEventType(event.eventName);
            // Set/get event from hash
            let eventHash = eventData.get(
                eventKey, {
                    isTrack: me.isTrack,
                    name: event.eventName,
                    fullEventName: event.fullEventName,
                    type: eventType,
                    isRelay: eventType === 'T',
                    code: event.eventCode,
                    distance: event.eventDistance,
                    order: event.eventGenreOrder,
                    gender: event.genderName,
                    ageGroup: event.ageGroupName,
                    division: event.divisionName,
                    round: event.roundName,
                    key: eventKey,
                    target: (event.gender+event.eventCode).toLowerCase(),
                    hasWind: (me.isTrack && event.windReading.length > 0),
                    windReading: (me.isTrack && event.windReading ? 'wind' : ''),
                    resultsIndex: [],
                    results: function() {
                        return getResults.call(this);
                    },
                    teamScores: [],
                    videoIdsByHeat: new _DF_.HashTable()
                }
            );
            /*if (!event.videoId) {
                event.videoId = resultsData.length + 1;
            }*/
            // If we have a new videoId for this event
            if (event.videoId) {
                var heat = event.heat || 1;
                eventHash.videoIdsByHeat.get(
                    event.heat, {
                        id: event.videoId,
                        heat: heat
                    }
                );
            }
            // Now place the result
            var resultIndex = resultsData.length - 1;
            eventHash.resultsIndex.push(resultIndex);
            // set the results heat place
            me.rows(resultIndex).heatPlace = me.isTrack
                ? eventHash.resultsIndex.length
                : me.rows(resultIndex).place;
            // Return the event hash
            return eventHash;
        }
        return null;
    };

    me.events = function(id) {
        if (id === undefined) {
            return eventData.asc(['gender string', 'order numeric', 'distance numeric', 'name string', 'round string']).get();
        }
        return eventData.get(id);
    };

    me.addTeam = function(team) {
        if (team[fieldTeamId]) {
            return teamData.get(team[fieldTeamId], {
                isTrack: me.isTrack,
                name: team[fieldTeamName],
                id: team[fieldTeamId],
                resultsIndex: [],
                results: function() {
                    return getResults.call(this);
                }
            }).resultsIndex.push(resultsData.length - 1);
        }
        return null;
    };

    me.teams = function(id) {
        if (id === undefined) {
            return teamData.asc('Name').get();
        }
        return teamData.get(id);
    };

    me.addGrade = function(result) {
        if (result.grade > 0) {
            return gradeData.get(result.grade, {
                isTrack: me.isTrack,
                name: getGradeName(result.grade),
                id: result.grade,
                resultsIndex: [],
                results: function() {
                    return getResults.call(this);
                }
            }).resultsIndex.push(resultsData.length - 1);
        }
        return null;
    };

    me.grades = function(id) {
        if (id === undefined) {
            return gradeData.asc('ID numeric').get();
        }
        return gradeData.get(id);
    };

    me.addResult = function(result) {
        if (result) {
            // Generate 10 random 0 or 1
            result.trickery = [];
            for (var x = 0; x < 10; x++) {
                result.trickery.push(disableCopy ? Math.round(Math.random()) : 0);
            }
            result.grade = me.gradYearToGrade(result);
            resultsData.push(result);
            // Save event to hash and place this result
            me.addEvent(result);
            // Save team to hash
            me.addTeam(result);
            // Save result to grade hash
            me.addGrade(result);
            // Return the result
            return resultsData[resultsData.length - 1];
        }
        return null;
    };

    me.rows = function(i) {
        if (i != 'undefined' && i >= 0) {
            return resultsData[i];
        }
        return resultsData;
    };

    me.unitsToFeet = function(units){
        var realUnits = units / 1000,
            feet = parseInt(realUnits/12),
            inches = parseFloat(realUnits % 12).toFixed(2);
        if (inches < 10){
            inches = "0" + inches;
        }
        return feet + "-" + inches;
    };

    me.unitsToTime = function(units) {
        if (units > 0) {
            var sec_num = parseInt(units / 1000, 10),
                hours = Math.floor(sec_num / 3600),
                minutes = Math.floor((sec_num - (hours * 3600)) / 60),
                seconds = sec_num - (hours * 3600) - (minutes * 60),
                time = '';
            if (minutes < 10 && hours > 0) {
                minutes = "0" + minutes;
            }
            if (seconds < 10 && minutes > 0) {
                seconds = "0" + seconds;
            }
            if (hours > 0) {
                time += hours + ':';
            }
            time += minutes + ':' + seconds;
            return time;
        }
        return '--:--';
    };

    /**
     * Scoring method, builds scores and sorts them
     *
     * @param  eventKey The type of event (for example 5000m)
     * @return obj             The object of team scores
     */
    me.scoreXC = function(eventKey) {
        if (eventKey === undefined) {
            var arr = {};
            $.each(me.events(), function(){
                var scores = me.scoreXC(this.key);
                if (scores) {
                    arr[this.key] = scores;
                }
            });
            return arr;
        }
        else {
            var thisEvent = me.events(eventKey),
                scorers = 5,
                displacers = 7,
                nextPoints = 1,
                adjustmentPoints = 0,
                teamScores = new _DF_.HashTable();
            if (thisEvent) {
                // Score first time through
                $.each(thisEvent.resultsIndex, function(){
                    // Get team
                    var row = me.rows(this);
                    let teamScoreKey = row[fieldTeamId] + '_' + row[fieldTeamName];
                    let team = teamScores.get(teamScoreKey, {
                            name: row[fieldTeamName],
                            teamProfileUrl: row[fieldTeamUrl],
                            teamLogo: row[fieldTeamLogo],
                            id: teamScoreKey,
                            athleteCount: 0,
                            points: 0,
                            sixthRunner: 99999,
                            firstScorer: row.units,
                            lastScorer: 0,
                            scorerSum: 0,
                            scorers: [],
                            displacers: []
                        });
                    // Increment athletes
                    team.athleteCount++;
                    // Team not over scoring limit
                    if (team.athleteCount <= scorers) {
                        // Add to team score
                        row.points = nextPoints;
                    }
                    // If sixth runner
                    if (team.athleteCount == 6) {
                        team.sixthRunner = nextPoints;
                    }
                    // Scorer or displacer?
                    if (team.athleteCount <= displacers) {
                        row.points = nextPoints;
                        nextPoints++;
                    }
                    // No score
                    else {
                        row.points = 0;
                    }
                });
                // Score again, throwing people out
                $.each(thisEvent.resultsIndex, function(){
                    // Get team
                    var row = me.rows(this);

                    if (!row.statusCode) {
                        let teamScoreKey = row[fieldTeamId] + '_' + row[fieldTeamName];
                        let team = teamScores.get(teamScoreKey);

                        // Team did not have enough scorers AND the region doesn't do phantom runners?
                        if(team.athleteCount < scorers && !phantomRunnerRule) {
                            row.points = 0;
                            team.points = 0;
                            adjustmentPoints++;
                        }
                        // Team scores, but remove extra points from invalidated teams
                        else if (row.points > 0) {
                            row.points -= adjustmentPoints;
                            if (team.scorers.length < scorers) {
                                team.scorers.push(row.points);
                                team.points += parseInt(row.points);
                                team.scorerSum += parseInt(row.units);
                                team.lastScorer = row.units;
                            }
                            else {
                                team.displacers.push(row.points);
                            }
                        }
                        // If the region does phantom runners, add them in (with italics)
                        if (team.athleteCount < scorers && team.athleteCount == team.scorers.length && phantomRunnerRule) {
                            //console.log(team);
                            for(var x = team.athleteCount; x < scorers; x++) {
                                team.scorers.push("<i>"+nextPoints+"</i>");
                                team.athleteCount++;
                                team.points += nextPoints;
                            }
                            team.scorerSum = 0;
                        }
                    }

                });
                return teamScores;
            }
        }
    };
    // Chaining
    return me;
};

// set clipboard data with plain text (no tabs or newlines)
if (disableCopy) {
    document.addEventListener('copy', function(e) {
        e.preventDefault();
        // table html is converted to tabs and newlines
        var text = window.getSelection().toString();
        // remove all tabs and newlines with spaces
        while (text.match(/\t|[ ]{2,}/)) {
            text = text.replace(/\t|[ ]{2,}/, ' ');
        }
        // set clipboard data with plain text
        e.clipboardData.setData('text/plain', text);
    });
}

// Disable copy
/*$(document).on("keydown", function(e) {
    if (disableCopy && e.keyCode == 67 && (e.ctrlKey || e.metaKey)) {
        e.preventDefault();
        return false;
    }
});

// Disable context menu
window.oncontextmenu = function () {
    if (disableCopy) {
        return false;
    }
};*/
