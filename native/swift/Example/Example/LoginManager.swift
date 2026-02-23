import Foundation
import WordPressAPI
import AuthenticationServices

@MainActor
final class LoginManager: ObservableObject {

    private let accountsRoot = URL.applicationSupportDirectory
    private let accountStore: AccountRepository

    private let wpcomClientId = ProcessInfo.processInfo.environment["WPCOM_CLIENT_ID"] ?? ""
    private let wpcomClientSecret = ProcessInfo.processInfo.environment["WPCOM_CLIENT_SECRET"] ?? ""

    public var wpcomLoginUrl: URL {
        URL(string: "https://public-api.wordpress.com/oauth2/authorize")!.appending(queryItems: [
            URLQueryItem(name: "redirect_uri", value: "x-wordpress-app://oauth2-callback"),
            URLQueryItem(name: "client_id", value: wpcomClientId),
            URLQueryItem(name: "client_secret", value: wpcomClientSecret),
            URLQueryItem(name: "response_type", value: "code")
        ])
    }

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

    private var wpComLoginTask: Task<Void, Never>?

    init() throws {
        #if DEBUG && os(macOS) // Avoids unlocking the keychain after every build during development on Mac devices
        let keyFile = accountsRoot.appendingPathComponent("keyfile.dat") // not a security risk, no key here
        let transformer = try SecureEnclavePasswordTransformer(keyFile: keyFile)
        #else
        let transformer = try SecureEnclavePasswordTransformer()
        #endif

        self.accountStore = try AccountRepository(
            rootPath: accountsRoot.path(percentEncoded: false),
            passwordTransformer: transformer
        )
        self.isLoggedIn = accountStore.hasSelfHostedAccount()
        self.isLoggedInToWpCom = accountStore.hasWpComAccount()
    }

    public func hasStoredLoginCredentials() -> Bool {
        return accountStore.hasSelfHostedAccount()
    }

    public func setLoginCredentials(to newValue: WpApiApplicationPasswordDetails, apiRootURL: URL) async throws {
        _ = try accountStore.store(account: .selfHostedSite(
            id: 42,
            domain: newValue.siteUrl,
            username: newValue.userLogin,
            password: newValue.password,
            siteApiRoot: apiRootURL.absoluteString
        ))

        isLoggedIn = true
    }

    public func getApiRootUrl() throws  -> String? {
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
            case .selfHostedSite(id: _, domain: _, let username, let password, siteApiRoot: _):
            return WpAuthentication(username: username, password: password)
            case .wpCom(id: _, username: _, token: _, siteApiRoot: _):
            preconditionFailure("This should never happen")
        }
    }

    public func setWpComLoginCredentials(to newValue: String) throws {
        _ = try self.accountStore.store(account: .wpCom(
            id: 42,
            username: "",
            token: newValue,
            siteApiRoot: "")
        )

        self.objectWillChange.send()
        self.isLoggedInToWpCom = true
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
