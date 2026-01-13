import Foundation
import WordPressAPIInternal

public final class WPComApiClient: Sendable {

    public struct OAuth2 {
        public static func buildTokenRequestUrl(
            clientId: UInt64,
            redirectUri: URL,
            scope: [String],
            state: String = UUID().uuidString,
            blog: UInt64? = nil
        ) -> URL {
            WordPressAPIInternal.buildTokenRequestUrl(
                clientId: clientId,
                redirectUri: redirectUri.absoluteString,
                scope: scope.joined(separator: ","),
                state: state,
                blog: blog
            ).asURL()
        }

        public static func parseTokenResponse(url: URL) throws -> AuthorizationCodeExtractionResult {
            try WordPressAPIInternal.parseAuthorizationUrl(response: url.absoluteString)
        }
    }

    private let internalClient: WordPressAPIInternal.UniffiWpComApiClient
    private let delegate: WpApiClientDelegate

    public init(delegate: WpApiClientDelegate) {
        self.delegate = delegate // We need to retain this ourselves because it's passed to a Rust object
        self.internalClient = UniffiWpComApiClient(delegate: delegate)
    }

    public convenience init(
        urlSession: URLSession = .shared,
        authenticationProvider: WpAuthenticationProvider,
        middlewarePipeline: WpApiMiddlewarePipeline = .default,
        appNotifier: WpAppNotifier? = nil,
        notifyingDelegate: URLSessionTaskDelegate? = nil
    ) {

        let delegate = WpApiClientDelegate(
            authProvider: authenticationProvider,
            requestExecutor: WpRequestExecutor(urlSession: urlSession, notifyingDelegate: notifyingDelegate),
            middlewarePipeline: middlewarePipeline,
            appNotifier: appNotifier ?? EmptyAppNotifier()
        )

        self.init(delegate: delegate)
    }

    public convenience init(
        urlSession: URLSession = .shared,
        authentication: WpAuthentication,
        middlewarePipeline: WpApiMiddlewarePipeline = .default,
        appNotifier: WpAppNotifier? = nil,
        notifyingDelegate: URLSessionTaskDelegate? = nil
    ) {
        self.init(
            urlSession: urlSession,
            authenticationProvider: .staticWithAuth(auth: authentication),
            middlewarePipeline: middlewarePipeline,
            appNotifier: appNotifier,
            notifyingDelegate: notifyingDelegate
        )
    }

    // swiftlint:disable:next identifier_name
    public var me: MeRequestExecutor {
        internalClient.me()
    }

    public var oauth2: Oauth2RequestExecutor {
        internalClient.oauth2()
    }

    public var jetpackConnection: JetpackConnectionRequestExecutor {
        internalClient.jetpackConnection()
    }

    public var subscribers: SubscribersRequestExecutor {
        internalClient.subscribers()
    }

    public var supportBots: SupportBotsRequestExecutor {
        internalClient.supportBots()
    }

    public var supportEligibility: SupportEligibilityRequestExecutor {
        internalClient.supportEligibility()
    }

    public var supportTickets: SupportTicketsRequestExecutor {
        internalClient.supportTickets()
    }
}
