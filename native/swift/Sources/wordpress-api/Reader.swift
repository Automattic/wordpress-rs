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

extension ReaderError {
    /// What the error says, in words, for an app to show.
    public var message: String {
        readerErrorMessage(error: self)
    }
}

extension ReaderLinkNotInReader {
    /// Why the link opens nothing in the Reader, in words, for an app to show.
    public var message: String {
        readerLinkNotInReaderMessage(reason: self)
    }
}

extension ReaderSavedError {
    /// What the error says, in words, for an app to show.
    public var message: String {
        readerSavedErrorMessage(error: self)
    }
}

extension ReaderLikesResponse {
    /// The parameters for the page after this one, asking for `number` likes, or `nil` when this is the last
    /// page.
    public func nextPageParams(number: UInt32) -> ReaderLikesParams? {
        readerLikesNextPageParams(response: self, number: number)
    }
}

extension ReaderMenuItem {
    /// The stream the entry opens, or `nil` for an entry that isn't a stream this version knows.
    public var stream: ReaderStream? {
        readerStreamForMenuItem(item: self)
    }
}

extension ReaderData {
    /// Whether a change to the cache is one this shows, so that a screen reloads only when what it shows
    /// changed.
    public func isRelevantUpdate(hook: UpdateHook) -> Bool {
        readerDataIsRelevantUpdate(data: self, hook: hook)
    }
}
