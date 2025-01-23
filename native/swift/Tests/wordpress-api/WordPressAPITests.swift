import Foundation
import Testing
@testable import WordPressAPI

struct WordPressAPITests {

    @Test
    func testExample() async throws {
        let response = """
          {
            "id": 1,
            "name": "User Name",
            "url": "",
            "description": "",
            "link": "https://profiles.wordpress.org/user/",
            "slug": "poliuk",
            "avatar_urls": {
              "24": "https://secure.gravatar.com/avatar/uuid?s=24&d=mm&r=g",
              "48": "https://secure.gravatar.com/avatar/uuid?s=48&d=mm&r=g",
              "96": "https://secure.gravatar.com/avatar/uuid?s=96&d=mm&r=g"
            },
            "meta": [],
            "_links": {
              "self": [
                {
                  "href": "https://wordpress.org/wp-json/wp/v2/users/1"
                }
              ],
              "collection": [
                {
                  "href": "https://wordpress.org/wp-json/wp/v2/users"
                }
              ]
            }
          }
        """
        let stubs = HTTPStubs(stubs: [
            HTTPStubs.stub(path: "/wp-json/wp/v2/users/1", with: try .json(response))
        ])

        let api = try WordPressAPI(
            urlSession: .shared,
            baseUrl: ParsedUrl.parse(input: "https://wordpress.org"),
            authenticationStategy: .none,
            executor: stubs
        )
        let user = try await api.users.retrieveWithViewContext(userId: 1)
        #expect(user.data.name == "User Name")
    }
}

#if canImport(WordPressAPIInternal)
import WordPressAPIInternal
#endif

struct ConversionTest {

    @Test
    func useRustObjectHeaderMap() throws {
        let opaque = wpApiRustObjectGetHeaderMap()
        let headers = try UniffiHeaderMap(opaque: opaque)
        #expect(headers.headerValue(key: "Content-Type") == "application/json")
        #expect(headers.headerValue(key: "content-type") == "application/json")
        #expect(headers.headerValue(key: "ConTenT-Type") == "application/json")
        #expect(headers.headerValue(key: "User-Agent") == "wp-api-rs")
        #expect(headers.headerValue(key: "user-agent") == "wp-api-rs")
        #expect(headers.headerValue(key: "uSEr-aGENt") == "wp-api-rs")
    }

    @Test
    func useAnotherHeaderMap() throws {
        let headerMap = wpApiRustObjectGetNetworkHeaderMap()
        #expect(headerMap is Sendable)

        let headers = try UniffiHeaderMap(opaque: headerMap.inner)
        #expect(headers.headerValue(key: "X-WP-Total") == "10")
        #expect(headers.headerValue(key: "x-wp-total") == "10")
        #expect(headers.headerValue(key: "X-wp-TotAl") == "10")
    }

    @Test
    func useRustObjectAnotherType() throws {
        let opaque = wpApiRustObjectGetAnotherRandomType()
        let foo = try UniffiAnotherRandomType(opaque: opaque)
        #expect(foo.value() == "Hello from Rust!")
    }

    @Test
    func useRustObjectMismatch() {
        let opaque = wpApiRustObjectGetAnotherRandomType()
        #expect(performing: {
            try UniffiHeaderMap(opaque: opaque)
        }, throws: { error in
            error as? OpaqueRustObjectConversionError == .TypeMismatch(expected: .headerMap)
        })
    }
}

import XCTest

class BenchmarkTest: XCTestCase {

    let iterations = 1_000

    func testRustToSwiftBaseline() {
        self.measure(metrics: [XCTClockMetric()]) {
            for _ in 1...iterations {
                let _ = wpApiRustObjectGetLargeValue()
            }
        }
    }

    func testRustToSwift() {
        self.measure(metrics: [XCTClockMetric()]) {
            for _ in 1...iterations {
                let _ = wpApiRustObjectGetLargeOpaque()
            }
        }
    }

    func testSwiftToRustBaseline() {
        let value = wpApiRustObjectGetLargeValue()
        self.measure(metrics: [XCTClockMetric()]) {
            for _ in 1...iterations {
                let result = wpApiRustObjectUseLargeValue(map: value)
                XCTAssertEqual(result, 3)
            }
        }
    }

    func testSwiftToRust() {
        let value = wpApiRustObjectGetLargeOpaque()

        self.measure(metrics: [XCTClockMetric()]) {
            for _ in 1...iterations {
                let result = wpApiRustObjectUseLargeOpaque(opaque: value)
                XCTAssertEqual(result, 3)
            }
        }
    }

    func testRoundTripBaseline() {
        self.measure(metrics: [XCTClockMetric()]) {
            for _ in 1...iterations {
                let value = wpApiRustObjectGetLargeValue()
                let result = wpApiRustObjectUseLargeValue(map: value)
                XCTAssertEqual(result, 3)
            }
        }
    }

    func testRoundTrip() {
        self.measure(metrics: [XCTClockMetric()]) {
            for _ in 1...iterations {
                let value = wpApiRustObjectGetLargeOpaque()
                let result = wpApiRustObjectUseLargeOpaque(opaque: value)
                XCTAssertEqual(result, 3)
            }
        }
    }

}
