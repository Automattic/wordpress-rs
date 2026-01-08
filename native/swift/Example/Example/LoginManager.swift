import Foundation
import WordPressAPI

@MainActor
class LoginManager: NSObject, ObservableObject {


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

    override init() {
        super.init()
        self.isLoggedIn = hasStoredLoginCredentials()
    }

    public func getApiRootUrl() -> String? {
        guard let string = UserDefaults.standard.string(forKey: "api-root-url") else {
            return nil
        }

        return string
    }

    func setApiRootUrl(to newValue: String) {
        UserDefaults.standard.setValue(newValue, forKey: "api-root-url")
    }

    public func hasStoredLoginCredentials() -> Bool {
        guard let siteUrl = getApiRootUrl() else {
            return false
        }

        do {
            return try Keychain.hasCredentials(for: siteUrl)
        } catch {
            return false
        }
    }

    public func setLoginCredentials(to newValue: WpApiApplicationPasswordDetails, apiRootURL: URL) async throws {
        setApiRootUrl(to: apiRootURL.absoluteString)
        try Keychain.store(username: newValue.userLogin, password: newValue.password, for: apiRootURL.absoluteString)

        isLoggedIn = true
    }

    public func getLoginCredentials() throws -> WpAuthentication? {

        guard
            let siteUrl = getApiRootUrl(),
            let keychainItem = try Keychain.lookup(for: siteUrl)
        else {
            return nil
        }

        return keychainItem
    }

    public func setWpComLoginCredentials(to newValue: String) async throws {
        try Keychain.storeForWpCom(token: newValue)
        isLoggedInToWpCom = true
    }

    public func getWpComLoginCredentials() throws -> WpAuthentication? {
        guard let token = try Keychain.lookup(for: "wordpress.com") else {
            return nil
        }

        return token
    }

    public func logout() async {
        UserDefaults.standard.removeObject(forKey: "api-root-url")

        await MainActor.run {
            self.objectWillChange.send()
            self.isLoggedIn = false
        }
    }
}

// MARK: Keychain Wrapper
struct Keychain {
    enum KeychainError: Error {
        case noPassword
        case invalidPassword
        case unexpectedPasswordData
        case unhandledError(status: OSStatus)
    }

    static func store(username: String, password: String, for server: String) throws {
        guard let utf8Password = password.data(using: .utf8) else {
            throw KeychainError.invalidPassword
        }

        if try lookup(for: server) != nil {
            let deletionStatus = SecItemDelete([
                kSecClass as String: kSecClassInternetPassword,
                kSecAttrServer as String: server as CFString
            ] as CFDictionary)

            guard deletionStatus == errSecSuccess else { throw KeychainError.unhandledError(status: deletionStatus) }
        }

        let status = SecItemAdd([
            kSecClass as String: kSecClassInternetPassword,
            kSecAttrAccount as String: username as CFString,
            kSecAttrServer as String: server as CFString,
            kSecValueData as String: utf8Password as CFData
        ] as CFDictionary, nil)
        guard status == errSecSuccess else { throw KeychainError.unhandledError(status: status) }
    }

    static func storeForWpCom(token: String) throws {
        guard let utf8Token = token.data(using: .utf8) else {
            throw KeychainError.invalidPassword
        }

        if try lookup(for: "wordpress.com") != nil {
            let deletionStatus = SecItemDelete([
                kSecClass as String: kSecClassInternetPassword,
                kSecAttrServer as String: "wordpress.com" as CFString
            ] as CFDictionary)

            guard deletionStatus == errSecSuccess else { throw KeychainError.unhandledError(status: deletionStatus) }
        }

        let status = SecItemAdd([
            kSecClass as String: kSecClassInternetPassword,
            kSecAttrAccount as String: "username" as CFString,
            kSecAttrServer as String: "wordpress.com" as CFString,
            kSecValueData as String: utf8Token as CFData
        ] as CFDictionary, nil)
        guard status == errSecSuccess else { throw KeychainError.unhandledError(status: status) }

    }

    static func lookup(for server: String) throws -> WpAuthentication? {
        let query: [String: Any] = [
            kSecClass as String: kSecClassInternetPassword,
            kSecAttrServer as String: server,
            kSecMatchLimit as String: kSecMatchLimitOne,
            kSecReturnAttributes as String: true,
            kSecReturnData as String: true
        ]

        var item: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &item)

        guard status != errSecItemNotFound else {
            return nil
        }

        guard status == errSecSuccess else {
            throw KeychainError.unhandledError(status: status)
        }

        guard let existingItem = item as? [String: Any],
            let passwordData = existingItem[kSecValueData as String] as? Data,
            let password = String(data: passwordData, encoding: String.Encoding.utf8),
            let username = existingItem[kSecAttrAccount as String] as? String
        else {
            throw KeychainError.unexpectedPasswordData
        }

        return WpAuthentication(username: username, password: password)
    }

    static func lookupForWpcom() async throws -> WpAuthentication? {
        let query: [String: Any] = [
            kSecClass as String: kSecClassInternetPassword,
            kSecAttrServer as String: "wordpress.com",
            kSecMatchLimit as String: kSecMatchLimitOne,
            kSecReturnAttributes as String: true,
            kSecReturnData as String: true
        ]

        var item: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &item)

        guard status != errSecItemNotFound else {
            return nil
        }

        guard status == errSecSuccess else {
            throw KeychainError.unhandledError(status: status)
        }

        guard let existingItem = item as? [String: Any],
            let passwordData = existingItem[kSecValueData as String] as? Data,
            let password = String(data: passwordData, encoding: String.Encoding.utf8)
        else {
            throw KeychainError.unexpectedPasswordData
        }

        return WpAuthentication.bearer(token: password)
    }

    static func hasCredentials(for server: String) throws -> Bool {
        try lookup(for: server) != nil
    }
}
