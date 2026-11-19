import Foundation
import WordPressAPIInternal
import WordPressApiCache

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

    /// The Reader's streams of posts.
    public var readerPosts: ReaderPostsRequestExecutor {
        internalClient.readerPosts()
    }

    /// One Reader post at a time: a site's post, a feed's post, a post found by its slug or its
    /// address, and the posts related to a post.
    public var readerPost: ReaderPostRequestExecutor {
        internalClient.readerPost()
    }

    public var readerSaved: ReaderSavedRequestExecutor {
        internalClient.readerSaved()
    }

    /// Marks posts as seen or unseen. Needs an account.
    public var readerSeen: ReaderSeenRequestExecutor {
        internalClient.readerSeen()
    }

    /// The sites the user blocked, and blocking and unblocking one. Needs an account.
    public var readerBlocks: ReaderBlocksRequestExecutor {
        internalClient.readerBlocks()
    }

    /// The sites and feeds the user is subscribed to in the Reader.
    public var readerSubscriptions: ReaderSubscriptionsRequestExecutor {
        internalClient.readerSubscriptions()
    }

    /// The Reader menu, a tag's info, and the interests a new reader picks from.
    public var readerTags: ReaderTagsRequestExecutor {
        internalClient.readerTags()
    }

    /// The users who liked a post or a comment.
    public var readerLikes: ReaderLikesRequestExecutor {
        internalClient.readerLikes()
    }

    /// A post's comments, and the user's subscription to them.
    public var readerComments: ReaderCommentsRequestExecutor {
        internalClient.readerComments()
    }

    /// Creates the account's saved Reader posts service. Prefer it over `readerSaved` for anything a
    /// user sees: a save or unsave shows straight away and survives a relaunch, and one the server
    /// hasn't got yet is sent by a later call.
    ///
    /// What it knows is kept in `cache` under `userID`. The cache's migrations must have run. Call
    /// `sync()` at launch, when the app comes to the foreground, and when the device comes back
    /// online. Call `WordPressApiCache.removeWordpressComAccount(userId:)` on sign-out to delete it.
    ///
    /// Create one per account and share it: it sends one request at a time, and a second instance
    /// would send alongside it.
    public func makeReaderSavedService(
        source: ReaderSavedSource,
        cache: WordPressApiCache,
        userID: WpComUserId
    ) throws -> ReaderSavedService {
        try ReaderSavedService(delegate: delegate, source: source, cache: cache.cache, userId: userID)
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
