import WordPressAPIInternal
#if canImport(OSLog)
import OSLog
#endif

/// Logs each request's method and URL, for debugging. It logs no header or body, and the URL without its user info,
/// its fragment and secret query values such as `token`.
public final class DebugMiddleware: WpApiMiddleware {
    public init() {}

    public func process(
        requestExecutor: any WordPressAPIInternal.RequestExecutor,
        response: WordPressAPIInternal.WpNetworkResponse,
        request: WordPressAPIInternal.WpNetworkRequest,
        context: RequestContext?
    ) async throws -> WordPressAPIInternal.WpNetworkResponse {
        #if canImport(OSLog)
        Logger.requests.info("Performed Request:")
        Logger.requests.info("\t Method:\t\(request.method().rawValue)")
        // Without the user info, the fragment and secret query values, such as `token`.
        Logger.requests.info("\t URL:\t\(redactRequestUrlForLog(url: request.url(), detail: .full))")
        #endif
        return response
    }
}
