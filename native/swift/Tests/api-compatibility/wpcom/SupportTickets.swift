import Foundation
import Testing
import WordPressAPI

// This test ensures that existing code compiles – it's not meant to run anything
struct SupportTicketsCompatTests {

    @Test func `test ticket creation params`() async throws {
        _ = CreateSupportTicketParams(
            subject: "Hello World",
            message: "Test Message",
            application: "Test Suite",
            wpcomSiteId: 1234,
            tags: ["tag1", "tag2"],
            encryptedLogIds: [UUID().uuidString]
        )
    }

    @Test func `test ticket reply params`() async throws {
        _ = AddMessageToSupportConversationParams(
            message: "This is a reply",
            attachments: [
                "/path/to/file/on/disk"
            ]
        )
    }
}
