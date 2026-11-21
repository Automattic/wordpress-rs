import Foundation
import Testing

@testable import WordPressAPI

@Suite("Debug log redaction")
struct DebugLogRedactionTests {
    private static let secret = "not-a-real-token"

    @Test("A request's credentials are not printed, but their header names are")
    func credentialsAreNotPrinted() {
        let lines = DebugLogRedaction.requestLines(
            method: "GET",
            url: "https://example.com/wp-json/wp/v2/posts",
            headers: [
                "authorization": ["Bearer \(Self.secret)"],
                "Proxy-Authorization": ["Basic \(Self.secret)"],
                "Cookie": ["wordpress_logged_in=\(Self.secret)"],
                "SET-COOKIE": ["session=\(Self.secret)"],
                "X-WP-Nonce": [Self.secret],
                "accept": ["application/json"]
            ],
            body: "password=\(Self.secret)"
        )
        let output = lines.joined(separator: "\n")

        #expect(!output.contains(Self.secret))
        #expect(lines.contains("authorization: Bearer <redacted>"))
        #expect(lines.contains("Proxy-Authorization: Basic <redacted>"))
        #expect(lines.contains("Cookie: <redacted>"))
        #expect(lines.contains("SET-COOKIE: <redacted>"))
        #expect(lines.contains("X-WP-Nonce: <redacted>"))
        #expect(lines.contains("accept: application/json"))
        #expect(lines.contains("<body of 25 bytes, not printed>"))
    }

    @Test(
        "Header names are matched without regard to case",
        arguments: ["Authorization", "AUTHORIZATION", "aUtHoRiZaTiOn"]
    )
    func headerNamesIgnoreCase(name: String) {
        #expect(DebugLogRedaction.headerValue("Bearer \(Self.secret)", forHeader: name) == "Bearer <redacted>")
    }

    @Test("An Authorization value without a scheme is redacted whole")
    func valueWithoutAScheme() {
        #expect(DebugLogRedaction.headerValue(Self.secret, forHeader: "Authorization") == "<redacted>")
        #expect(DebugLogRedaction.headerValue("\"\(Self.secret)\" more", forHeader: "Authorization") == "<redacted>")
    }

    @Test("Other headers are printed as they are")
    func otherHeaders() {
        #expect(DebugLogRedaction.headerValue("application/json", forHeader: "Accept") == "application/json")
    }
}
