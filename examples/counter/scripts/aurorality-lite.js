// aurorality-lite — reactive state helpers for .crepus JS backends.
//
//   $.state.init({ count: 0 })
//   $.state.set("count", $.state.get("count") + 1)
//   $.state.patch({ mood: "climbing" })
//   return $.state.all()
//
// Always available — auto-injected by the Rust bridge before your code runs.

var $ = {
  state: (function () {
    var _s = {};

    return {
      init: function (obj) {
        _s = obj || {};
        return _s;
      },

      get: function (key) {
        return _s[key];
      },

      set: function (key, val) {
        _s[key] = val;
        return _s;
      },

      all: function () {
        return _s;
      },

      patch: function (obj) {
        for (var k in obj) {
          if (obj.hasOwnProperty(k)) _s[k] = obj[k];
        }
        return _s;
      },
    };
  })(),
};
