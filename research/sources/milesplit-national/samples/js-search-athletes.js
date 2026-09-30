(function ($) {
    $(document).ready(function () {
        //search athletes name
        $('body').on('click keyup', '.athlete-search', function (e) {
            if (e.keyCode === 13 || e.target.id === 'athleteSearch') {
                var q = $('input[name=query]').val(),
                    isState = $('input[name=isState]').val();
                var data = {
                    searchToken: $('input[name=searchToken]').val(),
                    q: q,
                    perPage: 25
                }
                if (isState) {
                    data.filters = {
                        subdomain: $('input[name=subdomain]').val()
                    }
                }
                if (q) {
                    _DF_.API.showLoader('Searching for ' + q);
                    $.ajax({
                        method: "POST",
                        url: "/search/v2/athletes",
                        data: data,
                        success: function (results) {
                            _DF_.API.hideLoader();
                            results.isState = isState;
                            results.stateFullName = $('input[name=stateFullName]').val()
                            results.nationalDomain = $('input[name=nationalDomain]').val();
                            results.nextPage = results.page + 1;
                            if (results.totalPages > 1) {
                                results.seeMore = 1;
                            }
                            $("main").html(Handlebars.templates['athletes'](results));
                        },
                        error: function (response) {
                            _DF_.API.hideLoader();
                            alert('There was an error searching profile: ' + response.error.message);
                        }
                    });
                }
                else {
                    e.preventDefault();
                    alert('Please enter name.');
                }
            }
        });
        //pagination - see more button
        $('body').on('click',  '#btnShowMoreResults', function(e) {
            e.preventDefault();
            $(this).text('Loading');
            var resultList = $('#resultsList'),
                nextPage = $(this).data('page'),
                q = $('input[name=query]').val(),
                isState = $('input[name=isState]').val(),
                showMore = $('div.show-more');
            var data = {
                searchToken: $('input[name=searchToken]').val(),
                q: q,
                perPage: 25,
                page: nextPage
            }
            if (isState) {
                data.filters = {
                    subdomain: $('input[name=subdomain]').val()
                }
            }
            $.ajax({
                method: "POST",
                url: "/search/v2/athletes",
                data: data,
                success: function (results) {
                    if (results.hits.length > 0) {
                        results.hits.forEach(function (hit) {
                            var resultDescription = hit.data.fields.url !== ''
                                ? '<a class="result-description" href="' + hit.data.fields.url + '">' + hit.data.fields.description + '</a>'
                                : '<span class="result-description">' + hit.data.fields.description + '</span>';
                            var item = $('' +
                                '<li class="result d-flex justify-content-between">' +
                                '   <div class="result-info">' +
                                '       <h3 class="result-title">' + hit.data.fields.title + '</h3>' + resultDescription +
                                '   </div>' +
                                '   <div class="result-type bodylight">Athlete</div>' +
                                '</li>'
                            );
                            resultList.append(item);
                        });
                        if(results.page === results.totalPages ) {
                            $('#btnShowMoreResults').hide();
                        }
                        else {
                            showMore.html('<button id="btnShowMoreResults" data-page="'+(++nextPage)+'" class="btn btn-primary btn-l" type="button">See More</button>')
                        }
                    }
                    else {
                        $('#btnShowMoreResults').hide();
                    }
                },
                error: function (response) {
                    alert('There was an error fetching more search results: ' + response.error.message);
                    $(this).text('See more');
                }
            });
        });
    });
})(jQuery);
