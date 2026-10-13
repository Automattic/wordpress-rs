import Foundation
import Testing
import WordPressAPI

struct LocalContentTests {
    private func openStore(path: String? = nil) throws -> WpLocalContentStore {
        let store = try WpLocalContentStore(path: path)
        _ = try store.performMigrations()
        return store
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

        #expect(throws: LocalContentError.self) {
            try service.savePostWorkingCopy(id: unknownId, workingCopy: PostWorkingCopy(title: "Title", content: "content"))
        }
    }

    @Test func postWorkingCopySurvivesReopeningTheDatabase() throws {
        let path = FileManager.default.temporaryDirectory
            .appendingPathComponent("local_content-\(UUID().uuidString).db")
            .path
        defer { try? FileManager.default.removeItem(atPath: path) }
        let created = PostWorkingCopy(title: "Kept", content: "kept content")

        let id = try LocalContentService(store: try openStore(path: path)).createPostWorkingCopy(workingCopy: created)
        let read = try #require(try LocalContentService(store: try openStore(path: path)).readPostWorkingCopy(id: id))

        #expect(read.workingCopy == created)
    }
}
