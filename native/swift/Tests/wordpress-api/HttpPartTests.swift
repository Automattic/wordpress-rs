import Foundation
import Testing
@testable import WordPressAPI

@Suite("MutlipartRequest")
struct HttpPartTests {

    private let formPart = HttpPart.formData(name: "form-part", data: [:])

    private let filePart = HttpPart.file(
        name: "file-part",
        filePath: URL(fileURLWithPath: #file).appendingPathComponent("test.txt"),
        mimeType: "text/plain"
    )

    @Test("Form Part uses correct content disposition")
    func formPartUsesCorrectType() async throws {
        #expect(formPart.httpHeadersString.contains("Content-Disposition: form-data;"))
    }

    @Test("Form Part uses correct name")
    func formPartUsesCorrectName() async throws {
        #expect(formPart.httpHeadersString.contains("name=\"form-part\""))
    }

    @Test("Headers end with double line break")
    func formPartEndsWithDoubleLineBreak() async throws {
        #expect(formPart.httpHeadersString.hasSuffix("\r\n\r\n"))
    }

    @Test("File Part has content disposition")
    func filePartHasContentDisposition() async throws {
        #expect(filePart.httpHeadersString.hasPrefix("Content-Disposition: form-data;"))
    }

    @Test("File part uses supplied name")
    func filePartUsesSuppliedName() async throws {
        #expect(filePart.httpHeadersString.contains("name=\"file-part\""))
    }

    @Test("File part uses filename")
    func filePartUsesFilename() async throws {
        #expect(filePart.httpHeadersString.contains("filename=\"test.txt\""))
    }

    @Test("File part has line break between filename and content type")
    func filePartHasLineBreakBeforeContentType() async throws {
        #expect(filePart.httpHeadersString.contains("\r\nContent-Type:"))
    }

    @Test("File part uses supplied content type")
    func filePartUsesSuppliedContentType() async throws {
        #expect(filePart.httpHeadersString.contains("Content-Type: text/plain"))
    }

    @Test("Form part encodes data correctly", arguments: [
        ("key=value", ["key": "value"]),
        ("key=%26+%3D%3F%3C%3E%40%23%24%25%2F%5C", ["key": "& =?<>@#$%/\\"]),
        ("key1=&key2=", ["key1": "", "key2": ""]),
        ("first+name=alice", ["first name": "alice"]),
        ("first_name=alice", ["first_name": "alice"]),
        ("first-name=alice", ["first-name": "alice"]),
        ("first.name=alice", ["first.name": "alice"]),
        ("first*name=alice", ["first*name": "alice"]),
        ("name%2Ffirst=alice", ["name/first": "alice"]),
        ("alpha=%CE%B1&integral=%E2%88%AB&pi=%CF%80", ["alpha": "α", "integral": "∫", "pi": "π"]),
        ("1=%F0%9F%98%81&2=%F0%9F%98%A2&3=%F0%9F%92%81%F0%9F%8F%BD", ["1": "😁", "2": "😢", "3": "💁🏽"]),
        ("", [:]),
    ])
    func formPartEncodesDataCorrectly(encodedValue: String, rawValue: [String: String]) async throws {
        #expect(formPart.convertToFormString(rawValue) == encodedValue)
    }
}
