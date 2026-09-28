import Foundation

#if canImport(Network)
import Network
#endif

/// Whether the device currently has a usable network path.
///
/// The executor consults this when a `URLError` alone can't say whether the device went offline —
/// `.networkConnectionLost` is reported both when the device drops off the network mid-request and
/// when the server (or a proxy or load balancer) severs a connection on a healthy network. This is
/// the Swift counterpart of Kotlin's `NetworkAvailabilityProvider`.
protocol NetworkAvailability: Sendable {
    var isNetworkAvailable: Bool { get }
}

#if canImport(Network)
/// Tracks the system's network path with a single, process-wide `NWPathMonitor`.
final class SystemNetworkAvailability: NetworkAvailability, @unchecked Sendable {
    static let shared = SystemNetworkAvailability()

    private let monitor = NWPathMonitor()
    private let lock = NSLock()
    private var status: NWPath.Status?

    private init() {
        monitor.pathUpdateHandler = { [weak self] path in
            guard let self else { return }
            lock.withLock { self.status = path.status }
        }
        monitor.start(queue: DispatchQueue(label: "org.wordpress.api.network-availability"))
    }

    var isNetworkAvailable: Bool {
        // Before the monitor's first update there's no signal either way. Assume the network is up,
        // so the failure is reported as what the connection did rather than as an offline device.
        lock.withLock { status.map { $0 == .satisfied } ?? true }
    }
}
#endif

/// For platforms without the Network framework (Linux), where there's no path signal to consult.
struct AssumedNetworkAvailability: NetworkAvailability {
    var isNetworkAvailable: Bool { true }
}
