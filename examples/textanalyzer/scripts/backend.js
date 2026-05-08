function score(payload) {
  var words = Number(payload.wordCount || 0);
  var chars = Number(payload.charCount || 0);
  var density = words === 0 ? 0 : Math.round(chars / Math.max(words, 1));
  var read = density <= 4 ? "crisp" : density <= 7 ? "balanced" : "dense";
  return {
    density: density,
    readability: read,
    summary: words + " words, " + chars + " chars, " + read + " cadence",
  };
}
