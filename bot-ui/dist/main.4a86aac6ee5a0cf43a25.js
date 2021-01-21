/******/ (() => { // webpackBootstrap
/******/ 	var __webpack_modules__ = ({

/***/ 65046:
/***/ ((__unused_webpack_module, __unused_webpack___webpack_exports__, __webpack_require__) => {

"use strict";

// EXTERNAL MODULE: ./node_modules/core-js/modules/es.array.iterator.js
var es_array_iterator = __webpack_require__(66992);
// EXTERNAL MODULE: ./node_modules/core-js/modules/es.array.map.js
var es_array_map = __webpack_require__(21249);
// EXTERNAL MODULE: ./node_modules/core-js/modules/es.function.name.js
var es_function_name = __webpack_require__(68309);
// EXTERNAL MODULE: ./node_modules/core-js/modules/es.object.get-prototype-of.js
var es_object_get_prototype_of = __webpack_require__(30489);
// EXTERNAL MODULE: ./node_modules/core-js/modules/es.object.set-prototype-of.js
var es_object_set_prototype_of = __webpack_require__(68304);
// EXTERNAL MODULE: ./node_modules/core-js/modules/es.object.to-string.js
var es_object_to_string = __webpack_require__(41539);
// EXTERNAL MODULE: ./node_modules/core-js/modules/es.regexp.exec.js
var es_regexp_exec = __webpack_require__(74916);
// EXTERNAL MODULE: ./node_modules/core-js/modules/es.string.iterator.js
var es_string_iterator = __webpack_require__(78783);
// EXTERNAL MODULE: ./node_modules/core-js/modules/es.string.search.js
var es_string_search = __webpack_require__(64765);
// EXTERNAL MODULE: ./node_modules/core-js/modules/es.string.starts-with.js
var es_string_starts_with = __webpack_require__(23157);
// EXTERNAL MODULE: ./node_modules/core-js/modules/web.dom-collections.iterator.js
var web_dom_collections_iterator = __webpack_require__(33948);
// EXTERNAL MODULE: ./node_modules/core-js/modules/web.url.js
var web_url = __webpack_require__(60285);
// EXTERNAL MODULE: ./node_modules/regenerator-runtime/runtime.js
var runtime = __webpack_require__(35666);
// EXTERNAL MODULE: ./node_modules/style-loader/dist/runtime/injectStylesIntoStyleTag.js
var injectStylesIntoStyleTag = __webpack_require__(93379);
var injectStylesIntoStyleTag_default = /*#__PURE__*/__webpack_require__.n(injectStylesIntoStyleTag);
// EXTERNAL MODULE: ./node_modules/css-loader/dist/cjs.js!./node_modules/sass-loader/dist/cjs.js??ruleSet[1].rules[2].use[2]!./src/index.scss
var cjs_ruleSet_1_rules_2_use_2_src = __webpack_require__(62233);
;// CONCATENATED MODULE: ./src/index.scss

            

var options = {};

options.insert = "head";
options.singleton = false;

var update = injectStylesIntoStyleTag_default()(cjs_ruleSet_1_rules_2_use_2_src/* default */.Z, options);



/* harmony default export */ const src = (cjs_ruleSet_1_rules_2_use_2_src/* default.locals */.Z.locals || {});
// EXTERNAL MODULE: ./node_modules/core-js/modules/es.array.concat.js
var es_array_concat = __webpack_require__(92222);
// EXTERNAL MODULE: ./node_modules/core-js/modules/es.array.join.js
var es_array_join = __webpack_require__(69600);
// EXTERNAL MODULE: ./node_modules/core-js/modules/es.array.slice.js
var es_array_slice = __webpack_require__(47042);
// EXTERNAL MODULE: ./node_modules/core-js/modules/es.object.keys.js
var es_object_keys = __webpack_require__(47941);
// EXTERNAL MODULE: ./node_modules/core-js/modules/es.string.split.js
var es_string_split = __webpack_require__(23123);
// EXTERNAL MODULE: ./node_modules/react/index.js
var react = __webpack_require__(67294);
// EXTERNAL MODULE: ./node_modules/@fortawesome/react-fontawesome/index.es.js + 1 modules
var index_es = __webpack_require__(17625);
;// CONCATENATED MODULE: ./src/utils.js







function _createForOfIteratorHelper(o, allowArrayLike) { var it; if (typeof Symbol === "undefined" || o[Symbol.iterator] == null) { if (Array.isArray(o) || (it = _unsupportedIterableToArray(o)) || allowArrayLike && o && typeof o.length === "number") { if (it) o = it; var i = 0; var F = function F() {}; return { s: F, n: function n() { if (i >= o.length) return { done: true }; return { done: false, value: o[i++] }; }, e: function e(_e) { throw _e; }, f: F }; } throw new TypeError("Invalid attempt to iterate non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); } var normalCompletion = true, didErr = false, err; return { s: function s() { it = o[Symbol.iterator](); }, n: function n() { var step = it.next(); normalCompletion = step.done; return step; }, e: function e(_e2) { didErr = true; err = _e2; }, f: function f() { try { if (!normalCompletion && it.return != null) it.return(); } finally { if (didErr) throw err; } } }; }

function _unsupportedIterableToArray(o, minLen) { if (!o) return; if (typeof o === "string") return _arrayLikeToArray(o, minLen); var n = Object.prototype.toString.call(o).slice(8, -1); if (n === "Object" && o.constructor) n = o.constructor.name; if (n === "Map" || n === "Set") return Array.from(o); if (n === "Arguments" || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(n)) return _arrayLikeToArray(o, minLen); }

function _arrayLikeToArray(arr, len) { if (len == null || len > arr.length) len = arr.length; for (var i = 0, arr2 = new Array(len); i < len; i++) { arr2[i] = arr[i]; } return arr2; }



/**
 * Partition data so that it is displayer per-group.
 */

function partition(data, key) {
  var def = [];
  var groups = {};

  var _iterator = _createForOfIteratorHelper(data),
      _step;

  try {
    for (_iterator.s(); !(_step = _iterator.n()).done;) {
      var d = _step.value;
      var p = key(d).split('/');

      if (p.length === 1) {
        def.push(d);
        continue;
      }

      var rest = p[p.length - 1];
      var g = p.slice(0, p.length - 1).join('/');
      var group = groups[g] || [];
      group.push({
        short: rest,
        data: d
      });
      groups[g] = group;
    }
  } catch (err) {
    _iterator.e(err);
  } finally {
    _iterator.f();
  }

  var order = Object.keys(groups);
  order.sort();
  return {
    order: order,
    groups: groups,
    def: def
  };
}
/**
 * Generate a browser-originated download.
 * @param {*} contentType
 * @param {*} content
 */

function download(contentType, content, filename) {
  var element = document.createElement('a');
  element.setAttribute('href', "data:".concat(contentType, ";charset=utf-8,") + encodeURIComponent(content));
  element.setAttribute('download', filename);
  element.style.display = 'none';
  document.body.appendChild(element);
  element.click();
  document.body.removeChild(element);
}
/**
 * Format duration in a human-readable way.
 * @param {*} duration
 */

function formatDuration(duration) {
  var seconds = duration % 60;
  var minutes = Math.floor(duration / 60);
  return zeroPad(minutes, 2) + ":" + zeroPad(seconds, 2);
}
/**
 * Pad a number with zeroes.
 *
 * @param {number} num number to pad
 * @param {number} size width of the padding
 */

function zeroPad(num, size) {
  var s = num + "";

  while (s.length < size) {
    s = "0" + s;
  }

  return s;
}
/**
 * Get a percentage form a part and a total.
 *
 * @param {number} part
 * @param {number} total
 */

function percentage(part, total) {
  if (part === total) {
    return 100;
  }

  return Math.round(part / total * 10000) / 100;
}
/**
 * Get the current URL to connect to.
 */

function websocketUrl(path) {
  var loc = window.location;
  var scheme = "ws";

  if (loc.protocol === "https:") {
    scheme = "wss";
  }

  return "".concat(scheme, "://").concat(loc.host, "/").concat(path);
}
/**
 * Get the current URL to connect to.
 */

function apiUrl() {
  var loc = window.location;
  var scheme = "http";

  if (loc.protocol === "https:") {
    scheme = "https";
  }

  return "".concat(scheme, "://").concat(loc.host, "/api");
}
/**
 * Pick the image best suited for album art.
 */

function pickAlbumArt(images, smaller) {
  for (var i = 0; i < images.length; i++) {
    var image = images[i];

    if (image.width <= smaller && image.height <= smaller) {
      return image;
    }
  }

  return null;
}
/**
 * Pick the image best suited for album art.
 */

function pickArtist(artists) {
  if (artists.length == 0) {
    return null;
  }

  return artists[0];
}
/**
 * Indicator that a value is true.
 */

function True() {
  return /*#__PURE__*/react.createElement(index_es/* FontAwesomeIcon */.G, {
    className: "boolean-icon",
    icon: "check"
  });
}
/**
 * Indicator that a value is falso.
 */

function False() {
  return /*#__PURE__*/react.createElement(index_es/* FontAwesomeIcon */.G, {
    className: "boolean-icon",
    icon: "times"
  });
}
// EXTERNAL MODULE: ./node_modules/core-js/modules/es.promise.js
var es_promise = __webpack_require__(88674);
// EXTERNAL MODULE: ./node_modules/core-js/modules/es.regexp.to-string.js
var es_regexp_to_string = __webpack_require__(39714);
;// CONCATENATED MODULE: ./src/api.js









function api_createForOfIteratorHelper(o, allowArrayLike) { var it; if (typeof Symbol === "undefined" || o[Symbol.iterator] == null) { if (Array.isArray(o) || (it = api_unsupportedIterableToArray(o)) || allowArrayLike && o && typeof o.length === "number") { if (it) o = it; var i = 0; var F = function F() {}; return { s: F, n: function n() { if (i >= o.length) return { done: true }; return { done: false, value: o[i++] }; }, e: function e(_e2) { throw _e2; }, f: F }; } throw new TypeError("Invalid attempt to iterate non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); } var normalCompletion = true, didErr = false, err; return { s: function s() { it = o[Symbol.iterator](); }, n: function n() { var step = it.next(); normalCompletion = step.done; return step; }, e: function e(_e3) { didErr = true; err = _e3; }, f: function f() { try { if (!normalCompletion && it.return != null) it.return(); } finally { if (didErr) throw err; } } }; }

function _slicedToArray(arr, i) { return _arrayWithHoles(arr) || _iterableToArrayLimit(arr, i) || api_unsupportedIterableToArray(arr, i) || _nonIterableRest(); }

function _nonIterableRest() { throw new TypeError("Invalid attempt to destructure non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); }

function api_unsupportedIterableToArray(o, minLen) { if (!o) return; if (typeof o === "string") return api_arrayLikeToArray(o, minLen); var n = Object.prototype.toString.call(o).slice(8, -1); if (n === "Object" && o.constructor) n = o.constructor.name; if (n === "Map" || n === "Set") return Array.from(o); if (n === "Arguments" || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(n)) return api_arrayLikeToArray(o, minLen); }

function api_arrayLikeToArray(arr, len) { if (len == null || len > arr.length) len = arr.length; for (var i = 0, arr2 = new Array(len); i < len; i++) { arr2[i] = arr[i]; } return arr2; }

function _iterableToArrayLimit(arr, i) { if (typeof Symbol === "undefined" || !(Symbol.iterator in Object(arr))) return; var _arr = []; var _n = true; var _d = false; var _e = undefined; try { for (var _i = arr[Symbol.iterator](), _s; !(_n = (_s = _i.next()).done); _n = true) { _arr.push(_s.value); if (i && _arr.length === i) break; } } catch (err) { _d = true; _e = err; } finally { try { if (!_n && _i["return"] != null) _i["return"](); } finally { if (_d) throw _e; } } return _arr; }

function _arrayWithHoles(arr) { if (Array.isArray(arr)) return arr; }

function _classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function _defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function _createClass(Constructor, protoProps, staticProps) { if (protoProps) _defineProperties(Constructor.prototype, protoProps); if (staticProps) _defineProperties(Constructor, staticProps); return Constructor; }

var Api = /*#__PURE__*/function () {
  function Api(url) {
    _classCallCheck(this, Api);

    this.url = url;
  }
  /**
   *
   * @param {string | array<string>} path
   * @param {*} data
   */


  _createClass(Api, [{
    key: "fetch",
    value: function (_fetch) {
      function fetch(_x) {
        return _fetch.apply(this, arguments);
      }

      fetch.toString = function () {
        return _fetch.toString();
      };

      return fetch;
    }(function (path) {
      var data = arguments.length > 1 && arguments[1] !== undefined ? arguments[1] : {};

      if (path instanceof Array) {
        path = encodePath(path);
      }

      return fetch("".concat(this.url, "/").concat(path), data).then(function (r) {
        if (!r.ok) {
          return r.text().then(function (text) {
            throw Error("got bad status code: ".concat(r.status, ": ").concat(text));
          });
        }

        return r.json();
      });
    })
    /**
     * Get version information.
     */

  }, {
    key: "version",
    value: function version() {
      return this.fetch(["version"]);
    }
    /**
     * List active connections.
     */

  }, {
    key: "listConnections",
    value: function listConnections() {
      return this.fetch(["auth", "connections"]);
    }
    /**
     * Get a list of devices.
     */

  }, {
    key: "devices",
    value: function devices() {
      return this.fetch("devices");
    }
    /**
     * Set the device to play back from.
     *
     * @param {string} id the id of the device to set.
     */

  }, {
    key: "setDevice",
    value: function setDevice(id) {
      return this.fetch("device/".concat(id), {
        method: "POST"
      });
    }
    /**
     * Get a list of devices.
     */

  }, {
    key: "afterStreams",
    value: function afterStreams() {
      return this.fetch("after-streams");
    }
    /**
     * Delete an after stream.
     *
     * @param {number} id id of after stream to delete.
     */

  }, {
    key: "deleteAfterStream",
    value: function deleteAfterStream(id) {
      return this.fetch("after-stream/".concat(id), {
        method: "DELETE"
      });
    }
    /**
     * Get the list of settings.
     */

  }, {
    key: "settings",
    value: function settings() {
      var filter = arguments.length > 0 && arguments[0] !== undefined ? arguments[0] : {};
      var queries = [];

      if (!!filter.key) {
        var value = filter.key.join(",");
        queries.push("key=".concat(value));
      }

      if (!!filter.prefix) {
        var _value = filter.prefix.join(",");

        queries.push("prefix=".concat(_value));
      }

      if (filter.feature !== undefined) {
        queries.push("feature=true");
      }

      var query = "";

      if (queries.length > 0) {
        query = queries.join("&");
        query = "?".concat(query);
      }

      return this.fetch("settings".concat(query));
    }
    /**
     * Get all cache entries.
     */

  }, {
    key: "cacheDelete",
    value: function cacheDelete(k) {
      var _k = _slicedToArray(k, 2),
          ns = _k[0],
          key = _k[1];

      return this.fetch("cache", {
        method: "DELETE",
        headers: {
          "Content-Type": "application/json"
        },
        body: JSON.stringify({
          ns: ns,
          key: key
        })
      });
    }
    /**
     * Get all cache entries.
     */

  }, {
    key: "cache",
    value: function cache() {
      return this.fetch("cache");
    }
    /**
     * Delete a setting.
     *
     * @param {string} key the key of the setting to delete.
     */

  }, {
    key: "deleteSetting",
    value: function deleteSetting(key) {
      key = settingsKey(key);
      return this.fetch("settings/".concat(key), {
        method: "DELETE"
      });
    }
    /**
     * Edit the given setting.
     *
     * @param {string} key the key to edit
     * @param {any} value the value to edit
     */

  }, {
    key: "editSetting",
    value: function editSetting(key, value) {
      key = settingsKey(key);
      return this.fetch("settings/".concat(key), {
        method: "PUT",
        headers: {
          "Content-Type": "application/json"
        },
        body: JSON.stringify({
          value: value
        })
      });
    }
    /**
     * Export balances.
     */

  }, {
    key: "exportBalances",
    value: function exportBalances() {
      return this.fetch("balances");
    }
    /**
     * Import balances.
     */

  }, {
    key: "importBalances",
    value: function importBalances(balances) {
      return this.fetch("balances", {
        method: "PUT",
        headers: {
          "Content-Type": "application/json"
        },
        body: JSON.stringify(balances)
      });
    }
    /**
     * Get information on the current user.
     */

  }, {
    key: "current",
    value: function current() {
      return this.fetch("current");
    }
  }, {
    key: "aliases",
    value: function aliases(channel) {
      return this.fetch(["aliases", channel]);
    }
    /**
     * Edit the disabled state of an alias.
     *
     * @param {object} key key of the alias to edit
     * @param {bool} disabled set the alias disabled or not
     */

  }, {
    key: "aliasesEditDisabled",
    value: function aliasesEditDisabled(key, disabled) {
      return this.fetch(["aliases", key.channel, key.name, "disabled"], {
        method: "POST",
        headers: {
          "Content-Type": "application/json"
        },
        body: JSON.stringify({
          disabled: disabled
        })
      });
    }
    /**
     * List all commands from a channel.
     */

  }, {
    key: "commands",
    value: function commands(channel) {
      return this.fetch(["commands", channel]);
    }
    /**
     * Edit the disabled state of a command.
     *
     * @param {object} key key of the command to edit
     * @param {bool} disabled set the command disabled or not
     */

  }, {
    key: "commandsEditDisabled",
    value: function commandsEditDisabled(key, disabled) {
      return this.fetch(["commands", key.channel, key.name, "disabled"], {
        method: "POST",
        headers: {
          "Content-Type": "application/json"
        },
        body: JSON.stringify({
          disabled: disabled
        })
      });
    }
  }, {
    key: "promotions",
    value: function promotions(channel) {
      return this.fetch(["promotions", channel]);
    }
    /**
     * Edit the disabled state of a promotion.
     *
     * @param {object} key key of the promotion to edit
     * @param {bool} disabled set the promotion disabled or not
     */

  }, {
    key: "promotionsEditDisabled",
    value: function promotionsEditDisabled(key, disabled) {
      return this.fetch(["promotions", key.channel, key.name, "disabled"], {
        method: "POST",
        headers: {
          "Content-Type": "application/json"
        },
        body: JSON.stringify({
          disabled: disabled
        })
      });
    }
  }, {
    key: "themes",
    value: function themes(channel) {
      return this.fetch(["themes", channel]);
    }
    /**
     * Edit the disabled state of an theme.
     *
     * @param {object} key key of the theme to edit
     * @param {bool} disabled set the theme disabled or not
     */

  }, {
    key: "themesEditDisabled",
    value: function themesEditDisabled(key, disabled) {
      return this.fetch(["themes", key.channel, key.name, "disabled"], {
        method: "POST",
        headers: {
          "Content-Type": "application/json"
        },
        body: JSON.stringify({
          disabled: disabled
        })
      });
    }
    /**
     * Get a list of all available scopes.
     */

  }, {
    key: "authScopes",
    value: function authScopes() {
      return this.fetch(["auth", "scopes"]);
    }
    /**
     * Get a list of all available roles.
     */

  }, {
    key: "authRoles",
    value: function authRoles() {
      return this.fetch(["auth", "roles"]);
    }
    /**
     * Get a list of all enabled grants.
     */

  }, {
    key: "authGrants",
    value: function authGrants() {
      return this.fetch(["auth", "grants"]);
    }
    /**
     * Insert a grant into the database.
     */

  }, {
    key: "authInsertGrant",
    value: function authInsertGrant(auth) {
      return this.fetch(["auth", "grants"], {
        method: "PUT",
        headers: {
          "Content-Type": "application/json"
        },
        body: JSON.stringify(auth)
      });
    }
    /**
     * Delete a grant from the database.
     */

  }, {
    key: "authDeleteGrant",
    value: function authDeleteGrant(scope, role) {
      return this.fetch(["auth", "grants", scope, role], {
        method: "DELETE"
      });
    }
    /**
     * Get all existing chat messages.
     */

  }, {
    key: "chatMessages",
    value: function chatMessages() {
      return this.fetch(["chat", "messages"]);
    }
  }]);

  return Api;
}();

function encodePath(path) {
  var out = [];

  var _iterator = api_createForOfIteratorHelper(path),
      _step;

  try {
    for (_iterator.s(); !(_step = _iterator.n()).done;) {
      var part = _step.value;
      out.push(encodeURIComponent(part));
    }
  } catch (err) {
    _iterator.e(err);
  } finally {
    _iterator.f();
  }

  return out.join("/");
}
/**
 * Encode the URI for a settings key.
 *
 * @param {string} key
 */


function settingsKey(key) {
  var parts = key.split("/");
  var out = [];

  var _iterator2 = api_createForOfIteratorHelper(parts),
      _step2;

  try {
    for (_iterator2.s(); !(_step2 = _iterator2.n()).done;) {
      var part = _step2.value;
      out.push(encodeURIComponent(part));
    }
  } catch (err) {
    _iterator2.e(err);
  } finally {
    _iterator2.f();
  }

  return out.join("/");
}
// EXTERNAL MODULE: ./node_modules/react-dom/index.js
var react_dom = __webpack_require__(73935);
// EXTERNAL MODULE: ./node_modules/react-router-dom/esm/react-router-dom.js
var react_router_dom = __webpack_require__(73727);
// EXTERNAL MODULE: ./node_modules/react-router/esm/react-router.js + 1 modules
var react_router = __webpack_require__(5977);
// EXTERNAL MODULE: ./node_modules/react-bootstrap/esm/Alert.js
var Alert = __webpack_require__(88375);
// EXTERNAL MODULE: ./node_modules/react-bootstrap/esm/Row.js
var Row = __webpack_require__(34051);
// EXTERNAL MODULE: ./node_modules/react-bootstrap/esm/Col.js
var Col = __webpack_require__(31555);
// EXTERNAL MODULE: ./node_modules/react-bootstrap/esm/Navbar.js + 4 modules
var Navbar = __webpack_require__(103);
// EXTERNAL MODULE: ./node_modules/react-bootstrap/esm/Container.js
var Container = __webpack_require__(10682);
// EXTERNAL MODULE: ./node_modules/react-bootstrap/esm/Nav.js + 2 modules
var Nav = __webpack_require__(24779);
// EXTERNAL MODULE: ./node_modules/react-bootstrap/esm/NavDropdown.js + 64 modules
var NavDropdown = __webpack_require__(81855);
// EXTERNAL MODULE: ./node_modules/react-bootstrap/esm/Form.js + 13 modules
var Form = __webpack_require__(2151);
// EXTERNAL MODULE: ./node_modules/react-bootstrap/esm/Button.js
var Button = __webpack_require__(77104);
// EXTERNAL MODULE: ./node_modules/core-js/modules/es.symbol.js
var es_symbol = __webpack_require__(82526);
// EXTERNAL MODULE: ./node_modules/core-js/modules/es.symbol.description.js
var es_symbol_description = __webpack_require__(41817);
// EXTERNAL MODULE: ./node_modules/react-bootstrap/esm/Table.js
var Table = __webpack_require__(75147);
// EXTERNAL MODULE: ../shared-ui/node_modules/react/index.js
var node_modules_react = __webpack_require__(72321);
;// CONCATENATED MODULE: ../shared-ui/components/Loading.js

function Loading(props) {
  if (props.isLoading !== undefined && !props.isLoading) {
    return null;
  }

  var info = null;

  if (props.children) {
    info = /*#__PURE__*/node_modules_react.createElement("div", {
      className: "oxi-loading-info"
    }, props.children);
  }

  return /*#__PURE__*/node_modules_react.createElement("div", {
    className: "oxi-loading"
  }, info, /*#__PURE__*/node_modules_react.createElement("div", {
    className: "spinner-border",
    role: "status"
  }, /*#__PURE__*/node_modules_react.createElement("span", {
    className: "sr-only"
  }, "Loading...")));
}
// EXTERNAL MODULE: ../shared-ui/node_modules/core-js/modules/es.object.to-string.js
var modules_es_object_to_string = __webpack_require__(25283);
// EXTERNAL MODULE: ../shared-ui/node_modules/core-js/modules/es.regexp.to-string.js
var modules_es_regexp_to_string = __webpack_require__(52931);
;// CONCATENATED MODULE: ../shared-ui/components/Error.js



function Error_Loading(props) {
  if (!props.error) {
    return null;
  }

  return /*#__PURE__*/node_modules_react.createElement("div", {
    className: "oxi-error alert alert-danger"
  }, props.error.toString());
}
;// CONCATENATED MODULE: ./src/components/Connections.js
function _typeof(obj) { "@babel/helpers - typeof"; if (typeof Symbol === "function" && typeof Symbol.iterator === "symbol") { _typeof = function _typeof(obj) { return typeof obj; }; } else { _typeof = function _typeof(obj) { return obj && typeof Symbol === "function" && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; }; } return _typeof(obj); }








function asyncGeneratorStep(gen, resolve, reject, _next, _throw, key, arg) { try { var info = gen[key](arg); var value = info.value; } catch (error) { reject(error); return; } if (info.done) { resolve(value); } else { Promise.resolve(value).then(_next, _throw); } }

function _asyncToGenerator(fn) { return function () { var self = this, args = arguments; return new Promise(function (resolve, reject) { var gen = fn.apply(self, args); function _next(value) { asyncGeneratorStep(gen, resolve, reject, _next, _throw, "next", value); } function _throw(err) { asyncGeneratorStep(gen, resolve, reject, _next, _throw, "throw", err); } _next(undefined); }); }; }

function Connections_classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function Connections_defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function Connections_createClass(Constructor, protoProps, staticProps) { if (protoProps) Connections_defineProperties(Constructor.prototype, protoProps); if (staticProps) Connections_defineProperties(Constructor, staticProps); return Constructor; }

function _inherits(subClass, superClass) { if (typeof superClass !== "function" && superClass !== null) { throw new TypeError("Super expression must either be null or a function"); } subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, writable: true, configurable: true } }); if (superClass) _setPrototypeOf(subClass, superClass); }

function _setPrototypeOf(o, p) { _setPrototypeOf = Object.setPrototypeOf || function _setPrototypeOf(o, p) { o.__proto__ = p; return o; }; return _setPrototypeOf(o, p); }

function _createSuper(Derived) { var hasNativeReflectConstruct = _isNativeReflectConstruct(); return function _createSuperInternal() { var Super = _getPrototypeOf(Derived), result; if (hasNativeReflectConstruct) { var NewTarget = _getPrototypeOf(this).constructor; result = Reflect.construct(Super, arguments, NewTarget); } else { result = Super.apply(this, arguments); } return _possibleConstructorReturn(this, result); }; }

function _possibleConstructorReturn(self, call) { if (call && (_typeof(call) === "object" || typeof call === "function")) { return call; } return _assertThisInitialized(self); }

function _assertThisInitialized(self) { if (self === void 0) { throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); } return self; }

function _isNativeReflectConstruct() { if (typeof Reflect === "undefined" || !Reflect.construct) return false; if (Reflect.construct.sham) return false; if (typeof Proxy === "function") return true; try { Date.prototype.toString.call(Reflect.construct(Date, [], function () {})); return true; } catch (e) { return false; } }

function _getPrototypeOf(o) { _getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf : function _getPrototypeOf(o) { return o.__proto__ || Object.getPrototypeOf(o); }; return _getPrototypeOf(o); }






var Connections = /*#__PURE__*/function (_React$Component) {
  _inherits(Connections, _React$Component);

  var _super = _createSuper(Connections);

  function Connections(props) {
    var _this;

    Connections_classCallCheck(this, Connections);

    _this = _super.call(this, props);
    _this.api = _this.props.api;
    _this.state = {
      loading: true,
      error: null,
      connections: []
    };
    return _this;
  }

  Connections_createClass(Connections, [{
    key: "componentDidMount",
    value: function () {
      var _componentDidMount = _asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee() {
        var connections;
        return regeneratorRuntime.wrap(function _callee$(_context) {
          while (1) {
            switch (_context.prev = _context.next) {
              case 0:
                _context.prev = 0;
                _context.next = 3;
                return this.api.listConnections();

              case 3:
                connections = _context.sent;
                this.setState({
                  loading: false,
                  error: null,
                  connections: connections
                });
                _context.next = 10;
                break;

              case 7:
                _context.prev = 7;
                _context.t0 = _context["catch"](0);
                this.setState({
                  loading: false,
                  error: "failed to request connections: ".concat(_context.t0)
                });

              case 10:
              case "end":
                return _context.stop();
            }
          }
        }, _callee, this, [[0, 7]]);
      }));

      function componentDidMount() {
        return _componentDidMount.apply(this, arguments);
      }

      return componentDidMount;
    }()
  }, {
    key: "render",
    value: function render() {
      var error = null;

      if (this.state.error) {
        error = /*#__PURE__*/react.createElement(Alert/* default */.Z, {
          variant: "warning"
        }, this.state.error);
      }

      var content = null;

      if (!this.state.loading) {
        content = /*#__PURE__*/react.createElement(Table/* default */.Z, {
          responsive: "sm"
        }, /*#__PURE__*/react.createElement("tbody", null, this.state.connections.map(function (c, id) {
          return /*#__PURE__*/react.createElement("tr", {
            key: id
          }, /*#__PURE__*/react.createElement("td", null, /*#__PURE__*/react.createElement("b", null, c.title), /*#__PURE__*/react.createElement("br", null), c.description));
        })));
      }

      return /*#__PURE__*/react.createElement(react.Fragment, null, /*#__PURE__*/react.createElement("p", null, "These are your active connections. You can manage them in ", /*#__PURE__*/react.createElement("a", {
        href: "https://setbac.tv/connections"
      }, "My Connections on setbac.tv"), "."), /*#__PURE__*/react.createElement(Loading, {
        isLoading: this.state.loading
      }), /*#__PURE__*/react.createElement(Error_Loading, {
        error: this.state.error
      }), content);
    }
  }]);

  return Connections;
}(react.Component);


// EXTERNAL MODULE: ./node_modules/core-js/modules/es.array.every.js
var es_array_every = __webpack_require__(26541);
;// CONCATENATED MODULE: ./src/components/Devices.js
function Devices_typeof(obj) { "@babel/helpers - typeof"; if (typeof Symbol === "function" && typeof Symbol.iterator === "symbol") { Devices_typeof = function _typeof(obj) { return typeof obj; }; } else { Devices_typeof = function _typeof(obj) { return obj && typeof Symbol === "function" && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; }; } return Devices_typeof(obj); }








function Devices_asyncGeneratorStep(gen, resolve, reject, _next, _throw, key, arg) { try { var info = gen[key](arg); var value = info.value; } catch (error) { reject(error); return; } if (info.done) { resolve(value); } else { Promise.resolve(value).then(_next, _throw); } }

function Devices_asyncToGenerator(fn) { return function () { var self = this, args = arguments; return new Promise(function (resolve, reject) { var gen = fn.apply(self, args); function _next(value) { Devices_asyncGeneratorStep(gen, resolve, reject, _next, _throw, "next", value); } function _throw(err) { Devices_asyncGeneratorStep(gen, resolve, reject, _next, _throw, "throw", err); } _next(undefined); }); }; }

function Devices_classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function Devices_defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function Devices_createClass(Constructor, protoProps, staticProps) { if (protoProps) Devices_defineProperties(Constructor.prototype, protoProps); if (staticProps) Devices_defineProperties(Constructor, staticProps); return Constructor; }

function Devices_inherits(subClass, superClass) { if (typeof superClass !== "function" && superClass !== null) { throw new TypeError("Super expression must either be null or a function"); } subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, writable: true, configurable: true } }); if (superClass) Devices_setPrototypeOf(subClass, superClass); }

function Devices_setPrototypeOf(o, p) { Devices_setPrototypeOf = Object.setPrototypeOf || function _setPrototypeOf(o, p) { o.__proto__ = p; return o; }; return Devices_setPrototypeOf(o, p); }

function Devices_createSuper(Derived) { var hasNativeReflectConstruct = Devices_isNativeReflectConstruct(); return function _createSuperInternal() { var Super = Devices_getPrototypeOf(Derived), result; if (hasNativeReflectConstruct) { var NewTarget = Devices_getPrototypeOf(this).constructor; result = Reflect.construct(Super, arguments, NewTarget); } else { result = Super.apply(this, arguments); } return Devices_possibleConstructorReturn(this, result); }; }

function Devices_possibleConstructorReturn(self, call) { if (call && (Devices_typeof(call) === "object" || typeof call === "function")) { return call; } return Devices_assertThisInitialized(self); }

function Devices_assertThisInitialized(self) { if (self === void 0) { throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); } return self; }

function Devices_isNativeReflectConstruct() { if (typeof Reflect === "undefined" || !Reflect.construct) return false; if (Reflect.construct.sham) return false; if (typeof Proxy === "function") return true; try { Date.prototype.toString.call(Reflect.construct(Date, [], function () {})); return true; } catch (e) { return false; } }

function Devices_getPrototypeOf(o) { Devices_getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf : function _getPrototypeOf(o) { return o.__proto__ || Object.getPrototypeOf(o); }; return Devices_getPrototypeOf(o); }






var Authentication = /*#__PURE__*/function (_React$Component) {
  Devices_inherits(Authentication, _React$Component);

  var _super = Devices_createSuper(Authentication);

  function Authentication(props) {
    var _this;

    Devices_classCallCheck(this, Authentication);

    _this = _super.call(this, props);
    _this.api = _this.props.api;
    _this.state = {
      loading: false,
      error: null,
      data: null
    };
    return _this;
  }

  Devices_createClass(Authentication, [{
    key: "componentDidMount",
    value: function () {
      var _componentDidMount = Devices_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee() {
        return regeneratorRuntime.wrap(function _callee$(_context) {
          while (1) {
            switch (_context.prev = _context.next) {
              case 0:
                _context.next = 2;
                return this.listDevices();

              case 2:
              case "end":
                return _context.stop();
            }
          }
        }, _callee, this);
      }));

      function componentDidMount() {
        return _componentDidMount.apply(this, arguments);
      }

      return componentDidMount;
    }()
    /**
     * Refresh the list of devices.
     */

  }, {
    key: "listDevices",
    value: function () {
      var _listDevices = Devices_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee2() {
        var data;
        return regeneratorRuntime.wrap(function _callee2$(_context2) {
          while (1) {
            switch (_context2.prev = _context2.next) {
              case 0:
                this.setState({
                  loading: true
                });
                _context2.prev = 1;
                _context2.next = 4;
                return this.api.devices();

              case 4:
                data = _context2.sent;
                this.setState({
                  loading: false,
                  error: null,
                  data: data
                });
                _context2.next = 11;
                break;

              case 8:
                _context2.prev = 8;
                _context2.t0 = _context2["catch"](1);
                this.setState({
                  loading: false,
                  error: "failed to request devices: ".concat(_context2.t0),
                  data: null
                });

              case 11:
              case "end":
                return _context2.stop();
            }
          }
        }, _callee2, this, [[1, 8]]);
      }));

      function listDevices() {
        return _listDevices.apply(this, arguments);
      }

      return listDevices;
    }()
    /**
     * Pick the specified device.
     *
     * @param {string} id the device to pick.
     */

  }, {
    key: "pickDevice",
    value: function () {
      var _pickDevice = Devices_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee3(id) {
        return regeneratorRuntime.wrap(function _callee3$(_context3) {
          while (1) {
            switch (_context3.prev = _context3.next) {
              case 0:
                this.setState({
                  loading: true
                });
                _context3.prev = 1;
                _context3.next = 4;
                return this.api.setDevice(id);

              case 4:
                _context3.next = 9;
                break;

              case 6:
                _context3.prev = 6;
                _context3.t0 = _context3["catch"](1);
                this.setState({
                  loading: false,
                  error: "failed to pick device: ".concat(_context3.t0)
                });

              case 9:
              case "end":
                return _context3.stop();
            }
          }
        }, _callee3, this, [[1, 6]]);
      }));

      function pickDevice(_x) {
        return _pickDevice.apply(this, arguments);
      }

      return pickDevice;
    }()
  }, {
    key: "render",
    value: function render() {
      var _this2 = this;

      var error = null;

      if (this.state.error) {
        error = /*#__PURE__*/react.createElement(Alert/* default */.Z, {
          variant: "warning"
        }, this.state.error);
      }

      var selectOne = null;
      var content = null;

      if (this.state.data) {
        if (this.state.data.devices.every(function (d) {
          return !d.is_current;
        })) {
          selectOne = /*#__PURE__*/react.createElement(Alert/* default */.Z, {
            variant: "danger"
          }, /*#__PURE__*/react.createElement("b", null, "No audio device selected"), /*#__PURE__*/react.createElement("br", null), "Press ", /*#__PURE__*/react.createElement(index_es/* FontAwesomeIcon */.G, {
            icon: "play"
          }), " below to select one.");
        }

        if (this.state.data.devices.length === 0) {
          content = /*#__PURE__*/react.createElement(Alert/* default */.Z, {
            variant: "warning"
          }, "No audio devices found, you might have to Authorize Spotify. Otherwise try starting a device and refreshing.");
        } else {
          content = /*#__PURE__*/react.createElement(Table/* default */.Z, {
            responsive: "sm"
          }, /*#__PURE__*/react.createElement("thead", null, /*#__PURE__*/react.createElement("tr", null, /*#__PURE__*/react.createElement("th", {
            colSpan: "2"
          }, "Device Name"), /*#__PURE__*/react.createElement("th", null, "Device Type"))), /*#__PURE__*/react.createElement("tbody", null, this.state.data.devices.map(function (d, id) {
            if (d.is_current) {
              return /*#__PURE__*/react.createElement("tr", {
                key: id
              }, /*#__PURE__*/react.createElement("td", {
                width: "24",
                title: "Current device"
              }, /*#__PURE__*/react.createElement(index_es/* FontAwesomeIcon */.G, {
                icon: "volume-up"
              })), /*#__PURE__*/react.createElement("td", null, d.name), /*#__PURE__*/react.createElement("td", null, d.type));
            }

            return /*#__PURE__*/react.createElement("tr", {
              key: id
            }, /*#__PURE__*/react.createElement("td", {
              width: "24",
              title: "Switch to device"
            }, /*#__PURE__*/react.createElement(index_es/* FontAwesomeIcon */.G, {
              icon: "play",
              className: "clickable",
              onClick: function onClick() {
                return _this2.pickDevice(d.id);
              }
            })), /*#__PURE__*/react.createElement("td", null, d.name), /*#__PURE__*/react.createElement("td", null, d.type));
          })));
        }
      }

      return /*#__PURE__*/react.createElement(react.Fragment, null, /*#__PURE__*/react.createElement(Loading, {
        isLoading: this.state.loading
      }), error, selectOne, content);
    }
  }]);

  return Authentication;
}(react.Component);


// EXTERNAL MODULE: ./node_modules/core-js/modules/es.array.filter.js
var es_array_filter = __webpack_require__(57327);
// EXTERNAL MODULE: ./node_modules/core-js/modules/es.array.index-of.js
var es_array_index_of = __webpack_require__(82772);
// EXTERNAL MODULE: ./node_modules/core-js/modules/es.object.assign.js
var es_object_assign = __webpack_require__(19601);
// EXTERNAL MODULE: ./node_modules/core-js/modules/es.string.replace.js
var es_string_replace = __webpack_require__(15306);
// EXTERNAL MODULE: ./node_modules/react-bootstrap/esm/InputGroup.js
var InputGroup = __webpack_require__(62318);
;// CONCATENATED MODULE: ./src/components/Settings/Base.js
function Base_classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function Base_defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function Base_createClass(Constructor, protoProps, staticProps) { if (protoProps) Base_defineProperties(Constructor.prototype, protoProps); if (staticProps) Base_defineProperties(Constructor, staticProps); return Constructor; }

var Base = /*#__PURE__*/function () {
  function Base(optional) {
    Base_classCallCheck(this, Base);

    this.optional = optional;
  }

  Base_createClass(Base, [{
    key: "edit",
    value: function edit() {
      throw new Error("missing edit() implementation");
    }
  }, {
    key: "hasEditControl",
    value: function hasEditControl() {
      return true;
    }
  }, {
    key: "isSingular",
    value: function isSingular() {
      return true;
    }
  }]);

  return Base;
}();
;// CONCATENATED MODULE: ./src/components/Settings/String.js
function String_typeof(obj) { "@babel/helpers - typeof"; if (typeof Symbol === "function" && typeof Symbol.iterator === "symbol") { String_typeof = function _typeof(obj) { return typeof obj; }; } else { String_typeof = function _typeof(obj) { return obj && typeof Symbol === "function" && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; }; } return String_typeof(obj); }




function String_classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function String_defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function String_createClass(Constructor, protoProps, staticProps) { if (protoProps) String_defineProperties(Constructor.prototype, protoProps); if (staticProps) String_defineProperties(Constructor, staticProps); return Constructor; }

function String_inherits(subClass, superClass) { if (typeof superClass !== "function" && superClass !== null) { throw new TypeError("Super expression must either be null or a function"); } subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, writable: true, configurable: true } }); if (superClass) String_setPrototypeOf(subClass, superClass); }

function String_setPrototypeOf(o, p) { String_setPrototypeOf = Object.setPrototypeOf || function _setPrototypeOf(o, p) { o.__proto__ = p; return o; }; return String_setPrototypeOf(o, p); }

function String_createSuper(Derived) { var hasNativeReflectConstruct = String_isNativeReflectConstruct(); return function _createSuperInternal() { var Super = String_getPrototypeOf(Derived), result; if (hasNativeReflectConstruct) { var NewTarget = String_getPrototypeOf(this).constructor; result = Reflect.construct(Super, arguments, NewTarget); } else { result = Super.apply(this, arguments); } return String_possibleConstructorReturn(this, result); }; }

function String_possibleConstructorReturn(self, call) { if (call && (String_typeof(call) === "object" || typeof call === "function")) { return call; } return String_assertThisInitialized(self); }

function String_assertThisInitialized(self) { if (self === void 0) { throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); } return self; }

function String_isNativeReflectConstruct() { if (typeof Reflect === "undefined" || !Reflect.construct) return false; if (Reflect.construct.sham) return false; if (typeof Proxy === "function") return true; try { Date.prototype.toString.call(Reflect.construct(Date, [], function () {})); return true; } catch (e) { return false; } }

function String_getPrototypeOf(o) { String_getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf : function _getPrototypeOf(o) { return o.__proto__ || Object.getPrototypeOf(o); }; return String_getPrototypeOf(o); }




var String_String = /*#__PURE__*/function (_Base) {
  String_inherits(String, _Base);

  var _super = String_createSuper(String);

  function String(optional, format, placeholder) {
    var _this;

    String_classCallCheck(this, String);

    _this = _super.call(this, optional);
    _this.format = format;
    _this.placeholder = placeholder;
    return _this;
  }

  String_createClass(String, [{
    key: "default",
    value: function _default() {
      return "";
    }
  }, {
    key: "construct",
    value: function construct(value) {
      return value;
    }
  }, {
    key: "serialize",
    value: function serialize(value) {
      return value;
    }
  }, {
    key: "render",
    value: function render(value) {
      return /*#__PURE__*/react.createElement(Form/* default.Control */.Z.Control, {
        size: "sm",
        value: value,
        disabled: true
      });
    }
  }, {
    key: "editControl",
    value: function editControl() {
      return new EditString(this.format, this.placeholder);
    }
  }, {
    key: "edit",
    value: function edit(value) {
      return value;
    }
  }]);

  return String;
}(Base);

var EditString = /*#__PURE__*/function () {
  function EditString(format, placeholder) {
    String_classCallCheck(this, EditString);

    this.format = format;
    this.placeholder = placeholder;
  }

  String_createClass(EditString, [{
    key: "validate",
    value: function validate(value) {
      return this.format.validate(value);
    }
  }, {
    key: "save",
    value: function save(value) {
      return value;
    }
  }, {
    key: "render",
    value: function render(value, _onChange, isValid) {
      return /*#__PURE__*/react.createElement(Form/* default.Control */.Z.Control, {
        size: "sm",
        type: "value",
        placeholder: this.placeholder,
        isInvalid: !isValid,
        value: value,
        onChange: function onChange(e) {
          return _onChange(e.target.value);
        }
      });
    }
  }]);

  return EditString;
}();
;// CONCATENATED MODULE: ./src/components/Settings/Text.js
function Text_typeof(obj) { "@babel/helpers - typeof"; if (typeof Symbol === "function" && typeof Symbol.iterator === "symbol") { Text_typeof = function _typeof(obj) { return typeof obj; }; } else { Text_typeof = function _typeof(obj) { return obj && typeof Symbol === "function" && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; }; } return Text_typeof(obj); }




function Text_classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function Text_defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function Text_createClass(Constructor, protoProps, staticProps) { if (protoProps) Text_defineProperties(Constructor.prototype, protoProps); if (staticProps) Text_defineProperties(Constructor, staticProps); return Constructor; }

function Text_inherits(subClass, superClass) { if (typeof superClass !== "function" && superClass !== null) { throw new TypeError("Super expression must either be null or a function"); } subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, writable: true, configurable: true } }); if (superClass) Text_setPrototypeOf(subClass, superClass); }

function Text_setPrototypeOf(o, p) { Text_setPrototypeOf = Object.setPrototypeOf || function _setPrototypeOf(o, p) { o.__proto__ = p; return o; }; return Text_setPrototypeOf(o, p); }

function Text_createSuper(Derived) { var hasNativeReflectConstruct = Text_isNativeReflectConstruct(); return function _createSuperInternal() { var Super = Text_getPrototypeOf(Derived), result; if (hasNativeReflectConstruct) { var NewTarget = Text_getPrototypeOf(this).constructor; result = Reflect.construct(Super, arguments, NewTarget); } else { result = Super.apply(this, arguments); } return Text_possibleConstructorReturn(this, result); }; }

function Text_possibleConstructorReturn(self, call) { if (call && (Text_typeof(call) === "object" || typeof call === "function")) { return call; } return Text_assertThisInitialized(self); }

function Text_assertThisInitialized(self) { if (self === void 0) { throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); } return self; }

function Text_isNativeReflectConstruct() { if (typeof Reflect === "undefined" || !Reflect.construct) return false; if (Reflect.construct.sham) return false; if (typeof Proxy === "function") return true; try { Date.prototype.toString.call(Reflect.construct(Date, [], function () {})); return true; } catch (e) { return false; } }

function Text_getPrototypeOf(o) { Text_getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf : function _getPrototypeOf(o) { return o.__proto__ || Object.getPrototypeOf(o); }; return Text_getPrototypeOf(o); }




var Text = /*#__PURE__*/function (_Base) {
  Text_inherits(Text, _Base);

  var _super = Text_createSuper(Text);

  function Text(optional) {
    Text_classCallCheck(this, Text);

    return _super.call(this, optional);
  }

  Text_createClass(Text, [{
    key: "default",
    value: function _default() {
      return "";
    }
  }, {
    key: "construct",
    value: function construct(value) {
      return value;
    }
  }, {
    key: "serialize",
    value: function serialize(value) {
      return value;
    }
  }, {
    key: "render",
    value: function render(value) {
      return /*#__PURE__*/react.createElement("pre", {
        className: "settings-text"
      }, /*#__PURE__*/react.createElement("code", null, value));
    }
  }, {
    key: "editControl",
    value: function editControl() {
      return new Text_EditString();
    }
  }, {
    key: "edit",
    value: function edit(value) {
      return value;
    }
  }]);

  return Text;
}(Base);

var Text_EditString = /*#__PURE__*/function () {
  function EditString() {
    Text_classCallCheck(this, EditString);
  }

  Text_createClass(EditString, [{
    key: "validate",
    value: function validate(value) {
      return true;
    }
  }, {
    key: "save",
    value: function save(value) {
      return value;
    }
  }, {
    key: "render",
    value: function render(value, _onChange, _isValid) {
      return /*#__PURE__*/react.createElement(Form/* default.Control */.Z.Control, {
        size: "sm",
        as: "textarea",
        value: value,
        onChange: function onChange(e) {
          return _onChange(e.target.value);
        }
      });
    }
  }]);

  return EditString;
}();
;// CONCATENATED MODULE: ./src/components/Settings/Duration.js
function Duration_typeof(obj) { "@babel/helpers - typeof"; if (typeof Symbol === "function" && typeof Symbol.iterator === "symbol") { Duration_typeof = function _typeof(obj) { return typeof obj; }; } else { Duration_typeof = function _typeof(obj) { return obj && typeof Symbol === "function" && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; }; } return Duration_typeof(obj); }






function Duration_classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function Duration_defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function Duration_createClass(Constructor, protoProps, staticProps) { if (protoProps) Duration_defineProperties(Constructor.prototype, protoProps); if (staticProps) Duration_defineProperties(Constructor, staticProps); return Constructor; }

function Duration_inherits(subClass, superClass) { if (typeof superClass !== "function" && superClass !== null) { throw new TypeError("Super expression must either be null or a function"); } subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, writable: true, configurable: true } }); if (superClass) Duration_setPrototypeOf(subClass, superClass); }

function Duration_setPrototypeOf(o, p) { Duration_setPrototypeOf = Object.setPrototypeOf || function _setPrototypeOf(o, p) { o.__proto__ = p; return o; }; return Duration_setPrototypeOf(o, p); }

function Duration_createSuper(Derived) { var hasNativeReflectConstruct = Duration_isNativeReflectConstruct(); return function _createSuperInternal() { var Super = Duration_getPrototypeOf(Derived), result; if (hasNativeReflectConstruct) { var NewTarget = Duration_getPrototypeOf(this).constructor; result = Reflect.construct(Super, arguments, NewTarget); } else { result = Super.apply(this, arguments); } return Duration_possibleConstructorReturn(this, result); }; }

function Duration_possibleConstructorReturn(self, call) { if (call && (Duration_typeof(call) === "object" || typeof call === "function")) { return call; } return Duration_assertThisInitialized(self); }

function Duration_assertThisInitialized(self) { if (self === void 0) { throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); } return self; }

function Duration_isNativeReflectConstruct() { if (typeof Reflect === "undefined" || !Reflect.construct) return false; if (Reflect.construct.sham) return false; if (typeof Proxy === "function") return true; try { Date.prototype.toString.call(Reflect.construct(Date, [], function () {})); return true; } catch (e) { return false; } }

function Duration_getPrototypeOf(o) { Duration_getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf : function _getPrototypeOf(o) { return o.__proto__ || Object.getPrototypeOf(o); }; return Duration_getPrototypeOf(o); }




var DURATION_REGEX = /^((\d+)d)?((\d+)h)?((\d+)m)?((\d+)s)?$/;
var Duration = /*#__PURE__*/function (_Base) {
  Duration_inherits(Duration, _Base);

  var _super = Duration_createSuper(Duration);

  function Duration(optional) {
    Duration_classCallCheck(this, Duration);

    return _super.call(this, optional);
  }
  /**
   * Parse the given duration.
   *
   * @param {string} input input to parse.
   */


  Duration_createClass(Duration, [{
    key: "default",
    value: function _default() {
      return {
        days: 0,
        hours: 0,
        minutes: 0,
        seconds: 1
      };
    }
  }, {
    key: "construct",
    value: function construct(data) {
      return Duration.parse(data);
    }
    /**
     * Serialize to remote representation.
     */

  }, {
    key: "serialize",
    value: function serialize(value) {
      return this.convertToString(value);
    }
  }, {
    key: "render",
    value: function render(value) {
      var text = this.convertToString(value);
      return /*#__PURE__*/react.createElement(Form/* default.Control */.Z.Control, {
        size: "sm",
        value: text,
        disabled: true
      });
    }
  }, {
    key: "editControl",
    value: function editControl() {
      return new EditDuration();
    }
  }, {
    key: "edit",
    value: function edit(value) {
      return value;
    }
    /**
     * Convert the duration into a string.
     */

  }, {
    key: "convertToString",
    value: function convertToString(value) {
      var nothing = true;
      var s = "";

      if (value.days > 0) {
        nothing = false;
        s += "".concat(value.days, "d");
      }

      if (value.hours > 0) {
        nothing = false;
        s += "".concat(value.hours, "h");
      }

      if (value.minutes > 0) {
        nothing = false;
        s += "".concat(value.minutes, "m");
      }

      if (value.seconds > 0 || nothing) {
        s += "".concat(value.seconds, "s");
      }

      return s;
    }
  }], [{
    key: "parse",
    value: function parse(input) {
      var m = DURATION_REGEX.exec(input);

      if (!m) {
        throw new Error("Bad duration: ".concat(input));
      }

      var days = 0;
      var hours = 0;
      var minutes = 0;
      var seconds = 0;

      if (!!m[2]) {
        days = parseInt(m[2]);
      }

      if (!!m[4]) {
        hours = parseInt(m[4]);
      }

      if (!!m[6]) {
        minutes = parseInt(m[6]);
      }

      if (!!m[8]) {
        seconds = parseInt(m[8]);
      }

      return {
        days: days,
        hours: hours,
        minutes: minutes,
        seconds: seconds
      };
    }
  }]);

  return Duration;
}(Base);

var EditDuration = /*#__PURE__*/function () {
  function EditDuration() {
    Duration_classCallCheck(this, EditDuration);
  }

  Duration_createClass(EditDuration, [{
    key: "validate",
    value: function validate(value) {
      return value.days >= 0 && value.hours >= 0 && value.hours < 24 && value.minutes >= 0 && value.minutes < 60 && value.seconds >= 0 && value.seconds < 60;
    }
  }, {
    key: "save",
    value: function save(value) {
      return Object.assign(value, {});
    }
  }, {
    key: "render",
    value: function render(value, onChange, _isValid) {
      var days = this.digitControl(value.days, "d", function (v) {
        return onChange(Object.assign(value, {
          days: v
        }));
      }, function (_) {
        return true;
      });
      var hours = this.digitControl(value.hours, "h", function (v) {
        return onChange(Object.assign(value, {
          hours: v
        }));
      }, function (v) {
        return v >= 0 && v < 24;
      });
      var minutes = this.digitControl(value.minutes, "m", function (v) {
        return onChange(Object.assign(value, {
          minutes: v
        }));
      }, function (v) {
        return v >= 0 && v < 60;
      });
      var seconds = this.digitControl(value.seconds, "s", function (v) {
        return onChange(Object.assign(value, {
          seconds: v
        }));
      }, function (v) {
        return v >= 0 && v < 60;
      });
      return [days, hours, minutes, seconds];
    }
  }, {
    key: "digitControl",
    value: function digitControl(value, suffix, _onChange, validate) {
      var isValid = validate(value);
      return [/*#__PURE__*/react.createElement(Form/* default.Control */.Z.Control, {
        key: suffix,
        type: "number",
        value: value,
        isInvalid: !isValid,
        onChange: function onChange(e) {
          _onChange(parseInt(e.target.value) || 0);
        }
      }), /*#__PURE__*/react.createElement(InputGroup/* default.Append */.Z.Append, {
        key: "".concat(suffix, "-append")
      }, /*#__PURE__*/react.createElement(InputGroup/* default.Text */.Z.Text, null, suffix))];
    }
  }]);

  return EditDuration;
}();
;// CONCATENATED MODULE: ./src/components/Settings/Boolean.js
function Boolean_typeof(obj) { "@babel/helpers - typeof"; if (typeof Symbol === "function" && typeof Symbol.iterator === "symbol") { Boolean_typeof = function _typeof(obj) { return typeof obj; }; } else { Boolean_typeof = function _typeof(obj) { return obj && typeof Symbol === "function" && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; }; } return Boolean_typeof(obj); }




function _defineProperty(obj, key, value) { if (key in obj) { Object.defineProperty(obj, key, { value: value, enumerable: true, configurable: true, writable: true }); } else { obj[key] = value; } return obj; }

function Boolean_classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function Boolean_defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function Boolean_createClass(Constructor, protoProps, staticProps) { if (protoProps) Boolean_defineProperties(Constructor.prototype, protoProps); if (staticProps) Boolean_defineProperties(Constructor, staticProps); return Constructor; }

function Boolean_inherits(subClass, superClass) { if (typeof superClass !== "function" && superClass !== null) { throw new TypeError("Super expression must either be null or a function"); } subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, writable: true, configurable: true } }); if (superClass) Boolean_setPrototypeOf(subClass, superClass); }

function Boolean_setPrototypeOf(o, p) { Boolean_setPrototypeOf = Object.setPrototypeOf || function _setPrototypeOf(o, p) { o.__proto__ = p; return o; }; return Boolean_setPrototypeOf(o, p); }

function Boolean_createSuper(Derived) { var hasNativeReflectConstruct = Boolean_isNativeReflectConstruct(); return function _createSuperInternal() { var Super = Boolean_getPrototypeOf(Derived), result; if (hasNativeReflectConstruct) { var NewTarget = Boolean_getPrototypeOf(this).constructor; result = Reflect.construct(Super, arguments, NewTarget); } else { result = Super.apply(this, arguments); } return Boolean_possibleConstructorReturn(this, result); }; }

function Boolean_possibleConstructorReturn(self, call) { if (call && (Boolean_typeof(call) === "object" || typeof call === "function")) { return call; } return Boolean_assertThisInitialized(self); }

function Boolean_assertThisInitialized(self) { if (self === void 0) { throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); } return self; }

function Boolean_isNativeReflectConstruct() { if (typeof Reflect === "undefined" || !Reflect.construct) return false; if (Reflect.construct.sham) return false; if (typeof Proxy === "function") return true; try { Date.prototype.toString.call(Reflect.construct(Date, [], function () {})); return true; } catch (e) { return false; } }

function Boolean_getPrototypeOf(o) { Boolean_getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf : function _getPrototypeOf(o) { return o.__proto__ || Object.getPrototypeOf(o); }; return Boolean_getPrototypeOf(o); }





var Boolean = /*#__PURE__*/function (_Base) {
  Boolean_inherits(Boolean, _Base);

  var _super = Boolean_createSuper(Boolean);

  function Boolean(optional) {
    Boolean_classCallCheck(this, Boolean);

    return _super.call(this, optional);
  }

  Boolean_createClass(Boolean, [{
    key: "default",
    value: function _default() {
      return false;
    }
  }, {
    key: "validate",
    value: function validate(value) {
      return true;
    }
  }, {
    key: "construct",
    value: function construct(value) {
      return value;
    }
  }, {
    key: "serialize",
    value: function serialize(value) {
      return value;
    }
  }, {
    key: "render",
    value: function render(value, onChange) {
      if (value) {
        var _React$createElement;

        return /*#__PURE__*/react.createElement(Button/* default */.Z, (_React$createElement = {
          className: "settings-boolean-icon",
          size: "sm",
          title: "Toggle to false"
        }, _defineProperty(_React$createElement, "size", "sm"), _defineProperty(_React$createElement, "variant", "success"), _defineProperty(_React$createElement, "onClick", function onClick() {
          return onChange(false);
        }), _React$createElement), /*#__PURE__*/react.createElement(True, null));
      } else {
        var _React$createElement2;

        return /*#__PURE__*/react.createElement(Button/* default */.Z, (_React$createElement2 = {
          className: "settings-boolean-icon",
          size: "sm",
          title: "Toggle to true"
        }, _defineProperty(_React$createElement2, "size", "sm"), _defineProperty(_React$createElement2, "variant", "danger"), _defineProperty(_React$createElement2, "onClick", function onClick() {
          return onChange(true);
        }), _React$createElement2), /*#__PURE__*/react.createElement(False, null));
      }
    }
  }, {
    key: "editControl",
    value: function editControl() {
      return this;
    }
  }, {
    key: "edit",
    value: function edit(value) {
      return value;
    }
  }, {
    key: "save",
    value: function save(value) {
      return value;
    }
  }, {
    key: "hasEditControl",
    value: function hasEditControl() {
      return false;
    }
  }]);

  return Boolean;
}(Base);
;// CONCATENATED MODULE: ./src/components/Settings/Number.js
function Number_typeof(obj) { "@babel/helpers - typeof"; if (typeof Symbol === "function" && typeof Symbol.iterator === "symbol") { Number_typeof = function _typeof(obj) { return typeof obj; }; } else { Number_typeof = function _typeof(obj) { return obj && typeof Symbol === "function" && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; }; } return Number_typeof(obj); }






function Number_classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function Number_defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function Number_createClass(Constructor, protoProps, staticProps) { if (protoProps) Number_defineProperties(Constructor.prototype, protoProps); if (staticProps) Number_defineProperties(Constructor, staticProps); return Constructor; }

function Number_inherits(subClass, superClass) { if (typeof superClass !== "function" && superClass !== null) { throw new TypeError("Super expression must either be null or a function"); } subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, writable: true, configurable: true } }); if (superClass) Number_setPrototypeOf(subClass, superClass); }

function Number_setPrototypeOf(o, p) { Number_setPrototypeOf = Object.setPrototypeOf || function _setPrototypeOf(o, p) { o.__proto__ = p; return o; }; return Number_setPrototypeOf(o, p); }

function Number_createSuper(Derived) { var hasNativeReflectConstruct = Number_isNativeReflectConstruct(); return function _createSuperInternal() { var Super = Number_getPrototypeOf(Derived), result; if (hasNativeReflectConstruct) { var NewTarget = Number_getPrototypeOf(this).constructor; result = Reflect.construct(Super, arguments, NewTarget); } else { result = Super.apply(this, arguments); } return Number_possibleConstructorReturn(this, result); }; }

function Number_possibleConstructorReturn(self, call) { if (call && (Number_typeof(call) === "object" || typeof call === "function")) { return call; } return Number_assertThisInitialized(self); }

function Number_assertThisInitialized(self) { if (self === void 0) { throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); } return self; }

function Number_isNativeReflectConstruct() { if (typeof Reflect === "undefined" || !Reflect.construct) return false; if (Reflect.construct.sham) return false; if (typeof Proxy === "function") return true; try { Date.prototype.toString.call(Reflect.construct(Date, [], function () {})); return true; } catch (e) { return false; } }

function Number_getPrototypeOf(o) { Number_getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf : function _getPrototypeOf(o) { return o.__proto__ || Object.getPrototypeOf(o); }; return Number_getPrototypeOf(o); }




var Number_Number = /*#__PURE__*/function (_Base) {
  Number_inherits(Number, _Base);

  var _super = Number_createSuper(Number);

  function Number(optional) {
    Number_classCallCheck(this, Number);

    return _super.call(this, optional);
  }

  Number_createClass(Number, [{
    key: "default",
    value: function _default() {
      return 0;
    }
  }, {
    key: "construct",
    value: function construct(data) {
      return data;
    }
  }, {
    key: "serialize",
    value: function serialize(value) {
      return value;
    }
  }, {
    key: "render",
    value: function render(value) {
      return /*#__PURE__*/react.createElement(Form/* default.Control */.Z.Control, {
        size: "sm",
        value: value.toString(),
        disabled: true
      });
    }
  }, {
    key: "editControl",
    value: function editControl() {
      return new EditNumber();
    }
  }, {
    key: "edit",
    value: function edit(value) {
      return value.toString();
    }
  }]);

  return Number;
}(Base);

var EditNumber = /*#__PURE__*/function () {
  function EditNumber() {
    Number_classCallCheck(this, EditNumber);
  }

  Number_createClass(EditNumber, [{
    key: "validate",
    value: function validate(value) {
      return !isNaN(parseInt(value));
    }
  }, {
    key: "save",
    value: function save(value) {
      return parseInt(value);
    }
  }, {
    key: "render",
    value: function render(value, _onChange, isValid) {
      return /*#__PURE__*/react.createElement(Form/* default.Control */.Z.Control, {
        size: "sm",
        type: "number",
        isInvalid: !isValid,
        value: value,
        onChange: function onChange(e) {
          _onChange(e.target.value);
        }
      });
    }
  }]);

  return EditNumber;
}();
;// CONCATENATED MODULE: ./src/components/Settings/Percentage.js
function Percentage_typeof(obj) { "@babel/helpers - typeof"; if (typeof Symbol === "function" && typeof Symbol.iterator === "symbol") { Percentage_typeof = function _typeof(obj) { return typeof obj; }; } else { Percentage_typeof = function _typeof(obj) { return obj && typeof Symbol === "function" && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; }; } return Percentage_typeof(obj); }






function Percentage_classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function Percentage_defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function Percentage_createClass(Constructor, protoProps, staticProps) { if (protoProps) Percentage_defineProperties(Constructor.prototype, protoProps); if (staticProps) Percentage_defineProperties(Constructor, staticProps); return Constructor; }

function Percentage_inherits(subClass, superClass) { if (typeof superClass !== "function" && superClass !== null) { throw new TypeError("Super expression must either be null or a function"); } subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, writable: true, configurable: true } }); if (superClass) Percentage_setPrototypeOf(subClass, superClass); }

function Percentage_setPrototypeOf(o, p) { Percentage_setPrototypeOf = Object.setPrototypeOf || function _setPrototypeOf(o, p) { o.__proto__ = p; return o; }; return Percentage_setPrototypeOf(o, p); }

function Percentage_createSuper(Derived) { var hasNativeReflectConstruct = Percentage_isNativeReflectConstruct(); return function _createSuperInternal() { var Super = Percentage_getPrototypeOf(Derived), result; if (hasNativeReflectConstruct) { var NewTarget = Percentage_getPrototypeOf(this).constructor; result = Reflect.construct(Super, arguments, NewTarget); } else { result = Super.apply(this, arguments); } return Percentage_possibleConstructorReturn(this, result); }; }

function Percentage_possibleConstructorReturn(self, call) { if (call && (Percentage_typeof(call) === "object" || typeof call === "function")) { return call; } return Percentage_assertThisInitialized(self); }

function Percentage_assertThisInitialized(self) { if (self === void 0) { throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); } return self; }

function Percentage_isNativeReflectConstruct() { if (typeof Reflect === "undefined" || !Reflect.construct) return false; if (Reflect.construct.sham) return false; if (typeof Proxy === "function") return true; try { Date.prototype.toString.call(Reflect.construct(Date, [], function () {})); return true; } catch (e) { return false; } }

function Percentage_getPrototypeOf(o) { Percentage_getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf : function _getPrototypeOf(o) { return o.__proto__ || Object.getPrototypeOf(o); }; return Percentage_getPrototypeOf(o); }




var Percentage = /*#__PURE__*/function (_Base) {
  Percentage_inherits(Percentage, _Base);

  var _super = Percentage_createSuper(Percentage);

  function Percentage(optional) {
    Percentage_classCallCheck(this, Percentage);

    return _super.call(this, optional);
  }

  Percentage_createClass(Percentage, [{
    key: "default",
    value: function _default() {
      return 0;
    }
  }, {
    key: "construct",
    value: function construct(data) {
      return data;
    }
  }, {
    key: "serialize",
    value: function serialize(value) {
      return value;
    }
  }, {
    key: "render",
    value: function render(value) {
      return /*#__PURE__*/react.createElement(Form/* default.Control */.Z.Control, {
        size: "sm",
        value: "".concat(value, "%"),
        disabled: true
      });
    }
  }, {
    key: "editControl",
    value: function editControl() {
      return new EditPercentage();
    }
  }, {
    key: "edit",
    value: function edit(value) {
      return value.toString();
    }
  }]);

  return Percentage;
}(Base);

var EditPercentage = /*#__PURE__*/function () {
  function EditPercentage() {
    Percentage_classCallCheck(this, EditPercentage);
  }

  Percentage_createClass(EditPercentage, [{
    key: "validate",
    value: function validate(value) {
      var n = parseInt(value);

      if (isNaN(n)) {
        return false;
      }

      return n >= 0;
    }
  }, {
    key: "save",
    value: function save(value) {
      return parseInt(value) || 0;
    }
  }, {
    key: "render",
    value: function render(value, _onChange, isValid) {
      return [/*#__PURE__*/react.createElement(Form/* default.Control */.Z.Control, {
        key: "percentage",
        type: "number",
        isInvalid: !isValid,
        value: value,
        onChange: function onChange(e) {
          _onChange(e.target.value);
        }
      }), /*#__PURE__*/react.createElement(InputGroup/* default.Append */.Z.Append, {
        key: "percentage-append"
      }, /*#__PURE__*/react.createElement(InputGroup/* default.Text */.Z.Text, null, "%"))];
    }
  }]);

  return EditPercentage;
}();
// EXTERNAL MODULE: ./node_modules/yaml/browser/index.js
var browser = __webpack_require__(69741);
var browser_default = /*#__PURE__*/__webpack_require__.n(browser);
;// CONCATENATED MODULE: ./src/components/Settings/Raw.js
function Raw_typeof(obj) { "@babel/helpers - typeof"; if (typeof Symbol === "function" && typeof Symbol.iterator === "symbol") { Raw_typeof = function _typeof(obj) { return typeof obj; }; } else { Raw_typeof = function _typeof(obj) { return obj && typeof Symbol === "function" && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; }; } return Raw_typeof(obj); }




function Raw_classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function Raw_defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function Raw_createClass(Constructor, protoProps, staticProps) { if (protoProps) Raw_defineProperties(Constructor.prototype, protoProps); if (staticProps) Raw_defineProperties(Constructor, staticProps); return Constructor; }

function Raw_inherits(subClass, superClass) { if (typeof superClass !== "function" && superClass !== null) { throw new TypeError("Super expression must either be null or a function"); } subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, writable: true, configurable: true } }); if (superClass) Raw_setPrototypeOf(subClass, superClass); }

function Raw_setPrototypeOf(o, p) { Raw_setPrototypeOf = Object.setPrototypeOf || function _setPrototypeOf(o, p) { o.__proto__ = p; return o; }; return Raw_setPrototypeOf(o, p); }

function Raw_createSuper(Derived) { var hasNativeReflectConstruct = Raw_isNativeReflectConstruct(); return function _createSuperInternal() { var Super = Raw_getPrototypeOf(Derived), result; if (hasNativeReflectConstruct) { var NewTarget = Raw_getPrototypeOf(this).constructor; result = Reflect.construct(Super, arguments, NewTarget); } else { result = Super.apply(this, arguments); } return Raw_possibleConstructorReturn(this, result); }; }

function Raw_possibleConstructorReturn(self, call) { if (call && (Raw_typeof(call) === "object" || typeof call === "function")) { return call; } return Raw_assertThisInitialized(self); }

function Raw_assertThisInitialized(self) { if (self === void 0) { throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); } return self; }

function Raw_isNativeReflectConstruct() { if (typeof Reflect === "undefined" || !Reflect.construct) return false; if (Reflect.construct.sham) return false; if (typeof Proxy === "function") return true; try { Date.prototype.toString.call(Reflect.construct(Date, [], function () {})); return true; } catch (e) { return false; } }

function Raw_getPrototypeOf(o) { Raw_getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf : function _getPrototypeOf(o) { return o.__proto__ || Object.getPrototypeOf(o); }; return Raw_getPrototypeOf(o); }



 // FIXME: yaml dependency just doesn't work for some reason.


var Raw = /*#__PURE__*/function (_Base) {
  Raw_inherits(Raw, _Base);

  var _super = Raw_createSuper(Raw);

  function Raw(optional) {
    Raw_classCallCheck(this, Raw);

    return _super.call(this, optional);
  }

  Raw_createClass(Raw, [{
    key: "default",
    value: function _default() {
      return {};
    }
  }, {
    key: "construct",
    value: function construct(value) {
      return value;
    }
  }, {
    key: "serialize",
    value: function serialize(data) {
      return data;
    }
  }, {
    key: "render",
    value: function render(data) {
      return /*#__PURE__*/react.createElement("code", null, JSON.stringify(data));
    }
  }, {
    key: "editControl",
    value: function editControl() {
      return new EditRaw();
    }
  }, {
    key: "edit",
    value: function edit(data) {
      return browser_default().stringify(data);
    }
  }]);

  return Raw;
}(Base);

var EditRaw = /*#__PURE__*/function () {
  function EditRaw(value) {
    Raw_classCallCheck(this, EditRaw);

    this.value = value;
  }

  Raw_createClass(EditRaw, [{
    key: "validate",
    value: function validate(value) {
      try {
        browser_default().parse(value);
        return true;
      } catch (e) {
        return false;
      }
    }
  }, {
    key: "save",
    value: function save(value) {
      return browser_default().parse(value);
    }
  }, {
    key: "render",
    value: function render(value, _onChange, isValid) {
      return /*#__PURE__*/react.createElement(Form/* default.Control */.Z.Control, {
        as: "textarea",
        rows: 5,
        size: "sm",
        type: "value",
        isInvalid: !isValid,
        value: value,
        onChange: function onChange(e) {
          _onChange(e.target.value);
        }
      });
    }
  }]);

  return EditRaw;
}();
// EXTERNAL MODULE: ./node_modules/core-js/modules/es.array.splice.js
var es_array_splice = __webpack_require__(40561);
;// CONCATENATED MODULE: ./src/components/Settings/Set.js
function Set_typeof(obj) { "@babel/helpers - typeof"; if (typeof Symbol === "function" && typeof Symbol.iterator === "symbol") { Set_typeof = function _typeof(obj) { return typeof obj; }; } else { Set_typeof = function _typeof(obj) { return obj && typeof Symbol === "function" && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; }; } return Set_typeof(obj); }








function Set_classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function Set_defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function Set_createClass(Constructor, protoProps, staticProps) { if (protoProps) Set_defineProperties(Constructor.prototype, protoProps); if (staticProps) Set_defineProperties(Constructor, staticProps); return Constructor; }

function Set_inherits(subClass, superClass) { if (typeof superClass !== "function" && superClass !== null) { throw new TypeError("Super expression must either be null or a function"); } subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, writable: true, configurable: true } }); if (superClass) Set_setPrototypeOf(subClass, superClass); }

function Set_setPrototypeOf(o, p) { Set_setPrototypeOf = Object.setPrototypeOf || function _setPrototypeOf(o, p) { o.__proto__ = p; return o; }; return Set_setPrototypeOf(o, p); }

function Set_createSuper(Derived) { var hasNativeReflectConstruct = Set_isNativeReflectConstruct(); return function _createSuperInternal() { var Super = Set_getPrototypeOf(Derived), result; if (hasNativeReflectConstruct) { var NewTarget = Set_getPrototypeOf(this).constructor; result = Reflect.construct(Super, arguments, NewTarget); } else { result = Super.apply(this, arguments); } return Set_possibleConstructorReturn(this, result); }; }

function Set_possibleConstructorReturn(self, call) { if (call && (Set_typeof(call) === "object" || typeof call === "function")) { return call; } return Set_assertThisInitialized(self); }

function Set_assertThisInitialized(self) { if (self === void 0) { throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); } return self; }

function Set_isNativeReflectConstruct() { if (typeof Reflect === "undefined" || !Reflect.construct) return false; if (Reflect.construct.sham) return false; if (typeof Proxy === "function") return true; try { Date.prototype.toString.call(Reflect.construct(Date, [], function () {})); return true; } catch (e) { return false; } }

function Set_getPrototypeOf(o) { Set_getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf : function _getPrototypeOf(o) { return o.__proto__ || Object.getPrototypeOf(o); }; return Set_getPrototypeOf(o); }





var Set = /*#__PURE__*/function (_Base) {
  Set_inherits(Set, _Base);

  var _super = Set_createSuper(Set);

  function Set(optional, control) {
    var _this;

    Set_classCallCheck(this, Set);

    _this = _super.call(this, optional);
    _this.control = control;
    return _this;
  }

  Set_createClass(Set, [{
    key: "default",
    value: function _default() {
      return [];
    }
  }, {
    key: "construct",
    value: function construct(value) {
      var _this2 = this;

      return value.map(function (v) {
        return _this2.control.construct(v);
      });
    }
  }, {
    key: "serialize",
    value: function serialize(values) {
      var _this3 = this;

      return values.map(function (value) {
        return _this3.control.serialize(value);
      });
    }
  }, {
    key: "render",
    value: function render(values, parentOnChange) {
      var _this4 = this;

      return /*#__PURE__*/react.createElement("div", null, values.map(function (value, key) {
        var onChange = function onChange(update) {
          var newValues = values.slice();
          newValues[key] = update;
          parentOnChange(newValues);
        };

        return /*#__PURE__*/react.createElement("div", {
          key: key,
          className: "mb-3"
        }, _this4.control.render(value, onChange));
      }));
    }
  }, {
    key: "editControl",
    value: function editControl() {
      return new EditSet(this.optional, this.control, this.control.editControl());
    }
  }, {
    key: "edit",
    value: function edit(values) {
      var _this5 = this;

      return values.map(function (value) {
        return _this5.control.edit(value);
      });
    }
  }, {
    key: "isSingular",
    value: function isSingular() {
      return false;
    }
  }]);

  return Set;
}(Base);

var EditSet = /*#__PURE__*/function () {
  function EditSet(optional, control, editControl) {
    Set_classCallCheck(this, EditSet);

    this.optional = optional;
    this.control = control;
    this.editControl = editControl;
  }

  Set_createClass(EditSet, [{
    key: "validate",
    value: function validate(values) {
      var _this6 = this;

      return values.every(function (value) {
        return _this6.editControl.validate(value);
      });
    }
  }, {
    key: "save",
    value: function save(values) {
      var _this7 = this;

      return values.map(function (value) {
        return _this7.editControl.save(value);
      });
    }
  }, {
    key: "render",
    value: function render(values, onChange, _isValid) {
      var _this8 = this;

      var add = function add() {
        var newValues = values.slice();

        var value = _this8.control.edit(_this8.control.default());

        newValues.push(value);
        onChange(newValues);
      };

      var remove = function remove(key) {
        return function (_) {
          var newValues = values.slice();
          newValues.splice(key, 1);
          onChange(newValues);
        };
      };

      return /*#__PURE__*/react.createElement("div", null, values.map(function (editValue, key) {
        var isValid = _this8.editControl.validate(editValue);

        var control = _this8.editControl.render(editValue, function (v) {
          var newValues = values.slice();
          newValues[key] = v;
          onChange(newValues);
        }, isValid);

        if (_this8.control.isSingular()) {
          return /*#__PURE__*/react.createElement(InputGroup/* default */.Z, {
            key: key,
            className: "mb-1"
          }, control, /*#__PURE__*/react.createElement(InputGroup/* default.Append */.Z.Append, null, /*#__PURE__*/react.createElement(Button/* default */.Z, {
            size: "sm",
            variant: "danger",
            onClick: remove(key)
          }, /*#__PURE__*/react.createElement(index_es/* FontAwesomeIcon */.G, {
            icon: "minus"
          }))));
        } else {
          return [/*#__PURE__*/react.createElement(InputGroup/* default */.Z, {
            key: key,
            className: "mb-1"
          }, control), /*#__PURE__*/react.createElement(Button/* default */.Z, {
            className: "mb-2",
            key: "".concat(key, "-delete"),
            size: "sm",
            variant: "danger",
            onClick: remove(key)
          }, /*#__PURE__*/react.createElement(index_es/* FontAwesomeIcon */.G, {
            icon: "minus"
          }))];
        }
      }), /*#__PURE__*/react.createElement("div", null, /*#__PURE__*/react.createElement(Button/* default */.Z, {
        size: "sm",
        variant: "primary",
        onClick: add
      }, /*#__PURE__*/react.createElement(index_es/* FontAwesomeIcon */.G, {
        icon: "plus"
      }))));
    }
  }]);

  return EditSet;
}();
;// CONCATENATED MODULE: ./src/components/Settings/Select.js
function Select_typeof(obj) { "@babel/helpers - typeof"; if (typeof Symbol === "function" && typeof Symbol.iterator === "symbol") { Select_typeof = function _typeof(obj) { return typeof obj; }; } else { Select_typeof = function _typeof(obj) { return obj && typeof Symbol === "function" && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; }; } return Select_typeof(obj); }





function Select_classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function Select_defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function Select_createClass(Constructor, protoProps, staticProps) { if (protoProps) Select_defineProperties(Constructor.prototype, protoProps); if (staticProps) Select_defineProperties(Constructor, staticProps); return Constructor; }

function Select_inherits(subClass, superClass) { if (typeof superClass !== "function" && superClass !== null) { throw new TypeError("Super expression must either be null or a function"); } subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, writable: true, configurable: true } }); if (superClass) Select_setPrototypeOf(subClass, superClass); }

function Select_setPrototypeOf(o, p) { Select_setPrototypeOf = Object.setPrototypeOf || function _setPrototypeOf(o, p) { o.__proto__ = p; return o; }; return Select_setPrototypeOf(o, p); }

function Select_createSuper(Derived) { var hasNativeReflectConstruct = Select_isNativeReflectConstruct(); return function _createSuperInternal() { var Super = Select_getPrototypeOf(Derived), result; if (hasNativeReflectConstruct) { var NewTarget = Select_getPrototypeOf(this).constructor; result = Reflect.construct(Super, arguments, NewTarget); } else { result = Super.apply(this, arguments); } return Select_possibleConstructorReturn(this, result); }; }

function Select_possibleConstructorReturn(self, call) { if (call && (Select_typeof(call) === "object" || typeof call === "function")) { return call; } return Select_assertThisInitialized(self); }

function Select_assertThisInitialized(self) { if (self === void 0) { throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); } return self; }

function Select_isNativeReflectConstruct() { if (typeof Reflect === "undefined" || !Reflect.construct) return false; if (Reflect.construct.sham) return false; if (typeof Proxy === "function") return true; try { Date.prototype.toString.call(Reflect.construct(Date, [], function () {})); return true; } catch (e) { return false; } }

function Select_getPrototypeOf(o) { Select_getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf : function _getPrototypeOf(o) { return o.__proto__ || Object.getPrototypeOf(o); }; return Select_getPrototypeOf(o); }




var Select = /*#__PURE__*/function (_Base) {
  Select_inherits(Select, _Base);

  var _super = Select_createSuper(Select);

  function Select(optional, value, options) {
    var _this;

    Select_classCallCheck(this, Select);

    _this = _super.call(this, optional);
    _this.value = value;
    _this.options = options;
    return _this;
  }

  Select_createClass(Select, [{
    key: "default",
    value: function _default() {
      return this.value.default();
    }
  }, {
    key: "validate",
    value: function validate(value) {
      return true;
    }
  }, {
    key: "construct",
    value: function construct(value) {
      return this.value.construct(value);
    }
  }, {
    key: "serialize",
    value: function serialize(value) {
      return this.value.serialize(value);
    }
  }, {
    key: "render",
    value: function render(value, parentOnChange) {
      var _this2 = this;

      var onChange = function onChange(e) {
        var option = _this2.options[e.target.selectedIndex];

        var value = _this2.value.construct(option.value);

        parentOnChange(value);
      };

      return /*#__PURE__*/react.createElement(Form/* default.Control */.Z.Control, {
        as: "select",
        size: "sm",
        type: "value",
        value: value,
        onChange: onChange
      }, this.options.map(function (o, i) {
        return /*#__PURE__*/react.createElement("option", {
          value: o.value,
          key: i
        }, o.title);
      }));
    }
  }, {
    key: "editControl",
    value: function editControl() {
      return this;
    }
  }, {
    key: "edit",
    value: function edit(value) {
      return value;
    }
  }, {
    key: "save",
    value: function save(value) {
      return value;
    }
  }, {
    key: "hasEditControl",
    value: function hasEditControl() {
      return false;
    }
  }]);

  return Select;
}(Base);
// EXTERNAL MODULE: ./node_modules/core-js/modules/es.array.find.js
var es_array_find = __webpack_require__(69826);
// EXTERNAL MODULE: ./node_modules/react-bootstrap-typeahead/es/index.js + 52 modules
var es = __webpack_require__(37229);
;// CONCATENATED MODULE: ./src/components/Settings/Typeahead.js
function Typeahead_typeof(obj) { "@babel/helpers - typeof"; if (typeof Symbol === "function" && typeof Symbol.iterator === "symbol") { Typeahead_typeof = function _typeof(obj) { return typeof obj; }; } else { Typeahead_typeof = function _typeof(obj) { return obj && typeof Symbol === "function" && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; }; } return Typeahead_typeof(obj); }





function Typeahead_classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function Typeahead_defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function Typeahead_createClass(Constructor, protoProps, staticProps) { if (protoProps) Typeahead_defineProperties(Constructor.prototype, protoProps); if (staticProps) Typeahead_defineProperties(Constructor, staticProps); return Constructor; }

function Typeahead_inherits(subClass, superClass) { if (typeof superClass !== "function" && superClass !== null) { throw new TypeError("Super expression must either be null or a function"); } subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, writable: true, configurable: true } }); if (superClass) Typeahead_setPrototypeOf(subClass, superClass); }

function Typeahead_setPrototypeOf(o, p) { Typeahead_setPrototypeOf = Object.setPrototypeOf || function _setPrototypeOf(o, p) { o.__proto__ = p; return o; }; return Typeahead_setPrototypeOf(o, p); }

function Typeahead_createSuper(Derived) { var hasNativeReflectConstruct = Typeahead_isNativeReflectConstruct(); return function _createSuperInternal() { var Super = Typeahead_getPrototypeOf(Derived), result; if (hasNativeReflectConstruct) { var NewTarget = Typeahead_getPrototypeOf(this).constructor; result = Reflect.construct(Super, arguments, NewTarget); } else { result = Super.apply(this, arguments); } return Typeahead_possibleConstructorReturn(this, result); }; }

function Typeahead_possibleConstructorReturn(self, call) { if (call && (Typeahead_typeof(call) === "object" || typeof call === "function")) { return call; } return Typeahead_assertThisInitialized(self); }

function Typeahead_assertThisInitialized(self) { if (self === void 0) { throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); } return self; }

function Typeahead_isNativeReflectConstruct() { if (typeof Reflect === "undefined" || !Reflect.construct) return false; if (Reflect.construct.sham) return false; if (typeof Proxy === "function") return true; try { Date.prototype.toString.call(Reflect.construct(Date, [], function () {})); return true; } catch (e) { return false; } }

function Typeahead_getPrototypeOf(o) { Typeahead_getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf : function _getPrototypeOf(o) { return o.__proto__ || Object.getPrototypeOf(o); }; return Typeahead_getPrototypeOf(o); }





var Typeahead = /*#__PURE__*/function (_Base) {
  Typeahead_inherits(Typeahead, _Base);

  var _super = Typeahead_createSuper(Typeahead);

  function Typeahead(optional, value, options) {
    var _this;

    var what = arguments.length > 3 && arguments[3] !== undefined ? arguments[3] : "thing";

    Typeahead_classCallCheck(this, Typeahead);

    _this = _super.call(this, optional);
    _this.value = value;
    _this.options = options;
    _this.what = what;
    return _this;
  }

  Typeahead_createClass(Typeahead, [{
    key: "default",
    value: function _default() {
      return this.value.default();
    }
  }, {
    key: "validate",
    value: function validate(value) {
      return true;
    }
  }, {
    key: "construct",
    value: function construct(value) {
      return this.value.construct(value);
    }
  }, {
    key: "serialize",
    value: function serialize(value) {
      return this.value.serialize(value);
    }
  }, {
    key: "render",
    value: function render(value, parentOnChange) {
      var _this2 = this;

      var onChange = function onChange(e) {
        if (e.length === 0) {
          return;
        }

        var option = e[0];

        var value = _this2.value.construct(option.value);

        parentOnChange(value);
      };

      var current = this.options.find(function (o) {
        return o.value === value;
      });

      if (current) {
        current = current.title;
      } else {
        current = "";
      }

      return /*#__PURE__*/react.createElement(es/* Typeahead */.pY, {
        bsSize: "sm",
        id: "select",
        labelKey: "title",
        value: value,
        options: this.options,
        placeholder: "Choose a ".concat(this.what, "..."),
        defaultInputValue: current,
        onChange: onChange
      });
    }
  }, {
    key: "editControl",
    value: function editControl() {
      return this;
    }
  }, {
    key: "edit",
    value: function edit(value) {
      return value;
    }
  }, {
    key: "save",
    value: function save(value) {
      return value;
    }
  }, {
    key: "hasEditControl",
    value: function hasEditControl() {
      return false;
    }
  }]);

  return Typeahead;
}(Base);
;// CONCATENATED MODULE: ./src/components/Settings/Oauth2Config.js
function Oauth2Config_typeof(obj) { "@babel/helpers - typeof"; if (typeof Symbol === "function" && typeof Symbol.iterator === "symbol") { Oauth2Config_typeof = function _typeof(obj) { return typeof obj; }; } else { Oauth2Config_typeof = function _typeof(obj) { return obj && typeof Symbol === "function" && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; }; } return Oauth2Config_typeof(obj); }





function Oauth2Config_classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function Oauth2Config_defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function Oauth2Config_createClass(Constructor, protoProps, staticProps) { if (protoProps) Oauth2Config_defineProperties(Constructor.prototype, protoProps); if (staticProps) Oauth2Config_defineProperties(Constructor, staticProps); return Constructor; }

function Oauth2Config_inherits(subClass, superClass) { if (typeof superClass !== "function" && superClass !== null) { throw new TypeError("Super expression must either be null or a function"); } subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, writable: true, configurable: true } }); if (superClass) Oauth2Config_setPrototypeOf(subClass, superClass); }

function Oauth2Config_setPrototypeOf(o, p) { Oauth2Config_setPrototypeOf = Object.setPrototypeOf || function _setPrototypeOf(o, p) { o.__proto__ = p; return o; }; return Oauth2Config_setPrototypeOf(o, p); }

function Oauth2Config_createSuper(Derived) { var hasNativeReflectConstruct = Oauth2Config_isNativeReflectConstruct(); return function _createSuperInternal() { var Super = Oauth2Config_getPrototypeOf(Derived), result; if (hasNativeReflectConstruct) { var NewTarget = Oauth2Config_getPrototypeOf(this).constructor; result = Reflect.construct(Super, arguments, NewTarget); } else { result = Super.apply(this, arguments); } return Oauth2Config_possibleConstructorReturn(this, result); }; }

function Oauth2Config_possibleConstructorReturn(self, call) { if (call && (Oauth2Config_typeof(call) === "object" || typeof call === "function")) { return call; } return Oauth2Config_assertThisInitialized(self); }

function Oauth2Config_assertThisInitialized(self) { if (self === void 0) { throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); } return self; }

function Oauth2Config_isNativeReflectConstruct() { if (typeof Reflect === "undefined" || !Reflect.construct) return false; if (Reflect.construct.sham) return false; if (typeof Proxy === "function") return true; try { Date.prototype.toString.call(Reflect.construct(Date, [], function () {})); return true; } catch (e) { return false; } }

function Oauth2Config_getPrototypeOf(o) { Oauth2Config_getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf : function _getPrototypeOf(o) { return o.__proto__ || Object.getPrototypeOf(o); }; return Oauth2Config_getPrototypeOf(o); }




var Oauth2Config = /*#__PURE__*/function (_Base) {
  Oauth2Config_inherits(Oauth2Config, _Base);

  var _super = Oauth2Config_createSuper(Oauth2Config);

  function Oauth2Config(optional) {
    Oauth2Config_classCallCheck(this, Oauth2Config);

    return _super.call(this, optional);
  }

  Oauth2Config_createClass(Oauth2Config, [{
    key: "default",
    value: function _default() {
      return {};
    }
  }, {
    key: "construct",
    value: function construct(value) {
      return value;
    }
  }, {
    key: "serialize",
    value: function serialize(data) {
      return data;
    }
  }, {
    key: "render",
    value: function render(data) {
      return [/*#__PURE__*/react.createElement(Form/* default.Group */.Z.Group, {
        key: "client-id"
      }, /*#__PURE__*/react.createElement(Form/* default.Label */.Z.Label, null, "Client ID"), /*#__PURE__*/react.createElement(Form/* default.Control */.Z.Control, {
        size: "sm",
        disabled: true,
        value: data.client_id
      })), /*#__PURE__*/react.createElement(Form/* default.Group */.Z.Group, {
        key: "client-secret"
      }, /*#__PURE__*/react.createElement(Form/* default.Label */.Z.Label, null, "Client Secret"), /*#__PURE__*/react.createElement(Form/* default.Control */.Z.Control, {
        size: "sm",
        disabled: true,
        value: data.client_secret
      }))];
    }
  }, {
    key: "editControl",
    value: function editControl() {
      return new EditOauth2Config();
    }
  }, {
    key: "edit",
    value: function edit(data) {
      data = Object.assign({}, data);

      if (!data.client_id) {
        data.client_id = "";
      }

      if (!data.client_secret) {
        data.client_secret = "";
      }

      return data;
    }
  }]);

  return Oauth2Config;
}(Base);

var EditOauth2Config = /*#__PURE__*/function () {
  function EditOauth2Config(value) {
    Oauth2Config_classCallCheck(this, EditOauth2Config);

    this.value = value;
  }

  Oauth2Config_createClass(EditOauth2Config, [{
    key: "validate",
    value: function validate(value) {
      if (!value.client_id) {
        return false;
      }

      if (!value.client_secret) {
        return false;
      }

      return true;
    }
  }, {
    key: "save",
    value: function save(value) {
      return {
        "client_id": value.client_id,
        "client_secret": value.client_secret
      };
    }
  }, {
    key: "render",
    value: function render(value, onChange, _isValid) {
      var changeclient_id = function changeclient_id(e) {
        value = Object.assign({}, value);
        value.client_id = e.target.value;
        onChange(value);
      };

      var changeclient_secret = function changeclient_secret(e) {
        value = Object.assign({}, value);
        value.client_secret = e.target.value;
        onChange(value);
      };

      var client_idValid = !!value.client_id;
      var client_secretValid = !!value.client_secret;
      return [/*#__PURE__*/react.createElement(Form/* default.Group */.Z.Group, {
        key: "client-id",
        controlId: "client-id"
      }, /*#__PURE__*/react.createElement(Form/* default.Label */.Z.Label, null, "Client ID"), /*#__PURE__*/react.createElement(Form/* default.Control */.Z.Control, {
        size: "sm",
        isInvalid: !client_idValid,
        value: value.client_id,
        onChange: changeclient_id
      })), /*#__PURE__*/react.createElement(Form/* default.Group */.Z.Group, {
        key: "client-secret",
        controlId: "client-secret",
        className: "mb-0"
      }, /*#__PURE__*/react.createElement(Form/* default.Label */.Z.Label, null, "Client Secret"), /*#__PURE__*/react.createElement(Form/* default.Control */.Z.Control, {
        size: "sm",
        isInvalid: !client_secretValid,
        value: value.client_secret,
        onChange: changeclient_secret
      }))];
    }
  }]);

  return EditOauth2Config;
}();
;// CONCATENATED MODULE: ./src/components/Settings/Object.js
function Object_typeof(obj) { "@babel/helpers - typeof"; if (typeof Symbol === "function" && typeof Symbol.iterator === "symbol") { Object_typeof = function _typeof(obj) { return typeof obj; }; } else { Object_typeof = function _typeof(obj) { return obj && typeof Symbol === "function" && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; }; } return Object_typeof(obj); }






function ownKeys(object, enumerableOnly) { var keys = Object.keys(object); if (Object.getOwnPropertySymbols) { var symbols = Object.getOwnPropertySymbols(object); if (enumerableOnly) symbols = symbols.filter(function (sym) { return Object.getOwnPropertyDescriptor(object, sym).enumerable; }); keys.push.apply(keys, symbols); } return keys; }

function _objectSpread(target) { for (var i = 1; i < arguments.length; i++) { var source = arguments[i] != null ? arguments[i] : {}; if (i % 2) { ownKeys(Object(source), true).forEach(function (key) { Object_defineProperty(target, key, source[key]); }); } else if (Object.getOwnPropertyDescriptors) { Object.defineProperties(target, Object.getOwnPropertyDescriptors(source)); } else { ownKeys(Object(source)).forEach(function (key) { Object.defineProperty(target, key, Object.getOwnPropertyDescriptor(source, key)); }); } } return target; }

function Object_defineProperty(obj, key, value) { if (key in obj) { Object.defineProperty(obj, key, { value: value, enumerable: true, configurable: true, writable: true }); } else { obj[key] = value; } return obj; }

function Object_createForOfIteratorHelper(o, allowArrayLike) { var it; if (typeof Symbol === "undefined" || o[Symbol.iterator] == null) { if (Array.isArray(o) || (it = Object_unsupportedIterableToArray(o)) || allowArrayLike && o && typeof o.length === "number") { if (it) o = it; var i = 0; var F = function F() {}; return { s: F, n: function n() { if (i >= o.length) return { done: true }; return { done: false, value: o[i++] }; }, e: function e(_e) { throw _e; }, f: F }; } throw new TypeError("Invalid attempt to iterate non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); } var normalCompletion = true, didErr = false, err; return { s: function s() { it = o[Symbol.iterator](); }, n: function n() { var step = it.next(); normalCompletion = step.done; return step; }, e: function e(_e2) { didErr = true; err = _e2; }, f: function f() { try { if (!normalCompletion && it.return != null) it.return(); } finally { if (didErr) throw err; } } }; }

function Object_unsupportedIterableToArray(o, minLen) { if (!o) return; if (typeof o === "string") return Object_arrayLikeToArray(o, minLen); var n = Object.prototype.toString.call(o).slice(8, -1); if (n === "Object" && o.constructor) n = o.constructor.name; if (n === "Map" || n === "Set") return Array.from(o); if (n === "Arguments" || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(n)) return Object_arrayLikeToArray(o, minLen); }

function Object_arrayLikeToArray(arr, len) { if (len == null || len > arr.length) len = arr.length; for (var i = 0, arr2 = new Array(len); i < len; i++) { arr2[i] = arr[i]; } return arr2; }

function Object_classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function Object_defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function Object_createClass(Constructor, protoProps, staticProps) { if (protoProps) Object_defineProperties(Constructor.prototype, protoProps); if (staticProps) Object_defineProperties(Constructor, staticProps); return Constructor; }

function Object_inherits(subClass, superClass) { if (typeof superClass !== "function" && superClass !== null) { throw new TypeError("Super expression must either be null or a function"); } subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, writable: true, configurable: true } }); if (superClass) Object_setPrototypeOf(subClass, superClass); }

function Object_setPrototypeOf(o, p) { Object_setPrototypeOf = Object.setPrototypeOf || function _setPrototypeOf(o, p) { o.__proto__ = p; return o; }; return Object_setPrototypeOf(o, p); }

function Object_createSuper(Derived) { var hasNativeReflectConstruct = Object_isNativeReflectConstruct(); return function _createSuperInternal() { var Super = Object_getPrototypeOf(Derived), result; if (hasNativeReflectConstruct) { var NewTarget = Object_getPrototypeOf(this).constructor; result = Reflect.construct(Super, arguments, NewTarget); } else { result = Super.apply(this, arguments); } return Object_possibleConstructorReturn(this, result); }; }

function Object_possibleConstructorReturn(self, call) { if (call && (Object_typeof(call) === "object" || typeof call === "function")) { return call; } return Object_assertThisInitialized(self); }

function Object_assertThisInitialized(self) { if (self === void 0) { throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); } return self; }

function Object_isNativeReflectConstruct() { if (typeof Reflect === "undefined" || !Reflect.construct) return false; if (Reflect.construct.sham) return false; if (typeof Proxy === "function") return true; try { Date.prototype.toString.call(Reflect.construct(Date, [], function () {})); return true; } catch (e) { return false; } }

function Object_getPrototypeOf(o) { Object_getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf : function _getPrototypeOf(o) { return o.__proto__ || Object.getPrototypeOf(o); }; return Object_getPrototypeOf(o); }






var _Object = /*#__PURE__*/function (_Base) {
  Object_inherits(Object, _Base);

  var _super = Object_createSuper(Object);

  function Object(optional, fields) {
    var _this;

    Object_classCallCheck(this, Object);

    _this = _super.call(this, optional);
    _this.fields = fields;
    return _this;
  }

  Object_createClass(Object, [{
    key: "default",
    value: function _default() {
      var o = {};

      var _iterator = Object_createForOfIteratorHelper(this.fields),
          _step;

      try {
        for (_iterator.s(); !(_step = _iterator.n()).done;) {
          var f = _step.value;
          o[f.field] = f.control.default();
        }
      } catch (err) {
        _iterator.e(err);
      } finally {
        _iterator.f();
      }

      return o;
    }
  }, {
    key: "construct",
    value: function construct(value) {
      var o = {};

      var _iterator2 = Object_createForOfIteratorHelper(this.fields),
          _step2;

      try {
        for (_iterator2.s(); !(_step2 = _iterator2.n()).done;) {
          var f = _step2.value;
          o[f.field] = f.control.construct(value[f.field]);
        }
      } catch (err) {
        _iterator2.e(err);
      } finally {
        _iterator2.f();
      }

      return o;
    }
  }, {
    key: "serialize",
    value: function serialize(values) {
      var o = {};

      var _iterator3 = Object_createForOfIteratorHelper(this.fields),
          _step3;

      try {
        for (_iterator3.s(); !(_step3 = _iterator3.n()).done;) {
          var f = _step3.value;
          o[f.field] = f.control.serialize(values[f.field]);
        }
      } catch (err) {
        _iterator3.e(err);
      } finally {
        _iterator3.f();
      }

      return o;
    }
  }, {
    key: "render",
    value: function render(values, parentOnChange) {
      return /*#__PURE__*/react.createElement("div", null, this.fields.map(function (f) {
        var value = values[f.field];

        var onChange = function onChange(update) {
          var newValues = _objectSpread({}, values);

          newValues[f.field] = update;
          parentOnChange(newValues);
        };

        return /*#__PURE__*/react.createElement(InputGroup/* default */.Z, {
          key: f.field,
          size: "sm",
          className: "mb-1"
        }, /*#__PURE__*/react.createElement(InputGroup/* default.Prepend */.Z.Prepend, null, /*#__PURE__*/react.createElement(InputGroup/* default.Text */.Z.Text, null, f.title)), f.control.render(value, onChange));
      }));
    }
  }, {
    key: "editControl",
    value: function editControl() {
      var editControls = {};

      var _iterator4 = Object_createForOfIteratorHelper(this.fields),
          _step4;

      try {
        for (_iterator4.s(); !(_step4 = _iterator4.n()).done;) {
          var f = _step4.value;
          editControls[f.field] = f.control.editControl();
        }
      } catch (err) {
        _iterator4.e(err);
      } finally {
        _iterator4.f();
      }

      return new EditObject(this.optional, this.fields, editControls);
    }
  }, {
    key: "edit",
    value: function edit(values) {
      var o = {};

      var _iterator5 = Object_createForOfIteratorHelper(this.fields),
          _step5;

      try {
        for (_iterator5.s(); !(_step5 = _iterator5.n()).done;) {
          var f = _step5.value;
          o[f.field] = f.control.edit(values[f.field]);
        }
      } catch (err) {
        _iterator5.e(err);
      } finally {
        _iterator5.f();
      }

      return o;
    }
  }, {
    key: "isSingular",
    value: function isSingular() {
      return false;
    }
  }]);

  return Object;
}(Base);



var EditObject = /*#__PURE__*/function () {
  function EditObject(optional, fields, editControls) {
    Object_classCallCheck(this, EditObject);

    this.optional = optional;
    this.fields = fields;
    this.editControls = editControls;
  }

  Object_createClass(EditObject, [{
    key: "validate",
    value: function validate(values) {
      var _this2 = this;

      return this.fields.every(function (f) {
        return _this2.editControls[f.field].validate(values[f.field]);
      });
    }
  }, {
    key: "save",
    value: function save(values) {
      var o = {};

      var _iterator6 = Object_createForOfIteratorHelper(this.fields),
          _step6;

      try {
        for (_iterator6.s(); !(_step6 = _iterator6.n()).done;) {
          var f = _step6.value;
          o[f.field] = this.editControls[f.field].save(values[f.field]);
        }
      } catch (err) {
        _iterator6.e(err);
      } finally {
        _iterator6.f();
      }

      return o;
    }
  }, {
    key: "render",
    value: function render(values, parentOnChange, _isValid) {
      var _this3 = this;

      return /*#__PURE__*/react.createElement("div", null, this.fields.map(function (f) {
        var value = values[f.field];

        var onChange = function onChange(update) {
          var newValues = _objectSpread({}, values);

          newValues[f.field] = update;
          parentOnChange(newValues);
        };

        var control = _this3.editControls[f.field];
        var isValid = control.validate(value);
        return /*#__PURE__*/react.createElement(InputGroup/* default */.Z, {
          key: f.field,
          size: "sm",
          className: "mb-1"
        }, /*#__PURE__*/react.createElement(InputGroup/* default.Prepend */.Z.Prepend, null, /*#__PURE__*/react.createElement(InputGroup/* default.Text */.Z.Text, null, f.title)), control.render(value, onChange, isValid));
      }));
    }
  }]);

  return EditObject;
}();
// EXTERNAL MODULE: ./node_modules/core-js/modules/es.regexp.constructor.js
var es_regexp_constructor = __webpack_require__(24603);
;// CONCATENATED MODULE: ./src/components/Settings/Format.js




function Format_classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function Format_defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function Format_createClass(Constructor, protoProps, staticProps) { if (protoProps) Format_defineProperties(Constructor.prototype, protoProps); if (staticProps) Format_defineProperties(Constructor, staticProps); return Constructor; }

var Regex = /*#__PURE__*/function () {
  function Regex(pattern) {
    Format_classCallCheck(this, Regex);

    this.pattern = new RegExp(pattern);
  }

  Format_createClass(Regex, [{
    key: "validate",
    value: function validate(value) {
      return this.pattern.test(value);
    }
  }]);

  return Regex;
}();
var None = /*#__PURE__*/function () {
  function None() {
    Format_classCallCheck(this, None);
  }

  Format_createClass(None, [{
    key: "validate",
    value: function validate(value) {
      return true;
    }
  }]);

  return None;
}();
function decode(format) {
  switch (format.type) {
    case "regex":
      return new Regex(format.pattern);

    default:
      return new None();
  }
}
// EXTERNAL MODULE: ./node_modules/moment-timezone/index.js
var moment_timezone = __webpack_require__(80008);
;// CONCATENATED MODULE: ./src/components/Settings/Types.js








function Types_createForOfIteratorHelper(o, allowArrayLike) { var it; if (typeof Symbol === "undefined" || o[Symbol.iterator] == null) { if (Array.isArray(o) || (it = Types_unsupportedIterableToArray(o)) || allowArrayLike && o && typeof o.length === "number") { if (it) o = it; var i = 0; var F = function F() {}; return { s: F, n: function n() { if (i >= o.length) return { done: true }; return { done: false, value: o[i++] }; }, e: function e(_e) { throw _e; }, f: F }; } throw new TypeError("Invalid attempt to iterate non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); } var normalCompletion = true, didErr = false, err; return { s: function s() { it = o[Symbol.iterator](); }, n: function n() { var step = it.next(); normalCompletion = step.done; return step; }, e: function e(_e2) { didErr = true; err = _e2; }, f: function f() { try { if (!normalCompletion && it.return != null) it.return(); } finally { if (didErr) throw err; } } }; }

function Types_unsupportedIterableToArray(o, minLen) { if (!o) return; if (typeof o === "string") return Types_arrayLikeToArray(o, minLen); var n = Object.prototype.toString.call(o).slice(8, -1); if (n === "Object" && o.constructor) n = o.constructor.name; if (n === "Map" || n === "Set") return Array.from(o); if (n === "Arguments" || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(n)) return Types_arrayLikeToArray(o, minLen); }

function Types_arrayLikeToArray(arr, len) { if (len == null || len > arr.length) len = arr.length; for (var i = 0, arr2 = new Array(len); i < len; i++) { arr2[i] = arr[i]; } return arr2; }

function Types_ownKeys(object, enumerableOnly) { var keys = Object.keys(object); if (Object.getOwnPropertySymbols) { var symbols = Object.getOwnPropertySymbols(object); if (enumerableOnly) symbols = symbols.filter(function (sym) { return Object.getOwnPropertyDescriptor(object, sym).enumerable; }); keys.push.apply(keys, symbols); } return keys; }

function Types_objectSpread(target) { for (var i = 1; i < arguments.length; i++) { var source = arguments[i] != null ? arguments[i] : {}; if (i % 2) { Types_ownKeys(Object(source), true).forEach(function (key) { Types_defineProperty(target, key, source[key]); }); } else if (Object.getOwnPropertyDescriptors) { Object.defineProperties(target, Object.getOwnPropertyDescriptors(source)); } else { Types_ownKeys(Object(source)).forEach(function (key) { Object.defineProperty(target, key, Object.getOwnPropertyDescriptor(source, key)); }); } } return target; }

function Types_defineProperty(obj, key, value) { if (key in obj) { Object.defineProperty(obj, key, { value: value, enumerable: true, configurable: true, writable: true }); } else { obj[key] = value; } return obj; }















/**
 * Decode the given type and value.
 *
 * @param {object} type the type to decode
 * @param {any} value the value to decode
 */

function Types_decode(type) {
  var what = arguments.length > 1 && arguments[1] !== undefined ? arguments[1] : "thing";

  if (type === null) {
    throw new Error("bad type: ".concat(type));
  }

  var value = null;

  switch (type.id) {
    case "oauth2-config":
      return new Oauth2Config(type.optional);

    case "duration":
      return new Duration(type.optional);

    case "bool":
      return new Boolean(type.optional);

    case "string":
      return new String_String(type.optional, decode(type.format), type.placeholder);

    case "text":
      return new Text(type.optional);

    case "number":
      return new Number_Number(type.optional);

    case "percentage":
      return new Percentage(type.optional);

    case "set":
      value = Types_decode(type.value);
      return new Set(type.optional, value);

    case "select":
      value = Types_decode(type.value);

      switch (type.variant) {
        case "typeahead":
          return new Typeahead(type.optional, value, type.options, what);

        default:
          return new Select(type.optional, value, type.options);
      }

    case "object":
      var fields = type.fields.map(function (f) {
        var control = Types_decode(f.type, what = f.title);
        return Types_objectSpread({
          control: control
        }, f);
      });
      return new _Object(type.optional, fields);

    case "time-zone":
      value = new String_String(false, new None(), "");
      ;
      return new Typeahead(type.optional, value, timezoneOptions, "timezone");

    default:
      return new Raw(type.optional);
  }
}
var timezoneOptions = buildTimezoneOptions();

function buildTimezoneOptions() {
  var out = [];

  var _iterator = Types_createForOfIteratorHelper(moment_timezone.tz.names()),
      _step;

  try {
    for (_iterator.s(); !(_step = _iterator.n()).done;) {
      var name = _step.value;
      var now = moment_timezone();
      var zone = moment_timezone.tz.zone(name);
      var offset = zone.utcOffset(now);
      var abbr = zone.abbr(now);
      var id = null;

      if (offset >= 0) {
        id = "UTC-".concat(tzOffset(offset));
      } else {
        id = "UTC+".concat(tzOffset(-offset));
      }

      name = name.split("/").map(function (n) {
        return n.replace('_', ' ');
      }).join(' / ');
      out.push({
        title: "".concat(id, " - ").concat(name, " (").concat(abbr, ")"),
        value: zone.name
      });
    }
  } catch (err) {
    _iterator.e(err);
  } finally {
    _iterator.f();
  }

  return out;
}

function tzOffset(offset) {
  var rest = offset % 60;
  return "".concat(pad((offset - rest) / 60, 2)).concat(pad(rest, 2));

  function pad(num, size) {
    var s = num + "";

    while (s.length < size) {
      s = "0" + s;
    }

    return s;
  }
}
// EXTERNAL MODULE: ./node_modules/core-js/modules/web.timers.js
var web_timers = __webpack_require__(32564);
// EXTERNAL MODULE: ./node_modules/react-bootstrap/esm/ButtonGroup.js
var ButtonGroup = __webpack_require__(2086);
// EXTERNAL MODULE: ./node_modules/react-markdown/lib/react-markdown.js
var react_markdown = __webpack_require__(30724);
;// CONCATENATED MODULE: ./src/components/Setting.js
function Setting_typeof(obj) { "@babel/helpers - typeof"; if (typeof Symbol === "function" && typeof Symbol.iterator === "symbol") { Setting_typeof = function _typeof(obj) { return typeof obj; }; } else { Setting_typeof = function _typeof(obj) { return obj && typeof Symbol === "function" && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; }; } return Setting_typeof(obj); }






function Setting_classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function Setting_defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function Setting_createClass(Constructor, protoProps, staticProps) { if (protoProps) Setting_defineProperties(Constructor.prototype, protoProps); if (staticProps) Setting_defineProperties(Constructor, staticProps); return Constructor; }

function Setting_inherits(subClass, superClass) { if (typeof superClass !== "function" && superClass !== null) { throw new TypeError("Super expression must either be null or a function"); } subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, writable: true, configurable: true } }); if (superClass) Setting_setPrototypeOf(subClass, superClass); }

function Setting_setPrototypeOf(o, p) { Setting_setPrototypeOf = Object.setPrototypeOf || function _setPrototypeOf(o, p) { o.__proto__ = p; return o; }; return Setting_setPrototypeOf(o, p); }

function Setting_createSuper(Derived) { var hasNativeReflectConstruct = Setting_isNativeReflectConstruct(); return function _createSuperInternal() { var Super = Setting_getPrototypeOf(Derived), result; if (hasNativeReflectConstruct) { var NewTarget = Setting_getPrototypeOf(this).constructor; result = Reflect.construct(Super, arguments, NewTarget); } else { result = Super.apply(this, arguments); } return Setting_possibleConstructorReturn(this, result); }; }

function Setting_possibleConstructorReturn(self, call) { if (call && (Setting_typeof(call) === "object" || typeof call === "function")) { return call; } return Setting_assertThisInitialized(self); }

function Setting_assertThisInitialized(self) { if (self === void 0) { throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); } return self; }

function Setting_isNativeReflectConstruct() { if (typeof Reflect === "undefined" || !Reflect.construct) return false; if (Reflect.construct.sham) return false; if (typeof Proxy === "function") return true; try { Date.prototype.toString.call(Reflect.construct(Date, [], function () {})); return true; } catch (e) { return false; } }

function Setting_getPrototypeOf(o) { Setting_getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf : function _getPrototypeOf(o) { return o.__proto__ || Object.getPrototypeOf(o); }; return Setting_getPrototypeOf(o); }





var SECRET_PREFIX = "secrets/";

function confirmButtons(_ref) {
  var what = _ref.what,
      onConfirm = _ref.onConfirm,
      onCancel = _ref.onCancel,
      confirmDisabled = _ref.confirmDisabled;
  confirmDisabled = confirmDisabled || false;
  return [/*#__PURE__*/react.createElement(Button/* default */.Z, {
    key: "cancel",
    title: "Cancel ".concat(what),
    variant: "primary",
    size: "sm",
    onClick: function onClick(e) {
      return onCancel(e);
    }
  }, /*#__PURE__*/react.createElement(index_es/* FontAwesomeIcon */.G, {
    icon: "window-close"
  })), /*#__PURE__*/react.createElement(Button/* default */.Z, {
    key: "confirm",
    title: "Confirm ".concat(what),
    disabled: confirmDisabled,
    variant: "danger",
    size: "sm",
    onClick: function onClick(e) {
      return onConfirm(e);
    }
  }, /*#__PURE__*/react.createElement(index_es/* FontAwesomeIcon */.G, {
    icon: "check-circle"
  }))];
}

var Setting = /*#__PURE__*/function (_React$Component) {
  Setting_inherits(Setting, _React$Component);

  var _super = Setting_createSuper(Setting);

  function Setting(props) {
    var _this;

    Setting_classCallCheck(this, Setting);

    _this = _super.call(this, props);
    _this.state = {
      delete: false,
      edit: null,
      secretShown: false,
      editValue: null,
      hideCountdown: 0,
      hideInterval: null
    };
    return _this;
  }
  /**
   * Delete the given setting.
   *
   * @param {string} key key of the setting to delete.
   */


  Setting_createClass(Setting, [{
    key: "delete",
    value: function _delete(key) {
      this.props.onDelete(key);
    }
    /**
     * Edit the given setting.
     *
     * @param {string} key key of the setting to edit.
     * @param {string} value the new value to edit it to.
     */

  }, {
    key: "edit",
    value: function edit(key, control, value) {
      this.props.onEdit(key, control, value);
    }
  }, {
    key: "render",
    value: function render() {
      var _this2 = this;

      var setting = this.props.setting;
      var keyOverride = this.props.keyOverride;
      var isSecretShown = this.state.secretShown; // onChange handler used for things which support immediate editing.

      var renderOnChange = function renderOnChange(value) {
        _this2.edit(setting.key, setting.control, value);
      };

      var buttons = [];
      var isSecret = setting.key.startsWith(SECRET_PREFIX) || setting.secret;
      var isNotSet = setting.value === null;

      if (isSecretShown) {
        var hide = function hide() {
          if (_this2.state.hideInterval !== null) {
            clearInterval(_this2.state.hideInterval);
          }

          _this2.setState({
            secretShown: false,
            hideInterval: null
          });
        };

        buttons.push( /*#__PURE__*/react.createElement(Button/* default */.Z, {
          title: "Hide the secret value",
          key: "show",
          size: "sm",
          variant: "secondary",
          className: "action",
          disabled: this.state.loading,
          onClick: hide
        }, /*#__PURE__*/react.createElement(index_es/* FontAwesomeIcon */.G, {
          icon: "eye-slash"
        }), /*#__PURE__*/react.createElement("span", {
          className: "settings-countdown"
        }, this.state.hideCountdown, "s")));
      }

      if (isSecret && !isSecretShown && !isNotSet) {
        var hideFeedback = function hideFeedback() {
          _this2.setState(function (state) {
            if (state.hideCountdown <= 1) {
              clearInterval(state.hideInterval);
              return {
                secretShown: false,
                hideCountdown: 0
              };
            }

            return {
              hideCountdown: state.hideCountdown - 1
            };
          });
        };

        var show = function show() {
          _this2.setState({
            hideCountdown: 10,
            secretShown: true,
            hideInterval: setInterval(hideFeedback, 1000)
          });
        };

        buttons.push( /*#__PURE__*/react.createElement(Button/* default */.Z, {
          title: "Show the secret value",
          key: "show",
          size: "sm",
          variant: "secondary",
          className: "action",
          disabled: this.state.loading,
          onClick: show
        }, /*#__PURE__*/react.createElement(index_es/* FontAwesomeIcon */.G, {
          icon: "eye"
        })));
      }

      if (setting.control.optional && !isSecretShown) {
        var del = function del() {
          _this2.setState({
            delete: true
          });
        };

        if (setting.value !== null) {
          buttons.push( /*#__PURE__*/react.createElement(Button/* default */.Z, {
            key: "delete",
            size: "sm",
            variant: "danger",
            className: "action",
            disabled: this.state.loading,
            onClick: del
          }, /*#__PURE__*/react.createElement(index_es/* FontAwesomeIcon */.G, {
            icon: "trash"
          })));
        }
      }

      if (setting.control.hasEditControl()) {
        if (!isSecretShown) {
          var edit = function edit() {
            var value = setting.value;

            if (value == null) {
              value = setting.control.default();
            }

            var edit = setting.control.editControl();
            var editValue = setting.control.edit(value);

            _this2.setState({
              edit: edit,
              editValue: editValue
            });
          };

          buttons.push( /*#__PURE__*/react.createElement(Button/* default */.Z, {
            key: "edit",
            size: "sm",
            variant: "info",
            className: "action",
            disabled: this.state.loading,
            onClick: edit
          }, /*#__PURE__*/react.createElement(index_es/* FontAwesomeIcon */.G, {
            icon: "edit"
          })));
        }
      }

      var value = null;

      if (isNotSet) {
        value = /*#__PURE__*/react.createElement("em", {
          title: "Value not set"
        }, "not set");
        ;
      } else {
        if (isSecret && !isSecretShown) {
          value = /*#__PURE__*/react.createElement("b", {
            title: "Secret value, only showed when editing"
          }, "****");
        } else {
          value = setting.control.render(setting.value, renderOnChange);
        }
      }

      if (this.state.delete) {
        buttons = confirmButtons({
          what: "deletion",
          onConfirm: function onConfirm() {
            _this2.setState({
              delete: false
            });

            _this2.delete(setting.key);
          },
          onCancel: function onCancel() {
            _this2.setState({
              delete: false
            });
          }
        });
      }

      if (this.state.edit !== null) {
        var isValid = this.state.edit.validate(this.state.editValue);

        var save = function save(e) {
          e.preventDefault();

          if (isValid) {
            var _value = _this2.state.edit.save(_this2.state.editValue);

            _this2.edit(setting.key, setting.control, _value);

            _this2.setState({
              edit: null
            });
          }

          return false;
        };

        var control = this.state.edit.render(this.state.editValue, function (editValue) {
          _this2.setState({
            editValue: editValue
          });
        }, isValid);
        value = /*#__PURE__*/react.createElement(Form/* default */.Z, {
          onSubmit: function onSubmit(e) {
            return save(e);
          }
        }, /*#__PURE__*/react.createElement(InputGroup/* default */.Z, {
          size: "sm"
        }, control));
        buttons = confirmButtons({
          what: "edit",
          confirmDisabled: !isValid,
          onConfirm: function onConfirm(e) {
            return save(e);
          },
          onCancel: function onCancel() {
            _this2.setState({
              edit: null
            });
          }
        });
      }

      if (buttons.length > 0) {
        buttons = /*#__PURE__*/react.createElement("div", {
          className: "ml-3"
        }, /*#__PURE__*/react.createElement(ButtonGroup/* default */.Z, null, buttons));
      }

      var key = keyOverride || setting.key;

      if (this.props.useTitle && !!setting.title) {
        key = /*#__PURE__*/react.createElement(react_markdown, {
          source: setting.title
        });
      }

      var doc = null;

      if (!this.props.disableDoc) {
        doc = /*#__PURE__*/react.createElement("div", {
          className: "settings-key-doc"
        }, /*#__PURE__*/react.createElement(react_markdown, {
          source: setting.doc
        }));
      }

      return /*#__PURE__*/react.createElement("tr", null, /*#__PURE__*/react.createElement("td", null, /*#__PURE__*/react.createElement(Row/* default */.Z, null, /*#__PURE__*/react.createElement(Col/* default */.Z, {
        lg: "4",
        className: "settings-key mb-1"
      }, /*#__PURE__*/react.createElement("div", {
        className: "settings-key-name mb-1"
      }, key), doc), /*#__PURE__*/react.createElement(Col/* default */.Z, {
        lg: "8"
      }, /*#__PURE__*/react.createElement("div", {
        className: "d-flex align-items-top"
      }, /*#__PURE__*/react.createElement("div", {
        className: "flex-fill align-middle"
      }, value), buttons)))));
    }
  }]);

  return Setting;
}(react.Component);


;// CONCATENATED MODULE: ../shared-ui/components/InlineLoading.js

function InlineLoading_Loading(props) {
  if (props.isLoading !== undefined && !props.isLoading) {
    return null;
  }

  return /*#__PURE__*/node_modules_react.createElement("span", {
    className: "oxi-inline-loading spinner-border",
    role: "status"
  }, /*#__PURE__*/node_modules_react.createElement("span", {
    className: "sr-only"
  }, "Loading..."));
}
;// CONCATENATED MODULE: ../shared-ui/components/index.js




;// CONCATENATED MODULE: ./src/components/Settings.js
function Settings_typeof(obj) { "@babel/helpers - typeof"; if (typeof Symbol === "function" && typeof Symbol.iterator === "symbol") { Settings_typeof = function _typeof(obj) { return typeof obj; }; } else { Settings_typeof = function _typeof(obj) { return obj && typeof Symbol === "function" && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; }; } return Settings_typeof(obj); }



















function _extends() { _extends = Object.assign || function (target) { for (var i = 1; i < arguments.length; i++) { var source = arguments[i]; for (var key in source) { if (Object.prototype.hasOwnProperty.call(source, key)) { target[key] = source[key]; } } } return target; }; return _extends.apply(this, arguments); }

function Settings_createForOfIteratorHelper(o, allowArrayLike) { var it; if (typeof Symbol === "undefined" || o[Symbol.iterator] == null) { if (Array.isArray(o) || (it = Settings_unsupportedIterableToArray(o)) || allowArrayLike && o && typeof o.length === "number") { if (it) o = it; var i = 0; var F = function F() {}; return { s: F, n: function n() { if (i >= o.length) return { done: true }; return { done: false, value: o[i++] }; }, e: function e(_e2) { throw _e2; }, f: F }; } throw new TypeError("Invalid attempt to iterate non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); } var normalCompletion = true, didErr = false, err; return { s: function s() { it = o[Symbol.iterator](); }, n: function n() { var step = it.next(); normalCompletion = step.done; return step; }, e: function e(_e3) { didErr = true; err = _e3; }, f: function f() { try { if (!normalCompletion && it.return != null) it.return(); } finally { if (didErr) throw err; } } }; }

function Settings_slicedToArray(arr, i) { return Settings_arrayWithHoles(arr) || Settings_iterableToArrayLimit(arr, i) || Settings_unsupportedIterableToArray(arr, i) || Settings_nonIterableRest(); }

function Settings_nonIterableRest() { throw new TypeError("Invalid attempt to destructure non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); }

function Settings_unsupportedIterableToArray(o, minLen) { if (!o) return; if (typeof o === "string") return Settings_arrayLikeToArray(o, minLen); var n = Object.prototype.toString.call(o).slice(8, -1); if (n === "Object" && o.constructor) n = o.constructor.name; if (n === "Map" || n === "Set") return Array.from(o); if (n === "Arguments" || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(n)) return Settings_arrayLikeToArray(o, minLen); }

function Settings_arrayLikeToArray(arr, len) { if (len == null || len > arr.length) len = arr.length; for (var i = 0, arr2 = new Array(len); i < len; i++) { arr2[i] = arr[i]; } return arr2; }

function Settings_iterableToArrayLimit(arr, i) { if (typeof Symbol === "undefined" || !(Symbol.iterator in Object(arr))) return; var _arr = []; var _n = true; var _d = false; var _e = undefined; try { for (var _i = arr[Symbol.iterator](), _s; !(_n = (_s = _i.next()).done); _n = true) { _arr.push(_s.value); if (i && _arr.length === i) break; } } catch (err) { _d = true; _e = err; } finally { try { if (!_n && _i["return"] != null) _i["return"](); } finally { if (_d) throw _e; } } return _arr; }

function Settings_arrayWithHoles(arr) { if (Array.isArray(arr)) return arr; }

function Settings_asyncGeneratorStep(gen, resolve, reject, _next, _throw, key, arg) { try { var info = gen[key](arg); var value = info.value; } catch (error) { reject(error); return; } if (info.done) { resolve(value); } else { Promise.resolve(value).then(_next, _throw); } }

function Settings_asyncToGenerator(fn) { return function () { var self = this, args = arguments; return new Promise(function (resolve, reject) { var gen = fn.apply(self, args); function _next(value) { Settings_asyncGeneratorStep(gen, resolve, reject, _next, _throw, "next", value); } function _throw(err) { Settings_asyncGeneratorStep(gen, resolve, reject, _next, _throw, "throw", err); } _next(undefined); }); }; }

function Settings_classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function Settings_defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function Settings_createClass(Constructor, protoProps, staticProps) { if (protoProps) Settings_defineProperties(Constructor.prototype, protoProps); if (staticProps) Settings_defineProperties(Constructor, staticProps); return Constructor; }

function Settings_inherits(subClass, superClass) { if (typeof superClass !== "function" && superClass !== null) { throw new TypeError("Super expression must either be null or a function"); } subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, writable: true, configurable: true } }); if (superClass) Settings_setPrototypeOf(subClass, superClass); }

function Settings_setPrototypeOf(o, p) { Settings_setPrototypeOf = Object.setPrototypeOf || function _setPrototypeOf(o, p) { o.__proto__ = p; return o; }; return Settings_setPrototypeOf(o, p); }

function Settings_createSuper(Derived) { var hasNativeReflectConstruct = Settings_isNativeReflectConstruct(); return function _createSuperInternal() { var Super = Settings_getPrototypeOf(Derived), result; if (hasNativeReflectConstruct) { var NewTarget = Settings_getPrototypeOf(this).constructor; result = Reflect.construct(Super, arguments, NewTarget); } else { result = Super.apply(this, arguments); } return Settings_possibleConstructorReturn(this, result); }; }

function Settings_possibleConstructorReturn(self, call) { if (call && (Settings_typeof(call) === "object" || typeof call === "function")) { return call; } return Settings_assertThisInitialized(self); }

function Settings_assertThisInitialized(self) { if (self === void 0) { throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); } return self; }

function Settings_isNativeReflectConstruct() { if (typeof Reflect === "undefined" || !Reflect.construct) return false; if (Reflect.construct.sham) return false; if (typeof Proxy === "function") return true; try { Date.prototype.toString.call(Reflect.construct(Date, [], function () {})); return true; } catch (e) { return false; } }

function Settings_getPrototypeOf(o) { Settings_getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf : function _getPrototypeOf(o) { return o.__proto__ || Object.getPrototypeOf(o); }; return Settings_getPrototypeOf(o); }

function Settings_ownKeys(object, enumerableOnly) { var keys = Object.keys(object); if (Object.getOwnPropertySymbols) { var symbols = Object.getOwnPropertySymbols(object); if (enumerableOnly) symbols = symbols.filter(function (sym) { return Object.getOwnPropertyDescriptor(object, sym).enumerable; }); keys.push.apply(keys, symbols); } return keys; }

function Settings_objectSpread(target) { for (var i = 1; i < arguments.length; i++) { var source = arguments[i] != null ? arguments[i] : {}; if (i % 2) { Settings_ownKeys(Object(source), true).forEach(function (key) { Settings_defineProperty(target, key, source[key]); }); } else if (Object.getOwnPropertyDescriptors) { Object.defineProperties(target, Object.getOwnPropertyDescriptors(source)); } else { Settings_ownKeys(Object(source)).forEach(function (key) { Object.defineProperty(target, key, Object.getOwnPropertyDescriptor(source, key)); }); } } return target; }

function Settings_defineProperty(obj, key, value) { if (key in obj) { Object.defineProperty(obj, key, { value: value, enumerable: true, configurable: true, writable: true }); } else { obj[key] = value; } return obj; }








/**
 * Build a single data entry.
 *
 * @param {*} d data to use a source of entry.
 */

function buildEntry(d) {
  var control = Types_decode(d.schema.type);
  var value = null;

  if (d.value !== null) {
    value = control.construct(d.value);
  }

  return Settings_objectSpread({
    key: d.key,
    control: control,
    value: value
  }, d.schema);
}

var Settings = /*#__PURE__*/function (_React$Component) {
  Settings_inherits(Settings, _React$Component);

  var _super = Settings_createSuper(Settings);

  function Settings(props) {
    var _this;

    Settings_classCallCheck(this, Settings);

    _this = _super.call(this, props);
    var filter = "";

    if (_this.props.location) {
      var search = new URLSearchParams(_this.props.location.search);
      filter = search.get("q") || "";
    }

    _this.api = _this.props.api;
    _this.state = {
      data: null,
      // current filter being applied to filter visible settings.
      filter: filter
    };

    _this.onLoading = function () {};

    if (_this.props.onLoading !== undefined) {
      _this.onLoading = _this.props.onLoading;
    }

    _this.onError = function () {};

    if (_this.props.onError !== undefined) {
      _this.onError = _this.props.onError;
    }

    return _this;
  }

  Settings_createClass(Settings, [{
    key: "componentDidMount",
    value: function () {
      var _componentDidMount = Settings_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee() {
        return regeneratorRuntime.wrap(function _callee$(_context) {
          while (1) {
            switch (_context.prev = _context.next) {
              case 0:
                _context.next = 2;
                return this.list();

              case 2:
              case "end":
                return _context.stop();
            }
          }
        }, _callee, this);
      }));

      function componentDidMount() {
        return _componentDidMount.apply(this, arguments);
      }

      return componentDidMount;
    }()
    /**
     * Update the current filter.
     */

  }, {
    key: "setFilter",
    value: function setFilter(filter) {
      if (this.props.location) {
        var path = "".concat(this.props.location.pathname);

        if (!!filter) {
          var search = new URLSearchParams(this.props.location.search);
          search.set("q", filter);
          path = "".concat(path, "?").concat(search);
        }

        this.props.history.replace(path);
      }

      this.setState({
        filter: filter
      });
    }
    /**
     * Refresh the list of settings.
     */

  }, {
    key: "list",
    value: function () {
      var _list = Settings_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee2() {
        var data;
        return regeneratorRuntime.wrap(function _callee2$(_context2) {
          while (1) {
            switch (_context2.prev = _context2.next) {
              case 0:
                this.onLoading(true);
                _context2.prev = 1;
                _context2.next = 4;
                return this.api.settings(this.props.filter);

              case 4:
                data = _context2.sent;
                data = data.map(function (d) {
                  return buildEntry(d);
                });
                this.setState({
                  data: data
                });
                _context2.next = 12;
                break;

              case 9:
                _context2.prev = 9;
                _context2.t0 = _context2["catch"](1);
                this.onError(_context2.t0);

              case 12:
                this.onLoading(false);

              case 13:
              case "end":
                return _context2.stop();
            }
          }
        }, _callee2, this, [[1, 9]]);
      }));

      function list() {
        return _list.apply(this, arguments);
      }

      return list;
    }()
    /**
     * Delete the given setting.
     *
     * @param {string} key key of the setting to delete.
     */

  }, {
    key: "delete",
    value: function () {
      var _delete2 = Settings_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee3(key, index) {
        return regeneratorRuntime.wrap(function _callee3$(_context3) {
          while (1) {
            switch (_context3.prev = _context3.next) {
              case 0:
                this.onLoading(true);
                this.setState(function (state) {
                  return {
                    data: state.data.map(function (setting) {
                      if (setting.key === key) {
                        return Object.assign(setting, {
                          value: null
                        });
                      }

                      return setting;
                    })
                  };
                });
                _context3.prev = 2;
                _context3.next = 5;
                return this.api.deleteSetting(key);

              case 5:
                _context3.next = 10;
                break;

              case 7:
                _context3.prev = 7;
                _context3.t0 = _context3["catch"](2);
                this.onError(_context3.t0);

              case 10:
                this.onLoading(false);

              case 11:
              case "end":
                return _context3.stop();
            }
          }
        }, _callee3, this, [[2, 7]]);
      }));

      function _delete(_x, _x2) {
        return _delete2.apply(this, arguments);
      }

      return _delete;
    }()
    /**
     * Edit the given setting.
     *
     * @param {string} key key of the setting to edit.
     * @param {string} value the new value to edit it to.
     */

  }, {
    key: "edit",
    value: function () {
      var _edit = Settings_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee4(key, control, value) {
        var _yield$this$api$setti, _yield$this$api$setti2, update;

        return regeneratorRuntime.wrap(function _callee4$(_context4) {
          while (1) {
            switch (_context4.prev = _context4.next) {
              case 0:
                this.onLoading(true);
                this.setState(function (state) {
                  return {
                    data: state.data.map(function (setting) {
                      if (setting.key === key) {
                        return Object.assign(setting, {
                          value: value
                        });
                      }

                      return setting;
                    })
                  };
                });
                _context4.prev = 2;
                _context4.next = 5;
                return this.api.editSetting(key, control.serialize(value));

              case 5:
                _context4.next = 7;
                return this.api.settings({
                  key: [key]
                });

              case 7:
                _yield$this$api$setti = _context4.sent;
                _yield$this$api$setti2 = Settings_slicedToArray(_yield$this$api$setti, 1);
                update = _yield$this$api$setti2[0];
                this.setState(function (state) {
                  return {
                    data: state.data.map(function (setting) {
                      if (setting.key === key) {
                        return buildEntry(update);
                      }

                      return setting;
                    })
                  };
                });
                _context4.next = 16;
                break;

              case 13:
                _context4.prev = 13;
                _context4.t0 = _context4["catch"](2);
                this.onError(_context4.t0);

              case 16:
                this.onLoading(false);

              case 17:
              case "end":
                return _context4.stop();
            }
          }
        }, _callee4, this, [[2, 13]]);
      }));

      function edit(_x3, _x4, _x5) {
        return _edit.apply(this, arguments);
      }

      return edit;
    }()
    /**
     * Filter the data if applicable.
     */

  }, {
    key: "filtered",
    value: function filtered(data) {
      if (!this.state.filter) {
        return data;
      }

      if (this.state.filter.startsWith('^')) {
        var filter = this.state.filter.substring(1);
        return data.filter(function (d) {
          return d.key.startsWith(filter);
        });
      }

      var parts = this.state.filter.split(" ").map(function (f) {
        return f.toLowerCase();
      });
      return data.filter(function (d) {
        return parts.every(function (p) {
          if (d.key.toLowerCase().indexOf(p) != -1) {
            return true;
          }

          if (d.title && d.title.toLowerCase().indexOf(p) != -1) {
            return true;
          }

          return false;
        });
      });
    }
    /**
     * Render the given name as a set of clickable links.
     */

  }, {
    key: "filterLinks",
    value: function filterLinks(name) {
      var _this2 = this;

      var setFilter = function setFilter(filter) {
        return function () {
          return _this2.setFilter("^".concat(filter, "/"));
        };
      };

      var parts = name.split("/");
      var path = [];
      var len = 0;
      var out = [];

      var _iterator = Settings_createForOfIteratorHelper(parts),
          _step;

      try {
        for (_iterator.s(); !(_step = _iterator.n()).done;) {
          var p = _step.value;
          path.push(p);
          len += p.length;
          var filter = name.substring(0, Math.min(len, name.length));
          len += 1;
          out.push( /*#__PURE__*/react.createElement("a", {
            className: "settings-filter",
            title: "Filter for \"".concat(filter, "\" prefix."),
            key: filter,
            onClick: setFilter(filter)
          }, p));
          out.push("/");
        }
      } catch (err) {
        _iterator.e(err);
      } finally {
        _iterator.f();
      }

      return out;
    }
  }, {
    key: "content",
    value: function content() {
      var _this3 = this;

      if (!this.state.data) {
        return null;
      }

      if (this.state.data.length === 0) {
        return /*#__PURE__*/react.createElement(Alert/* default */.Z, {
          variant: "info"
        }, "No Settings!");
      }

      var settingProps = {
        useTitle: !!this.props.useTitle,
        disableDoc: !!this.props.disableDoc
      };
      var data = this.filtered(this.state.data);

      if (!this.props.group) {
        return /*#__PURE__*/react.createElement("div", null, /*#__PURE__*/react.createElement(Table/* default */.Z, {
          className: "mb-0"
        }, /*#__PURE__*/react.createElement("tbody", null, data.map(function (s) {
          return /*#__PURE__*/react.createElement(Setting, _extends({
            key: s.key,
            setting: s,
            onEdit: _this3.edit.bind(_this3),
            onDelete: _this3.delete.bind(_this3)
          }, settingProps));
        }))));
      }

      var _partition = partition(data, function (d) {
        return d.key;
      }),
          order = _partition.order,
          groups = _partition.groups,
          def = _partition.def;

      return /*#__PURE__*/react.createElement("div", null, /*#__PURE__*/react.createElement(Table/* default */.Z, {
        className: "mb-0"
      }, /*#__PURE__*/react.createElement("tbody", null, def.map(function (s) {
        return /*#__PURE__*/react.createElement(Setting, _extends({
          key: s.key,
          setting: s,
          onEdit: _this3.edit.bind(_this3),
          onDelete: _this3.delete.bind(_this3)
        }, settingProps));
      }))), order.map(function (name) {
        var group = groups[name];
        var title = null;

        if (_this3.props.filterable) {
          title = _this3.filterLinks(name);
        } else {
          title = name;
        }

        return /*#__PURE__*/react.createElement(Table/* default */.Z, {
          className: "mb-0",
          key: name
        }, /*#__PURE__*/react.createElement("tbody", null, /*#__PURE__*/react.createElement("tr", null, /*#__PURE__*/react.createElement("th", {
          className: "settings-group"
        }, title)), group.map(function (_ref) {
          var short = _ref.short,
              data = _ref.data;
          return /*#__PURE__*/react.createElement(Setting, _extends({
            key: data.key,
            setting: data,
            onEdit: _this3.edit.bind(_this3),
            onDelete: _this3.delete.bind(_this3),
            keyOverride: short
          }, settingProps));
        })));
      }));
    }
  }, {
    key: "render",
    value: function render() {
      var _this4 = this;

      var content = this.content();
      var filter = null;

      if (this.props.filterable) {
        var filterOnChange = function filterOnChange(e) {
          return _this4.setFilter(e.target.value);
        };

        var clearFilter = function clearFilter() {
          return _this4.setFilter("");
        };

        var clear = null;

        if (!!this.state.filter) {
          clear = /*#__PURE__*/react.createElement(InputGroup/* default.Append */.Z.Append, null, /*#__PURE__*/react.createElement(Button/* default */.Z, {
            variant: "primary",
            onClick: clearFilter
          }, "Clear Filter"));
        }

        filter = /*#__PURE__*/react.createElement(Form/* default */.Z, {
          className: "mt-4 mb-4"
        }, /*#__PURE__*/react.createElement(InputGroup/* default */.Z, null, /*#__PURE__*/react.createElement(Form/* default.Control */.Z.Control, {
          value: this.state.filter,
          placeholder: "Search",
          onChange: filterOnChange
        }), clear));
      }

      return /*#__PURE__*/react.createElement("div", {
        className: "settings"
      }, filter, content);
    }
  }]);

  return Settings;
}(react.Component);


;// CONCATENATED MODULE: ./src/components/ConfigurationPrompt.js
function ConfigurationPrompt_typeof(obj) { "@babel/helpers - typeof"; if (typeof Symbol === "function" && typeof Symbol.iterator === "symbol") { ConfigurationPrompt_typeof = function _typeof(obj) { return typeof obj; }; } else { ConfigurationPrompt_typeof = function _typeof(obj) { return obj && typeof Symbol === "function" && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; }; } return ConfigurationPrompt_typeof(obj); }







function ConfigurationPrompt_asyncGeneratorStep(gen, resolve, reject, _next, _throw, key, arg) { try { var info = gen[key](arg); var value = info.value; } catch (error) { reject(error); return; } if (info.done) { resolve(value); } else { Promise.resolve(value).then(_next, _throw); } }

function ConfigurationPrompt_asyncToGenerator(fn) { return function () { var self = this, args = arguments; return new Promise(function (resolve, reject) { var gen = fn.apply(self, args); function _next(value) { ConfigurationPrompt_asyncGeneratorStep(gen, resolve, reject, _next, _throw, "next", value); } function _throw(err) { ConfigurationPrompt_asyncGeneratorStep(gen, resolve, reject, _next, _throw, "throw", err); } _next(undefined); }); }; }

function ConfigurationPrompt_classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function ConfigurationPrompt_defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function ConfigurationPrompt_createClass(Constructor, protoProps, staticProps) { if (protoProps) ConfigurationPrompt_defineProperties(Constructor.prototype, protoProps); if (staticProps) ConfigurationPrompt_defineProperties(Constructor, staticProps); return Constructor; }

function ConfigurationPrompt_inherits(subClass, superClass) { if (typeof superClass !== "function" && superClass !== null) { throw new TypeError("Super expression must either be null or a function"); } subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, writable: true, configurable: true } }); if (superClass) ConfigurationPrompt_setPrototypeOf(subClass, superClass); }

function ConfigurationPrompt_setPrototypeOf(o, p) { ConfigurationPrompt_setPrototypeOf = Object.setPrototypeOf || function _setPrototypeOf(o, p) { o.__proto__ = p; return o; }; return ConfigurationPrompt_setPrototypeOf(o, p); }

function ConfigurationPrompt_createSuper(Derived) { var hasNativeReflectConstruct = ConfigurationPrompt_isNativeReflectConstruct(); return function _createSuperInternal() { var Super = ConfigurationPrompt_getPrototypeOf(Derived), result; if (hasNativeReflectConstruct) { var NewTarget = ConfigurationPrompt_getPrototypeOf(this).constructor; result = Reflect.construct(Super, arguments, NewTarget); } else { result = Super.apply(this, arguments); } return ConfigurationPrompt_possibleConstructorReturn(this, result); }; }

function ConfigurationPrompt_possibleConstructorReturn(self, call) { if (call && (ConfigurationPrompt_typeof(call) === "object" || typeof call === "function")) { return call; } return ConfigurationPrompt_assertThisInitialized(self); }

function ConfigurationPrompt_assertThisInitialized(self) { if (self === void 0) { throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); } return self; }

function ConfigurationPrompt_isNativeReflectConstruct() { if (typeof Reflect === "undefined" || !Reflect.construct) return false; if (Reflect.construct.sham) return false; if (typeof Proxy === "function") return true; try { Date.prototype.toString.call(Reflect.construct(Date, [], function () {})); return true; } catch (e) { return false; } }

function ConfigurationPrompt_getPrototypeOf(o) { ConfigurationPrompt_getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf : function _getPrototypeOf(o) { return o.__proto__ || Object.getPrototypeOf(o); }; return ConfigurationPrompt_getPrototypeOf(o); }




var ConfigurationPrompt = /*#__PURE__*/function (_React$Component) {
  ConfigurationPrompt_inherits(ConfigurationPrompt, _React$Component);

  var _super = ConfigurationPrompt_createSuper(ConfigurationPrompt);

  function ConfigurationPrompt(props) {
    var _this;

    ConfigurationPrompt_classCallCheck(this, ConfigurationPrompt);

    _this = _super.call(this, props);
    _this.state = {
      configured: true
    };

    _this.onLoading = function () {};

    if (_this.props.onLoading !== undefined) {
      _this.onLoading = _this.props.onLoading;
    }

    _this.onError = function () {};

    if (_this.props.onError !== undefined) {
      _this.onError = _this.props.onError;
    }

    return _this;
  }

  ConfigurationPrompt_createClass(ConfigurationPrompt, [{
    key: "componentDidMount",
    value: function () {
      var _componentDidMount = ConfigurationPrompt_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee() {
        return regeneratorRuntime.wrap(function _callee$(_context) {
          while (1) {
            switch (_context.prev = _context.next) {
              case 0:
                _context.next = 2;
                return this.list();

              case 2:
              case "end":
                return _context.stop();
            }
          }
        }, _callee, this);
      }));

      function componentDidMount() {
        return _componentDidMount.apply(this, arguments);
      }

      return componentDidMount;
    }()
  }, {
    key: "list",
    value: function () {
      var _list = ConfigurationPrompt_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee2() {
        var settings;
        return regeneratorRuntime.wrap(function _callee2$(_context2) {
          while (1) {
            switch (_context2.prev = _context2.next) {
              case 0:
                if (this.props.hideWhenConfigured) {
                  _context2.next = 2;
                  break;
                }

                return _context2.abrupt("return");

              case 2:
                this.onLoading(true);
                _context2.prev = 3;
                _context2.next = 6;
                return this.props.api.settings(this.props.filter);

              case 6:
                settings = _context2.sent;
                this.onLoading(false);
                this.setState({
                  configured: settings.every(function (s) {
                    return s.value !== null;
                  })
                });
                _context2.next = 14;
                break;

              case 11:
                _context2.prev = 11;
                _context2.t0 = _context2["catch"](3);
                this.onError(_context2.t0);

              case 14:
              case "end":
                return _context2.stop();
            }
          }
        }, _callee2, this, [[3, 11]]);
      }));

      function list() {
        return _list.apply(this, arguments);
      }

      return list;
    }()
  }, {
    key: "render",
    value: function render() {
      if (this.props.hideWhenConfigured && this.state.configured) {
        return null;
      }

      return /*#__PURE__*/react.createElement(react.Fragment, null, this.props.children, /*#__PURE__*/react.createElement(Settings, {
        useTitle: this.props.useTitle,
        disableDoc: this.props.disableDoc,
        group: this.props.group,
        api: this.props.api,
        filter: this.props.filter,
        filterable: !!this.props.filterable,
        location: this.props.location,
        history: this.props.history,
        onLoading: this.onLoading,
        onError: this.onError
      }));
    }
  }]);

  return ConfigurationPrompt;
}(react.Component);


;// CONCATENATED MODULE: ./src/components/AfterStreams.js
function AfterStreams_typeof(obj) { "@babel/helpers - typeof"; if (typeof Symbol === "function" && typeof Symbol.iterator === "symbol") { AfterStreams_typeof = function _typeof(obj) { return typeof obj; }; } else { AfterStreams_typeof = function _typeof(obj) { return obj && typeof Symbol === "function" && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; }; } return AfterStreams_typeof(obj); }






function AfterStreams_asyncGeneratorStep(gen, resolve, reject, _next, _throw, key, arg) { try { var info = gen[key](arg); var value = info.value; } catch (error) { reject(error); return; } if (info.done) { resolve(value); } else { Promise.resolve(value).then(_next, _throw); } }

function AfterStreams_asyncToGenerator(fn) { return function () { var self = this, args = arguments; return new Promise(function (resolve, reject) { var gen = fn.apply(self, args); function _next(value) { AfterStreams_asyncGeneratorStep(gen, resolve, reject, _next, _throw, "next", value); } function _throw(err) { AfterStreams_asyncGeneratorStep(gen, resolve, reject, _next, _throw, "throw", err); } _next(undefined); }); }; }

function AfterStreams_classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function AfterStreams_defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function AfterStreams_createClass(Constructor, protoProps, staticProps) { if (protoProps) AfterStreams_defineProperties(Constructor.prototype, protoProps); if (staticProps) AfterStreams_defineProperties(Constructor, staticProps); return Constructor; }

function AfterStreams_inherits(subClass, superClass) { if (typeof superClass !== "function" && superClass !== null) { throw new TypeError("Super expression must either be null or a function"); } subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, writable: true, configurable: true } }); if (superClass) AfterStreams_setPrototypeOf(subClass, superClass); }

function AfterStreams_setPrototypeOf(o, p) { AfterStreams_setPrototypeOf = Object.setPrototypeOf || function _setPrototypeOf(o, p) { o.__proto__ = p; return o; }; return AfterStreams_setPrototypeOf(o, p); }

function AfterStreams_createSuper(Derived) { var hasNativeReflectConstruct = AfterStreams_isNativeReflectConstruct(); return function _createSuperInternal() { var Super = AfterStreams_getPrototypeOf(Derived), result; if (hasNativeReflectConstruct) { var NewTarget = AfterStreams_getPrototypeOf(this).constructor; result = Reflect.construct(Super, arguments, NewTarget); } else { result = Super.apply(this, arguments); } return AfterStreams_possibleConstructorReturn(this, result); }; }

function AfterStreams_possibleConstructorReturn(self, call) { if (call && (AfterStreams_typeof(call) === "object" || typeof call === "function")) { return call; } return AfterStreams_assertThisInitialized(self); }

function AfterStreams_assertThisInitialized(self) { if (self === void 0) { throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); } return self; }

function AfterStreams_isNativeReflectConstruct() { if (typeof Reflect === "undefined" || !Reflect.construct) return false; if (Reflect.construct.sham) return false; if (typeof Proxy === "function") return true; try { Date.prototype.toString.call(Reflect.construct(Date, [], function () {})); return true; } catch (e) { return false; } }

function AfterStreams_getPrototypeOf(o) { AfterStreams_getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf : function _getPrototypeOf(o) { return o.__proto__ || Object.getPrototypeOf(o); }; return AfterStreams_getPrototypeOf(o); }







var AfterStreams = /*#__PURE__*/function (_React$Component) {
  AfterStreams_inherits(AfterStreams, _React$Component);

  var _super = AfterStreams_createSuper(AfterStreams);

  function AfterStreams(props) {
    var _this;

    AfterStreams_classCallCheck(this, AfterStreams);

    _this = _super.call(this, props);
    _this.api = _this.props.api;
    _this.state = {
      loading: false,
      configLoading: false,
      error: null,
      data: null
    };
    return _this;
  }

  AfterStreams_createClass(AfterStreams, [{
    key: "componentDidMount",
    value: function () {
      var _componentDidMount = AfterStreams_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee() {
        return regeneratorRuntime.wrap(function _callee$(_context) {
          while (1) {
            switch (_context.prev = _context.next) {
              case 0:
                _context.next = 2;
                return this.list();

              case 2:
              case "end":
                return _context.stop();
            }
          }
        }, _callee, this);
      }));

      function componentDidMount() {
        return _componentDidMount.apply(this, arguments);
      }

      return componentDidMount;
    }()
    /**
     * Refresh the list of after streams.
     */

  }, {
    key: "list",
    value: function () {
      var _list = AfterStreams_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee2() {
        var data;
        return regeneratorRuntime.wrap(function _callee2$(_context2) {
          while (1) {
            switch (_context2.prev = _context2.next) {
              case 0:
                this.setState({
                  loading: true
                });
                _context2.prev = 1;
                _context2.next = 4;
                return this.api.afterStreams();

              case 4:
                data = _context2.sent;
                this.setState({
                  loading: false,
                  error: null,
                  data: data
                });
                _context2.next = 11;
                break;

              case 8:
                _context2.prev = 8;
                _context2.t0 = _context2["catch"](1);
                this.setState({
                  loading: false,
                  error: "failed to request after streams: ".concat(_context2.t0),
                  data: null
                });

              case 11:
              case "end":
                return _context2.stop();
            }
          }
        }, _callee2, this, [[1, 8]]);
      }));

      function list() {
        return _list.apply(this, arguments);
      }

      return list;
    }()
    /**
     * Delete the given afterstream.
     *
     * @param {number} id afterstream id to delete
     */

  }, {
    key: "delete",
    value: function () {
      var _delete2 = AfterStreams_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee3(id) {
        return regeneratorRuntime.wrap(function _callee3$(_context3) {
          while (1) {
            switch (_context3.prev = _context3.next) {
              case 0:
                _context3.prev = 0;
                _context3.next = 3;
                return this.api.deleteAfterStream(id);

              case 3:
                _context3.next = 5;
                return this.list();

              case 5:
                _context3.next = 10;
                break;

              case 7:
                _context3.prev = 7;
                _context3.t0 = _context3["catch"](0);
                this.setState({
                  loading: false,
                  error: "failed to delete after stream: ".concat(_context3.t0)
                });

              case 10:
              case "end":
                return _context3.stop();
            }
          }
        }, _callee3, this, [[0, 7]]);
      }));

      function _delete(_x) {
        return _delete2.apply(this, arguments);
      }

      return _delete;
    }()
  }, {
    key: "render",
    value: function render() {
      var _this2 = this;

      var content = null;

      if (this.state.data) {
        if (this.state.data.length === 0) {
          content = /*#__PURE__*/react.createElement(Alert/* default */.Z, {
            variant: "info"
          }, "No After Streams!");
        } else {
          content = /*#__PURE__*/react.createElement(Table/* default */.Z, {
            responsive: "sm"
          }, /*#__PURE__*/react.createElement("thead", null, /*#__PURE__*/react.createElement("tr", null, /*#__PURE__*/react.createElement("th", null, "User"), /*#__PURE__*/react.createElement("th", {
            className: "table-fill"
          }, "Message"), /*#__PURE__*/react.createElement("th", null))), /*#__PURE__*/react.createElement("tbody", null, this.state.data.map(function (a, id) {
            return /*#__PURE__*/react.createElement("tr", {
              key: id
            }, /*#__PURE__*/react.createElement("td", {
              className: "afterstream-user"
            }, /*#__PURE__*/react.createElement("a", {
              className: "afterstream-name",
              href: "https://twitch.tv/".concat(a.user)
            }, "@", a.user), /*#__PURE__*/react.createElement("span", {
              className: "afterstream-added-at"
            }, /*#__PURE__*/react.createElement("span", {
              className: "afterstream-at"
            }, "at"), /*#__PURE__*/react.createElement("span", {
              className: "afterstream-datetime datetime"
            }, a.added_at))), /*#__PURE__*/react.createElement("td", null, /*#__PURE__*/react.createElement("code", null, a.text)), /*#__PURE__*/react.createElement("td", null, /*#__PURE__*/react.createElement(Button/* default */.Z, {
              size: "sm",
              variant: "danger",
              className: "action",
              onClick: function onClick() {
                return _this2.delete(a.id);
              }
            }, /*#__PURE__*/react.createElement(index_es/* FontAwesomeIcon */.G, {
              icon: "trash"
            }))));
          })));
        }
      }

      return /*#__PURE__*/react.createElement(react.Fragment, null, /*#__PURE__*/react.createElement("h1", {
        className: "oxi-page-title"
      }, "After Streams"), /*#__PURE__*/react.createElement(Loading, {
        isLoading: this.state.loading || this.state.configLoading
      }), /*#__PURE__*/react.createElement(Error_Loading, {
        error: this.state.error
      }), /*#__PURE__*/react.createElement(ConfigurationPrompt, {
        api: this.api,
        filter: {
          prefix: ["afterstream"]
        },
        onLoading: function onLoading(configLoading) {
          return _this2.setState({
            configLoading: configLoading,
            error: null
          });
        },
        onError: function onError(error) {
          return _this2.setState({
            configLoading: false,
            error: error
          });
        }
      }), content);
    }
  }]);

  return AfterStreams;
}(react.Component);


// EXTERNAL MODULE: ./node_modules/react-websocket/build/index.js
var build = __webpack_require__(35267);
var build_default = /*#__PURE__*/__webpack_require__.n(build);
;// CONCATENATED MODULE: ./src/components/Overlay.js
function Overlay_typeof(obj) { "@babel/helpers - typeof"; if (typeof Symbol === "function" && typeof Symbol.iterator === "symbol") { Overlay_typeof = function _typeof(obj) { return typeof obj; }; } else { Overlay_typeof = function _typeof(obj) { return obj && typeof Symbol === "function" && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; }; } return Overlay_typeof(obj); }





function Overlay_classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function Overlay_defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function Overlay_createClass(Constructor, protoProps, staticProps) { if (protoProps) Overlay_defineProperties(Constructor.prototype, protoProps); if (staticProps) Overlay_defineProperties(Constructor, staticProps); return Constructor; }

function Overlay_inherits(subClass, superClass) { if (typeof superClass !== "function" && superClass !== null) { throw new TypeError("Super expression must either be null or a function"); } subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, writable: true, configurable: true } }); if (superClass) Overlay_setPrototypeOf(subClass, superClass); }

function Overlay_setPrototypeOf(o, p) { Overlay_setPrototypeOf = Object.setPrototypeOf || function _setPrototypeOf(o, p) { o.__proto__ = p; return o; }; return Overlay_setPrototypeOf(o, p); }

function Overlay_createSuper(Derived) { var hasNativeReflectConstruct = Overlay_isNativeReflectConstruct(); return function _createSuperInternal() { var Super = Overlay_getPrototypeOf(Derived), result; if (hasNativeReflectConstruct) { var NewTarget = Overlay_getPrototypeOf(this).constructor; result = Reflect.construct(Super, arguments, NewTarget); } else { result = Super.apply(this, arguments); } return Overlay_possibleConstructorReturn(this, result); }; }

function Overlay_possibleConstructorReturn(self, call) { if (call && (Overlay_typeof(call) === "object" || typeof call === "function")) { return call; } return Overlay_assertThisInitialized(self); }

function Overlay_assertThisInitialized(self) { if (self === void 0) { throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); } return self; }

function Overlay_isNativeReflectConstruct() { if (typeof Reflect === "undefined" || !Reflect.construct) return false; if (Reflect.construct.sham) return false; if (typeof Proxy === "function") return true; try { Date.prototype.toString.call(Reflect.construct(Date, [], function () {})); return true; } catch (e) { return false; } }

function Overlay_getPrototypeOf(o) { Overlay_getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf : function _getPrototypeOf(o) { return o.__proto__ || Object.getPrototypeOf(o); }; return Overlay_getPrototypeOf(o); }




/**
 * Pick the image best suited for album art.
 */

function pickYouTubeAlbumArt(thumbnails, smaller) {
  var smallest = null;

  for (var key in thumbnails) {
    var thumbnail = thumbnails[key];

    if (smallest === null) {
      smallest = thumbnail;
      continue;
    }

    if (smallest.width > thumbnail.width) {
      smallest = thumbnail;
    }
  }

  if (smallest.width > smaller) {
    var factor = smaller / smallest.width;
    smallest.width *= factor;
    smallest.height *= factor;
  }

  return smallest;
}

var CurrentSong = /*#__PURE__*/function (_React$Component) {
  Overlay_inherits(CurrentSong, _React$Component);

  var _super = Overlay_createSuper(CurrentSong);

  function CurrentSong(props) {
    Overlay_classCallCheck(this, CurrentSong);

    return _super.call(this, props);
  }

  Overlay_createClass(CurrentSong, [{
    key: "render",
    value: function render() {
      var requestBy = null;

      if (this.props.requestBy !== null) {
        requestBy = /*#__PURE__*/react.createElement("span", {
          className: "request"
        }, /*#__PURE__*/react.createElement("span", {
          className: "request-by"
        }, "request by"), /*#__PURE__*/react.createElement("span", {
          className: "request-user"
        }, this.props.requestBy));
      }

      var state = null;
      var albumArt = null;

      if (this.props.albumArt) {
        state = /*#__PURE__*/react.createElement("div", {
          className: stateClasses
        });
        albumArt = /*#__PURE__*/react.createElement("img", {
          className: "album-art",
          width: this.props.albumArt.width,
          height: this.props.albumArt.height,
          src: this.props.albumArt.url
        });
      }

      var progressBarStyle = {
        width: "".concat(percentage(this.props.elapsed, this.props.duration), "%")
      };
      var stateClasses = "state";

      if (this.props.isPlaying) {
        stateClasses += " state-playing";
      } else {
        stateClasses += " state-paused";
      }

      var trackName = "Unknown Track";

      if (this.props.track) {
        trackName = this.props.track;
      }

      var artistName = "Unknown Artist";

      if (this.props.artist) {
        artistName = this.props.artist.name;
      }

      return /*#__PURE__*/react.createElement("div", {
        id: "current-song"
      }, /*#__PURE__*/react.createElement("div", {
        className: "album"
      }, state, albumArt), /*#__PURE__*/react.createElement("div", {
        className: "info"
      }, /*#__PURE__*/react.createElement("div", {
        className: "track"
      }, /*#__PURE__*/react.createElement("div", {
        className: "track-name"
      }, trackName)), /*#__PURE__*/react.createElement("div", {
        className: "artist"
      }, /*#__PURE__*/react.createElement("span", {
        className: "artist-name"
      }, artistName), requestBy), /*#__PURE__*/react.createElement("div", {
        className: "progress"
      }, /*#__PURE__*/react.createElement("span", {
        className: "timer"
      }, /*#__PURE__*/react.createElement("span", {
        className: "elapsed"
      }, formatDuration(this.props.elapsed)), /*#__PURE__*/react.createElement("span", null, "/"), /*#__PURE__*/react.createElement("span", {
        className: "duration"
      }, formatDuration(this.props.duration))), /*#__PURE__*/react.createElement("div", {
        className: "progress-bar",
        role: "progressbar",
        "aria-valuenow": "0",
        "aria-valuemin": "0",
        "aria-valuemax": "100",
        style: progressBarStyle
      }))));
    }
  }]);

  return CurrentSong;
}(react.Component);

var Overlay = /*#__PURE__*/function (_React$Component2) {
  Overlay_inherits(Overlay, _React$Component2);

  var _super2 = Overlay_createSuper(Overlay);

  function Overlay(props) {
    var _this;

    Overlay_classCallCheck(this, Overlay);

    _this = _super2.call(this, props);
    _this.state = {
      artist: "Unknown",
      track: null,
      requestBy: null,
      albumArt: null,
      elapsed: 0,
      duration: 0
    };
    return _this;
  }

  Overlay_createClass(Overlay, [{
    key: "handleData",
    value: function handleData(d) {
      var data = null;

      try {
        data = JSON.parse(d);
      } catch (e) {
        console.log("failed to deserialize message");
        return;
      }

      switch (data.type) {
        case "song/current":
          var update = {
            requestBy: data.user,
            elapsed: data.elapsed,
            duration: data.duration
          };

          if (data.track) {
            switch (data.track.type) {
              case "spotify":
                var track = data.track.track;
                update.track = track.name;
                update.artist = pickArtist(track.artists);
                update.albumArt = pickAlbumArt(track.album.images, 64);
                break;

              case "youtube":
                var video = data.track.video;

                if (video.snippet) {
                  update.artist = {
                    name: "channel: ".concat(video.snippet.channelTitle)
                  };
                  update.track = video.snippet.title;
                  update.albumArt = pickYouTubeAlbumArt(video.snippet.thumbnails, 64);
                } else {
                  update.track = null;
                  update.albumArt = null;
                  update.artist = null;
                }

                break;

              default:
                break;
            }
          }

          this.setState(update);
          break;

        case "song/progress":
          this.setState({
            elapsed: data.elapsed,
            duration: data.duration
          });
          break;
      }
    }
  }, {
    key: "render",
    value: function render() {
      return /*#__PURE__*/react.createElement("div", {
        id: "overlay"
      }, /*#__PURE__*/react.createElement((build_default()), {
        url: websocketUrl("ws/overlay"),
        onMessage: this.handleData.bind(this)
      }), /*#__PURE__*/react.createElement(CurrentSong, {
        artist: this.state.artist,
        track: this.state.track,
        requestBy: this.state.requestBy,
        albumArt: this.state.albumArt,
        elapsed: this.state.elapsed,
        duration: this.state.duration
      }));
    }
  }]);

  return Overlay;
}(react.Component);


// EXTERNAL MODULE: ./node_modules/react-bootstrap/esm/Modal.js + 20 modules
var Modal = __webpack_require__(93521);
// EXTERNAL MODULE: ./node_modules/moment/moment.js
var moment = __webpack_require__(30381);
;// CONCATENATED MODULE: ./src/components/Cache.js




















function Cache_createForOfIteratorHelper(o, allowArrayLike) { var it; if (typeof Symbol === "undefined" || o[Symbol.iterator] == null) { if (Array.isArray(o) || (it = Cache_unsupportedIterableToArray(o)) || allowArrayLike && o && typeof o.length === "number") { if (it) o = it; var i = 0; var F = function F() {}; return { s: F, n: function n() { if (i >= o.length) return { done: true }; return { done: false, value: o[i++] }; }, e: function e(_e2) { throw _e2; }, f: F }; } throw new TypeError("Invalid attempt to iterate non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); } var normalCompletion = true, didErr = false, err; return { s: function s() { it = o[Symbol.iterator](); }, n: function n() { var step = it.next(); normalCompletion = step.done; return step; }, e: function e(_e3) { didErr = true; err = _e3; }, f: function f() { try { if (!normalCompletion && it.return != null) it.return(); } finally { if (didErr) throw err; } } }; }

function Cache_typeof(obj) { "@babel/helpers - typeof"; if (typeof Symbol === "function" && typeof Symbol.iterator === "symbol") { Cache_typeof = function _typeof(obj) { return typeof obj; }; } else { Cache_typeof = function _typeof(obj) { return obj && typeof Symbol === "function" && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; }; } return Cache_typeof(obj); }

function Cache_slicedToArray(arr, i) { return Cache_arrayWithHoles(arr) || Cache_iterableToArrayLimit(arr, i) || Cache_unsupportedIterableToArray(arr, i) || Cache_nonIterableRest(); }

function Cache_nonIterableRest() { throw new TypeError("Invalid attempt to destructure non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); }

function Cache_unsupportedIterableToArray(o, minLen) { if (!o) return; if (typeof o === "string") return Cache_arrayLikeToArray(o, minLen); var n = Object.prototype.toString.call(o).slice(8, -1); if (n === "Object" && o.constructor) n = o.constructor.name; if (n === "Map" || n === "Set") return Array.from(o); if (n === "Arguments" || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(n)) return Cache_arrayLikeToArray(o, minLen); }

function Cache_arrayLikeToArray(arr, len) { if (len == null || len > arr.length) len = arr.length; for (var i = 0, arr2 = new Array(len); i < len; i++) { arr2[i] = arr[i]; } return arr2; }

function Cache_iterableToArrayLimit(arr, i) { if (typeof Symbol === "undefined" || !(Symbol.iterator in Object(arr))) return; var _arr = []; var _n = true; var _d = false; var _e = undefined; try { for (var _i = arr[Symbol.iterator](), _s; !(_n = (_s = _i.next()).done); _n = true) { _arr.push(_s.value); if (i && _arr.length === i) break; } } catch (err) { _d = true; _e = err; } finally { try { if (!_n && _i["return"] != null) _i["return"](); } finally { if (_d) throw _e; } } return _arr; }

function Cache_arrayWithHoles(arr) { if (Array.isArray(arr)) return arr; }

function Cache_asyncGeneratorStep(gen, resolve, reject, _next, _throw, key, arg) { try { var info = gen[key](arg); var value = info.value; } catch (error) { reject(error); return; } if (info.done) { resolve(value); } else { Promise.resolve(value).then(_next, _throw); } }

function Cache_asyncToGenerator(fn) { return function () { var self = this, args = arguments; return new Promise(function (resolve, reject) { var gen = fn.apply(self, args); function _next(value) { Cache_asyncGeneratorStep(gen, resolve, reject, _next, _throw, "next", value); } function _throw(err) { Cache_asyncGeneratorStep(gen, resolve, reject, _next, _throw, "throw", err); } _next(undefined); }); }; }

function Cache_classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function Cache_defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function Cache_createClass(Constructor, protoProps, staticProps) { if (protoProps) Cache_defineProperties(Constructor.prototype, protoProps); if (staticProps) Cache_defineProperties(Constructor, staticProps); return Constructor; }

function Cache_inherits(subClass, superClass) { if (typeof superClass !== "function" && superClass !== null) { throw new TypeError("Super expression must either be null or a function"); } subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, writable: true, configurable: true } }); if (superClass) Cache_setPrototypeOf(subClass, superClass); }

function Cache_setPrototypeOf(o, p) { Cache_setPrototypeOf = Object.setPrototypeOf || function _setPrototypeOf(o, p) { o.__proto__ = p; return o; }; return Cache_setPrototypeOf(o, p); }

function Cache_createSuper(Derived) { var hasNativeReflectConstruct = Cache_isNativeReflectConstruct(); return function _createSuperInternal() { var Super = Cache_getPrototypeOf(Derived), result; if (hasNativeReflectConstruct) { var NewTarget = Cache_getPrototypeOf(this).constructor; result = Reflect.construct(Super, arguments, NewTarget); } else { result = Super.apply(this, arguments); } return Cache_possibleConstructorReturn(this, result); }; }

function Cache_possibleConstructorReturn(self, call) { if (call && (Cache_typeof(call) === "object" || typeof call === "function")) { return call; } return Cache_assertThisInitialized(self); }

function Cache_assertThisInitialized(self) { if (self === void 0) { throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); } return self; }

function Cache_isNativeReflectConstruct() { if (typeof Reflect === "undefined" || !Reflect.construct) return false; if (Reflect.construct.sham) return false; if (typeof Proxy === "function") return true; try { Date.prototype.toString.call(Reflect.construct(Date, [], function () {})); return true; } catch (e) { return false; } }

function Cache_getPrototypeOf(o) { Cache_getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf : function _getPrototypeOf(o) { return o.__proto__ || Object.getPrototypeOf(o); }; return Cache_getPrototypeOf(o); }







var Cache = /*#__PURE__*/function (_React$Component) {
  Cache_inherits(Cache, _React$Component);

  var _super = Cache_createSuper(Cache);

  function Cache(props) {
    var _this;

    Cache_classCallCheck(this, Cache);

    _this = _super.call(this, props);
    var filter = "";

    if (_this.props.location) {
      var search = new URLSearchParams(_this.props.location.search);
      filter = search.get("q") || "";
    }

    _this.api = _this.props.api;
    _this.state = {
      loading: false,
      error: null,
      data: null,
      // current filter being applied to filter visible settings.
      filter: filter,
      show: null
    };
    return _this;
  }

  Cache_createClass(Cache, [{
    key: "componentDidMount",
    value: function () {
      var _componentDidMount = Cache_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee() {
        var _this2 = this;

        return regeneratorRuntime.wrap(function _callee$(_context) {
          while (1) {
            switch (_context.prev = _context.next) {
              case 0:
                this.interval = setInterval(function () {
                  return _this2.setState({
                    time: Date.now()
                  });
                }, 1000);
                _context.next = 3;
                return this.list();

              case 3:
              case "end":
                return _context.stop();
            }
          }
        }, _callee, this);
      }));

      function componentDidMount() {
        return _componentDidMount.apply(this, arguments);
      }

      return componentDidMount;
    }()
  }, {
    key: "componentWillUnmount",
    value: function componentWillUnmount() {
      clearInterval(this.interval);
    }
    /**
     * Update the current filter.
     */

  }, {
    key: "setFilter",
    value: function setFilter(filter) {
      if (this.props.location) {
        var path = "".concat(this.props.location.pathname);

        if (!!filter) {
          var search = new URLSearchParams(this.props.location.search);
          search.set("q", filter);
          path = "".concat(path, "?").concat(search);
        }

        this.props.history.replace(path);
      }

      this.setState({
        filter: filter
      });
    }
    /**
     * Refresh the list of settings.
     */

  }, {
    key: "list",
    value: function () {
      var _list = Cache_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee2() {
        var data;
        return regeneratorRuntime.wrap(function _callee2$(_context2) {
          while (1) {
            switch (_context2.prev = _context2.next) {
              case 0:
                this.setState({
                  loading: true
                });
                _context2.prev = 1;
                _context2.next = 4;
                return this.api.cache();

              case 4:
                data = _context2.sent;
                this.setState({
                  loading: false,
                  error: null,
                  data: data
                });
                _context2.next = 11;
                break;

              case 8:
                _context2.prev = 8;
                _context2.t0 = _context2["catch"](1);
                this.setState({
                  loading: false,
                  error: "failed to request cache: ".concat(_context2.t0),
                  data: null
                });

              case 11:
              case "end":
                return _context2.stop();
            }
          }
        }, _callee2, this, [[1, 8]]);
      }));

      function list() {
        return _list.apply(this, arguments);
      }

      return list;
    }()
    /**
     * Remove a cache entry.
     */

  }, {
    key: "cacheDelete",
    value: function () {
      var _cacheDelete = Cache_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee3(key) {
        return regeneratorRuntime.wrap(function _callee3$(_context3) {
          while (1) {
            switch (_context3.prev = _context3.next) {
              case 0:
                this.setState({
                  loading: true
                });
                _context3.prev = 1;
                _context3.next = 4;
                return this.api.cacheDelete(key);

              case 4:
                _context3.next = 6;
                return this.list();

              case 6:
                _context3.next = 11;
                break;

              case 8:
                _context3.prev = 8;
                _context3.t0 = _context3["catch"](1);
                this.setState({
                  loading: false,
                  error: "failed to delete cache entry: ".concat(_context3.t0)
                });

              case 11:
              case "end":
                return _context3.stop();
            }
          }
        }, _callee3, this, [[1, 8]]);
      }));

      function cacheDelete(_x) {
        return _cacheDelete.apply(this, arguments);
      }

      return cacheDelete;
    }()
    /**
     * Filter the data if applicable.
     */

  }, {
    key: "filtered",
    value: function filtered(data) {
      if (!this.state.filter) {
        return data;
      }

      var parts = this.state.filter.split(" ").map(function (f) {
        return f.toLowerCase();
      });
      return data.filter(function (d) {
        return parts.every(function (p) {
          var _d$key = Cache_slicedToArray(d.key, 2),
              ns = _d$key[0],
              key = _d$key[1];

          if (ns !== null && ns.toLowerCase().indexOf(p) != -1) {
            return true;
          }

          if (Cache_typeof(key) !== "object") {
            return false;
          }

          var any = false;

          for (var keyName in key) {
            var v = key[keyName];

            if (typeof v === "string") {
              any = v.toLowerCase().indexOf(p) != -1;

              if (any) {
                break;
              }
            }
          }

          return any;
        });
      });
    }
  }, {
    key: "modal",
    value: function modal(now) {
      var _this3 = this;

      var header = null;
      var body = null;

      if (this.state.show !== null) {
        var _this$state$show = this.state.show,
            key = _this$state$show.key,
            value = _this$state$show.value,
            expires_at = _this$state$show.expires_at;

        var _key = Cache_slicedToArray(key, 2),
            ns = _key[0],
            k = _key[1];

        if (ns !== null) {
          ns = /*#__PURE__*/react.createElement("span", null, /*#__PURE__*/react.createElement("b", null, ns), " \xA0");
        }

        header = /*#__PURE__*/react.createElement("span", null, ns, " ", /*#__PURE__*/react.createElement("code", null, JSON.stringify(k)), " ", this.renderExpiresAt(now, expires_at));
        body = /*#__PURE__*/react.createElement("code", null, /*#__PURE__*/react.createElement("pre", null, JSON.stringify(value, null, 2)));
      }

      var hide = function hide() {
        _this3.setState({
          show: null
        });
      };

      return /*#__PURE__*/react.createElement(Modal/* default */.Z, {
        className: "chat-settings",
        show: this.state.show !== null,
        onHide: hide
      }, /*#__PURE__*/react.createElement(Modal/* default.Header */.Z.Header, null, header), /*#__PURE__*/react.createElement(Modal/* default.Body */.Z.Body, null, body));
    }
  }, {
    key: "groupByNamespace",
    value: function groupByNamespace(data) {
      var def = [];
      var groups = {};

      var _iterator = Cache_createForOfIteratorHelper(data),
          _step;

      try {
        for (_iterator.s(); !(_step = _iterator.n()).done;) {
          var d = _step.value;
          var key = d.key,
              value = d.value;

          var _key2 = Cache_slicedToArray(key, 2),
              ns = _key2[0],
              k = _key2[1];

          if (ns === null) {
            def.push({
              key: k,
              data: d
            });
            continue;
          }

          var group = groups[ns];

          if (!group) {
            groups[ns] = [{
              key: k,
              data: d
            }];
            continue;
          }

          group.push({
            key: k,
            data: d
          });
        }
      } catch (err) {
        _iterator.e(err);
      } finally {
        _iterator.f();
      }

      var order = Object.keys(groups);
      order.sort();
      return {
        def: def,
        groups: groups,
        order: order
      };
    }
    /**
     * Render when a thing expires.
     */

  }, {
    key: "renderExpiresAt",
    value: function renderExpiresAt(now, at) {
      var when = moment(at);
      var diff = moment(when - now);
      return diff.format("D.hh:mm:ss");
    }
    /**
     * Render a single key.
     */

  }, {
    key: "renderKey",
    value: function renderKey(now, i, key, data) {
      var _this4 = this;

      var cacheDelete = function cacheDelete() {
        return _this4.cacheDelete(data.key);
      };

      var show = function show() {
        return _this4.setState({
          show: data
        });
      };

      return /*#__PURE__*/react.createElement("tr", {
        key: i
      }, /*#__PURE__*/react.createElement("td", null, /*#__PURE__*/react.createElement("code", null, JSON.stringify(key))), /*#__PURE__*/react.createElement("td", {
        className: "cache-expires"
      }, this.renderExpiresAt(now, data.expires_at)), /*#__PURE__*/react.createElement("td", {
        width: "1%"
      }, /*#__PURE__*/react.createElement(ButtonGroup/* default */.Z, null, /*#__PURE__*/react.createElement(Button/* default */.Z, {
        variant: "danger",
        onClick: cacheDelete
      }, /*#__PURE__*/react.createElement(index_es/* FontAwesomeIcon */.G, {
        icon: "trash"
      })), /*#__PURE__*/react.createElement(Button/* default */.Z, {
        onClick: show
      }, /*#__PURE__*/react.createElement(index_es/* FontAwesomeIcon */.G, {
        icon: "eye"
      })))));
    }
  }, {
    key: "render",
    value: function render() {
      var _this5 = this;

      var filterOnChange = function filterOnChange(e) {
        return _this5.setFilter(e.target.value);
      };

      var clearFilter = function clearFilter() {
        return _this5.setFilter("");
      };

      var clear = null;

      if (!!this.state.filter) {
        clear = /*#__PURE__*/react.createElement(InputGroup/* default.Append */.Z.Append, null, /*#__PURE__*/react.createElement(Button/* default */.Z, {
          variant: "primary",
          onClick: clearFilter
        }, "Clear Filter"));
      }

      var filter = /*#__PURE__*/react.createElement(Form/* default */.Z, {
        className: "mt-4 mb-4"
      }, /*#__PURE__*/react.createElement(InputGroup/* default */.Z, null, /*#__PURE__*/react.createElement(Form/* default.Control */.Z.Control, {
        value: this.state.filter,
        placeholder: "Search",
        onChange: filterOnChange
      }), clear));
      var now = moment();
      var modal = this.modal(now);
      var content = null;

      if (this.state.data !== null) {
        var data = this.filtered(this.state.data);

        var _this$groupByNamespac = this.groupByNamespace(data),
            def = _this$groupByNamespac.def,
            groups = _this$groupByNamespac.groups,
            order = _this$groupByNamespac.order;

        content = /*#__PURE__*/react.createElement(Table/* default */.Z, null, /*#__PURE__*/react.createElement("thead", null, /*#__PURE__*/react.createElement("tr", null, /*#__PURE__*/react.createElement("th", null, "key"), /*#__PURE__*/react.createElement("th", null, "expires"), /*#__PURE__*/react.createElement("th", null))), /*#__PURE__*/react.createElement("tbody", null, def.map(function (_ref, i) {
          var key = _ref.key,
              data = _ref.data;
          return _this5.renderKey(now, i, key, data);
        })), order.map(function (o) {
          var title = /*#__PURE__*/react.createElement("tbody", {
            key: "title"
          }, /*#__PURE__*/react.createElement("tr", null, /*#__PURE__*/react.createElement("td", {
            className: "cache-namespace-header"
          }, o)));
          var body = /*#__PURE__*/react.createElement("tbody", {
            key: "body"
          }, groups[o].map(function (_ref2, i) {
            var key = _ref2.key,
                data = _ref2.data;
            return _this5.renderKey(now, i, key, data);
          }));
          return [title, body];
        }));
      }

      return /*#__PURE__*/react.createElement("div", {
        className: "cache"
      }, /*#__PURE__*/react.createElement(Loading, {
        isLoading: this.state.loading
      }), /*#__PURE__*/react.createElement(Error_Loading, {
        error: this.state.error
      }), filter, modal, content);
    }
  }]);

  return Cache;
}(react.Component);


;// CONCATENATED MODULE: ./src/components/Modules.js
function Modules_typeof(obj) { "@babel/helpers - typeof"; if (typeof Symbol === "function" && typeof Symbol.iterator === "symbol") { Modules_typeof = function _typeof(obj) { return typeof obj; }; } else { Modules_typeof = function _typeof(obj) { return obj && typeof Symbol === "function" && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; }; } return Modules_typeof(obj); }



function Modules_classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function Modules_defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function Modules_createClass(Constructor, protoProps, staticProps) { if (protoProps) Modules_defineProperties(Constructor.prototype, protoProps); if (staticProps) Modules_defineProperties(Constructor, staticProps); return Constructor; }

function Modules_inherits(subClass, superClass) { if (typeof superClass !== "function" && superClass !== null) { throw new TypeError("Super expression must either be null or a function"); } subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, writable: true, configurable: true } }); if (superClass) Modules_setPrototypeOf(subClass, superClass); }

function Modules_setPrototypeOf(o, p) { Modules_setPrototypeOf = Object.setPrototypeOf || function _setPrototypeOf(o, p) { o.__proto__ = p; return o; }; return Modules_setPrototypeOf(o, p); }

function Modules_createSuper(Derived) { var hasNativeReflectConstruct = Modules_isNativeReflectConstruct(); return function _createSuperInternal() { var Super = Modules_getPrototypeOf(Derived), result; if (hasNativeReflectConstruct) { var NewTarget = Modules_getPrototypeOf(this).constructor; result = Reflect.construct(Super, arguments, NewTarget); } else { result = Super.apply(this, arguments); } return Modules_possibleConstructorReturn(this, result); }; }

function Modules_possibleConstructorReturn(self, call) { if (call && (Modules_typeof(call) === "object" || typeof call === "function")) { return call; } return Modules_assertThisInitialized(self); }

function Modules_assertThisInitialized(self) { if (self === void 0) { throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); } return self; }

function Modules_isNativeReflectConstruct() { if (typeof Reflect === "undefined" || !Reflect.construct) return false; if (Reflect.construct.sham) return false; if (typeof Proxy === "function") return true; try { Date.prototype.toString.call(Reflect.construct(Date, [], function () {})); return true; } catch (e) { return false; } }

function Modules_getPrototypeOf(o) { Modules_getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf : function _getPrototypeOf(o) { return o.__proto__ || Object.getPrototypeOf(o); }; return Modules_getPrototypeOf(o); }

function Modules_extends() { Modules_extends = Object.assign || function (target) { for (var i = 1; i < arguments.length; i++) { var source = arguments[i]; for (var key in source) { if (Object.prototype.hasOwnProperty.call(source, key)) { target[key] = source[key]; } } } return target; }; return Modules_extends.apply(this, arguments); }







function Remote(props) {
  return /*#__PURE__*/react.createElement("div", null, /*#__PURE__*/react.createElement("h3", null, "Remote connection to ", /*#__PURE__*/react.createElement("b", null, "setbac.tv")), /*#__PURE__*/react.createElement("p", null, "Handles connections to remote services."), /*#__PURE__*/react.createElement("h4", null, "Connections"), /*#__PURE__*/react.createElement(Connections, {
    api: props.api
  }), /*#__PURE__*/react.createElement("h4", null, "Configuration"), /*#__PURE__*/react.createElement(ConfigurationPrompt, Modules_extends({
    group: true,
    filterable: true,
    filter: {
      prefix: ["remote"]
    }
  }, props)));
}

function Player(props) {
  return /*#__PURE__*/react.createElement("div", null, /*#__PURE__*/react.createElement("h3", null, "Music Player"), /*#__PURE__*/react.createElement("p", null, "Handles playing music and taking song requests in Oxidize Bot."), /*#__PURE__*/react.createElement(ConfigurationPrompt, Modules_extends({
    group: true,
    filterable: true,
    filter: {
      prefix: ["player", "song"]
    }
  }, props)));
}

function Gtav(props) {
  return /*#__PURE__*/react.createElement("div", null, /*#__PURE__*/react.createElement("h3", null, "ChaosMod"), /*#__PURE__*/react.createElement("p", null, /*#__PURE__*/react.createElement("a", {
    href: "https://github.com/udoprog/ChaosMod"
  }, "ChaosMod"), " is a mod for GTA V that allows viewers to interact with your game."), /*#__PURE__*/react.createElement(ConfigurationPrompt, Modules_extends({
    group: true,
    filterable: true,
    filter: {
      prefix: ["gtav"]
    }
  }, props)));
}

function Currency(props) {
  return /*#__PURE__*/react.createElement("div", null, /*#__PURE__*/react.createElement("h3", null, "Stream Currency"), /*#__PURE__*/react.createElement("p", null, "A stream currency is a kind of loyalty points system. It integrated with many other components and can be configured to reward viewers for watching, requesting songs, or other activities."), /*#__PURE__*/react.createElement(ConfigurationPrompt, Modules_extends({
    group: true,
    filterable: true,
    filter: {
      prefix: ["currency"]
    }
  }, props)));
}

function ChatLog(props) {
  return /*#__PURE__*/react.createElement("div", null, /*#__PURE__*/react.createElement("h3", null, "Chat Log"), /*#__PURE__*/react.createElement("p", null, "Experimental Chat Log Support"), /*#__PURE__*/react.createElement(ConfigurationPrompt, Modules_extends({
    group: true,
    filter: {
      prefix: ["chat-log"]
    }
  }, props)));
}

function Index(props) {
  return /*#__PURE__*/react.createElement("div", null, /*#__PURE__*/react.createElement("p", null, "This section contains a list of all features that can be toggled on or off. Each feature might have more settings. If so, they are detailed to the left."), /*#__PURE__*/react.createElement(ConfigurationPrompt, Modules_extends({
    useTitle: true,
    filterable: true,
    filter: {
      feature: true
    }
  }, props)));
}

var Modules = /*#__PURE__*/function (_React$Component) {
  Modules_inherits(Modules, _React$Component);

  var _super = Modules_createSuper(Modules);

  function Modules(props) {
    Modules_classCallCheck(this, Modules);

    return _super.call(this, props);
  }

  Modules_createClass(Modules, [{
    key: "render",
    value: function render() {
      var _this = this;

      var path = this.props.location.pathname;
      return /*#__PURE__*/react.createElement(Row/* default */.Z, null, /*#__PURE__*/react.createElement(Col/* default */.Z, {
        sm: "2"
      }, /*#__PURE__*/react.createElement(Nav/* default */.Z, {
        className: "flex-column",
        variant: "pills"
      }, /*#__PURE__*/react.createElement(Nav/* default.Link */.Z.Link, {
        as: react_router_dom/* Link */.rU,
        active: path === "/modules/remote",
        to: "/modules/remote"
      }, /*#__PURE__*/react.createElement("b", null, "setbac.tv")), /*#__PURE__*/react.createElement(Nav/* default.Link */.Z.Link, {
        as: react_router_dom/* Link */.rU,
        active: path === "/modules/player",
        to: "/modules/player"
      }, "Music Player"), /*#__PURE__*/react.createElement(Nav/* default.Link */.Z.Link, {
        as: react_router_dom/* Link */.rU,
        active: path === "/modules/currency",
        to: "/modules/currency"
      }, "Stream Currency"), /*#__PURE__*/react.createElement(Nav/* default.Link */.Z.Link, {
        as: react_router_dom/* Link */.rU,
        active: path === "/modules/chat-log",
        to: "/modules/chat-log"
      }, "Chat Log"), /*#__PURE__*/react.createElement(Nav/* default.Link */.Z.Link, {
        as: react_router_dom/* Link */.rU,
        active: path === "/modules/gtav",
        to: "/modules/gtav"
      }, "ChaosMod"))), /*#__PURE__*/react.createElement(Col/* default */.Z, null, /*#__PURE__*/react.createElement(react_router/* Route */.AW, {
        path: "/modules",
        exact: true,
        render: function render(props) {
          return /*#__PURE__*/react.createElement(Index, Modules_extends({
            api: _this.props.api
          }, props));
        }
      }), /*#__PURE__*/react.createElement(react_router/* Route */.AW, {
        path: "/modules/remote",
        render: function render(props) {
          return /*#__PURE__*/react.createElement(Remote, Modules_extends({
            api: _this.props.api
          }, props));
        }
      }), /*#__PURE__*/react.createElement(react_router/* Route */.AW, {
        path: "/modules/player",
        render: function render(props) {
          return /*#__PURE__*/react.createElement(Player, Modules_extends({
            api: _this.props.api
          }, props));
        }
      }), /*#__PURE__*/react.createElement(react_router/* Route */.AW, {
        path: "/modules/currency",
        render: function render(props) {
          return /*#__PURE__*/react.createElement(Currency, Modules_extends({
            api: _this.props.api
          }, props));
        }
      }), /*#__PURE__*/react.createElement(react_router/* Route */.AW, {
        path: "/modules/chat-log",
        render: function render(props) {
          return /*#__PURE__*/react.createElement(ChatLog, Modules_extends({
            api: _this.props.api
          }, props));
        }
      }), /*#__PURE__*/react.createElement(react_router/* Route */.AW, {
        path: "/modules/gtav",
        render: function render(props) {
          return /*#__PURE__*/react.createElement(Gtav, Modules_extends({
            api: _this.props.api
          }, props));
        }
      })));
    }
  }]);

  return Modules;
}(react.Component);


// EXTERNAL MODULE: ./node_modules/core-js/modules/es.string.trim.js
var es_string_trim = __webpack_require__(73210);
// EXTERNAL MODULE: ./node_modules/react-bootstrap/esm/Card.js + 1 modules
var Card = __webpack_require__(15881);
;// CONCATENATED MODULE: ./src/components/ImportExport.js
function ImportExport_typeof(obj) { "@babel/helpers - typeof"; if (typeof Symbol === "function" && typeof Symbol.iterator === "symbol") { ImportExport_typeof = function _typeof(obj) { return typeof obj; }; } else { ImportExport_typeof = function _typeof(obj) { return obj && typeof Symbol === "function" && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; }; } return ImportExport_typeof(obj); }










function ImportExport_createForOfIteratorHelper(o, allowArrayLike) { var it; if (typeof Symbol === "undefined" || o[Symbol.iterator] == null) { if (Array.isArray(o) || (it = ImportExport_unsupportedIterableToArray(o)) || allowArrayLike && o && typeof o.length === "number") { if (it) o = it; var i = 0; var F = function F() {}; return { s: F, n: function n() { if (i >= o.length) return { done: true }; return { done: false, value: o[i++] }; }, e: function e(_e) { throw _e; }, f: F }; } throw new TypeError("Invalid attempt to iterate non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); } var normalCompletion = true, didErr = false, err; return { s: function s() { it = o[Symbol.iterator](); }, n: function n() { var step = it.next(); normalCompletion = step.done; return step; }, e: function e(_e2) { didErr = true; err = _e2; }, f: function f() { try { if (!normalCompletion && it.return != null) it.return(); } finally { if (didErr) throw err; } } }; }

function ImportExport_unsupportedIterableToArray(o, minLen) { if (!o) return; if (typeof o === "string") return ImportExport_arrayLikeToArray(o, minLen); var n = Object.prototype.toString.call(o).slice(8, -1); if (n === "Object" && o.constructor) n = o.constructor.name; if (n === "Map" || n === "Set") return Array.from(o); if (n === "Arguments" || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(n)) return ImportExport_arrayLikeToArray(o, minLen); }

function ImportExport_arrayLikeToArray(arr, len) { if (len == null || len > arr.length) len = arr.length; for (var i = 0, arr2 = new Array(len); i < len; i++) { arr2[i] = arr[i]; } return arr2; }

function ImportExport_asyncGeneratorStep(gen, resolve, reject, _next, _throw, key, arg) { try { var info = gen[key](arg); var value = info.value; } catch (error) { reject(error); return; } if (info.done) { resolve(value); } else { Promise.resolve(value).then(_next, _throw); } }

function ImportExport_asyncToGenerator(fn) { return function () { var self = this, args = arguments; return new Promise(function (resolve, reject) { var gen = fn.apply(self, args); function _next(value) { ImportExport_asyncGeneratorStep(gen, resolve, reject, _next, _throw, "next", value); } function _throw(err) { ImportExport_asyncGeneratorStep(gen, resolve, reject, _next, _throw, "throw", err); } _next(undefined); }); }; }

function ImportExport_extends() { ImportExport_extends = Object.assign || function (target) { for (var i = 1; i < arguments.length; i++) { var source = arguments[i]; for (var key in source) { if (Object.prototype.hasOwnProperty.call(source, key)) { target[key] = source[key]; } } } return target; }; return ImportExport_extends.apply(this, arguments); }

function ImportExport_classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function ImportExport_defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function ImportExport_createClass(Constructor, protoProps, staticProps) { if (protoProps) ImportExport_defineProperties(Constructor.prototype, protoProps); if (staticProps) ImportExport_defineProperties(Constructor, staticProps); return Constructor; }

function ImportExport_inherits(subClass, superClass) { if (typeof superClass !== "function" && superClass !== null) { throw new TypeError("Super expression must either be null or a function"); } subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, writable: true, configurable: true } }); if (superClass) ImportExport_setPrototypeOf(subClass, superClass); }

function ImportExport_setPrototypeOf(o, p) { ImportExport_setPrototypeOf = Object.setPrototypeOf || function _setPrototypeOf(o, p) { o.__proto__ = p; return o; }; return ImportExport_setPrototypeOf(o, p); }

function ImportExport_createSuper(Derived) { var hasNativeReflectConstruct = ImportExport_isNativeReflectConstruct(); return function _createSuperInternal() { var Super = ImportExport_getPrototypeOf(Derived), result; if (hasNativeReflectConstruct) { var NewTarget = ImportExport_getPrototypeOf(this).constructor; result = Reflect.construct(Super, arguments, NewTarget); } else { result = Super.apply(this, arguments); } return ImportExport_possibleConstructorReturn(this, result); }; }

function ImportExport_possibleConstructorReturn(self, call) { if (call && (ImportExport_typeof(call) === "object" || typeof call === "function")) { return call; } return ImportExport_assertThisInitialized(self); }

function ImportExport_assertThisInitialized(self) { if (self === void 0) { throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); } return self; }

function ImportExport_isNativeReflectConstruct() { if (typeof Reflect === "undefined" || !Reflect.construct) return false; if (Reflect.construct.sham) return false; if (typeof Proxy === "function") return true; try { Date.prototype.toString.call(Reflect.construct(Date, [], function () {})); return true; } catch (e) { return false; } }

function ImportExport_getPrototypeOf(o) { ImportExport_getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf : function _getPrototypeOf(o) { return o.__proto__ || Object.getPrototypeOf(o); }; return ImportExport_getPrototypeOf(o); }








var ImportExport = /*#__PURE__*/function (_React$Component) {
  ImportExport_inherits(ImportExport, _React$Component);

  var _super = ImportExport_createSuper(ImportExport);

  function ImportExport(props) {
    ImportExport_classCallCheck(this, ImportExport);

    return _super.call(this, props);
  }

  ImportExport_createClass(ImportExport, [{
    key: "render",
    value: function render() {
      var _this = this;

      var path = this.props.location.pathname;
      return /*#__PURE__*/react.createElement(Row/* default */.Z, null, /*#__PURE__*/react.createElement(Col/* default */.Z, {
        sm: "2"
      }, /*#__PURE__*/react.createElement(Nav/* default */.Z, {
        className: "flex-column",
        variant: "pills"
      }, /*#__PURE__*/react.createElement(Nav/* default.Link */.Z.Link, {
        as: react_router_dom/* Link */.rU,
        active: path === "/import-export/phantombot",
        to: "/import-export/phantombot"
      }, "PhantomBot"), /*#__PURE__*/react.createElement(Nav/* default.Link */.Z.Link, {
        as: react_router_dom/* Link */.rU,
        active: path === "/import-export/drangrybot",
        to: "/import-export/drangrybot"
      }, "DrangryBot"))), /*#__PURE__*/react.createElement(Col/* default */.Z, null, /*#__PURE__*/react.createElement(react_router/* Route */.AW, {
        path: "/import-export",
        exact: true,
        render: function render(props) {
          return /*#__PURE__*/react.createElement(ImportExport_Index, props);
        }
      }), /*#__PURE__*/react.createElement(react_router/* Route */.AW, {
        path: "/import-export/phantombot",
        render: function render(props) {
          return /*#__PURE__*/react.createElement(PhantomBot, ImportExport_extends({
            api: _this.props.api
          }, props));
        }
      }), /*#__PURE__*/react.createElement(react_router/* Route */.AW, {
        path: "/import-export/drangrybot",
        render: function render(props) {
          return /*#__PURE__*/react.createElement(DrangryBot, ImportExport_extends({
            api: _this.props.api
          }, props));
        }
      })));
    }
  }]);

  return ImportExport;
}(react.Component);



var ImportExport_Index = /*#__PURE__*/function (_React$Component2) {
  ImportExport_inherits(Index, _React$Component2);

  var _super2 = ImportExport_createSuper(Index);

  function Index(props) {
    ImportExport_classCallCheck(this, Index);

    return _super2.call(this, props);
  }

  ImportExport_createClass(Index, [{
    key: "render",
    value: function render() {
      return /*#__PURE__*/react.createElement("div", null, /*#__PURE__*/react.createElement("h2", null, "Import / Export modules for OxidizeBot"), /*#__PURE__*/react.createElement("p", null, "In here you'll find modules for importing and exporting data to third party systems."));
    }
  }]);

  return Index;
}(react.Component);

var PhantomBot = /*#__PURE__*/function (_React$Component3) {
  ImportExport_inherits(PhantomBot, _React$Component3);

  var _super3 = ImportExport_createSuper(PhantomBot);

  function PhantomBot(props) {
    var _this2;

    ImportExport_classCallCheck(this, PhantomBot);

    _this2 = _super3.call(this, props);
    _this2.api = _this2.props.api;
    _this2.state = {
      loading: false,
      error: null
    };
    return _this2;
  }

  ImportExport_createClass(PhantomBot, [{
    key: "exportCsv",
    value: function () {
      var _exportCsv = ImportExport_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee(e) {
        var balances, balancesCsv, _iterator, _step, balance;

        return regeneratorRuntime.wrap(function _callee$(_context) {
          while (1) {
            switch (_context.prev = _context.next) {
              case 0:
                e.preventDefault();
                _context.next = 3;
                return this.api.exportBalances();

              case 3:
                balances = _context.sent;
                balancesCsv = "";
                _iterator = ImportExport_createForOfIteratorHelper(balances);

                try {
                  for (_iterator.s(); !(_step = _iterator.n()).done;) {
                    balance = _step.value;
                    balancesCsv += ",".concat(balance.user, ",").concat(balance.amount, "\r\n");
                  }
                } catch (err) {
                  _iterator.e(err);
                } finally {
                  _iterator.f();
                }

                download("text/plain", balancesCsv, "balances.csv");
                return _context.abrupt("return", false);

              case 9:
              case "end":
                return _context.stop();
            }
          }
        }, _callee, this);
      }));

      function exportCsv(_x) {
        return _exportCsv.apply(this, arguments);
      }

      return exportCsv;
    }()
  }, {
    key: "render",
    value: function render() {
      var _this3 = this;

      return /*#__PURE__*/react.createElement(react.Fragment, null, /*#__PURE__*/react.createElement("div", {
        className: "mb-3"
      }, /*#__PURE__*/react.createElement("h2", null, "PhantomBot"), /*#__PURE__*/react.createElement("p", null, "Site: ", /*#__PURE__*/react.createElement("a", {
        href: "https://phantombot.tv"
      }, "phantombot.tv")), /*#__PURE__*/react.createElement(Card/* default */.Z, null, /*#__PURE__*/react.createElement(Card/* default.Body */.Z.Body, null, /*#__PURE__*/react.createElement("blockquote", {
        className: "blockquote mb-0"
      }, "PhantomBot is an actively developed open source interactive Twitch bot with a vibrant community that provides entertainment and moderation for your channel, allowing you to focus on what matters the most to you - your game and your viewers.")))), /*#__PURE__*/react.createElement(Row/* default */.Z, null, /*#__PURE__*/react.createElement(Col/* default */.Z, null, /*#__PURE__*/react.createElement("h4", null, "Import"), /*#__PURE__*/react.createElement(PhantomBotImportCsvForm, {
        api: this.api
      })), /*#__PURE__*/react.createElement(Col/* default */.Z, null, /*#__PURE__*/react.createElement("h4", null, "Export"), /*#__PURE__*/react.createElement(Form/* default */.Z, {
        onSubmit: function onSubmit(e) {
          return _this3.exportCsv(e);
        }
      }, /*#__PURE__*/react.createElement(Button/* default */.Z, {
        type: "submit"
      }, "Export to File")))));
    }
  }]);

  return PhantomBot;
}(react.Component);

var DrangryBot = /*#__PURE__*/function (_React$Component4) {
  ImportExport_inherits(DrangryBot, _React$Component4);

  var _super4 = ImportExport_createSuper(DrangryBot);

  function DrangryBot(props) {
    var _this4;

    ImportExport_classCallCheck(this, DrangryBot);

    _this4 = _super4.call(this, props);
    _this4.api = _this4.props.api;
    _this4.state = {
      loading: false,
      error: null
    };
    return _this4;
  }

  ImportExport_createClass(DrangryBot, [{
    key: "exportScv",
    value: function () {
      var _exportScv = ImportExport_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee2(e) {
        var balances, balancesCsv, _iterator2, _step2, balance;

        return regeneratorRuntime.wrap(function _callee2$(_context2) {
          while (1) {
            switch (_context2.prev = _context2.next) {
              case 0:
                e.preventDefault();
                _context2.next = 3;
                return this.api.exportBalances();

              case 3:
                balances = _context2.sent;
                balancesCsv = "Name,Balance,TimeInSeconds\r\n";
                _iterator2 = ImportExport_createForOfIteratorHelper(balances);

                try {
                  for (_iterator2.s(); !(_step2 = _iterator2.n()).done;) {
                    balance = _step2.value;
                    balancesCsv += "".concat(balance.user, ",").concat(balance.amount, ",").concat(balance.watch_time, "\r\n");
                  }
                } catch (err) {
                  _iterator2.e(err);
                } finally {
                  _iterator2.f();
                }

                download("text/plain", balancesCsv, "balances.csv");
                return _context2.abrupt("return", false);

              case 9:
              case "end":
                return _context2.stop();
            }
          }
        }, _callee2, this);
      }));

      function exportScv(_x2) {
        return _exportScv.apply(this, arguments);
      }

      return exportScv;
    }()
  }, {
    key: "render",
    value: function render() {
      var _this5 = this;

      return /*#__PURE__*/react.createElement(react.Fragment, null, /*#__PURE__*/react.createElement("div", {
        className: "mb-3"
      }, /*#__PURE__*/react.createElement("h2", null, "DrangryBot"), /*#__PURE__*/react.createElement("p", null, "Site: ", /*#__PURE__*/react.createElement("a", {
        href: "https://drangrybot.tv"
      }, "drangrybot.tv")), /*#__PURE__*/react.createElement(Card/* default */.Z, null, /*#__PURE__*/react.createElement(Card/* default.Body */.Z.Body, null, /*#__PURE__*/react.createElement("blockquote", {
        className: "blockquote mb-0"
      }, "It is your all-in-one solution to enhance your Twitch channel.")))), /*#__PURE__*/react.createElement(Row/* default */.Z, null, /*#__PURE__*/react.createElement(Col/* default */.Z, null, /*#__PURE__*/react.createElement("h4", null, "Import"), /*#__PURE__*/react.createElement(DrangryBotImportCsvForm, {
        api: this.api
      })), /*#__PURE__*/react.createElement(Col/* default */.Z, null, /*#__PURE__*/react.createElement("h4", null, "Export"), /*#__PURE__*/react.createElement(Form/* default */.Z, {
        onSubmit: function onSubmit(e) {
          return _this5.exportScv(e);
        }
      }, /*#__PURE__*/react.createElement(Button/* default */.Z, {
        type: "submit"
      }, "Export to File")))));
    }
  }]);

  return DrangryBot;
}(react.Component);

var DrangryBotImportCsvForm = /*#__PURE__*/function (_React$Component5) {
  ImportExport_inherits(DrangryBotImportCsvForm, _React$Component5);

  var _super5 = ImportExport_createSuper(DrangryBotImportCsvForm);

  function DrangryBotImportCsvForm(props) {
    var _this6;

    ImportExport_classCallCheck(this, DrangryBotImportCsvForm);

    _this6 = _super5.call(this, props);
    _this6.api = _this6.props.api;
    _this6.state = {
      loading: false,
      success: null,
      error: null,
      channel: "",
      text: "Name,Balance,TimeInSeconds\r\nsetbac,1000,3600",
      errors: {
        channel: "",
        text: ""
      }
    };
    return _this6;
  }
  /**
   * Convert PhantomBot CSV to JSON.
   *
   * @param {string} text the text to convert.
   */


  ImportExport_createClass(DrangryBotImportCsvForm, [{
    key: "convertJson",
    value: function convertJson(text) {
      var json = [];

      if (!this.state.channel) {
        this.setState({
          errors: {
            channel: "Channel must be specified"
          }
        });
        throw new Error("Channel must be specified");
      }

      var document = importCsv(text);

      var _iterator3 = ImportExport_createForOfIteratorHelper(document),
          _step3;

      try {
        for (_iterator3.s(); !(_step3 = _iterator3.n()).done;) {
          var line = _step3.value;
          var user = line["Name"];
          var amount = parseInt(line["Balance"].trim());
          var watch_time = parseInt(line["TimeInSeconds"].trim());
          json.push({
            channel: this.state.channel,
            user: user,
            amount: amount,
            watch_time: watch_time
          });
        }
      } catch (err) {
        _iterator3.e(err);
      } finally {
        _iterator3.f();
      }

      return json;
    }
    /**
     * Import PhantomBot CSV to OxidizeBot.
     *
     * @param {*} e the event being handled.
     */

  }, {
    key: "import",
    value: function () {
      var _import2 = ImportExport_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee3(e) {
        var json;
        return regeneratorRuntime.wrap(function _callee3$(_context3) {
          while (1) {
            switch (_context3.prev = _context3.next) {
              case 0:
                this.setState({
                  errors: {}
                });
                e.preventDefault();
                json = [];
                _context3.prev = 3;
                json = this.convertJson(this.state.text);
                _context3.next = 11;
                break;

              case 7:
                _context3.prev = 7;
                _context3.t0 = _context3["catch"](3);
                console.log(_context3.t0);
                return _context3.abrupt("return");

              case 11:
                this.setState({
                  loading: true
                });
                _context3.prev = 12;
                _context3.next = 15;
                return this.api.importBalances(json);

              case 15:
                this.setState({
                  loading: false,
                  error: null,
                  success: "Successfully imported balances!"
                });
                _context3.next = 21;
                break;

              case 18:
                _context3.prev = 18;
                _context3.t1 = _context3["catch"](12);
                this.setState({
                  loading: false,
                  error: "Failed to import balances: ".concat(_context3.t1),
                  success: null
                });

              case 21:
              case "end":
                return _context3.stop();
            }
          }
        }, _callee3, this, [[3, 7], [12, 18]]);
      }));

      function _import(_x3) {
        return _import2.apply(this, arguments);
      }

      return _import;
    }()
  }, {
    key: "handleChannelChange",
    value: function handleChannelChange(e) {
      this.setState({
        channel: e.target.value
      });
    }
  }, {
    key: "handleTextChange",
    value: function handleTextChange(e) {
      this.setState({
        text: e.target.value
      });
    }
  }, {
    key: "render",
    value: function render() {
      var _this7 = this;

      var message = null;

      if (!!this.state.success) {
        message = /*#__PURE__*/react.createElement(Alert/* default */.Z, {
          variant: "info"
        }, this.state.success);
      }

      if (!!this.state.error) {
        message = /*#__PURE__*/react.createElement(Alert/* default */.Z, {
          variant: "danger"
        }, this.state.error);
      }

      var channelError = null;

      if (!!this.state.errors.channel) {
        channelError = /*#__PURE__*/react.createElement(Form/* default.Control.Feedback */.Z.Control.Feedback, {
          type: "invalid"
        }, this.state.errors.channel);
      }

      var textError = null;

      if (!!this.state.errors.text) {
        textError = /*#__PURE__*/react.createElement(Form/* default.Control.Feedback */.Z.Control.Feedback, {
          type: "invalid"
        }, this.state.errors.text);
      }

      return /*#__PURE__*/react.createElement("div", null, message, /*#__PURE__*/react.createElement(Form/* default */.Z, {
        onSubmit: function onSubmit(e) {
          return _this7.import(e);
        },
        disabled: this.state.loading
      }, /*#__PURE__*/react.createElement(Form/* default.Group */.Z.Group, {
        id: "channel"
      }, /*#__PURE__*/react.createElement(Form/* default.Label */.Z.Label, null, "Channel"), /*#__PURE__*/react.createElement(Form/* default.Control */.Z.Control, {
        disabled: this.state.loading,
        isInvalid: !!this.state.errors.channel,
        value: this.state.channel,
        onChange: function onChange(e) {
          return _this7.handleChannelChange(e);
        },
        placeholder: "#setbac"
      }), channelError, /*#__PURE__*/react.createElement(Form/* default.Text */.Z.Text, null, "Name of channel to import balances for. Like ", /*#__PURE__*/react.createElement("b", null, "#setbac"), ".")), /*#__PURE__*/react.createElement(Form/* default.Group */.Z.Group, {
        id: "content"
      }, /*#__PURE__*/react.createElement(Form/* default.Control */.Z.Control, {
        as: "textarea",
        rows: "10",
        disabled: this.state.loading,
        isInvalid: !!this.state.errors.text,
        value: this.state.text,
        onChange: function onChange(e) {
          return _this7.handleTextChange(e);
        }
      }), textError, /*#__PURE__*/react.createElement(Form/* default.Text */.Z.Text, null, "Balances to import. Each line should be ", /*#__PURE__*/react.createElement("code", null, "name,balance,watch_time"), ".")), /*#__PURE__*/react.createElement(Button/* default */.Z, {
        variant: "primary",
        type: "submit",
        disabled: this.state.loading
      }, "Import"), /*#__PURE__*/react.createElement(Loading, {
        isLoading: this.state.loading
      })));
    }
  }]);

  return DrangryBotImportCsvForm;
}(react.Component);

var PhantomBotImportCsvForm = /*#__PURE__*/function (_React$Component6) {
  ImportExport_inherits(PhantomBotImportCsvForm, _React$Component6);

  var _super6 = ImportExport_createSuper(PhantomBotImportCsvForm);

  function PhantomBotImportCsvForm(props) {
    var _this8;

    ImportExport_classCallCheck(this, PhantomBotImportCsvForm);

    _this8 = _super6.call(this, props);
    _this8.api = _this8.props.api;
    _this8.state = {
      loading: false,
      success: null,
      error: null,
      channel: "",
      text: "",
      errors: {
        channel: "",
        text: ""
      }
    };
    return _this8;
  }
  /**
   * Convert PhantomBot CSV to JSON.
   *
   * @param {string} text the text to convert.
   */


  ImportExport_createClass(PhantomBotImportCsvForm, [{
    key: "convertJson",
    value: function convertJson(text) {
      var json = [];

      if (!this.state.channel) {
        this.setState({
          errors: {
            channel: "Channel must be specified"
          }
        });
        throw new Error("Channel must be specified");
      }

      var _iterator4 = ImportExport_createForOfIteratorHelper(text.split("\n")),
          _step4;

      try {
        for (_iterator4.s(); !(_step4 = _iterator4.n()).done;) {
          var line = _step4.value;
          line = line.trim();

          if (line === "") {
            continue;
          }

          var cols = line.split(",");

          if (cols.length !== 3) {
            this.setState({
              errors: {
                text: "expected 3 columns but got: ".concat(line)
              }
            });
            throw new Error("expected 3 columns but got: ".concat(line));
          }

          var user = cols[1].trim();
          var amountText = cols[2].trim();

          if (amountText === "null") {
            continue;
          }

          var amount = 0;

          try {
            amount = parseInt(amountText);
          } catch (_unused) {
            throw new Error("expected numeric third column on line: ".concat(line));
          }

          json.push({
            channel: this.state.channel,
            user: user,
            amount: amount
          });
        }
      } catch (err) {
        _iterator4.e(err);
      } finally {
        _iterator4.f();
      }

      return json;
    }
    /**
     * Import PhantomBot CSV to OxidizeBot.
     *
     * @param {*} e the event being handled.
     */

  }, {
    key: "import",
    value: function () {
      var _import3 = ImportExport_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee4(e) {
        var json;
        return regeneratorRuntime.wrap(function _callee4$(_context4) {
          while (1) {
            switch (_context4.prev = _context4.next) {
              case 0:
                this.setState({
                  errors: {}
                });
                e.preventDefault();
                json = [];
                _context4.prev = 3;
                json = this.convertJson(this.state.text);
                _context4.next = 11;
                break;

              case 7:
                _context4.prev = 7;
                _context4.t0 = _context4["catch"](3);
                console.log(_context4.t0);
                return _context4.abrupt("return");

              case 11:
                this.setState({
                  loading: true
                });
                _context4.prev = 12;
                _context4.next = 15;
                return this.api.importBalances(json);

              case 15:
                this.setState({
                  loading: false,
                  error: null,
                  success: "Successfully imported balances!"
                });
                _context4.next = 21;
                break;

              case 18:
                _context4.prev = 18;
                _context4.t1 = _context4["catch"](12);
                this.setState({
                  loading: false,
                  error: "Failed to import balances: ".concat(_context4.t1),
                  success: null
                });

              case 21:
              case "end":
                return _context4.stop();
            }
          }
        }, _callee4, this, [[3, 7], [12, 18]]);
      }));

      function _import(_x4) {
        return _import3.apply(this, arguments);
      }

      return _import;
    }()
  }, {
    key: "handleChannelChange",
    value: function handleChannelChange(e) {
      this.setState({
        channel: e.target.value
      });
    }
  }, {
    key: "handleTextChange",
    value: function handleTextChange(e) {
      this.setState({
        text: e.target.value
      });
    }
  }, {
    key: "render",
    value: function render() {
      var _this9 = this;

      var message = null;

      if (!!this.state.success) {
        message = /*#__PURE__*/react.createElement(Alert/* default */.Z, {
          variant: "info"
        }, this.state.success);
      }

      if (!!this.state.error) {
        message = /*#__PURE__*/react.createElement(Alert/* default */.Z, {
          variant: "danger"
        }, this.state.error);
      }

      var channelError = null;

      if (!!this.state.errors.channel) {
        channelError = /*#__PURE__*/react.createElement(Form/* default.Control.Feedback */.Z.Control.Feedback, {
          type: "invalid"
        }, this.state.errors.channel);
      }

      var textError = null;

      if (!!this.state.errors.text) {
        textError = /*#__PURE__*/react.createElement(Form/* default.Control.Feedback */.Z.Control.Feedback, {
          type: "invalid"
        }, this.state.errors.text);
      }

      return /*#__PURE__*/react.createElement("div", null, message, /*#__PURE__*/react.createElement(Form/* default */.Z, {
        onSubmit: function onSubmit(e) {
          return _this9.import(e);
        },
        disabled: this.state.loading
      }, /*#__PURE__*/react.createElement(Form/* default.Group */.Z.Group, {
        id: "channel"
      }, /*#__PURE__*/react.createElement(Form/* default.Label */.Z.Label, null, "Channel"), /*#__PURE__*/react.createElement(Form/* default.Control */.Z.Control, {
        disabled: this.state.loading,
        isInvalid: !!this.state.errors.channel,
        value: this.state.channel,
        onChange: function onChange(e) {
          return _this9.handleChannelChange(e);
        },
        placeholder: "#setbac"
      }), channelError, /*#__PURE__*/react.createElement(Form/* default.Text */.Z.Text, null, "Name of channel to import balances for. Like ", /*#__PURE__*/react.createElement("b", null, "#setbac"), ".")), /*#__PURE__*/react.createElement(Form/* default.Group */.Z.Group, {
        id: "content"
      }, /*#__PURE__*/react.createElement(Form/* default.Control */.Z.Control, {
        as: "textarea",
        rows: "10",
        disabled: this.state.loading,
        isInvalid: !!this.state.errors.text,
        value: this.state.text,
        onChange: function onChange(e) {
          return _this9.handleTextChange(e);
        },
        placeholder: ",PhantomBot,1000"
      }), textError, /*#__PURE__*/react.createElement(Form/* default.Text */.Z.Text, null, "Balances to import. Each line should be ", /*#__PURE__*/react.createElement("code", null, ",user,amount"), ".")), /*#__PURE__*/react.createElement(Button/* default */.Z, {
        variant: "primary",
        type: "submit",
        disabled: this.state.loading
      }, "Import"), /*#__PURE__*/react.createElement(Loading, {
        isLoading: this.state.loading
      })));
    }
  }]);

  return PhantomBotImportCsvForm;
}(react.Component);

function importCsv(text) {
  var out = [];
  var lines = text.split('\n');
  var columnNames = lines[0].split(',');

  var _iterator5 = ImportExport_createForOfIteratorHelper(lines.slice(1)),
      _step5;

  try {
    for (_iterator5.s(); !(_step5 = _iterator5.n()).done;) {
      _line = _step5.value;

      var cols = _line.split(',');

      var _line = {};

      for (var i = 0; i < columnNames.length; i++) {
        _line[columnNames[i]] = cols[i];
      }

      out.push(_line);
    }
  } catch (err) {
    _iterator5.e(err);
  } finally {
    _iterator5.f();
  }

  return out;
}
;// CONCATENATED MODULE: ./src/components/Commands.js
function Commands_typeof(obj) { "@babel/helpers - typeof"; if (typeof Symbol === "function" && typeof Symbol.iterator === "symbol") { Commands_typeof = function _typeof(obj) { return typeof obj; }; } else { Commands_typeof = function _typeof(obj) { return obj && typeof Symbol === "function" && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; }; } return Commands_typeof(obj); }







function Commands_asyncGeneratorStep(gen, resolve, reject, _next, _throw, key, arg) { try { var info = gen[key](arg); var value = info.value; } catch (error) { reject(error); return; } if (info.done) { resolve(value); } else { Promise.resolve(value).then(_next, _throw); } }

function Commands_asyncToGenerator(fn) { return function () { var self = this, args = arguments; return new Promise(function (resolve, reject) { var gen = fn.apply(self, args); function _next(value) { Commands_asyncGeneratorStep(gen, resolve, reject, _next, _throw, "next", value); } function _throw(err) { Commands_asyncGeneratorStep(gen, resolve, reject, _next, _throw, "throw", err); } _next(undefined); }); }; }

function Commands_classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function Commands_defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function Commands_createClass(Constructor, protoProps, staticProps) { if (protoProps) Commands_defineProperties(Constructor.prototype, protoProps); if (staticProps) Commands_defineProperties(Constructor, staticProps); return Constructor; }

function Commands_inherits(subClass, superClass) { if (typeof superClass !== "function" && superClass !== null) { throw new TypeError("Super expression must either be null or a function"); } subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, writable: true, configurable: true } }); if (superClass) Commands_setPrototypeOf(subClass, superClass); }

function Commands_setPrototypeOf(o, p) { Commands_setPrototypeOf = Object.setPrototypeOf || function _setPrototypeOf(o, p) { o.__proto__ = p; return o; }; return Commands_setPrototypeOf(o, p); }

function Commands_createSuper(Derived) { var hasNativeReflectConstruct = Commands_isNativeReflectConstruct(); return function _createSuperInternal() { var Super = Commands_getPrototypeOf(Derived), result; if (hasNativeReflectConstruct) { var NewTarget = Commands_getPrototypeOf(this).constructor; result = Reflect.construct(Super, arguments, NewTarget); } else { result = Super.apply(this, arguments); } return Commands_possibleConstructorReturn(this, result); }; }

function Commands_possibleConstructorReturn(self, call) { if (call && (Commands_typeof(call) === "object" || typeof call === "function")) { return call; } return Commands_assertThisInitialized(self); }

function Commands_assertThisInitialized(self) { if (self === void 0) { throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); } return self; }

function Commands_isNativeReflectConstruct() { if (typeof Reflect === "undefined" || !Reflect.construct) return false; if (Reflect.construct.sham) return false; if (typeof Proxy === "function") return true; try { Date.prototype.toString.call(Reflect.construct(Date, [], function () {})); return true; } catch (e) { return false; } }

function Commands_getPrototypeOf(o) { Commands_getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf : function _getPrototypeOf(o) { return o.__proto__ || Object.getPrototypeOf(o); }; return Commands_getPrototypeOf(o); }






var Commands = /*#__PURE__*/function (_React$Component) {
  Commands_inherits(Commands, _React$Component);

  var _super = Commands_createSuper(Commands);

  function Commands(props) {
    var _this;

    Commands_classCallCheck(this, Commands);

    _this = _super.call(this, props);
    _this.api = _this.props.api;
    _this.state = {
      loading: false,
      configLoading: false,
      error: null,
      data: null
    };
    return _this;
  }

  Commands_createClass(Commands, [{
    key: "componentDidMount",
    value: function () {
      var _componentDidMount = Commands_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee() {
        return regeneratorRuntime.wrap(function _callee$(_context) {
          while (1) {
            switch (_context.prev = _context.next) {
              case 0:
                _context.next = 2;
                return this.list();

              case 2:
              case "end":
                return _context.stop();
            }
          }
        }, _callee, this);
      }));

      function componentDidMount() {
        return _componentDidMount.apply(this, arguments);
      }

      return componentDidMount;
    }()
    /**
     * Refresh the list of after streams.
     */

  }, {
    key: "list",
    value: function () {
      var _list = Commands_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee2() {
        var data;
        return regeneratorRuntime.wrap(function _callee2$(_context2) {
          while (1) {
            switch (_context2.prev = _context2.next) {
              case 0:
                this.setState({
                  loading: true
                });
                _context2.prev = 1;
                _context2.next = 4;
                return this.api.commands(this.props.current.channel);

              case 4:
                data = _context2.sent;
                this.setState({
                  loading: false,
                  error: null,
                  data: data
                });
                _context2.next = 11;
                break;

              case 8:
                _context2.prev = 8;
                _context2.t0 = _context2["catch"](1);
                this.setState({
                  loading: false,
                  error: "failed to request after streams: ".concat(_context2.t0)
                });

              case 11:
              case "end":
                return _context2.stop();
            }
          }
        }, _callee2, this, [[1, 8]]);
      }));

      function list() {
        return _list.apply(this, arguments);
      }

      return list;
    }()
  }, {
    key: "editDisabled",
    value: function () {
      var _editDisabled = Commands_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee3(key, disabled) {
        var _this2 = this;

        return regeneratorRuntime.wrap(function _callee3$(_context3) {
          while (1) {
            switch (_context3.prev = _context3.next) {
              case 0:
                this.setState({
                  loading: true,
                  error: null
                });
                _context3.next = 3;
                return this.api.commandsEditDisabled(key, disabled);

              case 3:
                _context3.next = 5;
                return this.list();

              case 5:
                try {
                  this.setState({
                    loading: false,
                    error: "Failed to set disabled state: ".concat(e)
                  });
                } catch (e) {
                  (function (e) {
                    _this2.setState({
                      loading: false,
                      error: "Failed to set disabled state: ".concat(e)
                    });
                  });
                }

              case 6:
              case "end":
                return _context3.stop();
            }
          }
        }, _callee3, this);
      }));

      function editDisabled(_x, _x2) {
        return _editDisabled.apply(this, arguments);
      }

      return editDisabled;
    }()
  }, {
    key: "render",
    value: function render() {
      var _this3 = this;

      var content = null;

      if (this.state.data) {
        if (this.state.data.length === 0) {
          content = /*#__PURE__*/react.createElement(Alert/* default */.Z, {
            variant: "info"
          }, "No commands!");
        } else {
          content = /*#__PURE__*/react.createElement(Table/* default */.Z, {
            responsive: "sm"
          }, /*#__PURE__*/react.createElement("thead", null, /*#__PURE__*/react.createElement("tr", null, /*#__PURE__*/react.createElement("th", null, "Name"), /*#__PURE__*/react.createElement("th", null, "Group"), /*#__PURE__*/react.createElement("th", {
            className: "table-fill"
          }, "Text"), /*#__PURE__*/react.createElement("th", null))), /*#__PURE__*/react.createElement("tbody", null, this.state.data.map(function (c, id) {
            var disabled = null;

            if (c.disabled) {
              var onClick = function onClick(_) {
                _this3.editDisabled(c.key, false);
              };

              disabled = /*#__PURE__*/react.createElement(Button/* default */.Z, {
                className: "button-fill",
                size: "sm",
                variant: "danger",
                onClick: onClick
              }, "Disabled");
            } else {
              var _onClick = function _onClick(_) {
                _this3.editDisabled(c.key, true);
              };

              disabled = /*#__PURE__*/react.createElement(Button/* default */.Z, {
                className: "button-fill",
                size: "sm",
                variant: "success",
                onClick: _onClick
              }, "Enabled");
            }

            return /*#__PURE__*/react.createElement("tr", {
              key: id
            }, /*#__PURE__*/react.createElement("td", {
              className: "command-name"
            }, c.key.name), /*#__PURE__*/react.createElement("td", {
              className: "command-group"
            }, /*#__PURE__*/react.createElement("b", null, c.group)), /*#__PURE__*/react.createElement("td", {
              className: "command-template"
            }, c.template), /*#__PURE__*/react.createElement("td", null, disabled));
          })));
        }
      }

      return /*#__PURE__*/react.createElement(react.Fragment, null, /*#__PURE__*/react.createElement("h1", {
        className: "oxi-page-title"
      }, "Commands"), /*#__PURE__*/react.createElement(Error_Loading, {
        error: this.state.error
      }), /*#__PURE__*/react.createElement(Loading, {
        isLoading: this.state.loading || this.state.configLoading
      }), /*#__PURE__*/react.createElement(ConfigurationPrompt, {
        api: this.api,
        filter: {
          prefix: ["command"]
        },
        onLoading: function onLoading(configLoading) {
          return _this3.setState({
            configLoading: configLoading,
            error: null
          });
        },
        onError: function onError(error) {
          return _this3.setState({
            configLoading: false,
            error: error
          });
        }
      }), content);
    }
  }]);

  return Commands;
}(react.Component);


// EXTERNAL MODULE: ./node_modules/@fortawesome/fontawesome-free-solid/index.es.js
var fontawesome_free_solid_index_es = __webpack_require__(95742);
;// CONCATENATED MODULE: ./src/components/Promotions.js
function Promotions_typeof(obj) { "@babel/helpers - typeof"; if (typeof Symbol === "function" && typeof Symbol.iterator === "symbol") { Promotions_typeof = function _typeof(obj) { return typeof obj; }; } else { Promotions_typeof = function _typeof(obj) { return obj && typeof Symbol === "function" && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; }; } return Promotions_typeof(obj); }







function Promotions_asyncGeneratorStep(gen, resolve, reject, _next, _throw, key, arg) { try { var info = gen[key](arg); var value = info.value; } catch (error) { reject(error); return; } if (info.done) { resolve(value); } else { Promise.resolve(value).then(_next, _throw); } }

function Promotions_asyncToGenerator(fn) { return function () { var self = this, args = arguments; return new Promise(function (resolve, reject) { var gen = fn.apply(self, args); function _next(value) { Promotions_asyncGeneratorStep(gen, resolve, reject, _next, _throw, "next", value); } function _throw(err) { Promotions_asyncGeneratorStep(gen, resolve, reject, _next, _throw, "throw", err); } _next(undefined); }); }; }

function Promotions_classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function Promotions_defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function Promotions_createClass(Constructor, protoProps, staticProps) { if (protoProps) Promotions_defineProperties(Constructor.prototype, protoProps); if (staticProps) Promotions_defineProperties(Constructor, staticProps); return Constructor; }

function Promotions_inherits(subClass, superClass) { if (typeof superClass !== "function" && superClass !== null) { throw new TypeError("Super expression must either be null or a function"); } subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, writable: true, configurable: true } }); if (superClass) Promotions_setPrototypeOf(subClass, superClass); }

function Promotions_setPrototypeOf(o, p) { Promotions_setPrototypeOf = Object.setPrototypeOf || function _setPrototypeOf(o, p) { o.__proto__ = p; return o; }; return Promotions_setPrototypeOf(o, p); }

function Promotions_createSuper(Derived) { var hasNativeReflectConstruct = Promotions_isNativeReflectConstruct(); return function _createSuperInternal() { var Super = Promotions_getPrototypeOf(Derived), result; if (hasNativeReflectConstruct) { var NewTarget = Promotions_getPrototypeOf(this).constructor; result = Reflect.construct(Super, arguments, NewTarget); } else { result = Super.apply(this, arguments); } return Promotions_possibleConstructorReturn(this, result); }; }

function Promotions_possibleConstructorReturn(self, call) { if (call && (Promotions_typeof(call) === "object" || typeof call === "function")) { return call; } return Promotions_assertThisInitialized(self); }

function Promotions_assertThisInitialized(self) { if (self === void 0) { throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); } return self; }

function Promotions_isNativeReflectConstruct() { if (typeof Reflect === "undefined" || !Reflect.construct) return false; if (Reflect.construct.sham) return false; if (typeof Proxy === "function") return true; try { Date.prototype.toString.call(Reflect.construct(Date, [], function () {})); return true; } catch (e) { return false; } }

function Promotions_getPrototypeOf(o) { Promotions_getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf : function _getPrototypeOf(o) { return o.__proto__ || Object.getPrototypeOf(o); }; return Promotions_getPrototypeOf(o); }






var Promotions = /*#__PURE__*/function (_React$Component) {
  Promotions_inherits(Promotions, _React$Component);

  var _super = Promotions_createSuper(Promotions);

  function Promotions(props) {
    var _this;

    Promotions_classCallCheck(this, Promotions);

    _this = _super.call(this, props);
    _this.api = _this.props.api;
    _this.state = {
      loading: false,
      configLoading: false,
      error: null,
      data: null
    };
    return _this;
  }

  Promotions_createClass(Promotions, [{
    key: "componentDidMount",
    value: function () {
      var _componentDidMount = Promotions_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee() {
        return regeneratorRuntime.wrap(function _callee$(_context) {
          while (1) {
            switch (_context.prev = _context.next) {
              case 0:
                _context.next = 2;
                return this.list();

              case 2:
              case "end":
                return _context.stop();
            }
          }
        }, _callee, this);
      }));

      function componentDidMount() {
        return _componentDidMount.apply(this, arguments);
      }

      return componentDidMount;
    }()
    /**
     * Refresh the list of after streams.
     */

  }, {
    key: "list",
    value: function () {
      var _list = Promotions_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee2() {
        var data;
        return regeneratorRuntime.wrap(function _callee2$(_context2) {
          while (1) {
            switch (_context2.prev = _context2.next) {
              case 0:
                this.setState({
                  loading: true
                });
                _context2.prev = 1;
                _context2.next = 4;
                return this.api.promotions(this.props.current.channel);

              case 4:
                data = _context2.sent;
                this.setState({
                  loading: false,
                  error: null,
                  data: data
                });
                _context2.next = 11;
                break;

              case 8:
                _context2.prev = 8;
                _context2.t0 = _context2["catch"](1);
                this.setState({
                  loading: false,
                  error: "failed to request after streams: ".concat(_context2.t0),
                  data: null
                });

              case 11:
              case "end":
                return _context2.stop();
            }
          }
        }, _callee2, this, [[1, 8]]);
      }));

      function list() {
        return _list.apply(this, arguments);
      }

      return list;
    }()
  }, {
    key: "editDisabled",
    value: function () {
      var _editDisabled = Promotions_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee3(key, disabled) {
        return regeneratorRuntime.wrap(function _callee3$(_context3) {
          while (1) {
            switch (_context3.prev = _context3.next) {
              case 0:
                this.setState({
                  loading: true,
                  error: null
                });
                _context3.prev = 1;
                _context3.next = 4;
                return this.api.promotionsEditDisabled(key, disabled);

              case 4:
                _context3.next = 6;
                return this.list();

              case 6:
                _context3.next = 11;
                break;

              case 8:
                _context3.prev = 8;
                _context3.t0 = _context3["catch"](1);
                this.setState({
                  loading: false,
                  error: "Failed to set disabled state: ".concat(_context3.t0)
                });

              case 11:
              case "end":
                return _context3.stop();
            }
          }
        }, _callee3, this, [[1, 8]]);
      }));

      function editDisabled(_x, _x2) {
        return _editDisabled.apply(this, arguments);
      }

      return editDisabled;
    }()
  }, {
    key: "render",
    value: function render() {
      var _this2 = this;

      var content = null;

      if (this.state.data) {
        if (this.state.data.length === 0) {
          content = /*#__PURE__*/react.createElement(Alert/* default */.Z, {
            variant: "info"
          }, "No promotions!");
        } else {
          content = /*#__PURE__*/react.createElement(Table/* default */.Z, {
            responsive: "sm"
          }, /*#__PURE__*/react.createElement("thead", null, /*#__PURE__*/react.createElement("tr", null, /*#__PURE__*/react.createElement("th", null, "Name"), /*#__PURE__*/react.createElement("th", null, "Group"), /*#__PURE__*/react.createElement("th", {
            className: "table-fill"
          }, "Text"), /*#__PURE__*/react.createElement("th", null))), /*#__PURE__*/react.createElement("tbody", null, this.state.data.map(function (c, id) {
            var disabled = null;

            if (c.disabled) {
              var onClick = function onClick(_) {
                _this2.editDisabled(c.key, false);
              };

              disabled = /*#__PURE__*/react.createElement(Button/* default */.Z, {
                className: "button-fill",
                size: "sm",
                variant: "danger",
                onClick: onClick
              }, "Disabled");
            } else {
              var _onClick = function _onClick(_) {
                _this2.editDisabled(c.key, true);
              };

              disabled = /*#__PURE__*/react.createElement(Button/* default */.Z, {
                className: "button-fill",
                size: "sm",
                variant: "success",
                onClick: _onClick
              }, "Enabled");
            }

            return /*#__PURE__*/react.createElement("tr", {
              key: id
            }, /*#__PURE__*/react.createElement("td", {
              className: "promotion-name"
            }, c.key.name), /*#__PURE__*/react.createElement("td", {
              className: "promotion-group"
            }, /*#__PURE__*/react.createElement("b", null, c.group)), /*#__PURE__*/react.createElement("td", {
              className: "promotion-template"
            }, c.template), /*#__PURE__*/react.createElement("td", null, disabled));
          })));
        }
      }

      return /*#__PURE__*/react.createElement(react.Fragment, null, /*#__PURE__*/react.createElement("h1", {
        className: "oxi-page-title"
      }, "Promotions"), /*#__PURE__*/react.createElement(Loading, {
        isLoading: this.state.loading || this.state.configLoading
      }), /*#__PURE__*/react.createElement(Error_Loading, {
        error: this.state.error || this.state.configError
      }), /*#__PURE__*/react.createElement(ConfigurationPrompt, {
        api: this.api,
        filter: {
          prefix: ["promotions"]
        },
        onLoading: function onLoading(configLoading) {
          return _this2.setState({
            configLoading: configLoading,
            error: null
          });
        },
        onError: function onError(error) {
          return _this2.setState({
            configLoading: false,
            error: error
          });
        }
      }), content);
    }
  }]);

  return Promotions;
}(react.Component);


;// CONCATENATED MODULE: ./src/components/Aliases.js
function Aliases_typeof(obj) { "@babel/helpers - typeof"; if (typeof Symbol === "function" && typeof Symbol.iterator === "symbol") { Aliases_typeof = function _typeof(obj) { return typeof obj; }; } else { Aliases_typeof = function _typeof(obj) { return obj && typeof Symbol === "function" && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; }; } return Aliases_typeof(obj); }







function Aliases_asyncGeneratorStep(gen, resolve, reject, _next, _throw, key, arg) { try { var info = gen[key](arg); var value = info.value; } catch (error) { reject(error); return; } if (info.done) { resolve(value); } else { Promise.resolve(value).then(_next, _throw); } }

function Aliases_asyncToGenerator(fn) { return function () { var self = this, args = arguments; return new Promise(function (resolve, reject) { var gen = fn.apply(self, args); function _next(value) { Aliases_asyncGeneratorStep(gen, resolve, reject, _next, _throw, "next", value); } function _throw(err) { Aliases_asyncGeneratorStep(gen, resolve, reject, _next, _throw, "throw", err); } _next(undefined); }); }; }

function Aliases_classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function Aliases_defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function Aliases_createClass(Constructor, protoProps, staticProps) { if (protoProps) Aliases_defineProperties(Constructor.prototype, protoProps); if (staticProps) Aliases_defineProperties(Constructor, staticProps); return Constructor; }

function Aliases_inherits(subClass, superClass) { if (typeof superClass !== "function" && superClass !== null) { throw new TypeError("Super expression must either be null or a function"); } subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, writable: true, configurable: true } }); if (superClass) Aliases_setPrototypeOf(subClass, superClass); }

function Aliases_setPrototypeOf(o, p) { Aliases_setPrototypeOf = Object.setPrototypeOf || function _setPrototypeOf(o, p) { o.__proto__ = p; return o; }; return Aliases_setPrototypeOf(o, p); }

function Aliases_createSuper(Derived) { var hasNativeReflectConstruct = Aliases_isNativeReflectConstruct(); return function _createSuperInternal() { var Super = Aliases_getPrototypeOf(Derived), result; if (hasNativeReflectConstruct) { var NewTarget = Aliases_getPrototypeOf(this).constructor; result = Reflect.construct(Super, arguments, NewTarget); } else { result = Super.apply(this, arguments); } return Aliases_possibleConstructorReturn(this, result); }; }

function Aliases_possibleConstructorReturn(self, call) { if (call && (Aliases_typeof(call) === "object" || typeof call === "function")) { return call; } return Aliases_assertThisInitialized(self); }

function Aliases_assertThisInitialized(self) { if (self === void 0) { throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); } return self; }

function Aliases_isNativeReflectConstruct() { if (typeof Reflect === "undefined" || !Reflect.construct) return false; if (Reflect.construct.sham) return false; if (typeof Proxy === "function") return true; try { Date.prototype.toString.call(Reflect.construct(Date, [], function () {})); return true; } catch (e) { return false; } }

function Aliases_getPrototypeOf(o) { Aliases_getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf : function _getPrototypeOf(o) { return o.__proto__ || Object.getPrototypeOf(o); }; return Aliases_getPrototypeOf(o); }





var Aliases = /*#__PURE__*/function (_React$Component) {
  Aliases_inherits(Aliases, _React$Component);

  var _super = Aliases_createSuper(Aliases);

  function Aliases(props) {
    var _this;

    Aliases_classCallCheck(this, Aliases);

    _this = _super.call(this, props);
    _this.api = _this.props.api;
    _this.state = {
      loading: true,
      error: null,
      data: null
    };
    return _this;
  }

  Aliases_createClass(Aliases, [{
    key: "componentDidMount",
    value: function () {
      var _componentDidMount = Aliases_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee() {
        return regeneratorRuntime.wrap(function _callee$(_context) {
          while (1) {
            switch (_context.prev = _context.next) {
              case 0:
                _context.next = 2;
                return this.list();

              case 2:
              case "end":
                return _context.stop();
            }
          }
        }, _callee, this);
      }));

      function componentDidMount() {
        return _componentDidMount.apply(this, arguments);
      }

      return componentDidMount;
    }()
    /**
     * Refresh the list of after streams.
     */

  }, {
    key: "list",
    value: function () {
      var _list = Aliases_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee2() {
        var data;
        return regeneratorRuntime.wrap(function _callee2$(_context2) {
          while (1) {
            switch (_context2.prev = _context2.next) {
              case 0:
                this.setState({
                  loading: true
                });
                _context2.prev = 1;
                _context2.next = 4;
                return this.api.aliases(this.props.current.channel);

              case 4:
                data = _context2.sent;
                this.setState({
                  loading: false,
                  error: null,
                  data: data
                });
                _context2.next = 11;
                break;

              case 8:
                _context2.prev = 8;
                _context2.t0 = _context2["catch"](1);
                this.setState({
                  loading: false,
                  error: "failed to request after streams: ".concat(_context2.t0),
                  data: null
                });

              case 11:
              case "end":
                return _context2.stop();
            }
          }
        }, _callee2, this, [[1, 8]]);
      }));

      function list() {
        return _list.apply(this, arguments);
      }

      return list;
    }()
  }, {
    key: "editDisabled",
    value: function () {
      var _editDisabled = Aliases_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee3(key, disabled) {
        return regeneratorRuntime.wrap(function _callee3$(_context3) {
          while (1) {
            switch (_context3.prev = _context3.next) {
              case 0:
                this.setState({
                  loading: true,
                  error: null
                });
                _context3.prev = 1;
                _context3.next = 4;
                return this.api.aliasesEditDisabled(key, disabled);

              case 4:
                _context3.next = 9;
                break;

              case 6:
                _context3.prev = 6;
                _context3.t0 = _context3["catch"](1);
                this.setState({
                  loading: false,
                  error: "Failed to set disabled state: ".concat(_context3.t0)
                });

              case 9:
                return _context3.abrupt("return", this.list());

              case 10:
              case "end":
                return _context3.stop();
            }
          }
        }, _callee3, this, [[1, 6]]);
      }));

      function editDisabled(_x, _x2) {
        return _editDisabled.apply(this, arguments);
      }

      return editDisabled;
    }()
  }, {
    key: "render",
    value: function render() {
      var _this2 = this;

      var error = null;

      if (this.state.error) {
        error = /*#__PURE__*/react.createElement(Alert/* default */.Z, {
          variant: "warning"
        }, this.state.error);
      }

      var content = null;

      if (this.state.data) {
        if (this.state.data.length === 0) {
          content = /*#__PURE__*/react.createElement(Alert/* default */.Z, {
            variant: "info"
          }, "No aliases!");
        } else {
          content = /*#__PURE__*/react.createElement(Table/* default */.Z, {
            responsive: "sm"
          }, /*#__PURE__*/react.createElement("thead", null, /*#__PURE__*/react.createElement("tr", null, /*#__PURE__*/react.createElement("th", null, "Name"), /*#__PURE__*/react.createElement("th", null, "Group"), /*#__PURE__*/react.createElement("th", {
            className: "table-fill"
          }, "Text"), /*#__PURE__*/react.createElement("th", null))), /*#__PURE__*/react.createElement("tbody", null, this.state.data.map(function (c, id) {
            var disabled = null;

            if (c.disabled) {
              var onClick = function onClick(_) {
                _this2.editDisabled(c.key, false);
              };

              disabled = /*#__PURE__*/react.createElement(Button/* default */.Z, {
                className: "button-fill",
                size: "sm",
                variant: "danger",
                onClick: onClick
              }, "Disabled");
            } else {
              var _onClick = function _onClick(_) {
                _this2.editDisabled(c.key, true);
              };

              disabled = /*#__PURE__*/react.createElement(Button/* default */.Z, {
                className: "button-fill",
                size: "sm",
                variant: "success",
                onClick: _onClick
              }, "Enabled");
            }

            return /*#__PURE__*/react.createElement("tr", {
              key: id
            }, /*#__PURE__*/react.createElement("td", {
              className: "alias-name"
            }, c.key.name), /*#__PURE__*/react.createElement("td", {
              className: "alias-group"
            }, /*#__PURE__*/react.createElement("b", null, c.group)), /*#__PURE__*/react.createElement("td", {
              className: "alias-template"
            }, c.template), /*#__PURE__*/react.createElement("td", null, disabled));
          })));
        }
      }

      return /*#__PURE__*/react.createElement(react.Fragment, null, /*#__PURE__*/react.createElement("h1", {
        className: "oxi-page-title"
      }, "Aliases"), /*#__PURE__*/react.createElement(Loading, {
        isLoading: this.state.loading
      }), error, content);
    }
  }]);

  return Aliases;
}(react.Component);


;// CONCATENATED MODULE: ./src/components/Themes.js
function Themes_typeof(obj) { "@babel/helpers - typeof"; if (typeof Symbol === "function" && typeof Symbol.iterator === "symbol") { Themes_typeof = function _typeof(obj) { return typeof obj; }; } else { Themes_typeof = function _typeof(obj) { return obj && typeof Symbol === "function" && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; }; } return Themes_typeof(obj); }










function Themes_asyncGeneratorStep(gen, resolve, reject, _next, _throw, key, arg) { try { var info = gen[key](arg); var value = info.value; } catch (error) { reject(error); return; } if (info.done) { resolve(value); } else { Promise.resolve(value).then(_next, _throw); } }

function Themes_asyncToGenerator(fn) { return function () { var self = this, args = arguments; return new Promise(function (resolve, reject) { var gen = fn.apply(self, args); function _next(value) { Themes_asyncGeneratorStep(gen, resolve, reject, _next, _throw, "next", value); } function _throw(err) { Themes_asyncGeneratorStep(gen, resolve, reject, _next, _throw, "throw", err); } _next(undefined); }); }; }

function Themes_classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function Themes_defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function Themes_createClass(Constructor, protoProps, staticProps) { if (protoProps) Themes_defineProperties(Constructor.prototype, protoProps); if (staticProps) Themes_defineProperties(Constructor, staticProps); return Constructor; }

function Themes_inherits(subClass, superClass) { if (typeof superClass !== "function" && superClass !== null) { throw new TypeError("Super expression must either be null or a function"); } subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, writable: true, configurable: true } }); if (superClass) Themes_setPrototypeOf(subClass, superClass); }

function Themes_setPrototypeOf(o, p) { Themes_setPrototypeOf = Object.setPrototypeOf || function _setPrototypeOf(o, p) { o.__proto__ = p; return o; }; return Themes_setPrototypeOf(o, p); }

function Themes_createSuper(Derived) { var hasNativeReflectConstruct = Themes_isNativeReflectConstruct(); return function _createSuperInternal() { var Super = Themes_getPrototypeOf(Derived), result; if (hasNativeReflectConstruct) { var NewTarget = Themes_getPrototypeOf(this).constructor; result = Reflect.construct(Super, arguments, NewTarget); } else { result = Super.apply(this, arguments); } return Themes_possibleConstructorReturn(this, result); }; }

function Themes_possibleConstructorReturn(self, call) { if (call && (Themes_typeof(call) === "object" || typeof call === "function")) { return call; } return Themes_assertThisInitialized(self); }

function Themes_assertThisInitialized(self) { if (self === void 0) { throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); } return self; }

function Themes_isNativeReflectConstruct() { if (typeof Reflect === "undefined" || !Reflect.construct) return false; if (Reflect.construct.sham) return false; if (typeof Proxy === "function") return true; try { Date.prototype.toString.call(Reflect.construct(Date, [], function () {})); return true; } catch (e) { return false; } }

function Themes_getPrototypeOf(o) { Themes_getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf : function _getPrototypeOf(o) { return o.__proto__ || Object.getPrototypeOf(o); }; return Themes_getPrototypeOf(o); }





function trackUrl(trackId) {
  if (trackId.startsWith("spotify:track:")) {
    var id = trackId.split(":")[2];
    return "https://open.spotify.com/track/".concat(id);
  }

  if (trackId.startsWith("youtube:video:")) {
    var _id = trackId.split(":")[2];
    return "https://youtu.be/".concat(_id);
  }

  return null;
}

var Themes = /*#__PURE__*/function (_React$Component) {
  Themes_inherits(Themes, _React$Component);

  var _super = Themes_createSuper(Themes);

  function Themes(props) {
    var _this;

    Themes_classCallCheck(this, Themes);

    _this = _super.call(this, props);
    _this.api = _this.props.api;
    _this.state = {
      loading: true,
      error: null,
      data: null
    };
    return _this;
  }

  Themes_createClass(Themes, [{
    key: "componentDidMount",
    value: function () {
      var _componentDidMount = Themes_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee() {
        return regeneratorRuntime.wrap(function _callee$(_context) {
          while (1) {
            switch (_context.prev = _context.next) {
              case 0:
                _context.next = 2;
                return this.list();

              case 2:
              case "end":
                return _context.stop();
            }
          }
        }, _callee, this);
      }));

      function componentDidMount() {
        return _componentDidMount.apply(this, arguments);
      }

      return componentDidMount;
    }()
    /**
     * Refresh the list of after streams.
     */

  }, {
    key: "list",
    value: function () {
      var _list = Themes_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee2() {
        var data;
        return regeneratorRuntime.wrap(function _callee2$(_context2) {
          while (1) {
            switch (_context2.prev = _context2.next) {
              case 0:
                this.setState({
                  loading: true
                });
                _context2.prev = 1;
                _context2.next = 4;
                return this.api.themes(this.props.current.channel);

              case 4:
                data = _context2.sent;
                this.setState({
                  loading: false,
                  error: null,
                  data: data
                });
                _context2.next = 11;
                break;

              case 8:
                _context2.prev = 8;
                _context2.t0 = _context2["catch"](1);
                this.setState({
                  loading: false,
                  error: "failed to request after streams: ".concat(_context2.t0),
                  data: null
                });

              case 11:
              case "end":
                return _context2.stop();
            }
          }
        }, _callee2, this, [[1, 8]]);
      }));

      function list() {
        return _list.apply(this, arguments);
      }

      return list;
    }()
  }, {
    key: "editDisabled",
    value: function () {
      var _editDisabled = Themes_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee3(key, disabled) {
        return regeneratorRuntime.wrap(function _callee3$(_context3) {
          while (1) {
            switch (_context3.prev = _context3.next) {
              case 0:
                this.setState({
                  loading: true,
                  error: null
                });
                _context3.prev = 1;
                _context3.next = 4;
                return this.api.themesEditDisabled(key, disabled);

              case 4:
                _context3.next = 6;
                return this.list();

              case 6:
                _context3.next = 11;
                break;

              case 8:
                _context3.prev = 8;
                _context3.t0 = _context3["catch"](1);
                this.setState({
                  loading: false,
                  error: "Failed to set disabled state: ".concat(_context3.t0)
                });

              case 11:
              case "end":
                return _context3.stop();
            }
          }
        }, _callee3, this, [[1, 8]]);
      }));

      function editDisabled(_x, _x2) {
        return _editDisabled.apply(this, arguments);
      }

      return editDisabled;
    }()
  }, {
    key: "render",
    value: function render() {
      var _this2 = this;

      var loading = null;
      var content = null;

      if (this.state.data) {
        if (this.state.data.length === 0) {
          content = /*#__PURE__*/react.createElement(Alert/* default */.Z, {
            variant: "info"
          }, "No themes!");
        } else {
          content = /*#__PURE__*/react.createElement(Table/* default */.Z, {
            responsive: "sm"
          }, /*#__PURE__*/react.createElement("thead", null, /*#__PURE__*/react.createElement("tr", null, /*#__PURE__*/react.createElement("th", null, "Name"), /*#__PURE__*/react.createElement("th", null, "Group"), /*#__PURE__*/react.createElement("th", null, "Start"), /*#__PURE__*/react.createElement("th", null, "End"), /*#__PURE__*/react.createElement("th", {
            className: "table-fill"
          }, "Track ID"), /*#__PURE__*/react.createElement("th", null))), /*#__PURE__*/react.createElement("tbody", null, this.state.data.map(function (c, id) {
            var disabled = null;

            if (c.disabled) {
              var onClick = function onClick(_) {
                _this2.editDisabled(c.key, false);
              };

              disabled = /*#__PURE__*/react.createElement(Button/* default */.Z, {
                className: "button-fill",
                size: "sm",
                variant: "danger",
                onClick: onClick
              }, "Disabled");
            } else {
              var _onClick = function _onClick(_) {
                _this2.editDisabled(c.key, true);
              };

              disabled = /*#__PURE__*/react.createElement(Button/* default */.Z, {
                className: "button-fill",
                size: "sm",
                variant: "success",
                onClick: _onClick
              }, "Enabled");
            }

            var track = c.track_id;
            var url = trackUrl(c.track_id);

            if (!!url) {
              track = /*#__PURE__*/react.createElement("a", {
                href: url,
                target: "track"
              }, c.track_id);
            }

            return /*#__PURE__*/react.createElement("tr", {
              key: id
            }, /*#__PURE__*/react.createElement("td", {
              className: "theme-name"
            }, c.key.name), /*#__PURE__*/react.createElement("td", {
              className: "theme-group"
            }, /*#__PURE__*/react.createElement("b", null, c.group)), /*#__PURE__*/react.createElement("td", {
              className: "theme-start"
            }, c.start), /*#__PURE__*/react.createElement("td", {
              className: "theme-end"
            }, c.end), /*#__PURE__*/react.createElement("td", {
              className: "theme-track-id"
            }, track), /*#__PURE__*/react.createElement("td", null, disabled));
          })));
        }
      }

      return /*#__PURE__*/react.createElement("div", null, /*#__PURE__*/react.createElement("h1", {
        className: "oxi-page-title"
      }, "Themes"), /*#__PURE__*/react.createElement(Loading, {
        isLoading: this.state.loading
      }), /*#__PURE__*/react.createElement(Error_Loading, {
        error: this.state.error
      }), content, loading);
    }
  }]);

  return Themes;
}(react.Component);


;// CONCATENATED MODULE: ./src/components/YouTube.js
function YouTube_typeof(obj) { "@babel/helpers - typeof"; if (typeof Symbol === "function" && typeof Symbol.iterator === "symbol") { YouTube_typeof = function _typeof(obj) { return typeof obj; }; } else { YouTube_typeof = function _typeof(obj) { return obj && typeof Symbol === "function" && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; }; } return YouTube_typeof(obj); }





function YouTube_createForOfIteratorHelper(o, allowArrayLike) { var it; if (typeof Symbol === "undefined" || o[Symbol.iterator] == null) { if (Array.isArray(o) || (it = YouTube_unsupportedIterableToArray(o)) || allowArrayLike && o && typeof o.length === "number") { if (it) o = it; var i = 0; var F = function F() {}; return { s: F, n: function n() { if (i >= o.length) return { done: true }; return { done: false, value: o[i++] }; }, e: function e(_e) { throw _e; }, f: F }; } throw new TypeError("Invalid attempt to iterate non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); } var normalCompletion = true, didErr = false, err; return { s: function s() { it = o[Symbol.iterator](); }, n: function n() { var step = it.next(); normalCompletion = step.done; return step; }, e: function e(_e2) { didErr = true; err = _e2; }, f: function f() { try { if (!normalCompletion && it.return != null) it.return(); } finally { if (didErr) throw err; } } }; }

function YouTube_unsupportedIterableToArray(o, minLen) { if (!o) return; if (typeof o === "string") return YouTube_arrayLikeToArray(o, minLen); var n = Object.prototype.toString.call(o).slice(8, -1); if (n === "Object" && o.constructor) n = o.constructor.name; if (n === "Map" || n === "Set") return Array.from(o); if (n === "Arguments" || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(n)) return YouTube_arrayLikeToArray(o, minLen); }

function YouTube_arrayLikeToArray(arr, len) { if (len == null || len > arr.length) len = arr.length; for (var i = 0, arr2 = new Array(len); i < len; i++) { arr2[i] = arr[i]; } return arr2; }

function YouTube_classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function YouTube_defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function YouTube_createClass(Constructor, protoProps, staticProps) { if (protoProps) YouTube_defineProperties(Constructor.prototype, protoProps); if (staticProps) YouTube_defineProperties(Constructor, staticProps); return Constructor; }

function YouTube_inherits(subClass, superClass) { if (typeof superClass !== "function" && superClass !== null) { throw new TypeError("Super expression must either be null or a function"); } subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, writable: true, configurable: true } }); if (superClass) YouTube_setPrototypeOf(subClass, superClass); }

function YouTube_setPrototypeOf(o, p) { YouTube_setPrototypeOf = Object.setPrototypeOf || function _setPrototypeOf(o, p) { o.__proto__ = p; return o; }; return YouTube_setPrototypeOf(o, p); }

function YouTube_createSuper(Derived) { var hasNativeReflectConstruct = YouTube_isNativeReflectConstruct(); return function _createSuperInternal() { var Super = YouTube_getPrototypeOf(Derived), result; if (hasNativeReflectConstruct) { var NewTarget = YouTube_getPrototypeOf(this).constructor; result = Reflect.construct(Super, arguments, NewTarget); } else { result = Super.apply(this, arguments); } return YouTube_possibleConstructorReturn(this, result); }; }

function YouTube_possibleConstructorReturn(self, call) { if (call && (YouTube_typeof(call) === "object" || typeof call === "function")) { return call; } return YouTube_assertThisInitialized(self); }

function YouTube_assertThisInitialized(self) { if (self === void 0) { throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); } return self; }

function YouTube_isNativeReflectConstruct() { if (typeof Reflect === "undefined" || !Reflect.construct) return false; if (Reflect.construct.sham) return false; if (typeof Proxy === "function") return true; try { Date.prototype.toString.call(Reflect.construct(Date, [], function () {})); return true; } catch (e) { return false; } }

function YouTube_getPrototypeOf(o) { YouTube_getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf : function _getPrototypeOf(o) { return o.__proto__ || Object.getPrototypeOf(o); }; return YouTube_getPrototypeOf(o); }





var OBS_CSS = ["body.youtube-body { background-color: rgba(0, 0, 0, 0); }", ".overlay-hidden { display: none }"];
var UNSTARTED = (/* unused pure expression or super */ null && (-1));
var ENDED = 0;
var PLAYING = 1;
var PAUSED = 2;
var BUFFERING = 3;
var VIDEO_CUED = 5;
var SUGGESTED_QUALITY = "hd720";

var YouTube = /*#__PURE__*/function (_React$Component) {
  YouTube_inherits(YouTube, _React$Component);

  var _super = YouTube_createSuper(YouTube);

  function YouTube(props) {
    var _this;

    YouTube_classCallCheck(this, YouTube);

    _this = _super.call(this, props);
    _this.playerElement = null;
    _this.player = null;
    _this.playerRef = /*#__PURE__*/react.createRef();
    _this.state = {
      stopped: true,
      paused: true,
      loading: true,
      events: [],
      api: null,
      videoId: null
    };
    return _this;
  }

  YouTube_createClass(YouTube, [{
    key: "handleData",
    value: function handleData(d) {
      var data = null;

      try {
        data = JSON.parse(d);
      } catch (e) {
        console.log("failed to deserialize message");
        return;
      }

      switch (data.type) {
        case "youtube/current":
          switch (data.event.type) {
            case "play":
              var update = {
                stopped: false,
                paused: false
              };

              if (this.state.videoId !== data.event.video_id) {
                var videoId = data.event.video_id;
                this.player.loadVideoById({
                  videoId: videoId,
                  suggestedQuality: SUGGESTED_QUALITY
                });
                this.player.seekTo(data.event.elapsed);
                this.player.playVideo();
                update.videoId = data.event.video_id;
              } else {
                if (this.state.paused) {
                  this.player.playVideo();
                }

                if (Math.abs(data.event.elapsed - this.player.getCurrentTime()) > 2) {
                  this.player.seekTo(data.event.elapsed);
                }
              }

              this.setState(update);
              break;

            case "pause":
              this.player.pauseVideo();
              this.setState({
                stopped: false,
                paused: true
              });
              break;

            case "stop":
              this.player.pauseVideo();
              this.setState({
                stopped: true,
                paused: false,
                videoId: null
              });
              break;

            default:
              break;
          }

          break;

        case "youtube/volume":
          this.player.setVolume(data.volume);
          break;

        case "song/progress":
          return;

        default:
          return;
      }
    }
  }, {
    key: "setupPlayer",
    value: function setupPlayer() {
      var _this2 = this;

      if (!this.playerRef.current) {
        throw new Error("Reference to player is not available");
      }

      this.player = new YT.Player(this.playerRef.current, {
        width: 1280,
        height: 720,
        autoplay: false,
        events: {
          onReady: function onReady() {
            _this2.setState({
              loading: false
            });
          },
          onPlaybackQualityChange: function onPlaybackQualityChange(e) {},
          onStateChange: function onStateChange(e) {}
        }
      });
    }
  }, {
    key: "componentDidMount",
    value: function componentDidMount() {
      var _this3 = this;

      window.onYouTubeIframeAPIReady = function () {
        _this3.setupPlayer();
      };

      var tag = document.createElement('script');
      tag.src = "https://www.youtube.com/iframe_api";
      tag.setAttribute("x-youtube", "");
      var firstScriptTag = document.getElementsByTagName('script')[0];
      firstScriptTag.parentNode.insertBefore(tag, firstScriptTag);
    }
  }, {
    key: "componentWillMount",
    value: function componentWillMount() {
      document.body.classList.add('youtube-body');
    }
  }, {
    key: "componentWillUnmount",
    value: function componentWillUnmount() {
      var scripts = document.getElementsByTagName('script');

      var _iterator = YouTube_createForOfIteratorHelper(scripts),
          _step;

      try {
        for (_iterator.s(); !(_step = _iterator.n()).done;) {
          var script = _step.value;

          if (scripts.hasAttribute("x-youtube")) {
            script.parentNode.removeChild(script);
          }
        }
      } catch (err) {
        _iterator.e(err);
      } finally {
        _iterator.f();
      }

      delete window.onYouTubeIframeAPIReady;
      document.body.classList.remove('youtube-body');
    }
  }, {
    key: "render",
    value: function render() {
      var ws = null;
      var playerStyle = {};

      if (!this.state.loading) {
        ws = /*#__PURE__*/react.createElement((build_default()), {
          url: websocketUrl("ws/youtube"),
          onMessage: this.handleData.bind(this)
        });
      }

      var noVideo = null;

      if (this.state.stopped) {
        playerStyle.display = "none";
        noVideo = /*#__PURE__*/react.createElement("div", {
          className: "overlay-hidden youtube-not-loaded p-4 container"
        }, /*#__PURE__*/react.createElement("h1", null, "No Video Loaded"), /*#__PURE__*/react.createElement("p", null, "If you want to embed this into OBS, please add the following Custom CSS:"), /*#__PURE__*/react.createElement("pre", {
          className: "youtube-not-loaded-obs"
        }, /*#__PURE__*/react.createElement("code", null, OBS_CSS.join("\n"))));
      }

      return /*#__PURE__*/react.createElement("div", {
        id: "youtube"
      }, ws, noVideo, /*#__PURE__*/react.createElement(Loading, {
        isLoading: this.state.loading
      }), /*#__PURE__*/react.createElement("div", {
        className: "youtube-container",
        style: playerStyle
      }, /*#__PURE__*/react.createElement("div", {
        ref: this.playerRef,
        className: "youtube-embedded"
      })));
    }
  }]);

  return YouTube;
}(react.Component);


// EXTERNAL MODULE: ./node_modules/core-js/modules/es.string.small.js
var es_string_small = __webpack_require__(37268);
;// CONCATENATED MODULE: ./src/components/Chat.js
function Chat_typeof(obj) { "@babel/helpers - typeof"; if (typeof Symbol === "function" && typeof Symbol.iterator === "symbol") { Chat_typeof = function _typeof(obj) { return typeof obj; }; } else { Chat_typeof = function _typeof(obj) { return obj && typeof Symbol === "function" && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; }; } return Chat_typeof(obj); }




















function Chat_extends() { Chat_extends = Object.assign || function (target) { for (var i = 1; i < arguments.length; i++) { var source = arguments[i]; for (var key in source) { if (Object.prototype.hasOwnProperty.call(source, key)) { target[key] = source[key]; } } } return target; }; return Chat_extends.apply(this, arguments); }

function Chat_classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function Chat_defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function Chat_createClass(Constructor, protoProps, staticProps) { if (protoProps) Chat_defineProperties(Constructor.prototype, protoProps); if (staticProps) Chat_defineProperties(Constructor, staticProps); return Constructor; }

function Chat_inherits(subClass, superClass) { if (typeof superClass !== "function" && superClass !== null) { throw new TypeError("Super expression must either be null or a function"); } subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, writable: true, configurable: true } }); if (superClass) Chat_setPrototypeOf(subClass, superClass); }

function Chat_setPrototypeOf(o, p) { Chat_setPrototypeOf = Object.setPrototypeOf || function _setPrototypeOf(o, p) { o.__proto__ = p; return o; }; return Chat_setPrototypeOf(o, p); }

function Chat_createSuper(Derived) { var hasNativeReflectConstruct = Chat_isNativeReflectConstruct(); return function _createSuperInternal() { var Super = Chat_getPrototypeOf(Derived), result; if (hasNativeReflectConstruct) { var NewTarget = Chat_getPrototypeOf(this).constructor; result = Reflect.construct(Super, arguments, NewTarget); } else { result = Super.apply(this, arguments); } return Chat_possibleConstructorReturn(this, result); }; }

function Chat_possibleConstructorReturn(self, call) { if (call && (Chat_typeof(call) === "object" || typeof call === "function")) { return call; } return Chat_assertThisInitialized(self); }

function Chat_assertThisInitialized(self) { if (self === void 0) { throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); } return self; }

function Chat_isNativeReflectConstruct() { if (typeof Reflect === "undefined" || !Reflect.construct) return false; if (Reflect.construct.sham) return false; if (typeof Proxy === "function") return true; try { Date.prototype.toString.call(Reflect.construct(Date, [], function () {})); return true; } catch (e) { return false; } }

function Chat_getPrototypeOf(o) { Chat_getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf : function _getPrototypeOf(o) { return o.__proto__ || Object.getPrototypeOf(o); }; return Chat_getPrototypeOf(o); }

function Chat_createForOfIteratorHelper(o, allowArrayLike) { var it; if (typeof Symbol === "undefined" || o[Symbol.iterator] == null) { if (Array.isArray(o) || (it = Chat_unsupportedIterableToArray(o)) || allowArrayLike && o && typeof o.length === "number") { if (it) o = it; var i = 0; var F = function F() {}; return { s: F, n: function n() { if (i >= o.length) return { done: true }; return { done: false, value: o[i++] }; }, e: function e(_e) { throw _e; }, f: F }; } throw new TypeError("Invalid attempt to iterate non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); } var normalCompletion = true, didErr = false, err; return { s: function s() { it = o[Symbol.iterator](); }, n: function n() { var step = it.next(); normalCompletion = step.done; return step; }, e: function e(_e2) { didErr = true; err = _e2; }, f: function f() { try { if (!normalCompletion && it.return != null) it.return(); } finally { if (didErr) throw err; } } }; }

function Chat_unsupportedIterableToArray(o, minLen) { if (!o) return; if (typeof o === "string") return Chat_arrayLikeToArray(o, minLen); var n = Object.prototype.toString.call(o).slice(8, -1); if (n === "Object" && o.constructor) n = o.constructor.name; if (n === "Map" || n === "Set") return Array.from(o); if (n === "Arguments" || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(n)) return Chat_arrayLikeToArray(o, minLen); }

function Chat_arrayLikeToArray(arr, len) { if (len == null || len > arr.length) len = arr.length; for (var i = 0, arr2 = new Array(len); i < len; i++) { arr2[i] = arr[i]; } return arr2; }









var Chat_OBS_CSS = ["body.chat-body {", "  font-size: 120%;", "  background-color: rgba(0, 0, 0, 0);", "}", ".chat-message {", "  background-color: rgba(0, 0, 0, 0) !important;", "  text-shadow:", "    -2px -2px 0 #000,", "     0   -2px 0 #000,", "     2px -2px 0 #000,", "     2px  0   0 #000,", "     2px  2px 0 #000,", "     0    2px 0 #000,", "    -2px  2px 0 #000,", "    -2px  0   0 #000;", "}", ".overlay-hidden { display: none; }"];

function isASCII(str) {
  return /^[\x00-\x7F]*$/.test(str);
}
/**
 * Convert and limit the number of messages.
 *
 * @param {Array} messages array of messages
 * @param {number | null} limit limit for the number of messages
 */


function filterMessages(messages, limit, ids) {
  if (limit === null) {
    return {
      ids: ids,
      messages: messages
    };
  }

  var len = messages.length;

  if (len < limit) {
    return {
      ids: ids,
      messages: messages
    };
  }

  var _iterator = Chat_createForOfIteratorHelper(messages.slice(0, len - limit)),
      _step;

  try {
    for (_iterator.s(); !(_step = _iterator.n()).done;) {
      var m = _step.value;
      delete ids[m.id];
    }
  } catch (err) {
    _iterator.e(err);
  } finally {
    _iterator.f();
  }

  messages = messages.slice(len - limit, len);
  return {
    ids: ids,
    messages: messages
  };
}
/**
 * Filter first messages.
 */


function filterUnique(messages) {
  var seen = {};
  var out = [];
  var ids = {};

  var _iterator2 = Chat_createForOfIteratorHelper(messages),
      _step2;

  try {
    for (_iterator2.s(); !(_step2 = _iterator2.n()).done;) {
      var m = _step2.value;

      if (seen[m.user.name]) {
        continue;
      }

      seen[m.user.name] = true;
      ids[m.id] = true;
      out.push(m);
    }
  } catch (err) {
    _iterator2.e(err);
  } finally {
    _iterator2.f();
  }

  return {
    seen: seen,
    ids: ids,
    messages: out
  };
}

function searchLimit(search) {
  var update = search.get("limit");

  if (!update) {
    return {
      limit: null,
      limitText: ""
    };
  }

  update = parseInt(update);

  if (!isFinite(update)) {
    return {
      limit: null,
      limitText: ""
    };
  }

  return {
    limit: update,
    limitText: update.toString()
  };
}

function searchBoolean(search, key) {
  var def = arguments.length > 2 && arguments[2] !== undefined ? arguments[2] : false;
  var update = search.get(key);

  if (update === null) {
    return def;
  }

  switch (update) {
    case "true":
      return true;

    default:
      return false;
  }
}

function searchInactivity(search) {
  var update = search.get("inactivity");

  if (!update) {
    return {
      inactivity: null,
      inactivityText: ""
    };
  }

  update = parseInt(update);

  if (!isFinite(update)) {
    return {
      inactivity: null,
      inactivityText: ""
    };
  }

  return {
    inactivity: update,
    inactivityText: update.toString()
  };
}
/**
 * Computer the collection of seen users from the given set of messages.
 */


function computeSeen(messages) {
  var seen = {};

  var _iterator3 = Chat_createForOfIteratorHelper(messages),
      _step3;

  try {
    for (_iterator3.s(); !(_step3 = _iterator3.n()).done;) {
      var m = _step3.value;
      seen[m.user.name] = true;
    }
  } catch (err) {
    _iterator3.e(err);
  } finally {
    _iterator3.f();
  }

  return seen;
}

var Chat = /*#__PURE__*/function (_React$Component) {
  Chat_inherits(Chat, _React$Component);

  var _super = Chat_createSuper(Chat);

  function Chat(props) {
    var _this;

    Chat_classCallCheck(this, Chat);

    _this = _super.call(this, props);
    _this.api = new Api(apiUrl());
    _this.bottomRef = /*#__PURE__*/react.createRef();
    _this.inactivityTimeout = null;
    _this.cachedEmotes = {};
    var search = new URLSearchParams(_this.props.location.search);

    var _searchLimit = searchLimit(search),
        limit = _searchLimit.limit,
        limitText = _searchLimit.limitText;

    var first = searchBoolean(search, "first");
    var deleted = searchBoolean(search, "deleted");
    var highRes = searchBoolean(search, "highres");
    var rounded = searchBoolean(search, "rounded");

    var _searchInactivity = searchInactivity(search),
        inactivity = _searchInactivity.inactivity,
        inactivityText = _searchInactivity.inactivityText;

    _this.state = {
      messages: [],
      limit: limit,
      limitText: limitText,
      first: first,
      deleted: deleted,
      highRes: highRes,
      rounded: rounded,
      inactivity: inactivity,
      inactivityText: inactivityText,
      seen: {},
      ids: {},
      edit: false,
      changed: false,
      visible: true,
      enabled: true
    };
    return _this;
  }

  Chat_createClass(Chat, [{
    key: "scrollToChat",
    value: function scrollToChat() {
      /* don't scroll if we are editing */
      if (this.state.edit) {
        return;
      }

      this.bottomRef.current.scrollIntoView({
        block: "end",
        behavior: "smooth"
      });
    }
  }, {
    key: "componentDidMount",
    value: function componentDidMount() {
      this.reloadChatMessages();
      this.bumpInactivity();
    }
  }, {
    key: "componentWillMount",
    value: function componentWillMount() {
      document.body.classList.add('chat-body');
    }
  }, {
    key: "componentWillUnmount",
    value: function componentWillUnmount() {
      document.body.classList.remove('chat-body');
    }
    /**
     * Reload chat messages.
     */

  }, {
    key: "reloadChatMessages",
    value: function reloadChatMessages() {
      var _this2 = this;

      this.api.chatMessages().then(function (messages) {
        if (!_this2.state.deleted) {
          messages = messages.filter(function (m) {
            return !m.deleted;
          });
        }

        var update = filterMessages(messages, _this2.state.limit, _this2.state.ids);

        if (_this2.state.first) {
          update = filterUnique(update.messages);
        }

        _this2.setState(update);
      });
    }
    /**
     * Set the inactivity timeout.
     */

  }, {
    key: "bumpInactivity",
    value: function bumpInactivity() {
      var _this3 = this;

      if (this.state.inactivity === null) {
        if (!this.state.visible) {
          this.setState({
            visible: true
          });
        }

        return;
      }

      this.setState({
        visible: true
      });

      if (this.inactivityTimeout !== null) {
        clearTimeout(this.inactivityTimeout);
      }

      this.inactivityTimeout = setTimeout(function () {
        _this3.inactivityTimeout = null;

        _this3.setState({
          visible: false
        });
      }, this.state.inactivity * 1000);
    }
  }, {
    key: "handleData",
    value: function handleData(d) {
      var _this4 = this;

      var data = null;

      try {
        data = JSON.parse(d);
      } catch (e) {
        console.log("failed to deserialize message");
        return;
      }

      switch (data.type) {
        case "message":
          if (this.state.first && !!this.state.seen[data.user.name]) {
            return;
          }

          if (!!this.state.ids[data.id]) {
            return;
          }

          this.setState(function (s) {
            var messages = s.messages.slice();
            messages.push(data);
            var ids = Object.assign({}, s.ids);
            ids[data.id] = true;
            var update = filterMessages(messages, s.limit, ids);

            if (!s.first) {
              return update;
            }

            var seen = Object.assign({}, s.seen);
            seen[data.user.name] = true;
            update.seen = seen;
            return update;
          }, function () {
            _this4.scrollToChat();

            _this4.bumpInactivity();
          });
          break;

        case "delete-by-user":
          this.setState(function (s) {
            var messages = s.messages.map(function (m) {
              if (m.user.name !== data.name) {
                return m;
              }

              m = Object.assign({}, m);
              m.deleted = true;
              return m;
            });
            return {
              messages: messages
            };
          });
          break;

        case "delete-by-id":
          this.setState(function (s) {
            var messages = s.messages.map(function (m) {
              if (m.id !== data.id) {
                return m;
              }

              m = Object.assign({}, m);
              m.deleted = true;
              return m;
            });
            return {
              messages: messages
            };
          });
          break;

        case "delete-all":
          this.setState(function (s) {
            var messages = s.messages.map(function (m) {
              m = Object.assign({}, m);
              m.deleted = true;
              return m;
            });
            return {
              messages: messages
            };
          });
          break;

        case "enabled":
          this.setState({
            enabled: data.enabled
          }, this.bumpInactivity.bind(this));
          break;

        default:
          break;
      }
    }
    /**
     * Create a new element.
     */

  }, {
    key: "createEmote",
    value: function createEmote(rendered, item) {
      var emote = rendered.emotes[item.emote];

      if (!emote) {
        return /*#__PURE__*/react.createElement("span", {
          className: "text failed-emote"
        }, item.emote);
      }

      var emoteUrl = this.pickUrl(emote.urls);
      var props = {
        src: emoteUrl.url,
        title: item.emote
      };
      var width = null;
      var height = calculateHeight(emote);

      if (emoteUrl.size !== null) {
        width = calculateWidth(height, emoteUrl.size);
      }

      props.style = {};

      if (height !== null) {
        props.style.height = "".concat(height, "px");
      }

      if (width !== null) {
        props.style.width = "".concat(width, "px");
      }

      return /*#__PURE__*/react.createElement("img", props);
      /**
       * Calculate the height to use.
       */

      function calculateHeight(emote) {
        var small = emote.urls.small;

        if (small === null || small.size === null) {
          return null;
        }

        return Math.min(32, small.size.height);
      }

      function calculateWidth(height, size) {
        if (height === null) {
          return null;
        }

        return Math.round(size.width * (height / size.height));
      }
    }
    /**
     * Create a new cached emote.
     */

  }, {
    key: "cachedEmote",
    value: function cachedEmote(key, rendered, item) {
      var img = this.cachedEmotes[item.emote];

      if (!img) {
        img = this.createEmote(rendered, item);
        this.cachedEmotes[item.emote] = img;
      }

      return /*#__PURE__*/react.cloneElement(img, {
        key: key
      });
    }
    /**
     * Renders all badges as elements.
     */

  }, {
    key: "renderBadges",
    value: function renderBadges(m) {
      var _this5 = this;

      var rendered = m.rendered;

      if (rendered === null) {
        return null;
      }

      return rendered.badges.map(function (badge, i) {
        var badgeUrl = _this5.pickUrl(badge.urls);

        var props = {};
        props.src = badgeUrl.url;
        props.title = badge.title;
        var className = "chat-badge";

        if (_this5.state.rounded) {
          className = "".concat(className, " rounded");
        }

        if (badge.bg_color !== null) {
          var style = {
            backgroundColor: badge.bg_color
          };
          return /*#__PURE__*/react.createElement("span", {
            key: i,
            style: style,
            className: className
          }, /*#__PURE__*/react.createElement("img", Chat_extends({
            key: i
          }, props)));
        }

        if (badge.badge_url !== null) {
          return /*#__PURE__*/react.createElement("a", {
            href: badge.badge_url
          }, /*#__PURE__*/react.createElement("img", Chat_extends({
            key: i,
            className: className
          }, props)));
        }

        return /*#__PURE__*/react.createElement("img", Chat_extends({
          key: i,
          className: className
        }, props));
      });
    }
  }, {
    key: "renderText",
    value: function renderText(m) {
      var _this6 = this;

      var rendered = m.rendered;

      if (rendered === null) {
        return /*#__PURE__*/react.createElement("span", {
          className: "text"
        }, m.text);
      }

      return rendered.items.map(function (item, i) {
        switch (item.type) {
          case "text":
            return /*#__PURE__*/react.createElement("span", {
              className: "text",
              key: i
            }, item.text);

          case "url":
            return /*#__PURE__*/react.createElement("a", {
              className: "url",
              href: item.url,
              key: i
            }, item.url);

          case "emote":
            return _this6.cachedEmote(i, rendered, item);

          default:
            return /*#__PURE__*/react.createElement("em", {
              key: i
            }, "?");
        }
      });
    }
    /**
     * Pick an appropriate URL depending on settings.
     */

  }, {
    key: "pickUrl",
    value: function pickUrl(urls) {
      var alts = [urls.large, urls.medium, urls.small];

      if (!this.state.highRes) {
        alts = [urls.small, urls.medium, urls.large];
      }

      for (var _i = 0, _alts = alts; _i < _alts.length; _i++) {
        var alt = _alts[_i];

        if (alt !== null) {
          return alt;
        }
      }

      return urls.small;
    }
  }, {
    key: "renderMessages",
    value: function renderMessages(messages) {
      var _this7 = this;

      return messages.map(function (m) {
        var messageClasses = "";

        if (m.deleted) {
          messageClasses = "chat-message-deleted";
        }

        var t = new Date(m.timestamp);
        var timestamp = "[".concat(zeroPad(t.getHours(), 2), ":").concat(zeroPad(t.getMinutes(), 2), "]");
        var nameStyle = {
          color: m.user.color
        };
        var name = m.user.display_name;

        if (!isASCII(name)) {
          name = "".concat(name, " (").concat(m.user.name, ")");
        }

        var badges = _this7.renderBadges(m);

        var text = _this7.renderText(m);

        if (badges !== null) {
          badges = /*#__PURE__*/react.createElement("div", {
            className: "chat-badges"
          }, badges);
        }

        return /*#__PURE__*/react.createElement("div", {
          className: "chat-message ".concat(messageClasses),
          key: m.id
        }, /*#__PURE__*/react.createElement("span", {
          className: "overlay-hidden chat-timestamp"
        }, timestamp), badges, /*#__PURE__*/react.createElement("span", {
          className: "chat-name",
          style: nameStyle
        }, name, ":"), /*#__PURE__*/react.createElement("span", {
          className: "chat-text"
        }, text));
      });
    }
  }, {
    key: "updateSearch",
    value: function updateSearch() {
      if (!this.props.location) {
        return;
      }

      var path = "".concat(this.props.location.pathname);
      var search = new URLSearchParams(this.props.location.search);
      var set = false;

      if (this.state.limit !== null) {
        search.set("limit", this.state.limit.toString());
        set = true;
      } else {
        search.delete("limit");
      }

      if (!!this.state.first) {
        search.set("first", this.state.first.toString());
        set = true;
      } else {
        search.delete("first");
      }

      if (!!this.state.deleted) {
        search.set("deleted", this.state.deleted.toString());
        set = true;
      } else {
        search.delete("deleted");
      }

      if (!!this.state.highRes) {
        search.set("highres", this.state.highRes.toString());
        set = true;
      } else {
        search.delete("highres");
      }

      if (!!this.state.rounded) {
        search.set("rounded", this.state.rounded.toString());
        set = true;
      } else {
        search.delete("rounded");
      }

      if (this.state.inactivity !== null) {
        search.set("inactivity", this.state.inactivity.toString());
        set = true;
      } else {
        search.delete("inactivity");
      }

      if (set) {
        path = "".concat(path, "?").concat(search);
      }

      this.props.history.replace(path);
    }
  }, {
    key: "limitChanged",
    value: function limitChanged(e) {
      var _this8 = this;

      var limitText = e.target.value;
      var limit = parseInt(limitText);

      if (!isFinite(limit) || limit === 0) {
        limit = null;
      }

      this.setState({
        limit: limit,
        limitText: limitText,
        changed: true
      }, function () {
        _this8.updateSearch();
      });
    }
  }, {
    key: "inactivityChanged",
    value: function inactivityChanged(e) {
      var _this9 = this;

      var inactivityText = e.target.value;
      var inactivity = parseInt(inactivityText);

      if (!isFinite(inactivity) || inactivity === 0) {
        inactivity = null;
      }

      this.setState({
        inactivity: inactivity,
        inactivityText: inactivityText,
        changed: true
      }, function () {
        _this9.updateSearch();
      });
    }
  }, {
    key: "firstChanged",
    value: function firstChanged(e) {
      var _this10 = this;

      this.setState({
        first: e.target.checked,
        changed: true
      }, function () {
        _this10.updateSearch();
      });
    }
  }, {
    key: "deletedChanged",
    value: function deletedChanged(e) {
      var _this11 = this;

      this.setState({
        deleted: e.target.checked,
        changed: true
      }, function () {
        _this11.updateSearch();
      });
    }
  }, {
    key: "highResChanged",
    value: function highResChanged(e) {
      var _this12 = this;

      this.cachedEmotes = {};
      this.setState({
        highRes: e.target.checked,
        changed: true
      }, function () {
        _this12.updateSearch();
      });
    }
  }, {
    key: "roundedChanged",
    value: function roundedChanged(e) {
      var _this13 = this;

      this.setState({
        rounded: e.target.checked,
        changed: true
      }, function () {
        _this13.updateSearch();
      });
    }
  }, {
    key: "toggleEdit",
    value: function toggleEdit() {
      var _this14 = this;

      var changed = this.state.changed;
      this.setState({
        edit: !this.state.edit,
        changed: false
      }, function () {
        if (changed) {
          _this14.reloadChatMessages();

          _this14.bumpInactivity();
        }
      });
    }
  }, {
    key: "render",
    value: function render() {
      var ws = /*#__PURE__*/react.createElement((build_default()), {
        url: websocketUrl("ws/messages"),
        onMessage: this.handleData.bind(this)
      });
      var form = /*#__PURE__*/react.createElement(Modal/* default */.Z, {
        className: "chat-settings",
        show: this.state.edit,
        onHide: this.toggleEdit.bind(this)
      }, /*#__PURE__*/react.createElement(Modal/* default.Header */.Z.Header, {
        closeButton: true
      }, /*#__PURE__*/react.createElement(Modal/* default.Title */.Z.Title, null, "Configuration")), /*#__PURE__*/react.createElement(Modal/* default.Body */.Z.Body, null, /*#__PURE__*/react.createElement("p", null, "Configuration options are stored in the URL and can be copy-pasted into the URL used for OBS."), /*#__PURE__*/react.createElement(Form/* default */.Z, null, /*#__PURE__*/react.createElement(Form/* default.Row */.Z.Row, null, /*#__PURE__*/react.createElement(Form/* default.Group */.Z.Group, {
        as: Col/* default */.Z
      }, /*#__PURE__*/react.createElement(Form/* default.Label */.Z.Label, null, "Limit on number of messages:"), /*#__PURE__*/react.createElement(Form/* default.Control */.Z.Control, {
        placeholder: "Disabled",
        type: "number",
        value: this.state.limitText,
        onChange: this.limitChanged.bind(this)
      })), /*#__PURE__*/react.createElement(Form/* default.Group */.Z.Group, {
        as: Col/* default */.Z
      }, /*#__PURE__*/react.createElement(Form/* default.Label */.Z.Label, null, "Inactivity timeout in seconds:"), /*#__PURE__*/react.createElement(Form/* default.Control */.Z.Control, {
        placeholder: "Disabled",
        type: "number",
        value: this.state.inactivityText,
        onChange: this.inactivityChanged.bind(this)
      }))), /*#__PURE__*/react.createElement(Form/* default.Group */.Z.Group, {
        as: Col/* default */.Z
      }, /*#__PURE__*/react.createElement(Form/* default.Check */.Z.Check, {
        id: "first",
        label: "Only show first message",
        type: "checkbox",
        checked: this.state.first,
        onChange: this.firstChanged.bind(this)
      })), /*#__PURE__*/react.createElement(Form/* default.Group */.Z.Group, {
        as: Col/* default */.Z
      }, /*#__PURE__*/react.createElement(Form/* default.Check */.Z.Check, {
        id: "deleted",
        label: "Show deleted",
        type: "checkbox",
        checked: this.state.deleted,
        onChange: this.deletedChanged.bind(this)
      })), /*#__PURE__*/react.createElement(Form/* default.Group */.Z.Group, {
        as: Col/* default */.Z
      }, /*#__PURE__*/react.createElement(Form/* default.Check */.Z.Check, {
        id: "highres",
        label: "High resolution graphics",
        type: "checkbox",
        checked: this.state.highRes,
        onChange: this.highResChanged.bind(this)
      })), /*#__PURE__*/react.createElement(Form/* default.Group */.Z.Group, {
        as: Col/* default */.Z
      }, /*#__PURE__*/react.createElement(Form/* default.Check */.Z.Check, {
        id: "rounded",
        label: "Uses rounded badges",
        type: "checkbox",
        checked: this.state.rounded,
        onChange: this.roundedChanged.bind(this)
      })), /*#__PURE__*/react.createElement("div", null, /*#__PURE__*/react.createElement("hr", null), /*#__PURE__*/react.createElement(Col/* default */.Z, null, /*#__PURE__*/react.createElement("p", null, "If you want to embed this into OBS, please add the following Custom CSS:"), /*#__PURE__*/react.createElement("pre", {
        className: "chat-obs"
      }, /*#__PURE__*/react.createElement("code", null, Chat_OBS_CSS.join("\n"))))))));
      var messagesClasses = "";
      var messagesStyle = {};

      if (!this.state.visible) {
        messagesClasses = "hidden";
        messagesStyle.opacity = "0";
      }

      var messages = this.state.messages;

      if (!this.state.deleted) {
        messages = messages.filter(function (m) {
          return !m.deleted;
        });
      }

      if (messages.length === 0) {
        messages = /*#__PURE__*/react.createElement("div", {
          className: "overlay-hidden chat-no-messages"
        }, "No Messages");
      } else {
        messages = /*#__PURE__*/react.createElement("div", {
          style: messagesStyle,
          className: "chat-messages ".concat(messagesClasses)
        }, this.renderMessages(messages));
      }

      var edit = null;

      if (!this.state.edit) {
        edit = /*#__PURE__*/react.createElement("div", {
          className: "overlay-hidden edit-button"
        }, /*#__PURE__*/react.createElement("a", {
          onClick: this.toggleEdit.bind(this)
        }, /*#__PURE__*/react.createElement(index_es/* FontAwesomeIcon */.G, {
          icon: "cog"
        })));
      }

      var enabled = null;

      if (!this.state.enabled) {
        enabled = /*#__PURE__*/react.createElement(Alert/* default */.Z, {
          className: "chat-warning",
          variant: "warning"
        }, "Chat not enabled in ", /*#__PURE__*/react.createElement(react_router_dom/* Link */.rU, {
          to: "/modules/chat-log"
        }, "settings"), ":", /*#__PURE__*/react.createElement("br", null), /*#__PURE__*/react.createElement("code", null, "chat-log/enabled = false"));
      }

      return /*#__PURE__*/react.createElement("div", {
        id: "chat"
      }, ws, messages, enabled, /*#__PURE__*/react.createElement("div", {
        style: {
          clear: "both"
        },
        ref: this.bottomRef
      }), form, edit);
    }
  }]);

  return Chat;
}(react.Component);


;// CONCATENATED MODULE: ./src/components/Authorization.js
function Authorization_typeof(obj) { "@babel/helpers - typeof"; if (typeof Symbol === "function" && typeof Symbol.iterator === "symbol") { Authorization_typeof = function _typeof(obj) { return typeof obj; }; } else { Authorization_typeof = function _typeof(obj) { return obj && typeof Symbol === "function" && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; }; } return Authorization_typeof(obj); }






















function Authorization_createForOfIteratorHelper(o, allowArrayLike) { var it; if (typeof Symbol === "undefined" || o[Symbol.iterator] == null) { if (Array.isArray(o) || (it = Authorization_unsupportedIterableToArray(o)) || allowArrayLike && o && typeof o.length === "number") { if (it) o = it; var i = 0; var F = function F() {}; return { s: F, n: function n() { if (i >= o.length) return { done: true }; return { done: false, value: o[i++] }; }, e: function e(_e2) { throw _e2; }, f: F }; } throw new TypeError("Invalid attempt to iterate non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); } var normalCompletion = true, didErr = false, err; return { s: function s() { it = o[Symbol.iterator](); }, n: function n() { var step = it.next(); normalCompletion = step.done; return step; }, e: function e(_e3) { didErr = true; err = _e3; }, f: function f() { try { if (!normalCompletion && it.return != null) it.return(); } finally { if (didErr) throw err; } } }; }

function Authorization_slicedToArray(arr, i) { return Authorization_arrayWithHoles(arr) || Authorization_iterableToArrayLimit(arr, i) || Authorization_unsupportedIterableToArray(arr, i) || Authorization_nonIterableRest(); }

function Authorization_nonIterableRest() { throw new TypeError("Invalid attempt to destructure non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); }

function Authorization_unsupportedIterableToArray(o, minLen) { if (!o) return; if (typeof o === "string") return Authorization_arrayLikeToArray(o, minLen); var n = Object.prototype.toString.call(o).slice(8, -1); if (n === "Object" && o.constructor) n = o.constructor.name; if (n === "Map" || n === "Set") return Array.from(o); if (n === "Arguments" || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(n)) return Authorization_arrayLikeToArray(o, minLen); }

function Authorization_arrayLikeToArray(arr, len) { if (len == null || len > arr.length) len = arr.length; for (var i = 0, arr2 = new Array(len); i < len; i++) { arr2[i] = arr[i]; } return arr2; }

function Authorization_iterableToArrayLimit(arr, i) { if (typeof Symbol === "undefined" || !(Symbol.iterator in Object(arr))) return; var _arr = []; var _n = true; var _d = false; var _e = undefined; try { for (var _i = arr[Symbol.iterator](), _s; !(_n = (_s = _i.next()).done); _n = true) { _arr.push(_s.value); if (i && _arr.length === i) break; } } catch (err) { _d = true; _e = err; } finally { try { if (!_n && _i["return"] != null) _i["return"](); } finally { if (_d) throw _e; } } return _arr; }

function Authorization_arrayWithHoles(arr) { if (Array.isArray(arr)) return arr; }

function Authorization_asyncGeneratorStep(gen, resolve, reject, _next, _throw, key, arg) { try { var info = gen[key](arg); var value = info.value; } catch (error) { reject(error); return; } if (info.done) { resolve(value); } else { Promise.resolve(value).then(_next, _throw); } }

function Authorization_asyncToGenerator(fn) { return function () { var self = this, args = arguments; return new Promise(function (resolve, reject) { var gen = fn.apply(self, args); function _next(value) { Authorization_asyncGeneratorStep(gen, resolve, reject, _next, _throw, "next", value); } function _throw(err) { Authorization_asyncGeneratorStep(gen, resolve, reject, _next, _throw, "throw", err); } _next(undefined); }); }; }

function Authorization_classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function Authorization_defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function Authorization_createClass(Constructor, protoProps, staticProps) { if (protoProps) Authorization_defineProperties(Constructor.prototype, protoProps); if (staticProps) Authorization_defineProperties(Constructor, staticProps); return Constructor; }

function Authorization_inherits(subClass, superClass) { if (typeof superClass !== "function" && superClass !== null) { throw new TypeError("Super expression must either be null or a function"); } subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, writable: true, configurable: true } }); if (superClass) Authorization_setPrototypeOf(subClass, superClass); }

function Authorization_setPrototypeOf(o, p) { Authorization_setPrototypeOf = Object.setPrototypeOf || function _setPrototypeOf(o, p) { o.__proto__ = p; return o; }; return Authorization_setPrototypeOf(o, p); }

function Authorization_createSuper(Derived) { var hasNativeReflectConstruct = Authorization_isNativeReflectConstruct(); return function _createSuperInternal() { var Super = Authorization_getPrototypeOf(Derived), result; if (hasNativeReflectConstruct) { var NewTarget = Authorization_getPrototypeOf(this).constructor; result = Reflect.construct(Super, arguments, NewTarget); } else { result = Super.apply(this, arguments); } return Authorization_possibleConstructorReturn(this, result); }; }

function Authorization_possibleConstructorReturn(self, call) { if (call && (Authorization_typeof(call) === "object" || typeof call === "function")) { return call; } return Authorization_assertThisInitialized(self); }

function Authorization_assertThisInitialized(self) { if (self === void 0) { throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); } return self; }

function Authorization_isNativeReflectConstruct() { if (typeof Reflect === "undefined" || !Reflect.construct) return false; if (Reflect.construct.sham) return false; if (typeof Proxy === "function") return true; try { Date.prototype.toString.call(Reflect.construct(Date, [], function () {})); return true; } catch (e) { return false; } }

function Authorization_getPrototypeOf(o) { Authorization_getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf : function _getPrototypeOf(o) { return o.__proto__ || Object.getPrototypeOf(o); }; return Authorization_getPrototypeOf(o); }







/**
 * Special role that everyone belongs to.
 */

var EVERYONE = "@everyone";
var STREAMER = "@streamer";
var MODERATOR = "@moderator";
var SUBSCRIBER = "@subscriber";
/**
 * Check if the given role is a risky role.
 *
 * @param {string} role name of the role to check.
 */

function is_risky_role(role) {
  switch (role) {
    case EVERYONE:
      return true;

    case SUBSCRIBER:
      return true;

    default:
      return false;
  }
}

var Authorization = /*#__PURE__*/function (_React$Component) {
  Authorization_inherits(Authorization, _React$Component);

  var _super = Authorization_createSuper(Authorization);

  function Authorization(props) {
    var _this;

    Authorization_classCallCheck(this, Authorization);

    _this = _super.call(this, props);
    var search = new URLSearchParams(_this.props.location.search);
    _this.api = _this.props.api;
    _this.state = {
      loading: false,
      error: null,
      data: null,
      filter: search.get("q") || "",
      checked: {
        title: "",
        prompt: "",
        visible: false,
        verify: function verify() {}
      }
    };
    return _this;
  }

  Authorization_createClass(Authorization, [{
    key: "componentDidMount",
    value: function () {
      var _componentDidMount = Authorization_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee() {
        return regeneratorRuntime.wrap(function _callee$(_context) {
          while (1) {
            switch (_context.prev = _context.next) {
              case 0:
                _context.next = 2;
                return this.list();

              case 2:
              case "end":
                return _context.stop();
            }
          }
        }, _callee, this);
      }));

      function componentDidMount() {
        return _componentDidMount.apply(this, arguments);
      }

      return componentDidMount;
    }()
    /**
     * Update the current filter.
     */

  }, {
    key: "setFilter",
    value: function setFilter(filter) {
      var path = "".concat(this.props.location.pathname);

      if (!!filter) {
        var search = new URLSearchParams(this.props.location.search);
        search.set("q", filter);
        path = "".concat(path, "?").concat(search);
      }

      this.props.history.replace(path);
      this.setState({
        filter: filter
      });
    }
    /**
     * Refresh the list of after streams.
     */

  }, {
    key: "list",
    value: function () {
      var _list = Authorization_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee2() {
        var requests, _yield$Promise$all, _yield$Promise$all2, roles, scopes, grants, allowsObject, _iterator, _step, _step$value, scope, role;

        return regeneratorRuntime.wrap(function _callee2$(_context2) {
          while (1) {
            switch (_context2.prev = _context2.next) {
              case 0:
                this.setState({
                  loading: true
                });
                requests = [this.api.authRoles(this.props.current.channel), this.api.authScopes(this.props.current.channel), this.api.authGrants(this.props.current.channel)];
                _context2.prev = 2;
                _context2.next = 5;
                return Promise.all(requests);

              case 5:
                _yield$Promise$all = _context2.sent;
                _yield$Promise$all2 = Authorization_slicedToArray(_yield$Promise$all, 3);
                roles = _yield$Promise$all2[0];
                scopes = _yield$Promise$all2[1];
                grants = _yield$Promise$all2[2];
                allowsObject = {};
                _iterator = Authorization_createForOfIteratorHelper(grants);

                try {
                  for (_iterator.s(); !(_step = _iterator.n()).done;) {
                    _step$value = Authorization_slicedToArray(_step.value, 2), scope = _step$value[0], role = _step$value[1];
                    allowsObject["".concat(scope, ":").concat(role)] = true;
                  }
                } catch (err) {
                  _iterator.e(err);
                } finally {
                  _iterator.f();
                }

                this.setState({
                  loading: false,
                  error: null,
                  data: {
                    roles: roles,
                    scopes: scopes,
                    grants: allowsObject
                  }
                });
                _context2.next = 19;
                break;

              case 16:
                _context2.prev = 16;
                _context2.t0 = _context2["catch"](2);
                this.setState({
                  loading: false,
                  error: "failed to request after streams: ".concat(_context2.t0),
                  data: null
                });

              case 19:
              case "end":
                return _context2.stop();
            }
          }
        }, _callee2, this, [[2, 16]]);
      }));

      function list() {
        return _list.apply(this, arguments);
      }

      return list;
    }()
  }, {
    key: "deny",
    value: function () {
      var _deny = Authorization_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee3(scope, role) {
        return regeneratorRuntime.wrap(function _callee3$(_context3) {
          while (1) {
            switch (_context3.prev = _context3.next) {
              case 0:
                this.setState({
                  loading: true
                });
                _context3.prev = 1;
                _context3.next = 4;
                return this.api.authDeleteGrant(scope, role);

              case 4:
                _context3.next = 6;
                return this.list();

              case 6:
                _context3.next = 11;
                break;

              case 8:
                _context3.prev = 8;
                _context3.t0 = _context3["catch"](1);
                this.setState({
                  loading: false,
                  error: "failed to insert an allow permit: ".concat(_context3.t0)
                });

              case 11:
              case "end":
                return _context3.stop();
            }
          }
        }, _callee3, this, [[1, 8]]);
      }));

      function deny(_x, _x2) {
        return _deny.apply(this, arguments);
      }

      return deny;
    }()
  }, {
    key: "allow",
    value: function () {
      var _allow = Authorization_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee4(scope, role) {
        return regeneratorRuntime.wrap(function _callee4$(_context4) {
          while (1) {
            switch (_context4.prev = _context4.next) {
              case 0:
                this.setState({
                  loading: true
                });
                _context4.prev = 1;
                _context4.next = 4;
                return this.api.authInsertGrant({
                  scope: scope,
                  role: role
                });

              case 4:
                _context4.next = 6;
                return this.list();

              case 6:
                _context4.next = 11;
                break;

              case 8:
                _context4.prev = 8;
                _context4.t0 = _context4["catch"](1);
                this.setState({
                  loading: false,
                  error: "failed to insert an allow permit: ".concat(_context4.t0)
                });

              case 11:
              case "end":
                return _context4.stop();
            }
          }
        }, _callee4, this, [[1, 8]]);
      }));

      function allow(_x3, _x4) {
        return _allow.apply(this, arguments);
      }

      return allow;
    }()
  }, {
    key: "filtered",
    value: function filtered(data) {
      if (!this.state.filter) {
        return data;
      }

      var scopes = data.scopes;

      if (this.state.filter.startsWith('^')) {
        var filter = this.state.filter.substring(1);
        scopes = scopes.filter(function (scope) {
          return scope.scope.startsWith(filter);
        });
      } else {
        var parts = this.state.filter.split(" ").map(function (f) {
          return f.toLowerCase();
        });
        scopes = data.scopes.filter(function (scope) {
          return parts.every(function (p) {
            if (scope.scope.toLowerCase().indexOf(p) != -1) {
              return true;
            }

            return scope.doc.toLowerCase().indexOf(p) != -1;
          });
        });
      }

      return Object.assign({}, data, {
        scopes: scopes
      });
    }
    /**
     * Render authentication button.
     */

  }, {
    key: "renderAuthButton",
    value: function renderAuthButton(scope, role, grants) {
      var _this2 = this;

      var has_implicit = null;
      var title = null;

      var is_allowed = function is_allowed(role) {
        return grants["".concat(scope.scope, ":").concat(role)] || false;
      };

      var test_implicit = function test_implicit(roles) {
        var _iterator2 = Authorization_createForOfIteratorHelper(roles),
            _step2;

        try {
          for (_iterator2.s(); !(_step2 = _iterator2.n()).done;) {
            var _role = _step2.value;

            if (is_allowed(_role)) {
              return _role;
            }
          }
        } catch (err) {
          _iterator2.e(err);
        } finally {
          _iterator2.f();
        }

        return null;
      };

      switch (role.role) {
        case EVERYONE:
          break;

        case STREAMER:
          has_implicit = test_implicit([EVERYONE, SUBSCRIBER]) || false;
          break;

        default:
          has_implicit = test_implicit([EVERYONE]) || false;
          break;
      }

      var allowed = !!has_implicit || is_allowed(role.role) || false;
      var button = null;

      if (!!has_implicit) {
        title = "allowed because ".concat(has_implicit, " is allowed");
      } else {
        if (allowed) {
          title = "".concat(scope.scope, " scope is allowed by ").concat(role.role);
        } else {
          title = "".concat(scope.scope, " scope is denied to ").concat(role.role);
        }
      }

      if (!!has_implicit) {
        button = /*#__PURE__*/react.createElement(Button/* default */.Z, {
          className: "auth-boolean-icon",
          disabled: true,
          title: title,
          size: "sm",
          variant: "secondary"
        }, /*#__PURE__*/react.createElement(True, null));
      } else {
        if (allowed) {
          var deny = function deny() {
            return _this2.deny(scope.scope, role.role);
          };

          button = /*#__PURE__*/react.createElement(Button/* default */.Z, {
            className: "auth-boolean-icon",
            title: title,
            size: "sm",
            variant: "success",
            onClick: deny
          }, /*#__PURE__*/react.createElement(True, null));
        } else {
          var allow = function allow() {
            return _this2.allow(scope.scope, role.role);
          };

          if (is_risky_role(role.role) && scope.risk === "high") {
            allow = function allow() {
              _this2.setState({
                checked: {
                  title: "Grant high-risk scope?",
                  prompt: /*#__PURE__*/react.createElement("div", null, /*#__PURE__*/react.createElement("div", null, /*#__PURE__*/react.createElement("b", null, scope.scope), " is a ", /*#__PURE__*/react.createElement("b", null, "high risk"), " scope."), /*#__PURE__*/react.createElement("div", {
                    className: "mb-3"
                  }, "Granting it to ", /*#__PURE__*/react.createElement("b", null, role.role), " might pose a ", /*#__PURE__*/react.createElement("b", null, "security risk"), "."), /*#__PURE__*/react.createElement("div", {
                    className: "align-center"
                  }, /*#__PURE__*/react.createElement("em", null, "Are you sure?"))),
                  visible: true,
                  verify: function verify() {
                    return _this2.allow(scope.scope, role.role);
                  }
                }
              });
            };
          }

          button = /*#__PURE__*/react.createElement(Button/* default */.Z, {
            className: "auth-boolean-icon",
            title: title,
            size: "sm",
            variant: "danger",
            onClick: allow
          }, /*#__PURE__*/react.createElement(False, null));
        }
      }

      return /*#__PURE__*/react.createElement("td", {
        key: role.role,
        align: "center"
      }, button);
    }
    /**
     * Render a single group body.
     */

  }, {
    key: "renderScope",
    value: function renderScope(scope, data) {
      var _this3 = this;

      var nameOverride = arguments.length > 2 && arguments[2] !== undefined ? arguments[2] : null;
      return /*#__PURE__*/react.createElement("tr", {
        key: scope.scope
      }, /*#__PURE__*/react.createElement("td", {
        className: "auth-scope-key"
      }, /*#__PURE__*/react.createElement("div", {
        className: "auth-scope-key-name"
      }, nameOverride || scope.scope), /*#__PURE__*/react.createElement("div", {
        className: "auth-scope-key-doc"
      }, /*#__PURE__*/react.createElement(react_markdown, {
        source: scope.doc
      }))), data.roles.map(function (role) {
        return _this3.renderAuthButton(scope, role, data.grants);
      }));
    }
    /**
     * Render a single group.
     */

  }, {
    key: "renderGroup",
    value: function renderGroup(group, name, data) {
      var _this4 = this;

      var setFilter = function setFilter(filter) {
        return function () {
          return _this4.setFilter("^".concat(filter, "/"));
        };
      };

      return [/*#__PURE__*/react.createElement("tr", {
        key: "title:".concat(name),
        className: "auth-scope-short"
      }, /*#__PURE__*/react.createElement("td", {
        colSpan: data.roles.length + 1,
        className: "auth-group",
        title: "Filter for \"".concat(name, "\""),
        onClick: setFilter(name)
      }, name, /*#__PURE__*/react.createElement("a", {
        className: "auth-group-filter"
      }, /*#__PURE__*/react.createElement(index_es/* FontAwesomeIcon */.G, {
        icon: "search"
      })))), group.map(function (d) {
        return _this4.renderScope(d.data, data, d.short);
      })];
    }
  }, {
    key: "render",
    value: function render() {
      var _this5 = this;

      var content = null;
      var data = null;

      if (this.state.data) {
        data = this.filtered(this.state.data);
      }

      if (data && data.scopes.length > 0) {
        var _partition = partition(data.scopes, function (s) {
          return s.scope;
        }),
            order = _partition.order,
            groups = _partition.groups,
            def = _partition.def;

        content = /*#__PURE__*/react.createElement(Table/* default */.Z, {
          key: name,
          className: "mb-0"
        }, /*#__PURE__*/react.createElement("tbody", null, /*#__PURE__*/react.createElement("tr", null, /*#__PURE__*/react.createElement("th", {
          className: "table-fill"
        }), data.roles.map(function (role) {
          return /*#__PURE__*/react.createElement("th", {
            key: role.role,
            title: role.doc
          }, /*#__PURE__*/react.createElement("div", {
            className: "auth-role-name"
          }, role.role));
        })), def.map(function (scope) {
          return _this5.renderScope(scope, data);
        }), order.map(function (name) {
          return _this5.renderGroup(groups[name], name, data);
        })));
      } else {
        content = /*#__PURE__*/react.createElement(Alert/* default */.Z, {
          variant: "info"
        }, "No Scopes!");
      }

      var clear = null;

      if (!!this.state.filter) {
        var clearFilter = function clearFilter() {
          return _this5.setFilter("");
        };

        clear = /*#__PURE__*/react.createElement(InputGroup/* default.Append */.Z.Append, null, /*#__PURE__*/react.createElement(Button/* default */.Z, {
          variant: "primary",
          onClick: clearFilter
        }, "Clear Filter"));
      }

      var filterOnChange = function filterOnChange(e) {
        return _this5.setFilter(e.target.value);
      };

      var filter = /*#__PURE__*/react.createElement(Form/* default */.Z, {
        className: "mt-4 mb-4"
      }, /*#__PURE__*/react.createElement(InputGroup/* default */.Z, null, /*#__PURE__*/react.createElement(Form/* default.Control */.Z.Control, {
        value: this.state.filter,
        placeholder: "Filter Scopes",
        onChange: filterOnChange
      }), clear));

      var handleClose = function handleClose() {
        _this5.setState({
          checked: {
            title: "",
            prompt: "",
            visible: false,
            verify: function verify() {}
          }
        });
      };

      var handleVerify = function handleVerify() {
        _this5.state.checked.verify();

        handleClose();
      };

      var modal = /*#__PURE__*/react.createElement(Modal/* default */.Z, {
        show: !!this.state.checked.visible,
        onHide: handleClose
      }, /*#__PURE__*/react.createElement(Modal/* default.Header */.Z.Header, {
        closeButton: true
      }, /*#__PURE__*/react.createElement(Modal/* default.Title */.Z.Title, {
        className: "align-center"
      }, this.state.checked.title)), /*#__PURE__*/react.createElement(Modal/* default.Body */.Z.Body, {
        className: "align-center"
      }, this.state.checked.prompt), /*#__PURE__*/react.createElement(Modal/* default.Footer */.Z.Footer, null, /*#__PURE__*/react.createElement(Button/* default */.Z, {
        variant: "secondary",
        onClick: handleClose
      }, "No"), /*#__PURE__*/react.createElement(Button/* default */.Z, {
        variant: "primary",
        onClick: handleVerify
      }, "Yes")));
      return /*#__PURE__*/react.createElement("div", null, /*#__PURE__*/react.createElement("h1", {
        className: "oxi-page-title"
      }, "Authorization"), /*#__PURE__*/react.createElement(Loading, {
        isLoading: this.state.loading
      }), /*#__PURE__*/react.createElement(Error_Loading, {
        error: this.state.error
      }), filter, content, modal);
    }
  }]);

  return Authorization;
}(react.Component);


// EXTERNAL MODULE: ./node_modules/semver/index.js
var semver = __webpack_require__(81249);
;// CONCATENATED MODULE: ./src/logo.png
/* harmony default export */ const logo = (__webpack_require__.p + "d1c48652deb589dccee97a958403d081.png");
;// CONCATENATED MODULE: ./src/index.js
function src_typeof(obj) { "@babel/helpers - typeof"; if (typeof Symbol === "function" && typeof Symbol.iterator === "symbol") { src_typeof = function _typeof(obj) { return typeof obj; }; } else { src_typeof = function _typeof(obj) { return obj && typeof Symbol === "function" && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; }; } return src_typeof(obj); }















function src_asyncGeneratorStep(gen, resolve, reject, _next, _throw, key, arg) { try { var info = gen[key](arg); var value = info.value; } catch (error) { reject(error); return; } if (info.done) { resolve(value); } else { Promise.resolve(value).then(_next, _throw); } }

function src_asyncToGenerator(fn) { return function () { var self = this, args = arguments; return new Promise(function (resolve, reject) { var gen = fn.apply(self, args); function _next(value) { src_asyncGeneratorStep(gen, resolve, reject, _next, _throw, "next", value); } function _throw(err) { src_asyncGeneratorStep(gen, resolve, reject, _next, _throw, "throw", err); } _next(undefined); }); }; }

function src_extends() { src_extends = Object.assign || function (target) { for (var i = 1; i < arguments.length; i++) { var source = arguments[i]; for (var key in source) { if (Object.prototype.hasOwnProperty.call(source, key)) { target[key] = source[key]; } } } return target; }; return src_extends.apply(this, arguments); }

function src_classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function src_defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function src_createClass(Constructor, protoProps, staticProps) { if (protoProps) src_defineProperties(Constructor.prototype, protoProps); if (staticProps) src_defineProperties(Constructor, staticProps); return Constructor; }

function src_inherits(subClass, superClass) { if (typeof superClass !== "function" && superClass !== null) { throw new TypeError("Super expression must either be null or a function"); } subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, writable: true, configurable: true } }); if (superClass) src_setPrototypeOf(subClass, superClass); }

function src_setPrototypeOf(o, p) { src_setPrototypeOf = Object.setPrototypeOf || function _setPrototypeOf(o, p) { o.__proto__ = p; return o; }; return src_setPrototypeOf(o, p); }

function src_createSuper(Derived) { var hasNativeReflectConstruct = src_isNativeReflectConstruct(); return function _createSuperInternal() { var Super = src_getPrototypeOf(Derived), result; if (hasNativeReflectConstruct) { var NewTarget = src_getPrototypeOf(this).constructor; result = Reflect.construct(Super, arguments, NewTarget); } else { result = Super.apply(this, arguments); } return src_possibleConstructorReturn(this, result); }; }

function src_possibleConstructorReturn(self, call) { if (call && (src_typeof(call) === "object" || typeof call === "function")) { return call; } return src_assertThisInitialized(self); }

function src_assertThisInitialized(self) { if (self === void 0) { throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); } return self; }

function src_isNativeReflectConstruct() { if (typeof Reflect === "undefined" || !Reflect.construct) return false; if (Reflect.construct.sham) return false; if (typeof Proxy === "function") return true; try { Date.prototype.toString.call(Reflect.construct(Date, [], function () {})); return true; } catch (e) { return false; } }

function src_getPrototypeOf(o) { src_getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf : function _getPrototypeOf(o) { return o.__proto__ || Object.getPrototypeOf(o); }; return src_getPrototypeOf(o); }





























/**
 * Required spotify configuration.
 */

var SECRET_KEY_CONFIG = "remote/secret-key";
var RouteLayout = (0,react_router/* withRouter */.EN)(function (props) {
  return /*#__PURE__*/react.createElement(Layout, props);
});

var AfterStreamsPage = /*#__PURE__*/function (_React$Component) {
  src_inherits(AfterStreamsPage, _React$Component);

  var _super = src_createSuper(AfterStreamsPage);

  function AfterStreamsPage(props) {
    var _this;

    src_classCallCheck(this, AfterStreamsPage);

    _this = _super.call(this, props);
    _this.api = new Api(apiUrl());
    return _this;
  }

  src_createClass(AfterStreamsPage, [{
    key: "render",
    value: function render() {
      return /*#__PURE__*/react.createElement(RouteLayout, null, /*#__PURE__*/react.createElement(AfterStreams, {
        api: this.api
      }));
    }
  }]);

  return AfterStreamsPage;
}(react.Component);

var SettingsPage = /*#__PURE__*/function (_React$Component2) {
  src_inherits(SettingsPage, _React$Component2);

  var _super2 = src_createSuper(SettingsPage);

  function SettingsPage(props) {
    var _this2;

    src_classCallCheck(this, SettingsPage);

    _this2 = _super2.call(this, props);
    _this2.api = new Api(apiUrl());
    return _this2;
  }

  src_createClass(SettingsPage, [{
    key: "render",
    value: function render() {
      return /*#__PURE__*/react.createElement(RouteLayout, null, /*#__PURE__*/react.createElement("h1", {
        className: "oxi-page-title"
      }, "Settings"), /*#__PURE__*/react.createElement(Settings, src_extends({
        group: true,
        api: this.api,
        filterable: true
      }, this.props)));
    }
  }]);

  return SettingsPage;
}(react.Component);

var CachePage = /*#__PURE__*/function (_React$Component3) {
  src_inherits(CachePage, _React$Component3);

  var _super3 = src_createSuper(CachePage);

  function CachePage(props) {
    var _this3;

    src_classCallCheck(this, CachePage);

    _this3 = _super3.call(this, props);
    _this3.api = new Api(apiUrl());
    return _this3;
  }

  src_createClass(CachePage, [{
    key: "render",
    value: function render() {
      return /*#__PURE__*/react.createElement(RouteLayout, null, /*#__PURE__*/react.createElement("h1", {
        className: "oxi-page-title"
      }, "Cache"), /*#__PURE__*/react.createElement(Cache, src_extends({
        api: this.api
      }, this.props)));
    }
  }]);

  return CachePage;
}(react.Component);

var ModulesPage = /*#__PURE__*/function (_React$Component4) {
  src_inherits(ModulesPage, _React$Component4);

  var _super4 = src_createSuper(ModulesPage);

  function ModulesPage(props) {
    var _this4;

    src_classCallCheck(this, ModulesPage);

    _this4 = _super4.call(this, props);
    _this4.api = new Api(apiUrl());
    return _this4;
  }

  src_createClass(ModulesPage, [{
    key: "render",
    value: function render() {
      return /*#__PURE__*/react.createElement(RouteLayout, null, /*#__PURE__*/react.createElement(Modules, src_extends({
        api: this.api
      }, this.props)));
    }
  }]);

  return ModulesPage;
}(react.Component);

var ImportExportPage = /*#__PURE__*/function (_React$Component5) {
  src_inherits(ImportExportPage, _React$Component5);

  var _super5 = src_createSuper(ImportExportPage);

  function ImportExportPage(props) {
    var _this5;

    src_classCallCheck(this, ImportExportPage);

    _this5 = _super5.call(this, props);
    _this5.api = new Api(apiUrl());
    return _this5;
  }

  src_createClass(ImportExportPage, [{
    key: "render",
    value: function render() {
      return /*#__PURE__*/react.createElement(RouteLayout, null, /*#__PURE__*/react.createElement(ImportExport, src_extends({
        api: this.api
      }, this.props)));
    }
  }]);

  return ImportExportPage;
}(react.Component);

var AuthorizedPage = /*#__PURE__*/function (_React$Component6) {
  src_inherits(AuthorizedPage, _React$Component6);

  var _super6 = src_createSuper(AuthorizedPage);

  function AuthorizedPage(props, page) {
    var _this6;

    src_classCallCheck(this, AuthorizedPage);

    _this6 = _super6.call(this, props);
    _this6.state = {
      current: null,
      error: null
    };
    _this6.api = new Api(apiUrl());
    _this6.page = page;
    return _this6;
  }

  src_createClass(AuthorizedPage, [{
    key: "componentDidMount",
    value: function () {
      var _componentDidMount = src_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee() {
        var current;
        return regeneratorRuntime.wrap(function _callee$(_context) {
          while (1) {
            switch (_context.prev = _context.next) {
              case 0:
                _context.prev = 0;
                _context.next = 3;
                return this.api.current();

              case 3:
                current = _context.sent;

                if (current.channel) {
                  this.setState({
                    current: current
                  });
                }

                _context.next = 10;
                break;

              case 7:
                _context.prev = 7;
                _context.t0 = _context["catch"](0);
                this.setState({
                  error: "Failed to get current user: ".concat(_context.t0)
                });

              case 10:
              case "end":
                return _context.stop();
            }
          }
        }, _callee, this, [[0, 7]]);
      }));

      function componentDidMount() {
        return _componentDidMount.apply(this, arguments);
      }

      return componentDidMount;
    }()
  }, {
    key: "render",
    value: function render() {
      var _this7 = this;

      if (this.state.error) {
        return /*#__PURE__*/react.createElement(RouteLayout, null, /*#__PURE__*/react.createElement(Error_Loading, {
          error: this.state.error
        }));
      }

      if (!this.state.current) {
        return /*#__PURE__*/react.createElement(RouteLayout, null, /*#__PURE__*/react.createElement(Loading, null, "Loading user information"));
      }

      var children = react.Children.map(this.props.children, function (child) {
        return /*#__PURE__*/react.cloneElement(child, {
          api: _this7.api,
          current: _this7.state.current
        });
      });
      return /*#__PURE__*/react.createElement(RouteLayout, null, children);
    }
  }]);

  return AuthorizedPage;
}(react.Component);

function HeaderAction(props) {
  var link = {};
  var icon = {};

  if (!!props.icon) {
    icon.icon = props.icon;
  }

  if (!!props.to) {
    link.to = props.to;
  }

  return /*#__PURE__*/react.createElement(react_router_dom/* Link */.rU, src_extends({
    className: "oxi-header-action"
  }, link), /*#__PURE__*/react.createElement(index_es/* FontAwesomeIcon */.G, icon), "\xA0", props.children);
}

var IndexPage = /*#__PURE__*/function (_React$Component7) {
  src_inherits(IndexPage, _React$Component7);

  var _super7 = src_createSuper(IndexPage);

  function IndexPage(props) {
    var _this8;

    src_classCallCheck(this, IndexPage);

    _this8 = _super7.call(this, props);
    _this8.api = new Api(apiUrl());
    var q = new URLSearchParams(props.location.search);
    _this8.state = {
      version: null,
      receivedKey: q.get("received-key") === "true"
    };
    return _this8;
  }

  src_createClass(IndexPage, [{
    key: "componentDidMount",
    value: function componentDidMount() {
      var _this9 = this;

      this.api.version().then(function (version) {
        _this9.setState({
          version: version
        });
      });
    }
    /**
     * Get default version information.
     */

  }, {
    key: "defaultVersionInfo",
    value: function defaultVersionInfo(version) {
      return /*#__PURE__*/react.createElement(Alert/* default */.Z, {
        variant: "info",
        style: {
          textAlign: "center"
        }
      }, "You're running the latest version of ", /*#__PURE__*/react.createElement("b", null, "OxidizeBot"), " (", /*#__PURE__*/react.createElement("b", null, version), ").");
    }
    /**
     * Get information on new versions available, or the current version of the bot.
     */

  }, {
    key: "renderVersionInfo",
    value: function renderVersionInfo() {
      var version = /*#__PURE__*/react.createElement(InlineLoading_Loading, null);
      var latest = null;

      if (this.state.version) {
        version = this.state.version.version;
        latest = this.state.version.latest;
      }

      if (!latest || !semver.valid(latest.version) || !semver.valid(version)) {
        return this.defaultVersionInfo(version);
      }

      if (!semver.gt(latest.version, version) || !latest.asset) {
        return this.defaultVersionInfo(version);
      }

      return /*#__PURE__*/react.createElement(Alert/* default */.Z, {
        variant: "warning",
        className: "center"
      }, /*#__PURE__*/react.createElement("div", {
        className: "mb-2",
        style: {
          fontSize: "150%"
        }
      }, "OxidizeBot ", /*#__PURE__*/react.createElement("b", null, latest.version), " is available (current: ", /*#__PURE__*/react.createElement("b", null, version), ")."), /*#__PURE__*/react.createElement("div", null, "Download link:\xA0", /*#__PURE__*/react.createElement("a", {
        href: latest.asset.download_url
      }, latest.asset.name)));
    }
  }, {
    key: "render",
    value: function render() {
      var receivedKey = null;

      if (this.state.receivedKey) {
        receivedKey = /*#__PURE__*/react.createElement(Alert/* default */.Z, {
          variant: "info",
          className: "center"
        }, /*#__PURE__*/react.createElement(index_es/* FontAwesomeIcon */.G, {
          icon: "key"
        }), " Received new ", /*#__PURE__*/react.createElement("b", null, "Secret Key"), " from setbac.tv");
      }

      var versionInfo = this.renderVersionInfo();
      return /*#__PURE__*/react.createElement(RouteLayout, null, versionInfo, /*#__PURE__*/react.createElement(Row/* default */.Z, null, /*#__PURE__*/react.createElement(Col/* default */.Z, {
        lg: "6"
      }, /*#__PURE__*/react.createElement("h4", null, "Connections", /*#__PURE__*/react.createElement(HeaderAction, {
        to: "/modules/remote",
        icon: "wrench"
      }, "Configure")), receivedKey, /*#__PURE__*/react.createElement(Connections, {
        api: this.api
      })), /*#__PURE__*/react.createElement(Col/* default */.Z, {
        lg: "6"
      }, /*#__PURE__*/react.createElement("h4", null, "Devices"), /*#__PURE__*/react.createElement(Authentication, {
        api: this.api
      }))), /*#__PURE__*/react.createElement(ConfigurationPrompt, {
        api: this.api,
        hideWhenConfigured: true,
        filter: {
          key: [SECRET_KEY_CONFIG]
        }
      }, /*#__PURE__*/react.createElement("h4", null, /*#__PURE__*/react.createElement("b", null, "Action Required"), ": Configure your connection to ", /*#__PURE__*/react.createElement("a", {
        href: "https://setbac.tv"
      }, "setbac.tv")), "Go to ", /*#__PURE__*/react.createElement("a", {
        href: "https://setbac.tv/connections"
      }, "your connections"), " and login using Twitch. Generate a new key and configure it below."));
    }
  }]);

  return IndexPage;
}(react.Component);

var Layout = /*#__PURE__*/function (_React$Component8) {
  src_inherits(Layout, _React$Component8);

  var _super8 = src_createSuper(Layout);

  function Layout(props) {
    src_classCallCheck(this, Layout);

    return _super8.call(this, props);
  }

  src_createClass(Layout, [{
    key: "goToWebsite",
    value: function goToWebsite() {
      location.href = "https://setbac.tv";
    }
  }, {
    key: "render",
    value: function render() {
      var path = this.props.location.pathname;
      return /*#__PURE__*/react.createElement(react.Fragment, null, /*#__PURE__*/react.createElement(Navbar/* default */.Z, {
        className: "mb-3",
        bg: "dark",
        variant: "dark",
        expand: "md"
      }, /*#__PURE__*/react.createElement(Container/* default */.Z, null, /*#__PURE__*/react.createElement(Navbar/* default.Brand */.Z.Brand, {
        as: react_router_dom/* Link */.rU,
        to: "/"
      }, /*#__PURE__*/react.createElement("img", {
        src: logo,
        alt: "Logo",
        width: "32",
        height: "32"
      })), /*#__PURE__*/react.createElement(Navbar/* default.Collapse */.Z.Collapse, null, /*#__PURE__*/react.createElement(Nav/* default */.Z, null, /*#__PURE__*/react.createElement(Nav/* default.Link */.Z.Link, {
        as: react_router_dom/* Link */.rU,
        active: path.startsWith("/modules"),
        to: "/modules"
      }, "Modules"), /*#__PURE__*/react.createElement(Nav/* default.Link */.Z.Link, {
        as: react_router_dom/* Link */.rU,
        active: path === "/authorization",
        to: "/authorization"
      }, "Authorization"), /*#__PURE__*/react.createElement(NavDropdown/* default */.Z, {
        title: "Chat"
      }, /*#__PURE__*/react.createElement(NavDropdown/* default.Item */.Z.Item, {
        as: react_router_dom/* Link */.rU,
        active: path === "/after-streams",
        to: "/after-streams"
      }, "After Streams"), /*#__PURE__*/react.createElement(NavDropdown/* default.Item */.Z.Item, {
        as: react_router_dom/* Link */.rU,
        active: path === "/aliases",
        to: "/aliases"
      }, "Aliases"), /*#__PURE__*/react.createElement(NavDropdown/* default.Item */.Z.Item, {
        as: react_router_dom/* Link */.rU,
        active: path === "/commands",
        to: "/commands"
      }, "Commands"), /*#__PURE__*/react.createElement(NavDropdown/* default.Item */.Z.Item, {
        as: react_router_dom/* Link */.rU,
        active: path === "/promotions",
        to: "/promotions"
      }, "Promotions"), /*#__PURE__*/react.createElement(NavDropdown/* default.Item */.Z.Item, {
        as: react_router_dom/* Link */.rU,
        active: path === "/themes",
        to: "/themes"
      }, "Themes")), /*#__PURE__*/react.createElement(NavDropdown/* default */.Z, {
        title: "Advanced"
      }, /*#__PURE__*/react.createElement(NavDropdown/* default.Item */.Z.Item, {
        as: react_router_dom/* Link */.rU,
        active: path === "/settings",
        to: "/settings"
      }, "Settings"), /*#__PURE__*/react.createElement(NavDropdown/* default.Item */.Z.Item, {
        as: react_router_dom/* Link */.rU,
        active: path === "/cache",
        to: "/cache"
      }, "Cache")), /*#__PURE__*/react.createElement(NavDropdown/* default */.Z, {
        title: "Misc"
      }, /*#__PURE__*/react.createElement(NavDropdown/* default.Item */.Z.Item, {
        as: react_router_dom/* Link */.rU,
        active: path === "/import-export",
        to: "/import-export"
      }, "Import / Export")), /*#__PURE__*/react.createElement(NavDropdown/* default */.Z, {
        title: "Experimental"
      }, /*#__PURE__*/react.createElement(NavDropdown/* default.Item */.Z.Item, {
        as: react_router_dom/* Link */.rU,
        active: path === "/overlay",
        to: "/overlay",
        target: "overlay"
      }, "Overlay"), /*#__PURE__*/react.createElement(NavDropdown/* default.Item */.Z.Item, {
        as: react_router_dom/* Link */.rU,
        active: path === "/youtube",
        to: "/youtube",
        target: "youtube"
      }, "YouTube Player"), /*#__PURE__*/react.createElement(NavDropdown/* default.Item */.Z.Item, {
        as: react_router_dom/* Link */.rU,
        active: path === "/chat",
        to: "/chat",
        target: "chat"
      }, "Chat"))), /*#__PURE__*/react.createElement(Nav/* default */.Z, {
        className: "ml-auto"
      }, /*#__PURE__*/react.createElement(Form/* default */.Z, {
        inline: true,
        key: "second"
      }, /*#__PURE__*/react.createElement("a", {
        href: "https://setbac.tv",
        title: "Go to https://setbac.tv"
      }, /*#__PURE__*/react.createElement(Button/* default */.Z, {
        variant: "primary",
        size: "sm"
      }, "Go to setbac.tv"))))), /*#__PURE__*/react.createElement(Navbar/* default.Toggle */.Z.Toggle, {
        "aria-controls": "basic-navbar-nav"
      }))), /*#__PURE__*/react.createElement(Container/* default */.Z, {
        className: "content"
      }, this.props.children));
    }
  }]);

  return Layout;
}(react.Component);

function AppRouter() {
  return /*#__PURE__*/react.createElement(react_router_dom/* BrowserRouter */.VK, null, /*#__PURE__*/react.createElement(react_router/* Route */.AW, {
    path: "/",
    exact: true,
    component: IndexPage
  }), /*#__PURE__*/react.createElement(react_router/* Route */.AW, {
    path: "/after-streams",
    exact: true,
    component: AfterStreamsPage
  }), /*#__PURE__*/react.createElement(react_router/* Route */.AW, {
    path: "/settings",
    exact: true,
    component: SettingsPage
  }), /*#__PURE__*/react.createElement(react_router/* Route */.AW, {
    path: "/cache",
    exact: true,
    component: CachePage
  }), /*#__PURE__*/react.createElement(react_router/* Route */.AW, {
    path: "/modules",
    component: ModulesPage
  }), /*#__PURE__*/react.createElement(react_router/* Route */.AW, {
    path: "/authorization",
    exact: true,
    component: function component(props) {
      return /*#__PURE__*/react.createElement(AuthorizedPage, null, /*#__PURE__*/react.createElement(Authorization, props));
    }
  }), /*#__PURE__*/react.createElement(react_router/* Route */.AW, {
    path: "/import-export",
    component: ImportExportPage
  }), /*#__PURE__*/react.createElement(react_router/* Route */.AW, {
    path: "/aliases",
    exact: true,
    render: function render(props) {
      return /*#__PURE__*/react.createElement(AuthorizedPage, null, /*#__PURE__*/react.createElement(Aliases, props));
    }
  }), /*#__PURE__*/react.createElement(react_router/* Route */.AW, {
    path: "/commands",
    exact: true,
    render: function render(props) {
      return /*#__PURE__*/react.createElement(AuthorizedPage, null, /*#__PURE__*/react.createElement(Commands, props));
    }
  }), /*#__PURE__*/react.createElement(react_router/* Route */.AW, {
    path: "/promotions",
    exact: true,
    render: function render(props) {
      return /*#__PURE__*/react.createElement(AuthorizedPage, null, /*#__PURE__*/react.createElement(Promotions, props));
    }
  }), /*#__PURE__*/react.createElement(react_router/* Route */.AW, {
    path: "/themes",
    exact: true,
    render: function render(props) {
      return /*#__PURE__*/react.createElement(AuthorizedPage, null, /*#__PURE__*/react.createElement(Themes, props));
    }
  }), /*#__PURE__*/react.createElement(react_router/* Route */.AW, {
    path: "/overlay/",
    component: Overlay
  }), /*#__PURE__*/react.createElement(react_router/* Route */.AW, {
    path: "/youtube",
    component: YouTube
  }), /*#__PURE__*/react.createElement(react_router/* Route */.AW, {
    path: "/chat",
    component: Chat
  }));
}

react_dom.render( /*#__PURE__*/react.createElement(AppRouter, null), document.getElementById("index"));

/***/ }),

/***/ 62233:
/***/ ((module, __webpack_exports__, __webpack_require__) => {

"use strict";
/* harmony export */ __webpack_require__.d(__webpack_exports__, {
/* harmony export */   "Z": () => __WEBPACK_DEFAULT_EXPORT__
/* harmony export */ });
/* harmony import */ var _node_modules_css_loader_dist_runtime_api_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(23645);
/* harmony import */ var _node_modules_css_loader_dist_runtime_api_js__WEBPACK_IMPORTED_MODULE_0___default = /*#__PURE__*/__webpack_require__.n(_node_modules_css_loader_dist_runtime_api_js__WEBPACK_IMPORTED_MODULE_0__);
/* harmony import */ var _node_modules_css_loader_dist_cjs_js_node_modules_react_bootstrap_typeahead_css_Typeahead_css__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(98606);
// Imports


var ___CSS_LOADER_EXPORT___ = _node_modules_css_loader_dist_runtime_api_js__WEBPACK_IMPORTED_MODULE_0___default()(function(i){return i[1]});
___CSS_LOADER_EXPORT___.i(_node_modules_css_loader_dist_cjs_js_node_modules_react_bootstrap_typeahead_css_Typeahead_css__WEBPACK_IMPORTED_MODULE_1__/* .default */ .Z);
// Module
___CSS_LOADER_EXPORT___.push([module.id, "/*!\n * Bootstrap v4.6.0 (https://getbootstrap.com/)\n * Copyright 2011-2021 The Bootstrap Authors\n * Copyright 2011-2021 Twitter, Inc.\n * Licensed under MIT (https://github.com/twbs/bootstrap/blob/main/LICENSE)\n */:root{--blue: #007bff;--indigo: #6610f2;--purple: #6f42c1;--pink: #e83e8c;--red: #dc3545;--orange: #fd7e14;--yellow: #ffc107;--green: #28a745;--teal: #20c997;--cyan: #17a2b8;--white: #fff;--gray: #6c757d;--gray-dark: #343a40;--primary: #007bff;--secondary: #6c757d;--success: #28a745;--info: #17a2b8;--warning: #ffc107;--danger: #dc3545;--light: #f8f9fa;--dark: #343a40;--breakpoint-xs: 0;--breakpoint-sm: 576px;--breakpoint-md: 768px;--breakpoint-lg: 992px;--breakpoint-xl: 1200px;--font-family-sans-serif: -apple-system, BlinkMacSystemFont, \"Segoe UI\", Roboto, \"Helvetica Neue\", Arial, sans-serif, \"Apple Color Emoji\", \"Segoe UI Emoji\", \"Segoe UI Symbol\", \"Noto Color Emoji\";--font-family-monospace: SFMono-Regular, Menlo, Monaco, Consolas, \"Liberation Mono\", \"Courier New\", monospace}*,*::before,*::after{box-sizing:border-box}html{font-family:sans-serif;line-height:1.15;-webkit-text-size-adjust:100%;-webkit-tap-highlight-color:rgba(0,0,0,0)}article,aside,figcaption,figure,footer,header,hgroup,main,nav,section{display:block}body{margin:0;font-family:-apple-system,BlinkMacSystemFont,\"Segoe UI\",Roboto,\"Helvetica Neue\",Arial,sans-serif,\"Apple Color Emoji\",\"Segoe UI Emoji\",\"Segoe UI Symbol\",\"Noto Color Emoji\";font-size:1rem;font-weight:400;line-height:1.5;color:#212529;text-align:left;background-color:#fff}[tabindex=\"-1\"]:focus:not(:focus-visible){outline:0 !important}hr{box-sizing:content-box;height:0;overflow:visible}h1,h2,h3,h4,h5,h6{margin-top:0;margin-bottom:.5rem}p{margin-top:0;margin-bottom:1rem}abbr[title],abbr[data-original-title]{text-decoration:underline;text-decoration:underline dotted;cursor:help;border-bottom:0;text-decoration-skip-ink:none}address{margin-bottom:1rem;font-style:normal;line-height:inherit}ol,ul,dl{margin-top:0;margin-bottom:1rem}ol ol,ul ul,ol ul,ul ol{margin-bottom:0}dt{font-weight:700}dd{margin-bottom:.5rem;margin-left:0}blockquote{margin:0 0 1rem}b,strong{font-weight:bolder}small{font-size:80%}sub,sup{position:relative;font-size:75%;line-height:0;vertical-align:baseline}sub{bottom:-0.25em}sup{top:-0.5em}a{color:#007bff;text-decoration:none;background-color:transparent}a:hover{color:#0056b3;text-decoration:underline}a:not([href]):not([class]){color:inherit;text-decoration:none}a:not([href]):not([class]):hover{color:inherit;text-decoration:none}pre,code,kbd,samp{font-family:SFMono-Regular,Menlo,Monaco,Consolas,\"Liberation Mono\",\"Courier New\",monospace;font-size:1em}pre{margin-top:0;margin-bottom:1rem;overflow:auto;-ms-overflow-style:scrollbar}figure{margin:0 0 1rem}img{vertical-align:middle;border-style:none}svg{overflow:hidden;vertical-align:middle}table{border-collapse:collapse}caption{padding-top:.75rem;padding-bottom:.75rem;color:#6c757d;text-align:left;caption-side:bottom}th{text-align:inherit;text-align:-webkit-match-parent}label{display:inline-block;margin-bottom:.5rem}button{border-radius:0}button:focus:not(:focus-visible){outline:0}input,button,select,optgroup,textarea{margin:0;font-family:inherit;font-size:inherit;line-height:inherit}button,input{overflow:visible}button,select{text-transform:none}[role=button]{cursor:pointer}select{word-wrap:normal}button,[type=button],[type=reset],[type=submit]{-webkit-appearance:button}button:not(:disabled),[type=button]:not(:disabled),[type=reset]:not(:disabled),[type=submit]:not(:disabled){cursor:pointer}button::-moz-focus-inner,[type=button]::-moz-focus-inner,[type=reset]::-moz-focus-inner,[type=submit]::-moz-focus-inner{padding:0;border-style:none}input[type=radio],input[type=checkbox]{box-sizing:border-box;padding:0}textarea{overflow:auto;resize:vertical}fieldset{min-width:0;padding:0;margin:0;border:0}legend{display:block;width:100%;max-width:100%;padding:0;margin-bottom:.5rem;font-size:1.5rem;line-height:inherit;color:inherit;white-space:normal}progress{vertical-align:baseline}[type=number]::-webkit-inner-spin-button,[type=number]::-webkit-outer-spin-button{height:auto}[type=search]{outline-offset:-2px;-webkit-appearance:none}[type=search]::-webkit-search-decoration{-webkit-appearance:none}::-webkit-file-upload-button{font:inherit;-webkit-appearance:button}output{display:inline-block}summary{display:list-item;cursor:pointer}template{display:none}[hidden]{display:none !important}h1,h2,h3,h4,h5,h6,.h1,.h2,.h3,.h4,.h5,.h6{margin-bottom:.5rem;font-weight:500;line-height:1.2}h1,.h1{font-size:2.5rem}h2,.h2{font-size:2rem}h3,.h3{font-size:1.75rem}h4,.h4{font-size:1.5rem}h5,.h5{font-size:1.25rem}h6,.h6{font-size:1rem}.lead{font-size:1.25rem;font-weight:300}.display-1{font-size:6rem;font-weight:300;line-height:1.2}.display-2{font-size:5.5rem;font-weight:300;line-height:1.2}.display-3{font-size:4.5rem;font-weight:300;line-height:1.2}.display-4{font-size:3.5rem;font-weight:300;line-height:1.2}hr{margin-top:1rem;margin-bottom:1rem;border:0;border-top:1px solid rgba(0,0,0,.1)}small,.small{font-size:80%;font-weight:400}mark,.mark{padding:.2em;background-color:#fcf8e3}.list-unstyled{padding-left:0;list-style:none}.list-inline{padding-left:0;list-style:none}.list-inline-item{display:inline-block}.list-inline-item:not(:last-child){margin-right:.5rem}.initialism{font-size:90%;text-transform:uppercase}.blockquote{margin-bottom:1rem;font-size:1.25rem}.blockquote-footer{display:block;font-size:80%;color:#6c757d}.blockquote-footer::before{content:\"— \"}.img-fluid{max-width:100%;height:auto}.img-thumbnail{padding:.25rem;background-color:#fff;border:1px solid #dee2e6;border-radius:.25rem;max-width:100%;height:auto}.figure{display:inline-block}.figure-img{margin-bottom:.5rem;line-height:1}.figure-caption{font-size:90%;color:#6c757d}code{font-size:87.5%;color:#e83e8c;word-wrap:break-word}a>code{color:inherit}kbd{padding:.2rem .4rem;font-size:87.5%;color:#fff;background-color:#212529;border-radius:.2rem}kbd kbd{padding:0;font-size:100%;font-weight:700}pre{display:block;font-size:87.5%;color:#212529}pre code{font-size:inherit;color:inherit;word-break:normal}.pre-scrollable{max-height:340px;overflow-y:scroll}.container,.container-fluid,.container-xl,.container-lg,.container-md,.container-sm{width:100%;padding-right:15px;padding-left:15px;margin-right:auto;margin-left:auto}@media(min-width: 576px){.container-sm,.container{max-width:540px}}@media(min-width: 768px){.container-md,.container-sm,.container{max-width:720px}}@media(min-width: 992px){.container-lg,.container-md,.container-sm,.container{max-width:960px}}@media(min-width: 1200px){.container-xl,.container-lg,.container-md,.container-sm,.container{max-width:1140px}}.row{display:flex;flex-wrap:wrap;margin-right:-15px;margin-left:-15px}.no-gutters{margin-right:0;margin-left:0}.no-gutters>.col,.no-gutters>[class*=col-]{padding-right:0;padding-left:0}.col-xl,.col-xl-auto,.col-xl-12,.col-xl-11,.col-xl-10,.col-xl-9,.col-xl-8,.col-xl-7,.col-xl-6,.col-xl-5,.col-xl-4,.col-xl-3,.col-xl-2,.col-xl-1,.col-lg,.col-lg-auto,.col-lg-12,.col-lg-11,.col-lg-10,.col-lg-9,.col-lg-8,.col-lg-7,.col-lg-6,.col-lg-5,.col-lg-4,.col-lg-3,.col-lg-2,.col-lg-1,.col-md,.col-md-auto,.col-md-12,.col-md-11,.col-md-10,.col-md-9,.col-md-8,.col-md-7,.col-md-6,.col-md-5,.col-md-4,.col-md-3,.col-md-2,.col-md-1,.col-sm,.col-sm-auto,.col-sm-12,.col-sm-11,.col-sm-10,.col-sm-9,.col-sm-8,.col-sm-7,.col-sm-6,.col-sm-5,.col-sm-4,.col-sm-3,.col-sm-2,.col-sm-1,.col,.col-auto,.col-12,.col-11,.col-10,.col-9,.col-8,.col-7,.col-6,.col-5,.col-4,.col-3,.col-2,.col-1{position:relative;width:100%;padding-right:15px;padding-left:15px}.col{flex-basis:0;flex-grow:1;max-width:100%}.row-cols-1>*{flex:0 0 100%;max-width:100%}.row-cols-2>*{flex:0 0 50%;max-width:50%}.row-cols-3>*{flex:0 0 33.3333333333%;max-width:33.3333333333%}.row-cols-4>*{flex:0 0 25%;max-width:25%}.row-cols-5>*{flex:0 0 20%;max-width:20%}.row-cols-6>*{flex:0 0 16.6666666667%;max-width:16.6666666667%}.col-auto{flex:0 0 auto;width:auto;max-width:100%}.col-1{flex:0 0 8.3333333333%;max-width:8.3333333333%}.col-2{flex:0 0 16.6666666667%;max-width:16.6666666667%}.col-3{flex:0 0 25%;max-width:25%}.col-4{flex:0 0 33.3333333333%;max-width:33.3333333333%}.col-5{flex:0 0 41.6666666667%;max-width:41.6666666667%}.col-6{flex:0 0 50%;max-width:50%}.col-7{flex:0 0 58.3333333333%;max-width:58.3333333333%}.col-8{flex:0 0 66.6666666667%;max-width:66.6666666667%}.col-9{flex:0 0 75%;max-width:75%}.col-10{flex:0 0 83.3333333333%;max-width:83.3333333333%}.col-11{flex:0 0 91.6666666667%;max-width:91.6666666667%}.col-12{flex:0 0 100%;max-width:100%}.order-first{order:-1}.order-last{order:13}.order-0{order:0}.order-1{order:1}.order-2{order:2}.order-3{order:3}.order-4{order:4}.order-5{order:5}.order-6{order:6}.order-7{order:7}.order-8{order:8}.order-9{order:9}.order-10{order:10}.order-11{order:11}.order-12{order:12}.offset-1{margin-left:8.3333333333%}.offset-2{margin-left:16.6666666667%}.offset-3{margin-left:25%}.offset-4{margin-left:33.3333333333%}.offset-5{margin-left:41.6666666667%}.offset-6{margin-left:50%}.offset-7{margin-left:58.3333333333%}.offset-8{margin-left:66.6666666667%}.offset-9{margin-left:75%}.offset-10{margin-left:83.3333333333%}.offset-11{margin-left:91.6666666667%}@media(min-width: 576px){.col-sm{flex-basis:0;flex-grow:1;max-width:100%}.row-cols-sm-1>*{flex:0 0 100%;max-width:100%}.row-cols-sm-2>*{flex:0 0 50%;max-width:50%}.row-cols-sm-3>*{flex:0 0 33.3333333333%;max-width:33.3333333333%}.row-cols-sm-4>*{flex:0 0 25%;max-width:25%}.row-cols-sm-5>*{flex:0 0 20%;max-width:20%}.row-cols-sm-6>*{flex:0 0 16.6666666667%;max-width:16.6666666667%}.col-sm-auto{flex:0 0 auto;width:auto;max-width:100%}.col-sm-1{flex:0 0 8.3333333333%;max-width:8.3333333333%}.col-sm-2{flex:0 0 16.6666666667%;max-width:16.6666666667%}.col-sm-3{flex:0 0 25%;max-width:25%}.col-sm-4{flex:0 0 33.3333333333%;max-width:33.3333333333%}.col-sm-5{flex:0 0 41.6666666667%;max-width:41.6666666667%}.col-sm-6{flex:0 0 50%;max-width:50%}.col-sm-7{flex:0 0 58.3333333333%;max-width:58.3333333333%}.col-sm-8{flex:0 0 66.6666666667%;max-width:66.6666666667%}.col-sm-9{flex:0 0 75%;max-width:75%}.col-sm-10{flex:0 0 83.3333333333%;max-width:83.3333333333%}.col-sm-11{flex:0 0 91.6666666667%;max-width:91.6666666667%}.col-sm-12{flex:0 0 100%;max-width:100%}.order-sm-first{order:-1}.order-sm-last{order:13}.order-sm-0{order:0}.order-sm-1{order:1}.order-sm-2{order:2}.order-sm-3{order:3}.order-sm-4{order:4}.order-sm-5{order:5}.order-sm-6{order:6}.order-sm-7{order:7}.order-sm-8{order:8}.order-sm-9{order:9}.order-sm-10{order:10}.order-sm-11{order:11}.order-sm-12{order:12}.offset-sm-0{margin-left:0}.offset-sm-1{margin-left:8.3333333333%}.offset-sm-2{margin-left:16.6666666667%}.offset-sm-3{margin-left:25%}.offset-sm-4{margin-left:33.3333333333%}.offset-sm-5{margin-left:41.6666666667%}.offset-sm-6{margin-left:50%}.offset-sm-7{margin-left:58.3333333333%}.offset-sm-8{margin-left:66.6666666667%}.offset-sm-9{margin-left:75%}.offset-sm-10{margin-left:83.3333333333%}.offset-sm-11{margin-left:91.6666666667%}}@media(min-width: 768px){.col-md{flex-basis:0;flex-grow:1;max-width:100%}.row-cols-md-1>*{flex:0 0 100%;max-width:100%}.row-cols-md-2>*{flex:0 0 50%;max-width:50%}.row-cols-md-3>*{flex:0 0 33.3333333333%;max-width:33.3333333333%}.row-cols-md-4>*{flex:0 0 25%;max-width:25%}.row-cols-md-5>*{flex:0 0 20%;max-width:20%}.row-cols-md-6>*{flex:0 0 16.6666666667%;max-width:16.6666666667%}.col-md-auto{flex:0 0 auto;width:auto;max-width:100%}.col-md-1{flex:0 0 8.3333333333%;max-width:8.3333333333%}.col-md-2{flex:0 0 16.6666666667%;max-width:16.6666666667%}.col-md-3{flex:0 0 25%;max-width:25%}.col-md-4{flex:0 0 33.3333333333%;max-width:33.3333333333%}.col-md-5{flex:0 0 41.6666666667%;max-width:41.6666666667%}.col-md-6{flex:0 0 50%;max-width:50%}.col-md-7{flex:0 0 58.3333333333%;max-width:58.3333333333%}.col-md-8{flex:0 0 66.6666666667%;max-width:66.6666666667%}.col-md-9{flex:0 0 75%;max-width:75%}.col-md-10{flex:0 0 83.3333333333%;max-width:83.3333333333%}.col-md-11{flex:0 0 91.6666666667%;max-width:91.6666666667%}.col-md-12{flex:0 0 100%;max-width:100%}.order-md-first{order:-1}.order-md-last{order:13}.order-md-0{order:0}.order-md-1{order:1}.order-md-2{order:2}.order-md-3{order:3}.order-md-4{order:4}.order-md-5{order:5}.order-md-6{order:6}.order-md-7{order:7}.order-md-8{order:8}.order-md-9{order:9}.order-md-10{order:10}.order-md-11{order:11}.order-md-12{order:12}.offset-md-0{margin-left:0}.offset-md-1{margin-left:8.3333333333%}.offset-md-2{margin-left:16.6666666667%}.offset-md-3{margin-left:25%}.offset-md-4{margin-left:33.3333333333%}.offset-md-5{margin-left:41.6666666667%}.offset-md-6{margin-left:50%}.offset-md-7{margin-left:58.3333333333%}.offset-md-8{margin-left:66.6666666667%}.offset-md-9{margin-left:75%}.offset-md-10{margin-left:83.3333333333%}.offset-md-11{margin-left:91.6666666667%}}@media(min-width: 992px){.col-lg{flex-basis:0;flex-grow:1;max-width:100%}.row-cols-lg-1>*{flex:0 0 100%;max-width:100%}.row-cols-lg-2>*{flex:0 0 50%;max-width:50%}.row-cols-lg-3>*{flex:0 0 33.3333333333%;max-width:33.3333333333%}.row-cols-lg-4>*{flex:0 0 25%;max-width:25%}.row-cols-lg-5>*{flex:0 0 20%;max-width:20%}.row-cols-lg-6>*{flex:0 0 16.6666666667%;max-width:16.6666666667%}.col-lg-auto{flex:0 0 auto;width:auto;max-width:100%}.col-lg-1{flex:0 0 8.3333333333%;max-width:8.3333333333%}.col-lg-2{flex:0 0 16.6666666667%;max-width:16.6666666667%}.col-lg-3{flex:0 0 25%;max-width:25%}.col-lg-4{flex:0 0 33.3333333333%;max-width:33.3333333333%}.col-lg-5{flex:0 0 41.6666666667%;max-width:41.6666666667%}.col-lg-6{flex:0 0 50%;max-width:50%}.col-lg-7{flex:0 0 58.3333333333%;max-width:58.3333333333%}.col-lg-8{flex:0 0 66.6666666667%;max-width:66.6666666667%}.col-lg-9{flex:0 0 75%;max-width:75%}.col-lg-10{flex:0 0 83.3333333333%;max-width:83.3333333333%}.col-lg-11{flex:0 0 91.6666666667%;max-width:91.6666666667%}.col-lg-12{flex:0 0 100%;max-width:100%}.order-lg-first{order:-1}.order-lg-last{order:13}.order-lg-0{order:0}.order-lg-1{order:1}.order-lg-2{order:2}.order-lg-3{order:3}.order-lg-4{order:4}.order-lg-5{order:5}.order-lg-6{order:6}.order-lg-7{order:7}.order-lg-8{order:8}.order-lg-9{order:9}.order-lg-10{order:10}.order-lg-11{order:11}.order-lg-12{order:12}.offset-lg-0{margin-left:0}.offset-lg-1{margin-left:8.3333333333%}.offset-lg-2{margin-left:16.6666666667%}.offset-lg-3{margin-left:25%}.offset-lg-4{margin-left:33.3333333333%}.offset-lg-5{margin-left:41.6666666667%}.offset-lg-6{margin-left:50%}.offset-lg-7{margin-left:58.3333333333%}.offset-lg-8{margin-left:66.6666666667%}.offset-lg-9{margin-left:75%}.offset-lg-10{margin-left:83.3333333333%}.offset-lg-11{margin-left:91.6666666667%}}@media(min-width: 1200px){.col-xl{flex-basis:0;flex-grow:1;max-width:100%}.row-cols-xl-1>*{flex:0 0 100%;max-width:100%}.row-cols-xl-2>*{flex:0 0 50%;max-width:50%}.row-cols-xl-3>*{flex:0 0 33.3333333333%;max-width:33.3333333333%}.row-cols-xl-4>*{flex:0 0 25%;max-width:25%}.row-cols-xl-5>*{flex:0 0 20%;max-width:20%}.row-cols-xl-6>*{flex:0 0 16.6666666667%;max-width:16.6666666667%}.col-xl-auto{flex:0 0 auto;width:auto;max-width:100%}.col-xl-1{flex:0 0 8.3333333333%;max-width:8.3333333333%}.col-xl-2{flex:0 0 16.6666666667%;max-width:16.6666666667%}.col-xl-3{flex:0 0 25%;max-width:25%}.col-xl-4{flex:0 0 33.3333333333%;max-width:33.3333333333%}.col-xl-5{flex:0 0 41.6666666667%;max-width:41.6666666667%}.col-xl-6{flex:0 0 50%;max-width:50%}.col-xl-7{flex:0 0 58.3333333333%;max-width:58.3333333333%}.col-xl-8{flex:0 0 66.6666666667%;max-width:66.6666666667%}.col-xl-9{flex:0 0 75%;max-width:75%}.col-xl-10{flex:0 0 83.3333333333%;max-width:83.3333333333%}.col-xl-11{flex:0 0 91.6666666667%;max-width:91.6666666667%}.col-xl-12{flex:0 0 100%;max-width:100%}.order-xl-first{order:-1}.order-xl-last{order:13}.order-xl-0{order:0}.order-xl-1{order:1}.order-xl-2{order:2}.order-xl-3{order:3}.order-xl-4{order:4}.order-xl-5{order:5}.order-xl-6{order:6}.order-xl-7{order:7}.order-xl-8{order:8}.order-xl-9{order:9}.order-xl-10{order:10}.order-xl-11{order:11}.order-xl-12{order:12}.offset-xl-0{margin-left:0}.offset-xl-1{margin-left:8.3333333333%}.offset-xl-2{margin-left:16.6666666667%}.offset-xl-3{margin-left:25%}.offset-xl-4{margin-left:33.3333333333%}.offset-xl-5{margin-left:41.6666666667%}.offset-xl-6{margin-left:50%}.offset-xl-7{margin-left:58.3333333333%}.offset-xl-8{margin-left:66.6666666667%}.offset-xl-9{margin-left:75%}.offset-xl-10{margin-left:83.3333333333%}.offset-xl-11{margin-left:91.6666666667%}}.table{width:100%;margin-bottom:1rem;color:#212529}.table th,.table td{padding:.75rem;vertical-align:top;border-top:1px solid #dee2e6}.table thead th{vertical-align:bottom;border-bottom:2px solid #dee2e6}.table tbody+tbody{border-top:2px solid #dee2e6}.table-sm th,.table-sm td{padding:.3rem}.table-bordered{border:1px solid #dee2e6}.table-bordered th,.table-bordered td{border:1px solid #dee2e6}.table-bordered thead th,.table-bordered thead td{border-bottom-width:2px}.table-borderless th,.table-borderless td,.table-borderless thead th,.table-borderless tbody+tbody{border:0}.table-striped tbody tr:nth-of-type(odd){background-color:rgba(0,0,0,.05)}.table-hover tbody tr:hover{color:#212529;background-color:rgba(0,0,0,.075)}.table-primary,.table-primary>th,.table-primary>td{background-color:#b8daff}.table-primary th,.table-primary td,.table-primary thead th,.table-primary tbody+tbody{border-color:#7abaff}.table-hover .table-primary:hover{background-color:#9fcdff}.table-hover .table-primary:hover>td,.table-hover .table-primary:hover>th{background-color:#9fcdff}.table-secondary,.table-secondary>th,.table-secondary>td{background-color:#d6d8db}.table-secondary th,.table-secondary td,.table-secondary thead th,.table-secondary tbody+tbody{border-color:#b3b7bb}.table-hover .table-secondary:hover{background-color:#c8cbcf}.table-hover .table-secondary:hover>td,.table-hover .table-secondary:hover>th{background-color:#c8cbcf}.table-success,.table-success>th,.table-success>td{background-color:#c3e6cb}.table-success th,.table-success td,.table-success thead th,.table-success tbody+tbody{border-color:#8fd19e}.table-hover .table-success:hover{background-color:#b1dfbb}.table-hover .table-success:hover>td,.table-hover .table-success:hover>th{background-color:#b1dfbb}.table-info,.table-info>th,.table-info>td{background-color:#bee5eb}.table-info th,.table-info td,.table-info thead th,.table-info tbody+tbody{border-color:#86cfda}.table-hover .table-info:hover{background-color:#abdde5}.table-hover .table-info:hover>td,.table-hover .table-info:hover>th{background-color:#abdde5}.table-warning,.table-warning>th,.table-warning>td{background-color:#ffeeba}.table-warning th,.table-warning td,.table-warning thead th,.table-warning tbody+tbody{border-color:#ffdf7e}.table-hover .table-warning:hover{background-color:#ffe8a1}.table-hover .table-warning:hover>td,.table-hover .table-warning:hover>th{background-color:#ffe8a1}.table-danger,.table-danger>th,.table-danger>td{background-color:#f5c6cb}.table-danger th,.table-danger td,.table-danger thead th,.table-danger tbody+tbody{border-color:#ed969e}.table-hover .table-danger:hover{background-color:#f1b0b7}.table-hover .table-danger:hover>td,.table-hover .table-danger:hover>th{background-color:#f1b0b7}.table-light,.table-light>th,.table-light>td{background-color:#fdfdfe}.table-light th,.table-light td,.table-light thead th,.table-light tbody+tbody{border-color:#fbfcfc}.table-hover .table-light:hover{background-color:#ececf6}.table-hover .table-light:hover>td,.table-hover .table-light:hover>th{background-color:#ececf6}.table-dark,.table-dark>th,.table-dark>td{background-color:#c6c8ca}.table-dark th,.table-dark td,.table-dark thead th,.table-dark tbody+tbody{border-color:#95999c}.table-hover .table-dark:hover{background-color:#b9bbbe}.table-hover .table-dark:hover>td,.table-hover .table-dark:hover>th{background-color:#b9bbbe}.table-active,.table-active>th,.table-active>td{background-color:rgba(0,0,0,.075)}.table-hover .table-active:hover{background-color:rgba(0,0,0,.075)}.table-hover .table-active:hover>td,.table-hover .table-active:hover>th{background-color:rgba(0,0,0,.075)}.table .thead-dark th{color:#fff;background-color:#343a40;border-color:#454d55}.table .thead-light th{color:#495057;background-color:#e9ecef;border-color:#dee2e6}.table-dark{color:#fff;background-color:#343a40}.table-dark th,.table-dark td,.table-dark thead th{border-color:#454d55}.table-dark.table-bordered{border:0}.table-dark.table-striped tbody tr:nth-of-type(odd){background-color:rgba(255,255,255,.05)}.table-dark.table-hover tbody tr:hover{color:#fff;background-color:rgba(255,255,255,.075)}@media(max-width: 575.98px){.table-responsive-sm{display:block;width:100%;overflow-x:auto;-webkit-overflow-scrolling:touch}.table-responsive-sm>.table-bordered{border:0}}@media(max-width: 767.98px){.table-responsive-md{display:block;width:100%;overflow-x:auto;-webkit-overflow-scrolling:touch}.table-responsive-md>.table-bordered{border:0}}@media(max-width: 991.98px){.table-responsive-lg{display:block;width:100%;overflow-x:auto;-webkit-overflow-scrolling:touch}.table-responsive-lg>.table-bordered{border:0}}@media(max-width: 1199.98px){.table-responsive-xl{display:block;width:100%;overflow-x:auto;-webkit-overflow-scrolling:touch}.table-responsive-xl>.table-bordered{border:0}}.table-responsive{display:block;width:100%;overflow-x:auto;-webkit-overflow-scrolling:touch}.table-responsive>.table-bordered{border:0}.form-control{display:block;width:100%;height:calc(1.5em + 0.75rem + 2px);padding:.375rem .75rem;font-size:1rem;font-weight:400;line-height:1.5;color:#495057;background-color:#fff;background-clip:padding-box;border:1px solid #ced4da;border-radius:.25rem;transition:border-color .15s ease-in-out,box-shadow .15s ease-in-out}@media(prefers-reduced-motion: reduce){.form-control{transition:none}}.form-control::-ms-expand{background-color:transparent;border:0}.form-control:-moz-focusring{color:transparent;text-shadow:0 0 0 #495057}.form-control:focus{color:#495057;background-color:#fff;border-color:#80bdff;outline:0;box-shadow:0 0 0 .2rem rgba(0,123,255,.25)}.form-control::placeholder{color:#6c757d;opacity:1}.form-control:disabled,.form-control[readonly]{background-color:#e9ecef;opacity:1}input[type=date].form-control,input[type=time].form-control,input[type=datetime-local].form-control,input[type=month].form-control{appearance:none}select.form-control:focus::-ms-value{color:#495057;background-color:#fff}.form-control-file,.form-control-range{display:block;width:100%}.col-form-label{padding-top:calc(0.375rem + 1px);padding-bottom:calc(0.375rem + 1px);margin-bottom:0;font-size:inherit;line-height:1.5}.col-form-label-lg{padding-top:calc(0.5rem + 1px);padding-bottom:calc(0.5rem + 1px);font-size:1.25rem;line-height:1.5}.col-form-label-sm{padding-top:calc(0.25rem + 1px);padding-bottom:calc(0.25rem + 1px);font-size:0.875rem;line-height:1.5}.form-control-plaintext{display:block;width:100%;padding:.375rem 0;margin-bottom:0;font-size:1rem;line-height:1.5;color:#212529;background-color:transparent;border:solid transparent;border-width:1px 0}.form-control-plaintext.form-control-sm,.form-control-plaintext.form-control-lg{padding-right:0;padding-left:0}.form-control-sm{height:calc(1.5em + 0.5rem + 2px);padding:.25rem .5rem;font-size:0.875rem;line-height:1.5;border-radius:.2rem}.form-control-lg{height:calc(1.5em + 1rem + 2px);padding:.5rem 1rem;font-size:1.25rem;line-height:1.5;border-radius:.3rem}select.form-control[size],select.form-control[multiple]{height:auto}textarea.form-control{height:auto}.form-group{margin-bottom:1rem}.form-text{display:block;margin-top:.25rem}.form-row{display:flex;flex-wrap:wrap;margin-right:-5px;margin-left:-5px}.form-row>.col,.form-row>[class*=col-]{padding-right:5px;padding-left:5px}.form-check{position:relative;display:block;padding-left:1.25rem}.form-check-input{position:absolute;margin-top:.3rem;margin-left:-1.25rem}.form-check-input[disabled]~.form-check-label,.form-check-input:disabled~.form-check-label{color:#6c757d}.form-check-label{margin-bottom:0}.form-check-inline{display:inline-flex;align-items:center;padding-left:0;margin-right:.75rem}.form-check-inline .form-check-input{position:static;margin-top:0;margin-right:.3125rem;margin-left:0}.valid-feedback{display:none;width:100%;margin-top:.25rem;font-size:80%;color:#28a745}.valid-tooltip{position:absolute;top:100%;left:0;z-index:5;display:none;max-width:100%;padding:.25rem .5rem;margin-top:.1rem;font-size:0.875rem;line-height:1.5;color:#fff;background-color:rgba(40,167,69,.9);border-radius:.25rem}.form-row>.col>.valid-tooltip,.form-row>[class*=col-]>.valid-tooltip{left:5px}.was-validated :valid~.valid-feedback,.was-validated :valid~.valid-tooltip,.is-valid~.valid-feedback,.is-valid~.valid-tooltip{display:block}.was-validated .form-control:valid,.form-control.is-valid{border-color:#28a745;padding-right:calc(1.5em + 0.75rem);background-image:url(\"data:image/svg+xml,%3csvg xmlns='http://www.w3.org/2000/svg' width='8' height='8' viewBox='0 0 8 8'%3e%3cpath fill='%2328a745' d='M2.3 6.73L.6 4.53c-.4-1.04.46-1.4 1.1-.8l1.1 1.4 3.4-3.8c.6-.63 1.6-.27 1.2.7l-4 4.6c-.43.5-.8.4-1.1.1z'/%3e%3c/svg%3e\");background-repeat:no-repeat;background-position:right calc(0.375em + 0.1875rem) center;background-size:calc(0.75em + 0.375rem) calc(0.75em + 0.375rem)}.was-validated .form-control:valid:focus,.form-control.is-valid:focus{border-color:#28a745;box-shadow:0 0 0 .2rem rgba(40,167,69,.25)}.was-validated textarea.form-control:valid,textarea.form-control.is-valid{padding-right:calc(1.5em + 0.75rem);background-position:top calc(0.375em + 0.1875rem) right calc(0.375em + 0.1875rem)}.was-validated .custom-select:valid,.custom-select.is-valid{border-color:#28a745;padding-right:calc(0.75em + 2.3125rem);background:url(\"data:image/svg+xml,%3csvg xmlns='http://www.w3.org/2000/svg' width='4' height='5' viewBox='0 0 4 5'%3e%3cpath fill='%23343a40' d='M2 0L0 2h4zm0 5L0 3h4z'/%3e%3c/svg%3e\") right .75rem center/8px 10px no-repeat,#fff url(\"data:image/svg+xml,%3csvg xmlns='http://www.w3.org/2000/svg' width='8' height='8' viewBox='0 0 8 8'%3e%3cpath fill='%2328a745' d='M2.3 6.73L.6 4.53c-.4-1.04.46-1.4 1.1-.8l1.1 1.4 3.4-3.8c.6-.63 1.6-.27 1.2.7l-4 4.6c-.43.5-.8.4-1.1.1z'/%3e%3c/svg%3e\") center right 1.75rem/calc(0.75em + 0.375rem) calc(0.75em + 0.375rem) no-repeat}.was-validated .custom-select:valid:focus,.custom-select.is-valid:focus{border-color:#28a745;box-shadow:0 0 0 .2rem rgba(40,167,69,.25)}.was-validated .form-check-input:valid~.form-check-label,.form-check-input.is-valid~.form-check-label{color:#28a745}.was-validated .form-check-input:valid~.valid-feedback,.was-validated .form-check-input:valid~.valid-tooltip,.form-check-input.is-valid~.valid-feedback,.form-check-input.is-valid~.valid-tooltip{display:block}.was-validated .custom-control-input:valid~.custom-control-label,.custom-control-input.is-valid~.custom-control-label{color:#28a745}.was-validated .custom-control-input:valid~.custom-control-label::before,.custom-control-input.is-valid~.custom-control-label::before{border-color:#28a745}.was-validated .custom-control-input:valid:checked~.custom-control-label::before,.custom-control-input.is-valid:checked~.custom-control-label::before{border-color:#34ce57;background-color:#34ce57}.was-validated .custom-control-input:valid:focus~.custom-control-label::before,.custom-control-input.is-valid:focus~.custom-control-label::before{box-shadow:0 0 0 .2rem rgba(40,167,69,.25)}.was-validated .custom-control-input:valid:focus:not(:checked)~.custom-control-label::before,.custom-control-input.is-valid:focus:not(:checked)~.custom-control-label::before{border-color:#28a745}.was-validated .custom-file-input:valid~.custom-file-label,.custom-file-input.is-valid~.custom-file-label{border-color:#28a745}.was-validated .custom-file-input:valid:focus~.custom-file-label,.custom-file-input.is-valid:focus~.custom-file-label{border-color:#28a745;box-shadow:0 0 0 .2rem rgba(40,167,69,.25)}.invalid-feedback{display:none;width:100%;margin-top:.25rem;font-size:80%;color:#dc3545}.invalid-tooltip{position:absolute;top:100%;left:0;z-index:5;display:none;max-width:100%;padding:.25rem .5rem;margin-top:.1rem;font-size:0.875rem;line-height:1.5;color:#fff;background-color:rgba(220,53,69,.9);border-radius:.25rem}.form-row>.col>.invalid-tooltip,.form-row>[class*=col-]>.invalid-tooltip{left:5px}.was-validated :invalid~.invalid-feedback,.was-validated :invalid~.invalid-tooltip,.is-invalid~.invalid-feedback,.is-invalid~.invalid-tooltip{display:block}.was-validated .form-control:invalid,.form-control.is-invalid{border-color:#dc3545;padding-right:calc(1.5em + 0.75rem);background-image:url(\"data:image/svg+xml,%3csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' fill='none' stroke='%23dc3545' viewBox='0 0 12 12'%3e%3ccircle cx='6' cy='6' r='4.5'/%3e%3cpath stroke-linejoin='round' d='M5.8 3.6h.4L6 6.5z'/%3e%3ccircle cx='6' cy='8.2' r='.6' fill='%23dc3545' stroke='none'/%3e%3c/svg%3e\");background-repeat:no-repeat;background-position:right calc(0.375em + 0.1875rem) center;background-size:calc(0.75em + 0.375rem) calc(0.75em + 0.375rem)}.was-validated .form-control:invalid:focus,.form-control.is-invalid:focus{border-color:#dc3545;box-shadow:0 0 0 .2rem rgba(220,53,69,.25)}.was-validated textarea.form-control:invalid,textarea.form-control.is-invalid{padding-right:calc(1.5em + 0.75rem);background-position:top calc(0.375em + 0.1875rem) right calc(0.375em + 0.1875rem)}.was-validated .custom-select:invalid,.custom-select.is-invalid{border-color:#dc3545;padding-right:calc(0.75em + 2.3125rem);background:url(\"data:image/svg+xml,%3csvg xmlns='http://www.w3.org/2000/svg' width='4' height='5' viewBox='0 0 4 5'%3e%3cpath fill='%23343a40' d='M2 0L0 2h4zm0 5L0 3h4z'/%3e%3c/svg%3e\") right .75rem center/8px 10px no-repeat,#fff url(\"data:image/svg+xml,%3csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' fill='none' stroke='%23dc3545' viewBox='0 0 12 12'%3e%3ccircle cx='6' cy='6' r='4.5'/%3e%3cpath stroke-linejoin='round' d='M5.8 3.6h.4L6 6.5z'/%3e%3ccircle cx='6' cy='8.2' r='.6' fill='%23dc3545' stroke='none'/%3e%3c/svg%3e\") center right 1.75rem/calc(0.75em + 0.375rem) calc(0.75em + 0.375rem) no-repeat}.was-validated .custom-select:invalid:focus,.custom-select.is-invalid:focus{border-color:#dc3545;box-shadow:0 0 0 .2rem rgba(220,53,69,.25)}.was-validated .form-check-input:invalid~.form-check-label,.form-check-input.is-invalid~.form-check-label{color:#dc3545}.was-validated .form-check-input:invalid~.invalid-feedback,.was-validated .form-check-input:invalid~.invalid-tooltip,.form-check-input.is-invalid~.invalid-feedback,.form-check-input.is-invalid~.invalid-tooltip{display:block}.was-validated .custom-control-input:invalid~.custom-control-label,.custom-control-input.is-invalid~.custom-control-label{color:#dc3545}.was-validated .custom-control-input:invalid~.custom-control-label::before,.custom-control-input.is-invalid~.custom-control-label::before{border-color:#dc3545}.was-validated .custom-control-input:invalid:checked~.custom-control-label::before,.custom-control-input.is-invalid:checked~.custom-control-label::before{border-color:#e4606d;background-color:#e4606d}.was-validated .custom-control-input:invalid:focus~.custom-control-label::before,.custom-control-input.is-invalid:focus~.custom-control-label::before{box-shadow:0 0 0 .2rem rgba(220,53,69,.25)}.was-validated .custom-control-input:invalid:focus:not(:checked)~.custom-control-label::before,.custom-control-input.is-invalid:focus:not(:checked)~.custom-control-label::before{border-color:#dc3545}.was-validated .custom-file-input:invalid~.custom-file-label,.custom-file-input.is-invalid~.custom-file-label{border-color:#dc3545}.was-validated .custom-file-input:invalid:focus~.custom-file-label,.custom-file-input.is-invalid:focus~.custom-file-label{border-color:#dc3545;box-shadow:0 0 0 .2rem rgba(220,53,69,.25)}.form-inline{display:flex;flex-flow:row wrap;align-items:center}.form-inline .form-check{width:100%}@media(min-width: 576px){.form-inline label{display:flex;align-items:center;justify-content:center;margin-bottom:0}.form-inline .form-group{display:flex;flex:0 0 auto;flex-flow:row wrap;align-items:center;margin-bottom:0}.form-inline .form-control{display:inline-block;width:auto;vertical-align:middle}.form-inline .form-control-plaintext{display:inline-block}.form-inline .input-group,.form-inline .custom-select{width:auto}.form-inline .form-check{display:flex;align-items:center;justify-content:center;width:auto;padding-left:0}.form-inline .form-check-input{position:relative;flex-shrink:0;margin-top:0;margin-right:.25rem;margin-left:0}.form-inline .custom-control{align-items:center;justify-content:center}.form-inline .custom-control-label{margin-bottom:0}}.btn{display:inline-block;font-weight:400;color:#212529;text-align:center;vertical-align:middle;user-select:none;background-color:transparent;border:1px solid transparent;padding:.375rem .75rem;font-size:1rem;line-height:1.5;border-radius:.25rem;transition:color .15s ease-in-out,background-color .15s ease-in-out,border-color .15s ease-in-out,box-shadow .15s ease-in-out}@media(prefers-reduced-motion: reduce){.btn{transition:none}}.btn:hover{color:#212529;text-decoration:none}.btn:focus,.btn.focus{outline:0;box-shadow:0 0 0 .2rem rgba(0,123,255,.25)}.btn.disabled,.btn:disabled{opacity:.65}.btn:not(:disabled):not(.disabled){cursor:pointer}a.btn.disabled,fieldset:disabled a.btn{pointer-events:none}.btn-primary{color:#fff;background-color:#007bff;border-color:#007bff}.btn-primary:hover{color:#fff;background-color:#0069d9;border-color:#0062cc}.btn-primary:focus,.btn-primary.focus{color:#fff;background-color:#0069d9;border-color:#0062cc;box-shadow:0 0 0 .2rem rgba(38,143,255,.5)}.btn-primary.disabled,.btn-primary:disabled{color:#fff;background-color:#007bff;border-color:#007bff}.btn-primary:not(:disabled):not(.disabled):active,.btn-primary:not(:disabled):not(.disabled).active,.show>.btn-primary.dropdown-toggle{color:#fff;background-color:#0062cc;border-color:#005cbf}.btn-primary:not(:disabled):not(.disabled):active:focus,.btn-primary:not(:disabled):not(.disabled).active:focus,.show>.btn-primary.dropdown-toggle:focus{box-shadow:0 0 0 .2rem rgba(38,143,255,.5)}.btn-secondary{color:#fff;background-color:#6c757d;border-color:#6c757d}.btn-secondary:hover{color:#fff;background-color:#5a6268;border-color:#545b62}.btn-secondary:focus,.btn-secondary.focus{color:#fff;background-color:#5a6268;border-color:#545b62;box-shadow:0 0 0 .2rem rgba(130,138,145,.5)}.btn-secondary.disabled,.btn-secondary:disabled{color:#fff;background-color:#6c757d;border-color:#6c757d}.btn-secondary:not(:disabled):not(.disabled):active,.btn-secondary:not(:disabled):not(.disabled).active,.show>.btn-secondary.dropdown-toggle{color:#fff;background-color:#545b62;border-color:#4e555b}.btn-secondary:not(:disabled):not(.disabled):active:focus,.btn-secondary:not(:disabled):not(.disabled).active:focus,.show>.btn-secondary.dropdown-toggle:focus{box-shadow:0 0 0 .2rem rgba(130,138,145,.5)}.btn-success{color:#fff;background-color:#28a745;border-color:#28a745}.btn-success:hover{color:#fff;background-color:#218838;border-color:#1e7e34}.btn-success:focus,.btn-success.focus{color:#fff;background-color:#218838;border-color:#1e7e34;box-shadow:0 0 0 .2rem rgba(72,180,97,.5)}.btn-success.disabled,.btn-success:disabled{color:#fff;background-color:#28a745;border-color:#28a745}.btn-success:not(:disabled):not(.disabled):active,.btn-success:not(:disabled):not(.disabled).active,.show>.btn-success.dropdown-toggle{color:#fff;background-color:#1e7e34;border-color:#1c7430}.btn-success:not(:disabled):not(.disabled):active:focus,.btn-success:not(:disabled):not(.disabled).active:focus,.show>.btn-success.dropdown-toggle:focus{box-shadow:0 0 0 .2rem rgba(72,180,97,.5)}.btn-info{color:#fff;background-color:#17a2b8;border-color:#17a2b8}.btn-info:hover{color:#fff;background-color:#138496;border-color:#117a8b}.btn-info:focus,.btn-info.focus{color:#fff;background-color:#138496;border-color:#117a8b;box-shadow:0 0 0 .2rem rgba(58,176,195,.5)}.btn-info.disabled,.btn-info:disabled{color:#fff;background-color:#17a2b8;border-color:#17a2b8}.btn-info:not(:disabled):not(.disabled):active,.btn-info:not(:disabled):not(.disabled).active,.show>.btn-info.dropdown-toggle{color:#fff;background-color:#117a8b;border-color:#10707f}.btn-info:not(:disabled):not(.disabled):active:focus,.btn-info:not(:disabled):not(.disabled).active:focus,.show>.btn-info.dropdown-toggle:focus{box-shadow:0 0 0 .2rem rgba(58,176,195,.5)}.btn-warning{color:#212529;background-color:#ffc107;border-color:#ffc107}.btn-warning:hover{color:#212529;background-color:#e0a800;border-color:#d39e00}.btn-warning:focus,.btn-warning.focus{color:#212529;background-color:#e0a800;border-color:#d39e00;box-shadow:0 0 0 .2rem rgba(222,170,12,.5)}.btn-warning.disabled,.btn-warning:disabled{color:#212529;background-color:#ffc107;border-color:#ffc107}.btn-warning:not(:disabled):not(.disabled):active,.btn-warning:not(:disabled):not(.disabled).active,.show>.btn-warning.dropdown-toggle{color:#212529;background-color:#d39e00;border-color:#c69500}.btn-warning:not(:disabled):not(.disabled):active:focus,.btn-warning:not(:disabled):not(.disabled).active:focus,.show>.btn-warning.dropdown-toggle:focus{box-shadow:0 0 0 .2rem rgba(222,170,12,.5)}.btn-danger{color:#fff;background-color:#dc3545;border-color:#dc3545}.btn-danger:hover{color:#fff;background-color:#c82333;border-color:#bd2130}.btn-danger:focus,.btn-danger.focus{color:#fff;background-color:#c82333;border-color:#bd2130;box-shadow:0 0 0 .2rem rgba(225,83,97,.5)}.btn-danger.disabled,.btn-danger:disabled{color:#fff;background-color:#dc3545;border-color:#dc3545}.btn-danger:not(:disabled):not(.disabled):active,.btn-danger:not(:disabled):not(.disabled).active,.show>.btn-danger.dropdown-toggle{color:#fff;background-color:#bd2130;border-color:#b21f2d}.btn-danger:not(:disabled):not(.disabled):active:focus,.btn-danger:not(:disabled):not(.disabled).active:focus,.show>.btn-danger.dropdown-toggle:focus{box-shadow:0 0 0 .2rem rgba(225,83,97,.5)}.btn-light{color:#212529;background-color:#f8f9fa;border-color:#f8f9fa}.btn-light:hover{color:#212529;background-color:#e2e6ea;border-color:#dae0e5}.btn-light:focus,.btn-light.focus{color:#212529;background-color:#e2e6ea;border-color:#dae0e5;box-shadow:0 0 0 .2rem rgba(216,217,219,.5)}.btn-light.disabled,.btn-light:disabled{color:#212529;background-color:#f8f9fa;border-color:#f8f9fa}.btn-light:not(:disabled):not(.disabled):active,.btn-light:not(:disabled):not(.disabled).active,.show>.btn-light.dropdown-toggle{color:#212529;background-color:#dae0e5;border-color:#d3d9df}.btn-light:not(:disabled):not(.disabled):active:focus,.btn-light:not(:disabled):not(.disabled).active:focus,.show>.btn-light.dropdown-toggle:focus{box-shadow:0 0 0 .2rem rgba(216,217,219,.5)}.btn-dark{color:#fff;background-color:#343a40;border-color:#343a40}.btn-dark:hover{color:#fff;background-color:#23272b;border-color:#1d2124}.btn-dark:focus,.btn-dark.focus{color:#fff;background-color:#23272b;border-color:#1d2124;box-shadow:0 0 0 .2rem rgba(82,88,93,.5)}.btn-dark.disabled,.btn-dark:disabled{color:#fff;background-color:#343a40;border-color:#343a40}.btn-dark:not(:disabled):not(.disabled):active,.btn-dark:not(:disabled):not(.disabled).active,.show>.btn-dark.dropdown-toggle{color:#fff;background-color:#1d2124;border-color:#171a1d}.btn-dark:not(:disabled):not(.disabled):active:focus,.btn-dark:not(:disabled):not(.disabled).active:focus,.show>.btn-dark.dropdown-toggle:focus{box-shadow:0 0 0 .2rem rgba(82,88,93,.5)}.btn-outline-primary{color:#007bff;border-color:#007bff}.btn-outline-primary:hover{color:#fff;background-color:#007bff;border-color:#007bff}.btn-outline-primary:focus,.btn-outline-primary.focus{box-shadow:0 0 0 .2rem rgba(0,123,255,.5)}.btn-outline-primary.disabled,.btn-outline-primary:disabled{color:#007bff;background-color:transparent}.btn-outline-primary:not(:disabled):not(.disabled):active,.btn-outline-primary:not(:disabled):not(.disabled).active,.show>.btn-outline-primary.dropdown-toggle{color:#fff;background-color:#007bff;border-color:#007bff}.btn-outline-primary:not(:disabled):not(.disabled):active:focus,.btn-outline-primary:not(:disabled):not(.disabled).active:focus,.show>.btn-outline-primary.dropdown-toggle:focus{box-shadow:0 0 0 .2rem rgba(0,123,255,.5)}.btn-outline-secondary{color:#6c757d;border-color:#6c757d}.btn-outline-secondary:hover{color:#fff;background-color:#6c757d;border-color:#6c757d}.btn-outline-secondary:focus,.btn-outline-secondary.focus{box-shadow:0 0 0 .2rem rgba(108,117,125,.5)}.btn-outline-secondary.disabled,.btn-outline-secondary:disabled{color:#6c757d;background-color:transparent}.btn-outline-secondary:not(:disabled):not(.disabled):active,.btn-outline-secondary:not(:disabled):not(.disabled).active,.show>.btn-outline-secondary.dropdown-toggle{color:#fff;background-color:#6c757d;border-color:#6c757d}.btn-outline-secondary:not(:disabled):not(.disabled):active:focus,.btn-outline-secondary:not(:disabled):not(.disabled).active:focus,.show>.btn-outline-secondary.dropdown-toggle:focus{box-shadow:0 0 0 .2rem rgba(108,117,125,.5)}.btn-outline-success{color:#28a745;border-color:#28a745}.btn-outline-success:hover{color:#fff;background-color:#28a745;border-color:#28a745}.btn-outline-success:focus,.btn-outline-success.focus{box-shadow:0 0 0 .2rem rgba(40,167,69,.5)}.btn-outline-success.disabled,.btn-outline-success:disabled{color:#28a745;background-color:transparent}.btn-outline-success:not(:disabled):not(.disabled):active,.btn-outline-success:not(:disabled):not(.disabled).active,.show>.btn-outline-success.dropdown-toggle{color:#fff;background-color:#28a745;border-color:#28a745}.btn-outline-success:not(:disabled):not(.disabled):active:focus,.btn-outline-success:not(:disabled):not(.disabled).active:focus,.show>.btn-outline-success.dropdown-toggle:focus{box-shadow:0 0 0 .2rem rgba(40,167,69,.5)}.btn-outline-info{color:#17a2b8;border-color:#17a2b8}.btn-outline-info:hover{color:#fff;background-color:#17a2b8;border-color:#17a2b8}.btn-outline-info:focus,.btn-outline-info.focus{box-shadow:0 0 0 .2rem rgba(23,162,184,.5)}.btn-outline-info.disabled,.btn-outline-info:disabled{color:#17a2b8;background-color:transparent}.btn-outline-info:not(:disabled):not(.disabled):active,.btn-outline-info:not(:disabled):not(.disabled).active,.show>.btn-outline-info.dropdown-toggle{color:#fff;background-color:#17a2b8;border-color:#17a2b8}.btn-outline-info:not(:disabled):not(.disabled):active:focus,.btn-outline-info:not(:disabled):not(.disabled).active:focus,.show>.btn-outline-info.dropdown-toggle:focus{box-shadow:0 0 0 .2rem rgba(23,162,184,.5)}.btn-outline-warning{color:#ffc107;border-color:#ffc107}.btn-outline-warning:hover{color:#212529;background-color:#ffc107;border-color:#ffc107}.btn-outline-warning:focus,.btn-outline-warning.focus{box-shadow:0 0 0 .2rem rgba(255,193,7,.5)}.btn-outline-warning.disabled,.btn-outline-warning:disabled{color:#ffc107;background-color:transparent}.btn-outline-warning:not(:disabled):not(.disabled):active,.btn-outline-warning:not(:disabled):not(.disabled).active,.show>.btn-outline-warning.dropdown-toggle{color:#212529;background-color:#ffc107;border-color:#ffc107}.btn-outline-warning:not(:disabled):not(.disabled):active:focus,.btn-outline-warning:not(:disabled):not(.disabled).active:focus,.show>.btn-outline-warning.dropdown-toggle:focus{box-shadow:0 0 0 .2rem rgba(255,193,7,.5)}.btn-outline-danger{color:#dc3545;border-color:#dc3545}.btn-outline-danger:hover{color:#fff;background-color:#dc3545;border-color:#dc3545}.btn-outline-danger:focus,.btn-outline-danger.focus{box-shadow:0 0 0 .2rem rgba(220,53,69,.5)}.btn-outline-danger.disabled,.btn-outline-danger:disabled{color:#dc3545;background-color:transparent}.btn-outline-danger:not(:disabled):not(.disabled):active,.btn-outline-danger:not(:disabled):not(.disabled).active,.show>.btn-outline-danger.dropdown-toggle{color:#fff;background-color:#dc3545;border-color:#dc3545}.btn-outline-danger:not(:disabled):not(.disabled):active:focus,.btn-outline-danger:not(:disabled):not(.disabled).active:focus,.show>.btn-outline-danger.dropdown-toggle:focus{box-shadow:0 0 0 .2rem rgba(220,53,69,.5)}.btn-outline-light{color:#f8f9fa;border-color:#f8f9fa}.btn-outline-light:hover{color:#212529;background-color:#f8f9fa;border-color:#f8f9fa}.btn-outline-light:focus,.btn-outline-light.focus{box-shadow:0 0 0 .2rem rgba(248,249,250,.5)}.btn-outline-light.disabled,.btn-outline-light:disabled{color:#f8f9fa;background-color:transparent}.btn-outline-light:not(:disabled):not(.disabled):active,.btn-outline-light:not(:disabled):not(.disabled).active,.show>.btn-outline-light.dropdown-toggle{color:#212529;background-color:#f8f9fa;border-color:#f8f9fa}.btn-outline-light:not(:disabled):not(.disabled):active:focus,.btn-outline-light:not(:disabled):not(.disabled).active:focus,.show>.btn-outline-light.dropdown-toggle:focus{box-shadow:0 0 0 .2rem rgba(248,249,250,.5)}.btn-outline-dark{color:#343a40;border-color:#343a40}.btn-outline-dark:hover{color:#fff;background-color:#343a40;border-color:#343a40}.btn-outline-dark:focus,.btn-outline-dark.focus{box-shadow:0 0 0 .2rem rgba(52,58,64,.5)}.btn-outline-dark.disabled,.btn-outline-dark:disabled{color:#343a40;background-color:transparent}.btn-outline-dark:not(:disabled):not(.disabled):active,.btn-outline-dark:not(:disabled):not(.disabled).active,.show>.btn-outline-dark.dropdown-toggle{color:#fff;background-color:#343a40;border-color:#343a40}.btn-outline-dark:not(:disabled):not(.disabled):active:focus,.btn-outline-dark:not(:disabled):not(.disabled).active:focus,.show>.btn-outline-dark.dropdown-toggle:focus{box-shadow:0 0 0 .2rem rgba(52,58,64,.5)}.btn-link{font-weight:400;color:#007bff;text-decoration:none}.btn-link:hover{color:#0056b3;text-decoration:underline}.btn-link:focus,.btn-link.focus{text-decoration:underline}.btn-link:disabled,.btn-link.disabled{color:#6c757d;pointer-events:none}.btn-lg,.btn-group-lg>.btn{padding:.5rem 1rem;font-size:1.25rem;line-height:1.5;border-radius:.3rem}.btn-sm,.btn-group-sm>.btn{padding:.25rem .5rem;font-size:0.875rem;line-height:1.5;border-radius:.2rem}.btn-block{display:block;width:100%}.btn-block+.btn-block{margin-top:.5rem}input[type=submit].btn-block,input[type=reset].btn-block,input[type=button].btn-block{width:100%}.fade{transition:opacity .15s linear}@media(prefers-reduced-motion: reduce){.fade{transition:none}}.fade:not(.show){opacity:0}.collapse:not(.show){display:none}.collapsing{position:relative;height:0;overflow:hidden;transition:height .35s ease}@media(prefers-reduced-motion: reduce){.collapsing{transition:none}}.dropup,.dropright,.dropdown,.dropleft{position:relative}.dropdown-toggle{white-space:nowrap}.dropdown-toggle::after{display:inline-block;margin-left:.255em;vertical-align:.255em;content:\"\";border-top:.3em solid;border-right:.3em solid transparent;border-bottom:0;border-left:.3em solid transparent}.dropdown-toggle:empty::after{margin-left:0}.dropdown-menu{position:absolute;top:100%;left:0;z-index:1000;display:none;float:left;min-width:10rem;padding:.5rem 0;margin:.125rem 0 0;font-size:1rem;color:#212529;text-align:left;list-style:none;background-color:#fff;background-clip:padding-box;border:1px solid rgba(0,0,0,.15);border-radius:.25rem}.dropdown-menu-left{right:auto;left:0}.dropdown-menu-right{right:0;left:auto}@media(min-width: 576px){.dropdown-menu-sm-left{right:auto;left:0}.dropdown-menu-sm-right{right:0;left:auto}}@media(min-width: 768px){.dropdown-menu-md-left{right:auto;left:0}.dropdown-menu-md-right{right:0;left:auto}}@media(min-width: 992px){.dropdown-menu-lg-left{right:auto;left:0}.dropdown-menu-lg-right{right:0;left:auto}}@media(min-width: 1200px){.dropdown-menu-xl-left{right:auto;left:0}.dropdown-menu-xl-right{right:0;left:auto}}.dropup .dropdown-menu{top:auto;bottom:100%;margin-top:0;margin-bottom:.125rem}.dropup .dropdown-toggle::after{display:inline-block;margin-left:.255em;vertical-align:.255em;content:\"\";border-top:0;border-right:.3em solid transparent;border-bottom:.3em solid;border-left:.3em solid transparent}.dropup .dropdown-toggle:empty::after{margin-left:0}.dropright .dropdown-menu{top:0;right:auto;left:100%;margin-top:0;margin-left:.125rem}.dropright .dropdown-toggle::after{display:inline-block;margin-left:.255em;vertical-align:.255em;content:\"\";border-top:.3em solid transparent;border-right:0;border-bottom:.3em solid transparent;border-left:.3em solid}.dropright .dropdown-toggle:empty::after{margin-left:0}.dropright .dropdown-toggle::after{vertical-align:0}.dropleft .dropdown-menu{top:0;right:100%;left:auto;margin-top:0;margin-right:.125rem}.dropleft .dropdown-toggle::after{display:inline-block;margin-left:.255em;vertical-align:.255em;content:\"\"}.dropleft .dropdown-toggle::after{display:none}.dropleft .dropdown-toggle::before{display:inline-block;margin-right:.255em;vertical-align:.255em;content:\"\";border-top:.3em solid transparent;border-right:.3em solid;border-bottom:.3em solid transparent}.dropleft .dropdown-toggle:empty::after{margin-left:0}.dropleft .dropdown-toggle::before{vertical-align:0}.dropdown-menu[x-placement^=top],.dropdown-menu[x-placement^=right],.dropdown-menu[x-placement^=bottom],.dropdown-menu[x-placement^=left]{right:auto;bottom:auto}.dropdown-divider{height:0;margin:.5rem 0;overflow:hidden;border-top:1px solid #e9ecef}.dropdown-item{display:block;width:100%;padding:.25rem 1.5rem;clear:both;font-weight:400;color:#212529;text-align:inherit;white-space:nowrap;background-color:transparent;border:0}.dropdown-item:hover,.dropdown-item:focus{color:#16181b;text-decoration:none;background-color:#e9ecef}.dropdown-item.active,.dropdown-item:active{color:#fff;text-decoration:none;background-color:#007bff}.dropdown-item.disabled,.dropdown-item:disabled{color:#adb5bd;pointer-events:none;background-color:transparent}.dropdown-menu.show{display:block}.dropdown-header{display:block;padding:.5rem 1.5rem;margin-bottom:0;font-size:0.875rem;color:#6c757d;white-space:nowrap}.dropdown-item-text{display:block;padding:.25rem 1.5rem;color:#212529}.btn-group,.btn-group-vertical{position:relative;display:inline-flex;vertical-align:middle}.btn-group>.btn,.btn-group-vertical>.btn{position:relative;flex:1 1 auto}.btn-group>.btn:hover,.btn-group-vertical>.btn:hover{z-index:1}.btn-group>.btn:focus,.btn-group>.btn:active,.btn-group>.btn.active,.btn-group-vertical>.btn:focus,.btn-group-vertical>.btn:active,.btn-group-vertical>.btn.active{z-index:1}.btn-toolbar{display:flex;flex-wrap:wrap;justify-content:flex-start}.btn-toolbar .input-group{width:auto}.btn-group>.btn:not(:first-child),.btn-group>.btn-group:not(:first-child){margin-left:-1px}.btn-group>.btn:not(:last-child):not(.dropdown-toggle),.btn-group>.btn-group:not(:last-child)>.btn{border-top-right-radius:0;border-bottom-right-radius:0}.btn-group>.btn:not(:first-child),.btn-group>.btn-group:not(:first-child)>.btn{border-top-left-radius:0;border-bottom-left-radius:0}.dropdown-toggle-split{padding-right:.5625rem;padding-left:.5625rem}.dropdown-toggle-split::after,.dropup .dropdown-toggle-split::after,.dropright .dropdown-toggle-split::after{margin-left:0}.dropleft .dropdown-toggle-split::before{margin-right:0}.btn-sm+.dropdown-toggle-split,.btn-group-sm>.btn+.dropdown-toggle-split{padding-right:.375rem;padding-left:.375rem}.btn-lg+.dropdown-toggle-split,.btn-group-lg>.btn+.dropdown-toggle-split{padding-right:.75rem;padding-left:.75rem}.btn-group-vertical{flex-direction:column;align-items:flex-start;justify-content:center}.btn-group-vertical>.btn,.btn-group-vertical>.btn-group{width:100%}.btn-group-vertical>.btn:not(:first-child),.btn-group-vertical>.btn-group:not(:first-child){margin-top:-1px}.btn-group-vertical>.btn:not(:last-child):not(.dropdown-toggle),.btn-group-vertical>.btn-group:not(:last-child)>.btn{border-bottom-right-radius:0;border-bottom-left-radius:0}.btn-group-vertical>.btn:not(:first-child),.btn-group-vertical>.btn-group:not(:first-child)>.btn{border-top-left-radius:0;border-top-right-radius:0}.btn-group-toggle>.btn,.btn-group-toggle>.btn-group>.btn{margin-bottom:0}.btn-group-toggle>.btn input[type=radio],.btn-group-toggle>.btn input[type=checkbox],.btn-group-toggle>.btn-group>.btn input[type=radio],.btn-group-toggle>.btn-group>.btn input[type=checkbox]{position:absolute;clip:rect(0, 0, 0, 0);pointer-events:none}.input-group{position:relative;display:flex;flex-wrap:wrap;align-items:stretch;width:100%}.input-group>.form-control,.input-group>.form-control-plaintext,.input-group>.custom-select,.input-group>.custom-file{position:relative;flex:1 1 auto;width:1%;min-width:0;margin-bottom:0}.input-group>.form-control+.form-control,.input-group>.form-control+.custom-select,.input-group>.form-control+.custom-file,.input-group>.form-control-plaintext+.form-control,.input-group>.form-control-plaintext+.custom-select,.input-group>.form-control-plaintext+.custom-file,.input-group>.custom-select+.form-control,.input-group>.custom-select+.custom-select,.input-group>.custom-select+.custom-file,.input-group>.custom-file+.form-control,.input-group>.custom-file+.custom-select,.input-group>.custom-file+.custom-file{margin-left:-1px}.input-group>.form-control:focus,.input-group>.custom-select:focus,.input-group>.custom-file .custom-file-input:focus~.custom-file-label{z-index:3}.input-group>.custom-file .custom-file-input:focus{z-index:4}.input-group>.form-control:not(:first-child),.input-group>.custom-select:not(:first-child){border-top-left-radius:0;border-bottom-left-radius:0}.input-group>.custom-file{display:flex;align-items:center}.input-group>.custom-file:not(:last-child) .custom-file-label,.input-group>.custom-file:not(:first-child) .custom-file-label{border-top-left-radius:0;border-bottom-left-radius:0}.input-group:not(.has-validation)>.form-control:not(:last-child),.input-group:not(.has-validation)>.custom-select:not(:last-child),.input-group:not(.has-validation)>.custom-file:not(:last-child) .custom-file-label::after{border-top-right-radius:0;border-bottom-right-radius:0}.input-group.has-validation>.form-control:nth-last-child(n+3),.input-group.has-validation>.custom-select:nth-last-child(n+3),.input-group.has-validation>.custom-file:nth-last-child(n+3) .custom-file-label::after{border-top-right-radius:0;border-bottom-right-radius:0}.input-group-prepend,.input-group-append{display:flex}.input-group-prepend .btn,.input-group-append .btn{position:relative;z-index:2}.input-group-prepend .btn:focus,.input-group-append .btn:focus{z-index:3}.input-group-prepend .btn+.btn,.input-group-prepend .btn+.input-group-text,.input-group-prepend .input-group-text+.input-group-text,.input-group-prepend .input-group-text+.btn,.input-group-append .btn+.btn,.input-group-append .btn+.input-group-text,.input-group-append .input-group-text+.input-group-text,.input-group-append .input-group-text+.btn{margin-left:-1px}.input-group-prepend{margin-right:-1px}.input-group-append{margin-left:-1px}.input-group-text{display:flex;align-items:center;padding:.375rem .75rem;margin-bottom:0;font-size:1rem;font-weight:400;line-height:1.5;color:#495057;text-align:center;white-space:nowrap;background-color:#e9ecef;border:1px solid #ced4da;border-radius:.25rem}.input-group-text input[type=radio],.input-group-text input[type=checkbox]{margin-top:0}.input-group-lg>.form-control:not(textarea),.input-group-lg>.custom-select{height:calc(1.5em + 1rem + 2px)}.input-group-lg>.form-control,.input-group-lg>.custom-select,.input-group-lg>.input-group-prepend>.input-group-text,.input-group-lg>.input-group-append>.input-group-text,.input-group-lg>.input-group-prepend>.btn,.input-group-lg>.input-group-append>.btn{padding:.5rem 1rem;font-size:1.25rem;line-height:1.5;border-radius:.3rem}.input-group-sm>.form-control:not(textarea),.input-group-sm>.custom-select{height:calc(1.5em + 0.5rem + 2px)}.input-group-sm>.form-control,.input-group-sm>.custom-select,.input-group-sm>.input-group-prepend>.input-group-text,.input-group-sm>.input-group-append>.input-group-text,.input-group-sm>.input-group-prepend>.btn,.input-group-sm>.input-group-append>.btn{padding:.25rem .5rem;font-size:0.875rem;line-height:1.5;border-radius:.2rem}.input-group-lg>.custom-select,.input-group-sm>.custom-select{padding-right:1.75rem}.input-group>.input-group-prepend>.btn,.input-group>.input-group-prepend>.input-group-text,.input-group:not(.has-validation)>.input-group-append:not(:last-child)>.btn,.input-group:not(.has-validation)>.input-group-append:not(:last-child)>.input-group-text,.input-group.has-validation>.input-group-append:nth-last-child(n+3)>.btn,.input-group.has-validation>.input-group-append:nth-last-child(n+3)>.input-group-text,.input-group>.input-group-append:last-child>.btn:not(:last-child):not(.dropdown-toggle),.input-group>.input-group-append:last-child>.input-group-text:not(:last-child){border-top-right-radius:0;border-bottom-right-radius:0}.input-group>.input-group-append>.btn,.input-group>.input-group-append>.input-group-text,.input-group>.input-group-prepend:not(:first-child)>.btn,.input-group>.input-group-prepend:not(:first-child)>.input-group-text,.input-group>.input-group-prepend:first-child>.btn:not(:first-child),.input-group>.input-group-prepend:first-child>.input-group-text:not(:first-child){border-top-left-radius:0;border-bottom-left-radius:0}.custom-control{position:relative;z-index:1;display:block;min-height:1.5rem;padding-left:1.5rem;color-adjust:exact}.custom-control-inline{display:inline-flex;margin-right:1rem}.custom-control-input{position:absolute;left:0;z-index:-1;width:1rem;height:1.25rem;opacity:0}.custom-control-input:checked~.custom-control-label::before{color:#fff;border-color:#007bff;background-color:#007bff}.custom-control-input:focus~.custom-control-label::before{box-shadow:0 0 0 .2rem rgba(0,123,255,.25)}.custom-control-input:focus:not(:checked)~.custom-control-label::before{border-color:#80bdff}.custom-control-input:not(:disabled):active~.custom-control-label::before{color:#fff;background-color:#b3d7ff;border-color:#b3d7ff}.custom-control-input[disabled]~.custom-control-label,.custom-control-input:disabled~.custom-control-label{color:#6c757d}.custom-control-input[disabled]~.custom-control-label::before,.custom-control-input:disabled~.custom-control-label::before{background-color:#e9ecef}.custom-control-label{position:relative;margin-bottom:0;vertical-align:top}.custom-control-label::before{position:absolute;top:.25rem;left:-1.5rem;display:block;width:1rem;height:1rem;pointer-events:none;content:\"\";background-color:#fff;border:#adb5bd solid 1px}.custom-control-label::after{position:absolute;top:.25rem;left:-1.5rem;display:block;width:1rem;height:1rem;content:\"\";background:50%/50% 50% no-repeat}.custom-checkbox .custom-control-label::before{border-radius:.25rem}.custom-checkbox .custom-control-input:checked~.custom-control-label::after{background-image:url(\"data:image/svg+xml,%3csvg xmlns='http://www.w3.org/2000/svg' width='8' height='8' viewBox='0 0 8 8'%3e%3cpath fill='%23fff' d='M6.564.75l-3.59 3.612-1.538-1.55L0 4.26l2.974 2.99L8 2.193z'/%3e%3c/svg%3e\")}.custom-checkbox .custom-control-input:indeterminate~.custom-control-label::before{border-color:#007bff;background-color:#007bff}.custom-checkbox .custom-control-input:indeterminate~.custom-control-label::after{background-image:url(\"data:image/svg+xml,%3csvg xmlns='http://www.w3.org/2000/svg' width='4' height='4' viewBox='0 0 4 4'%3e%3cpath stroke='%23fff' d='M0 2h4'/%3e%3c/svg%3e\")}.custom-checkbox .custom-control-input:disabled:checked~.custom-control-label::before{background-color:rgba(0,123,255,.5)}.custom-checkbox .custom-control-input:disabled:indeterminate~.custom-control-label::before{background-color:rgba(0,123,255,.5)}.custom-radio .custom-control-label::before{border-radius:50%}.custom-radio .custom-control-input:checked~.custom-control-label::after{background-image:url(\"data:image/svg+xml,%3csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='-4 -4 8 8'%3e%3ccircle r='3' fill='%23fff'/%3e%3c/svg%3e\")}.custom-radio .custom-control-input:disabled:checked~.custom-control-label::before{background-color:rgba(0,123,255,.5)}.custom-switch{padding-left:2.25rem}.custom-switch .custom-control-label::before{left:-2.25rem;width:1.75rem;pointer-events:all;border-radius:.5rem}.custom-switch .custom-control-label::after{top:calc(0.25rem + 2px);left:calc(-2.25rem + 2px);width:calc(1rem - 4px);height:calc(1rem - 4px);background-color:#adb5bd;border-radius:.5rem;transition:transform .15s ease-in-out,background-color .15s ease-in-out,border-color .15s ease-in-out,box-shadow .15s ease-in-out}@media(prefers-reduced-motion: reduce){.custom-switch .custom-control-label::after{transition:none}}.custom-switch .custom-control-input:checked~.custom-control-label::after{background-color:#fff;transform:translateX(0.75rem)}.custom-switch .custom-control-input:disabled:checked~.custom-control-label::before{background-color:rgba(0,123,255,.5)}.custom-select{display:inline-block;width:100%;height:calc(1.5em + 0.75rem + 2px);padding:.375rem 1.75rem .375rem .75rem;font-size:1rem;font-weight:400;line-height:1.5;color:#495057;vertical-align:middle;background:#fff url(\"data:image/svg+xml,%3csvg xmlns='http://www.w3.org/2000/svg' width='4' height='5' viewBox='0 0 4 5'%3e%3cpath fill='%23343a40' d='M2 0L0 2h4zm0 5L0 3h4z'/%3e%3c/svg%3e\") right .75rem center/8px 10px no-repeat;border:1px solid #ced4da;border-radius:.25rem;appearance:none}.custom-select:focus{border-color:#80bdff;outline:0;box-shadow:0 0 0 .2rem rgba(0,123,255,.25)}.custom-select:focus::-ms-value{color:#495057;background-color:#fff}.custom-select[multiple],.custom-select[size]:not([size=\"1\"]){height:auto;padding-right:.75rem;background-image:none}.custom-select:disabled{color:#6c757d;background-color:#e9ecef}.custom-select::-ms-expand{display:none}.custom-select:-moz-focusring{color:transparent;text-shadow:0 0 0 #495057}.custom-select-sm{height:calc(1.5em + 0.5rem + 2px);padding-top:.25rem;padding-bottom:.25rem;padding-left:.5rem;font-size:0.875rem}.custom-select-lg{height:calc(1.5em + 1rem + 2px);padding-top:.5rem;padding-bottom:.5rem;padding-left:1rem;font-size:1.25rem}.custom-file{position:relative;display:inline-block;width:100%;height:calc(1.5em + 0.75rem + 2px);margin-bottom:0}.custom-file-input{position:relative;z-index:2;width:100%;height:calc(1.5em + 0.75rem + 2px);margin:0;overflow:hidden;opacity:0}.custom-file-input:focus~.custom-file-label{border-color:#80bdff;box-shadow:0 0 0 .2rem rgba(0,123,255,.25)}.custom-file-input[disabled]~.custom-file-label,.custom-file-input:disabled~.custom-file-label{background-color:#e9ecef}.custom-file-input:lang(en)~.custom-file-label::after{content:\"Browse\"}.custom-file-input~.custom-file-label[data-browse]::after{content:attr(data-browse)}.custom-file-label{position:absolute;top:0;right:0;left:0;z-index:1;height:calc(1.5em + 0.75rem + 2px);padding:.375rem .75rem;overflow:hidden;font-weight:400;line-height:1.5;color:#495057;background-color:#fff;border:1px solid #ced4da;border-radius:.25rem}.custom-file-label::after{position:absolute;top:0;right:0;bottom:0;z-index:3;display:block;height:calc(1.5em + 0.75rem);padding:.375rem .75rem;line-height:1.5;color:#495057;content:\"Browse\";background-color:#e9ecef;border-left:inherit;border-radius:0 .25rem .25rem 0}.custom-range{width:100%;height:1.4rem;padding:0;background-color:transparent;appearance:none}.custom-range:focus{outline:0}.custom-range:focus::-webkit-slider-thumb{box-shadow:0 0 0 1px #fff,0 0 0 .2rem rgba(0,123,255,.25)}.custom-range:focus::-moz-range-thumb{box-shadow:0 0 0 1px #fff,0 0 0 .2rem rgba(0,123,255,.25)}.custom-range:focus::-ms-thumb{box-shadow:0 0 0 1px #fff,0 0 0 .2rem rgba(0,123,255,.25)}.custom-range::-moz-focus-outer{border:0}.custom-range::-webkit-slider-thumb{width:1rem;height:1rem;margin-top:-0.25rem;background-color:#007bff;border:0;border-radius:1rem;transition:background-color .15s ease-in-out,border-color .15s ease-in-out,box-shadow .15s ease-in-out;appearance:none}@media(prefers-reduced-motion: reduce){.custom-range::-webkit-slider-thumb{transition:none}}.custom-range::-webkit-slider-thumb:active{background-color:#b3d7ff}.custom-range::-webkit-slider-runnable-track{width:100%;height:.5rem;color:transparent;cursor:pointer;background-color:#dee2e6;border-color:transparent;border-radius:1rem}.custom-range::-moz-range-thumb{width:1rem;height:1rem;background-color:#007bff;border:0;border-radius:1rem;transition:background-color .15s ease-in-out,border-color .15s ease-in-out,box-shadow .15s ease-in-out;appearance:none}@media(prefers-reduced-motion: reduce){.custom-range::-moz-range-thumb{transition:none}}.custom-range::-moz-range-thumb:active{background-color:#b3d7ff}.custom-range::-moz-range-track{width:100%;height:.5rem;color:transparent;cursor:pointer;background-color:#dee2e6;border-color:transparent;border-radius:1rem}.custom-range::-ms-thumb{width:1rem;height:1rem;margin-top:0;margin-right:.2rem;margin-left:.2rem;background-color:#007bff;border:0;border-radius:1rem;transition:background-color .15s ease-in-out,border-color .15s ease-in-out,box-shadow .15s ease-in-out;appearance:none}@media(prefers-reduced-motion: reduce){.custom-range::-ms-thumb{transition:none}}.custom-range::-ms-thumb:active{background-color:#b3d7ff}.custom-range::-ms-track{width:100%;height:.5rem;color:transparent;cursor:pointer;background-color:transparent;border-color:transparent;border-width:.5rem}.custom-range::-ms-fill-lower{background-color:#dee2e6;border-radius:1rem}.custom-range::-ms-fill-upper{margin-right:15px;background-color:#dee2e6;border-radius:1rem}.custom-range:disabled::-webkit-slider-thumb{background-color:#adb5bd}.custom-range:disabled::-webkit-slider-runnable-track{cursor:default}.custom-range:disabled::-moz-range-thumb{background-color:#adb5bd}.custom-range:disabled::-moz-range-track{cursor:default}.custom-range:disabled::-ms-thumb{background-color:#adb5bd}.custom-control-label::before,.custom-file-label,.custom-select{transition:background-color .15s ease-in-out,border-color .15s ease-in-out,box-shadow .15s ease-in-out}@media(prefers-reduced-motion: reduce){.custom-control-label::before,.custom-file-label,.custom-select{transition:none}}.nav{display:flex;flex-wrap:wrap;padding-left:0;margin-bottom:0;list-style:none}.nav-link{display:block;padding:.5rem 1rem}.nav-link:hover,.nav-link:focus{text-decoration:none}.nav-link.disabled{color:#6c757d;pointer-events:none;cursor:default}.nav-tabs{border-bottom:1px solid #dee2e6}.nav-tabs .nav-link{margin-bottom:-1px;border:1px solid transparent;border-top-left-radius:.25rem;border-top-right-radius:.25rem}.nav-tabs .nav-link:hover,.nav-tabs .nav-link:focus{border-color:#e9ecef #e9ecef #dee2e6}.nav-tabs .nav-link.disabled{color:#6c757d;background-color:transparent;border-color:transparent}.nav-tabs .nav-link.active,.nav-tabs .nav-item.show .nav-link{color:#495057;background-color:#fff;border-color:#dee2e6 #dee2e6 #fff}.nav-tabs .dropdown-menu{margin-top:-1px;border-top-left-radius:0;border-top-right-radius:0}.nav-pills .nav-link{border-radius:.25rem}.nav-pills .nav-link.active,.nav-pills .show>.nav-link{color:#fff;background-color:#007bff}.nav-fill>.nav-link,.nav-fill .nav-item{flex:1 1 auto;text-align:center}.nav-justified>.nav-link,.nav-justified .nav-item{flex-basis:0;flex-grow:1;text-align:center}.tab-content>.tab-pane{display:none}.tab-content>.active{display:block}.navbar{position:relative;display:flex;flex-wrap:wrap;align-items:center;justify-content:space-between;padding:.5rem 1rem}.navbar .container,.navbar .container-fluid,.navbar .container-sm,.navbar .container-md,.navbar .container-lg,.navbar .container-xl{display:flex;flex-wrap:wrap;align-items:center;justify-content:space-between}.navbar-brand{display:inline-block;padding-top:.3125rem;padding-bottom:.3125rem;margin-right:1rem;font-size:1.25rem;line-height:inherit;white-space:nowrap}.navbar-brand:hover,.navbar-brand:focus{text-decoration:none}.navbar-nav{display:flex;flex-direction:column;padding-left:0;margin-bottom:0;list-style:none}.navbar-nav .nav-link{padding-right:0;padding-left:0}.navbar-nav .dropdown-menu{position:static;float:none}.navbar-text{display:inline-block;padding-top:.5rem;padding-bottom:.5rem}.navbar-collapse{flex-basis:100%;flex-grow:1;align-items:center}.navbar-toggler{padding:.25rem .75rem;font-size:1.25rem;line-height:1;background-color:transparent;border:1px solid transparent;border-radius:.25rem}.navbar-toggler:hover,.navbar-toggler:focus{text-decoration:none}.navbar-toggler-icon{display:inline-block;width:1.5em;height:1.5em;vertical-align:middle;content:\"\";background:50%/100% 100% no-repeat}.navbar-nav-scroll{max-height:75vh;overflow-y:auto}@media(max-width: 575.98px){.navbar-expand-sm>.container,.navbar-expand-sm>.container-fluid,.navbar-expand-sm>.container-sm,.navbar-expand-sm>.container-md,.navbar-expand-sm>.container-lg,.navbar-expand-sm>.container-xl{padding-right:0;padding-left:0}}@media(min-width: 576px){.navbar-expand-sm{flex-flow:row nowrap;justify-content:flex-start}.navbar-expand-sm .navbar-nav{flex-direction:row}.navbar-expand-sm .navbar-nav .dropdown-menu{position:absolute}.navbar-expand-sm .navbar-nav .nav-link{padding-right:.5rem;padding-left:.5rem}.navbar-expand-sm>.container,.navbar-expand-sm>.container-fluid,.navbar-expand-sm>.container-sm,.navbar-expand-sm>.container-md,.navbar-expand-sm>.container-lg,.navbar-expand-sm>.container-xl{flex-wrap:nowrap}.navbar-expand-sm .navbar-nav-scroll{overflow:visible}.navbar-expand-sm .navbar-collapse{display:flex !important;flex-basis:auto}.navbar-expand-sm .navbar-toggler{display:none}}@media(max-width: 767.98px){.navbar-expand-md>.container,.navbar-expand-md>.container-fluid,.navbar-expand-md>.container-sm,.navbar-expand-md>.container-md,.navbar-expand-md>.container-lg,.navbar-expand-md>.container-xl{padding-right:0;padding-left:0}}@media(min-width: 768px){.navbar-expand-md{flex-flow:row nowrap;justify-content:flex-start}.navbar-expand-md .navbar-nav{flex-direction:row}.navbar-expand-md .navbar-nav .dropdown-menu{position:absolute}.navbar-expand-md .navbar-nav .nav-link{padding-right:.5rem;padding-left:.5rem}.navbar-expand-md>.container,.navbar-expand-md>.container-fluid,.navbar-expand-md>.container-sm,.navbar-expand-md>.container-md,.navbar-expand-md>.container-lg,.navbar-expand-md>.container-xl{flex-wrap:nowrap}.navbar-expand-md .navbar-nav-scroll{overflow:visible}.navbar-expand-md .navbar-collapse{display:flex !important;flex-basis:auto}.navbar-expand-md .navbar-toggler{display:none}}@media(max-width: 991.98px){.navbar-expand-lg>.container,.navbar-expand-lg>.container-fluid,.navbar-expand-lg>.container-sm,.navbar-expand-lg>.container-md,.navbar-expand-lg>.container-lg,.navbar-expand-lg>.container-xl{padding-right:0;padding-left:0}}@media(min-width: 992px){.navbar-expand-lg{flex-flow:row nowrap;justify-content:flex-start}.navbar-expand-lg .navbar-nav{flex-direction:row}.navbar-expand-lg .navbar-nav .dropdown-menu{position:absolute}.navbar-expand-lg .navbar-nav .nav-link{padding-right:.5rem;padding-left:.5rem}.navbar-expand-lg>.container,.navbar-expand-lg>.container-fluid,.navbar-expand-lg>.container-sm,.navbar-expand-lg>.container-md,.navbar-expand-lg>.container-lg,.navbar-expand-lg>.container-xl{flex-wrap:nowrap}.navbar-expand-lg .navbar-nav-scroll{overflow:visible}.navbar-expand-lg .navbar-collapse{display:flex !important;flex-basis:auto}.navbar-expand-lg .navbar-toggler{display:none}}@media(max-width: 1199.98px){.navbar-expand-xl>.container,.navbar-expand-xl>.container-fluid,.navbar-expand-xl>.container-sm,.navbar-expand-xl>.container-md,.navbar-expand-xl>.container-lg,.navbar-expand-xl>.container-xl{padding-right:0;padding-left:0}}@media(min-width: 1200px){.navbar-expand-xl{flex-flow:row nowrap;justify-content:flex-start}.navbar-expand-xl .navbar-nav{flex-direction:row}.navbar-expand-xl .navbar-nav .dropdown-menu{position:absolute}.navbar-expand-xl .navbar-nav .nav-link{padding-right:.5rem;padding-left:.5rem}.navbar-expand-xl>.container,.navbar-expand-xl>.container-fluid,.navbar-expand-xl>.container-sm,.navbar-expand-xl>.container-md,.navbar-expand-xl>.container-lg,.navbar-expand-xl>.container-xl{flex-wrap:nowrap}.navbar-expand-xl .navbar-nav-scroll{overflow:visible}.navbar-expand-xl .navbar-collapse{display:flex !important;flex-basis:auto}.navbar-expand-xl .navbar-toggler{display:none}}.navbar-expand{flex-flow:row nowrap;justify-content:flex-start}.navbar-expand>.container,.navbar-expand>.container-fluid,.navbar-expand>.container-sm,.navbar-expand>.container-md,.navbar-expand>.container-lg,.navbar-expand>.container-xl{padding-right:0;padding-left:0}.navbar-expand .navbar-nav{flex-direction:row}.navbar-expand .navbar-nav .dropdown-menu{position:absolute}.navbar-expand .navbar-nav .nav-link{padding-right:.5rem;padding-left:.5rem}.navbar-expand>.container,.navbar-expand>.container-fluid,.navbar-expand>.container-sm,.navbar-expand>.container-md,.navbar-expand>.container-lg,.navbar-expand>.container-xl{flex-wrap:nowrap}.navbar-expand .navbar-nav-scroll{overflow:visible}.navbar-expand .navbar-collapse{display:flex !important;flex-basis:auto}.navbar-expand .navbar-toggler{display:none}.navbar-light .navbar-brand{color:rgba(0,0,0,.9)}.navbar-light .navbar-brand:hover,.navbar-light .navbar-brand:focus{color:rgba(0,0,0,.9)}.navbar-light .navbar-nav .nav-link{color:rgba(0,0,0,.5)}.navbar-light .navbar-nav .nav-link:hover,.navbar-light .navbar-nav .nav-link:focus{color:rgba(0,0,0,.7)}.navbar-light .navbar-nav .nav-link.disabled{color:rgba(0,0,0,.3)}.navbar-light .navbar-nav .show>.nav-link,.navbar-light .navbar-nav .active>.nav-link,.navbar-light .navbar-nav .nav-link.show,.navbar-light .navbar-nav .nav-link.active{color:rgba(0,0,0,.9)}.navbar-light .navbar-toggler{color:rgba(0,0,0,.5);border-color:rgba(0,0,0,.1)}.navbar-light .navbar-toggler-icon{background-image:url(\"data:image/svg+xml,%3csvg xmlns='http://www.w3.org/2000/svg' width='30' height='30' viewBox='0 0 30 30'%3e%3cpath stroke='rgba%280, 0, 0, 0.5%29' stroke-linecap='round' stroke-miterlimit='10' stroke-width='2' d='M4 7h22M4 15h22M4 23h22'/%3e%3c/svg%3e\")}.navbar-light .navbar-text{color:rgba(0,0,0,.5)}.navbar-light .navbar-text a{color:rgba(0,0,0,.9)}.navbar-light .navbar-text a:hover,.navbar-light .navbar-text a:focus{color:rgba(0,0,0,.9)}.navbar-dark .navbar-brand{color:#fff}.navbar-dark .navbar-brand:hover,.navbar-dark .navbar-brand:focus{color:#fff}.navbar-dark .navbar-nav .nav-link{color:rgba(255,255,255,.5)}.navbar-dark .navbar-nav .nav-link:hover,.navbar-dark .navbar-nav .nav-link:focus{color:rgba(255,255,255,.75)}.navbar-dark .navbar-nav .nav-link.disabled{color:rgba(255,255,255,.25)}.navbar-dark .navbar-nav .show>.nav-link,.navbar-dark .navbar-nav .active>.nav-link,.navbar-dark .navbar-nav .nav-link.show,.navbar-dark .navbar-nav .nav-link.active{color:#fff}.navbar-dark .navbar-toggler{color:rgba(255,255,255,.5);border-color:rgba(255,255,255,.1)}.navbar-dark .navbar-toggler-icon{background-image:url(\"data:image/svg+xml,%3csvg xmlns='http://www.w3.org/2000/svg' width='30' height='30' viewBox='0 0 30 30'%3e%3cpath stroke='rgba%28255, 255, 255, 0.5%29' stroke-linecap='round' stroke-miterlimit='10' stroke-width='2' d='M4 7h22M4 15h22M4 23h22'/%3e%3c/svg%3e\")}.navbar-dark .navbar-text{color:rgba(255,255,255,.5)}.navbar-dark .navbar-text a{color:#fff}.navbar-dark .navbar-text a:hover,.navbar-dark .navbar-text a:focus{color:#fff}.card{position:relative;display:flex;flex-direction:column;min-width:0;word-wrap:break-word;background-color:#fff;background-clip:border-box;border:1px solid rgba(0,0,0,.125);border-radius:.25rem}.card>hr{margin-right:0;margin-left:0}.card>.list-group{border-top:inherit;border-bottom:inherit}.card>.list-group:first-child{border-top-width:0;border-top-left-radius:calc(0.25rem - 1px);border-top-right-radius:calc(0.25rem - 1px)}.card>.list-group:last-child{border-bottom-width:0;border-bottom-right-radius:calc(0.25rem - 1px);border-bottom-left-radius:calc(0.25rem - 1px)}.card>.card-header+.list-group,.card>.list-group+.card-footer{border-top:0}.card-body{flex:1 1 auto;min-height:1px;padding:1.25rem}.card-title{margin-bottom:.75rem}.card-subtitle{margin-top:-0.375rem;margin-bottom:0}.card-text:last-child{margin-bottom:0}.card-link:hover{text-decoration:none}.card-link+.card-link{margin-left:1.25rem}.card-header{padding:.75rem 1.25rem;margin-bottom:0;background-color:rgba(0,0,0,.03);border-bottom:1px solid rgba(0,0,0,.125)}.card-header:first-child{border-radius:calc(0.25rem - 1px) calc(0.25rem - 1px) 0 0}.card-footer{padding:.75rem 1.25rem;background-color:rgba(0,0,0,.03);border-top:1px solid rgba(0,0,0,.125)}.card-footer:last-child{border-radius:0 0 calc(0.25rem - 1px) calc(0.25rem - 1px)}.card-header-tabs{margin-right:-0.625rem;margin-bottom:-0.75rem;margin-left:-0.625rem;border-bottom:0}.card-header-pills{margin-right:-0.625rem;margin-left:-0.625rem}.card-img-overlay{position:absolute;top:0;right:0;bottom:0;left:0;padding:1.25rem;border-radius:calc(0.25rem - 1px)}.card-img,.card-img-top,.card-img-bottom{flex-shrink:0;width:100%}.card-img,.card-img-top{border-top-left-radius:calc(0.25rem - 1px);border-top-right-radius:calc(0.25rem - 1px)}.card-img,.card-img-bottom{border-bottom-right-radius:calc(0.25rem - 1px);border-bottom-left-radius:calc(0.25rem - 1px)}.card-deck .card{margin-bottom:15px}@media(min-width: 576px){.card-deck{display:flex;flex-flow:row wrap;margin-right:-15px;margin-left:-15px}.card-deck .card{flex:1 0 0%;margin-right:15px;margin-bottom:0;margin-left:15px}}.card-group>.card{margin-bottom:15px}@media(min-width: 576px){.card-group{display:flex;flex-flow:row wrap}.card-group>.card{flex:1 0 0%;margin-bottom:0}.card-group>.card+.card{margin-left:0;border-left:0}.card-group>.card:not(:last-child){border-top-right-radius:0;border-bottom-right-radius:0}.card-group>.card:not(:last-child) .card-img-top,.card-group>.card:not(:last-child) .card-header{border-top-right-radius:0}.card-group>.card:not(:last-child) .card-img-bottom,.card-group>.card:not(:last-child) .card-footer{border-bottom-right-radius:0}.card-group>.card:not(:first-child){border-top-left-radius:0;border-bottom-left-radius:0}.card-group>.card:not(:first-child) .card-img-top,.card-group>.card:not(:first-child) .card-header{border-top-left-radius:0}.card-group>.card:not(:first-child) .card-img-bottom,.card-group>.card:not(:first-child) .card-footer{border-bottom-left-radius:0}}.card-columns .card{margin-bottom:.75rem}@media(min-width: 576px){.card-columns{column-count:3;column-gap:1.25rem;orphans:1;widows:1}.card-columns .card{display:inline-block;width:100%}}.accordion{overflow-anchor:none}.accordion>.card{overflow:hidden}.accordion>.card:not(:last-of-type){border-bottom:0;border-bottom-right-radius:0;border-bottom-left-radius:0}.accordion>.card:not(:first-of-type){border-top-left-radius:0;border-top-right-radius:0}.accordion>.card>.card-header{border-radius:0;margin-bottom:-1px}.breadcrumb{display:flex;flex-wrap:wrap;padding:.75rem 1rem;margin-bottom:1rem;list-style:none;background-color:#e9ecef;border-radius:.25rem}.breadcrumb-item+.breadcrumb-item{padding-left:.5rem}.breadcrumb-item+.breadcrumb-item::before{float:left;padding-right:.5rem;color:#6c757d;content:\"/\"}.breadcrumb-item+.breadcrumb-item:hover::before{text-decoration:underline}.breadcrumb-item+.breadcrumb-item:hover::before{text-decoration:none}.breadcrumb-item.active{color:#6c757d}.pagination{display:flex;padding-left:0;list-style:none;border-radius:.25rem}.page-link{position:relative;display:block;padding:.5rem .75rem;margin-left:-1px;line-height:1.25;color:#007bff;background-color:#fff;border:1px solid #dee2e6}.page-link:hover{z-index:2;color:#0056b3;text-decoration:none;background-color:#e9ecef;border-color:#dee2e6}.page-link:focus{z-index:3;outline:0;box-shadow:0 0 0 .2rem rgba(0,123,255,.25)}.page-item:first-child .page-link{margin-left:0;border-top-left-radius:.25rem;border-bottom-left-radius:.25rem}.page-item:last-child .page-link{border-top-right-radius:.25rem;border-bottom-right-radius:.25rem}.page-item.active .page-link{z-index:3;color:#fff;background-color:#007bff;border-color:#007bff}.page-item.disabled .page-link{color:#6c757d;pointer-events:none;cursor:auto;background-color:#fff;border-color:#dee2e6}.pagination-lg .page-link{padding:.75rem 1.5rem;font-size:1.25rem;line-height:1.5}.pagination-lg .page-item:first-child .page-link{border-top-left-radius:.3rem;border-bottom-left-radius:.3rem}.pagination-lg .page-item:last-child .page-link{border-top-right-radius:.3rem;border-bottom-right-radius:.3rem}.pagination-sm .page-link{padding:.25rem .5rem;font-size:0.875rem;line-height:1.5}.pagination-sm .page-item:first-child .page-link{border-top-left-radius:.2rem;border-bottom-left-radius:.2rem}.pagination-sm .page-item:last-child .page-link{border-top-right-radius:.2rem;border-bottom-right-radius:.2rem}.badge{display:inline-block;padding:.25em .4em;font-size:75%;font-weight:700;line-height:1;text-align:center;white-space:nowrap;vertical-align:baseline;border-radius:.25rem;transition:color .15s ease-in-out,background-color .15s ease-in-out,border-color .15s ease-in-out,box-shadow .15s ease-in-out}@media(prefers-reduced-motion: reduce){.badge{transition:none}}a.badge:hover,a.badge:focus{text-decoration:none}.badge:empty{display:none}.btn .badge{position:relative;top:-1px}.badge-pill{padding-right:.6em;padding-left:.6em;border-radius:10rem}.badge-primary{color:#fff;background-color:#007bff}a.badge-primary:hover,a.badge-primary:focus{color:#fff;background-color:#0062cc}a.badge-primary:focus,a.badge-primary.focus{outline:0;box-shadow:0 0 0 .2rem rgba(0,123,255,.5)}.badge-secondary{color:#fff;background-color:#6c757d}a.badge-secondary:hover,a.badge-secondary:focus{color:#fff;background-color:#545b62}a.badge-secondary:focus,a.badge-secondary.focus{outline:0;box-shadow:0 0 0 .2rem rgba(108,117,125,.5)}.badge-success{color:#fff;background-color:#28a745}a.badge-success:hover,a.badge-success:focus{color:#fff;background-color:#1e7e34}a.badge-success:focus,a.badge-success.focus{outline:0;box-shadow:0 0 0 .2rem rgba(40,167,69,.5)}.badge-info{color:#fff;background-color:#17a2b8}a.badge-info:hover,a.badge-info:focus{color:#fff;background-color:#117a8b}a.badge-info:focus,a.badge-info.focus{outline:0;box-shadow:0 0 0 .2rem rgba(23,162,184,.5)}.badge-warning{color:#212529;background-color:#ffc107}a.badge-warning:hover,a.badge-warning:focus{color:#212529;background-color:#d39e00}a.badge-warning:focus,a.badge-warning.focus{outline:0;box-shadow:0 0 0 .2rem rgba(255,193,7,.5)}.badge-danger{color:#fff;background-color:#dc3545}a.badge-danger:hover,a.badge-danger:focus{color:#fff;background-color:#bd2130}a.badge-danger:focus,a.badge-danger.focus{outline:0;box-shadow:0 0 0 .2rem rgba(220,53,69,.5)}.badge-light{color:#212529;background-color:#f8f9fa}a.badge-light:hover,a.badge-light:focus{color:#212529;background-color:#dae0e5}a.badge-light:focus,a.badge-light.focus{outline:0;box-shadow:0 0 0 .2rem rgba(248,249,250,.5)}.badge-dark{color:#fff;background-color:#343a40}a.badge-dark:hover,a.badge-dark:focus{color:#fff;background-color:#1d2124}a.badge-dark:focus,a.badge-dark.focus{outline:0;box-shadow:0 0 0 .2rem rgba(52,58,64,.5)}.jumbotron{padding:2rem 1rem;margin-bottom:2rem;background-color:#e9ecef;border-radius:.3rem}@media(min-width: 576px){.jumbotron{padding:4rem 2rem}}.jumbotron-fluid{padding-right:0;padding-left:0;border-radius:0}.alert{position:relative;padding:.75rem 1.25rem;margin-bottom:1rem;border:1px solid transparent;border-radius:.25rem}.alert-heading{color:inherit}.alert-link{font-weight:700}.alert-dismissible{padding-right:4rem}.alert-dismissible .close{position:absolute;top:0;right:0;z-index:2;padding:.75rem 1.25rem;color:inherit}.alert-primary{color:#004085;background-color:#cce5ff;border-color:#b8daff}.alert-primary hr{border-top-color:#9fcdff}.alert-primary .alert-link{color:#002752}.alert-secondary{color:#383d41;background-color:#e2e3e5;border-color:#d6d8db}.alert-secondary hr{border-top-color:#c8cbcf}.alert-secondary .alert-link{color:#202326}.alert-success{color:#155724;background-color:#d4edda;border-color:#c3e6cb}.alert-success hr{border-top-color:#b1dfbb}.alert-success .alert-link{color:#0b2e13}.alert-info{color:#0c5460;background-color:#d1ecf1;border-color:#bee5eb}.alert-info hr{border-top-color:#abdde5}.alert-info .alert-link{color:#062c33}.alert-warning{color:#856404;background-color:#fff3cd;border-color:#ffeeba}.alert-warning hr{border-top-color:#ffe8a1}.alert-warning .alert-link{color:#533f03}.alert-danger{color:#721c24;background-color:#f8d7da;border-color:#f5c6cb}.alert-danger hr{border-top-color:#f1b0b7}.alert-danger .alert-link{color:#491217}.alert-light{color:#818182;background-color:#fefefe;border-color:#fdfdfe}.alert-light hr{border-top-color:#ececf6}.alert-light .alert-link{color:#686868}.alert-dark{color:#1b1e21;background-color:#d6d8d9;border-color:#c6c8ca}.alert-dark hr{border-top-color:#b9bbbe}.alert-dark .alert-link{color:#040505}@keyframes progress-bar-stripes{from{background-position:1rem 0}to{background-position:0 0}}.progress{display:flex;height:1rem;overflow:hidden;line-height:0;font-size:0.75rem;background-color:#e9ecef;border-radius:.25rem}.progress-bar{display:flex;flex-direction:column;justify-content:center;overflow:hidden;color:#fff;text-align:center;white-space:nowrap;background-color:#007bff;transition:width .6s ease}@media(prefers-reduced-motion: reduce){.progress-bar{transition:none}}.progress-bar-striped{background-image:linear-gradient(45deg, rgba(255, 255, 255, 0.15) 25%, transparent 25%, transparent 50%, rgba(255, 255, 255, 0.15) 50%, rgba(255, 255, 255, 0.15) 75%, transparent 75%, transparent);background-size:1rem 1rem}.progress-bar-animated{animation:1s linear infinite progress-bar-stripes}@media(prefers-reduced-motion: reduce){.progress-bar-animated{animation:none}}.media{display:flex;align-items:flex-start}.media-body{flex:1}.list-group{display:flex;flex-direction:column;padding-left:0;margin-bottom:0;border-radius:.25rem}.list-group-item-action{width:100%;color:#495057;text-align:inherit}.list-group-item-action:hover,.list-group-item-action:focus{z-index:1;color:#495057;text-decoration:none;background-color:#f8f9fa}.list-group-item-action:active{color:#212529;background-color:#e9ecef}.list-group-item{position:relative;display:block;padding:.75rem 1.25rem;background-color:#fff;border:1px solid rgba(0,0,0,.125)}.list-group-item:first-child{border-top-left-radius:inherit;border-top-right-radius:inherit}.list-group-item:last-child{border-bottom-right-radius:inherit;border-bottom-left-radius:inherit}.list-group-item.disabled,.list-group-item:disabled{color:#6c757d;pointer-events:none;background-color:#fff}.list-group-item.active{z-index:2;color:#fff;background-color:#007bff;border-color:#007bff}.list-group-item+.list-group-item{border-top-width:0}.list-group-item+.list-group-item.active{margin-top:-1px;border-top-width:1px}.list-group-horizontal{flex-direction:row}.list-group-horizontal>.list-group-item:first-child{border-bottom-left-radius:.25rem;border-top-right-radius:0}.list-group-horizontal>.list-group-item:last-child{border-top-right-radius:.25rem;border-bottom-left-radius:0}.list-group-horizontal>.list-group-item.active{margin-top:0}.list-group-horizontal>.list-group-item+.list-group-item{border-top-width:1px;border-left-width:0}.list-group-horizontal>.list-group-item+.list-group-item.active{margin-left:-1px;border-left-width:1px}@media(min-width: 576px){.list-group-horizontal-sm{flex-direction:row}.list-group-horizontal-sm>.list-group-item:first-child{border-bottom-left-radius:.25rem;border-top-right-radius:0}.list-group-horizontal-sm>.list-group-item:last-child{border-top-right-radius:.25rem;border-bottom-left-radius:0}.list-group-horizontal-sm>.list-group-item.active{margin-top:0}.list-group-horizontal-sm>.list-group-item+.list-group-item{border-top-width:1px;border-left-width:0}.list-group-horizontal-sm>.list-group-item+.list-group-item.active{margin-left:-1px;border-left-width:1px}}@media(min-width: 768px){.list-group-horizontal-md{flex-direction:row}.list-group-horizontal-md>.list-group-item:first-child{border-bottom-left-radius:.25rem;border-top-right-radius:0}.list-group-horizontal-md>.list-group-item:last-child{border-top-right-radius:.25rem;border-bottom-left-radius:0}.list-group-horizontal-md>.list-group-item.active{margin-top:0}.list-group-horizontal-md>.list-group-item+.list-group-item{border-top-width:1px;border-left-width:0}.list-group-horizontal-md>.list-group-item+.list-group-item.active{margin-left:-1px;border-left-width:1px}}@media(min-width: 992px){.list-group-horizontal-lg{flex-direction:row}.list-group-horizontal-lg>.list-group-item:first-child{border-bottom-left-radius:.25rem;border-top-right-radius:0}.list-group-horizontal-lg>.list-group-item:last-child{border-top-right-radius:.25rem;border-bottom-left-radius:0}.list-group-horizontal-lg>.list-group-item.active{margin-top:0}.list-group-horizontal-lg>.list-group-item+.list-group-item{border-top-width:1px;border-left-width:0}.list-group-horizontal-lg>.list-group-item+.list-group-item.active{margin-left:-1px;border-left-width:1px}}@media(min-width: 1200px){.list-group-horizontal-xl{flex-direction:row}.list-group-horizontal-xl>.list-group-item:first-child{border-bottom-left-radius:.25rem;border-top-right-radius:0}.list-group-horizontal-xl>.list-group-item:last-child{border-top-right-radius:.25rem;border-bottom-left-radius:0}.list-group-horizontal-xl>.list-group-item.active{margin-top:0}.list-group-horizontal-xl>.list-group-item+.list-group-item{border-top-width:1px;border-left-width:0}.list-group-horizontal-xl>.list-group-item+.list-group-item.active{margin-left:-1px;border-left-width:1px}}.list-group-flush{border-radius:0}.list-group-flush>.list-group-item{border-width:0 0 1px}.list-group-flush>.list-group-item:last-child{border-bottom-width:0}.list-group-item-primary{color:#004085;background-color:#b8daff}.list-group-item-primary.list-group-item-action:hover,.list-group-item-primary.list-group-item-action:focus{color:#004085;background-color:#9fcdff}.list-group-item-primary.list-group-item-action.active{color:#fff;background-color:#004085;border-color:#004085}.list-group-item-secondary{color:#383d41;background-color:#d6d8db}.list-group-item-secondary.list-group-item-action:hover,.list-group-item-secondary.list-group-item-action:focus{color:#383d41;background-color:#c8cbcf}.list-group-item-secondary.list-group-item-action.active{color:#fff;background-color:#383d41;border-color:#383d41}.list-group-item-success{color:#155724;background-color:#c3e6cb}.list-group-item-success.list-group-item-action:hover,.list-group-item-success.list-group-item-action:focus{color:#155724;background-color:#b1dfbb}.list-group-item-success.list-group-item-action.active{color:#fff;background-color:#155724;border-color:#155724}.list-group-item-info{color:#0c5460;background-color:#bee5eb}.list-group-item-info.list-group-item-action:hover,.list-group-item-info.list-group-item-action:focus{color:#0c5460;background-color:#abdde5}.list-group-item-info.list-group-item-action.active{color:#fff;background-color:#0c5460;border-color:#0c5460}.list-group-item-warning{color:#856404;background-color:#ffeeba}.list-group-item-warning.list-group-item-action:hover,.list-group-item-warning.list-group-item-action:focus{color:#856404;background-color:#ffe8a1}.list-group-item-warning.list-group-item-action.active{color:#fff;background-color:#856404;border-color:#856404}.list-group-item-danger{color:#721c24;background-color:#f5c6cb}.list-group-item-danger.list-group-item-action:hover,.list-group-item-danger.list-group-item-action:focus{color:#721c24;background-color:#f1b0b7}.list-group-item-danger.list-group-item-action.active{color:#fff;background-color:#721c24;border-color:#721c24}.list-group-item-light{color:#818182;background-color:#fdfdfe}.list-group-item-light.list-group-item-action:hover,.list-group-item-light.list-group-item-action:focus{color:#818182;background-color:#ececf6}.list-group-item-light.list-group-item-action.active{color:#fff;background-color:#818182;border-color:#818182}.list-group-item-dark{color:#1b1e21;background-color:#c6c8ca}.list-group-item-dark.list-group-item-action:hover,.list-group-item-dark.list-group-item-action:focus{color:#1b1e21;background-color:#b9bbbe}.list-group-item-dark.list-group-item-action.active{color:#fff;background-color:#1b1e21;border-color:#1b1e21}.close{float:right;font-size:1.5rem;font-weight:700;line-height:1;color:#000;text-shadow:0 1px 0 #fff;opacity:.5}.close:hover{color:#000;text-decoration:none}.close:not(:disabled):not(.disabled):hover,.close:not(:disabled):not(.disabled):focus{opacity:.75}button.close{padding:0;background-color:transparent;border:0}a.close.disabled{pointer-events:none}.toast{flex-basis:350px;max-width:350px;font-size:0.875rem;background-color:rgba(255,255,255,.85);background-clip:padding-box;border:1px solid rgba(0,0,0,.1);box-shadow:0 .25rem .75rem rgba(0,0,0,.1);opacity:0;border-radius:.25rem}.toast:not(:last-child){margin-bottom:.75rem}.toast.showing{opacity:1}.toast.show{display:block;opacity:1}.toast.hide{display:none}.toast-header{display:flex;align-items:center;padding:.25rem .75rem;color:#6c757d;background-color:rgba(255,255,255,.85);background-clip:padding-box;border-bottom:1px solid rgba(0,0,0,.05);border-top-left-radius:calc(0.25rem - 1px);border-top-right-radius:calc(0.25rem - 1px)}.toast-body{padding:.75rem}.modal-open{overflow:hidden}.modal-open .modal{overflow-x:hidden;overflow-y:auto}.modal{position:fixed;top:0;left:0;z-index:1050;display:none;width:100%;height:100%;overflow:hidden;outline:0}.modal-dialog{position:relative;width:auto;margin:.5rem;pointer-events:none}.modal.fade .modal-dialog{transition:transform .3s ease-out;transform:translate(0, -50px)}@media(prefers-reduced-motion: reduce){.modal.fade .modal-dialog{transition:none}}.modal.show .modal-dialog{transform:none}.modal.modal-static .modal-dialog{transform:scale(1.02)}.modal-dialog-scrollable{display:flex;max-height:calc(100% - 1rem)}.modal-dialog-scrollable .modal-content{max-height:calc(100vh - 1rem);overflow:hidden}.modal-dialog-scrollable .modal-header,.modal-dialog-scrollable .modal-footer{flex-shrink:0}.modal-dialog-scrollable .modal-body{overflow-y:auto}.modal-dialog-centered{display:flex;align-items:center;min-height:calc(100% - 1rem)}.modal-dialog-centered::before{display:block;height:calc(100vh - 1rem);height:min-content;content:\"\"}.modal-dialog-centered.modal-dialog-scrollable{flex-direction:column;justify-content:center;height:100%}.modal-dialog-centered.modal-dialog-scrollable .modal-content{max-height:none}.modal-dialog-centered.modal-dialog-scrollable::before{content:none}.modal-content{position:relative;display:flex;flex-direction:column;width:100%;pointer-events:auto;background-color:#fff;background-clip:padding-box;border:1px solid rgba(0,0,0,.2);border-radius:.3rem;outline:0}.modal-backdrop{position:fixed;top:0;left:0;z-index:1040;width:100vw;height:100vh;background-color:#000}.modal-backdrop.fade{opacity:0}.modal-backdrop.show{opacity:.5}.modal-header{display:flex;align-items:flex-start;justify-content:space-between;padding:1rem 1rem;border-bottom:1px solid #dee2e6;border-top-left-radius:calc(0.3rem - 1px);border-top-right-radius:calc(0.3rem - 1px)}.modal-header .close{padding:1rem 1rem;margin:-1rem -1rem -1rem auto}.modal-title{margin-bottom:0;line-height:1.5}.modal-body{position:relative;flex:1 1 auto;padding:1rem}.modal-footer{display:flex;flex-wrap:wrap;align-items:center;justify-content:flex-end;padding:.75rem;border-top:1px solid #dee2e6;border-bottom-right-radius:calc(0.3rem - 1px);border-bottom-left-radius:calc(0.3rem - 1px)}.modal-footer>*{margin:.25rem}.modal-scrollbar-measure{position:absolute;top:-9999px;width:50px;height:50px;overflow:scroll}@media(min-width: 576px){.modal-dialog{max-width:500px;margin:1.75rem auto}.modal-dialog-scrollable{max-height:calc(100% - 3.5rem)}.modal-dialog-scrollable .modal-content{max-height:calc(100vh - 3.5rem)}.modal-dialog-centered{min-height:calc(100% - 3.5rem)}.modal-dialog-centered::before{height:calc(100vh - 3.5rem);height:min-content}.modal-sm{max-width:300px}}@media(min-width: 992px){.modal-lg,.modal-xl{max-width:800px}}@media(min-width: 1200px){.modal-xl{max-width:1140px}}.tooltip{position:absolute;z-index:1070;display:block;margin:0;font-family:-apple-system,BlinkMacSystemFont,\"Segoe UI\",Roboto,\"Helvetica Neue\",Arial,sans-serif,\"Apple Color Emoji\",\"Segoe UI Emoji\",\"Segoe UI Symbol\",\"Noto Color Emoji\";font-style:normal;font-weight:400;line-height:1.5;text-align:left;text-align:start;text-decoration:none;text-shadow:none;text-transform:none;letter-spacing:normal;word-break:normal;word-spacing:normal;white-space:normal;line-break:auto;font-size:0.875rem;word-wrap:break-word;opacity:0}.tooltip.show{opacity:.9}.tooltip .arrow{position:absolute;display:block;width:.8rem;height:.4rem}.tooltip .arrow::before{position:absolute;content:\"\";border-color:transparent;border-style:solid}.bs-tooltip-top,.bs-tooltip-auto[x-placement^=top]{padding:.4rem 0}.bs-tooltip-top .arrow,.bs-tooltip-auto[x-placement^=top] .arrow{bottom:0}.bs-tooltip-top .arrow::before,.bs-tooltip-auto[x-placement^=top] .arrow::before{top:0;border-width:.4rem .4rem 0;border-top-color:#000}.bs-tooltip-right,.bs-tooltip-auto[x-placement^=right]{padding:0 .4rem}.bs-tooltip-right .arrow,.bs-tooltip-auto[x-placement^=right] .arrow{left:0;width:.4rem;height:.8rem}.bs-tooltip-right .arrow::before,.bs-tooltip-auto[x-placement^=right] .arrow::before{right:0;border-width:.4rem .4rem .4rem 0;border-right-color:#000}.bs-tooltip-bottom,.bs-tooltip-auto[x-placement^=bottom]{padding:.4rem 0}.bs-tooltip-bottom .arrow,.bs-tooltip-auto[x-placement^=bottom] .arrow{top:0}.bs-tooltip-bottom .arrow::before,.bs-tooltip-auto[x-placement^=bottom] .arrow::before{bottom:0;border-width:0 .4rem .4rem;border-bottom-color:#000}.bs-tooltip-left,.bs-tooltip-auto[x-placement^=left]{padding:0 .4rem}.bs-tooltip-left .arrow,.bs-tooltip-auto[x-placement^=left] .arrow{right:0;width:.4rem;height:.8rem}.bs-tooltip-left .arrow::before,.bs-tooltip-auto[x-placement^=left] .arrow::before{left:0;border-width:.4rem 0 .4rem .4rem;border-left-color:#000}.tooltip-inner{max-width:200px;padding:.25rem .5rem;color:#fff;text-align:center;background-color:#000;border-radius:.25rem}.popover{position:absolute;top:0;left:0;z-index:1060;display:block;max-width:276px;font-family:-apple-system,BlinkMacSystemFont,\"Segoe UI\",Roboto,\"Helvetica Neue\",Arial,sans-serif,\"Apple Color Emoji\",\"Segoe UI Emoji\",\"Segoe UI Symbol\",\"Noto Color Emoji\";font-style:normal;font-weight:400;line-height:1.5;text-align:left;text-align:start;text-decoration:none;text-shadow:none;text-transform:none;letter-spacing:normal;word-break:normal;word-spacing:normal;white-space:normal;line-break:auto;font-size:0.875rem;word-wrap:break-word;background-color:#fff;background-clip:padding-box;border:1px solid rgba(0,0,0,.2);border-radius:.3rem}.popover .arrow{position:absolute;display:block;width:1rem;height:.5rem;margin:0 .3rem}.popover .arrow::before,.popover .arrow::after{position:absolute;display:block;content:\"\";border-color:transparent;border-style:solid}.bs-popover-top,.bs-popover-auto[x-placement^=top]{margin-bottom:.5rem}.bs-popover-top>.arrow,.bs-popover-auto[x-placement^=top]>.arrow{bottom:calc(-0.5rem - 1px)}.bs-popover-top>.arrow::before,.bs-popover-auto[x-placement^=top]>.arrow::before{bottom:0;border-width:.5rem .5rem 0;border-top-color:rgba(0,0,0,.25)}.bs-popover-top>.arrow::after,.bs-popover-auto[x-placement^=top]>.arrow::after{bottom:1px;border-width:.5rem .5rem 0;border-top-color:#fff}.bs-popover-right,.bs-popover-auto[x-placement^=right]{margin-left:.5rem}.bs-popover-right>.arrow,.bs-popover-auto[x-placement^=right]>.arrow{left:calc(-0.5rem - 1px);width:.5rem;height:1rem;margin:.3rem 0}.bs-popover-right>.arrow::before,.bs-popover-auto[x-placement^=right]>.arrow::before{left:0;border-width:.5rem .5rem .5rem 0;border-right-color:rgba(0,0,0,.25)}.bs-popover-right>.arrow::after,.bs-popover-auto[x-placement^=right]>.arrow::after{left:1px;border-width:.5rem .5rem .5rem 0;border-right-color:#fff}.bs-popover-bottom,.bs-popover-auto[x-placement^=bottom]{margin-top:.5rem}.bs-popover-bottom>.arrow,.bs-popover-auto[x-placement^=bottom]>.arrow{top:calc(-0.5rem - 1px)}.bs-popover-bottom>.arrow::before,.bs-popover-auto[x-placement^=bottom]>.arrow::before{top:0;border-width:0 .5rem .5rem .5rem;border-bottom-color:rgba(0,0,0,.25)}.bs-popover-bottom>.arrow::after,.bs-popover-auto[x-placement^=bottom]>.arrow::after{top:1px;border-width:0 .5rem .5rem .5rem;border-bottom-color:#fff}.bs-popover-bottom .popover-header::before,.bs-popover-auto[x-placement^=bottom] .popover-header::before{position:absolute;top:0;left:50%;display:block;width:1rem;margin-left:-0.5rem;content:\"\";border-bottom:1px solid #f7f7f7}.bs-popover-left,.bs-popover-auto[x-placement^=left]{margin-right:.5rem}.bs-popover-left>.arrow,.bs-popover-auto[x-placement^=left]>.arrow{right:calc(-0.5rem - 1px);width:.5rem;height:1rem;margin:.3rem 0}.bs-popover-left>.arrow::before,.bs-popover-auto[x-placement^=left]>.arrow::before{right:0;border-width:.5rem 0 .5rem .5rem;border-left-color:rgba(0,0,0,.25)}.bs-popover-left>.arrow::after,.bs-popover-auto[x-placement^=left]>.arrow::after{right:1px;border-width:.5rem 0 .5rem .5rem;border-left-color:#fff}.popover-header{padding:.5rem .75rem;margin-bottom:0;font-size:1rem;background-color:#f7f7f7;border-bottom:1px solid #ebebeb;border-top-left-radius:calc(0.3rem - 1px);border-top-right-radius:calc(0.3rem - 1px)}.popover-header:empty{display:none}.popover-body{padding:.5rem .75rem;color:#212529}.carousel{position:relative}.carousel.pointer-event{touch-action:pan-y}.carousel-inner{position:relative;width:100%;overflow:hidden}.carousel-inner::after{display:block;clear:both;content:\"\"}.carousel-item{position:relative;display:none;float:left;width:100%;margin-right:-100%;backface-visibility:hidden;transition:transform .6s ease-in-out}@media(prefers-reduced-motion: reduce){.carousel-item{transition:none}}.carousel-item.active,.carousel-item-next,.carousel-item-prev{display:block}.carousel-item-next:not(.carousel-item-left),.active.carousel-item-right{transform:translateX(100%)}.carousel-item-prev:not(.carousel-item-right),.active.carousel-item-left{transform:translateX(-100%)}.carousel-fade .carousel-item{opacity:0;transition-property:opacity;transform:none}.carousel-fade .carousel-item.active,.carousel-fade .carousel-item-next.carousel-item-left,.carousel-fade .carousel-item-prev.carousel-item-right{z-index:1;opacity:1}.carousel-fade .active.carousel-item-left,.carousel-fade .active.carousel-item-right{z-index:0;opacity:0;transition:opacity 0s .6s}@media(prefers-reduced-motion: reduce){.carousel-fade .active.carousel-item-left,.carousel-fade .active.carousel-item-right{transition:none}}.carousel-control-prev,.carousel-control-next{position:absolute;top:0;bottom:0;z-index:1;display:flex;align-items:center;justify-content:center;width:15%;color:#fff;text-align:center;opacity:.5;transition:opacity .15s ease}@media(prefers-reduced-motion: reduce){.carousel-control-prev,.carousel-control-next{transition:none}}.carousel-control-prev:hover,.carousel-control-prev:focus,.carousel-control-next:hover,.carousel-control-next:focus{color:#fff;text-decoration:none;outline:0;opacity:.9}.carousel-control-prev{left:0}.carousel-control-next{right:0}.carousel-control-prev-icon,.carousel-control-next-icon{display:inline-block;width:20px;height:20px;background:50%/100% 100% no-repeat}.carousel-control-prev-icon{background-image:url(\"data:image/svg+xml,%3csvg xmlns='http://www.w3.org/2000/svg' fill='%23fff' width='8' height='8' viewBox='0 0 8 8'%3e%3cpath d='M5.25 0l-4 4 4 4 1.5-1.5L4.25 4l2.5-2.5L5.25 0z'/%3e%3c/svg%3e\")}.carousel-control-next-icon{background-image:url(\"data:image/svg+xml,%3csvg xmlns='http://www.w3.org/2000/svg' fill='%23fff' width='8' height='8' viewBox='0 0 8 8'%3e%3cpath d='M2.75 0l-1.5 1.5L3.75 4l-2.5 2.5L2.75 8l4-4-4-4z'/%3e%3c/svg%3e\")}.carousel-indicators{position:absolute;right:0;bottom:0;left:0;z-index:15;display:flex;justify-content:center;padding-left:0;margin-right:15%;margin-left:15%;list-style:none}.carousel-indicators li{box-sizing:content-box;flex:0 1 auto;width:30px;height:3px;margin-right:3px;margin-left:3px;text-indent:-999px;cursor:pointer;background-color:#fff;background-clip:padding-box;border-top:10px solid transparent;border-bottom:10px solid transparent;opacity:.5;transition:opacity .6s ease}@media(prefers-reduced-motion: reduce){.carousel-indicators li{transition:none}}.carousel-indicators .active{opacity:1}.carousel-caption{position:absolute;right:15%;bottom:20px;left:15%;z-index:10;padding-top:20px;padding-bottom:20px;color:#fff;text-align:center}@keyframes spinner-border{to{transform:rotate(360deg)}}.spinner-border{display:inline-block;width:2rem;height:2rem;vertical-align:text-bottom;border:.25em solid currentColor;border-right-color:transparent;border-radius:50%;animation:.75s linear infinite spinner-border}.spinner-border-sm{width:1rem;height:1rem;border-width:.2em}@keyframes spinner-grow{0%{transform:scale(0)}50%{opacity:1;transform:none}}.spinner-grow{display:inline-block;width:2rem;height:2rem;vertical-align:text-bottom;background-color:currentColor;border-radius:50%;opacity:0;animation:.75s linear infinite spinner-grow}.spinner-grow-sm{width:1rem;height:1rem}@media(prefers-reduced-motion: reduce){.spinner-border,.spinner-grow{animation-duration:1.5s}}.align-baseline{vertical-align:baseline !important}.align-top{vertical-align:top !important}.align-middle{vertical-align:middle !important}.align-bottom{vertical-align:bottom !important}.align-text-bottom{vertical-align:text-bottom !important}.align-text-top{vertical-align:text-top !important}.bg-primary{background-color:#007bff !important}a.bg-primary:hover,a.bg-primary:focus,button.bg-primary:hover,button.bg-primary:focus{background-color:#0062cc !important}.bg-secondary{background-color:#6c757d !important}a.bg-secondary:hover,a.bg-secondary:focus,button.bg-secondary:hover,button.bg-secondary:focus{background-color:#545b62 !important}.bg-success{background-color:#28a745 !important}a.bg-success:hover,a.bg-success:focus,button.bg-success:hover,button.bg-success:focus{background-color:#1e7e34 !important}.bg-info{background-color:#17a2b8 !important}a.bg-info:hover,a.bg-info:focus,button.bg-info:hover,button.bg-info:focus{background-color:#117a8b !important}.bg-warning{background-color:#ffc107 !important}a.bg-warning:hover,a.bg-warning:focus,button.bg-warning:hover,button.bg-warning:focus{background-color:#d39e00 !important}.bg-danger{background-color:#dc3545 !important}a.bg-danger:hover,a.bg-danger:focus,button.bg-danger:hover,button.bg-danger:focus{background-color:#bd2130 !important}.bg-light{background-color:#f8f9fa !important}a.bg-light:hover,a.bg-light:focus,button.bg-light:hover,button.bg-light:focus{background-color:#dae0e5 !important}.bg-dark{background-color:#343a40 !important}a.bg-dark:hover,a.bg-dark:focus,button.bg-dark:hover,button.bg-dark:focus{background-color:#1d2124 !important}.bg-white{background-color:#fff !important}.bg-transparent{background-color:transparent !important}.border{border:1px solid #dee2e6 !important}.border-top{border-top:1px solid #dee2e6 !important}.border-right{border-right:1px solid #dee2e6 !important}.border-bottom{border-bottom:1px solid #dee2e6 !important}.border-left{border-left:1px solid #dee2e6 !important}.border-0{border:0 !important}.border-top-0{border-top:0 !important}.border-right-0{border-right:0 !important}.border-bottom-0{border-bottom:0 !important}.border-left-0{border-left:0 !important}.border-primary{border-color:#007bff !important}.border-secondary{border-color:#6c757d !important}.border-success{border-color:#28a745 !important}.border-info{border-color:#17a2b8 !important}.border-warning{border-color:#ffc107 !important}.border-danger{border-color:#dc3545 !important}.border-light{border-color:#f8f9fa !important}.border-dark{border-color:#343a40 !important}.border-white{border-color:#fff !important}.rounded-sm{border-radius:.2rem !important}.rounded{border-radius:.25rem !important}.rounded-top{border-top-left-radius:.25rem !important;border-top-right-radius:.25rem !important}.rounded-right{border-top-right-radius:.25rem !important;border-bottom-right-radius:.25rem !important}.rounded-bottom{border-bottom-right-radius:.25rem !important;border-bottom-left-radius:.25rem !important}.rounded-left{border-top-left-radius:.25rem !important;border-bottom-left-radius:.25rem !important}.rounded-lg{border-radius:.3rem !important}.rounded-circle{border-radius:50% !important}.rounded-pill{border-radius:50rem !important}.rounded-0{border-radius:0 !important}.clearfix::after{display:block;clear:both;content:\"\"}.d-none{display:none !important}.d-inline{display:inline !important}.d-inline-block{display:inline-block !important}.d-block{display:block !important}.d-table{display:table !important}.d-table-row{display:table-row !important}.d-table-cell{display:table-cell !important}.d-flex{display:flex !important}.d-inline-flex{display:inline-flex !important}@media(min-width: 576px){.d-sm-none{display:none !important}.d-sm-inline{display:inline !important}.d-sm-inline-block{display:inline-block !important}.d-sm-block{display:block !important}.d-sm-table{display:table !important}.d-sm-table-row{display:table-row !important}.d-sm-table-cell{display:table-cell !important}.d-sm-flex{display:flex !important}.d-sm-inline-flex{display:inline-flex !important}}@media(min-width: 768px){.d-md-none{display:none !important}.d-md-inline{display:inline !important}.d-md-inline-block{display:inline-block !important}.d-md-block{display:block !important}.d-md-table{display:table !important}.d-md-table-row{display:table-row !important}.d-md-table-cell{display:table-cell !important}.d-md-flex{display:flex !important}.d-md-inline-flex{display:inline-flex !important}}@media(min-width: 992px){.d-lg-none{display:none !important}.d-lg-inline{display:inline !important}.d-lg-inline-block{display:inline-block !important}.d-lg-block{display:block !important}.d-lg-table{display:table !important}.d-lg-table-row{display:table-row !important}.d-lg-table-cell{display:table-cell !important}.d-lg-flex{display:flex !important}.d-lg-inline-flex{display:inline-flex !important}}@media(min-width: 1200px){.d-xl-none{display:none !important}.d-xl-inline{display:inline !important}.d-xl-inline-block{display:inline-block !important}.d-xl-block{display:block !important}.d-xl-table{display:table !important}.d-xl-table-row{display:table-row !important}.d-xl-table-cell{display:table-cell !important}.d-xl-flex{display:flex !important}.d-xl-inline-flex{display:inline-flex !important}}@media print{.d-print-none{display:none !important}.d-print-inline{display:inline !important}.d-print-inline-block{display:inline-block !important}.d-print-block{display:block !important}.d-print-table{display:table !important}.d-print-table-row{display:table-row !important}.d-print-table-cell{display:table-cell !important}.d-print-flex{display:flex !important}.d-print-inline-flex{display:inline-flex !important}}.embed-responsive{position:relative;display:block;width:100%;padding:0;overflow:hidden}.embed-responsive::before{display:block;content:\"\"}.embed-responsive .embed-responsive-item,.embed-responsive iframe,.embed-responsive embed,.embed-responsive object,.embed-responsive video{position:absolute;top:0;bottom:0;left:0;width:100%;height:100%;border:0}.embed-responsive-21by9::before{padding-top:42.8571428571%}.embed-responsive-16by9::before{padding-top:56.25%}.embed-responsive-4by3::before{padding-top:75%}.embed-responsive-1by1::before{padding-top:100%}.flex-row{flex-direction:row !important}.flex-column{flex-direction:column !important}.flex-row-reverse{flex-direction:row-reverse !important}.flex-column-reverse{flex-direction:column-reverse !important}.flex-wrap{flex-wrap:wrap !important}.flex-nowrap{flex-wrap:nowrap !important}.flex-wrap-reverse{flex-wrap:wrap-reverse !important}.flex-fill{flex:1 1 auto !important}.flex-grow-0{flex-grow:0 !important}.flex-grow-1{flex-grow:1 !important}.flex-shrink-0{flex-shrink:0 !important}.flex-shrink-1{flex-shrink:1 !important}.justify-content-start{justify-content:flex-start !important}.justify-content-end{justify-content:flex-end !important}.justify-content-center{justify-content:center !important}.justify-content-between{justify-content:space-between !important}.justify-content-around{justify-content:space-around !important}.align-items-start{align-items:flex-start !important}.align-items-end{align-items:flex-end !important}.align-items-center{align-items:center !important}.align-items-baseline{align-items:baseline !important}.align-items-stretch{align-items:stretch !important}.align-content-start{align-content:flex-start !important}.align-content-end{align-content:flex-end !important}.align-content-center{align-content:center !important}.align-content-between{align-content:space-between !important}.align-content-around{align-content:space-around !important}.align-content-stretch{align-content:stretch !important}.align-self-auto{align-self:auto !important}.align-self-start{align-self:flex-start !important}.align-self-end{align-self:flex-end !important}.align-self-center{align-self:center !important}.align-self-baseline{align-self:baseline !important}.align-self-stretch{align-self:stretch !important}@media(min-width: 576px){.flex-sm-row{flex-direction:row !important}.flex-sm-column{flex-direction:column !important}.flex-sm-row-reverse{flex-direction:row-reverse !important}.flex-sm-column-reverse{flex-direction:column-reverse !important}.flex-sm-wrap{flex-wrap:wrap !important}.flex-sm-nowrap{flex-wrap:nowrap !important}.flex-sm-wrap-reverse{flex-wrap:wrap-reverse !important}.flex-sm-fill{flex:1 1 auto !important}.flex-sm-grow-0{flex-grow:0 !important}.flex-sm-grow-1{flex-grow:1 !important}.flex-sm-shrink-0{flex-shrink:0 !important}.flex-sm-shrink-1{flex-shrink:1 !important}.justify-content-sm-start{justify-content:flex-start !important}.justify-content-sm-end{justify-content:flex-end !important}.justify-content-sm-center{justify-content:center !important}.justify-content-sm-between{justify-content:space-between !important}.justify-content-sm-around{justify-content:space-around !important}.align-items-sm-start{align-items:flex-start !important}.align-items-sm-end{align-items:flex-end !important}.align-items-sm-center{align-items:center !important}.align-items-sm-baseline{align-items:baseline !important}.align-items-sm-stretch{align-items:stretch !important}.align-content-sm-start{align-content:flex-start !important}.align-content-sm-end{align-content:flex-end !important}.align-content-sm-center{align-content:center !important}.align-content-sm-between{align-content:space-between !important}.align-content-sm-around{align-content:space-around !important}.align-content-sm-stretch{align-content:stretch !important}.align-self-sm-auto{align-self:auto !important}.align-self-sm-start{align-self:flex-start !important}.align-self-sm-end{align-self:flex-end !important}.align-self-sm-center{align-self:center !important}.align-self-sm-baseline{align-self:baseline !important}.align-self-sm-stretch{align-self:stretch !important}}@media(min-width: 768px){.flex-md-row{flex-direction:row !important}.flex-md-column{flex-direction:column !important}.flex-md-row-reverse{flex-direction:row-reverse !important}.flex-md-column-reverse{flex-direction:column-reverse !important}.flex-md-wrap{flex-wrap:wrap !important}.flex-md-nowrap{flex-wrap:nowrap !important}.flex-md-wrap-reverse{flex-wrap:wrap-reverse !important}.flex-md-fill{flex:1 1 auto !important}.flex-md-grow-0{flex-grow:0 !important}.flex-md-grow-1{flex-grow:1 !important}.flex-md-shrink-0{flex-shrink:0 !important}.flex-md-shrink-1{flex-shrink:1 !important}.justify-content-md-start{justify-content:flex-start !important}.justify-content-md-end{justify-content:flex-end !important}.justify-content-md-center{justify-content:center !important}.justify-content-md-between{justify-content:space-between !important}.justify-content-md-around{justify-content:space-around !important}.align-items-md-start{align-items:flex-start !important}.align-items-md-end{align-items:flex-end !important}.align-items-md-center{align-items:center !important}.align-items-md-baseline{align-items:baseline !important}.align-items-md-stretch{align-items:stretch !important}.align-content-md-start{align-content:flex-start !important}.align-content-md-end{align-content:flex-end !important}.align-content-md-center{align-content:center !important}.align-content-md-between{align-content:space-between !important}.align-content-md-around{align-content:space-around !important}.align-content-md-stretch{align-content:stretch !important}.align-self-md-auto{align-self:auto !important}.align-self-md-start{align-self:flex-start !important}.align-self-md-end{align-self:flex-end !important}.align-self-md-center{align-self:center !important}.align-self-md-baseline{align-self:baseline !important}.align-self-md-stretch{align-self:stretch !important}}@media(min-width: 992px){.flex-lg-row{flex-direction:row !important}.flex-lg-column{flex-direction:column !important}.flex-lg-row-reverse{flex-direction:row-reverse !important}.flex-lg-column-reverse{flex-direction:column-reverse !important}.flex-lg-wrap{flex-wrap:wrap !important}.flex-lg-nowrap{flex-wrap:nowrap !important}.flex-lg-wrap-reverse{flex-wrap:wrap-reverse !important}.flex-lg-fill{flex:1 1 auto !important}.flex-lg-grow-0{flex-grow:0 !important}.flex-lg-grow-1{flex-grow:1 !important}.flex-lg-shrink-0{flex-shrink:0 !important}.flex-lg-shrink-1{flex-shrink:1 !important}.justify-content-lg-start{justify-content:flex-start !important}.justify-content-lg-end{justify-content:flex-end !important}.justify-content-lg-center{justify-content:center !important}.justify-content-lg-between{justify-content:space-between !important}.justify-content-lg-around{justify-content:space-around !important}.align-items-lg-start{align-items:flex-start !important}.align-items-lg-end{align-items:flex-end !important}.align-items-lg-center{align-items:center !important}.align-items-lg-baseline{align-items:baseline !important}.align-items-lg-stretch{align-items:stretch !important}.align-content-lg-start{align-content:flex-start !important}.align-content-lg-end{align-content:flex-end !important}.align-content-lg-center{align-content:center !important}.align-content-lg-between{align-content:space-between !important}.align-content-lg-around{align-content:space-around !important}.align-content-lg-stretch{align-content:stretch !important}.align-self-lg-auto{align-self:auto !important}.align-self-lg-start{align-self:flex-start !important}.align-self-lg-end{align-self:flex-end !important}.align-self-lg-center{align-self:center !important}.align-self-lg-baseline{align-self:baseline !important}.align-self-lg-stretch{align-self:stretch !important}}@media(min-width: 1200px){.flex-xl-row{flex-direction:row !important}.flex-xl-column{flex-direction:column !important}.flex-xl-row-reverse{flex-direction:row-reverse !important}.flex-xl-column-reverse{flex-direction:column-reverse !important}.flex-xl-wrap{flex-wrap:wrap !important}.flex-xl-nowrap{flex-wrap:nowrap !important}.flex-xl-wrap-reverse{flex-wrap:wrap-reverse !important}.flex-xl-fill{flex:1 1 auto !important}.flex-xl-grow-0{flex-grow:0 !important}.flex-xl-grow-1{flex-grow:1 !important}.flex-xl-shrink-0{flex-shrink:0 !important}.flex-xl-shrink-1{flex-shrink:1 !important}.justify-content-xl-start{justify-content:flex-start !important}.justify-content-xl-end{justify-content:flex-end !important}.justify-content-xl-center{justify-content:center !important}.justify-content-xl-between{justify-content:space-between !important}.justify-content-xl-around{justify-content:space-around !important}.align-items-xl-start{align-items:flex-start !important}.align-items-xl-end{align-items:flex-end !important}.align-items-xl-center{align-items:center !important}.align-items-xl-baseline{align-items:baseline !important}.align-items-xl-stretch{align-items:stretch !important}.align-content-xl-start{align-content:flex-start !important}.align-content-xl-end{align-content:flex-end !important}.align-content-xl-center{align-content:center !important}.align-content-xl-between{align-content:space-between !important}.align-content-xl-around{align-content:space-around !important}.align-content-xl-stretch{align-content:stretch !important}.align-self-xl-auto{align-self:auto !important}.align-self-xl-start{align-self:flex-start !important}.align-self-xl-end{align-self:flex-end !important}.align-self-xl-center{align-self:center !important}.align-self-xl-baseline{align-self:baseline !important}.align-self-xl-stretch{align-self:stretch !important}}.float-left{float:left !important}.float-right{float:right !important}.float-none{float:none !important}@media(min-width: 576px){.float-sm-left{float:left !important}.float-sm-right{float:right !important}.float-sm-none{float:none !important}}@media(min-width: 768px){.float-md-left{float:left !important}.float-md-right{float:right !important}.float-md-none{float:none !important}}@media(min-width: 992px){.float-lg-left{float:left !important}.float-lg-right{float:right !important}.float-lg-none{float:none !important}}@media(min-width: 1200px){.float-xl-left{float:left !important}.float-xl-right{float:right !important}.float-xl-none{float:none !important}}.user-select-all{user-select:all !important}.user-select-auto{user-select:auto !important}.user-select-none{user-select:none !important}.overflow-auto{overflow:auto !important}.overflow-hidden{overflow:hidden !important}.position-static{position:static !important}.position-relative{position:relative !important}.position-absolute{position:absolute !important}.position-fixed{position:fixed !important}.position-sticky{position:sticky !important}.fixed-top{position:fixed;top:0;right:0;left:0;z-index:1030}.fixed-bottom{position:fixed;right:0;bottom:0;left:0;z-index:1030}@supports(position: sticky){.sticky-top{position:sticky;top:0;z-index:1020}}.sr-only{position:absolute;width:1px;height:1px;padding:0;margin:-1px;overflow:hidden;clip:rect(0, 0, 0, 0);white-space:nowrap;border:0}.sr-only-focusable:active,.sr-only-focusable:focus{position:static;width:auto;height:auto;overflow:visible;clip:auto;white-space:normal}.shadow-sm{box-shadow:0 .125rem .25rem rgba(0,0,0,.075) !important}.shadow{box-shadow:0 .5rem 1rem rgba(0,0,0,.15) !important}.shadow-lg{box-shadow:0 1rem 3rem rgba(0,0,0,.175) !important}.shadow-none{box-shadow:none !important}.w-25{width:25% !important}.w-50{width:50% !important}.w-75{width:75% !important}.w-100{width:100% !important}.w-auto{width:auto !important}.h-25{height:25% !important}.h-50{height:50% !important}.h-75{height:75% !important}.h-100{height:100% !important}.h-auto{height:auto !important}.mw-100{max-width:100% !important}.mh-100{max-height:100% !important}.min-vw-100{min-width:100vw !important}.min-vh-100{min-height:100vh !important}.vw-100{width:100vw !important}.vh-100{height:100vh !important}.m-0{margin:0 !important}.mt-0,.my-0{margin-top:0 !important}.mr-0,.mx-0{margin-right:0 !important}.mb-0,.my-0{margin-bottom:0 !important}.ml-0,.mx-0{margin-left:0 !important}.m-1{margin:.25rem !important}.mt-1,.my-1{margin-top:.25rem !important}.mr-1,.mx-1{margin-right:.25rem !important}.mb-1,.my-1{margin-bottom:.25rem !important}.ml-1,.mx-1{margin-left:.25rem !important}.m-2{margin:.5rem !important}.mt-2,.my-2{margin-top:.5rem !important}.mr-2,.mx-2{margin-right:.5rem !important}.mb-2,.my-2{margin-bottom:.5rem !important}.ml-2,.mx-2{margin-left:.5rem !important}.m-3{margin:1rem !important}.mt-3,.my-3{margin-top:1rem !important}.mr-3,.mx-3{margin-right:1rem !important}.mb-3,.my-3{margin-bottom:1rem !important}.ml-3,.mx-3{margin-left:1rem !important}.m-4{margin:1.5rem !important}.mt-4,.my-4{margin-top:1.5rem !important}.mr-4,.mx-4{margin-right:1.5rem !important}.mb-4,.my-4{margin-bottom:1.5rem !important}.ml-4,.mx-4{margin-left:1.5rem !important}.m-5{margin:3rem !important}.mt-5,.my-5{margin-top:3rem !important}.mr-5,.mx-5{margin-right:3rem !important}.mb-5,.my-5{margin-bottom:3rem !important}.ml-5,.mx-5{margin-left:3rem !important}.p-0{padding:0 !important}.pt-0,.py-0{padding-top:0 !important}.pr-0,.px-0{padding-right:0 !important}.pb-0,.py-0{padding-bottom:0 !important}.pl-0,.px-0{padding-left:0 !important}.p-1{padding:.25rem !important}.pt-1,.py-1{padding-top:.25rem !important}.pr-1,.px-1{padding-right:.25rem !important}.pb-1,.py-1{padding-bottom:.25rem !important}.pl-1,.px-1{padding-left:.25rem !important}.p-2{padding:.5rem !important}.pt-2,.py-2{padding-top:.5rem !important}.pr-2,.px-2{padding-right:.5rem !important}.pb-2,.py-2{padding-bottom:.5rem !important}.pl-2,.px-2{padding-left:.5rem !important}.p-3{padding:1rem !important}.pt-3,.py-3{padding-top:1rem !important}.pr-3,.px-3{padding-right:1rem !important}.pb-3,.py-3{padding-bottom:1rem !important}.pl-3,.px-3{padding-left:1rem !important}.p-4{padding:1.5rem !important}.pt-4,.py-4{padding-top:1.5rem !important}.pr-4,.px-4{padding-right:1.5rem !important}.pb-4,.py-4{padding-bottom:1.5rem !important}.pl-4,.px-4{padding-left:1.5rem !important}.p-5{padding:3rem !important}.pt-5,.py-5{padding-top:3rem !important}.pr-5,.px-5{padding-right:3rem !important}.pb-5,.py-5{padding-bottom:3rem !important}.pl-5,.px-5{padding-left:3rem !important}.m-n1{margin:-0.25rem !important}.mt-n1,.my-n1{margin-top:-0.25rem !important}.mr-n1,.mx-n1{margin-right:-0.25rem !important}.mb-n1,.my-n1{margin-bottom:-0.25rem !important}.ml-n1,.mx-n1{margin-left:-0.25rem !important}.m-n2{margin:-0.5rem !important}.mt-n2,.my-n2{margin-top:-0.5rem !important}.mr-n2,.mx-n2{margin-right:-0.5rem !important}.mb-n2,.my-n2{margin-bottom:-0.5rem !important}.ml-n2,.mx-n2{margin-left:-0.5rem !important}.m-n3{margin:-1rem !important}.mt-n3,.my-n3{margin-top:-1rem !important}.mr-n3,.mx-n3{margin-right:-1rem !important}.mb-n3,.my-n3{margin-bottom:-1rem !important}.ml-n3,.mx-n3{margin-left:-1rem !important}.m-n4{margin:-1.5rem !important}.mt-n4,.my-n4{margin-top:-1.5rem !important}.mr-n4,.mx-n4{margin-right:-1.5rem !important}.mb-n4,.my-n4{margin-bottom:-1.5rem !important}.ml-n4,.mx-n4{margin-left:-1.5rem !important}.m-n5{margin:-3rem !important}.mt-n5,.my-n5{margin-top:-3rem !important}.mr-n5,.mx-n5{margin-right:-3rem !important}.mb-n5,.my-n5{margin-bottom:-3rem !important}.ml-n5,.mx-n5{margin-left:-3rem !important}.m-auto{margin:auto !important}.mt-auto,.my-auto{margin-top:auto !important}.mr-auto,.mx-auto{margin-right:auto !important}.mb-auto,.my-auto{margin-bottom:auto !important}.ml-auto,.mx-auto{margin-left:auto !important}@media(min-width: 576px){.m-sm-0{margin:0 !important}.mt-sm-0,.my-sm-0{margin-top:0 !important}.mr-sm-0,.mx-sm-0{margin-right:0 !important}.mb-sm-0,.my-sm-0{margin-bottom:0 !important}.ml-sm-0,.mx-sm-0{margin-left:0 !important}.m-sm-1{margin:.25rem !important}.mt-sm-1,.my-sm-1{margin-top:.25rem !important}.mr-sm-1,.mx-sm-1{margin-right:.25rem !important}.mb-sm-1,.my-sm-1{margin-bottom:.25rem !important}.ml-sm-1,.mx-sm-1{margin-left:.25rem !important}.m-sm-2{margin:.5rem !important}.mt-sm-2,.my-sm-2{margin-top:.5rem !important}.mr-sm-2,.mx-sm-2{margin-right:.5rem !important}.mb-sm-2,.my-sm-2{margin-bottom:.5rem !important}.ml-sm-2,.mx-sm-2{margin-left:.5rem !important}.m-sm-3{margin:1rem !important}.mt-sm-3,.my-sm-3{margin-top:1rem !important}.mr-sm-3,.mx-sm-3{margin-right:1rem !important}.mb-sm-3,.my-sm-3{margin-bottom:1rem !important}.ml-sm-3,.mx-sm-3{margin-left:1rem !important}.m-sm-4{margin:1.5rem !important}.mt-sm-4,.my-sm-4{margin-top:1.5rem !important}.mr-sm-4,.mx-sm-4{margin-right:1.5rem !important}.mb-sm-4,.my-sm-4{margin-bottom:1.5rem !important}.ml-sm-4,.mx-sm-4{margin-left:1.5rem !important}.m-sm-5{margin:3rem !important}.mt-sm-5,.my-sm-5{margin-top:3rem !important}.mr-sm-5,.mx-sm-5{margin-right:3rem !important}.mb-sm-5,.my-sm-5{margin-bottom:3rem !important}.ml-sm-5,.mx-sm-5{margin-left:3rem !important}.p-sm-0{padding:0 !important}.pt-sm-0,.py-sm-0{padding-top:0 !important}.pr-sm-0,.px-sm-0{padding-right:0 !important}.pb-sm-0,.py-sm-0{padding-bottom:0 !important}.pl-sm-0,.px-sm-0{padding-left:0 !important}.p-sm-1{padding:.25rem !important}.pt-sm-1,.py-sm-1{padding-top:.25rem !important}.pr-sm-1,.px-sm-1{padding-right:.25rem !important}.pb-sm-1,.py-sm-1{padding-bottom:.25rem !important}.pl-sm-1,.px-sm-1{padding-left:.25rem !important}.p-sm-2{padding:.5rem !important}.pt-sm-2,.py-sm-2{padding-top:.5rem !important}.pr-sm-2,.px-sm-2{padding-right:.5rem !important}.pb-sm-2,.py-sm-2{padding-bottom:.5rem !important}.pl-sm-2,.px-sm-2{padding-left:.5rem !important}.p-sm-3{padding:1rem !important}.pt-sm-3,.py-sm-3{padding-top:1rem !important}.pr-sm-3,.px-sm-3{padding-right:1rem !important}.pb-sm-3,.py-sm-3{padding-bottom:1rem !important}.pl-sm-3,.px-sm-3{padding-left:1rem !important}.p-sm-4{padding:1.5rem !important}.pt-sm-4,.py-sm-4{padding-top:1.5rem !important}.pr-sm-4,.px-sm-4{padding-right:1.5rem !important}.pb-sm-4,.py-sm-4{padding-bottom:1.5rem !important}.pl-sm-4,.px-sm-4{padding-left:1.5rem !important}.p-sm-5{padding:3rem !important}.pt-sm-5,.py-sm-5{padding-top:3rem !important}.pr-sm-5,.px-sm-5{padding-right:3rem !important}.pb-sm-5,.py-sm-5{padding-bottom:3rem !important}.pl-sm-5,.px-sm-5{padding-left:3rem !important}.m-sm-n1{margin:-0.25rem !important}.mt-sm-n1,.my-sm-n1{margin-top:-0.25rem !important}.mr-sm-n1,.mx-sm-n1{margin-right:-0.25rem !important}.mb-sm-n1,.my-sm-n1{margin-bottom:-0.25rem !important}.ml-sm-n1,.mx-sm-n1{margin-left:-0.25rem !important}.m-sm-n2{margin:-0.5rem !important}.mt-sm-n2,.my-sm-n2{margin-top:-0.5rem !important}.mr-sm-n2,.mx-sm-n2{margin-right:-0.5rem !important}.mb-sm-n2,.my-sm-n2{margin-bottom:-0.5rem !important}.ml-sm-n2,.mx-sm-n2{margin-left:-0.5rem !important}.m-sm-n3{margin:-1rem !important}.mt-sm-n3,.my-sm-n3{margin-top:-1rem !important}.mr-sm-n3,.mx-sm-n3{margin-right:-1rem !important}.mb-sm-n3,.my-sm-n3{margin-bottom:-1rem !important}.ml-sm-n3,.mx-sm-n3{margin-left:-1rem !important}.m-sm-n4{margin:-1.5rem !important}.mt-sm-n4,.my-sm-n4{margin-top:-1.5rem !important}.mr-sm-n4,.mx-sm-n4{margin-right:-1.5rem !important}.mb-sm-n4,.my-sm-n4{margin-bottom:-1.5rem !important}.ml-sm-n4,.mx-sm-n4{margin-left:-1.5rem !important}.m-sm-n5{margin:-3rem !important}.mt-sm-n5,.my-sm-n5{margin-top:-3rem !important}.mr-sm-n5,.mx-sm-n5{margin-right:-3rem !important}.mb-sm-n5,.my-sm-n5{margin-bottom:-3rem !important}.ml-sm-n5,.mx-sm-n5{margin-left:-3rem !important}.m-sm-auto{margin:auto !important}.mt-sm-auto,.my-sm-auto{margin-top:auto !important}.mr-sm-auto,.mx-sm-auto{margin-right:auto !important}.mb-sm-auto,.my-sm-auto{margin-bottom:auto !important}.ml-sm-auto,.mx-sm-auto{margin-left:auto !important}}@media(min-width: 768px){.m-md-0{margin:0 !important}.mt-md-0,.my-md-0{margin-top:0 !important}.mr-md-0,.mx-md-0{margin-right:0 !important}.mb-md-0,.my-md-0{margin-bottom:0 !important}.ml-md-0,.mx-md-0{margin-left:0 !important}.m-md-1{margin:.25rem !important}.mt-md-1,.my-md-1{margin-top:.25rem !important}.mr-md-1,.mx-md-1{margin-right:.25rem !important}.mb-md-1,.my-md-1{margin-bottom:.25rem !important}.ml-md-1,.mx-md-1{margin-left:.25rem !important}.m-md-2{margin:.5rem !important}.mt-md-2,.my-md-2{margin-top:.5rem !important}.mr-md-2,.mx-md-2{margin-right:.5rem !important}.mb-md-2,.my-md-2{margin-bottom:.5rem !important}.ml-md-2,.mx-md-2{margin-left:.5rem !important}.m-md-3{margin:1rem !important}.mt-md-3,.my-md-3{margin-top:1rem !important}.mr-md-3,.mx-md-3{margin-right:1rem !important}.mb-md-3,.my-md-3{margin-bottom:1rem !important}.ml-md-3,.mx-md-3{margin-left:1rem !important}.m-md-4{margin:1.5rem !important}.mt-md-4,.my-md-4{margin-top:1.5rem !important}.mr-md-4,.mx-md-4{margin-right:1.5rem !important}.mb-md-4,.my-md-4{margin-bottom:1.5rem !important}.ml-md-4,.mx-md-4{margin-left:1.5rem !important}.m-md-5{margin:3rem !important}.mt-md-5,.my-md-5{margin-top:3rem !important}.mr-md-5,.mx-md-5{margin-right:3rem !important}.mb-md-5,.my-md-5{margin-bottom:3rem !important}.ml-md-5,.mx-md-5{margin-left:3rem !important}.p-md-0{padding:0 !important}.pt-md-0,.py-md-0{padding-top:0 !important}.pr-md-0,.px-md-0{padding-right:0 !important}.pb-md-0,.py-md-0{padding-bottom:0 !important}.pl-md-0,.px-md-0{padding-left:0 !important}.p-md-1{padding:.25rem !important}.pt-md-1,.py-md-1{padding-top:.25rem !important}.pr-md-1,.px-md-1{padding-right:.25rem !important}.pb-md-1,.py-md-1{padding-bottom:.25rem !important}.pl-md-1,.px-md-1{padding-left:.25rem !important}.p-md-2{padding:.5rem !important}.pt-md-2,.py-md-2{padding-top:.5rem !important}.pr-md-2,.px-md-2{padding-right:.5rem !important}.pb-md-2,.py-md-2{padding-bottom:.5rem !important}.pl-md-2,.px-md-2{padding-left:.5rem !important}.p-md-3{padding:1rem !important}.pt-md-3,.py-md-3{padding-top:1rem !important}.pr-md-3,.px-md-3{padding-right:1rem !important}.pb-md-3,.py-md-3{padding-bottom:1rem !important}.pl-md-3,.px-md-3{padding-left:1rem !important}.p-md-4{padding:1.5rem !important}.pt-md-4,.py-md-4{padding-top:1.5rem !important}.pr-md-4,.px-md-4{padding-right:1.5rem !important}.pb-md-4,.py-md-4{padding-bottom:1.5rem !important}.pl-md-4,.px-md-4{padding-left:1.5rem !important}.p-md-5{padding:3rem !important}.pt-md-5,.py-md-5{padding-top:3rem !important}.pr-md-5,.px-md-5{padding-right:3rem !important}.pb-md-5,.py-md-5{padding-bottom:3rem !important}.pl-md-5,.px-md-5{padding-left:3rem !important}.m-md-n1{margin:-0.25rem !important}.mt-md-n1,.my-md-n1{margin-top:-0.25rem !important}.mr-md-n1,.mx-md-n1{margin-right:-0.25rem !important}.mb-md-n1,.my-md-n1{margin-bottom:-0.25rem !important}.ml-md-n1,.mx-md-n1{margin-left:-0.25rem !important}.m-md-n2{margin:-0.5rem !important}.mt-md-n2,.my-md-n2{margin-top:-0.5rem !important}.mr-md-n2,.mx-md-n2{margin-right:-0.5rem !important}.mb-md-n2,.my-md-n2{margin-bottom:-0.5rem !important}.ml-md-n2,.mx-md-n2{margin-left:-0.5rem !important}.m-md-n3{margin:-1rem !important}.mt-md-n3,.my-md-n3{margin-top:-1rem !important}.mr-md-n3,.mx-md-n3{margin-right:-1rem !important}.mb-md-n3,.my-md-n3{margin-bottom:-1rem !important}.ml-md-n3,.mx-md-n3{margin-left:-1rem !important}.m-md-n4{margin:-1.5rem !important}.mt-md-n4,.my-md-n4{margin-top:-1.5rem !important}.mr-md-n4,.mx-md-n4{margin-right:-1.5rem !important}.mb-md-n4,.my-md-n4{margin-bottom:-1.5rem !important}.ml-md-n4,.mx-md-n4{margin-left:-1.5rem !important}.m-md-n5{margin:-3rem !important}.mt-md-n5,.my-md-n5{margin-top:-3rem !important}.mr-md-n5,.mx-md-n5{margin-right:-3rem !important}.mb-md-n5,.my-md-n5{margin-bottom:-3rem !important}.ml-md-n5,.mx-md-n5{margin-left:-3rem !important}.m-md-auto{margin:auto !important}.mt-md-auto,.my-md-auto{margin-top:auto !important}.mr-md-auto,.mx-md-auto{margin-right:auto !important}.mb-md-auto,.my-md-auto{margin-bottom:auto !important}.ml-md-auto,.mx-md-auto{margin-left:auto !important}}@media(min-width: 992px){.m-lg-0{margin:0 !important}.mt-lg-0,.my-lg-0{margin-top:0 !important}.mr-lg-0,.mx-lg-0{margin-right:0 !important}.mb-lg-0,.my-lg-0{margin-bottom:0 !important}.ml-lg-0,.mx-lg-0{margin-left:0 !important}.m-lg-1{margin:.25rem !important}.mt-lg-1,.my-lg-1{margin-top:.25rem !important}.mr-lg-1,.mx-lg-1{margin-right:.25rem !important}.mb-lg-1,.my-lg-1{margin-bottom:.25rem !important}.ml-lg-1,.mx-lg-1{margin-left:.25rem !important}.m-lg-2{margin:.5rem !important}.mt-lg-2,.my-lg-2{margin-top:.5rem !important}.mr-lg-2,.mx-lg-2{margin-right:.5rem !important}.mb-lg-2,.my-lg-2{margin-bottom:.5rem !important}.ml-lg-2,.mx-lg-2{margin-left:.5rem !important}.m-lg-3{margin:1rem !important}.mt-lg-3,.my-lg-3{margin-top:1rem !important}.mr-lg-3,.mx-lg-3{margin-right:1rem !important}.mb-lg-3,.my-lg-3{margin-bottom:1rem !important}.ml-lg-3,.mx-lg-3{margin-left:1rem !important}.m-lg-4{margin:1.5rem !important}.mt-lg-4,.my-lg-4{margin-top:1.5rem !important}.mr-lg-4,.mx-lg-4{margin-right:1.5rem !important}.mb-lg-4,.my-lg-4{margin-bottom:1.5rem !important}.ml-lg-4,.mx-lg-4{margin-left:1.5rem !important}.m-lg-5{margin:3rem !important}.mt-lg-5,.my-lg-5{margin-top:3rem !important}.mr-lg-5,.mx-lg-5{margin-right:3rem !important}.mb-lg-5,.my-lg-5{margin-bottom:3rem !important}.ml-lg-5,.mx-lg-5{margin-left:3rem !important}.p-lg-0{padding:0 !important}.pt-lg-0,.py-lg-0{padding-top:0 !important}.pr-lg-0,.px-lg-0{padding-right:0 !important}.pb-lg-0,.py-lg-0{padding-bottom:0 !important}.pl-lg-0,.px-lg-0{padding-left:0 !important}.p-lg-1{padding:.25rem !important}.pt-lg-1,.py-lg-1{padding-top:.25rem !important}.pr-lg-1,.px-lg-1{padding-right:.25rem !important}.pb-lg-1,.py-lg-1{padding-bottom:.25rem !important}.pl-lg-1,.px-lg-1{padding-left:.25rem !important}.p-lg-2{padding:.5rem !important}.pt-lg-2,.py-lg-2{padding-top:.5rem !important}.pr-lg-2,.px-lg-2{padding-right:.5rem !important}.pb-lg-2,.py-lg-2{padding-bottom:.5rem !important}.pl-lg-2,.px-lg-2{padding-left:.5rem !important}.p-lg-3{padding:1rem !important}.pt-lg-3,.py-lg-3{padding-top:1rem !important}.pr-lg-3,.px-lg-3{padding-right:1rem !important}.pb-lg-3,.py-lg-3{padding-bottom:1rem !important}.pl-lg-3,.px-lg-3{padding-left:1rem !important}.p-lg-4{padding:1.5rem !important}.pt-lg-4,.py-lg-4{padding-top:1.5rem !important}.pr-lg-4,.px-lg-4{padding-right:1.5rem !important}.pb-lg-4,.py-lg-4{padding-bottom:1.5rem !important}.pl-lg-4,.px-lg-4{padding-left:1.5rem !important}.p-lg-5{padding:3rem !important}.pt-lg-5,.py-lg-5{padding-top:3rem !important}.pr-lg-5,.px-lg-5{padding-right:3rem !important}.pb-lg-5,.py-lg-5{padding-bottom:3rem !important}.pl-lg-5,.px-lg-5{padding-left:3rem !important}.m-lg-n1{margin:-0.25rem !important}.mt-lg-n1,.my-lg-n1{margin-top:-0.25rem !important}.mr-lg-n1,.mx-lg-n1{margin-right:-0.25rem !important}.mb-lg-n1,.my-lg-n1{margin-bottom:-0.25rem !important}.ml-lg-n1,.mx-lg-n1{margin-left:-0.25rem !important}.m-lg-n2{margin:-0.5rem !important}.mt-lg-n2,.my-lg-n2{margin-top:-0.5rem !important}.mr-lg-n2,.mx-lg-n2{margin-right:-0.5rem !important}.mb-lg-n2,.my-lg-n2{margin-bottom:-0.5rem !important}.ml-lg-n2,.mx-lg-n2{margin-left:-0.5rem !important}.m-lg-n3{margin:-1rem !important}.mt-lg-n3,.my-lg-n3{margin-top:-1rem !important}.mr-lg-n3,.mx-lg-n3{margin-right:-1rem !important}.mb-lg-n3,.my-lg-n3{margin-bottom:-1rem !important}.ml-lg-n3,.mx-lg-n3{margin-left:-1rem !important}.m-lg-n4{margin:-1.5rem !important}.mt-lg-n4,.my-lg-n4{margin-top:-1.5rem !important}.mr-lg-n4,.mx-lg-n4{margin-right:-1.5rem !important}.mb-lg-n4,.my-lg-n4{margin-bottom:-1.5rem !important}.ml-lg-n4,.mx-lg-n4{margin-left:-1.5rem !important}.m-lg-n5{margin:-3rem !important}.mt-lg-n5,.my-lg-n5{margin-top:-3rem !important}.mr-lg-n5,.mx-lg-n5{margin-right:-3rem !important}.mb-lg-n5,.my-lg-n5{margin-bottom:-3rem !important}.ml-lg-n5,.mx-lg-n5{margin-left:-3rem !important}.m-lg-auto{margin:auto !important}.mt-lg-auto,.my-lg-auto{margin-top:auto !important}.mr-lg-auto,.mx-lg-auto{margin-right:auto !important}.mb-lg-auto,.my-lg-auto{margin-bottom:auto !important}.ml-lg-auto,.mx-lg-auto{margin-left:auto !important}}@media(min-width: 1200px){.m-xl-0{margin:0 !important}.mt-xl-0,.my-xl-0{margin-top:0 !important}.mr-xl-0,.mx-xl-0{margin-right:0 !important}.mb-xl-0,.my-xl-0{margin-bottom:0 !important}.ml-xl-0,.mx-xl-0{margin-left:0 !important}.m-xl-1{margin:.25rem !important}.mt-xl-1,.my-xl-1{margin-top:.25rem !important}.mr-xl-1,.mx-xl-1{margin-right:.25rem !important}.mb-xl-1,.my-xl-1{margin-bottom:.25rem !important}.ml-xl-1,.mx-xl-1{margin-left:.25rem !important}.m-xl-2{margin:.5rem !important}.mt-xl-2,.my-xl-2{margin-top:.5rem !important}.mr-xl-2,.mx-xl-2{margin-right:.5rem !important}.mb-xl-2,.my-xl-2{margin-bottom:.5rem !important}.ml-xl-2,.mx-xl-2{margin-left:.5rem !important}.m-xl-3{margin:1rem !important}.mt-xl-3,.my-xl-3{margin-top:1rem !important}.mr-xl-3,.mx-xl-3{margin-right:1rem !important}.mb-xl-3,.my-xl-3{margin-bottom:1rem !important}.ml-xl-3,.mx-xl-3{margin-left:1rem !important}.m-xl-4{margin:1.5rem !important}.mt-xl-4,.my-xl-4{margin-top:1.5rem !important}.mr-xl-4,.mx-xl-4{margin-right:1.5rem !important}.mb-xl-4,.my-xl-4{margin-bottom:1.5rem !important}.ml-xl-4,.mx-xl-4{margin-left:1.5rem !important}.m-xl-5{margin:3rem !important}.mt-xl-5,.my-xl-5{margin-top:3rem !important}.mr-xl-5,.mx-xl-5{margin-right:3rem !important}.mb-xl-5,.my-xl-5{margin-bottom:3rem !important}.ml-xl-5,.mx-xl-5{margin-left:3rem !important}.p-xl-0{padding:0 !important}.pt-xl-0,.py-xl-0{padding-top:0 !important}.pr-xl-0,.px-xl-0{padding-right:0 !important}.pb-xl-0,.py-xl-0{padding-bottom:0 !important}.pl-xl-0,.px-xl-0{padding-left:0 !important}.p-xl-1{padding:.25rem !important}.pt-xl-1,.py-xl-1{padding-top:.25rem !important}.pr-xl-1,.px-xl-1{padding-right:.25rem !important}.pb-xl-1,.py-xl-1{padding-bottom:.25rem !important}.pl-xl-1,.px-xl-1{padding-left:.25rem !important}.p-xl-2{padding:.5rem !important}.pt-xl-2,.py-xl-2{padding-top:.5rem !important}.pr-xl-2,.px-xl-2{padding-right:.5rem !important}.pb-xl-2,.py-xl-2{padding-bottom:.5rem !important}.pl-xl-2,.px-xl-2{padding-left:.5rem !important}.p-xl-3{padding:1rem !important}.pt-xl-3,.py-xl-3{padding-top:1rem !important}.pr-xl-3,.px-xl-3{padding-right:1rem !important}.pb-xl-3,.py-xl-3{padding-bottom:1rem !important}.pl-xl-3,.px-xl-3{padding-left:1rem !important}.p-xl-4{padding:1.5rem !important}.pt-xl-4,.py-xl-4{padding-top:1.5rem !important}.pr-xl-4,.px-xl-4{padding-right:1.5rem !important}.pb-xl-4,.py-xl-4{padding-bottom:1.5rem !important}.pl-xl-4,.px-xl-4{padding-left:1.5rem !important}.p-xl-5{padding:3rem !important}.pt-xl-5,.py-xl-5{padding-top:3rem !important}.pr-xl-5,.px-xl-5{padding-right:3rem !important}.pb-xl-5,.py-xl-5{padding-bottom:3rem !important}.pl-xl-5,.px-xl-5{padding-left:3rem !important}.m-xl-n1{margin:-0.25rem !important}.mt-xl-n1,.my-xl-n1{margin-top:-0.25rem !important}.mr-xl-n1,.mx-xl-n1{margin-right:-0.25rem !important}.mb-xl-n1,.my-xl-n1{margin-bottom:-0.25rem !important}.ml-xl-n1,.mx-xl-n1{margin-left:-0.25rem !important}.m-xl-n2{margin:-0.5rem !important}.mt-xl-n2,.my-xl-n2{margin-top:-0.5rem !important}.mr-xl-n2,.mx-xl-n2{margin-right:-0.5rem !important}.mb-xl-n2,.my-xl-n2{margin-bottom:-0.5rem !important}.ml-xl-n2,.mx-xl-n2{margin-left:-0.5rem !important}.m-xl-n3{margin:-1rem !important}.mt-xl-n3,.my-xl-n3{margin-top:-1rem !important}.mr-xl-n3,.mx-xl-n3{margin-right:-1rem !important}.mb-xl-n3,.my-xl-n3{margin-bottom:-1rem !important}.ml-xl-n3,.mx-xl-n3{margin-left:-1rem !important}.m-xl-n4{margin:-1.5rem !important}.mt-xl-n4,.my-xl-n4{margin-top:-1.5rem !important}.mr-xl-n4,.mx-xl-n4{margin-right:-1.5rem !important}.mb-xl-n4,.my-xl-n4{margin-bottom:-1.5rem !important}.ml-xl-n4,.mx-xl-n4{margin-left:-1.5rem !important}.m-xl-n5{margin:-3rem !important}.mt-xl-n5,.my-xl-n5{margin-top:-3rem !important}.mr-xl-n5,.mx-xl-n5{margin-right:-3rem !important}.mb-xl-n5,.my-xl-n5{margin-bottom:-3rem !important}.ml-xl-n5,.mx-xl-n5{margin-left:-3rem !important}.m-xl-auto{margin:auto !important}.mt-xl-auto,.my-xl-auto{margin-top:auto !important}.mr-xl-auto,.mx-xl-auto{margin-right:auto !important}.mb-xl-auto,.my-xl-auto{margin-bottom:auto !important}.ml-xl-auto,.mx-xl-auto{margin-left:auto !important}}.stretched-link::after{position:absolute;top:0;right:0;bottom:0;left:0;z-index:1;pointer-events:auto;content:\"\";background-color:rgba(0,0,0,0)}.text-monospace{font-family:SFMono-Regular,Menlo,Monaco,Consolas,\"Liberation Mono\",\"Courier New\",monospace !important}.text-justify{text-align:justify !important}.text-wrap{white-space:normal !important}.text-nowrap{white-space:nowrap !important}.text-truncate{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.text-left{text-align:left !important}.text-right{text-align:right !important}.text-center{text-align:center !important}@media(min-width: 576px){.text-sm-left{text-align:left !important}.text-sm-right{text-align:right !important}.text-sm-center{text-align:center !important}}@media(min-width: 768px){.text-md-left{text-align:left !important}.text-md-right{text-align:right !important}.text-md-center{text-align:center !important}}@media(min-width: 992px){.text-lg-left{text-align:left !important}.text-lg-right{text-align:right !important}.text-lg-center{text-align:center !important}}@media(min-width: 1200px){.text-xl-left{text-align:left !important}.text-xl-right{text-align:right !important}.text-xl-center{text-align:center !important}}.text-lowercase{text-transform:lowercase !important}.text-uppercase{text-transform:uppercase !important}.text-capitalize{text-transform:capitalize !important}.font-weight-light{font-weight:300 !important}.font-weight-lighter{font-weight:lighter !important}.font-weight-normal{font-weight:400 !important}.font-weight-bold{font-weight:700 !important}.font-weight-bolder{font-weight:bolder !important}.font-italic{font-style:italic !important}.text-white{color:#fff !important}.text-primary{color:#007bff !important}a.text-primary:hover,a.text-primary:focus{color:#0056b3 !important}.text-secondary{color:#6c757d !important}a.text-secondary:hover,a.text-secondary:focus{color:#494f54 !important}.text-success{color:#28a745 !important}a.text-success:hover,a.text-success:focus{color:#19692c !important}.text-info{color:#17a2b8 !important}a.text-info:hover,a.text-info:focus{color:#0f6674 !important}.text-warning{color:#ffc107 !important}a.text-warning:hover,a.text-warning:focus{color:#ba8b00 !important}.text-danger{color:#dc3545 !important}a.text-danger:hover,a.text-danger:focus{color:#a71d2a !important}.text-light{color:#f8f9fa !important}a.text-light:hover,a.text-light:focus{color:#cbd3da !important}.text-dark{color:#343a40 !important}a.text-dark:hover,a.text-dark:focus{color:#121416 !important}.text-body{color:#212529 !important}.text-muted{color:#6c757d !important}.text-black-50{color:rgba(0,0,0,.5) !important}.text-white-50{color:rgba(255,255,255,.5) !important}.text-hide{font:0/0 a;color:transparent;text-shadow:none;background-color:transparent;border:0}.text-decoration-none{text-decoration:none !important}.text-break{word-break:break-word !important;word-wrap:break-word !important}.text-reset{color:inherit !important}.visible{visibility:visible !important}.invisible{visibility:hidden !important}@media print{*,*::before,*::after{text-shadow:none !important;box-shadow:none !important}a:not(.btn){text-decoration:underline}abbr[title]::after{content:\" (\" attr(title) \")\"}pre{white-space:pre-wrap !important}pre,blockquote{border:1px solid #adb5bd;page-break-inside:avoid}thead{display:table-header-group}tr,img{page-break-inside:avoid}p,h2,h3{orphans:3;widows:3}h2,h3{page-break-after:avoid}@page{size:a3}body{min-width:992px !important}.container{min-width:992px !important}.navbar{display:none}.badge{border:1px solid #000}.table{border-collapse:collapse !important}.table td,.table th{background-color:#fff !important}.table-bordered th,.table-bordered td{border:1px solid #dee2e6 !important}.table-dark{color:inherit}.table-dark th,.table-dark td,.table-dark thead th,.table-dark tbody+tbody{border-color:#dee2e6}.table .thead-dark th{color:inherit;border-color:#dee2e6}}.oxi-intro{margin:40px auto;max-width:720px}.oxi-title{font-size:400%;font-weight:300}.oxi-logo-big{text-align:center}.oxi-subtext{font-size:80%;margin-top:10px}.oxi-connected-title{font-weight:bold}.oxi-connected-validate{font-size:80%}.oxi-connected-meta{font-size:80%}.oxi-connected-description{margin-top:8px}.oxi-error{font-size:120%;text-align:center}.oxi-inline-loading{height:18px;width:18px;font-size:50%}.oxi-loading{text-align:center}.oxi-loading-info{margin-top:10px;margin-bottom:10px;font-size:140%;font-weight:300}.oxi-highlight{color:#6f42c1}.oxi-page-title{text-align:center;margin-bottom:20px}.oxi-center{text-align:center}.oxi-command-group-content>*{margin-top:8px;margin-bottom:8px}.oxi-command-group-name{font-size:160%;font-weight:bold;margin-top:8px;margin-bottom:8px}.oxi-command-group-commands{font-size:140%;font-weight:bold;margin:8px 0px}.oxi-command-group-actions{margin-bottom:10px}.oxi-command{margin:0px}.oxi-command>*{margin-top:8px;margin-bottom:8px}.oxi-command>*:last-child{margin-bottom:0}.oxi-command-name{font-size:140%;margin-top:0;margin-bottom:8px}.oxi-command-name>*{display:inline;margin:0;padding:0}.oxi-example{margin-bottom:8px}.oxi-example-name{font-size:100%;margin-top:12px;margin-bottom:12px}.oxi-example-name>*{display:inline;margin:0;padding:0}.oxi-example-content{border-left:4px solid #ff4500;font-size:80%;padding:8px;background-color:#efefef}.oxi-example-content>*{margin:0}.oxi-header-action{font-size:50%;margin-left:.5rem}.oxi-unstable{color:#ff4500;font-weight:300;font-size:80%;font-weight:bold}#index{width:100%;height:100%}body{background-color:rgba(0,0,0,0)}.center{text-align:center}#overlay{display:grid;grid-template-columns:auto;grid-template-rows:0px [top] auto [middle] 0px [bottom];margin:0;height:100%}#current-song{position:absolute;background-color:rgba(0,0,0,.25);display:grid;grid-template-columns:[left] 64px 10px [gutter] auto [end];align-items:center;padding:10px;min-width:800px;max-width:33%;color:#fff;font-family:Consolas,monospace;font-weight:bold;text-shadow:-1px -1px 0 #000,1px -1px 0 #000,-1px 1px 0 #000,1px 1px 0 #000;grid-row-end:top}#current-song .request{float:right}#current-song .request-by{margin-right:10px;font-size:.8em}#current-song .album{grid-column-start:left;width:64px;height:64px}#current-song .info{display:grid;height:100%;grid-template-rows:auto auto 16px;grid-column-start:gutter}#current-song .info .track{grid-row-start:1}#current-song .info .track-name{padding-left:1px;font-size:1.4em;line-height:.8em;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}#current-song .info .artist{grid-row-start:2}#current-song .info .progress{grid-row-start:3}#current-song .info .progress-bar{background-color:#888;transition:none}#current-song .info .progress .timer{position:absolute;padding-left:5px}#current-song .state{display:none;position:absolute;width:64px;height:64px;z-index:1000;color:coral;background-size:32px 32px;background-repeat:no-repeat;background-position:left 16px top 16px}#current-song .state-paused{display:block;background-color:rgba(255,255,255,.5)}.title-refresh{margin-left:.4em}.clickable{cursor:pointer}.afterstream-added-at{white-space:nowrap;display:block;font-size:.8em}.afterstream-datetime{margin-left:.4em}.right{float:right}.content{margin-top:1rem}.settings-countdown{display:inline-block;width:2em;text-align:center}.settings-text{margin:0;font-size:70%;overflow:hidden}.settings-boolean-icon{width:3em}.settings-filter{cursor:pointer}.settings-filter:hover{color:purple !important}.settings-group{font-weight:bold}.settings-key{width:300px;overflow:hidden}.settings-key-name{font-weight:bold;white-space:nowrap}.settings-key-doc{white-space:normal;font-size:80%}.settings-key-doc p{margin:0}.auth-boolean-icon{width:3em}.auth-scope-short{font-size:120%;font-weight:bold}.auth-scope-key{width:300px;overflow:hidden}.auth-scope-key-name{font-weight:bold;white-space:nowrap}.auth-scope-key-doc{white-space:normal;font-size:80%}.auth-scope-key-doc p{margin:0}.auth-role-name{text-align:center}.auth-group{font-size:150%;font-weight:bold;cursor:pointer}.auth-group-filter{font-size:50%;margin-left:.5em;display:none}.auth-group:hover{color:purple}.auth-group:hover .auth-group-filter{display:inline}.command-name{font-family:monospace}.command-template{font-family:monospace}body.youtube-body{background-color:#000;color:#fff}.youtube-container iframe{position:absolute;top:0;left:0;width:100%;height:100%}.youtube-loading{text-align:center;font-size:200%}.youtube-not-loaded-obs{color:#fff;background-color:#333}.table-fill{width:100%}.button-fill{width:100%}.align-center{text-align:center}body.chat-body{background-color:#000;color:#fff}.chat-obs{color:#fff;background-color:#333;padding:.5em}.chat-settings{color:#000}#chat .edit-button{display:table;position:absolute;right:0;top:0;padding:2px .4em;height:36px}#chat .edit-button a{vertical-align:middle;display:table-cell;opacity:.5}#chat .edit-button a:hover{cursor:pointer;opacity:1}#chat .close-button{position:absolute;right:.2em;top:.1em;top:0}#chat .close-button a:hover{color:#ccc;cursor:pointer}#chat .chat-message-deleted{text-decoration:line-through}#chat .chat-warning{margin:1em}#chat .chat-messages{-webkit-transition:opacity .1s ease-in-out;-moz-transition:opacity .1s ease-in-out;-ms-transition:opacity .1s ease-in-out;-o-transition:opacity .1s ease-in-out;transition:opacity .1s ease-in-out}#chat .chat-messages.hidden{-webkit-transition:opacity 1s ease-in-out;-moz-transition:opacity 1s ease-in-out;-ms-transition:opacity 1s ease-in-out;-o-transition:opacity 1s ease-in-out;transition:opacity 1s ease-in-out}#chat .chat-no-messages{height:36px;line-height:32px;padding:0 2px;font-size:.8em;background-color:#333;text-align:center}#chat .chat-message{padding:2px .4em;overflow:hidden}#chat .chat-message:nth-child(even){background-color:#222}#chat .chat-message:nth-child(odd){background-color:#333}#chat .chat-timestamp{line-height:32px;vertical-align:middle;margin-right:.4em}#chat .chat-badges{display:inline-block;height:18px;vertical-align:middle}#chat .chat-badge{display:inline-block;margin-right:.2em;width:18px;height:18px;vertical-align:top}#chat .chat-badge.rounded{border-radius:20%}#chat .chat-badge>img{vertical-align:top;width:18px;height:18px}#chat .chat-badge:last-child{margin-right:.4em}#chat .chat-name{display:inline-block;vertical-align:middle;margin-right:.4em;font-weight:bold}#chat .chat-text{height:32px;word-wrap:break-word}#chat .chat-text span.text{line-height:32px;vertical-align:middle}#chat .chat-text a.url{color:#fff;line-height:32px;vertical-align:middle}#chat .chat-text img{height:28px;vertical-align:middle}.cache-action{cursor:pointer}.cache-namespace-header{font-size:120%;font-weight:bold}.cache-expires{white-space:nowrap}", ""]);
// Exports
/* harmony default export */ const __WEBPACK_DEFAULT_EXPORT__ = (___CSS_LOADER_EXPORT___);


/***/ }),

/***/ 46700:
/***/ ((module, __unused_webpack_exports, __webpack_require__) => {

var map = {
	"./af": 42786,
	"./af.js": 42786,
	"./ar": 30867,
	"./ar-dz": 14130,
	"./ar-dz.js": 14130,
	"./ar-kw": 96135,
	"./ar-kw.js": 96135,
	"./ar-ly": 56440,
	"./ar-ly.js": 56440,
	"./ar-ma": 47702,
	"./ar-ma.js": 47702,
	"./ar-sa": 16040,
	"./ar-sa.js": 16040,
	"./ar-tn": 37100,
	"./ar-tn.js": 37100,
	"./ar.js": 30867,
	"./az": 31083,
	"./az.js": 31083,
	"./be": 9808,
	"./be.js": 9808,
	"./bg": 68338,
	"./bg.js": 68338,
	"./bm": 67438,
	"./bm.js": 67438,
	"./bn": 8905,
	"./bn-bd": 76225,
	"./bn-bd.js": 76225,
	"./bn.js": 8905,
	"./bo": 11560,
	"./bo.js": 11560,
	"./br": 1278,
	"./br.js": 1278,
	"./bs": 80622,
	"./bs.js": 80622,
	"./ca": 2468,
	"./ca.js": 2468,
	"./cs": 5822,
	"./cs.js": 5822,
	"./cv": 50877,
	"./cv.js": 50877,
	"./cy": 47373,
	"./cy.js": 47373,
	"./da": 24780,
	"./da.js": 24780,
	"./de": 59740,
	"./de-at": 60217,
	"./de-at.js": 60217,
	"./de-ch": 60894,
	"./de-ch.js": 60894,
	"./de.js": 59740,
	"./dv": 5300,
	"./dv.js": 5300,
	"./el": 50837,
	"./el.js": 50837,
	"./en-au": 78348,
	"./en-au.js": 78348,
	"./en-ca": 77925,
	"./en-ca.js": 77925,
	"./en-gb": 22243,
	"./en-gb.js": 22243,
	"./en-ie": 46436,
	"./en-ie.js": 46436,
	"./en-il": 47207,
	"./en-il.js": 47207,
	"./en-in": 44175,
	"./en-in.js": 44175,
	"./en-nz": 76319,
	"./en-nz.js": 76319,
	"./en-sg": 31662,
	"./en-sg.js": 31662,
	"./eo": 92915,
	"./eo.js": 92915,
	"./es": 55655,
	"./es-do": 55251,
	"./es-do.js": 55251,
	"./es-mx": 96112,
	"./es-mx.js": 96112,
	"./es-us": 71146,
	"./es-us.js": 71146,
	"./es.js": 55655,
	"./et": 5603,
	"./et.js": 5603,
	"./eu": 77763,
	"./eu.js": 77763,
	"./fa": 76959,
	"./fa.js": 76959,
	"./fi": 11897,
	"./fi.js": 11897,
	"./fil": 42549,
	"./fil.js": 42549,
	"./fo": 94694,
	"./fo.js": 94694,
	"./fr": 94470,
	"./fr-ca": 63049,
	"./fr-ca.js": 63049,
	"./fr-ch": 52330,
	"./fr-ch.js": 52330,
	"./fr.js": 94470,
	"./fy": 5044,
	"./fy.js": 5044,
	"./ga": 29295,
	"./ga.js": 29295,
	"./gd": 2101,
	"./gd.js": 2101,
	"./gl": 38794,
	"./gl.js": 38794,
	"./gom-deva": 27884,
	"./gom-deva.js": 27884,
	"./gom-latn": 23168,
	"./gom-latn.js": 23168,
	"./gu": 95349,
	"./gu.js": 95349,
	"./he": 24206,
	"./he.js": 24206,
	"./hi": 30094,
	"./hi.js": 30094,
	"./hr": 30316,
	"./hr.js": 30316,
	"./hu": 22138,
	"./hu.js": 22138,
	"./hy-am": 11423,
	"./hy-am.js": 11423,
	"./id": 29218,
	"./id.js": 29218,
	"./is": 90135,
	"./is.js": 90135,
	"./it": 90626,
	"./it-ch": 10150,
	"./it-ch.js": 10150,
	"./it.js": 90626,
	"./ja": 39183,
	"./ja.js": 39183,
	"./jv": 24286,
	"./jv.js": 24286,
	"./ka": 12105,
	"./ka.js": 12105,
	"./kk": 47772,
	"./kk.js": 47772,
	"./km": 18758,
	"./km.js": 18758,
	"./kn": 79282,
	"./kn.js": 79282,
	"./ko": 33730,
	"./ko.js": 33730,
	"./ku": 1408,
	"./ku.js": 1408,
	"./ky": 33291,
	"./ky.js": 33291,
	"./lb": 36841,
	"./lb.js": 36841,
	"./lo": 55466,
	"./lo.js": 55466,
	"./lt": 57010,
	"./lt.js": 57010,
	"./lv": 37595,
	"./lv.js": 37595,
	"./me": 39861,
	"./me.js": 39861,
	"./mi": 35493,
	"./mi.js": 35493,
	"./mk": 95966,
	"./mk.js": 95966,
	"./ml": 87341,
	"./ml.js": 87341,
	"./mn": 5115,
	"./mn.js": 5115,
	"./mr": 10370,
	"./mr.js": 10370,
	"./ms": 9847,
	"./ms-my": 41237,
	"./ms-my.js": 41237,
	"./ms.js": 9847,
	"./mt": 72126,
	"./mt.js": 72126,
	"./my": 56165,
	"./my.js": 56165,
	"./nb": 64924,
	"./nb.js": 64924,
	"./ne": 16744,
	"./ne.js": 16744,
	"./nl": 93901,
	"./nl-be": 59814,
	"./nl-be.js": 59814,
	"./nl.js": 93901,
	"./nn": 83877,
	"./nn.js": 83877,
	"./oc-lnc": 92135,
	"./oc-lnc.js": 92135,
	"./pa-in": 15858,
	"./pa-in.js": 15858,
	"./pl": 64495,
	"./pl.js": 64495,
	"./pt": 89520,
	"./pt-br": 57971,
	"./pt-br.js": 57971,
	"./pt.js": 89520,
	"./ro": 96459,
	"./ro.js": 96459,
	"./ru": 21793,
	"./ru.js": 21793,
	"./sd": 40950,
	"./sd.js": 40950,
	"./se": 10490,
	"./se.js": 10490,
	"./si": 90124,
	"./si.js": 90124,
	"./sk": 64249,
	"./sk.js": 64249,
	"./sl": 14985,
	"./sl.js": 14985,
	"./sq": 51104,
	"./sq.js": 51104,
	"./sr": 49131,
	"./sr-cyrl": 79915,
	"./sr-cyrl.js": 79915,
	"./sr.js": 49131,
	"./ss": 85893,
	"./ss.js": 85893,
	"./sv": 98760,
	"./sv.js": 98760,
	"./sw": 91172,
	"./sw.js": 91172,
	"./ta": 27333,
	"./ta.js": 27333,
	"./te": 23110,
	"./te.js": 23110,
	"./tet": 52095,
	"./tet.js": 52095,
	"./tg": 27321,
	"./tg.js": 27321,
	"./th": 9041,
	"./th.js": 9041,
	"./tk": 19005,
	"./tk.js": 19005,
	"./tl-ph": 75768,
	"./tl-ph.js": 75768,
	"./tlh": 89444,
	"./tlh.js": 89444,
	"./tr": 72397,
	"./tr.js": 72397,
	"./tzl": 28254,
	"./tzl.js": 28254,
	"./tzm": 51106,
	"./tzm-latn": 30699,
	"./tzm-latn.js": 30699,
	"./tzm.js": 51106,
	"./ug-cn": 9288,
	"./ug-cn.js": 9288,
	"./uk": 67691,
	"./uk.js": 67691,
	"./ur": 13795,
	"./ur.js": 13795,
	"./uz": 6791,
	"./uz-latn": 60588,
	"./uz-latn.js": 60588,
	"./uz.js": 6791,
	"./vi": 65666,
	"./vi.js": 65666,
	"./x-pseudo": 14378,
	"./x-pseudo.js": 14378,
	"./yo": 75805,
	"./yo.js": 75805,
	"./zh-cn": 83839,
	"./zh-cn.js": 83839,
	"./zh-hk": 55726,
	"./zh-hk.js": 55726,
	"./zh-mo": 99807,
	"./zh-mo.js": 99807,
	"./zh-tw": 74152,
	"./zh-tw.js": 74152
};


function webpackContext(req) {
	var id = webpackContextResolve(req);
	return __webpack_require__(id);
}
function webpackContextResolve(req) {
	if(!__webpack_require__.o(map, req)) {
		var e = new Error("Cannot find module '" + req + "'");
		e.code = 'MODULE_NOT_FOUND';
		throw e;
	}
	return map[req];
}
webpackContext.keys = function webpackContextKeys() {
	return Object.keys(map);
};
webpackContext.resolve = webpackContextResolve;
module.exports = webpackContext;
webpackContext.id = 46700;

/***/ })

/******/ 	});
/************************************************************************/
/******/ 	// The module cache
/******/ 	var __webpack_module_cache__ = {};
/******/ 	
/******/ 	// The require function
/******/ 	function __webpack_require__(moduleId) {
/******/ 		// Check if module is in cache
/******/ 		if(__webpack_module_cache__[moduleId]) {
/******/ 			return __webpack_module_cache__[moduleId].exports;
/******/ 		}
/******/ 		// Create a new module (and put it into the cache)
/******/ 		var module = __webpack_module_cache__[moduleId] = {
/******/ 			id: moduleId,
/******/ 			loaded: false,
/******/ 			exports: {}
/******/ 		};
/******/ 	
/******/ 		// Execute the module function
/******/ 		__webpack_modules__[moduleId].call(module.exports, module, module.exports, __webpack_require__);
/******/ 	
/******/ 		// Flag the module as loaded
/******/ 		module.loaded = true;
/******/ 	
/******/ 		// Return the exports of the module
/******/ 		return module.exports;
/******/ 	}
/******/ 	
/******/ 	// expose the modules object (__webpack_modules__)
/******/ 	__webpack_require__.m = __webpack_modules__;
/******/ 	
/******/ 	// the startup function
/******/ 	// It's empty as some runtime module handles the default behavior
/******/ 	__webpack_require__.x = x => {}
/************************************************************************/
/******/ 	/* webpack/runtime/compat get default export */
/******/ 	(() => {
/******/ 		// getDefaultExport function for compatibility with non-harmony modules
/******/ 		__webpack_require__.n = (module) => {
/******/ 			var getter = module && module.__esModule ?
/******/ 				() => module['default'] :
/******/ 				() => module;
/******/ 			__webpack_require__.d(getter, { a: getter });
/******/ 			return getter;
/******/ 		};
/******/ 	})();
/******/ 	
/******/ 	/* webpack/runtime/define property getters */
/******/ 	(() => {
/******/ 		// define getter functions for harmony exports
/******/ 		__webpack_require__.d = (exports, definition) => {
/******/ 			for(var key in definition) {
/******/ 				if(__webpack_require__.o(definition, key) && !__webpack_require__.o(exports, key)) {
/******/ 					Object.defineProperty(exports, key, { enumerable: true, get: definition[key] });
/******/ 				}
/******/ 			}
/******/ 		};
/******/ 	})();
/******/ 	
/******/ 	/* webpack/runtime/global */
/******/ 	(() => {
/******/ 		__webpack_require__.g = (function() {
/******/ 			if (typeof globalThis === 'object') return globalThis;
/******/ 			try {
/******/ 				return this || new Function('return this')();
/******/ 			} catch (e) {
/******/ 				if (typeof window === 'object') return window;
/******/ 			}
/******/ 		})();
/******/ 	})();
/******/ 	
/******/ 	/* webpack/runtime/hasOwnProperty shorthand */
/******/ 	(() => {
/******/ 		__webpack_require__.o = (obj, prop) => Object.prototype.hasOwnProperty.call(obj, prop)
/******/ 	})();
/******/ 	
/******/ 	/* webpack/runtime/node module decorator */
/******/ 	(() => {
/******/ 		__webpack_require__.nmd = (module) => {
/******/ 			module.paths = [];
/******/ 			if (!module.children) module.children = [];
/******/ 			return module;
/******/ 		};
/******/ 	})();
/******/ 	
/******/ 	/* webpack/runtime/publicPath */
/******/ 	(() => {
/******/ 		var scriptUrl;
/******/ 		if (__webpack_require__.g.importScripts) scriptUrl = __webpack_require__.g.location + "";
/******/ 		var document = __webpack_require__.g.document;
/******/ 		if (!scriptUrl && document) {
/******/ 			if (document.currentScript)
/******/ 				scriptUrl = document.currentScript.src
/******/ 			if (!scriptUrl) {
/******/ 				var scripts = document.getElementsByTagName("script");
/******/ 				if(scripts.length) scriptUrl = scripts[scripts.length - 1].src
/******/ 			}
/******/ 		}
/******/ 		// When supporting browsers where an automatic publicPath is not supported you must specify an output.publicPath manually via configuration
/******/ 		// or pass an empty string ("") and set the __webpack_public_path__ variable from your code to use your own logic.
/******/ 		if (!scriptUrl) throw new Error("Automatic publicPath is not supported in this browser");
/******/ 		scriptUrl = scriptUrl.replace(/#.*$/, "").replace(/\?.*$/, "").replace(/\/[^\/]+$/, "/");
/******/ 		__webpack_require__.p = scriptUrl;
/******/ 	})();
/******/ 	
/******/ 	/* webpack/runtime/jsonp chunk loading */
/******/ 	(() => {
/******/ 		// no baseURI
/******/ 		
/******/ 		// object to store loaded and loading chunks
/******/ 		// undefined = chunk not loaded, null = chunk preloaded/prefetched
/******/ 		// Promise = chunk loading, 0 = chunk loaded
/******/ 		var installedChunks = {
/******/ 			179: 0
/******/ 		};
/******/ 		
/******/ 		var deferredModules = [
/******/ 			[65046,897]
/******/ 		];
/******/ 		// no chunk on demand loading
/******/ 		
/******/ 		// no prefetching
/******/ 		
/******/ 		// no preloaded
/******/ 		
/******/ 		// no HMR
/******/ 		
/******/ 		// no HMR manifest
/******/ 		
/******/ 		var checkDeferredModules = x => {};
/******/ 		
/******/ 		// install a JSONP callback for chunk loading
/******/ 		var webpackJsonpCallback = (parentChunkLoadingFunction, data) => {
/******/ 			var [chunkIds, moreModules, runtime, executeModules] = data;
/******/ 			// add "moreModules" to the modules object,
/******/ 			// then flag all "chunkIds" as loaded and fire callback
/******/ 			var moduleId, chunkId, i = 0, resolves = [];
/******/ 			for(;i < chunkIds.length; i++) {
/******/ 				chunkId = chunkIds[i];
/******/ 				if(__webpack_require__.o(installedChunks, chunkId) && installedChunks[chunkId]) {
/******/ 					resolves.push(installedChunks[chunkId][0]);
/******/ 				}
/******/ 				installedChunks[chunkId] = 0;
/******/ 			}
/******/ 			for(moduleId in moreModules) {
/******/ 				if(__webpack_require__.o(moreModules, moduleId)) {
/******/ 					__webpack_require__.m[moduleId] = moreModules[moduleId];
/******/ 				}
/******/ 			}
/******/ 			if(runtime) runtime(__webpack_require__);
/******/ 			if(parentChunkLoadingFunction) parentChunkLoadingFunction(data);
/******/ 			while(resolves.length) {
/******/ 				resolves.shift()();
/******/ 			}
/******/ 		
/******/ 			// add entry modules from loaded chunk to deferred list
/******/ 			if(executeModules) deferredModules.push.apply(deferredModules, executeModules);
/******/ 		
/******/ 			// run deferred modules when all chunks ready
/******/ 			return checkDeferredModules();
/******/ 		}
/******/ 		
/******/ 		var chunkLoadingGlobal = self["webpackChunkweb"] = self["webpackChunkweb"] || [];
/******/ 		chunkLoadingGlobal.forEach(webpackJsonpCallback.bind(null, 0));
/******/ 		chunkLoadingGlobal.push = webpackJsonpCallback.bind(null, chunkLoadingGlobal.push.bind(chunkLoadingGlobal));
/******/ 		
/******/ 		function checkDeferredModulesImpl() {
/******/ 			var result;
/******/ 			for(var i = 0; i < deferredModules.length; i++) {
/******/ 				var deferredModule = deferredModules[i];
/******/ 				var fulfilled = true;
/******/ 				for(var j = 1; j < deferredModule.length; j++) {
/******/ 					var depId = deferredModule[j];
/******/ 					if(installedChunks[depId] !== 0) fulfilled = false;
/******/ 				}
/******/ 				if(fulfilled) {
/******/ 					deferredModules.splice(i--, 1);
/******/ 					result = __webpack_require__(__webpack_require__.s = deferredModule[0]);
/******/ 				}
/******/ 			}
/******/ 			if(deferredModules.length === 0) {
/******/ 				__webpack_require__.x();
/******/ 				__webpack_require__.x = x => {};
/******/ 			}
/******/ 			return result;
/******/ 		}
/******/ 		var startup = __webpack_require__.x;
/******/ 		__webpack_require__.x = () => {
/******/ 			// reset startup function so it can be called again when more startup code is added
/******/ 			__webpack_require__.x = startup || (x => {});
/******/ 			return (checkDeferredModules = checkDeferredModulesImpl)();
/******/ 		};
/******/ 	})();
/******/ 	
/************************************************************************/
/******/ 	// run startup
/******/ 	return __webpack_require__.x();
/******/ })()
;