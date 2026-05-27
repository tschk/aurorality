$.state.init({ count: "0", mood: "neutral", next: "Tap a button" });

function increment() {
  var c = Number($.state.get("count")) + 1;
  $.state.set("count", String(c));
  $.state.patch({
    mood: c === 0 ? "neutral" : c > 0 ? "climbing" : "recovering",
    next: c >= 10 ? "Reset soon" : c <= -10 ? "Bring it back" : "Keep tapping",
  });
  return $.state.all();
}

function decrement() {
  var c = Number($.state.get("count")) - 1;
  $.state.set("count", String(c));
  $.state.patch({
    mood: c === 0 ? "neutral" : c > 0 ? "climbing" : "recovering",
    next: c >= 10 ? "Reset soon" : c <= -10 ? "Bring it back" : "Keep tapping",
  });
  return $.state.all();
}

function state() {
  return $.state.all();
}
