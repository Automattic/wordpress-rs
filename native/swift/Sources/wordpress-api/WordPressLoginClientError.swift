import Foundation
import WordPressAPIInternal

#if os(Linux)
import FoundationNetworking
#endif

public enum WordPressLoginClientError: Swift.Error {
    case invalidSiteAddress(UrlDiscoveryError)
    case missingLoginUrl
    case authenticationError(OAuthResponseUrlError)
    case invalidApplicationPasswordCallback
    case cancelled
    case unknown(Swift.Error)

    func isAutodiscoveryError() -> Bool {
        guard case let .invalidSiteAddress(urlDiscoveryError) = self else {
            return false
        }

        guard case .UrlDiscoveryFailed = urlDiscoveryError else {
            return false
        }

        return true
    }

    var isFailedToFetchApiDetails: Bool {
        guard
            case let .invalidSiteAddress(urlDiscoveryError) = self,
            case .UrlDiscoveryFailed(let attempts) = urlDiscoveryError
        else {
            return false
        }

        return attempts.values.contains { state in
            return switch state {
            case .failure(let error): urlDiscoverErrorIsFetchApiDetailsFailed(error)
            default: false
            }
        }
    }

    var isFailedToFetchApiRoot: Bool {
        guard
            case let .invalidSiteAddress(urlDiscoveryError) = self,
            case .UrlDiscoveryFailed(let attempts) = urlDiscoveryError
        else {
            return false
        }

        return attempts.values.contains { state in
            if case let .failure(error) = state {
                return isFetchRootUrlFailedError(error)
            }

            return false
        }
    }

    private func urlDiscoverErrorIsFetchApiDetailsFailed(_ error: UrlDiscoveryAttemptError) -> Bool {
        return switch error {
        case .fetchApiDetailsFailed: true
        default: false
        }
    }

    private func isFetchRootUrlFailedError(_ error: UrlDiscoveryAttemptError) -> Bool {
        return switch error {
        case .fetchApiRootUrlFailed: true
        default: false
        }
    }
}
