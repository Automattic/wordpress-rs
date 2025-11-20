import Foundation
import WordPressAPIInternal
import Synchronization

/// A thread-safe singleton that manages database change notifications for WordPress API cache entities.
///
/// `DatabaseChangeNotifier` acts as a bridge between the Rust-based database layer and Swift observers,
/// using `NotificationCenter` to propagate database update events to registered observers. It supports
/// observing both individual entities (specific rows) and collections (entire tables).
///
/// The class implements `DatabaseDelegate` to receive callbacks when the underlying database changes,
/// then broadcasts these changes to all registered observers.
///
/// ## Usage
///
/// ```swift
/// // Start observing an entity
/// DatabaseChangeNotifier.shared.startObserving(observableEntity) { updateHook in
///     print("Entity updated: \(updateHook)")
/// }
///
/// // Start observing a collection
/// DatabaseChangeNotifier.shared.startObserving(observableCollection) { updateHook in
///     print("Collection updated: \(updateHook)")
/// }
///
/// // Stop observing when done
/// DatabaseChangeNotifier.shared.stopObserving(observableEntity)
/// DatabaseChangeNotifier.shared.stopObserving(observableCollection)
/// ```
///
/// - Note: Observers are automatically replaced if you start observing the same entity/collection multiple times.
///         The previous observer will be cleaned up automatically.
public final class DatabaseChangeNotifier: DatabaseDelegate, Sendable {

    /// The shared singleton instance of the database change notifier.
    public static let shared = DatabaseChangeNotifier()

    /// Internal storage for notification observer tokens.
    private let tokenStore = TokenStore()

    private init() {}

    /// Called by the database layer when an update occurs.
    ///
    /// This method posts notifications to both entity-specific and collection-wide observers
    /// based on the update information provided in the hook.
    ///
    /// - Parameter updateHook: Information about the database update, including table name and row ID.
    public func didUpdate(updateHook: WordPressAPIInternal.UpdateHook) {
        NotificationCenter.default.post(name: entityNotificationName(for: updateHook), object: nil, userInfo: [
            "updateHook": updateHook
        ])

        NotificationCenter.default.post(name: collectionNotificationName(for: updateHook), object: nil, userInfo: [
            "updateHook": updateHook
        ])
    }

    /// Starts observing changes to a specific database entity.
    ///
    /// Registers a callback to be invoked whenever the specified entity is updated in the database.
    /// If an observer is already registered for this entity, it will be replaced and the previous
    /// observer will be automatically cleaned up.
    ///
    /// - Parameters:
    ///   - object: The entity to observe.
    ///   - callback: A closure called when the entity is updated. The closure receives an `UpdateHook`
    ///               containing information about the database change.
    ///
    /// - Note: You must call `stopObserving(_:)` when you're done observing to prevent memory leaks.
    public func startObserving(
        _ object: ObservableEntity,
        callback: @escaping @Sendable (UpdateHook) -> Void
    ) {
        let token = NotificationCenter.default.addObserver(
            forName: notificationName(for: object),
            object: nil,
            queue: nil
        ) { notification in
            guard let updateHook = notification.userInfo?["updateHook"] as? UpdateHook else { return }
            callback(updateHook)
        }

        tokenStore.add(token, for: object.entityId) {
            NotificationCenter.default.removeObserver($0)
        }
    }

    /// Starts observing changes to all entities in a database collection (table).
    ///
    /// Registers a callback to be invoked whenever any entity in the specified collection is updated.
    /// If an observer is already registered for this collection, it will be replaced and the previous
    /// observer will be automatically cleaned up.
    ///
    /// - Parameters:
    ///   - collection: The collection to observe.
    ///   - callback: A closure called when any entity in the collection is updated. The closure receives
    ///               an `UpdateHook` containing information about the database change.
    ///
    /// - Note: You must call `stopObserving(_:)` when you're done observing to prevent memory leaks.
    public func startObserving(
        _ collection: ObservableCollection,
        callback: @escaping @Sendable (UpdateHook) -> Void
    ) {
        let token = NotificationCenter.default.addObserver(
            forName: notificationName(for: collection),
            object: nil,
            queue: nil
        ) { notification in
            guard let updateHook = notification.userInfo?["updateHook"] as? UpdateHook else { return }
            callback(updateHook)
        }

        tokenStore.add(token, for: collection.handle) {
            NotificationCenter.default.removeObserver($0)
        }
    }

    /// Stops observing changes to a specific database entity.
    ///
    /// Removes the observer callback previously registered with `startObserving(_:callback:)`.
    /// If no observer is registered for this entity, this method does nothing.
    ///
    /// - Parameter object: The entity to stop observing.
    public func stopObserving(_ object: ObservableEntity) {
        if let token = tokenStore.get(for: object.entityId) {
            NotificationCenter.default.removeObserver(token)
        }
    }

