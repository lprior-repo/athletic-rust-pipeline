var _kmq = _kmq || [];

$(document).ready(function() {

    // prevent autoplay videos from playing on locked content
    var paywall = $('.blurry');
    if (paywall.length) {
        var iFrames = $('#articleBody').find('iframe');
        iFrames.each(function(i, element) {
            var iFrame = $(element),
                src = iFrame.attr('src');

            // change param to prevent autoplay logic
            src = src.replace('autoplay', 'noautoplay');
            src = src.replace('autostart', 'noautoplay');
            iFrame.attr('src', src);
        });
    }
});