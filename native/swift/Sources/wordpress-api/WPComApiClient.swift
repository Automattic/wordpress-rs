import Foundation
import WordPressAPIInternal

#if os(Linux)
import FoundationNetworking
#endif

public final class WPComApiClient: Sendable {

    private let internalClient: WordPressAPIInternal.UniffiWpComApiClient
    private let delegate: WpApiClientDelegate

    public static func oauthConfiguration(
        clientId: UInt64,
        clientSecret: String,
        redirectUri: String,
        scope: [WpComOauthScope]
    ) -> OAuth2Configuration {
        wordpressComOauth2Configuration(
            clientId: clientId,
            clientSecret: clientSecret,
            redirectUri: redirectUri,
            scope: scope
        )
    }

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

    public var languages: LanguagesRequestExecutor {
        internalClient.languages()
    }

    public var oauth2: Oauth2RequestExecutor {
        internalClient.oauth2()
    }

    public var jetpackConnection: JetpackConnectionRequestExecutor {
        internalClient.jetpackConnection()
    }

    public var sites: SitesRequestExecutor {
        internalClient.sites()
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

    public var unifiedConversations: UnifiedConversationsRequestExecutor {
        internalClient.unifiedConversations()
    }

    public var statsVisits: StatsVisitsRequestExecutor {
        internalClient.statsVisits()
    }

    public var publicize: PublicizeRequestExecutor {
        internalClient.publicize()
    }

    public var meConnections: MeConnectionsRequestExecutor {
        internalClient.meConnections()
    }
}
