import WordPressAPIInternal
import OSLog

public final class DebugMiddleware: WpApiMiddleware {
    public init(){}

    public func process(
        requestExecutor: any WordPressAPIInternal.RequestExecutor,
        response: WordPressAPIInternal.WpNetworkResponse,
        request: WordPressAPIInternal.WpNetworkRequest,
        context: RequestContext?
    ) async throws -> WordPressAPIInternal.WpNetworkResponse {
        Logger.requests.info("Performed Request:")
        Logger.requests.info("\t Method:\t\(request.method().rawValue)")
        Logger.requests.info("\t URL:\t\(request.url())")
        return response
    }
}
