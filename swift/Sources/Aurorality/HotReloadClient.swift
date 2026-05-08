/// WebSocket client that receives HotReloadEnvelope messages from `aurorality dev`
/// and patches AurorState. Active in DEBUG builds only.

import Foundation

@Observable
public final class HotReloadClient {
    public enum Status: Equatable {
        case disconnected, connecting, connected, error(String)
    }

    public var status: Status = .disconnected
    public private(set) var host: String = "127.0.0.1"
    public private(set) var port: UInt16 = 47832

    private var webSocketTask: URLSessionWebSocketTask?
    private var lastSequence: UInt64 = 0
    private weak var state: AurorState?
    private var shouldReconnect = false
    private var reconnectAttempts = 0
    private var reconnectWorkItem: DispatchWorkItem?

    public init() {}

    public func connect(to host: String = "127.0.0.1", port: UInt16 = 47832, state: AurorState) {
        self.host = host
        self.port = port
        self.state = state
        shouldReconnect = true
        reconnectWorkItem?.cancel()

        disconnectSocketOnly()

        guard let url = URL(string: "ws://\(host):\(port)") else {
            status = .error("invalid URL")
            return
        }

        status = .connecting
        webSocketTask = URLSession.shared.webSocketTask(with: url)
        webSocketTask?.resume()
        status = .connected
        receiveLoop()
    }

    /// Disconnect and disable automatic reconnect.
    public func disconnect() {
        shouldReconnect = false
        reconnectWorkItem?.cancel()
        reconnectWorkItem = nil
        disconnectSocketOnly()
        status = .disconnected
    }

    private func disconnectSocketOnly() {
        webSocketTask?.cancel(with: .normalClosure, reason: nil)
        webSocketTask = nil
    }

    // MARK: - Private

    private func receiveLoop() {
        webSocketTask?.receive { [weak self] result in
            guard let self else { return }
            switch result {
            case .failure(let err):
                DispatchQueue.main.async {
                    self.status = .error(err.localizedDescription)
                    self.scheduleReconnectIfNeeded()
                }
            case .success(let message):
                if case .string(let text) = message {
                    self.handle(text)
                }
                self.receiveLoop()
            }
        }
    }

    private func scheduleReconnectIfNeeded() {
        guard shouldReconnect, state != nil else { return }
        reconnectWorkItem?.cancel()
        reconnectAttempts += 1
        let delay = min(30.0, 0.5 * pow(2.0, Double(min(reconnectAttempts, 6))))
        let item = DispatchWorkItem { [weak self] in
            guard let self, let st = self.state else { return }
            self.connect(to: self.host, port: self.port, state: st)
        }
        reconnectWorkItem = item
        DispatchQueue.main.asyncAfter(deadline: .now() + delay, execute: item)
    }

    private func handle(_ json: String) {
        guard let data = json.data(using: .utf8) else { return }
        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase
        guard let envelope = try? decoder.decode(HotReloadEnvelope.self, from: data) else {
            return
        }
        reconnectAttempts = 0
        guard envelope.sequence > lastSequence || envelope.sequence == 0 else { return }
        lastSequence = envelope.sequence

        DispatchQueue.main.async { [weak self] in
            guard let self else { return }
            HotReloadBus.shared.ingest(envelope.message)
            self.state?.apply(envelope.message)
        }
    }
}
