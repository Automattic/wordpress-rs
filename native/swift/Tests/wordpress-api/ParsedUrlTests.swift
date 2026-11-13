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
        "URLs with characters RFC 3986 doesn't allow become URLs",
        arguments: [
            "https://example.com/icon.png?v={hash}|2^x[0]",
            "https://example.com/a|b^c",
            "https://example.com/#a{b}#c",
            "https://example.com/100%?q=%zz"
        ]
    )
    func testStrictParsing(_ string: String) throws {
        let parsedUrl = try ParsedUrl.parse(input: string)
        _ = parsedUrl.asURL()
        // Before iOS 17 and macOS 14, `URL(string:)` doesn't encode invalid characters. This option restores that
        // behaviour, so the test covers the older systems the package supports.
        if #available(macOS 14, iOS 17, tvOS 17, watchOS 10, *) {
            #expect(URL(string: string, encodingInvalidCharacters: false) == nil)
            #expect(URL(string: parsedUrl.rfc3986Url(), encodingInvalidCharacters: false) != nil)
        }
    }
}
