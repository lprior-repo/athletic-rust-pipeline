"use strict";

/*
    Upload handlers for dsImagePicker controls that store their image as a FILE rather than as
    base64 in the database. Keyed by control id; a page registers one per picker:

        dsImagePickerUpload['XAthlete1Photo'] = function (id, dataURL) { ... };

    It has to hook in HERE rather than on the file input's change event because the bytes that get
    stored are not the bytes the user picked - dropChangeHandler runs the file through load-image
    first (resized to 800px, EXIF orientation applied, re-encoded as PNG) and that conversion is
    asynchronous. A handler on 'change' would read the previous image, or nothing at all.

    Pages with no handler registered are untouched: they keep posting the data URL back in the
    hidden field and saving it to the database, which is what every other picker still does.
*/
var dsImagePickerUpload = {};

function updateResults(img, id) {
    var content
    var pixelRatio = window.devicePixelRatio || 1

    var dataURL = img.toDataURL();

    $('#' + id + '_Image').attr('src', dataURL);
    $('#' + id + '_Hidden').val(dataURL);
    //$('#TEST_Image').Jcrop();

    if (dsImagePickerUpload[id]) dsImagePickerUpload[id](id, dataURL);
}


function dropChangeHandler(e, id) {
    e.preventDefault()
    e = e.originalEvent
    var target = e.dataTransfer || e.target
    var file = target && target.files && target.files[0]

    var options = {
        //maxWidth: $('#' + id + '_Image').width(),
        maxWidth: 800,
        canvas: true,
        //pixelRatio: window.devicePixelRatio,
        //downsamplingRatio: 1,
        orientation: true
    }

    loadImage(file, function (img) {updateResults(img, id);}, options);
}

function dropFileChangeHandler(e, id) {
    e.preventDefault()
    e = e.originalEvent
    var target = e.dataTransfer || e.target
    var file = target && target.files && target.files[0]
}

function formatPhoneText(target, key) {

    if (['End', 'Home', 'ArrowLeft', 'ArrowRight'].includes(key)) return;

    var value = target.value.replaceAll("-", "").replaceAll("(", "").replaceAll(")", "").replaceAll(" ", "").trim();

    if (value.length == 3)
        value = value + "-";
    else if (value.length > 3 && value.length < 6)
        value = value.slice(0, 3) + "-" + value.slice(3);
    else if (value.length == 6)
        value = value.slice(0, 3) + "-" + value.slice(3, 6) + "-";
    else if (value.length > 6)
        value = value.slice(0, 3) + "-" + value.slice(3, 6) + "-" + value.slice(6);

    target.value = value;
    setCaretPosition(target, value.length);
}

function setCaretPosition(ctrl, pos) {
    // Modern browsers
    if (ctrl.setSelectionRange) {
        ctrl.focus();
        ctrl.setSelectionRange(pos, pos);

    // IE8 and below
    } else if (ctrl.createTextRange) {
        var range = ctrl.createTextRange();
        range.collapse(true);
        range.moveEnd('character', pos);
        range.moveStart('character', pos);
        range.select();
    }
}









/*
function cropCickHandler(e) {
    var coordinates="????";
    event.preventDefault()
    var img = result.find('img, canvas')[0]
    var pixelRatio = window.devicePixelRatio || 1
    if (img && coordinates) {
        updateResults(loadImage.scale(img, {
            left: coordinates.x * pixelRatio,
            top: coordinates.y * pixelRatio,
            sourceWidth: coordinates.w * pixelRatio,
            sourceHeight: coordinates.h * pixelRatio,
            minWidth: result.width(),
            maxWidth: result.width(),
            pixelRatio: pixelRatio,
            downsamplingRatio: 0.5
        }))
        coordinates = null
    }
}

function ShowPreview(coords) {
    console.log(coords);

    var rx = 100 / coords.w;
    var ry = 100 / coords.h;

    //$('#" + _Image.ID + @"Hidden').css({
    //    width: Math.round(rx * 500) + 'px',
    //    height: Math.round(ry * 370) + 'px',
    //    marginLeft: '-' + Math.round(rx * coords.x) + 'px',
    //    marginTop: '-' + Math.round(ry * coords.y) + 'px'
    //});

    var img = new Image;
    img.src = $('#TEST_Image').attr('src');

    //alert(EXIF.getTag(img,"Orientation"));

    var canvas = $("#Test_Canvas")[0];
    var ctx = canvas.getContext("2d");
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    ctx.drawImage(img, coords.x, coords.y, coords.w, coords.h, 0, 0, coords.w, coords.h);
    //  var url = canvas.toDataURL();
    //   canvas.width = Math.round(rx * 500) + 'px';
    // canvas.height = Math.round(ry * 370) + 'px';
    // $('#" + _HiddenImage.ID + @"Hidden').attr(""src"", url);
};

*/