/// In-process hot reload for `aurorality dev`: no server, no port.
///
/// `aurorality dev` launches the app with `AURORALITY_VIEWS_DIR` set to the
/// project's `views/` directory. In DEBUG builds the app reads templates from
/// there instead of its bundle and re-renders them on every save, with the
/// context the app last passed to `AurorState.load`.
///
/// Limits: the app must see the host filesystem (macOS, iOS Simulator). Real
/// devices need `aurorality dev --serve` and `HotReloadClient`. Each save
/// re-renders the whole template; there is no patch.

import Foundation

public enum AurorTemplates {
    /// The live source file for `<name>.crepus` when running under `aurorality dev`
    /// (DEBUG builds only), else `nil`.
    public static func liveURL(_ name: String) -> URL? {
        #if DEBUG
        guard let dir = ProcessInfo.processInfo.environment["AURORALITY_VIEWS_DIR"] else { return nil }
        let url = URL(fileURLWithPath: dir).appendingPathComponent("\(name).crepus")
        return FileManager.default.fileExists(atPath: url.path) ? url : nil
        #else
        return nil
        #endif
    }

    /// Template source for `<name>.crepus`: the live file under `aurorality dev`,
    /// else the bundled resource.
    public static func source(_ name: String, bundle: Bundle = .main) throws -> String {
        if let live = liveURL(name) {
            return try String(contentsOf: live, encoding: .utf8)
        }
        guard let url = bundle.url(forResource: name, withExtension: "crepus") else {
            throw AurorPluginError("\(name).crepus is missing from the app bundle")
        }
        return try String(contentsOf: url, encoding: .utf8)
    }
}

extension AurorState {
    /// Under `aurorality dev`, re-render `<name>.crepus` from source whenever it is
    /// saved, using the context of the last `load`. No-op (returns `false`) otherwise.
    /// Load the template once yourself (e.g. via `AurorTemplates.source`) first.
    @discardableResult
    public func watchLiveTemplate(_ name: String) -> Bool {
        guard let url = AurorTemplates.liveURL(name) else { return false }
        if liveWatchers[name] != nil { return true }
        liveWatchers[name] = AurorTemplateWatcher(url: url) { [weak self] in
            guard let self, let text = try? String(contentsOf: url, encoding: .utf8) else { return }
            do {
                try self.load(template: text, context: self.lastContext)
            } catch {
                self.error = "\(error)"
            }
        }
        // Tell `aurorality dev` this app reloads templates itself, so a
        // template save does not trigger a rebuild and relaunch.
        if let marker = ProcessInfo.processInfo.environment["AURORALITY_LIVE_MARKER"] {
            FileManager.default.createFile(atPath: marker, contents: nil)
        }
        return true
    }
}

/// Calls `onChange` on the main queue whenever the file at `url` is written.
/// Survives editors that save by renaming a new file over the old one.
public final class AurorTemplateWatcher {
    private let url: URL
    private let onChange: () -> Void
    private var source: DispatchSourceFileSystemObject?

    public init(url: URL, onChange: @escaping () -> Void) {
        self.url = url
        self.onChange = onChange
        arm()
    }

    deinit { source?.cancel() }

    private func arm() {
        let fd = open(url.path, O_EVTONLY)
        guard fd >= 0 else {
            // Mid-save (file briefly absent): try again shortly.
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) { [weak self] in self?.arm() }
            return
        }
        let src = DispatchSource.makeFileSystemObjectSource(
            fileDescriptor: fd, eventMask: [.write, .extend, .rename, .delete], queue: .main)
        src.setEventHandler { [weak self] in
            guard let self else { return }
            let replaced = !src.data.isDisjoint(with: [.rename, .delete])
            if replaced {
                src.cancel()
                self.source = nil
                // The path now names a new file; watch it and reload once it exists.
                DispatchQueue.main.asyncAfter(deadline: .now() + 0.05) { [weak self] in
                    self?.arm()
                    self?.onChange()
                }
            } else {
                self.onChange()
            }
        }
        src.setCancelHandler { close(fd) }
        source = src
        src.resume()
    }
}
