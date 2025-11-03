import Foundation
import WordPressAPI
import WordPressAPIInternal
import Testing

@Suite(.serialized)
struct NonceAuthenticationTests {

    @Test
    func success() async throws {
        let credentials = TestCredentials.instance()
        let client = WordPressLoginClient(urlSession: .init(configuration: .ephemeral))
        let details = try await client.details(ofSite: credentials.siteUrl)
        let api = try await client.authenticateTemporarily(
            username: credentials.adminUsername,
            password: credentials.adminAccountPassword,
            details: details
        )
        let loggedIn = try await api.users.retrieveMeWithEditContext().data.username
        #expect(loggedIn == credentials.adminUsername)
    }

    @Test
    func signInWithADifferentUser() async throws {
        let credentials = TestCredentials.instance()
        let client = WordPressLoginClient(urlSession: .init(configuration: .ephemeral))
        let details = try await client.details(ofSite: credentials.siteUrl)

        // Given the URLSession is already signed in with the admin account.
        let api = try await client.authenticateTemporarily(
            username: credentials.adminUsername,
            password: credentials.adminAccountPassword,
            details: details
        )
        let loggedIn = try await api.users.retrieveMeWithEditContext().data.username
        #expect(loggedIn == credentials.adminUsername)

        // When sign in with another account, an error should be returned.
        await #expect(throws: NonceRetrievalError.AlreadyLoggedIn(username: credentials.adminUsername)) {
            _ = try await client.authenticateTemporarily(
                username: credentials.authorUsername,
                password: credentials.authorPassword,
                details: details
            )
        }
    }

}
