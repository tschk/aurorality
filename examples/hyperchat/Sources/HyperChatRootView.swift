#if canImport(Aurorality)
import Aurorality
#endif
import SwiftUI

/// Native chat shell + sidebar; detail can switch between **Messages** (model-driven) or **Template (live)** (`AurorState` fed by [`HotReloadClient`](https://)).
struct HyperChatRootView: View {
    @Bindable var model: HyperChatModel
    @Bindable var aurorState: AurorState
    @Bindable var hotReloadClient: HotReloadClient

    /// Sheet + ⌘⇧D (set from `HyperChatApp`).
    @Binding var devConnectShown: Bool

    @State private var detailPane: DetailPane = .messages

    private enum DetailPane: Int, CaseIterable {
        case messages = 0
        case template = 1
    }

    var body: some View {
        NavigationSplitView {
            conversationSidebar
                .navigationSplitViewColumnWidth(min: 240, ideal: 280, max: 400)
        } detail: {
            chatDetail
        }
        .onChange(of: model.selectedProtocol) { _, _ in
            model.syncProtocolLabelForSelection()
        }
        .onChange(of: model.selectedConversationId) { _, _ in
            model.syncProtocolLabelForSelection()
        }
        .sheet(isPresented: $devConnectShown) {
            HyperChatDevConnectView(
                client: hotReloadClient,
                aurorState: aurorState,
                isPresented: $devConnectShown
            )
        }
    }

    private var conversationSidebar: some View {
        List(selection: $model.selectedConversationId) {
            Section {
                ForEach(model.conversations) { conv in
                    Label {
                        VStack(alignment: .leading, spacing: 2) {
                            Text(conv.title)
                                .font(.body.weight(.medium))
                            Text(conv.subtitle)
                                .font(.caption)
                                .foregroundStyle(.secondary)
                                .lineLimit(1)
                            Text(conv.protocolLabel)
                                .font(.caption2)
                                .foregroundStyle(.tertiary)
                        }
                    } icon: {
                        Image(systemName: "bubble.left.and.bubble.right.fill")
                            .symbolRenderingMode(.hierarchical)
                            .foregroundStyle(.secondary)
                    }
                    .tag(Optional(conv.id))
                }
                .onDelete { model.deleteConversations(at: $0) }
            } header: {
                Text("Conversations")
            }

            Section {
                transportStatusRows
            } header: {
                Text("Transports")
            } footer: {
                Text("Set `MATRIX_*` and `STALWART_*` in your environment before launch (see README).")
                    .font(.caption2)
                    .foregroundStyle(.tertiary)
            }
        }
        .listStyle(.sidebar)
        .navigationTitle("HyperChat")
        .toolbar {
            ToolbarItem(placement: .automatic) {
                Button {
                    devConnectShown = true
                } label: {
                    Image(systemName: devToolbarIconName)
                        .symbolRenderingMode(.hierarchical)
                }
                .help("Connect to `aurorality dev` for .crepus hot reload (⌘⇧D)")
            }
            ToolbarItem(placement: .primaryAction) {
                Button("Add", systemImage: "square.and.pencil") {
                    model.addConversation()
                }
                .help("New conversation (⌘N)")
            }
        }
    }

    private var devToolbarIconName: String {
        switch hotReloadClient.status {
        case .connected: return "cable.connector"
        case .connecting: return "cable.connector"
        case .error: return "exclamationmark.triangle.fill"
        case .disconnected: return "cable.connector.slash"
        }
    }

    @ViewBuilder
    private var transportStatusRows: some View {
        transportRow(title: "Matrix", status: model.matrixTransport, symbol: "network")
        transportRow(title: "Stalwart", status: model.stalwartTransport, symbol: "cylinder.split.1x2")
        transportRow(title: "Bitchat", status: model.bitchatTransport, symbol: "antenna.radiowaves.left.and.right")
    }

