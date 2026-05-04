import Foundation
import WordPressAPI

#if canImport(FoundationNetworking)
import FoundationNetworking
#endif

func restoreTestServer() async throws {
    let url = URL(string: "http://localhost:4000/restore?db=true&plugins=true")!
    _ = try await URLSession(configuration: .ephemeral).data(from: url)
}

extension TestCredentials {
    var siteURL: ParsedUrl {
        // swiftlint:disable:next force_try
        try! ParsedUrl.parse(input: siteUrl)
    }

    var apiRootURL: ParsedUrl {
        // swiftlint:disable:next force_try
        try! ParsedUrl.parse(input: siteUrl + "/wp-json")
    }

    var adminAuthentication: WpAuthentication {
        .init(username: adminUsername, password: adminPassword)
    }
}

extension WordPressAPI {
    static func admin(notifyingDelegate: URLSessionTaskDelegate? = nil) -> WordPressAPI {
        let credentials = TestCredentials.instance()
        return WordPressAPI(
            urlSession: .init(configuration: .ephemeral),
            notifyingDelegate: notifyingDelegate,
            siteInfo: .selfHosted(
                siteUrl: credentials.siteURL,
                apiRoot: credentials.apiRootURL
            ),
            authentication: credentials.adminAuthentication
        )
    }
}
