/// WebSocket client that receives HotReloadEnvelope messages from `aurorality dev`
/// and patches AurorState. Active in DEBUG builds only.

import Foundation
import Network

@Observable
public final class HotReloadClient {
    public enum Status {
        case disconnected, connecting, connected, error(String)
    }

    public var status: Status = .disconnected
    public private(set) var host: String = "127.0.0.1"
    public private(set) var port: UInt16 = 47832

    private var webSocketTask: URLSessionWebSocketTask?
    private var lastSequence: UInt64 = 0
    private weak var state: AurorState?

    public init() {}

    public func connect(to host: String = "127.0.0.1", port: UInt16 = 47832, state: AurorState) {
        self.host = host
        self.port = port
        self.state = state

        disconnect()

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

    public func disconnect() {
        webSocketTask?.cancel(with: .normalClosure, reason: nil)
        webSocketTask = nil
        status = .disconnected
    }

    // MARK: - Private

    private func receiveLoop() {
        webSocketTask?.receive { [weak self] result in
            guard let self else { return }
            switch result {
            case .failure(let err):
                DispatchQueue.main.async {
                    self.status = .error(err.localizedDescription)
                }
            case .success(let message):
                if case .string(let text) = message {
                    self.handle(text)
                }
                self.receiveLoop()
            }
        }
    }

    private func handle(_ json: String) {
        guard let data = json.data(using: .utf8) else { return }
        let decoder = JSONDecoder()
        guard let envelope = try? decoder.decode(HotReloadEnvelope.self, from: data) else {
            return
        }
        guard envelope.sequence > lastSequence || envelope.sequence == 0 else { return }
        lastSequence = envelope.sequence

        DispatchQueue.main.async { [weak self] in
            self?.state?.apply(envelope.message)
        }
    }
}
