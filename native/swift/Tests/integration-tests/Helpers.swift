import Foundation
import WordPressAPI

#if canImport(FoundationNetworking)
import FoundationNetworking
#endif

func restoreTestServer() async throws {
    #if os(Linux)
    // Integration tests are run in a Docker container, where the test site
    // hostname is 'wordpress'.
    let url = URL(string: "http://wordpress:4000/restore?db=true&plugins=true")!
    #else
    let url = URL(string: "http://localhost:4000/restore?db=true&plugins=true")!
    #endif
    _ = try await URLSession(configuration: .ephemeral).data(from: url)
}

extension TestCredentials {
    var apiRootURL: ParsedUrl {
        // swiftlint:disable:next force_try
        try! ParsedUrl.parse(input: siteUrl + "/wp-json")
    }

    var adminAuthentication: WpAuthentication {
        .init(username: adminUsername, password: adminPassword)
    }
}

extension WordPressAPI {
    static func admin() -> WordPressAPI {
        let credentials = TestCredentials.instance()
        return WordPressAPI(
            urlSession: .init(configuration: .ephemeral),
            apiRootUrl: credentials.apiRootURL,
            authentication: credentials.adminAuthentication
        )
    }
}
