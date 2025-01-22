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
        #expect(headers.headerValue(key: "User-Agent") == "wp-api-rs")
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
