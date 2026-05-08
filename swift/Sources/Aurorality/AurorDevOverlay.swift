/// Opt-in dev overlay: hot-reload HUD + optional live IR preview toggle.

import SwiftUI

private struct AurorDevEnabledKey: EnvironmentKey {
    static let defaultValue = false
}

public extension EnvironmentValues {
    /// When true (or `AURORALITY_DEV=1`), [`aurorDevOverlay`] shows the HUD in DEBUG builds.
    var aurorDevEnabled: Bool {
        get { self[AurorDevEnabledKey.self] }
        set { self[AurorDevEnabledKey.self] = newValue }
    }
}

public extension View {
    /// Overlay development HUD + optional IR preview when connected to `aurorality dev`.
    func aurorDevOverlay(
        templatePath: String? = nil,
        hotReloadHost: String = "127.0.0.1",
        hotReloadPort: UInt16 = 47832
    ) -> some View {
        modifier(AurorDevOverlayModifier(
            templatePath: templatePath,
            hotReloadHost: hotReloadHost,
            hotReloadPort: hotReloadPort
        ))
    }
}

private struct AurorDevOverlayModifier: ViewModifier {
    let templatePath: String?
    let hotReloadHost: String
    let hotReloadPort: UInt16

    @Environment(\.aurorDevEnabled) private var aurorDevEnabled
    @Bindable private var bus = HotReloadBus.shared

    @State private var aurorState = AurorState()
    @State private var hotReload = HotReloadClient()
    @State private var templateLoaded = false

    func body(content: Content) -> some View {
        let envDev = ProcessInfo.processInfo.environment["AURORALITY_DEV"] == "1"
        let show = showHud(aurorDevEnabled: aurorDevEnabled || envDev)

        #if DEBUG
        return ZStack(alignment: .bottomTrailing) {
            Group {
                if bus.liveIRMode, bus.irEnabledFromServer {
                    AurorRootView(state: aurorState)
                } else {
                    content
                }
            }
            if show {
                HotReloadHUD()
                    .padding(16)
            }
        }
        .onAppear {
            guard show else { return }
            Task { await loadTemplateIfNeeded() }
            hotReload.connect(to: hotReloadHost, port: hotReloadPort, state: aurorState)
        }
        .onDisappear {
            hotReload.disconnect()
        }
        #else
        return content
        #endif
    }

    private func showHud(aurorDevEnabled: Bool) -> Bool {
        #if DEBUG
        return aurorDevEnabled || ProcessInfo.processInfo.environment["AURORALITY_DEV"] == "1"
        #else
        return false
        #endif
    }

    private func loadTemplateIfNeeded() async {
        guard !templateLoaded, let path = templatePath else { return }
        let url: URL
        if path.hasPrefix("/") {
            url = URL(fileURLWithPath: path)
        } else {
            url = URL(fileURLWithPath: FileManager.default.currentDirectoryPath).appendingPathComponent(path)
        }
        guard let data = try? Data(contentsOf: url),
            let tpl = String(data: data, encoding: .utf8)
        else { return }
        do {
            try aurorState.load(template: tpl, context: [:])
            templateLoaded = true
        } catch {
            aurorState.error = String(describing: error)
        }
    }
}
