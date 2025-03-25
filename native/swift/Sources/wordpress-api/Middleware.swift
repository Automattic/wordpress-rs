import WordPressAPIInternal

public final class DebugMiddleware: WpApiMiddleware {
    public func process(
        requestExecutor: any WordPressAPIInternal.RequestExecutor,
        response: WordPressAPIInternal.WpNetworkResponse,
        request: WordPressAPIInternal.WpNetworkRequest
    ) async throws -> WordPressAPIInternal.WpNetworkResponse {
        debugPrint("Performed request: \(request.asURLRequest())")
        debugPrint("Received response: \(response)")
        return response
    }
}
