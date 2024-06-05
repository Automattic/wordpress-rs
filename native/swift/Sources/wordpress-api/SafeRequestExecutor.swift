import Foundation
#if canImport(WordPressAPIInternal)
import WordPressAPIInternal
#endif

#if os(Linux)
import FoundationNetworking
#endif

public protocol SafeRequestExecutor: RequestExecutor {

    func execute(_ request: WpNetworkRequest) async -> Result<WpNetworkResponse, RequestExecutionError>

}

extension SafeRequestExecutor {

    public func execute(request: WpNetworkRequest) async throws  -> WpNetworkResponse {
        let result = await execute(request)
        return try result.get()
    }

}

extension URLSession: SafeRequestExecutor {

    public func execute(_ request: WpNetworkRequest) async -> Result<WpNetworkResponse, RequestExecutionError> {
        do {
            let (data, response) = try await self.data(for: request.asURLRequest())
            let urlResponse = response as! HTTPURLResponse
            return .success(
                WpNetworkResponse(
                    body: data,
                    statusCode: UInt16(urlResponse.statusCode),
                    headerMap: urlResponse.httpHeaders
                )
            )
        } catch {
            // TODO: Translate error into the Rust type
            return .failure(.RequestExecutionFailed(statusCode: nil, reason: ""))
        }
    }

}
