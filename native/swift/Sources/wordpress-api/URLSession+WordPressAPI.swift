import Foundation
import wordpress_api_wrapper

#if os(Linux)
import FoundationNetworking
#endif

typealias WordPressAPIResult<Response> = Result<Response, WordPressAPIError>

extension URLSession {

    /// Send a HTTP request and return its response as a `WordPressAPIResult` instance.
    ///
    /// ## Progress Tracking and Cancellation
    ///
    /// You can track the HTTP request's overall progress by passing a `Progress` instance to the `fulfillingProgress`
    /// parameter, which must satisify following requirements:
    /// - `totalUnitCount` must not be zero.
    /// - `completedUnitCount` must be zero.
    /// - It's used exclusivity for tracking the HTTP request overal progress: No children in its progress tree.
    /// - `cancellationHandler` must be nil. You can call `fulfillingProgress.cancel()` to cancel the ongoing HTTP request.
    ///
    ///  Upon completion, the HTTP request's progress fulfills the `fulfillingProgress`.
    ///
    ///  Please note: `parentProgress` may be updated from a background thread.
    ///
    /// - Parameters:
    ///   - request: A `WpNetworkRequest` instance that represents an HTTP request to be sent.
    ///   - parentProgress: A `Progress` instance that will be used as the parent progress of the HTTP request's overall
    ///         progress. See the function documentation regarding requirements on this argument.
    func perform(
        request: WpNetworkRequest,
        fulfilling parentProgress: Progress? = nil
    ) async -> WordPressAPIResult<WpNetworkResponse> {
#if WP_SUPPORT_BACKGROUND_URL_SESSION
        if configuration.identifier != nil {
            assert(delegate is BackgroundURLSessionDelegate, "Unexpected `URLSession` delegate type. See the `backgroundSession(configuration:)`")
        }
#endif

        if let parentProgress {
            assert(parentProgress.completedUnitCount == 0 && parentProgress.totalUnitCount > 0, "Invalid parent progress")
            assert(parentProgress.cancellationHandler == nil, "The progress instance's cancellationHandler property must be nil")
        }

        return await withCheckedContinuation { continuation in
            let completion: @Sendable (Data?, URLResponse?, Error?) -> Void = { data, response, error in
                let result: WordPressAPIResult<WpNetworkResponse> = Self.parseResponse(
                    data: data,
                    response: response,
                    error: error
                )

                continuation.resume(returning: result)
            }

            let task: URLSessionTask

            do {
                task = try self.task(for: request, completion: completion)
            } catch {
                continuation.resume(returning: .failure(.requestEncodingFailure(underlyingError: error)))
                return
            }

            task.resume()

            if let parentProgress, parentProgress.totalUnitCount > parentProgress.completedUnitCount {
                let pending = parentProgress.totalUnitCount - parentProgress.completedUnitCount
                parentProgress.addChild(task.progress, withPendingUnitCount: pending)

                parentProgress.cancellationHandler = { [weak task] in
                    task?.cancel()
                }
            }
        }
    }

    private func task(
        for wpRequest: WpNetworkRequest,
        completion: @escaping @Sendable (Data?, URLResponse?, Error?) -> Void
    ) throws -> URLSessionTask {
        let request = try wpRequest.asURLRequest()

        if let body = wpRequest.body {
            return createUploadTask(with: request, body: body, completion: completion)
        } else {
            return createDataTask(with: request, completion: completion)
        }
    }

    private static func parseResponse(
        data: Data?,
        response: URLResponse?,
        error: Error?
    ) -> WordPressAPIResult<WpNetworkResponse> {
        let result: WordPressAPIResult<WpNetworkResponse>

        if let error {
            if let urlError = error as? URLError {
                result = .failure(.connection(urlError))
            } else {
                result = .failure(.unknown(underlyingError: error))
            }
        } else {
            if let httpResponse = response as? HTTPURLResponse {
                result = .success(.init(body: data ?? Data(), statusCode: UInt16(httpResponse.statusCode), headerMap: httpResponse.httpHeaders))
            } else {
                result = .failure(.unparsableResponse(response: nil, body: data, underlyingError: URLError(.badServerResponse)))
            }
        }

        return result
    }

}

