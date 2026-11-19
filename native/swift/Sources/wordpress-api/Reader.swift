import Foundation
import WordPressAPIInternal

extension ReaderStream {
    /// Whether the stream needs a WordPress.com account. Without one, it fails with
    /// ``ReaderError/NeedsAccount``.
    public var needsAccount: Bool {
        readerStreamNeedsAccount(stream: self)
    }

    /// What orders the stream's posts.
    public var ordering: ReaderStreamOrdering {
        readerStreamOrdering(stream: self)
    }
}
