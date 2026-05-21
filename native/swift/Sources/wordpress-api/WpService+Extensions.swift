import Foundation
import WordPressAPIInternal
#if canImport(OSLog)
import OSLog
#endif

extension WpService {
    #if PROGRESS_REPORTING_ENABLED
    public func uploadMedia(
        params: MediaCreateParams,
        fulfilling progress: Progress
    ) async throws -> MediaWithEditContext {
        if let executor = requestExecutor() as? SafeRequestExecutor {
            return try await executor.fulfill(progress: progress) { [service = media()] context in
                try await service.createMedia(params: params, context: context)
            }
        } else {
            #if canImport(OSLog)
            Logger.requests.error(
                "WpService.uploadMedia: request executor is not a SafeRequestExecutor; upload progress and cancellation will not be reported."
            )
            #endif
            return try await media().createMedia(params: params, context: nil)
        }
    }
    #endif
}
