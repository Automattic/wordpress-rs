import Foundation
import WordPressApiCache
@preconcurrency import WordPressAPIInternal

#if os(Linux)
import FoundationNetworking
#endif

#if canImport(Combine)
import Combine
#endif

public final class WordPressAPI: Sendable {

    enum Errors: Error {
        case unableToParseResponse
    }

    private let apiUrlResolver: ApiUrlResolver
    private let urlSession: URLSession

    let requestExecutor: SafeRequestExecutor
    private let apiClientDelegate: WpApiClientDelegate
    package let requestBuilder: UniffiWpApiClient

    public convenience init(
        urlSession: URLSession,
        notifyingDelegate: URLSessionTaskDelegate? = nil,
        apiRootUrl: ParsedUrl,
        authentication: WpAuthentication,
        middlewarePipeline: MiddlewarePipeline = .default,
        appNotifier: WpAppNotifier? = nil
    ) {
        self.init(
            apiUrlResolver: WpOrgSiteApiUrlResolver(apiRootUrl: apiRootUrl),
            authenticationProvider: .staticWithAuth(auth: authentication),
            executor: WpRequestExecutor(urlSession: urlSession, notifyingDelegate: notifyingDelegate),
            middlewarePipeline: middlewarePipeline,
            appNotifier: appNotifier
        )
    }

    public convenience init(
        urlSession: URLSession,
        apiRootUrl: ParsedUrl,
        authenticationProvider: WpAuthenticationProvider,
        middlewarePipeline: MiddlewarePipeline = .default,
        appNotifier: WpAppNotifier? = nil
    ) {
        self.init(
            apiUrlResolver: WpOrgSiteApiUrlResolver(apiRootUrl: apiRootUrl),
            authenticationProvider: authenticationProvider,
            executor: WpRequestExecutor(urlSession: urlSession),
            middlewarePipeline: middlewarePipeline,
            appNotifier: appNotifier
        )
    }

    public convenience init(
        urlSession: URLSession,
        notifyingDelegate: URLSessionTaskDelegate? = nil,
        apiUrlResolver: ApiUrlResolver,
        authenticationProvider: WpAuthenticationProvider,
        middlewarePipeline: MiddlewarePipeline = .default,
        appNotifier: WpAppNotifier? = nil
    ) {
        self.init(
            apiUrlResolver: apiUrlResolver,
            authenticationProvider: authenticationProvider,
            executor: WpRequestExecutor(urlSession: urlSession, notifyingDelegate: notifyingDelegate),
            middlewarePipeline: middlewarePipeline,
            appNotifier: appNotifier
        )
    }

    public convenience init(
        urlSession: URLSession,
        notifyingDelegate: URLSessionTaskDelegate? = nil,
        siteUrl: String,
        apiRootUrl: ParsedUrl,
        username: String,
        password: String,
        middlewarePipeline: MiddlewarePipeline = .default,
        appNotifier: WpAppNotifier? = nil
    ) {
        let executor = WpRequestExecutor(urlSession: urlSession, notifyingDelegate: notifyingDelegate)
        let provider = CookiesNonceAuthenticationProvider.withSiteUrl(
            url: siteUrl,
            username: username,
            password: password,
            requestExecutor: executor
        )
        self.init(
            apiUrlResolver: WpOrgSiteApiUrlResolver(apiRootUrl: apiRootUrl),
            authenticationProvider: .dynamic(dynamicAuthenticationProvider: provider),
            executor: executor,
            middlewarePipeline: middlewarePipeline,
            appNotifier: appNotifier
        )
    }

