import Foundation
import WordPressAPI
import AuthenticationServices
import SwiftUI

struct WPComOAuthCredentials {
    let clientId: UInt64
    let clientSecret: String

    static func load() -> WPComOAuthCredentials? {
        fromCredentialsFile()
    }

    private static func fromCredentialsFile() -> WPComOAuthCredentials? {
        guard let credentialsURL = Bundle.main.url(forResource: "wp_com_test_credentials", withExtension: "json") else {
            return nil
        }

        guard
            let data = try? Data(contentsOf: credentialsURL),
            let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
            let clientId = json["client_id"] as? UInt64,
            clientId != 0,
            let clientSecret = json["client_secret"] as? String,
            !clientSecret.isEmpty
        else {
            return nil
        }

        return WPComOAuthCredentials(clientId: clientId, clientSecret: clientSecret)
    }

}

@MainActor
final class LoginManager: ObservableObject {

    private let accountsRoot = URL.applicationSupportDirectory
    private let accountStore: AccountRepository

    private let wpComOAuthCredentials = WPComOAuthCredentials.load()
    let wpComOAuthConfiguration: OAuth2Configuration?

    @Published
    var isLoggedIn: Bool = false

    @Published
    var isLoggedInToWpCom: Bool = false

    private var selfHostedAccount: Account? {
        get throws {
            try self.accountStore.all().first { $0.isSelfHosted() }
        }
    }

    private var wpComAccount: Account? {
        get throws {
            try self.accountStore.all().first { $0.isWpCom() }
        }
    }

    let oauthRegistry = OAuth2ConfigurationStore()

    private var wpComLoginTask: Task<Void, Never>?

    init() throws {
        #if DEBUG && os(macOS) // Avoids unlocking the keychain after every build during development on Mac devices
        let keyFile = accountsRoot.appendingPathComponent("keyfile.dat") // not a security risk, no key here
        let transformer = try SecureEnclavePasswordTransformer(keyFile: keyFile)
        #else
        let transformer = try SecureEnclavePasswordTransformer(applicationName: "wordpress-rs-example")
        #endif

        if let credentials = wpComOAuthCredentials {
            let configuration = WPComApiClient.oauthConfiguration(
                clientId: credentials.clientId,
                clientSecret: credentials.clientSecret,
                redirectUri: "x-wordpress-app://oauth2-callback",
                scope: [.auth, .sites, .media, .posts]
            )

            self.wpComOAuthConfiguration = configuration
            self.oauthRegistry.addConfiguration(config: configuration)
        } else {
            self.wpComOAuthConfiguration = nil
        }

        self.accountStore = try AccountRepository(
            rootPath: accountsRoot.path(percentEncoded: false),
            passwordTransformer: transformer
        )
        self.isLoggedIn = accountStore.hasSelfHostedAccount()
        self.isLoggedInToWpCom = accountStore.hasWpComAccount()

        try self.signInToWpComFromTokenFileIfNeeded()
    }

    /// A file in the app's own `Library` folder that holds a WordPress.com bearer token, used to sign a Simulator
    /// in without the OAuth flow. `scripts/example-app-signin.sh` writes it from standard input, so that the token
    /// is on no command line, where `ps` would show it.
    static let tokenFile = URL.libraryDirectory.appending(path: "wpcom-token")

    /// Stores the token in `tokenFile` as the WordPress.com account, unless one is already signed in, and deletes
    /// the file.
    ///
    /// Only called from `init`, and the file is deleted first, so it runs at most once per sign-in — signing out
    /// doesn't sign the account back in.
    private func signInToWpComFromTokenFileIfNeeded() throws {
        let file = Self.tokenFile
        guard FileManager.default.fileExists(atPath: file.path(percentEncoded: false)) else {
            return
        }

        let token = try String(contentsOf: file, encoding: .utf8).trimmingCharacters(in: .whitespacesAndNewlines)
        try FileManager.default.removeItem(at: file)
        guard !token.isEmpty, !isLoggedInToWpCom else {
            return
        }

        try setWpComLoginCredentials(to: token)
    }

    public func logInToWpCom(
        configuration: OAuth2Configuration,
        webAuthenticationSession: WebAuthenticationSession,
        blogId: WpComSiteIdentifier? = nil
    ) async throws {

        let state = UUID().uuidString

        let url = configuration.buildTokenRequestUrl(
            state: state,
            blog: blogId
        )

        let callbackUrl = try await webAuthenticationSession.authenticate(
            using: url.asURL(),
            callbackURLScheme: "x-wordpress-app"
        )

        let tokenResponse = try configuration.parseTokenResponse(
            url: callbackUrl.absoluteString,
            expectedState: state
        )

        let client = WPComApiClient(
            authentication: .none,
            middlewarePipeline: MiddlewarePipeline(middlewares: [DebugMiddleware()])
        )

        let requestParams = configuration.buildTokenRequestParameters(code: tokenResponse.code)

        let response = try await client.oauth2.requestToken(params: requestParams)
        try self.setWpComLoginCredentials(to: response.data.accessToken)
    }

    public func hasStoredLoginCredentials() -> Bool {
        accountStore.hasSelfHostedAccount()
    }

    public func setLoginCredentials(to newValue: WpApiApplicationPasswordDetails, apiRootURL: URL) throws {
        _ = try accountStore.store(
            account: .selfHostedSite(
                id: 42,
                domain: newValue.siteUrl,
                username: newValue.userLogin,
                password: newValue.password,
                siteApiRoot: apiRootURL.absoluteString
            )
        )

        isLoggedIn = true
    }

    public func getApiRootUrl() throws -> String? {
        guard let account = try self.selfHostedAccount else {
            return nil
        }

        switch account {
        case .selfHostedSite(id: _, domain: _, username: _, password: _, let siteApiRoot):
            return siteApiRoot
        case .wpCom:
            preconditionFailure("This should never happen")
        }
    }

    public func getLoginCredentials() throws -> WpAuthentication? {
        guard let account = try self.accountStore.all().first(where: { $0.isSelfHosted() }) else {
            return nil
        }

        switch account {
        case .selfHostedSite(_, _, let username, let password, _):
            return WpAuthentication(username: username, password: password)
        case .wpCom:
            preconditionFailure("This should never happen")
        }
    }

    public func setWpComLoginCredentials(to newValue: String) throws {
        _ = try self.accountStore.store(
            account: .wpCom(
                id: 42,
                username: "",
                token: newValue,
                siteApiRoot: ""
            )
        )

        self.objectWillChange.send()
        self.isLoggedInToWpCom = true
    }

    /// The signed-in WordPress.com account's ID in the account store, which isn't its user ID.
    public func getWpComAccountID() throws -> UInt64? {
        try self.wpComAccount?.id()
    }

    public func getWpComLoginCredentials() throws -> WpAuthentication? {
        switch try self.wpComAccount {
        case .wpCom(_, _, let password, _):
            return WpAuthentication.bearer(token: password)
        case .selfHostedSite:
            preconditionFailure("This should never happen")
        case .none:
            return nil
        }
    }

    @MainActor
    public func logout() throws {
        guard let account = try self.selfHostedAccount else {
            return
        }

        try self.accountStore.remove(id: account.id())

        self.objectWillChange.send()
        self.isLoggedIn = false
    }

    @MainActor
    public func logoutWpCom() throws {
        guard let account = try self.wpComAccount else {
            return
        }

        try self.accountStore.remove(id: account.id())

        self.objectWillChange.send()
        self.isLoggedInToWpCom = false
    }
}
