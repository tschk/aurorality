var count = 0;

function increment() {
  count++;
  return _state();
}

function decrement() {
  count--;
  return _state();
}

function state() {
  return _state();
}

function _state() {
  var mood = count === 0 ? "neutral" : count > 0 ? "climbing" : "recovering";
  var next = count >= 10 ? "Reset soon" : count <= -10 ? "Bring it back" : "Keep tapping";
  return {
    count: String(count),
    mood: mood,
    next: next,
  };
}
