import Foundation
import WordPressAPIInternal

public extension BotMessageContext {
    var userWantsToTalkToAHuman: Bool {
        WordPressAPIInternal.userWantsToTalkToAHuman(context: self)
    }
}

public extension BotConversation {
    var userWantsToTalkToAHuman: Bool {
        for message in self.messages {
            if case .bot(let botContext) = message.context {
                if botContext.userWantsToTalkToAHuman {
                    return true
                }
            }
        }

        return false
    }
}

extension WpComSiteIdentifier: ExpressibleByStringLiteral, ExpressibleByIntegerLiteral {
    public init(stringLiteral value: StringLiteralType) {
        self = .slug(value: value)
    }

    public init(integerLiteral value: IntegerLiteralType) {
        precondition(value > 0, "WpComSiteIdentifier must be a positive integer")
        self = .id(value: UInt64(value))
    }

    public init?(_ value: IntegerLiteralType?) {
        guard let value else {
            return nil
        }

        self = WpComSiteIdentifier(integerLiteral: value)
    }
}

// MARK: - Attachments

/// Support attachments cross the bindings as filesystem paths, and the request executor opens each
/// one directly. `URL.path()` percent-encodes by default, so passing it produces a path that
/// doesn't exist on disk — a filename with a space is enough — and the request fails with
/// `MediaFileNotFound`:
///
/// ```swift
/// let url = URL(fileURLWithPath: "/tmp/Screen Shot 1.png")
/// url.path()  // "/tmp/Screen%20Shot%201.png"  ❌
/// url.path    // "/tmp/Screen Shot 1.png"      ✅
/// ```
///
/// The decoded `path` is also what `MultipartFormContent` uses to open the file, so the value
/// produced here and the value consumed there are read the same way.
///
/// These initializers take the URLs and own the conversion, so a caller holding a file URL — from
/// a photo picker, say, where the filename comes from the user's library — can't get it wrong.
///
/// `attachmentURLs` deliberately has no default value. Giving it one would make a call that
/// passes no attachments at all match both this initializer and the generated one.
///
/// Keep the parameter lists in sync with the generated memberwise initializers in `wp_api.swift`.

public extension CreateSupportTicketParams {
    init(
        subject: String,
        message: String,
        application: String,
        wpcomSiteId: UInt64? = nil,
        tags: [String] = [],
        encryptedLogIds: [String] = [],
        attachmentURLs: [URL]
    ) {
        self.init(
            subject: subject,
            message: message,
            application: application,
            wpcomSiteId: wpcomSiteId,
            tags: tags,
            encryptedLogIds: encryptedLogIds,
            attachments: attachmentURLs.map(\.path)
        )
    }
}

public extension AddMessageToSupportConversationParams {
    init(message: String, attachmentURLs: [URL]) {
        self.init(
            message: message,
            attachments: attachmentURLs.map(\.path)
        )
    }
}
