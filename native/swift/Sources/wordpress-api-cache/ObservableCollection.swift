import Foundation
import WordPressAPIInternal

public protocol ObservableCollection<Element>: Sendable {
    associatedtype Element

    /// The underlying table affected by this change (this is how we subscribe to the correct `NotificationCenter` name)
    var tableName: String { get }

    /// An identifier for this specific collection (used to invalidate `NotificationCenter` handles). Right now this defaults to the
    var handle: UInt64 { get }

    /// The method used to load data from the cache. Usually not implemented directly – if `Element` is correct this can be mapped automatically
    func loadData() async throws -> [Element]
}

extension AllAnyPostWithEditContextCollection: ObservableCollection {
    public typealias Element = FullEntityAnyPostWithEditContext

    public var tableName: String { "postsEditContext" }
    public var handle: UInt64 { uniffiCloneHandle() }
}

extension PostCollectionWithEditContext: ObservableCollection {
    public typealias Element = FullEntityAnyPostWithEditContext

    public var tableName: String { "postsEditContext" }
    public var handle: UInt64 { uniffiCloneHandle() }
}
