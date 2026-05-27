$.init({ count: 0, mood: "neutral", next: "Tap a button" });

function increment() {
  var c = $.get("count") + 1;
  $.set("count", c);
  $.patch({
    mood: c === 0 ? "neutral" : c > 0 ? "climbing" : "recovering",
    next: c >= 10 ? "Reset soon" : c <= -10 ? "Bring it back" : "Keep tapping",
  });
  return $.state();
}

function decrement() {
  var c = $.get("count") - 1;
  $.set("count", c);
  $.patch({
    mood: c === 0 ? "neutral" : c > 0 ? "climbing" : "recovering",
    next: c >= 10 ? "Reset soon" : c <= -10 ? "Bring it back" : "Keep tapping",
  });
  return $.state();
}

function state() {
  return $.state();
}