    private func transportRow(title: String, status: TransportSidebarStatus, symbol: String) -> some View {
        HStack(alignment: .firstTextBaseline, spacing: 8) {
            Circle()
                .fill(status.state.dotColor)
                .frame(width: 7, height: 7)
                .accessibilityHidden(true)
            VStack(alignment: .leading, spacing: 3) {
                HStack(spacing: 6) {
                    Image(systemName: symbol)
                        .imageScale(.small)
                        .foregroundStyle(.secondary)
                    Text(title)
                        .font(.subheadline.weight(.semibold))
                    Spacer(minLength: 0)
                }
                Text(status.headline)
                    .font(.caption)
                    .foregroundStyle(.primary)
                if let d = status.detail?.trimmingCharacters(in: .whitespacesAndNewlines), !d.isEmpty {
                    Text(d)
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                        .lineLimit(2)
                        .fixedSize(horizontal: false, vertical: true)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .padding(.vertical, 2)
        .accessibilityElement(children: .combine)
    }

    private var chatDetail: some View {
        VStack(spacing: 0) {
            Picker("Main pane", selection: $detailPane) {
                Text("Messages").tag(DetailPane.messages)
                Text("Template (live)").tag(DetailPane.template)
            }
            .pickerStyle(.segmented)
            .padding(.horizontal)
            .padding(.top, 10)
            .accessibilityIdentifier("hyperchat-detail-pane-picker")

            Group {
                switch detailPane {
                case .messages:
                    messagesDetailColumn
                case .template:
                    templateLiveColumn
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
        }
        .frame(minWidth: 480, minHeight: 400)
        .navigationTitle(detailNavigationTitle)
        .toolbar {
            ToolbarItem(placement: .primaryAction) {
                Button("Refresh Status", systemImage: "arrow.clockwise") {
                    model.refreshTransportHealth()
                }
                .help("Refresh transport health (⌘R in Transport menu)")
            }
        }
    }

    private var detailNavigationTitle: String {
        switch detailPane {
        case .messages:
            if model.viewContext.headerBadge.isEmpty {
                return "Messages"
            }
            return model.viewContext.headerBadge
        case .template:
            return "Template (live)"
        }
    }

    @ViewBuilder
    private var messagesDetailColumn: some View {
        if model.selectedConversationId == nil {
            ContentUnavailableView {
                Label("No conversation", systemImage: "bubble.left.and.bubble.right")
            } description: {
                Text("Select a chat in the sidebar or create one with ⌘N.")
            }
        } else {
            VStack(spacing: 0) {
                detailHeader
                Divider()
                messageScroll
                Divider()
                composeBar
            }
        }
    }

    /// Renders **`views/main.crepus`** output as `ViewIr` from the WebSocket snapshot / patches (`aurorality dev`).
    private var templateLiveColumn: some View {
        VStack(alignment: .leading, spacing: 0) {
            Label {
                VStack(alignment: .leading, spacing: 4) {
                    Text("Watching `examples/hyperchat/views`")
                        .font(.subheadline.weight(.medium))
                    Text(
                        """
                        Edit `.crepus` files with `aurorality dev` running; connect via the sidebar cable icon or ⌘⇧D. Uses `AurorRootView` (same IR as production).
                        """
                    )
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .fixedSize(horizontal: false, vertical: true)
                }
            } icon: {
                Image(systemName: "arrow.triangle.2.circlepath")
                    .foregroundStyle(.secondary)
            }
            .padding()
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(Color(nsColor: .controlBackgroundColor).opacity(0.35))

            Divider()

            ScrollView {
                LazyVStack(alignment: .leading, spacing: 0) {
                    if let errorMsg = aurorState.error {
                        Text(errorMsg)
                            .font(.callout)
                            .foregroundStyle(.red)
                            .padding()
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .background(Color.red.opacity(0.08))
                    }
                    AurorRootView(state: aurorState)
                        .padding()
                        .frame(maxWidth: .infinity, alignment: .leading)
                }
            }
        }
    }

    private var detailHeader: some View {
        Form {
            Section {
                VStack(alignment: .leading, spacing: 4) {
                    Text(model.viewContext.recipientTitle)
                        .font(.title3)
                        .fontWeight(.semibold)
                    Text(model.viewContext.protocolSummary)
                        .font(.subheadline)
                        .foregroundStyle(.secondary)
                }
                .padding(.vertical, 4)
            }
            Section("Channel") {
                Picker("Channel", selection: $model.selectedProtocol) {
                    Text("Choose…").tag("")
                    Text("Matrix").tag("matrix")
                    Text("Stalwart").tag("stalwart")
                    Text("Bitchat").tag("bitchat")
                }
                .pickerStyle(.segmented)
                .accessibilityLabel("Message channel")

                if model.viewContext.newConversationPrompt {
                    Label("Pick a channel before sending.", systemImage: "exclamationmark.triangle.fill")
                        .font(.caption)
                        .foregroundStyle(.orange)
                } else if model.viewContext.bitchatSendBlocked {
                    Label(
                        "Bitchat is view-only here—choose Matrix or Stalwart to send.",
                        systemImage: "info.circle.fill"
                    )
                    .font(.caption)
                    .foregroundStyle(.secondary)
                }
            }
        }
        .formStyle(.grouped)
    }

    private var messageScroll: some View {
        ScrollViewReader { proxy in
            ScrollView {
                LazyVStack(alignment: .leading, spacing: 10) {
                    if model.viewContext.messages.isEmpty {
                        ContentUnavailableView {
                            Label("No messages yet", systemImage: "text.bubble")
                        } description: {
                            Text("Send a message using the field below.")
                        }
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 48)
                    } else {
                        ForEach(model.viewContext.messages) { m in
                            MessageBubble(message: m)
                                .id(m.id)
                        }
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding()
            }
            .onChange(of: model.viewContext.messages.last?.id) { _, newId in
                guard let newId else { return }
                withAnimation(.easeOut(duration: 0.18)) {
                    proxy.scrollTo(newId, anchor: .bottom)
                }
            }
        }
    }

    private var composeBar: some View {
        HStack(alignment: .bottom, spacing: 12) {
            TextField(
                "Write a message…",
                text: $model.draft,
                axis: .vertical
            )
            .textFieldStyle(.roundedBorder)
            .lineLimit(1 ... 8)
            .frame(minHeight: 22)
            .onSubmit {
                if model.viewContext.canSend {
                    model.commitSend()
                }
            }

            Button {
                model.commitSend()
            } label: {
                Text("Send")
                    .fontWeight(.semibold)
            }
            .keyboardShortcut(.return, modifiers: .command)
            .disabled(!model.viewContext.canSend)
            .help(model.viewContext.sendDisabledHint ?? "Send message")
        }
        .padding()
        .background(Color(nsColor: .textBackgroundColor))
    }
}

private struct MessageBubble: View {
    let message: MessageItem

    var body: some View {
        HStack(alignment: .top, spacing: 0) {
            if message.isOutgoing { Spacer(minLength: 56) }
            VStack(alignment: message.isOutgoing ? .trailing : .leading, spacing: 4) {
                Text(message.body)
                    .textSelection(.enabled)
                    .frame(maxWidth: 420, alignment: message.isOutgoing ? .trailing : .leading)
                Text(message.metaLine)
                    .font(.caption2)
                    .foregroundStyle(.secondary)
            }
            .padding(.horizontal, 12)
            .padding(.vertical, 8)
            .background {
                RoundedRectangle(cornerRadius: 14, style: .continuous)
                    .fill(message.isOutgoing
                        ? Color.accentColor.opacity(0.18)
                        : Color(nsColor: .controlBackgroundColor))
            }
            .overlay {
                RoundedRectangle(cornerRadius: 14, style: .continuous)
                    .strokeBorder(Color.primary.opacity(0.06), lineWidth: 0.5)
            }
            if !message.isOutgoing { Spacer(minLength: 56) }
        }
    }
}
