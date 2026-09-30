var Drivefaze;
(function (Drivefaze) {
    var API = (function () {
        function API() {
        }
        API.addSettings = function (settings) {
            this.settings = settings;
            return this;
        };
        API.mergeSettings = function (settings) {
            if (undefined !== this.settings) {
                $.extend(settings, this.settings);
                this.settings = undefined;
            }
            return settings;
        };
        API.useApiV3 = function (flag) {
            this.apiV3 = flag;
            return this;
        };
        API.buildQueryString = function (params, jsonp) {
            var queryString = $.param(params || {});
            if (jsonp) {
                queryString += '&callback=?';
            }
            if (queryString.indexOf('&') == 0) {
                queryString = queryString.substr(1);
            }
            return queryString;
        };
        API.buildURL = function (method, params, type, jsonp) {
            type = type || 'GET';
            var url = this.endPoint + method;
            var logUrl = window.location.protocol + '//' + window.location.hostname + url;
            if (this.apiV3) {
                url = this.apiV3Staging + method;
                if (window.location.hostname.indexOf('milesplit.com') > -1) {
                    url = this.apiV3Production + method;
                }
                logUrl = url;
            }
            var queryString = this.buildQueryString(params, jsonp);
            if (queryString.length > 0) {
                var appendSymbol = method.indexOf('?') == -1 ? '?' : '&';
                url += appendSymbol + queryString;
            }
            Drivefaze.Core.log('api ' + type + ': ' + logUrl);
            return url;
        };
        API.getLoader = function () {
            if (typeof this.loader == 'undefined') {
                this.loader = $('#loader');
            }
            return this.loader;
        };
        API.setLoaderDefaultText = function (text) {
            this.loaderDefaultText = text;
            return this;
        };
        API.showLoader = function (text, delay, keepOpen) {
            if (!(this.loaderId && !this.canCloseLoader)) {
                if (this.loaderTimeOut) {
                    clearTimeout(this.loaderTimeOut);
                }
                text = text || this.loaderDefaultText;
                this.getLoader().show().find('span.loaderText').text(text);
                if (typeof delay != 'undefined') {
                    this.loaderDelay = delay;
                }
                this.canCloseLoader = !keepOpen;
                this.loaderId = new Date().getTime();
            }
            return this;
        };
        API.hideLoader = function (force) {
            var me = this;
            if (me.loaderTimeOut) {
                clearTimeout(me.loaderTimeOut);
            }
            me.loaderTimeOut = setTimeout(function () {
                me.getLoader().hide().find('span.loaderText').text(me.loaderDefaultText);
                me.loaderId = null;
                me.loaderTimeOut = null;
            }, force ? 0 : me.loaderDelay);
            return me;
        };
        API.get = function (method, filters) {
            return this.ajax(method, 'GET', {}, filters);
        };
        API.patch = function (method, data, filters) {
            return this.ajax(method, 'PATCH', data, filters);
        };
        API.put = function (method, data) {
            return this.ajax(method, 'PUT', data);
        };
        API.post = function (method, data) {
            return this.ajax(method, 'POST', data);
        };
        API["delete"] = function (method, filters) {
            return this.ajax(method, 'DELETE', {}, filters);
        };
        API.responseHasData = function (obj) {
            return typeof obj === 'object' && obj.hasOwnProperty('data');
        };
        API.responseHasDataArray = function (obj) {
            return this.responseHasData(obj) && Array.isArray(obj['data']);
        };
        API.responseHasDataObject = function (obj) {
            return this.responseHasData(obj) && typeof obj['data'] === 'object';
        };
        API.ajax = function (method, type, data, filters) {
            var me = this, loaderId = me.loaderId, defer = $.Deferred();
            data = data || {};
            data.m = type;
            var settings = me.mergeSettings({
                url: this.buildURL(method, filters, type),
                type: type.toUpperCase(),
                headers: {
                    appName: Drivefaze.Core.get('appName'),
                    appToken: Drivefaze.Core.get('appHash'),
                    userId: Drivefaze.Core.get('userID'),
                    userToken: Drivefaze.Core.get('userToken')
                },
                data: data,
                success: function (json, textStatus, jqXHR) {
                    if (json.error) {
                        console.log(json.lastErrorText);
                        defer.reject(json, textStatus, jqXHR);
                    }
                    else {
                        defer.resolve(json, textStatus, jqXHR);
                    }
                },
                error: function (json, textStatus, errorThrown) {
                    var _a;
                    var apiError = json.responseJSON ? json.responseJSON.lastErrorText : errorThrown;
                    if (me.apiV3) {
                        apiError = ((_a = json.responseJSON) === null || _a === void 0 ? void 0 : _a.message) || errorThrown;
                    }
                    var error = {
                        code: json.status,
                        message: apiError,
                    };
                    defer.reject({
                        error: error,
                        errors: [error],
                        lastErrorText: apiError
                    }, textStatus);
                    $.error(apiError);
                }
            });
            if (!Drivefaze.Core.isProduction()) {
                console.log('api settings: ', settings);
            }
            $.ajax(settings).always(function () {
                if (loaderId === me.loaderId && me.canCloseLoader) {
                    me.hideLoader();
                }
            });
            return defer.promise();
        };
        API.loaderDelay = 200;
        API.loaderDefaultText = 'Loading';
        API.canCloseLoader = true;
        API.endPoint = '/api/';
        API.userAgent = 'MileSplit AJAX Client';
        API.apiV3 = false;
        API.apiV3Staging = 'https://api.stag.milesplit.com/int/v3/';
        API.apiV3Production = 'https://api.prod.milesplit.com/int/v3/';
        return API;
    }());
    Drivefaze.API = API;
})(Drivefaze || (Drivefaze = {}));
//# sourceMappingURL=api.js.map