// MARK: - Background URL Session Support

#if WP_SUPPORT_BACKGROUND_URL_SESSION

private extension URLSession {

    func createDataTask(with request: URLRequest, completion: @escaping @Sendable (Data?, URLResponse?, (any Error)?) -> Void) -> URLSessionDataTask {
        // This additional `callCompletionFromDelegate` is added to unit test `BackgroundURLSessionDelegate`.
        // Background `URLSession` doesn't work on unit tests, we have to create a non-background `URLSession`
        // which has a `BackgroundURLSessionDelegate` delegate in order to test `BackgroundURLSessionDelegate`.
        //
        // In reality, `callCompletionFromDelegate` and `isBackgroundSession` have the same value.
        let callCompletionFromDelegate = delegate is BackgroundURLSessionDelegate

        let task: URLSessionDataTask
        if callCompletionFromDelegate {
            task = dataTask(with: request)
            set(completion: completion, forTaskWithIdentifier: task.taskIdentifier)
        } else {
            task = dataTask(with: request, completionHandler: completion)
        }

        return task
    }

    func createUploadTask(with request: URLRequest, body: Either<Data, URL>, completion originalCompletion: @escaping @Sendable (Data?, URLResponse?, (any Error)?) -> Void) -> URLSessionUploadTask {
        // This additional `callCompletionFromDelegate` is added to unit test `BackgroundURLSessionDelegate`.
        // Background `URLSession` doesn't work on unit tests, we have to create a non-background `URLSession`
        // which has a `BackgroundURLSessionDelegate` delegate in order to test `BackgroundURLSessionDelegate`.
        //
        // In reality, `callCompletionFromDelegate` and `isBackgroundSession` have the same value.
        let callCompletionFromDelegate = delegate is BackgroundURLSessionDelegate

        var completion = originalCompletion

        let task = body.map(
            left: {
                if callCompletionFromDelegate {
                    return uploadTask(with: request, from: $0)
                } else {
                    return uploadTask(with: request, from: $0, completionHandler: completion)
                }
            },
            right: { tempFileURL in
                // Remove the temp file, which contains request body, once the HTTP request completes.
                completion = { data, response, error in
                    try? FileManager.default.removeItem(at: tempFileURL)
                    originalCompletion(data, response, error)
                }

                if callCompletionFromDelegate {
                    return uploadTask(with: request, fromFile: tempFileURL)
                } else {
                    return uploadTask(with: request, fromFile: tempFileURL, completionHandler: completion)
                }
            }
        )

        if callCompletionFromDelegate {
            set(completion: completion, forTaskWithIdentifier: task.taskIdentifier)
        }

        return task
    }

}

#else

private extension URLSession {

    func createDataTask(with request: URLRequest, completion: @escaping @Sendable (Data?, URLResponse?, (any Error)?) -> Void) -> URLSessionDataTask {
        dataTask(with: request, completionHandler: completion)
    }

    func createUploadTask(with request: URLRequest, body: Either<Data, URL>, completion originalCompletion: @escaping @Sendable (Data?, URLResponse?, (any Error)?) -> Void) -> URLSessionUploadTask {
        body.map(
            left: {
                uploadTask(with: request, from: $0, completionHandler: originalCompletion)
            },
            right: { tempFileURL in
                // Remove the temp file, which contains request body, once the HTTP request completes.
                let completion = { data, response, error in
                    try? FileManager.default.removeItem(at: tempFileURL)
                    originalCompletion(data, response, error)
                }
                return uploadTask(with: request, fromFile: tempFileURL, completionHandler: completion)
            }
        )
    }

}

#endif

#if WP_SUPPORT_BACKGROUND_URL_SESSION

extension URLSession {

