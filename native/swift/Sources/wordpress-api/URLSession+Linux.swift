#if os(Linux)

import Foundation
import FoundationNetworking

// `URLSession.data(for:) async throws` is not available on Linux's Foundation framework.
extension URLSession {
    func data(for request: URLRequest) async throws -> (Data, URLResponse) {
        try await withCheckedThrowingContinuation { continuation in
            let task = self.dataTask(with: request) { data, response, error in
                if let error {
                    continuation.resume(throwing: error)
                    return
                }

                guard let data = data, let response = response else {
                    continuation.resume(throwing: WordPressAPI.Errors.unableToParseResponse)
                    return
                }

                continuation.resume(returning: (data, response))
            }
            task.resume()
        }
    }
}

#endif
