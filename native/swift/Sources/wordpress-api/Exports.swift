// Expose necessary Rust APIs as public API to the Swift package's consumers.
//
// We could export all of them using `@_exported import`, but that probably puts
// us in a position where we need to make major releases due to Rust code changes.

import wordpress_api_wrapper

public typealias WpApiError = wordpress_api_wrapper.WpApiError

// MARK: - Users

public typealias SparseUser = wordpress_api_wrapper.SparseUser

public extension SparseUser {
    typealias ID = UserId
    typealias View = UserWithViewContext
    typealias Edit = UserWithEditContext
    typealias Embed = UserWithEmbedContext
}
