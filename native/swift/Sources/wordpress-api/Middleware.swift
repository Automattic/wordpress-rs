import WordPressAPIInternal

public final class DebugMiddleware: WpApiMiddleware {
    public func process(
        requestExecutor: any WordPressAPIInternal.RequestExecutor,
        response: WordPressAPIInternal.WpNetworkResponse,
        request: WordPressAPIInternal.WpNetworkRequest,
        cancellationToken: CancellationToken?
    ) async throws -> WordPressAPIInternal.WpNetworkResponse {
        debugPrint("Performed request: \(String(describing: try? request.buildURLRequest(additionalHeaders: [:])))")
        debugPrint("Received response: \(response)")
        return response
    }
}
