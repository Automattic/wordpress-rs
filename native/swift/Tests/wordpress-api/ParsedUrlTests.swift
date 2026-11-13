import Foundation
import Testing
import WordPressAPI

class ParsedUrlTests {

    @Test(
        "URLs are parsed successfully",
        arguments: [
            "http://example.com/",
            "https://www.example.com/path/to/resource",
            "https://example.com/search?q=unit+testing&sort=asc",
            "https://example.com/index.html#section",
            "http://example.com:8080/path",
            "https://subdomain.example.com/",
            "http://user:password@example.com/",
            "file:///home/user/file.txt",
            "ftp://ftp.example.com/resource.txt",
            "http://[2001:db8::1]:8080/"
        ]
    )
    func testRoundTrip(_ string: String) throws {
        let parsedUrl = try ParsedUrl.parse(input: string)
        #expect(string == parsedUrl.asURL().absoluteString)
    }

    @Test(
        "Query pairs are appended across the FFI boundary",
        arguments: [
            // Path root gains `?k=v&k=v`.
            (
                "https://example.com/wp-json/wp/v2/themes",
                [QueryPair(name: "context", value: "edit"), QueryPair(name: "status", value: "active")],
                "https://example.com/wp-json/wp/v2/themes?context=edit&status=active"
            ),
            // Query (`?rest_route=`) root keeps the existing value and gains `&k=v`.
            (
                "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fthemes",
                [QueryPair(name: "context", value: "edit"), QueryPair(name: "status", value: "active")],
                "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fthemes&context=edit&status=active"
            ),
            // Reserved characters in a value are form-urlencoded.
            (
                "https://example.com/wp-json/wp/v2/themes",
                [QueryPair(name: "exclude", value: "core,gutenberg")],
                "https://example.com/wp-json/wp/v2/themes?exclude=core%2Cgutenberg"
            )
        ]
    )
    func testByAppendingQueryPairs(_ input: String, _ pairs: [QueryPair], _ expected: String) throws {
        let parsedUrl = try ParsedUrl.parse(input: input)
        #expect(parsedUrl.byAppendingQueryPairs(pairs: pairs).url() == expected)
    }

    @Test("Appending an empty array leaves the URL unchanged")
    func testByAppendingEmptyQueryPairs() throws {
        let parsedUrl = try ParsedUrl.parse(input: "https://example.com/wp-json/wp/v2/themes")
        #expect(parsedUrl.byAppendingQueryPairs(pairs: []).url() == parsedUrl.url())
    }
}
