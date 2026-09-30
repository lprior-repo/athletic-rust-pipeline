var __values = (this && this.__values) || function(o) {
    var s = typeof Symbol === "function" && Symbol.iterator, m = s && o[s], i = 0;
    if (m) return m.call(o);
    if (o && typeof o.length === "number") return {
        next: function () {
            if (o && i >= o.length) o = void 0;
            return { value: o && o[i++], done: !o };
        }
    };
    throw new TypeError(s ? "Object is not iterable." : "Symbol.iterator is not defined.");
};
var Drivefaze;
(function (Drivefaze) {
    var Core = (function () {
        function Core() {
        }
        Core.init = function (settings) {
            if (typeof Core._settings == 'undefined') {
                Core._settings = settings;
            }
        };
        Core.log = function (str) {
            if (typeof console !== 'undefined' && typeof console.log !== 'undefined') {
                console.log(str);
            }
        };
        Core.set = function (key, value) {
            if (!Core._settings.hasOwnProperty(key)) {
                Core._settings[key] = value;
            }
            return Core;
        };
        Core.get = function (key) {
            var e_1, _a;
            var keys = key.split(':');
            if (keys.length > 1) {
                var value = Core._settings;
                try {
                    for (var keys_1 = __values(keys), keys_1_1 = keys_1.next(); !keys_1_1.done; keys_1_1 = keys_1.next()) {
                        var prop = keys_1_1.value;
                        if (typeof value === 'object' &&
                            value.hasOwnProperty(prop)) {
                            value = value[prop];
                        }
                        else {
                            return null;
                        }
                    }
                }
                catch (e_1_1) { e_1 = { error: e_1_1 }; }
                finally {
                    try {
                        if (keys_1_1 && !keys_1_1.done && (_a = keys_1["return"])) _a.call(keys_1);
                    }
                    finally { if (e_1) throw e_1.error; }
                }
                return value;
            }
            else if (Core._settings.hasOwnProperty(key)) {
                return Core._settings[key];
            }
            return null;
        };
        Core.getSiteSetting = function (key) {
            if (typeof Core._settings['site'] === 'object' &&
                Core._settings['site'].hasOwnProperty(key)) {
                return Core._settings['site'][key];
            }
            return null;
        };
        Core.getInt = function (val) {
            val = parseInt(val);
            return isNaN(val) ? 0 : val;
        };
        Core.getUrl = function (relativePath, params) {
            var query = params ? $.param(params) : '';
            if (relativePath && relativePath.substr(0, 1) != '/') {
                relativePath = '/' + window.location.pathname + '/' + relativePath;
            }
            else if (!relativePath) {
                relativePath = window.location.pathname;
            }
            relativePath.replace('//', '/');
            return window.location.protocol + '//' + window.location.hostname + relativePath + query;
        };
        Core.redirect = function (relativePath, params) {
            Drivefaze.API.showLoader('');
            window.location.href = Core.getUrl(relativePath, params);
        };
        Core.login = function (ref) {
            var accounts = Core._settings['accounts'];
            location.href = Core.getUrl(accounts['login']) + Core.getLoginRefString(ref);
        };
        Core.join = function (ref) {
            var accounts = Core._settings['accounts'];
            if (Core._settings['rootDomain'].indexOf('local') > -1) {
                Core.login(ref);
            }
            else {
                location.href = Core.getUrl(accounts['join']) + Core.getLoginRefString(ref);
            }
        };
        Core.signup = function (ref) {
            var accounts = Core._settings['accounts'];
            if (Core._settings['rootDomain'].indexOf('local') > -1) {
                Core.login(ref);
            }
            else {
                location.href = Core.getUrl(accounts['signup']) + Core.getLoginRefString(ref);
            }
        };
        Core.register = function (ref) {
            var accounts = Core._settings['accounts'];
            if (Core._settings['rootDomain'].indexOf('local') > -1) {
                Core.login(ref);
            }
            else {
                location.href = Core.getUrl(accounts['register']) + Core.getLoginRefString(ref);
            }
        };
        Core.getLoginRefString = function (ref) {
            return '?ref=' + ref + '&site=' + Core._settings['siteId'] + '&next=' + encodeURIComponent(Core.getUrl());
        };
        Core.getFloat = function (val, fixed) {
            if ((!isNaN(val)) && val % 1 !== 0) {
                val = parseFloat(val);
                fixed = !isNaN(fixed)
                    ? Core.getInt(fixed) : 2;
                val = val.toFixed(fixed).toString();
                while (val.substr(-1) == 0) {
                    val = val.substr(0, val.length - 1);
                }
            }
            else {
                return Core.getInt(val);
            }
            val = parseFloat(val);
            return isNaN(val) ? 0 : val;
        };
        Core.getBool = function (val) {
            val = (['false', 'undefined'].indexOf(val) > -1 || val == [] || val == {})
                ? false : val;
            return Boolean(val);
        };
        Core.isProduction = function () {
            return -1 !== Core.get('rootDomain').indexOf('.com');
        };
        return Core;
    }());
    Drivefaze.Core = Core;
    var Ads = (function () {
        function Ads() {
        }
        Ads.stickyMoney = function (allowClose) {
            if (allowClose === void 0) { allowClose = false; }
            var $body = $('body');
            var $stickyAd = $('.advertisement-adhesion');
            if (0 === $stickyAd.length) {
                var adhesionAdData = _DF_.Ads.getAdhesionDataFromUrl(window);
                $stickyAd = $('' +
                    '<div class="advertisement-adhesion">' +
                    '    <!-- ' + adhesionAdData.path + ' -->\n' +
                    '    <div id="' + adhesionAdData.id + '" style="width: 320px; height: 50px;"></div>\n' +
                    '</div>');
                $stickyAd.appendTo($body);
            }
            $body.toggleClass('sticky-ad');
        };
        Ads.getAdhesionDataFromUrl = function (window) {
            if ('/' === window.location.pathname) {
                return Ads.adhesionAds['front'];
            }
            var pathArray = window.location.pathname.trim().split('/');
            var route = pathArray[1].trim();
            return Ads.adhesionAds[route]
                ? Ads.adhesionAds[route]
                : Ads.adhesionAds['default'];
        };
        Ads.adhesionAds = {
            front: {
                path: '/43625987/MileSplit/Homepage/Mobile/Adhesion',
                id: 'div-gpt-ad-1579200567210-0'
            },
            articles: {
                path: '/43625987/MileSplit/Articles/Mobile/Adhesion',
                id: 'div-gpt-ad-1579201049560-0'
            },
            athletes: {
                path: '/43625987/MileSplit/Athletes/Mobile/Adhesion',
                id: 'div-gpt-ad-1585686207452-0'
            },
            calendar: {
                path: '/43625987/MileSplit/Calendar/Mobile/Adhesion',
                id: 'div-gpt-ad-1579201334181-0'
            },
            legacy: {
                path: '/43625987/MileSplit/Legacy/Mobile/Adhesion',
                id: 'div-gpt-ad-1579201481449-0'
            },
            meets: {
                path: '/43625987/MileSplit/meets/mobile/Adhesion',
                id: 'div-gpt-ad-1579201572388-0'
            },
            photos: {
                path: '/43625987/MileSplit/Photos/mobile/Adhesion',
                id: 'div-gpt-ad-1579201660037-0'
            },
            rankings: {
                path: '/43625987/MileSplit/Rankings/Mobile/Adhesion',
                id: 'div-gpt-ad-1579201885587-0'
            },
            results: {
                path: '/43625987/MileSplit/Results/Adhesion',
                id: 'div-gpt-ad-1579202072328-0'
            },
            teams: {
                path: '/43625987/MileSplit/Teams/Mobile/Adhesion',
                id: 'div-gpt-ad-1579202155582-0'
            },
            "default": {
                path: '/43625987/MileSplit/undefined/Adhesion',
                id: 'div-gpt-ad-1579202251595-0'
            },
            videos: {
                path: '/43625987/MileSplit/Videos/Mobile/Adhesion',
                id: 'div-gpt-ad-1579202329119-0'
            }
        };
        return Ads;
    }());
    Drivefaze.Ads = Ads;
    var Helper = (function () {
        function Helper() {
        }
        Helper.levelToText = function (level) {
            switch (level.toLowerCase()) {
                case 'c':
                case 'college':
                    return 'College';
                case 'h':
                case 'hs':
                    return 'High School';
                case 'm':
                case 'ms':
                    return 'Middle School';
                case 'o':
                case 'open':
                    return 'Open';
                case 'y':
                case 'youth':
                    return 'Youth';
                default:
                    return level;
            }
        };
        Helper.getOrdinal = function (number) {
            var ends = ['th', 'st', 'nd', 'rd', 'th', 'th', 'th', 'th', 'th', 'th'];
            var remainder = number % 100;
            return (remainder >= 11) && (remainder <= 13) ? number + ends[0] : number + ends[number % 10];
        };
        Helper.obfuscateNumber = function (rank) {
            return rank.replace(/\d/g, 'X');
        };
        Helper.getRound = function (round) {
            if (round === void 0) { round = ''; }
            var rounds = {
                'p': 'Prelims',
                'q': 'Quarters',
                's': 'Semis',
                'f': 'Finals'
            };
            return rounds[round.toLowerCase()]
                ? rounds[round.toLowerCase()]
                : 'Finals';
        };
        return Helper;
    }());
    Drivefaze.Helper = Helper;
    var ObjectHelper = (function () {
        function ObjectHelper() {
        }
        ObjectHelper.hydrate = function (data, instance, keyFunction, hydrateExistingProperties) {
            hydrateExistingProperties = typeof hydrateExistingProperties == 'undefined'
                ? true : hydrateExistingProperties;
            if (typeof data == 'object') {
                for (var property in data) {
                    var newProperty = property;
                    if (typeof keyFunction == 'function') {
                        newProperty = keyFunction(property);
                    }
                    if (hydrateExistingProperties &&
                        (newProperty === false || !instance.hasOwnProperty(newProperty))) {
                        continue;
                    }
                    if (typeof data[property] == 'object' && !Array.isArray(data[property])) {
                        ObjectHelper.hydrate(data[property], instance[newProperty], keyFunction);
                    }
                    else if (newProperty !== false) {
                        instance[newProperty] = data[property];
                    }
                }
            }
            return instance;
        };
        ObjectHelper.hydrateDefaults = function (instance, defaults) {
            return ObjectHelper.hydrate(defaults, instance, null, false);
        };
        return ObjectHelper;
    }());
    Drivefaze.ObjectHelper = ObjectHelper;
    var SportHelper = (function () {
        function SportHelper() {
        }
        SportHelper.isFieldEventType = function (eventType) {
            return eventType.toLowerCase() == 'f';
        };
        SportHelper.isFieldOrMultiEventType = function (eventType) {
            return SportHelper.isFieldEventType(eventType) ||
                SportHelper.isMultiEventType(eventType);
        };
        SportHelper.isMultiEventType = function (eventType) {
            return eventType.toLowerCase() == 'm';
        };
        SportHelper.isRelayEventType = function (eventType) {
            return eventType.toLowerCase() == 't';
        };
        SportHelper.isRunningEventType = function (eventType) {
            return ['t', 'r'].indexOf(eventType.toLowerCase()) > -1;
        };
        SportHelper.isTrackSeason = function (season) {
            return ['indoor', 'outdoor'].indexOf(season.toLowerCase()) > -1;
        };
        SportHelper.isXCSeason = function (season) {
            return season.toLowerCase() == 'cc';
        };
        SportHelper.toMillimeters = function (inches) {
            inches = parseFloat(inches);
            if (!isNaN(inches)) {
                return inches * 25.4;
            }
            return null;
        };
        SportHelper.toMark = function (units, eventType) {
            if (!isNaN(units)) {
                units /= 1000;
                if (!eventType) {
                    eventType = '';
                }
                var mark = '';
                if (eventType && SportHelper.isFieldEventType(eventType)) {
                    if (Core.getSiteSetting('fieldUnits') === 'metric') {
                        var mm = this.toMillimeters(units);
                        if (mm > 0) {
                            var m = (mm / 1000).toFixed(2);
                            mark = m.toString() + 'm';
                        }
                    }
                    else {
                        var feet = Math.floor(units / 12), inches = units - (feet * 12);
                        inches = Core.getFloat(inches, 2);
                        mark += feet + '-' + inches;
                    }
                }
                else {
                    var h = Math.floor(units / 3600), m = Math.floor(units / 60) - Math.floor(h * 60), s = units - (m * 60) - (h * 3600);
                    if (h > 0) {
                        mark += h + ':';
                    }
                    if (m > 0) {
                        if (h > 0 && m < 10) {
                            mark += '0';
                        }
                        mark += m + ':';
                    }
                    else if (mark.length > 0) {
                        mark += '00:';
                    }
                    if (s == 0) {
                        mark += '0';
                    }
                    else if (m > 0 && s < 10) {
                        mark += '0';
                    }
                    mark += Core.getFloat(s, 2).toFixed(2).toString();
                }
                return mark;
            }
            return units;
        };
        SportHelper.toUnits = function (mark, eventType) {
            var isMetric = this.isMetricMark(mark);
            mark = mark.toString().replace(/[']/g, '-').replace(/[^-.:0-9]/g, '');
            var units = 0;
            if (mark.indexOf('-') > 0 || eventType && SportHelper.isFieldEventType(eventType)) {
                var arr = mark.split('-');
                if (arr.length == 2) {
                    units = arr[0] * 12000 + arr[1] * 1000;
                }
                else if (!isNaN(mark)) {
                    if (isMetric) {
                        return Math.round((mark * 39.3701) * 1000);
                    }
                    else {
                        return mark * 1000;
                    }
                }
            }
            else if (mark.indexOf(':') > -1 || eventType && SportHelper.isRunningEventType(eventType)) {
                var arr = mark.split(':'), hours = 0, minutes = 0, seconds = 0;
                if (arr.length == 1) {
                    seconds = Core.getFloat(arr[0], 2);
                }
                else if (arr.length == 2) {
                    minutes = Core.getInt(arr[0]);
                    seconds = Core.getFloat(arr[1], 2);
                }
                else if (arr.length > 2) {
                    hours = Core.getInt(arr[0]);
                    minutes = Core.getInt(arr[1]);
                    seconds = Core.getFloat(arr[2], 2);
                }
                units = hours * 3600000 + minutes * 60000 + seconds * 1000;
            }
            else if (!isNaN(mark)) {
                return mark * 1000;
            }
            return units;
        };
        SportHelper.isMetricMark = function (mark) {
            return mark.toString().toLowerCase().substring(mark.length - 1) == 'm';
        };
        return SportHelper;
    }());
    Drivefaze.SportHelper = SportHelper;
    var TemplateHelper = (function () {
        function TemplateHelper() {
        }
        TemplateHelper.initInputMaxLength = function () {
            this.maxlength_elements = document.getElementsByClassName('maxlength-counter');
            var _loop_1 = function (i) {
                var element = this_1.maxlength_elements[i], max_length = element.getAttribute('maxlength');
                var valid_element = (null !== max_length
                    && (('input' === element.tagName.toLowerCase() &&
                        'text' === element.getAttribute('type')) || 'textarea' === element.tagName.toLowerCase()));
                if (valid_element) {
                    var counter_1 = $('<div>', { "class": 'input-counter', text: '/' + max_length }), counter_current_1 = $('<span>', { "class": "counter-current", text: element.value.length });
                    $(element).on('focus', function () {
                        $(this).after(counter_1.prepend(counter_current_1).hide());
                    }).on('keyup change', function () {
                        var current_length = $(this).val().length;
                        if (!counter_1.is(':visible')) {
                            counter_1.show();
                        }
                        if (current_length >= (Number(max_length) - 10)) {
                            counter_1.css('color', 'red');
                        }
                        else {
                            counter_1.css('color', '');
                        }
                        counter_1.find('.counter-current').text(current_length);
                    }).on('blur', function () {
                        counter_1.remove();
                    });
                }
                else {
                    if (null === max_length) {
                        console.warn('Element with class .maxlength-counter missing maxlength attribute');
                    }
                    if ('input' !== element.tagName.toLowerCase() && 'textarea' !== element.tagName.toLowerCase()) {
                        console.warn('Cannot use maxlength method on ' + element.tagName.toLowerCase() + ' element');
                    }
                    if ('text' !== element.getAttribute('type') && 'input' === element.tagName.toLowerCase()) {
                        console.warn('Cannot use maxlength method on input elements not of type "text"');
                    }
                }
            };
            var this_1 = this;
            for (var i = 0; i < this.maxlength_elements.length; i++) {
                _loop_1(i);
            }
        };
        TemplateHelper.getTmplText = function (obj) {
            var str = $(this[0]).html().trim();
            return TemplateHelper.replaceObj(str, obj);
        };
        ;
        TemplateHelper.replaceVal = function (str, findVal, replaceVal) {
            return str.replace(new RegExp('[$]{' + findVal + '}', 'g'), replaceVal);
        };
        ;
        TemplateHelper.replaceObj = function (str, obj) {
            if (typeof obj == 'object') {
                var _loop_2 = function (key) {
                    if (!obj.hasOwnProperty(key)) {
                        return "continue";
                    }
                    if (Array.isArray(obj[key])) {
                        var loopExp = new RegExp('#{' + key + '}((?!/{)(.|\n)*)/{' + key + '}', 'gm'), matches_1 = loopExp.exec(str);
                        if (matches_1 && matches_1[1]) {
                            var loopStr_1 = '';
                            obj[key].forEach(function (val) {
                                if (typeof val == 'object') {
                                    loopStr_1 += TemplateHelper.replaceObj(matches_1[1], val);
                                }
                                else {
                                    loopStr_1 += TemplateHelper.replaceVal(matches_1[1], '[.]', val);
                                }
                            });
                            str = str.replace(loopExp, loopStr_1);
                        }
                    }
                    else {
                        str = TemplateHelper.replaceVal(str, key, obj[key]);
                    }
                };
                for (var key in obj) {
                    _loop_2(key);
                }
            }
            return str;
        };
        ;
        return TemplateHelper;
    }());
    Drivefaze.TemplateHelper = TemplateHelper;
    var FileHelper = (function () {
        function FileHelper() {
        }
        FileHelper.isFile = function (input) {
            return 'File' in window && input instanceof File;
        };
        FileHelper.isImage = function (input) {
            var allowed_image_types = ['image/jpeg', 'image/png'];
            return _DF_.FileHelper.isFile(input) && -1 !== allowed_image_types.indexOf(input.type);
        };
        return FileHelper;
    }());
    Drivefaze.FileHelper = FileHelper;
    var ImageHelper = (function () {
        function ImageHelper() {
        }
        ImageHelper.generateImageFromFile = function (file) {
            return new Promise(function (resolve, reject) {
                var img = new Image();
                var url = window.URL || window.webkitURL;
                img.addEventListener('load', function (e) { return resolve(img); });
                img.addEventListener('error', function (event) {
                    reject('Not a valid image. Please select another file.');
                });
                if (!_DF_.FileHelper.isFile(file) || !_DF_.FileHelper.isImage(file)) {
                    var event_1 = new Event('error');
                    img.dispatchEvent(event_1);
                }
                img.src = url.createObjectURL(file);
            });
        };
        ;
        ImageHelper.isImageHtmlElement = function (html) {
            return 'object' == typeof html && 'IMG' === html.nodeName;
        };
        ImageHelper.getHeight = function (image) {
            return _DF_.ImageHelper.isImageHtmlElement(image)
                ? image.naturalHeight || image.height
                : undefined;
        };
        ImageHelper.getWidth = function (image) {
            return _DF_.ImageHelper.isImageHtmlElement(image)
                ? image.naturalWidth || image.width
                : undefined;
        };
        ImageHelper.isValidMinHeight = function (image, min) {
            var height = image.naturalHeight || image.height;
            return _DF_.ImageHelper.isImageHtmlElement(image) && min <= height;
        };
        ImageHelper.isValidMaxHeight = function (image, max) {
            var height = image.naturalHeight || image.height;
            return _DF_.ImageHelper.isImageHtmlElement(image) && max >= height;
        };
        ImageHelper.isValidHeightRange = function (image, min, max) {
            return _DF_.ImageHelper.isValidMinHeight(image, min) && _DF_.ImageHelper.isValidMaxHeight(image, max);
        };
        ImageHelper.isValidMinWidth = function (image, min) {
            var width = image.naturalWidth || image.width;
            return _DF_.ImageHelper.isImageHtmlElement(image) && min <= width;
        };
        ImageHelper.isValidMaxWidth = function (image, max) {
            var width = image.naturalWidth || image.width;
            return _DF_.ImageHelper.isImageHtmlElement(image) && max >= width;
        };
        ImageHelper.isValidWidthRange = function (image, min, max) {
            return _DF_.ImageHelper.isValidMinWidth(image, min) && _DF_.ImageHelper.isValidMaxWidth(image, max);
        };
        ImageHelper.isBetweenRatios = function (image, ratio1, ratio2) {
            if (_DF_.ImageHelper.isImageHtmlElement(image)) {
                var width = _DF_.ImageHelper.getWidth(image), height = _DF_.ImageHelper.getHeight(image);
                var calculated_ratio = (width > height)
                    ? width / height
                    : height / width;
                return ratio1 <= calculated_ratio && calculated_ratio <= ratio2;
            }
            return undefined;
        };
        return ImageHelper;
    }());
    Drivefaze.ImageHelper = ImageHelper;
    var RegexHelper = (function () {
        function RegexHelper() {
        }
        RegexHelper.EmailFormat = /^(([^<>()\[\]\\.,;:\s@"]+(\.[^<>()\[\]\\.,;:\s@"]+)*)|(".+"))@((\[[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}])|(([a-zA-Z\-0-9]+\.)+[a-zA-Z]{2,}))$/;
        RegexHelper.EventCodeRelay = /^SMR|DMR|[0-9]{1}[x]{1}[0-9]+[a-z]+/;
        RegexHelper.ArticleSummary = /^[a-zA-Z0-9\s.;,()!#\-_'"]+$/m;
        return RegexHelper;
    }());
    Drivefaze.RegexHelper = RegexHelper;
})(Drivefaze || (Drivefaze = {}));
var _DF_ = Drivefaze;
$.extend(_DF_, Drivefaze.Core);
$.fn.tmpl = function (d) {
    return $(_DF_.TemplateHelper.getTmplText.call(this, d));
};
$.fn.tmplText = function (d) {
    return _DF_.TemplateHelper.getTmplText.call(this, d);
};
$.fn.outerHTML = function () {
    var $t = $(this);
    if ('outerHTML' in $t[0]) {
        return $t[0].outerHTML;
    }
    else {
        var content = $t.wrap('<div></div>').parent().html();
        $t.unwrap();
        return content;
    }
};
//# sourceMappingURL=core.js.map