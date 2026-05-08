/// Compact floating HUD showing dev WebSocket + swiftgen status.

import SwiftUI

public struct HotReloadHUD: View {
    @Bindable private var bus = HotReloadBus.shared

    public init() {}

    public var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(spacing: 6) {
                Circle()
                    .fill(statusColor)
                    .frame(width: 8, height: 8)
                Text("Aurorality dev")
                    .font(.caption.weight(.semibold))
                Spacer(minLength: 0)
            }
            if let ok = bus.lastSwiftgenOk {
                Text(ok ? "swiftgen OK" : "swiftgen failed")
                    .font(.caption2.monospaced())
                    .foregroundStyle(ok ? Color.secondary : Color.red)
            }
            if !bus.lastSwiftgenErrors.isEmpty {
                Text(bus.lastSwiftgenErrors.joined(separator: "\n"))
                    .font(.caption2.monospaced())
                    .foregroundStyle(.red)
                    .lineLimit(4)
            }
            Toggle("Live IR preview", isOn: $bus.liveIRMode)
                .font(.caption2)
                .toggleStyle(.switch)
                .disabled(!bus.irEnabledFromServer)
        }
        .padding(10)
        .background(.ultraThinMaterial, in: RoundedRectangle(cornerRadius: 10, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: 10, style: .continuous)
                .strokeBorder(Color.primary.opacity(0.08), lineWidth: 1)
        )
        .frame(maxWidth: 280)
    }

    private var statusColor: Color {
        if let ok = bus.lastSwiftgenOk {
            return ok ? .green : .red
        }
        return .secondary
    }
}
