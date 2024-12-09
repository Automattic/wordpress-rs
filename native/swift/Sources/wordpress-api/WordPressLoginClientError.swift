import Foundation

#if canImport(WordPressAPIInternal)
import WordPressAPIInternal
#endif

#if os(Linux)
import FoundationNetworking
#endif

public enum WordPressLoginClientError: Swift.Error {
    case invalidSiteAddress
    case missingLoginUrl
    case authenticationError(OAuthResponseUrlError)
    case invalidApplicationPasswordCallback
    case cancelled
    case unknown(Swift.Error)
}