    /// Stops observing changes to a database collection.
    ///
    /// Removes the observer callback previously registered with `startObserving(_:callback:)`.
    /// If no observer is registered for this collection, this method does nothing.
    ///
    /// - Parameter collection: The collection to stop observing.
    public func stopObserving(_ collection: ObservableCollection) {
        if let token = tokenStore.get(for: collection.handle) {
            NotificationCenter.default.removeObserver(token)
        }
    }

    private func notificationName(for entity: ObservableEntity) -> Notification.Name {
        let table = entity.entityId.table
        let rowid = entity.entityId.rowid
        return Notification.Name("org.wordpress.swift.ObservableEntity.\(table).\(rowid)")
    }

    private func notificationName(for collection: ObservableCollection) -> Notification.Name {
        Notification.Name("org.wordpress.swift.ObservableCollection.\(collection.tableName)")
    }

    private func entityNotificationName(for hook: UpdateHook) -> Notification.Name {
        Notification.Name("org.wordpress.swift.ObservableEntity.\(hook.table).\(hook.rowId)")
    }

    private func collectionNotificationName(for hook: UpdateHook) -> Notification.Name {
        Notification.Name("org.wordpress.swift.ObservableCollection.\(hook.table)")
    }
}

/// Internal thread-safe storage for notification observer tokens.
///
/// `TokenStore` manages `NotificationCenter` observer tokens for both entities and collections,
/// ensuring thread-safe access using separate locks for each type of token. When a token is added
/// for a key that already has a token, the old token is presented for cleanup.
///
/// - Note: This class uses `@unchecked Sendable` because it implements thread safety manually
///         using `NSLock`, which is not `Sendable` itself.
class TokenStore: @unchecked Sendable {

    /// Lock for thread-safe access to entity tokens.
    private let entityLock = NSLock()

    /// Storage for entity-specific observer tokens, keyed by entity ID.
    private var entityTokens = [EntityId: NSObjectProtocol]()

    /// Lock for thread-safe access to collection tokens.
    private let collectionLock = NSLock()

    /// Storage for collection-specific observer tokens, keyed by collection handle.
    private var collectionTokens = [UInt64: NSObjectProtocol]()

    /// Adds an observer token for a specific entity.
    ///
    /// If a token already exists for the given entity ID, the replacement callback is called
    /// with the existing token (typically to remove the observer), then the new token replaces it.
    ///
    /// - Parameters:
    ///   - token: The notification observer token to store.
    ///   - key: The entity ID to associate with the token.
    ///   - replacementCallback: A closure called with the existing token if one exists.
    func add(_ token: NSObjectProtocol, for key: EntityId, replacementCallback: (NSObjectProtocol) -> Void) {
        entityLock.withLock {
            if let existingToken = self.entityTokens[key] {
                replacementCallback(existingToken)
            }

            self.entityTokens[key] = token
        }
    }

    /// Adds an observer token for a specific collection.
    ///
    /// If a token already exists for the given collection handle, the replacement callback is called
    /// with the existing token (typically to remove the observer), then the new token replaces it.
    ///
    /// - Parameters:
    ///   - token: The notification observer token to store.
    ///   - key: The collection handle to associate with the token.
    ///   - replacementCallback: A closure called with the existing token if one exists.
    func add(_ token: NSObjectProtocol, for key: UInt64, replacementCallback: (NSObjectProtocol) -> Void) {
        collectionLock.withLock {
            if let existingToken = self.collectionTokens[key] {
                replacementCallback(existingToken)
            }

            self.collectionTokens[key] = token
        }
    }

    /// Retrieves and removes an observer token for a specific entity.
    ///
    /// This method atomically retrieves the token and removes it from storage.
    /// If no token exists for the given entity ID, returns `nil`.
    ///
    /// - Parameter key: The entity ID to look up.
    /// - Returns: The observer token if one exists, or `nil` otherwise.
    func get(for key: EntityId) -> NSObjectProtocol? {
        entityLock.withLock {
            guard let value = self.entityTokens[key] else {
                return nil
            }

            self.entityTokens.removeValue(forKey: key)
            return value
        }
    }

    /// Retrieves and removes an observer token for a specific collection.
    ///
    /// This method atomically retrieves the token and removes it from storage.
    /// If no token exists for the given collection handle, returns `nil`.
    ///
    /// - Parameter key: The collection handle to look up.
    /// - Returns: The observer token if one exists, or `nil` otherwise.
    func get(for key: UInt64) -> NSObjectProtocol? {
        collectionLock.withLock {
            guard let value = self.collectionTokens[key] else {
                return nil
            }

            self.collectionTokens.removeValue(forKey: key)
            return value
        }
    }
}
