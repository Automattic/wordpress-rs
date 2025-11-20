import Foundation
import WordPressAPIInternal

public protocol ObservableCollection {
    /// The underlying table affected by this change (this is how we subscribe to the correct `NotificationCenter` name)
    var tableName: String { get }

    /// An identifier for this specific collection (used to invalidate `NotificationCenter` handles). Right now this defaults to the
    var handle: UInt64 { get }
}

extension AllAnyPostWithEditContextCollection: ObservableCollection {
    public var tableName: String { "postsEditContext" }
    public var handle: UInt64 { uniffiCloneHandle() }
}

extension PostCollectionWithEditContext: ObservableCollection {
    public var tableName: String { "postsEditContext" }
    public var handle: UInt64 { uniffiCloneHandle() }
}
