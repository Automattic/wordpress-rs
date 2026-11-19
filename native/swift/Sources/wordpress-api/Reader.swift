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

extension ReaderPostRecord {
    /// The images to show for the post when its card is a gallery, out of its content, in the
    /// order they appear. There are 10 at most.
    public var galleryImageURLs: [String] {
        readerPostGalleryImageUrls(content: content ?? "")
    }

    /// The addresses of the images in the post's content, for storing them so the post can be
    /// read with no connection. Tracking pixels are left out.
    public var contentImageURLs: [String] {
        readerPostContentImageUrls(content: content ?? "")
    }
}
