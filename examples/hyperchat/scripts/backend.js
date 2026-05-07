function routeMessage(payload) {
  var text = String(payload.text || "");
  var preferred = String(payload.preferred || "auto");
  var lower = text.toLowerCase();
  var selected = preferred !== "auto" ? preferred : "matrix";
  var reason = "federated room route";

  if (lower.indexOf("room") >= 0 || lower.indexOf("federat") >= 0 || lower.indexOf("matrix") >= 0) {
    selected = "matrix";
    reason = "room/federation wording prefers Matrix";
  } else if (lower.indexOf("archive") >= 0 || lower.indexOf("mail") >= 0 || lower.indexOf("stalwart") >= 0) {
    selected = "stalwart";
    reason = "durable archive wording prefers Stalwart";
  }

  if (preferred !== "auto") {
    reason = "manual transport override";
  }

  return {
    selected: selected,
    confidence: Math.min(99, 61 + text.length),
    reason: reason,
  };
}

function digest(payload) {
  var messages = payload.messages || [];
  var last = messages.length ? messages[0].text : "No messages yet";
  return {
    count: messages.length,
    lastPreview: last.length > 48 ? last.slice(0, 45) + "..." : last,
  };
}