    /// Create a background URLSession instance that can be used in the `perform(request:...)` async function.
    ///
    /// The `perform(request:...)` async function can be used in all non-background `URLSession` instances without any
    /// extra work. However, there is a requirement to make the function works with with background `URLSession` instances.
    /// That is the `URLSession` must have a delegate of `BackgroundURLSessionDelegate` type.
    static func backgroundSession(configuration: URLSessionConfiguration) -> URLSession {
        assert(configuration.identifier != nil)
        // Pass `delegateQueue: nil` to get a serial queue, which is required to ensure thread safe access to
        // `WordPressKitSessionDelegate` instances.
        return URLSession(configuration: configuration, delegate: BackgroundURLSessionDelegate(), delegateQueue: nil)
    }

}

private final class SessionTaskData {
    var responseBody = Data()
    var completion: ((Data?, URLResponse?, Error?) -> Void)?
}

class BackgroundURLSessionDelegate: NSObject, URLSessionDataDelegate {

    func urlSession(_ session: URLSession, dataTask: URLSessionDataTask, didReceive data: Data) {
        session.received(data, forTaskWithIdentifier: dataTask.taskIdentifier)
    }

    func urlSession(_ session: URLSession, task: URLSessionTask, didCompleteWithError error: Error?) {
        session.completed(with: error, response: task.response, forTaskWithIdentifier: task.taskIdentifier)
    }

}

private extension URLSession {

    static var taskDataKey = 0

    // A map from `URLSessionTask` identifier to in-memory data of the given task.
    //
    // This property is in `URLSession` not `BackgroundURLSessionDelegate` because task id (the key) is unique within
    // the context of a `URLSession` instance. And in theory `BackgroundURLSessionDelegate` can be used by multiple
    // `URLSession` instances.
    var taskData: [Int: SessionTaskData] {
        get {
            objc_getAssociatedObject(self, &URLSession.taskDataKey) as? [Int: SessionTaskData] ?? [:]
        }
        set {
            objc_setAssociatedObject(self, &URLSession.taskDataKey, newValue, .OBJC_ASSOCIATION_RETAIN)
        }
    }

    func updateData(forTaskWithIdentifier taskID: Int, using closure: (SessionTaskData) -> Void) {
        let task = self.taskData[taskID] ?? SessionTaskData()
        closure(task)
        self.taskData[taskID] = task
    }

    func set(completion: @escaping (Data?, URLResponse?, Error?) -> Void, forTaskWithIdentifier taskID: Int) {
        updateData(forTaskWithIdentifier: taskID) {
            $0.completion = completion
        }
    }

    func received(_ data: Data, forTaskWithIdentifier taskID: Int) {
        updateData(forTaskWithIdentifier: taskID) { task in
            task.responseBody.append(data)
        }
    }

    func completed(with error: Error?, response: URLResponse?, forTaskWithIdentifier taskID: Int) {
        guard let task = taskData[taskID] else {
            return
        }

        if let error {
            task.completion?(nil, response, error)
        } else {
            task.completion?(task.responseBody, response, nil)
        }

        self.taskData.removeValue(forKey: taskID)
    }

}

#endif

// MARK: - wordpress_api_wrapper helpers

extension WpNetworkRequest {
    func asURLRequest() throws -> URLRequest {
        guard let url = URL(string: self.url), (url.scheme == "http" || url.scheme == "https") else {
            throw URLError(.badURL)
        }
        var request = URLRequest(url: url)
        request.httpMethod = self.method.rawValue
        request.allHTTPHeaderFields = self.headerMap
        return request
    }

    var body: Either<Data, URL>? {
        // TODO: To be implemented
        return nil
    }
}

extension HTTPURLResponse {

    var httpHeaders: [String: String] {
        allHeaderFields.reduce(into: [String: String]()) {
            guard
                let key = $1.key as? String,
                let value = $1.value as? String
            else {
                return
            }

            $0.updateValue(value, forKey: key)
        }
    }
}

// MARK: - Debug / unit test supprt

extension URLSession {
    var debugNumberOfTaskData: Int {
#if WP_SUPPORT_BACKGROUND_URL_SESSION
        self.taskData.count
#else
        0
#endif
    }
}
