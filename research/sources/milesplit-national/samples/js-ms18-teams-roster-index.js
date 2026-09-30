(function ($) {
    $(document).ready(function () {
        var $rosterAthletes = $('li.athlete-row');
        var $rosterDataset = $('#rosterDataset');
        var $rosterFilterType = $('#rosterFilterType');
        var $rosterFilterGender = $('#rosterFilterGender');
        var $rosterFilterClass = $('#rosterFilterClass');
        function filter() {
            $rosterAthletes.each(function (rowIndex, el) {
                var show = true;
                if ($rosterFilterGender.val()) {
                    show = show ? $(el).find('.column-gender').text().toUpperCase() === $rosterFilterGender.val() : show;
                }
                if ($rosterFilterClass.val()) {
                    show = show ? $(el).find('.column-grad-year').text() === $rosterFilterClass.val() : show;
                }
                if ($rosterFilterType.val()) {
                    show = show ? $(el).find('[data-season-id="' + $rosterFilterType.val() + '"]').length > 0 : show;
                }
                if (show) {
                    $(el).show();
                }
                else {
                    $(el).hide();
                }
            });
        }
        $rosterFilterType.change(filter);
        $rosterFilterClass.change(filter);
        $rosterFilterGender.change(filter);
        filter();
    });
})($);
//# sourceMappingURL=index.js.map