    public convenience init(
        urlSession: URLSession,
        notifyingDelegate: URLSessionTaskDelegate? = nil,
        details: AutoDiscoveryAttemptSuccess,
        username: String,
        password: String,
        middlewarePipeline: MiddlewarePipeline = .default,
        appNotifier: WpAppNotifier? = nil
    ) {
        let executor = WpRequestExecutor(urlSession: urlSession, notifyingDelegate: notifyingDelegate)
        let provider = CookiesNonceAuthenticationProvider(
            username: username,
            password: password,
            details: details,
            requestExecutor: executor
        )
        self.init(
            apiUrlResolver: WpOrgSiteApiUrlResolver(apiRootUrl: details.apiRootUrl),
            authenticationProvider: .dynamic(dynamicAuthenticationProvider: provider),
            executor: executor,
            middlewarePipeline: middlewarePipeline,
            appNotifier: appNotifier
        )
    }

    init(
        urlSession: URLSession = .shared,
        apiUrlResolver: ApiUrlResolver,
        authenticationProvider: WpAuthenticationProvider,
        executor: SafeRequestExecutor,
        middlewarePipeline: MiddlewarePipeline,
        appNotifier: WpAppNotifier?
    ) {
        self.urlSession = urlSession
        self.apiUrlResolver = apiUrlResolver
        self.apiClientDelegate = WpApiClientDelegate(
            authProvider: authenticationProvider,
            requestExecutor: executor,
            middlewarePipeline: middlewarePipeline,
            appNotifier: appNotifier ?? EmptyAppNotifier()
        )
        self.requestBuilder = UniffiWpApiClient(
            apiUrlResolver: self.apiUrlResolver,
            delegate: self.apiClientDelegate
        )
        self.requestExecutor = executor
    }

    public func createSelfHostedService(cache: WordPressApiCache) throws -> WpService {
        let apiURL = apiUrlResolver.resolve(namespace: "", endpointSegments: []).asURL()
        return try WpService.selfHosted(
            siteUrl: apiURL.deletingLastPathComponent().absoluteString,
            apiRoot: apiURL.absoluteString,
            delegate: apiClientDelegate,
            cache: cache.cache
        )
    }

    public func createWordPressComService(siteId: WpComSiteId, cache: WordPressApiCache) throws -> WpService {
        try WpService.wordpressCom(siteId: siteId, delegate: apiClientDelegate, cache: cache.cache)
    }

    public var users: UsersRequestExecutor {
        self.requestBuilder.users()
    }

    public var plugins: PluginsRequestExecutor {
        self.requestBuilder.plugins()
    }

    public var apiRoot: ApiRootRequestExecutor {
        self.requestBuilder.apiRoot()
    }

    public var applicationPasswords: ApplicationPasswordsRequestExecutor {
        self.requestBuilder.applicationPasswords()
    }

    public var siteHealthTests: WpSiteHealthTestsRequestExecutor {
        self.requestBuilder.wpSiteHealthTests()
    }

    public var postTypes: PostTypesRequestExecutor {
        self.requestBuilder.postTypes()
    }

    public var posts: PostsRequestExecutor {
        self.requestBuilder.posts()
    }

    public var postStatuses: PostStatusesRequestExecutor {
        self.requestBuilder.postStatuses()
    }

    public var revisions: RevisionsRequestExecutor {
        self.requestBuilder.postRevisions()
    }

    public var comments: CommentsRequestExecutor {
        self.requestBuilder.comments()
    }

    public var media: MediaRequestExecutor {
        self.requestBuilder.media()
    }

    public var siteSettings: SiteSettingsRequestExecutor {
        self.requestBuilder.siteSettings()
    }

    public var taxonomies: TaxonomiesRequestExecutor {
        self.requestBuilder.taxonomies()
    }

    public var terms: TermsRequestExecutor {
        self.requestBuilder.terms()
    }

    public var themes: ThemesRequestExecutor {
        self.requestBuilder.themes()
    }

    public var blockEditor: WpBlockEditorRequestExecutor {
        self.requestBuilder.wpBlockEditor()
    }

    public var navigations: NavigationRequestExecutor {
        self.requestBuilder.navigations()
    }

