import Foundation
import Testing

#if canImport(FoundationNetworking)
import FoundationNetworking
#endif

@testable import WordPressAPI
@testable import WordPressAPIInternal

@Suite(.serialized)
struct TaskDelegateTests {

    @Test
    func success() async throws {
        let delegate = Delegate()
        let api = WordPressAPI.admin(notifyingDelegate: delegate)
        _ = try await api.users.retrieveMeWithEditContext()

        let invocations = delegate.invocations
        #expect(invocations.contains("urlSession(_:dataTask:didReceive:completionHandler:)"))
        #expect(invocations.contains("urlSession(_:dataTask:didReceive:)"))
        #expect(invocations.contains("urlSession(_:task:didCompleteWithError:)"))

        #if !os(Linux)
        #expect(invocations.contains("urlSession(_:task:didFinishCollecting:)"))
        #endif
    }

}

private final class Delegate: NSObject, URLSessionTaskDelegate, URLSessionDataDelegate, @unchecked Sendable {
    private let lock = NSLock()
    private var _invocations: [String] = []
    var invocations: [String] {
        lock.withLock {
            _invocations
        }
    }

    func urlSession(
        _ session: URLSession,
        dataTask: URLSessionDataTask,
        didReceive response: URLResponse,
        completionHandler: @escaping (URLSession.ResponseDisposition) -> Void
    ) {
        lock.withLock {
            _invocations.append(#function)
        }
    }

    func urlSession(_ session: URLSession, dataTask: URLSessionDataTask, didReceive data: Data) {
        lock.withLock {
            _invocations.append(#function)
        }
    }

    func urlSession(_ session: URLSession, task: URLSessionTask, didCompleteWithError error: (any Error)?) {
        lock.withLock {
            _invocations.append(#function)
        }
    }

    func urlSession(_ session: URLSession, task: URLSessionTask, didFinishCollecting metrics: URLSessionTaskMetrics) {
        lock.withLock {
            _invocations.append(#function)
        }
    }
}
