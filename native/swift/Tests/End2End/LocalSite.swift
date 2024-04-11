#if os(macOS) || os(Linux)

import XCTest
import wordpress_api

import wordpress_api_wrapper

let site = LocalSite()

final class LocalSite {

    enum Errors: Error {
        /// Run `make test-server` before running end to end tests.
        case testServerNotRunning(underlyingError: Error)
        /// `localhost:80` is not wordpress site. Make sure to run `make test-server` before running end to end tests.
        case notWordPressSite
        /// Can't read the test credential file for the local test site.
        case testCredentialNotFound(underlyingError: Error)
    }

    let siteURL = URL(string: "http://localhost")!
    let currentUserID: SparseUser.ID = 1

    private let username = "test@example.com"

    private var _api: WordPressAPI?

    /// Get an authenticationed API client for the admin user.
    var api: WordPressAPI {
        get async throws {
            if _api == nil {
                _api = try await createAPIClient()
            }
            return _api!
        }
    }

    private func createAPIClient() async throws -> WordPressAPI {
        try await ensureTestServerRunning()
        let password = try readPassword()

        return WordPressAPI(
            urlSession: .shared,
            baseUrl: siteURL,
            authenticationStategy: .init(username: username, password: password)
        )
    }

    private func ensureTestServerRunning() async throws {
        let api = WordPressAPI(urlSession: .shared, baseUrl: siteURL, authenticationStategy: .none)
        let response: WpNetworkResponse
        do {
            let request = WpNetworkRequest(
                method: .get, url: siteURL.appendingPathComponent("/wp-json").absoluteString,
                headerMap: [:], body: nil)
            response = try await api.perform(request: request)
        } catch {
            throw Errors.testServerNotRunning(underlyingError: error)
        }

        if response.statusCode != 200 {
            throw Errors.notWordPressSite
        }
    }

    private func readPassword() throws -> String {
        #if os(Linux)
        let file = URL(fileURLWithPath: #filePath)
        #else
        let file = URL(filePath: #filePath)
        #endif
        let testCredentialFile = URL(string: "../../../../test_credentials", relativeTo: file)!
            .absoluteURL
        let content: String
        do {
            content = try String(contentsOf: testCredentialFile)
        } catch {
            throw Errors.testCredentialNotFound(underlyingError: error)
        }

        return content.trimmingCharacters(in: .newlines)
    }

}

// MARK: - Helpers

extension LocalSite {

    func createUser(password: String? = nil) async throws -> SparseUser.Edit {
        let uuid = UUID().uuidString
        return try await api.users.create(
            using: .init(
                username: uuid, email: "\(uuid)@swift-test.com", password: password ?? "badpass",
                name: nil, firstName: "End2End", lastName: nil, url: "http://example.com",
                description: nil, locale: nil, nickname: nil, slug: nil, roles: ["subscriber"], meta: nil)
        )
    }

}

#endif
