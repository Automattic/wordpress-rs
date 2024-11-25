import Foundation
import Testing
@testable import WordPressAPI

@Suite("Multipart")
class MultipartRequestBodyTests {

    private let emptyForm = MultipartRequestBody()
        .addPart(.formData(name: "Test", data: [:]))

    private let emptyFile = MultipartRequestBody()
        .addPart(.file(
            name: "test-file",
            filePath: Bundle.module.url(
                forResource: "LoginTests-wp-json",
                withExtension: "json",
                subdirectory: "Responses"
            )!,
            mimeType: "application/json")
        )

    @Test("Adding form data starts with boundary")
    func formDataBoundary() async throws {
        try await #expect(lines(in: emptyForm.build()).first == "--" + emptyForm.boundaryString)
    }

    @Test("Adding file data starts with boundary")
    func fileDataBoundary() async throws {
        try await #expect(lines(in: emptyFile.build()).first == "--" + emptyForm.boundaryString)
    }

    @Test("Request ends with closing boundary")
    func requestEndsWithBoundary() async throws {
        try await #expect(lines(in: emptyForm.build()).last == "--" + emptyForm.boundaryString + "--")
        try await #expect(lines(in: emptyFile.build()).last == "--" + emptyFile.boundaryString + "--")
    }

    private func lines(in url: URL) throws -> [String] {
        guard let string = String(data: try data(from: url), encoding: .utf8) else {
            throw CocoaError(.fileReadUnknownStringEncoding)
        }

        return string.split(separator: "\r\n").map { String($0) }
    }

    private func data(from url: URL) throws -> Data {
        try Data(contentsOf: url)
    }
}
