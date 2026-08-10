#if os(Linux)
import Foundation
import WordPressAPIInternal

/// A pure-Rust `SafeRequestExecutor`, backed by `reqwest` and `rustls`. Linux only.
///
/// On Linux the default ``WpRequestExecutor`` runs on swift-corelibs-foundation's
/// libcurl bridge, which maps every curl SSL failure to `NSURLErrorUnknown`. So a
/// TLS failure can't be classified and degrades to `.genericError`
/// (Automattic/wordpress-rs#1509), and the `allowSSL` exception path is compiled
/// out. This executor reads the error from `rustls` directly, so its SSL, DNS,
/// timeout, and offline classification matches Darwin. Prefer it over
/// ``WpRequestExecutor`` on Linux — e.g. `WordPressLoginClient(executor:)`.
///
/// It is not part of the Apple xcframework: there `URLSession` is the first-class
/// executor, and pulling in `reqwest`/`rustls` would only add binary weight.
public final class ReqwestRequestExecutor: SafeRequestExecutor {
    private let inner: any RequestExecutor

    public init() {
        self.inner = newReqwestRequestExecutor()
    }

    public func execute(
        _ request: WpNetworkRequest
    ) async -> Result<WpNetworkResponse, RequestExecutionError> {
        do {
            return .success(try await inner.execute(request: request))
        } catch let error as RequestExecutionError {
            return .failure(error)
        } catch {
            // The reqwest executor only ever throws `RequestExecutionError`; this is defensive.
            return .failure(
                .RequestExecutionFailed(
                    statusCode: nil,
                    redirects: nil,
                    reason: .genericError(errorMessage: error.localizedDescription),
                    requestUrl: request.url(),
                    requestMethod: request.method()
                )
            )
        }
    }

    public func upload(
        request: WpMultipartFormRequest
    ) async -> Result<WpNetworkResponse, RequestExecutionError> {
        do {
            return .success(try await inner.upload(request: request))
        } catch let error as RequestExecutionError {
            return .failure(error)
        } catch {
            // The reqwest executor only ever throws `RequestExecutionError`; this is defensive.
            return .failure(
                .RequestExecutionFailed(
                    statusCode: nil,
                    redirects: nil,
                    reason: .genericError(errorMessage: error.localizedDescription),
                    requestUrl: request.url(),
                    requestMethod: request.method()
                )
            )
        }
    }

    public func sleep(millis: UInt64) async {
        await inner.sleep(millis: millis)
    }

    public func cancel(context: RequestContext) {
        inner.cancel(context: context)
    }
}
#endif
