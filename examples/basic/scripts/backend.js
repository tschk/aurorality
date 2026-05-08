function describe(payload) {
  var platform = payload.platform || {};
  var version = payload.version || {};
  var os = platform.os || "unknown";
  var arch = platform.arch || "unknown";
  return {
    headline: "Hello from aurorality",
    detail: "SwiftUI shell, Rust core " + (version.aurorality || "dev") + ", JavaScript polish on " + os + "/" + arch,
    badge: "rust + javascript + swift",
  };
}
