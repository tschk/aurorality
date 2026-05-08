function formatCounter(payload) {
  var count = Number(payload.count || 0);
  var mood = count === 0 ? "neutral" : count > 0 ? "climbing" : "recovering";
  var next = count >= 10 ? "Reset soon" : count <= -10 ? "Bring it back" : "Keep tapping";
  return {
    display: String(count),
    mood: mood,
    next: next,
  };
}