    public var navMenus: NavMenusRequestExecutor {
        self.requestBuilder.navMenus()
    }

    public var navMenuItems: NavMenuItemsRequestExecutor {
        self.requestBuilder.navMenuItems()
    }

    public var navMenuAutosaves: NavMenuItemAutosavesRequestExecutor {
        self.requestBuilder.navMenuItemAutosaves()
    }

    public var menuLocations: MenuLocationsRequestExecutor {
        self.requestBuilder.menuLocations()
    }

#if PROGRESS_REPORTING_ENABLED
    /// Track the progress of the given HTTP API calls in the `apiCall` closure.
    ///
    /// Note: pass the `RequestContext` parameter in `apiCall` to one and only one HTTP API call.
    public func fulfill<R: Sendable>(
        progress: Progress,
        withApiCall apiCall: sending @escaping (RequestContext) async throws -> R
    ) async throws -> R {
        precondition(progress.completedUnitCount == 0 && progress.totalUnitCount > 0)
        precondition(progress.cancellationHandler == nil)

        let context = RequestContext()

        let uploadTask = Task {
            try await withTaskCancellationHandler {
                try await apiCall(context)
            } onCancel: {
                requestExecutor.cancel(context: context)
            }
        }

        let progressObserver = Task {
            for await task in requestExecutor.progresses(for: context).values {
                // For one single request call, the Rust layer should send HTTP requests sequentially.
                // For example, the retry mechanism in the Rust layer only send the retry call when the initial
                // call fails.
                //
                // Since we can't know how many HTTP requests will be sent, the best we can do is make the `progress`
                // starts from zero to complete for each HTTP request.
                progress.completedUnitCount = 0
                progress.addChild(task, withPendingUnitCount: progress.totalUnitCount)
            }
        }

        progress.cancellationHandler = {
            uploadTask.cancel()
            progressObserver.cancel()
        }

        defer { progressObserver.cancel() }

        return try await withTaskCancellationHandler {
            try await uploadTask.value
        } onCancel: {
            progress.cancel()
        }
    }

    public func uploadMedia(
        params: MediaCreateParams,
        fulfilling progress: Progress
    ) async throws -> MediaRequestCreateResponse {
        try await fulfill(progress: progress) { [media] in
            try await media.createCancellation(params: params, context: $0)
        }
    }
#endif

    enum ParseError: Error {
        case invalidUrl
        case invalidHtml
    }
}

public extension WpNetworkHeaderMap {
    func toFlatMap() -> [String: String] {
        self.toMap().mapValues { $0.joined(separator: ",") }
    }
}

public extension WpNetworkRequest {

    #if DEBUG
    func debugPrint() {
        print("\(method().rawValue) \(self.url())")
        for (name, value) in self.headerMap().toMap() {
            print("\(name): \(value)")
        }

        print("")

        if let bodyString = self.bodyAsString() {
            print(bodyString)
        }
    }
    #endif
}

extension Result {
    @inlinable public func tryMap<NewSuccess>(
            _ transform: (Success) throws -> NewSuccess
    ) -> Result<NewSuccess, any Error> {
        switch self {
        case .success(let success):
            do {
                return .success(try transform(success))
            } catch let err {
                return .failure(err)
            }

        case .failure(let error): return .failure(error)
        }
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

// Note: Everything below this line should be moved into the Rust layer
public extension WpAuthentication {
    init(username: String, password: String) {
        self = .authorizationHeader(token: "\(username):\(password)".data(using: .utf8)!.base64EncodedString())
    }
}

extension RequestMethod {
    var rawValue: String {
        switch self {
        case .get: "GET"
        case .post: "POST"
        case .put: "PUT"
        case .delete: "DELETE"
        case .head: "HEAD"
        }
    }
}

public extension ParsedUrl {
    static func from(url: URL) throws -> ParsedUrl {
        try parse(input: url.absoluteString)
    }
}
