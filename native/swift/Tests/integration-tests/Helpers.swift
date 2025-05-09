import Foundation
import WordPressAPI

func restoreTestServer() async throws {
    _ = try await URLSession(configuration: .ephemeral)
        .data(from: URL(string: "http://localhost:4000/restore?db=true&plugins=true")!)
}

extension TestCredentials {
    var apiRootURL: ParsedUrl {
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
