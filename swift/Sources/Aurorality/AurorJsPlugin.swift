/// Helpers for loading JavaScript plugins into the Aurorality plugin bridge.
///
/// Requires the `aurorality-core` Rust library to be built with the `js` feature
/// (`cargo build -p aurorality-core --features js`). The `jsLoadPlugin` UniFFI
/// export is only present in that configuration.
///
/// JavaScriptCore (JSC) is used as the JS engine — available on all iOS and macOS targets.

import Foundation

/// Load a JavaScript source string as a named plugin.
///
/// After loading, call `bridge.invoke(pluginId: id, method: "myFn", payload: "...")` as normal.
///
/// - Parameters:
///   - id: Plugin identifier used in `invoke` calls, e.g. `"counter"`.
///   - code: Full JavaScript source. Top-level `function` declarations become callable methods.
/// - Throws: `AurorPluginError` if the JS code fails to parse or the JSC runtime errors.
public func loadJsPlugin(id: String, code: String) throws {
    try jsLoadPlugin(id: id, code: code)
}

/// Convenience overload: load a bundled `.js` resource file as a plugin.
///
/// - Parameters:
///   - id: Plugin identifier used in `invoke` calls.
///   - resource: Name of the `.js` file in the bundle (without extension).
///   - bundle: Bundle to search; defaults to `.main`.
/// - Throws: `AurorPluginError` if the file is not found or load fails.
public func loadJsPlugin(id: String, resource: String, bundle: Bundle = .main) throws {
    guard let url = bundle.url(forResource: resource, withExtension: "js") else {
        throw AurorPluginError("JS plugin resource not found: \(resource).js")
    }
    let code = try String(contentsOf: url, encoding: .utf8)
    try loadJsPlugin(id: id, code: code)
}
