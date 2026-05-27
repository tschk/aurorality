// aurorality-lite — reactive state helpers for .crepus JS backends.
//
// Usage:
//   $.init({ count: 0 })
//   $.set("count", $.get("count") + 1)
//   $.patch({ mood: "climbing" })
//   return $.state()
//
// Methods return updated state for the Swift render cycle.
// Swift calls your exported functions → gets state back → re-renders template.

var $ = (function () {
  var _state = {};

  return {
    init: function (state) {
      _state = state || {};
      return _state;
    },

    get: function (key) {
      return _state[key];
    },

    set: function (key, value) {
      _state[key] = value;
      return _state;
    },

    state: function () {
      return _state;
    },

    patch: function (obj) {
      for (var k in obj) {
        if (obj.hasOwnProperty(k)) _state[k] = obj[k];
      }
      return _state;
    },
  };
})();
