import Foundation
import Testing
import WordPressAPI

final class LocalContentTests {
    private let directory: URL

    private var databasePath: String {
        directory.appendingPathComponent("local_content.db").path
    }

    init() throws {
        directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("LocalContentTests-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    }

    deinit {
        try? FileManager.default.removeItem(at: directory)
    }

    private func openStore() throws -> WpLocalContentStore {
        let store = try WpLocalContentStore(path: databasePath)
        _ = try store.performMigrations()
        return store
    }

    @Test func openingAnInMemoryDatabaseThrows() {
        #expect(throws: LocalContentError.self) {
            try WpLocalContentStore(path: ":memory:")
        }
    }

    @Test func createThenReadReturnsTheCreatedPostWorkingCopy() throws {
        let service = LocalContentService(store: try openStore())
        let created = PostWorkingCopy(title: "Draft title", content: "Draft content")

        let id = try service.createPostWorkingCopy(workingCopy: created)
        let read = try #require(try service.readPostWorkingCopy(id: id))

        #expect(read.id == id)
        #expect(read.workingCopy == created)
    }

    @Test func saveThenReadReturnsTheSavedPostWorkingCopy() throws {
        let service = LocalContentService(store: try openStore())
        let id = try service.createPostWorkingCopy(workingCopy: PostWorkingCopy(title: "First", content: "first"))

        let saved = PostWorkingCopy(title: "Second", content: "second")
        try service.savePostWorkingCopy(id: id, workingCopy: saved)
        let read = try #require(try service.readPostWorkingCopy(id: id))

        #expect(read.workingCopy == saved)
    }

    @Test func savingAnUnknownPostWorkingCopyThrows() throws {
        let service = LocalContentService(store: try openStore())
        let unknownId = "00000000-0000-4000-8000-000000000000"
        let workingCopy = PostWorkingCopy(title: "Title", content: "content")

        #expect(throws: LocalContentError.self) {
            try service.savePostWorkingCopy(id: unknownId, workingCopy: workingCopy)
        }
    }

    @Test func postWorkingCopySurvivesReopeningTheDatabase() throws {
        let created = PostWorkingCopy(title: "Kept", content: "kept content")

        let id = try LocalContentService(store: try openStore()).createPostWorkingCopy(workingCopy: created)
        let read = try #require(try LocalContentService(store: try openStore()).readPostWorkingCopy(id: id))

        #expect(read.workingCopy == created)
    }
}
