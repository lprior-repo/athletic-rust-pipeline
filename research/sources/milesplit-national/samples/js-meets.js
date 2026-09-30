
$(function(){

    // Grab some elements that we'll need later
    var $meetHistory = $('.meetHistory');

    // Change results from raw to formatted
    $('#ddResultsView').on('change', function() {
        document.location.href = $(this).val();
    });

    // Change which page of results we are on
    $('#ddResultsPage').on('change', function() {
        document.location.href = $(this).val();
    });

    $('#ddMeetStatus').on('change', function() {
        document.location.href = $(this).val();
    });

    $('#ddEntriesList').on('change', function() {
        document.location.href = $(this).val();
    });

    // Any submittable form
    $('#content header select[name]').on('change', function() {
        if (!$(this).data('preventSubmit')) {
            this.form.submit();
        }
    });

    $meetHistory.find('a').on('click', function(e) {
        e.stopPropagation();
    });

    $meetHistory.find('ul.meets li.back a').on('click', function(e) {
        e.preventDefault();
        $meetHistory.find('.decades, .recordCategories').show();
        $meetHistory.find('.meets li').hide();
    });

    $meetHistory.find('.records li.back a').on('click', function(e) {
        e.preventDefault();
        $meetHistory.find('.records').hide();
        $meetHistory.find('.recordCategories, .pastMeets').show();
    });

    $meetHistory.find('.decades a').on('click', function(e) {
        e.preventDefault();
        var decade = $(this).attr('href').substr(1),
            $meetLinks = $meetHistory.find('.meets li').hide();
        $meetLinks.filter('[data-decade="' + decade + '"]').show();
        $meetLinks.filter('.back').show();
        $meetHistory.find('.decades, .records, .recordCategories').hide();
        $meetHistory.addClass('expand')
        $('body').one('click', function() {
            $meetHistory.removeClass('expand');
        });
    });

    $meetHistory.find('.recordCategories a').on('click', function(e) {
        e.preventDefault();
        var category = $(this).attr('href').substr(1),
            $records = $meetHistory.find('.records').hide();
        $records.filter('.' + category).show();
        $meetHistory.find('.recordCategories, .pastMeets').hide();
        $meetHistory.addClass('expand')
        $('body').one('click', function() {
            $meetHistory.removeClass('expand');
        });
    });


});
