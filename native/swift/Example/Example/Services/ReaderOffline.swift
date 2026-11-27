import Foundation
import os

/// Fails every request while ``isOffline`` is on, the way a device with no connection does, so the Reader's
/// offline behaviour can be seen without turning the network off.
///
/// It's only for the example app's "Simulate no connection" switch and the `-reader-offline` launch argument.
/// While it's off it declines every request, and the session loads them as usual.
final class ReaderOfflineURLProtocol: URLProtocol {
    /// Launch the app with this argument to start with no connection.
    static let launchArgument = "-reader-offline"

    private static let offline = OSAllocatedUnfairLock(
        initialState: ProcessInfo.processInfo.arguments.contains(launchArgument)
    )

    static var isOffline: Bool {
        get { offline.withLock { $0 } }
        set { offline.withLock { $0 = newValue } }
    }

    override static func canInit(with request: URLRequest) -> Bool {
        isOffline
    }

    override static func canonicalRequest(for request: URLRequest) -> URLRequest {
        request
    }

    override func startLoading() {
        // The words the system's own error has, which an error made here would otherwise lack.
        let error = URLError(
            .notConnectedToInternet,
            userInfo: [NSLocalizedDescriptionKey: "The Internet connection appears to be offline."]
        )
        client?.urlProtocol(self, didFailWithError: error)
    }

    override func stopLoading() {}
}

enum ReaderNetwork {
    /// The session every Reader request goes through.
    static let urlSession: URLSession = {
        let configuration = URLSessionConfiguration.default
        configuration.protocolClasses = [ReaderOfflineURLProtocol.self] + (configuration.protocolClasses ?? [])
        return URLSession(configuration: configuration)
    }()
}
