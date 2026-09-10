import Foundation
import Testing
import WordPressAPI

@Suite
struct SupportTicketAttachmentTests {

    /// A photo picked from the library keeps its original filename, and macOS names screenshots
    /// with spaces. `URL.path()` would hand the request executor `Screen%20Shot%201.png`.
    @Test func newTicketDecodesPercentEncodingInAttachmentPaths() {
        let params = CreateSupportTicketParams(
            subject: "Subject",
            message: "Message",
            application: "Test Suite",
            attachmentURLs: [URL(fileURLWithPath: "/tmp/attachments/Screen Shot 1.png")]
        )

        #expect(params.attachments == ["/tmp/attachments/Screen Shot 1.png"])
    }

    /// Characters beyond the space get encoded too — including non-ASCII, which `path()` renders as
    /// UTF-8 escapes.
    @Test func newTicketDecodesPercentEncodingBeyondSpaces() {
        let params = CreateSupportTicketParams(
            subject: "Subject",
            message: "Message",
            application: "Test Suite",
            attachmentURLs: [
                URL(fileURLWithPath: "/tmp/attachments/100% done.png"),
                URL(fileURLWithPath: "/tmp/attachments/café.png")
            ]
        )

        #expect(params.attachments == ["/tmp/attachments/100% done.png", "/tmp/attachments/café.png"])
    }

    /// A name needing no encoding has to survive untouched.
    @Test func newTicketLeavesAnOrdinaryPathAlone() {
        let params = CreateSupportTicketParams(
            subject: "Subject",
            message: "Message",
            application: "Test Suite",
            attachmentURLs: [URL(fileURLWithPath: "/tmp/attachments/IMG_0001.png")]
        )

        #expect(params.attachments == ["/tmp/attachments/IMG_0001.png"])
    }

    /// The convenience initializer has to carry every other field through unchanged.
    @Test func newTicketPassesThroughEveryOtherField() {
        let params = CreateSupportTicketParams(
            subject: "Subject",
            message: "Message",
            application: "Test Suite",
            wpcomSiteId: 1234,
            tags: ["tag1", "tag2"],
            encryptedLogIds: ["log-id"],
            attachmentURLs: []
        )

        #expect(params.subject == "Subject")
        #expect(params.message == "Message")
        #expect(params.application == "Test Suite")
        #expect(params.wpcomSiteId == 1234)
        #expect(params.tags == ["tag1", "tag2"])
        #expect(params.encryptedLogIds == ["log-id"])
        #expect(params.attachments.isEmpty)
    }

    /// The reply path is a different params type, so it needs its own coverage.
    @Test func replyDecodesPercentEncodingInAttachmentPaths() {
        let params = AddMessageToSupportConversationParams(
            message: "Message",
            attachmentURLs: [URL(fileURLWithPath: "/tmp/attachments/Screen Shot 1.png")]
        )

        #expect(params.attachments == ["/tmp/attachments/Screen Shot 1.png"])
    }

    @Test func replyLeavesAnOrdinaryPathAlone() {
        let params = AddMessageToSupportConversationParams(
            message: "Message",
            attachmentURLs: [URL(fileURLWithPath: "/tmp/attachments/IMG_0001.png")]
        )

        #expect(params.message == "Message")
        #expect(params.attachments == ["/tmp/attachments/IMG_0001.png"])
    }

    @Test func emptyAttachmentsProduceNoPaths() {
        let params = AddMessageToSupportConversationParams(message: "Message", attachmentURLs: [])

        #expect(params.attachments.isEmpty)
    }
}
