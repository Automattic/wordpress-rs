import Foundation
#if canImport(WordPressAPIInternal)
import WordPressAPIInternal
#endif

#if os(Linux)
import FoundationNetworking
#endif

public protocol SafeRequestExecutor: RequestExecutor, WordPressOrgApiRequestExecutor, Sendable {
    func execute(_ request: WpNetworkRequest) async -> Result<WpNetworkResponse, RequestExecutionError>
}

extension SafeRequestExecutor {

    public func execute(request: WpNetworkRequest) async throws -> WpNetworkResponse {
        let result = await execute(request)
        return try result.get()
    }

    public func execute(request: WpNetworkRequest) async throws -> WordPressOrgApiNetworkResponse {
        let result = await execute(request)
            .map { WordPressOrgApiNetworkResponse(inner: $0, dummy: .init()) }
            .mapError { error in
                switch error {
                case let .RequestExecutionFailed(statusCode, reason):
                    return WordPressOrgApiRequestExecutionError.RequestExecutionFailed(
                        statusCode: statusCode,
                        reason: reason
                    )
                }
            }
        return try result.get()
    }

}

extension URLSession: RequestExecutor {
    public func uploadMedia(mediaUploadRequest: MediaUploadRequest) async throws -> WpNetworkResponse {
        try WpNetworkResponse(body: Data(), statusCode: 500, headerMap: .fromMap(hashMap: [:]))
    }
}

extension URLSession: SafeRequestExecutor {

    public func execute(_ request: WpNetworkRequest) async -> Result<WpNetworkResponse, RequestExecutionError> {
        let (data, response): (Data, URLResponse)
        do {
            (data, response) = try await self.data(for: request.asURLRequest())
        } catch {
            return .failure(.RequestExecutionFailed(statusCode: nil, reason: error.localizedDescription))
        }

        // swiftlint:disable:next force_cast
        let urlResponse = response as! HTTPURLResponse

        let headerMap: WpNetworkHeaderMap

        do {
            headerMap = try WpNetworkHeaderMap.fromMap(hashMap: urlResponse.httpHeaders)
        } catch is WpNetworkHeaderMapError {
            return .failure(.RequestExecutionFailed(statusCode: nil, reason: "Invalid header"))
        } catch {
            return .failure(.RequestExecutionFailed(statusCode: nil, reason: "Unknown error: \(error)"))
        }

        return .success(
            WpNetworkResponse(
                body: data,
                statusCode: UInt16(urlResponse.statusCode),
                headerMap: headerMap
            )
        )
    }
}
