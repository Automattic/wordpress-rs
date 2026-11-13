import Foundation
import WordPressAPIInternal

extension ReaderPostSaveIdentity {
    /// The key to save this post under, or `nil` when the post is missing the IDs it needs.
    public var savedKey: ReaderSavedKey? {
        readerSavedKeyForPost(post: self)
    }

    /// Every key this post may already be saved under, starting with ``savedKey``.
    ///
    /// Show the post as saved if any of these keys is, and unsave with the keys that matched.
    /// Compare them by ``ReaderSavedKey/normalized``.
    public var savedKeys: [ReaderSavedKey] {
        readerSavedKeysForPost(post: self)
    }
}

extension ReaderSavedKey {
    /// The key as the API matches it: two spellings of a URL compare equal once normalized.
    public var normalized: ReaderSavedKey {
        readerSavedKeyNormalized(key: self)
    }
}
