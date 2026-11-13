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
            "https://example.com/100%?q=%zz",
            "https://{{cdn}}/favicon.png",
            "https://us%er@example.com/"
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

    @Test("Every printable ASCII character, anywhere in a URL, gives a URL the strict parser accepts")
    func testStrictParsingOfEveryCharacter() throws {
        guard #available(macOS 14, iOS 17, tvOS 17, watchOS 10, *) else { return }
        for scalar in (0x21...0x7E).compactMap(Unicode.Scalar.init) {
            let character = String(scalar)
            let inputs = [
                "https://a\(character)b.example/",
                "https://u\(character)v@example.com/",
                "https://u:p\(character)q@example.com/",
                "https://example.com/a\(character)b",
                "https://example.com/?a\(character)b",
                "https://example.com/#a\(character)b",
                "foo://a\(character)b/c",
                "foo:a\(character)b"
            ]
            for input in inputs {
                // Some characters make the URL invalid; Rust rejects those, and there's nothing to convert.
                guard let parsedUrl = try? ParsedUrl.parse(input: input) else { continue }
                #expect(
                    URL(string: parsedUrl.rfc3986Url(), encodingInvalidCharacters: false) != nil,
                    "\(input) became \(parsedUrl.rfc3986Url())"
                )
            }
        }
    }
}
