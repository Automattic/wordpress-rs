#if canImport(OSLog)
import OSLog

extension Logger {
    /// Using your bundle identifier is a great way to ensure a unique identifier.
    private static let subsystem = Bundle.main.bundleIdentifier ?? "wordpress-api"

    /// Logs the view cycles like a view that appeared.
    static let requests = Logger(subsystem: subsystem, category: "http")
}
#endif
