import Foundation
@preconcurrency import WordPressAPIInternal

#if os(Linux)
import FoundationNetworking
#endif

#if canImport(Combine)
import Combine
#endif

#if canImport(UniformTypeIdentifiers)
import UniformTypeIdentifiers
#endif

public actor WordPressAPI {

    enum Errors: Error {
        case unableToParseResponse
    }

    private let apiUrlResolver: ApiUrlResolver
    let requestExecutor: SafeRequestExecutor
    private let apiClientDelegate: WpApiClientDelegate
    package let requestBuilder: UniffiWpApiClient

    public init(
        urlSession: URLSession,
        apiRootUrl: ParsedUrl,
        authentication: WpAuthentication,
        middlewarePipeline: MiddlewarePipeline = .default,
        appNotifier: WpAppNotifier? = nil
    ) {
        self.init(
            apiUrlResolver: WpOrgSiteApiUrlResolver(apiRootUrl: apiRootUrl),
            authenticationProvider: .staticWithAuth(auth: authentication),
            executor: WpRequestExecutor(urlSession: urlSession),
            middlewarePipeline: middlewarePipeline,
            appNotifier: appNotifier
        )
    }

    public init(
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

    public init(
        urlSession: URLSession,
        apiUrlResolver: ApiUrlResolver,
        authenticationProvider: WpAuthenticationProvider,
        middlewarePipeline: MiddlewarePipeline = .default,
        appNotifier: WpAppNotifier? = nil
    ) {
        self.init(
            apiUrlResolver: apiUrlResolver,
            authenticationProvider: authenticationProvider,
            executor: WpRequestExecutor(urlSession: urlSession),
            middlewarePipeline: middlewarePipeline,
            appNotifier: appNotifier
        )
    }

    init(
        apiUrlResolver: ApiUrlResolver,
        authenticationProvider: WpAuthenticationProvider,
        executor: SafeRequestExecutor,
        middlewarePipeline: MiddlewarePipeline,
        appNotifier: WpAppNotifier?
    ) {
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

    public var blockEditor: WpBlockEditorRequestExecutor {
        self.requestBuilder.wpBlockEditor()
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
    public func uploadMedia(
        params: MediaCreateParams,
        fromLocalFileURL localFileURL: URL,
        fulfilling progress: Progress,
        mimeType: String? = nil,
    ) async throws -> MediaRequestCreateResponse {
        precondition(localFileURL.isFileURL)
        precondition(progress.completedUnitCount == 0 && progress.totalUnitCount > 0)
        precondition(progress.cancellationHandler == nil)

        let requestId = WpUuid()

        let fileContentType: String
        if let mimeType {
            fileContentType = mimeType
        } else if let mimeType = UTType(filenameExtension: localFileURL.pathExtension)?.preferredMIMEType {
            fileContentType = mimeType
        } else {
            fileContentType = "application/octet-stream"
        }

        let cancellable = requestExecutor
            .progress(forRequestWithId: requestId.uuidString())
            .sink {
                progress.addChild($0, withPendingUnitCount: progress.totalUnitCount - progress.completedUnitCount)
            }
        defer {
            cancellable.cancel()
        }

        let uploadTask = Task {
            try await media.create(
                params: params,
                filePath: localFileURL.path,
                fileContentType: fileContentType,
                requestId: requestId
            )
        }

        progress.cancellationHandler = {
            uploadTask.cancel()
        }

        return try await withTaskCancellationHandler {
            try await uploadTask.value
        } onCancel: {
            // Please note: the async functions exported by uniffi-rs _do not_ support cancellation.
            // That means cancelling an API call like `Task { try await api.users.retrieveMe() }.cancel()`
            // does not cancel the underlying HTTP request sent by URLSession.
            //
            // The `progress.cancel()` in this particular function can cancel the HTTP request, because the
            // `progress` instance is the parent progress of `URLSessionTask.progress`, and cancelling a parent
            // progress automatically cancels their child progress, which is the `URLSessionTask` in this case.
            progress.cancel()
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
