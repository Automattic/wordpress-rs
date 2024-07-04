import Foundation
import WordPressAPI

class LoginManager: NSObject, ObservableObject {

    @Published
    var isLoggedIn: Bool = false

    override init() {
        super.init()
        self.isLoggedIn = hasStoredLoginCredentials()
    }

    public func getDefaultSiteUrl() -> String? {
        guard let string = UserDefaults.standard.string(forKey: "default-site-url") else {
            return nil
        }

        return string
    }

    func setDefaultSiteUrl(to newValue: String) {
        UserDefaults.standard.setValue(newValue, forKey: "default-site-url")
    }

    public func hasStoredLoginCredentials() -> Bool {
        guard let siteUrl = getDefaultSiteUrl() else {
            return false
        }

        do {
            return try Keychain.hasCredentials(for: siteUrl)
        } catch {
            return false
        }
    }

    public func setLoginCredentials(to newValue: WpApiApplicationPasswordDetails) async throws {
        setDefaultSiteUrl(to: newValue.siteUrl)
        try Keychain.store(username: newValue.userLogin, password: newValue.password, for: newValue.siteUrl)

        await MainActor.run {
            isLoggedIn = true
        }
    }

    public func getLoginCredentials() throws -> WpAuthentication? {

        guard
            let siteUrl = getDefaultSiteUrl(),
            let keychainItem = try Keychain.lookup(for: siteUrl)
        else {
            return nil
        }

        return keychainItem
    }

    public func logout() async {
        UserDefaults.standard.removeObject(forKey: "default-site-url")

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
                kSecAttrServer as String: server as CFString,
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

    static func hasCredentials(for server: String) throws -> Bool {
        try lookup(for: server) != nil
    }
}
